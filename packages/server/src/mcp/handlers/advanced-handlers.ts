import { resolve } from 'node:path';
import { formatLSPLocation, toHumanPosition } from '../../../../@codeflow/core/src/utils/index.js';
import type { SymbolService } from '../../../../@codeflow/features/src/services/lsp/symbol-service.js';
import type { TextEdit, WorkspaceEdit } from '../../core/file-operations/editor.js';
import { applyWorkspaceEdit } from '../../core/file-operations/editor.js';
import { pathToUri, uriToPath } from '../../core/file-operations/path-utils.js';
import type { FileService } from '../../services/file-service.js';
import type { DocumentSymbol, SymbolInformation } from '../../types.js';
import { registerTools } from '../tool-registry.js';
import {
  createFileModificationResponse,
  createLimitedSupportResponse,
  createListResponse,
  createMCPResponse,
  createNoChangesResponse,
  createNoResultsResponse,
} from '../utils.js';

// Handler for get_code_actions tool
export async function handleGetCodeActions(
  fileService: FileService,
  args: {
    file_path: string;
    range?: {
      start: { line: number; character: number };
      end: { line: number; character: number };
    };
  }
) {
  const { file_path, range } = args;
  const absolutePath = resolve(file_path);

  try {
    const codeActions = await fileService.getCodeActions(absolutePath, range);

    if (codeActions.length === 0) {
      return createNoResultsResponse(
        'code actions',
        `${file_path}${range ? ` at lines ${toHumanPosition(range.start).line}-${toHumanPosition(range.end).line}` : ''}`
      );
    }

    const actionDescriptions = codeActions
      .filter((action) => action && (action.title || action.kind))
      .map((action, index) => {
        if (action.title) {
          return `${index + 1}. ${action.title}${action.kind ? ` (${action.kind})` : ''}`;
        }
        return `${index + 1}. Code action (${action.kind || 'unknown'})`;
      });

    const noteText =
      "\n\nNote: These actions show what's available but cannot be applied directly through this tool. Use your editor's code action functionality to apply them.";
    const listResponse = createListResponse(`for ${file_path}`, actionDescriptions, {
      singular: 'code action',
      plural: 'code actions',
      showTotal: true,
    });

    return {
      content: [
        {
          type: 'text',
          text: (listResponse.content[0]?.text || '') + noteText,
        },
      ],
    };
  } catch (error) {
    return createMCPResponse(
      `Error getting code actions: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for format_document tool
export async function handleFormatDocument(
  fileService: FileService,
  args: {
    file_path: string;
    options?: {
      tab_size?: number;
      insert_spaces?: boolean;
      trim_trailing_whitespace?: boolean;
      insert_final_newline?: boolean;
      trim_final_newlines?: boolean;
    };
  },
  lspClient?: import('../../lsp/lsp-client.js').LSPClient
) {
  const { file_path, options } = args;
  const absolutePath = resolve(file_path);

  try {
    // Convert snake_case to camelCase for LSP client
    const lspOptions = options
      ? {
          tabSize: options.tab_size,
          insertSpaces: options.insert_spaces,
          trimTrailingWhitespace: options.trim_trailing_whitespace,
          insertFinalNewline: options.insert_final_newline,
          trimFinalNewlines: options.trim_final_newlines,
        }
      : undefined;

    const formatEdits = await fileService.formatDocument(absolutePath, lspOptions);

    if (formatEdits.length === 0) {
      return createNoChangesResponse('formatting', `${file_path} is already properly formatted`);
    }

    // Apply the formatting edits using the existing infrastructure
    const workspaceEdit = {
      changes: {
        [pathToUri(absolutePath)]: formatEdits,
      },
    };

    const editResult = await applyWorkspaceEdit(workspaceEdit, {
      lspClient,
    });

    if (!editResult.success) {
      return createMCPResponse(`Failed to apply formatting: ${editResult.error}`);
    }

    return createFileModificationResponse('formatted', file_path, {
      changeCount: formatEdits.length,
    });
  } catch (error) {
    return createMCPResponse(
      `Error formatting document: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for search_workspace_symbols tool
export async function handleSearchWorkspaceSymbols(
  symbolService: SymbolService,
  args: { query: string; workspace_path?: string },
  lspClient: import('../../lsp/lsp-client.js').LSPClient
) {
  const { query, workspace_path } = args;

  // Handle empty query gracefully
  if (query.trim().length === 0) {
    return createMCPResponse(
      'Please provide a search query to find workspace symbols. Enter a symbol name or partial name to search across all files in the workspace.'
    );
  }

  try {
    const symbols = await symbolService.searchWorkspaceSymbols(
      query,
      lspClient.serverManager.activeServers,
      lspClient.preloadServers.bind(lspClient),
      workspace_path
    );

    if (symbols.length === 0) {
      return createNoResultsResponse('symbols', `matching "${query}"`, [
        'Try a different search term or ensure the language server is properly configured.',
      ]);
    }

    const symbolDescriptions = symbols
      .slice(0, 50) // Limit to first 50 results
      .map((symbol, index) => {
        const location = symbol.location;
        const symbolKind = symbol.kind ? String(symbol.kind) : 'unknown';
        const locationString = formatLSPLocation(location);

        return `${index + 1}. ${symbol.name} (${symbolKind}) - ${locationString}`;
      });

    return createListResponse(`matching "${query}"`, symbolDescriptions, {
      singular: 'symbol',
      plural: 'symbols',
      maxItems: 50,
      showTotal: true,
    });
  } catch (error) {
    return createMCPResponse(
      `Error searching workspace symbols: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for get_document_symbols tool
export async function handleGetDocumentSymbols(
  symbolService: SymbolService,
  args: { file_path: string }
) {
  const { file_path } = args;
  const absolutePath = resolve(file_path);

  try {
    const symbols = await symbolService.getDocumentSymbols(absolutePath);

    if (symbols.length === 0) {
      return createNoResultsResponse('symbols', file_path, [
        'The file may be empty or the language server may not support this file type.',
      ]);
    }

    // Check if we have DocumentSymbols (hierarchical) or SymbolInformation (flat)
    const isHierarchical = symbolService.isDocumentSymbolArray(symbols);

    let symbolDescriptions: string[];

    if (isHierarchical) {
      // Handle hierarchical DocumentSymbol[]
      const formatDocumentSymbol = (symbol: DocumentSymbol, indent = 0): string[] => {
        const prefix = '  '.repeat(indent);
        const humanPos = toHumanPosition(symbol.range.start);
        const symbolKind = symbolService.symbolKindToString(symbol.kind);

        const result = [
          `${prefix}${symbol.name} (${symbolKind}) - Line ${humanPos.line}:${humanPos.character}`,
        ];

        if (symbol.children && symbol.children.length > 0) {
          for (const child of symbol.children) {
            result.push(...formatDocumentSymbol(child, indent + 1));
          }
        }

        return result;
      };

      symbolDescriptions = [];
      for (const symbol of symbols) {
        symbolDescriptions.push(...formatDocumentSymbol(symbol));
      }
    } else {
      // Handle flat SymbolInformation[]
      symbolDescriptions = symbols.map((symbol: SymbolInformation, index: number) => {
        const humanPos = toHumanPosition(symbol.location.range.start);
        const symbolKind = symbol.kind ? symbolService.symbolKindToString(symbol.kind) : 'unknown';

        return `${index + 1}. ${symbol.name} (${symbolKind}) - Line ${humanPos.line}:${humanPos.character}`;
      });
    }

    return createMCPResponse(
      `Document outline for ${file_path}:\n\n${symbolDescriptions.join('\n')}`
    );
  } catch (error) {
    return createMCPResponse(
      `Error getting document symbols: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for apply_workspace_edit tool
export async function handleApplyWorkspaceEdit(
  _fileService: FileService,
  args: {
    edit?: {
      changes: Record<
        string,
        Array<{
          range: {
            start: { line: number; character: number };
            end: { line: number; character: number };
          };
          newText: string;
        }>
      >;
    };
    changes?: Record<
      string,
      Array<{
        range: {
          start: { line: number; character: number };
          end: { line: number; character: number };
        };
        newText: string;
      }>
    >;
    validate_before_apply?: boolean;
    dry_run?: boolean;
  },
  lspClient?: import('../../lsp/lsp-client.js').LSPClient
) {
  // Support both formats: { changes: {...} } and { edit: { changes: {...} } }
  const changes = args.changes || args.edit?.changes;
  const { validate_before_apply = true, dry_run = false } = args;

  if (!changes) {
    return createMCPResponse('No changes provided. Please specify changes to apply.');
  }

  try {
    // Convert the input format to internal WorkspaceEdit format
    const workspaceEdit: WorkspaceEdit = {
      changes: {},
    };

    // Process each file's changes
    for (const [filePath, edits] of Object.entries(changes)) {
      // Convert file path to URI if it's not already one
      const uri = filePath.startsWith('file://') ? filePath : pathToUri(resolve(filePath));

      // Convert edits to internal TextEdit format
      const textEdits: TextEdit[] = edits.map((edit) => ({
        range: edit.range,
        newText: edit.newText,
      }));

      if (!workspaceEdit.changes) {
        workspaceEdit.changes = {};
      }
      workspaceEdit.changes[uri] = textEdits;
    }

    // Validate that we have at least one change
    if (!workspaceEdit.changes || Object.keys(workspaceEdit.changes).length === 0) {
      return createNoChangesResponse('workspace edit', 'the workspace edit is empty');
    }

    const fileCount = Object.keys(workspaceEdit.changes).length;
    const editCount = Object.values(workspaceEdit.changes).reduce(
      (sum, edits) => sum + edits.length,
      0
    );

    // Handle dry-run mode
    if (dry_run) {
      const fileList = Object.keys(workspaceEdit.changes)
        .map((uri) => {
          const path = uriToPath(uri);
          const edits = workspaceEdit.changes?.[uri];
          return `  - ${path}: ${edits?.length || 0} edit(s)`;
        })
        .join('\n');

      return createMCPResponse(
        `[DRY RUN] Would apply ${editCount} edit(s) to ${fileCount} file(s):\n${fileList}\n\nNo changes were applied.`
      );
    }

    // Skip capability validation for now - just attempt the edit
    const serverSupportsWorkspaceEdit = true; // Assume support for file-based edits
    const serverDescription = 'File-based workspace edit';

    // Apply the workspace edit with LSP synchronization
    const result = await applyWorkspaceEdit(workspaceEdit, {
      validateBeforeApply: true,
      createBackupFiles: false, // Disable backup file creation
      lspClient,
    });

    if (!result.success) {
      return createMCPResponse(
        `❌ **Workspace edit failed**\n\n**Error:** ${result.error || 'Unknown error'}\n\n**Files targeted:** ${fileCount}\n**Total edits:** ${editCount}\n\n*No changes were applied due to the error. All files remain unchanged.*`
      );
    }

    // Success response
    let response = '✅ **Workspace edit applied successfully**\n\n';
    const modifiedFiles = result.filesModified;
    response += `**Files modified:** ${modifiedFiles.length}\n`;
    response += `**Total edits applied:** ${editCount}\n\n`;

    if (modifiedFiles.length > 0) {
      response += '**Modified files:**\n';
      for (const file of modifiedFiles) {
        const filePath = file.startsWith('file://') ? uriToPath(file) : file;
        response += `• ${filePath}\n`;
      }
    }

    if (!serverSupportsWorkspaceEdit) {
      response += `\n⚠️ **Note:** ${serverDescription} doesn't fully support workspace edits, but changes were applied successfully using Codebuddy's built-in editor.`;
    }

    // Note: FileService doesn't currently create backup files

    response +=
      '\n\n*All changes were applied atomically. If any edit had failed, all changes would have been rolled back.*';

    return createMCPResponse(response);
  } catch (error) {
    return createMCPResponse(
      `Error applying workspace edit: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Register advanced tools with the central registry
registerTools(
  {
    get_code_actions: { handler: handleGetCodeActions, requiresService: 'file' },
    format_document: { handler: handleFormatDocument, requiresService: 'file' },
    search_workspace_symbols: { handler: handleSearchWorkspaceSymbols, requiresService: 'symbol' },
    get_document_symbols: { handler: handleGetDocumentSymbols, requiresService: 'symbol' },
    apply_workspace_edit: { handler: handleApplyWorkspaceEdit, requiresService: 'file' },
  },
  'advanced-handlers'
);

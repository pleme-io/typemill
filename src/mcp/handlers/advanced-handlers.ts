import { resolve } from 'node:path';
import { applyWorkspaceEdit } from '../../file-editor.js';
import type { TextEdit, WorkspaceEdit } from '../../file-editor.js';
import type { LSPClient } from '../../lsp-client.js';
import { pathToUri, uriToPath } from '../../path-utils.js';
import type { FileService } from '../../services/file-service.js';
import type { SymbolService } from '../../services/symbol-service.js';
import type { DocumentSymbol, SymbolInformation } from '../../types.js';
import {
  createLimitedSupportResponse,
  createMCPResponse,
  createUnsupportedFeatureResponse,
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
      return {
        content: [
          {
            type: 'text',
            text: `No code actions available for ${file_path}${range ? ` at lines ${range.start.line + 1}-${range.end.line + 1}` : ''}.`,
          },
        ],
      };
    }

    const actionDescriptions = codeActions
      .filter((action) => action && (action.title || action.kind))
      .map((action, index) => {
        if (action.title) {
          return `${index + 1}. ${action.title}${action.kind ? ` (${action.kind})` : ''}`;
        }
        return `${index + 1}. Code action (${action.kind || 'unknown'})`;
      });

    return {
      content: [
        {
          type: 'text',
          text: `Found ${codeActions.length} code action${codeActions.length === 1 ? '' : 's'} for ${file_path}:\n\n${actionDescriptions.join('\n')}\n\nNote: These actions show what's available but cannot be applied directly through this tool. Use your editor's code action functionality to apply them.`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error getting code actions: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
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
  }
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
      return {
        content: [
          {
            type: 'text',
            text: `No formatting changes needed for ${file_path}. The file is already properly formatted.`,
          },
        ],
      };
    }

    // Apply the formatting edits using the existing infrastructure
    const workspaceEdit = {
      changes: {
        [pathToUri(absolutePath)]: formatEdits,
      },
    };

    const editResult = await applyWorkspaceEdit(workspaceEdit);

    if (!editResult.success) {
      return {
        content: [
          {
            type: 'text',
            text: `Failed to apply formatting: ${editResult.error}`,
          },
        ],
      };
    }

    return {
      content: [
        {
          type: 'text',
          text: `✅ Successfully formatted ${file_path} with ${formatEdits.length} change${formatEdits.length === 1 ? '' : 's'}.`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error formatting document: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
  }
}

// Handler for search_workspace_symbols tool
export async function handleSearchWorkspaceSymbols(lspClient: LSPClient, args: { query: string }) {
  const { query } = args;

  try {
    const symbols = await lspClient.searchWorkspaceSymbols(query);

    if (symbols.length === 0) {
      return {
        content: [
          {
            type: 'text',
            text: `No symbols found matching "${query}". Try a different search term or ensure the language server is properly configured.`,
          },
        ],
      };
    }

    const symbolDescriptions = symbols
      .slice(0, 50) // Limit to first 50 results
      .map((symbol, index) => {
        const location = symbol.location;
        const filePath = uriToPath(location.uri);
        const line = location.range.start.line + 1;
        const character = location.range.start.character + 1;
        const symbolKind = symbol.kind ? lspClient.symbolKindToString(symbol.kind) : 'unknown';

        return `${index + 1}. ${symbol.name} (${symbolKind}) - ${filePath}:${line}:${character}`;
      });

    const resultText =
      symbols.length > 50
        ? `Found ${symbols.length} symbols matching "${query}" (showing first 50):\n\n${symbolDescriptions.join('\n')}`
        : `Found ${symbols.length} symbol${symbols.length === 1 ? '' : 's'} matching "${query}":\n\n${symbolDescriptions.join('\n')}`;

    return {
      content: [
        {
          type: 'text',
          text: resultText,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error searching workspace symbols: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
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
      return {
        content: [
          {
            type: 'text',
            text: `No symbols found in ${file_path}. The file may be empty or the language server may not support this file type.`,
          },
        ],
      };
    }

    // Check if we have DocumentSymbols (hierarchical) or SymbolInformation (flat)
    const isHierarchical = symbolService.isDocumentSymbolArray(symbols);

    let symbolDescriptions: string[];

    if (isHierarchical) {
      // Handle hierarchical DocumentSymbol[]
      const formatDocumentSymbol = (symbol: DocumentSymbol, indent = 0): string[] => {
        const prefix = '  '.repeat(indent);
        const line = symbol.range.start.line + 1;
        const character = symbol.range.start.character + 1;
        const symbolKind = symbolService.symbolKindToString(symbol.kind);

        const result = [`${prefix}${symbol.name} (${symbolKind}) - Line ${line}:${character}`];

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
        const line = symbol.location.range.start.line + 1;
        const character = symbol.location.range.start.character + 1;
        const symbolKind = symbol.kind ? symbolService.symbolKindToString(symbol.kind) : 'unknown';

        return `${index + 1}. ${symbol.name} (${symbolKind}) - Line ${line}:${character}`;
      });
    }

    return {
      content: [
        {
          type: 'text',
          text: `Document outline for ${file_path}:\n\n${symbolDescriptions.join('\n')}`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error getting document symbols: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
  }
}

// Handler for get_folding_ranges tool
export async function handleGetFoldingRanges(lspClient: LSPClient, args: { file_path: string }) {
  const { file_path } = args;
  const absolutePath = resolve(file_path);

  try {
    // Check if server supports folding ranges
    const validation = await lspClient.validateCapabilities(absolutePath, ['foldingRangeProvider']);
    if (!validation.supported) {
      return createUnsupportedFeatureResponse(
        'Folding Ranges',
        validation.serverDescription,
        validation.missing,
        [
          'Use get_document_symbols to understand code structure',
          'Look at indentation patterns in the code',
          'Use selection ranges for hierarchical code block understanding',
        ]
      );
    }

    const foldingRanges = await lspClient.getFoldingRanges(absolutePath);

    if (foldingRanges.length === 0) {
      return createMCPResponse(
        `No folding ranges found in ${file_path}. The file may not have collapsible code blocks.`
      );
    }

    const rangeDescriptions = foldingRanges.map((range, index) => {
      const startLine = range.startLine + 1; // Convert to 1-indexed
      const endLine = range.endLine + 1;
      const kind = range.kind || 'code';
      const characterInfo =
        range.startCharacter !== undefined && range.endCharacter !== undefined
          ? ` (chars ${range.startCharacter}-${range.endCharacter})`
          : '';

      return `${index + 1}. **${kind}** block: Lines ${startLine}-${endLine}${characterInfo}${range.collapsedText ? ` ("${range.collapsedText}")` : ''}`;
    });

    const kindCount = foldingRanges.reduce(
      (acc, range) => {
        const kind = range.kind || 'code';
        acc[kind] = (acc[kind] || 0) + 1;
        return acc;
      },
      {} as Record<string, number>
    );

    const kindSummary = Object.entries(kindCount)
      .map(([kind, count]) => `${count} ${kind}`)
      .join(', ');

    const response = `## Folding Ranges for ${file_path}\n\n**Found ${foldingRanges.length} foldable regions:** ${kindSummary}\n\n${rangeDescriptions.join('\n')}\n\n*Folding ranges show logical code blocks that can be collapsed for better code navigation and understanding.*`;

    return createMCPResponse(response);
  } catch (error) {
    if (error instanceof Error && error.message.includes('not supported')) {
      const serverInfo = await lspClient.getCapabilityInfo(absolutePath);
      return createLimitedSupportResponse(
        'Folding Ranges',
        'Current Language Server',
        'Server may not fully support folding ranges or the file has no collapsible regions',
        `Server capabilities: ${serverInfo}`
      );
    }

    return createMCPResponse(
      `Error getting folding ranges: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for get_document_links tool
export async function handleGetDocumentLinks(lspClient: LSPClient, args: { file_path: string }) {
  const { file_path } = args;
  const absolutePath = resolve(file_path);

  try {
    // Check if server supports document links
    const validation = await lspClient.validateCapabilities(absolutePath, ['documentLinkProvider']);
    if (!validation.supported) {
      return createUnsupportedFeatureResponse(
        'Document Links',
        validation.serverDescription,
        validation.missing,
        [
          'Look for import statements and URLs manually in the code',
          'Use find_references to track symbol usage across files',
          'Check package.json or similar files for external dependencies',
        ]
      );
    }

    const documentLinks = await lspClient.getDocumentLinks(absolutePath);

    if (documentLinks.length === 0) {
      return createMCPResponse(
        `No document links found in ${file_path}. The file may not contain URLs, imports, or other linkable references.`
      );
    }

    const linkDescriptions = documentLinks.map((link, index) => {
      const startLine = link.range.start.line + 1; // Convert to 1-indexed
      const startChar = link.range.start.character + 1;
      const endLine = link.range.end.line + 1;
      const endChar = link.range.end.character + 1;

      let description = `${index + 1}. **Link** at Line ${startLine}:${startChar}`;
      if (startLine !== endLine || startChar !== endChar) {
        description += ` to ${endLine}:${endChar}`;
      }

      if (link.target) {
        description += `\n   Target: ${link.target}`;
      }

      if (link.tooltip) {
        description += `\n   Info: ${link.tooltip}`;
      }

      return description;
    });

    // Categorize links by type for better understanding
    const categories = {
      urls: documentLinks.filter((link) => link.target?.startsWith('http')),
      files: documentLinks.filter((link) => link.target?.startsWith('file:')),
      packages: documentLinks.filter(
        (link) =>
          link.target?.includes('pkg.go.dev') ||
          link.target?.includes('docs.rs') ||
          link.target?.includes('npmjs.com')
      ),
      other: documentLinks.filter(
        (link) => link.target && !link.target.startsWith('http') && !link.target.startsWith('file:')
      ),
    };

    let categorySummary = '';
    if (categories.urls.length > 0) categorySummary += `${categories.urls.length} URLs, `;
    if (categories.files.length > 0) categorySummary += `${categories.files.length} files, `;
    if (categories.packages.length > 0)
      categorySummary += `${categories.packages.length} packages, `;
    if (categories.other.length > 0) categorySummary += `${categories.other.length} other links, `;

    categorySummary = categorySummary.replace(/, $/, ''); // Remove trailing comma

    const response = `## Document Links for ${file_path}\n\n**Found ${documentLinks.length} links:** ${categorySummary}\n\n${linkDescriptions.join('\n\n')}\n\n*Document links help navigate between related files, external documentation, and web resources. Different language servers provide different types of links.*`;

    return createMCPResponse(response);
  } catch (error) {
    if (error instanceof Error && error.message.includes('not supported')) {
      const serverInfo = await lspClient.getCapabilityInfo(absolutePath);
      return createLimitedSupportResponse(
        'Document Links',
        'Current Language Server',
        'Server may not fully support document links or the file contains no linkable content',
        `Server capabilities: ${serverInfo}`
      );
    }

    return createMCPResponse(
      `Error getting document links: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for apply_workspace_edit tool
export async function handleApplyWorkspaceEdit(
  fileService: FileService,
  args: {
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
    validate_before_apply?: boolean;
  }
) {
  const { changes, validate_before_apply = true } = args;

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
      return createMCPResponse(
        'No changes provided. Please specify at least one file with edits to apply.'
      );
    }

    const fileCount = Object.keys(workspaceEdit.changes).length;
    const editCount = Object.values(workspaceEdit.changes).reduce(
      (sum, edits) => sum + edits.length,
      0
    );

    // Skip capability validation for now - just attempt the edit
    const serverSupportsWorkspaceEdit = true; // Assume support for file-based edits
    const serverDescription = 'File-based workspace edit';

    // Apply the workspace edit using the file service
    const result = await fileService.applyWorkspaceEdit({
      changes: workspaceEdit.changes,
    });

    if (!result.applied) {
      return createMCPResponse(
        `❌ **Workspace edit failed**\n\n**Error:** ${result.failureReason || 'Unknown error'}\n\n**Files targeted:** ${fileCount}\n**Total edits:** ${editCount}\n\n*No changes were applied due to the error. All files remain unchanged.*`
      );
    }

    // Success response
    let response = '✅ **Workspace edit applied successfully**\n\n';
    const modifiedFiles = Object.keys(workspaceEdit.changes);
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
      response += `\n⚠️ **Note:** ${serverDescription} doesn't fully support workspace edits, but changes were applied successfully using CCLSP's built-in editor.`;
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

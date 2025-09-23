import { resolve } from 'node:path';
import { applyWorkspaceEdit, type WorkspaceEdit } from '../../core/file-operations/editor.js';
import { uriToPath } from '../../core/file-operations/path-utils.js';
import type { SymbolService } from '../../services/lsp/symbol-service.js';
import { registerTools } from '../tool-registry.js';
import {
  createContextualErrorResponse,
  createFileModificationResponse,
  createMCPResponse,
  createNoChangesResponse,
  createNoResultsResponse,
} from '../utils.js';

// Handler for find_definition tool
export async function handleFindDefinition(
  symbolService: SymbolService,
  args: { file_path: string; symbol_name: string; symbol_kind?: string }
) {
  const { file_path, symbol_name, symbol_kind } = args;
  const absolutePath = resolve(file_path);

  const symbolMatches = await symbolService.findSymbolMatches(
    absolutePath,
    symbol_name,
    symbol_kind
  );

  process.stderr.write(
    `[DEBUG find_definition] Found ${symbolMatches.length} symbol matches for "${symbol_name}"\n`
  );

  if (symbolMatches.length === 0) {
    return createNoResultsResponse(
      'symbols',
      `"${symbol_name}"${symbol_kind ? ` (${symbol_kind})` : ''} in ${file_path}`,
      ['Please verify the symbol name and ensure the language server is properly configured.']
    );
  }

  const results = [];
  for (const match of symbolMatches) {
    process.stderr.write(
      `[DEBUG find_definition] Processing match: ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${match.position.line}:${match.position.character}\n`
    );
    try {
      const locations = await symbolService.findDefinition(absolutePath, match.position);
      process.stderr.write(
        `[DEBUG find_definition] findDefinition returned ${locations.length} locations\n`
      );

      if (locations.length > 0) {
        const locationResults = locations
          .map((loc) => {
            const filePath = uriToPath(loc.uri);
            const { start, end } = loc.range;
            return `${filePath}:${start.line + 1}:${start.character + 1}`;
          })
          .join('\n');

        results.push(
          `Results for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${file_path}:${match.position.line + 1}:${match.position.character + 1}:\n${locationResults}`
        );
      } else {
        results.push(
          `No definition found for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${file_path}:${match.position.line + 1}:${match.position.character + 1}`
        );
      }
    } catch (error) {
      results.push(
        `Error finding definition for ${match.name}: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  if (results.length === 0) {
    const responseText = 'No definitions found for the specified symbol.';
    return createMCPResponse(responseText);
  }

  const responseText = results.join('\n\n');
  return createMCPResponse(responseText);
}

// Handler for find_references tool
export async function handleFindReferences(
  symbolService: SymbolService,
  args: {
    file_path: string;
    symbol_name: string;
    symbol_kind?: string;
    include_declaration?: boolean;
  }
) {
  const { file_path, symbol_name, symbol_kind, include_declaration = true } = args;
  const absolutePath = resolve(file_path);

  const symbolMatches = await symbolService.findSymbolMatches(
    absolutePath,
    symbol_name,
    symbol_kind
  );

  process.stderr.write(
    `[DEBUG find_references] Found ${symbolMatches.length} symbol matches for "${symbol_name}"\n`
  );

  if (symbolMatches.length === 0) {
    return createNoResultsResponse(
      'symbols',
      `"${symbol_name}"${symbol_kind ? ` (${symbol_kind})` : ''} in ${file_path}`,
      ['Please verify the symbol name and ensure the language server is properly configured.']
    );
  }

  const results = [];
  for (const match of symbolMatches) {
    process.stderr.write(
      `[DEBUG find_references] Processing match: ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${match.position.line}:${match.position.character}\n`
    );
    try {
      const locations = await symbolService.findReferences(
        absolutePath,
        match.position,
        include_declaration
      );
      process.stderr.write(
        `[DEBUG find_references] findReferences returned ${locations.length} locations\n`
      );

      if (locations.length > 0) {
        const locationResults = locations
          .map((loc) => {
            const filePath = uriToPath(loc.uri);
            const { start, end } = loc.range;
            return `${filePath}:${start.line + 1}:${start.character + 1}`;
          })
          .join('\n');

        results.push(
          `References for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${file_path}:${match.position.line + 1}:${match.position.character + 1}:\n${locationResults}`
        );
      } else {
        results.push(
          `No references found for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${file_path}:${match.position.line + 1}:${match.position.character + 1}`
        );
      }
    } catch (error) {
      results.push(
        `Error finding references for ${match.name}: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  if (results.length === 0) {
    const responseText = 'No references found for the specified symbol.';
    return createMCPResponse(responseText);
  }

  const responseText = results.join('\n\n');
  return createMCPResponse(responseText);
}

// Handler for rename_symbol tool
export async function handleRenameSymbol(
  symbolService: SymbolService,
  args: {
    file_path: string;
    symbol_name: string;
    symbol_kind?: string;
    new_name: string;
    dry_run?: boolean;
  },
  lspClient?: import('../../lsp/lsp-client.js').LSPClient
) {
  const { file_path, symbol_name, symbol_kind, new_name, dry_run = false } = args;
  const absolutePath = resolve(file_path);

  const symbolMatches = await symbolService.findSymbolMatches(
    absolutePath,
    symbol_name,
    symbol_kind
  );

  if (symbolMatches.length === 0) {
    const responseText = `No symbols found with name "${symbol_name}"${symbol_kind ? ` and kind "${symbol_kind}"` : ''} in ${file_path}. Please verify the symbol name and ensure the language server is properly configured.`;

    return createMCPResponse(responseText);
  }

  if (symbolMatches.length > 1) {
    const matchDescriptions = symbolMatches
      .map(
        (match, index) =>
          `${index + 1}. ${match.name} (${symbolService.symbolKindToString(match.kind)}) at line ${match.position.line + 1}, character ${match.position.character + 1}`
      )
      .join('\n');

    const responseText = `Multiple symbols found with name "${symbol_name}". Please use rename_symbol_strict to specify which one to rename:\n\n${matchDescriptions}`;

    return createMCPResponse(responseText);
  }

  // Single match - proceed with rename
  const match = symbolMatches[0];
  if (!match) {
    throw new Error('Symbol match is undefined');
  }

  // Check if the new name is the same as the old name
  if (symbol_name === new_name) {
    return createNoChangesResponse(
      'rename symbol',
      `symbol "${symbol_name}" is already named "${new_name}"`
    );
  }

  try {
    const workspaceEdit = await symbolService.renameSymbol(
      absolutePath,
      match.position,
      new_name,
      dry_run
    );

    if (!workspaceEdit.changes || Object.keys(workspaceEdit.changes).length === 0) {
      return createNoChangesResponse(`renaming "${symbol_name}" to "${new_name}"`);
    }

    const changedFileCount = Object.keys(workspaceEdit.changes).length;

    if (dry_run) {
      return createMCPResponse(
        `[DRY RUN] Would rename "${symbol_name}" to "${new_name}" across ${changedFileCount} file${changedFileCount === 1 ? '' : 's'}`
      );
    }

    const editResult = await applyWorkspaceEdit(workspaceEdit, {
      validateBeforeApply: true,
      lspClient,
    });

    if (!editResult.success) {
      return createMCPResponse(`Failed to rename symbol: ${editResult.error}`);
    }

    return createFileModificationResponse(`renamed "${symbol_name}" to "${new_name}"`, file_path, {
      fileCount: changedFileCount,
    });
  } catch (error) {
    return createContextualErrorResponse(error, {
      operation: 'rename symbol',
      filePath: file_path,
      suggestions: [
        'Ensure the symbol exists in the file',
        'Check that the language server supports renaming',
        'Try using rename_symbol_strict for precise positioning',
      ],
    });
  }
}

// Handler for rename_symbol_strict tool
export async function handleRenameSymbolStrict(
  symbolService: SymbolService,
  args: {
    file_path: string;
    line: number;
    character: number;
    new_name: string;
    dry_run?: boolean;
  },
  lspClient?: import('../../lsp/lsp-client.js').LSPClient
) {
  const { file_path, line, character, new_name, dry_run = false } = args;
  const absolutePath = resolve(file_path);

  // Convert 1-indexed to 0-indexed for LSP
  const position = { line: line - 1, character: character - 1 };

  try {
    const workspaceEdit = await symbolService.renameSymbol(
      absolutePath,
      position,
      new_name,
      dry_run
    );

    if (!workspaceEdit.changes || Object.keys(workspaceEdit.changes).length === 0) {
      return createNoChangesResponse(
        `renaming symbol at ${file_path}:${line}:${character} to "${new_name}"`
      );
    }

    const changedFileCount = Object.keys(workspaceEdit.changes).length;

    if (dry_run) {
      return createMCPResponse(
        `[DRY RUN] Would rename symbol at ${file_path}:${line}:${character} to "${new_name}" across ${changedFileCount} file${changedFileCount === 1 ? '' : 's'}`
      );
    }

    const editResult = await applyWorkspaceEdit(workspaceEdit, {
      validateBeforeApply: true,
      lspClient,
    });

    if (!editResult.success) {
      return createMCPResponse(`Failed to rename symbol: ${editResult.error}`);
    }

    return createFileModificationResponse(
      `renamed symbol at ${file_path}:${line}:${character} to "${new_name}"`,
      file_path,
      { fileCount: changedFileCount }
    );
  } catch (error) {
    return createContextualErrorResponse(error, {
      operation: 'rename symbol at specific position',
      filePath: file_path,
      suggestions: [
        'Verify the line and character position are correct',
        'Check that there is a symbol at the specified position',
        'Ensure the language server supports renaming',
      ],
    });
  }
}

/**
 * Internal helper for getting raw WorkspaceEdit data from symbol rename operations
 * Used by orchestration handlers for atomic operations
 */
export async function getRenameSymbolWorkspaceEdit(
  symbolService: SymbolService,
  args: {
    file_path: string;
    symbol_name: string;
    symbol_kind?: string;
    new_name: string;
  }
): Promise<{ success: boolean; workspaceEdit?: WorkspaceEdit; error?: string }> {
  const { file_path, symbol_name, symbol_kind, new_name } = args;

  try {
    const absolutePath = resolve(file_path);
    const symbolMatches = await symbolService.findSymbolMatches(
      absolutePath,
      symbol_name,
      symbol_kind
    );

    if (symbolMatches.length === 0) {
      return {
        success: false,
        error: `No symbols found with name "${symbol_name}"${symbol_kind ? ` and kind "${symbol_kind}"` : ''} in ${file_path}`,
      };
    }

    if (symbolMatches.length > 1) {
      return {
        success: false,
        error: `Multiple symbols found with name "${symbol_name}". Use rename_symbol_strict for precise positioning.`,
      };
    }

    // Single match - proceed with rename
    const match = symbolMatches[0];
    if (!match) {
      return { success: false, error: 'Symbol match is undefined' };
    }

    // Check if the new name is the same as the old name
    if (symbol_name === new_name) {
      return {
        success: true,
        workspaceEdit: { changes: {} }, // Empty workspace edit for no-op
      };
    }

    const workspaceEdit = await symbolService.renameSymbol(
      absolutePath,
      match.position,
      new_name,
      true // Always dry run for workspace edit extraction
    );

    return { success: true, workspaceEdit };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

/**
 * Internal helper for getting raw WorkspaceEdit data from strict symbol rename operations
 * Used by orchestration handlers for atomic operations
 */
export async function getRenameSymbolWorkspaceEditStrict(
  symbolService: SymbolService,
  args: {
    file_path: string;
    line: number;
    character: number;
    new_name: string;
  }
): Promise<{ success: boolean; workspaceEdit?: WorkspaceEdit; error?: string }> {
  const { file_path, line, character, new_name } = args;

  try {
    const absolutePath = resolve(file_path);
    // Convert 1-indexed to 0-indexed for LSP
    const position = { line: line - 1, character: character - 1 };

    const workspaceEdit = await symbolService.renameSymbol(
      absolutePath,
      position,
      new_name,
      true // Always dry run for workspace edit extraction
    );

    return { success: true, workspaceEdit };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

// Register core tools with the central registry
registerTools(
  {
    find_definition: { handler: handleFindDefinition, requiresService: 'symbol' },
    find_references: { handler: handleFindReferences, requiresService: 'symbol' },
    rename_symbol: { handler: handleRenameSymbol, requiresService: 'symbol' },
    rename_symbol_strict: { handler: handleRenameSymbolStrict, requiresService: 'symbol' },
  },
  'core-handlers'
);

import { resolve } from 'node:path';
import { logger } from '../../../../../server/src/core/diagnostics/logger.js';
import {
  applyWorkspaceEdit,
  type WorkspaceEdit,
} from '../../../../../server/src/core/file-operations/editor.js';
import { registerTools } from '../../../../../server/src/mcp/tool-registry.js';
import {
  createContextualErrorResponse,
  createFileModificationResponse,
  createMCPResponse,
  createNoChangesResponse,
  createNoResultsResponse,
} from '../../../../../server/src/mcp/utils.js';
import {
  assertValidFilePath,
  assertValidSymbolName,
  formatFileLocation,
  formatLSPLocation,
  measureAndTrack,
  toHumanPosition,
  toLSPPosition,
  ValidationError,
} from '../../../../core/src/utils/index.js';
import type { SymbolService } from '../../services/lsp/symbol-service.js';

// Handler for find_definition tool
export async function handleFindDefinition(
  symbolService: SymbolService,
  args: { file_path: string; symbol_name: string; symbol_kind?: string }
) {
  const { file_path, symbol_name, symbol_kind } = args;

  return measureAndTrack(
    'find_definition',
    async () => {
      // Validate inputs
      try {
        assertValidFilePath(file_path);
        assertValidSymbolName(symbol_name);
      } catch (error) {
        if (error instanceof ValidationError) {
          return createContextualErrorResponse(error, {
            operation: 'find_definition',
            filePath: file_path,
          });
        }
        throw error;
      }

      const absolutePath = resolve(file_path);

      const symbolMatches = await symbolService.findSymbolMatches(
        absolutePath,
        symbol_name,
        symbol_kind
      );

      logger.debug('Symbol matches found', {
        tool: 'find_definition',
        symbol_name,
        match_count: symbolMatches.length,
        file_path,
      });

      if (symbolMatches.length === 0) {
        return createNoResultsResponse(
          'symbols',
          `"${symbol_name}"${symbol_kind ? ` (${symbol_kind})` : ''} in ${file_path}`,
          ['Please verify the symbol name and ensure the language server is properly configured.']
        );
      }

      const results = [];
      for (const match of symbolMatches) {
        const humanPos = toHumanPosition(match.position);
        logger.debug('Processing symbol match', {
          tool: 'find_definition',
          match_name: match.name,
          match_kind: symbolService.symbolKindToString(match.kind),
          position: formatFileLocation(file_path, humanPos),
        });
        try {
          const locations = await symbolService.findDefinition(absolutePath, match.position);
          logger.debug('Definition search completed', {
            tool: 'find_definition',
            match_name: match.name,
            location_count: locations.length,
          });

          if (locations.length > 0) {
            const locationResults = locations.map((loc) => formatLSPLocation(loc)).join('\n');

            const matchHumanPos = toHumanPosition(match.position);
            results.push(
              `Results for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${formatFileLocation(file_path, matchHumanPos)}:\n${locationResults}`
            );
          } else {
            const matchHumanPos = toHumanPosition(match.position);
            results.push(
              `No definition found for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${formatFileLocation(file_path, matchHumanPos)}`
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
    },
    {
      context: { file_path, symbol_name, symbol_kind },
    }
  );
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

  return measureAndTrack(
    'find_references',
    async () => {
      // Validate inputs
      try {
        assertValidFilePath(file_path);
        assertValidSymbolName(symbol_name);
      } catch (error) {
        if (error instanceof ValidationError) {
          return createContextualErrorResponse(error, {
            operation: 'find_references',
            filePath: file_path,
          });
        }
        throw error;
      }

      const absolutePath = resolve(file_path);

      const symbolMatches = await symbolService.findSymbolMatches(
        absolutePath,
        symbol_name,
        symbol_kind
      );

      logger.debug('Symbol matches found', {
        tool: 'find_references',
        symbol_name,
        match_count: symbolMatches.length,
        file_path,
        include_declaration,
      });

      if (symbolMatches.length === 0) {
        return createNoResultsResponse(
          'symbols',
          `"${symbol_name}"${symbol_kind ? ` (${symbol_kind})` : ''} in ${file_path}`,
          ['Please verify the symbol name and ensure the language server is properly configured.']
        );
      }

      const results = [];
      for (const match of symbolMatches) {
        const humanPos = toHumanPosition(match.position);
        logger.debug('Processing symbol match', {
          tool: 'find_references',
          match_name: match.name,
          match_kind: symbolService.symbolKindToString(match.kind),
          position: formatFileLocation(file_path, humanPos),
        });
        try {
          const locations = await symbolService.findReferences(
            absolutePath,
            match.position,
            include_declaration
          );
          logger.debug('References search completed', {
            tool: 'find_references',
            match_name: match.name,
            location_count: locations.length,
          });

          if (locations.length > 0) {
            const locationResults = locations.map((loc) => formatLSPLocation(loc)).join('\n');

            const matchHumanPos = toHumanPosition(match.position);
            results.push(
              `References for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${formatFileLocation(file_path, matchHumanPos)}:\n${locationResults}`
            );
          } else {
            const matchHumanPos = toHumanPosition(match.position);
            results.push(
              `No references found for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${formatFileLocation(file_path, matchHumanPos)}`
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
    },
    {
      context: { file_path, symbol_name, symbol_kind, include_declaration },
    }
  );
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
  lspClient?: import('../../../../../server/src/lsp/lsp-client.js').LSPClient
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
      .map((match, index) => {
        const humanPos = toHumanPosition(match.position);
        return `${index + 1}. ${match.name} (${symbolService.symbolKindToString(match.kind)}) at line ${humanPos.line}, character ${humanPos.character}`;
      })
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
      createBackupFiles: false, // Disable backup file creation
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
  lspClient?: import('../../../../../server/src/lsp/lsp-client.js').LSPClient
) {
  const { file_path, line, character, new_name, dry_run = false } = args;
  const absolutePath = resolve(file_path);

  // Convert 1-indexed to 0-indexed for LSP
  const humanPos = { line, character };
  const position = toLSPPosition(humanPos);

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
      createBackupFiles: false, // Disable backup file creation
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
    const humanPos = { line, character };
    const position = toLSPPosition(humanPos);

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

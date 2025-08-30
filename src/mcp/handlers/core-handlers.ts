import { resolve } from 'node:path';
import { applyWorkspaceEdit } from '../../file-editor.js';
import { uriToPath } from '../../path-utils.js';
import type { SymbolService } from '../../services/symbol-service.js';

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
  const warning = undefined; // Remove warning handling for now

  process.stderr.write(
    `[DEBUG find_definition] Found ${symbolMatches.length} symbol matches for "${symbol_name}"\n`
  );

  if (symbolMatches.length === 0) {
    return {
      content: [
        {
          type: 'text',
          text: `No symbols found with name "${symbol_name}"${symbol_kind ? ` and kind "${symbol_kind}"` : ''} in ${file_path}. Please verify the symbol name and ensure the language server is properly configured.`,
        },
      ],
    };
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
    const responseText = warning ? warning : 'No definitions found for the specified symbol.';
    return {
      content: [
        {
          type: 'text',
          text: responseText,
        },
      ],
    };
  }

  const responseText = warning ? `${warning}\n\n${results.join('\n\n')}` : results.join('\n\n');

  return {
    content: [
      {
        type: 'text',
        text: responseText,
      },
    ],
  };
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
  const warning = undefined; // Remove warning handling for now

  process.stderr.write(
    `[DEBUG find_references] Found ${symbolMatches.length} symbol matches for "${symbol_name}"\n`
  );

  if (symbolMatches.length === 0) {
    return {
      content: [
        {
          type: 'text',
          text: `No symbols found with name "${symbol_name}"${symbol_kind ? ` and kind "${symbol_kind}"` : ''} in ${file_path}. Please verify the symbol name and ensure the language server is properly configured.`,
        },
      ],
    };
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
    const responseText = warning
      ? `${warning}\n\nNo references found for the specified symbol.`
      : 'No references found for the specified symbol.';
    return {
      content: [
        {
          type: 'text',
          text: responseText,
        },
      ],
    };
  }

  const responseText = warning ? `${warning}\n\n${results.join('\n\n')}` : results.join('\n\n');

  return {
    content: [
      {
        type: 'text',
        text: responseText,
      },
    ],
  };
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
  }
) {
  const { file_path, symbol_name, symbol_kind, new_name, dry_run = false } = args;
  const absolutePath = resolve(file_path);

  const symbolMatches = await symbolService.findSymbolMatches(
    absolutePath,
    symbol_name,
    symbol_kind
  );
  const warning = undefined; // Remove warning handling for now

  if (symbolMatches.length === 0) {
    const responseText = warning
      ? `${warning}\n\nNo symbols found with name "${symbol_name}"${symbol_kind ? ` and kind "${symbol_kind}"` : ''} in ${file_path}. Please verify the symbol name and ensure the language server is properly configured.`
      : `No symbols found with name "${symbol_name}"${symbol_kind ? ` and kind "${symbol_kind}"` : ''} in ${file_path}. Please verify the symbol name and ensure the language server is properly configured.`;

    return {
      content: [
        {
          type: 'text',
          text: responseText,
        },
      ],
    };
  }

  if (symbolMatches.length > 1) {
    const matchDescriptions = symbolMatches
      .map(
        (match, index) =>
          `${index + 1}. ${match.name} (${symbolService.symbolKindToString(match.kind)}) at line ${match.position.line + 1}, character ${match.position.character + 1}`
      )
      .join('\n');

    const responseText = warning
      ? `${warning}\n\nMultiple symbols found with name "${symbol_name}". Please use rename_symbol_strict to specify which one to rename:\n\n${matchDescriptions}`
      : `Multiple symbols found with name "${symbol_name}". Please use rename_symbol_strict to specify which one to rename:\n\n${matchDescriptions}`;

    return {
      content: [
        {
          type: 'text',
          text: responseText,
        },
      ],
    };
  }

  // Single match - proceed with rename
  const match = symbolMatches[0];
  if (!match) {
    throw new Error('Symbol match is undefined');
  }
  try {
    const workspaceEdit = await symbolService.renameSymbol(
      absolutePath,
      match.position,
      new_name,
      dry_run
    );

    if (!workspaceEdit.changes || Object.keys(workspaceEdit.changes).length === 0) {
      return {
        content: [
          {
            type: 'text',
            text: `No changes needed for renaming "${symbol_name}" to "${new_name}".`,
          },
        ],
      };
    }

    const changedFileCount = Object.keys(workspaceEdit.changes).length;

    if (dry_run) {
      return {
        content: [
          {
            type: 'text',
            text: `[DRY RUN] Would rename "${symbol_name}" to "${new_name}" across ${changedFileCount} file${changedFileCount === 1 ? '' : 's'}`,
          },
        ],
      };
    }

    const editResult = await applyWorkspaceEdit(workspaceEdit, {
      validateBeforeApply: true,
    });

    if (!editResult.success) {
      return {
        content: [
          {
            type: 'text',
            text: `Failed to rename symbol: ${editResult.error}`,
          },
        ],
      };
    }

    return {
      content: [
        {
          type: 'text',
          text: `✅ Successfully renamed "${symbol_name}" to "${new_name}" across ${changedFileCount} file${changedFileCount === 1 ? '' : 's'}`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error renaming symbol: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
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
  }
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
      return {
        content: [
          {
            type: 'text',
            text: `No changes needed for renaming symbol at ${file_path}:${line}:${character} to "${new_name}".`,
          },
        ],
      };
    }

    const changedFileCount = Object.keys(workspaceEdit.changes).length;

    if (dry_run) {
      return {
        content: [
          {
            type: 'text',
            text: `[DRY RUN] Would rename symbol at ${file_path}:${line}:${character} to "${new_name}" across ${changedFileCount} file${changedFileCount === 1 ? '' : 's'}`,
          },
        ],
      };
    }

    const editResult = await applyWorkspaceEdit(workspaceEdit, {
      validateBeforeApply: true,
    });

    if (!editResult.success) {
      return {
        content: [
          {
            type: 'text',
            text: `Failed to rename symbol: ${editResult.error}`,
          },
        ],
      };
    }

    return {
      content: [
        {
          type: 'text',
          text: `✅ Successfully renamed symbol at ${file_path}:${line}:${character} to "${new_name}" across ${changedFileCount} file${changedFileCount === 1 ? '' : 's'}`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error renaming symbol: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
  }
}

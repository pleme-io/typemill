import type { LSPClient } from '../lsp-client.js';
import { pathToUri } from '../path-utils.js';
import type { LSPLocation, Location, Position } from '../types.js';

// Type definitions for the methods in this module
export interface CoreMethodsContext {
  getServer: LSPClient['getServer'];
  ensureFileOpen: LSPClient['ensureFileOpen'];
  sendRequest: LSPClient['sendRequest'];
}

export async function findDefinition(
  context: CoreMethodsContext,
  filePath: string,
  position: Position
): Promise<Location[]> {
  process.stderr.write(
    `[DEBUG findDefinition] Requesting definition for ${filePath} at ${position.line}:${position.character}\n`
  );

  const serverState = await context.getServer(filePath);

  // Wait for the server to be fully initialized
  await serverState.initializationPromise;

  // Ensure the file is opened and synced with the LSP server
  await context.ensureFileOpen(serverState, filePath);

  process.stderr.write('[DEBUG findDefinition] Sending textDocument/definition request\n');
  const result = await context.sendRequest(serverState.process, 'textDocument/definition', {
    textDocument: { uri: pathToUri(filePath) },
    position,
  });

  process.stderr.write(
    `[DEBUG findDefinition] Result type: ${typeof result}, isArray: ${Array.isArray(result)}\n`
  );

  if (Array.isArray(result)) {
    process.stderr.write(`[DEBUG findDefinition] Array result with ${result.length} locations\n`);
    if (result.length > 0) {
      process.stderr.write(
        `[DEBUG findDefinition] First location: ${JSON.stringify(result[0], null, 2)}\n`
      );
    }
    return result.map((loc: LSPLocation) => ({
      uri: loc.uri,
      range: loc.range,
    }));
  }
  if (result && typeof result === 'object' && 'uri' in result) {
    process.stderr.write(
      `[DEBUG findDefinition] Single location result: ${JSON.stringify(result, null, 2)}\n`
    );
    const location = result as LSPLocation;
    return [
      {
        uri: location.uri,
        range: location.range,
      },
    ];
  }

  process.stderr.write('[DEBUG findDefinition] No definition found or unexpected result format\n');
  return [];
}

export async function findReferences(
  context: CoreMethodsContext,
  filePath: string,
  position: Position,
  includeDeclaration = true
): Promise<Location[]> {
  const serverState = await context.getServer(filePath);

  // Wait for the server to be fully initialized
  await serverState.initializationPromise;

  // Ensure the file is opened and synced with the LSP server
  await context.ensureFileOpen(serverState, filePath);

  process.stderr.write(
    `[DEBUG] findReferences for ${filePath} at ${position.line}:${position.character}, includeDeclaration: ${includeDeclaration}\n`
  );

  const result = await context.sendRequest(serverState.process, 'textDocument/references', {
    textDocument: { uri: pathToUri(filePath) },
    position,
    context: { includeDeclaration },
  });

  process.stderr.write(
    `[DEBUG] findReferences result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : 'N/A'}\n`
  );

  if (result && Array.isArray(result) && result.length > 0) {
    process.stderr.write(`[DEBUG] First reference: ${JSON.stringify(result[0], null, 2)}\n`);
  } else if (result === null || result === undefined) {
    process.stderr.write('[DEBUG] findReferences returned null/undefined\n');
  } else {
    process.stderr.write(
      `[DEBUG] findReferences returned unexpected result: ${JSON.stringify(result)}\n`
    );
  }

  if (Array.isArray(result)) {
    return result.map((loc: LSPLocation) => ({
      uri: loc.uri,
      range: loc.range,
    }));
  }

  return [];
}

export async function renameSymbol(
  context: CoreMethodsContext,
  filePath: string,
  position: Position,
  newName: string
): Promise<{
  changes?: Record<string, Array<{ range: { start: Position; end: Position }; newText: string }>>;
}> {
  process.stderr.write(
    `[DEBUG renameSymbol] Requesting rename for ${filePath} at ${position.line}:${position.character} to "${newName}"\n`
  );

  const serverState = await context.getServer(filePath);

  // Wait for the server to be fully initialized
  await serverState.initializationPromise;

  // Ensure the file is opened and synced with the LSP server
  await context.ensureFileOpen(serverState, filePath);

  process.stderr.write('[DEBUG renameSymbol] Sending textDocument/rename request\n');
  const result = await context.sendRequest(serverState.process, 'textDocument/rename', {
    textDocument: { uri: pathToUri(filePath) },
    position,
    newName,
  });

  process.stderr.write(
    `[DEBUG renameSymbol] Result type: ${typeof result}, hasChanges: ${result && typeof result === 'object' && 'changes' in result}, hasDocumentChanges: ${result && typeof result === 'object' && 'documentChanges' in result}\n`
  );

  if (result && typeof result === 'object') {
    // Handle the 'changes' format (older LSP servers)
    if ('changes' in result) {
      const workspaceEdit = result as {
        changes: Record<
          string,
          Array<{ range: { start: Position; end: Position }; newText: string }>
        >;
      };

      const changeCount = Object.keys(workspaceEdit.changes || {}).length;
      process.stderr.write(
        `[DEBUG renameSymbol] WorkspaceEdit has changes for ${changeCount} files\n`
      );

      return workspaceEdit;
    }

    // Handle the 'documentChanges' format (modern LSP servers like gopls)
    if ('documentChanges' in result) {
      const workspaceEdit = result as {
        documentChanges?: Array<{
          textDocument: { uri: string; version?: number };
          edits: Array<{ range: { start: Position; end: Position }; newText: string }>;
        }>;
      };

      process.stderr.write(
        `[DEBUG renameSymbol] WorkspaceEdit has documentChanges with ${workspaceEdit.documentChanges?.length || 0} entries\n`
      );

      // Convert documentChanges to changes format for compatibility
      const changes: Record<
        string,
        Array<{ range: { start: Position; end: Position }; newText: string }>
      > = {};

      if (workspaceEdit.documentChanges) {
        for (const change of workspaceEdit.documentChanges) {
          // Handle TextDocumentEdit (the only type needed for symbol renames)
          if (change.textDocument && change.edits) {
            const uri = change.textDocument.uri;
            if (!changes[uri]) {
              changes[uri] = [];
            }
            changes[uri].push(...change.edits);
            process.stderr.write(
              `[DEBUG renameSymbol] Added ${change.edits.length} edits for ${uri}\n`
            );
          }
        }
      }

      return { changes };
    }
  }

  process.stderr.write('[DEBUG renameSymbol] No rename changes available\n');
  return {};
}

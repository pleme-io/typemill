import { readFileSync } from 'node:fs';
import type { LSPClient } from '../lsp-client.js';
import type { DiagnosticMethodsContext } from '../lsp-types.js';
import { pathToUri } from '../path-utils.js';
import type { CodeAction, Diagnostic, DocumentDiagnosticReport, Position } from '../types.js';

export async function getDiagnostics(
  context: DiagnosticMethodsContext,
  filePath: string
): Promise<Diagnostic[]> {
  process.stderr.write(`[DEBUG getDiagnostics] Requesting diagnostics for ${filePath}\n`);

  const serverState = await context.getServer(filePath);

  // Wait for the server to be fully initialized
  await serverState.initializationPromise;

  // Ensure the file is opened and synced with the LSP server
  await context.ensureFileOpen(serverState, filePath);

  // First, check if we have cached diagnostics from publishDiagnostics
  const fileUri = pathToUri(filePath);
  const cachedDiagnostics = serverState.diagnostics.get(fileUri);

  if (cachedDiagnostics !== undefined) {
    process.stderr.write(
      `[DEBUG getDiagnostics] Returning ${cachedDiagnostics.length} cached diagnostics from publishDiagnostics\n`
    );
    return cachedDiagnostics;
  }

  // If no cached diagnostics, try the pull-based textDocument/diagnostic
  process.stderr.write(
    '[DEBUG getDiagnostics] No cached diagnostics, trying textDocument/diagnostic request\n'
  );

  try {
    const result = await context.sendRequest(serverState.process, 'textDocument/diagnostic', {
      textDocument: { uri: fileUri },
    });

    process.stderr.write(
      `[DEBUG getDiagnostics] Result type: ${typeof result}, has kind: ${result && typeof result === 'object' && 'kind' in result}\n`
    );

    if (result && typeof result === 'object' && 'kind' in result) {
      const report = result as DocumentDiagnosticReport;

      if (report.kind === 'full' && report.items) {
        process.stderr.write(
          `[DEBUG getDiagnostics] Full report with ${report.items.length} diagnostics\n`
        );
        return report.items;
      }
      if (report.kind === 'unchanged') {
        process.stderr.write('[DEBUG getDiagnostics] Unchanged report (no new diagnostics)\n');
        return [];
      }
    }

    process.stderr.write(
      '[DEBUG getDiagnostics] Unexpected response format, returning empty array\n'
    );
    return [];
  } catch (error) {
    // Some LSP servers may not support textDocument/diagnostic
    // Try falling back to waiting for publishDiagnostics notifications
    process.stderr.write(
      `[DEBUG getDiagnostics] textDocument/diagnostic not supported or failed: ${error}. Waiting for publishDiagnostics...\n`
    );

    // Wait for the server to become idle and publish diagnostics
    // MCP tools can afford longer wait times for better reliability
    await context.waitForDiagnosticsIdle(serverState, fileUri, {
      maxWaitTime: 5000, // 5 seconds - generous timeout for MCP usage
      idleTime: 300, // 300ms idle time to ensure all diagnostics are received
    });

    // Check again for cached diagnostics
    const diagnosticsAfterWait = serverState.diagnostics.get(fileUri);
    if (diagnosticsAfterWait !== undefined) {
      process.stderr.write(
        `[DEBUG getDiagnostics] Returning ${diagnosticsAfterWait.length} diagnostics after waiting for idle state\n`
      );
      return diagnosticsAfterWait;
    }

    // If still no diagnostics, try triggering publishDiagnostics by making a no-op change
    process.stderr.write(
      '[DEBUG getDiagnostics] No diagnostics yet, triggering publishDiagnostics with no-op change\n'
    );

    try {
      // Get current file content
      const fileContent = readFileSync(filePath, 'utf-8');

      // Send a no-op change notification (add and remove a space at the end)
      // Use proper version tracking instead of timestamps
      const version1 = (serverState.fileVersions.get(filePath) || 1) + 1;
      serverState.fileVersions.set(filePath, version1);

      await context.sendNotification(serverState.process, 'textDocument/didChange', {
        textDocument: {
          uri: fileUri,
          version: version1,
        },
        contentChanges: [
          {
            text: `${fileContent} `,
          },
        ],
      });

      // Immediately revert the change with next version
      const version2 = version1 + 1;
      serverState.fileVersions.set(filePath, version2);

      await context.sendNotification(serverState.process, 'textDocument/didChange', {
        textDocument: {
          uri: fileUri,
          version: version2,
        },
        contentChanges: [
          {
            text: fileContent,
          },
        ],
      });

      // Wait for the server to process the changes and become idle
      // After making changes, servers may need time to re-analyze
      await context.waitForDiagnosticsIdle(serverState, fileUri, {
        maxWaitTime: 3000, // 3 seconds after triggering changes
        idleTime: 300, // Consistent idle time for reliability
      });

      // Check one more time
      const diagnosticsAfterTrigger = serverState.diagnostics.get(fileUri);
      if (diagnosticsAfterTrigger !== undefined) {
        process.stderr.write(
          `[DEBUG getDiagnostics] Returning ${diagnosticsAfterTrigger.length} diagnostics after triggering publishDiagnostics\n`
        );
        return diagnosticsAfterTrigger;
      }
    } catch (triggerError) {
      process.stderr.write(
        `[DEBUG getDiagnostics] Failed to trigger publishDiagnostics: ${triggerError}\n`
      );
    }

    return [];
  }
}

export async function getCodeActions(
  context: DiagnosticMethodsContext,
  filePath: string,
  range?: { start: Position; end: Position },
  actionContext?: { diagnostics?: Diagnostic[] }
): Promise<CodeAction[]> {
  const serverState = await context.getServer(filePath);
  if (!serverState.initialized) {
    throw new Error('Server not initialized');
  }

  await context.ensureFileOpen(serverState, filePath);
  const fileUri = pathToUri(filePath);

  // Get current diagnostics for the file to provide context
  const diagnostics = serverState.diagnostics.get(fileUri) || [];

  // Create a proper range - use a smaller, more realistic range
  const requestRange = range || {
    start: { line: 0, character: 0 },
    end: { line: Math.min(100, 999999), character: 0 },
  };

  // Ensure context includes diagnostics and only property
  const codeActionContext = {
    diagnostics: actionContext?.diagnostics || diagnostics,
    only: undefined, // Don't filter by specific code action kinds
  };

  process.stderr.write(
    `[DEBUG getCodeActions] Request params: ${JSON.stringify(
      {
        textDocument: { uri: fileUri },
        range: requestRange,
        context: codeActionContext,
      },
      null,
      2
    )}\n`
  );

  try {
    const result = await context.sendRequest(serverState.process, 'textDocument/codeAction', {
      textDocument: { uri: fileUri },
      range: requestRange,
      context: codeActionContext,
    });

    process.stderr.write(`[DEBUG getCodeActions] Raw result: ${JSON.stringify(result)}\n`);

    if (!result) return [];
    if (Array.isArray(result)) return result.filter((action) => action != null);
    return [];
  } catch (error) {
    process.stderr.write(`[DEBUG getCodeActions] Error: ${error}\n`);
    return [];
  }
}

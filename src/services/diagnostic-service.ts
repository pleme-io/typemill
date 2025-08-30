import { readFileSync } from 'node:fs';
import type { ServerState } from '../lsp-types.js';
import type { LSPProtocol } from '../lsp/protocol.js';
import { pathToUri } from '../path-utils.js';
import type { Diagnostic, DocumentDiagnosticReport } from '../types.js';

/**
 * Service for diagnostic-related LSP operations
 * Handles error and warning collection from language servers
 */
export class DiagnosticService {
  constructor(
    private getServer: (filePath: string) => Promise<ServerState>,
    private protocol: LSPProtocol
  ) {}

  /**
   * Get diagnostics for a file
   */
  async getDiagnostics(filePath: string): Promise<Diagnostic[]> {
    process.stderr.write(`[DEBUG getDiagnostics] Requesting diagnostics for ${filePath}\n`);

    const serverState = await this.getServer(filePath);

    // Wait for the server to be fully initialized
    await serverState.initializationPromise;

    // Ensure the file is opened and synced with the LSP server
    await this.ensureFileOpen(serverState, filePath);

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
      const result = await this.protocol.sendRequest(
        serverState.process,
        'textDocument/diagnostic',
        {
          textDocument: { uri: fileUri },
        }
      );

      process.stderr.write(
        `[DEBUG getDiagnostics] Result type: ${typeof result}, has kind: ${result && typeof result === 'object' && 'kind' in result}\n`
      );
      process.stderr.write(`[DEBUG getDiagnostics] Full result: ${JSON.stringify(result)}\n`);

      // Handle LSP 3.17+ DocumentDiagnosticReport format
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

      // Handle direct diagnostic array (legacy format)
      if (Array.isArray(result)) {
        process.stderr.write(
          `[DEBUG getDiagnostics] Direct diagnostic array with ${result.length} diagnostics\n`
        );
        return result as Diagnostic[];
      }

      // Handle null/empty responses (server may not have diagnostics yet)
      if (result === null || result === undefined) {
        process.stderr.write(
          '[DEBUG getDiagnostics] Null/undefined result, falling back to other methods\n'
        );
        // Don't return early, fall through to the publishDiagnostics fallback
      } else {
        process.stderr.write(
          '[DEBUG getDiagnostics] Unexpected response format, falling back to other methods\n'
        );
      }

      // If we reach here, the textDocument/diagnostic didn't work as expected
      // Fall through to publishDiagnostics method
    } catch (error) {
      // Some LSP servers may not support textDocument/diagnostic
      // Try falling back to waiting for publishDiagnostics notifications
      process.stderr.write(
        `[DEBUG getDiagnostics] textDocument/diagnostic not supported or failed: ${error}. Waiting for publishDiagnostics...\n`
      );

      // Wait for the server to become idle and publish diagnostics
      // MCP tools can afford longer wait times for better reliability
      await this.waitForDiagnosticsIdle(serverState, fileUri, {
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

        await this.protocol.sendNotification(serverState.process, 'textDocument/didChange', {
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

        await this.protocol.sendNotification(serverState.process, 'textDocument/didChange', {
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
        await this.waitForDiagnosticsIdle(serverState, fileUri, {
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

  /**
   * Filter diagnostics by severity level
   */
  filterDiagnosticsByLevel(diagnostics: Diagnostic[], minSeverity: number): Diagnostic[] {
    return diagnostics.filter(
      (diagnostic) => diagnostic.severity === undefined || diagnostic.severity <= minSeverity
    );
  }

  /**
   * Get diagnostics related to a specific position
   */
  getRelatedDiagnostics(
    diagnostics: Diagnostic[],
    position: { line: number; character: number }
  ): Diagnostic[] {
    return diagnostics.filter((diagnostic) => {
      const range = diagnostic.range;
      return (
        position.line >= range.start.line &&
        position.line <= range.end.line &&
        (position.line !== range.start.line || position.character >= range.start.character) &&
        (position.line !== range.end.line || position.character <= range.end.character)
      );
    });
  }

  /**
   * Categorize diagnostics by type
   */
  categorizeDiagnostics(diagnostics: Diagnostic[]): {
    errors: Diagnostic[];
    warnings: Diagnostic[];
    infos: Diagnostic[];
    hints: Diagnostic[];
  } {
    const errors: Diagnostic[] = [];
    const warnings: Diagnostic[] = [];
    const infos: Diagnostic[] = [];
    const hints: Diagnostic[] = [];

    for (const diagnostic of diagnostics) {
      switch (diagnostic.severity) {
        case 1: // Error
          errors.push(diagnostic);
          break;
        case 2: // Warning
          warnings.push(diagnostic);
          break;
        case 3: // Information
          infos.push(diagnostic);
          break;
        case 4: // Hint
          hints.push(diagnostic);
          break;
        default:
          errors.push(diagnostic); // Treat unknown as error
      }
    }

    return { errors, warnings, infos, hints };
  }

  /**
   * Wait for diagnostics to stabilize after file changes
   */
  private async waitForDiagnosticsIdle(
    serverState: ServerState,
    fileUri: string,
    options: { maxWaitTime?: number; idleTime?: number; checkInterval?: number } = {}
  ): Promise<void> {
    const {
      maxWaitTime = 10000, // 10 seconds max wait
      idleTime = 1000, // 1 second of no updates = idle
      checkInterval = 100, // Check every 100ms
    } = options;

    const startTime = Date.now();
    let lastUpdateTime = serverState.lastDiagnosticUpdate.get(fileUri) || 0;

    return new Promise((resolve) => {
      const checkIdle = () => {
        const now = Date.now();
        const currentUpdateTime = serverState.lastDiagnosticUpdate.get(fileUri) || 0;

        // Check if we've exceeded max wait time
        if (now - startTime >= maxWaitTime) {
          process.stderr.write(
            `[DEBUG waitForDiagnosticsIdle] Max wait time reached for ${fileUri}\n`
          );
          resolve();
          return;
        }

        // Check if there was a new update
        if (currentUpdateTime > lastUpdateTime) {
          lastUpdateTime = currentUpdateTime;
          // Reset idle timer
          setTimeout(checkIdle, checkInterval);
          return;
        }

        // Check if we've been idle long enough
        if (now - lastUpdateTime >= idleTime) {
          process.stderr.write(`[DEBUG waitForDiagnosticsIdle] Diagnostics idle for ${fileUri}\n`);
          resolve();
          return;
        }

        // Continue checking
        setTimeout(checkIdle, checkInterval);
      };

      // Start checking
      setTimeout(checkIdle, checkInterval);
    });
  }

  /**
   * Ensure file is open in LSP server
   */
  private async ensureFileOpen(serverState: ServerState, filePath: string): Promise<void> {
    if (serverState.openFiles.has(filePath)) {
      return;
    }

    try {
      const fileContent = readFileSync(filePath, 'utf-8');

      this.protocol.sendNotification(serverState.process, 'textDocument/didOpen', {
        textDocument: {
          uri: `file://${filePath}`,
          languageId: this.getLanguageId(filePath),
          version: 1,
          text: fileContent,
        },
      });

      serverState.openFiles.add(filePath);
    } catch (error) {
      throw new Error(
        `Failed to open file for LSP server: ${filePath} - ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  private getLanguageId(filePath: string): string {
    const ext = filePath.split('.').pop()?.toLowerCase();
    const languageMap: Record<string, string> = {
      ts: 'typescript',
      tsx: 'typescriptreact',
      js: 'javascript',
      jsx: 'javascriptreact',
      py: 'python',
      go: 'go',
      rs: 'rust',
      java: 'java',
      cpp: 'cpp',
      c: 'c',
      h: 'c',
      hpp: 'cpp',
    };
    return languageMap[ext || ''] || 'plaintext';
  }
}

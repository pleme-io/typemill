import { readFileSync } from 'node:fs';
import { pathToUri } from '../path-utils.js';
import type { Diagnostic, DocumentDiagnosticReport } from '../types.js';
import type { ServiceContext } from './service-context.js';

/**
 * Service for diagnostic-related LSP operations
 * Handles error and warning collection from language servers
 */
export class DiagnosticService {
  constructor(private context: ServiceContext) {}

  /**
   * Get diagnostics for a file
   */
  async getDiagnostics(filePath: string): Promise<Diagnostic[]> {
    process.stderr.write(`[DEBUG getDiagnostics] Requesting diagnostics for ${filePath}\n`);

    const serverState = await this.context.prepareFile(filePath);

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
      const result = await this.context.protocol.sendRequest(
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
      // Fall through to publishDiagnostics waiting logic below
      process.stderr.write(
        '[DEBUG getDiagnostics] textDocument/diagnostic returned null/invalid result, falling back to publishDiagnostics\n'
      );
    } catch (error) {
      // Some LSP servers may not support textDocument/diagnostic
      process.stderr.write(
        `[DEBUG getDiagnostics] textDocument/diagnostic not supported or failed: ${error}. Falling back to publishDiagnostics...\n`
      );
    }

    // Fallback: Wait for publishDiagnostics notifications (works for most LSP servers)
    process.stderr.write(
      '[DEBUG getDiagnostics] Waiting for publishDiagnostics notifications...\n'
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

      await this.context.protocol.sendNotification(serverState.process, 'textDocument/didChange', {
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

      await this.context.protocol.sendNotification(serverState.process, 'textDocument/didChange', {
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
    serverState: import('../lsp-types.js').ServerState,
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
  // ensureFileOpen() and getLanguageId() methods removed - provided by ServiceContext
  // This eliminates ~45 lines of duplicated code from this service
}

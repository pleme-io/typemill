import { readFileSync } from 'node:fs';
import { logDebugMessage } from '../../../../../server/src/core/diagnostics/debug-logger.js';
import { pathToUri } from '../../../../../server/src/core/file-operations/path-utils.js';
import type { Diagnostic, DocumentDiagnosticReport } from '../../../../../server/src/types.js';
import type { ServiceContext } from '../../../../../server/src/services/service-context.js';

// Diagnostic service constants
const DIAGNOSTIC_WAIT_TIMEOUT_MS = process.env.TEST_MODE ? 10000 : 5000; // Longer wait in tests
const DIAGNOSTIC_IDLE_TIME_MS = 300; // Idle time to ensure all diagnostics received
const DIAGNOSTIC_POST_CHANGE_TIMEOUT_MS = 3000; // Timeout after triggering changes
const DIAGNOSTIC_MAX_WAIT_TIME_MS = 10000; // Maximum wait time for idle state
const DIAGNOSTIC_IDLE_LONG_MS = 1000; // Longer idle time for general waiting
const DIAGNOSTIC_CHECK_INTERVAL_MS = 100; // How often to check for diagnostic updates

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
    logDebugMessage('DiagnosticService', `Requesting diagnostics for ${filePath}`);

    const serverState = await this.context.prepareFile(filePath);

    // First, check if we have cached diagnostics from publishDiagnostics
    const fileUri = pathToUri(filePath);
    const cachedDiagnostics = serverState.diagnostics.get(fileUri);

    if (cachedDiagnostics !== undefined) {
      logDebugMessage(
        'DiagnosticService',
        `Returning ${cachedDiagnostics.length} cached diagnostics from publishDiagnostics`
      );
      return cachedDiagnostics;
    }

    // If no cached diagnostics, try the pull-based textDocument/diagnostic
    logDebugMessage(
      'DiagnosticService',
      'No cached diagnostics, trying textDocument/diagnostic request'
    );

    try {
      const result = await this.context.protocol.sendRequest(
        serverState.process,
        'textDocument/diagnostic',
        {
          textDocument: { uri: fileUri },
        }
      );

      logDebugMessage(
        'DiagnosticService',
        `Result type: ${typeof result}, has kind: ${result && typeof result === 'object' && 'kind' in result}`
      );
      logDebugMessage('DiagnosticService', 'Full result:', result);

      // Handle LSP 3.17+ DocumentDiagnosticReport format
      if (result && typeof result === 'object' && 'kind' in result) {
        const report = result as DocumentDiagnosticReport;

        if (report.kind === 'full' && report.items) {
          logDebugMessage(
            'DiagnosticService',
            `Full report with ${report.items.length} diagnostics`
          );
          return report.items;
        }
        if (report.kind === 'unchanged') {
          logDebugMessage('DiagnosticService', 'Unchanged report (no new diagnostics)');
          return [];
        }
      }

      // Handle direct diagnostic array (legacy format)
      if (Array.isArray(result)) {
        logDebugMessage(
          'DiagnosticService',
          `Direct diagnostic array with ${result.length} diagnostics`
        );
        return result as Diagnostic[];
      }

      // Handle null/empty responses (server may not have diagnostics yet)
      // Fall through to publishDiagnostics waiting logic below
      logDebugMessage(
        'DiagnosticService',
        'textDocument/diagnostic returned null/invalid result, falling back to publishDiagnostics'
      );
    } catch (error) {
      // Some LSP servers may not support textDocument/diagnostic
      logDebugMessage(
        'DiagnosticService',
        `textDocument/diagnostic not supported or failed: ${error}. Falling back to publishDiagnostics...`
      );
    }

    // Fallback: Wait for publishDiagnostics notifications (works for most LSP servers)
    logDebugMessage('DiagnosticService', 'Waiting for publishDiagnostics notifications...');

    // Wait for the server to become idle and publish diagnostics
    // MCP tools can afford longer wait times for better reliability
    await this.waitForDiagnosticsIdle(serverState, fileUri, {
      maxWaitTime: DIAGNOSTIC_WAIT_TIMEOUT_MS, // Generous timeout for MCP usage
      idleTime: DIAGNOSTIC_IDLE_TIME_MS, // Idle time to ensure all diagnostics are received
    });

    // Check again for cached diagnostics
    const diagnosticsAfterWait = serverState.diagnostics.get(fileUri);
    if (diagnosticsAfterWait !== undefined) {
      logDebugMessage(
        'DiagnosticService',
        `Returning ${diagnosticsAfterWait.length} diagnostics after waiting for idle state`
      );
      return diagnosticsAfterWait;
    }

    // If still no diagnostics, try triggering publishDiagnostics by making a no-op change
    logDebugMessage(
      'DiagnosticService',
      'No diagnostics yet, triggering publishDiagnostics with no-op change'
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
        maxWaitTime: DIAGNOSTIC_POST_CHANGE_TIMEOUT_MS, // Timeout after triggering changes
        idleTime: DIAGNOSTIC_IDLE_TIME_MS, // Consistent idle time for reliability
      });

      // Check one more time
      const diagnosticsAfterTrigger = serverState.diagnostics.get(fileUri);
      if (diagnosticsAfterTrigger !== undefined) {
        logDebugMessage(
          'DiagnosticService',
          `Returning ${diagnosticsAfterTrigger.length} diagnostics after triggering publishDiagnostics`
        );
        return diagnosticsAfterTrigger;
      }
    } catch (triggerError) {
      logDebugMessage('DiagnosticService', `Failed to trigger publishDiagnostics: ${triggerError}`);
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
    serverState: import('../../../../../server/src/lsp/types.js').ServerState,
    fileUri: string,
    options: { maxWaitTime?: number; idleTime?: number; checkInterval?: number } = {}
  ): Promise<void> {
    const {
      maxWaitTime = DIAGNOSTIC_MAX_WAIT_TIME_MS, // Max wait for idle state
      idleTime = DIAGNOSTIC_IDLE_LONG_MS, // No updates = idle
      checkInterval = DIAGNOSTIC_CHECK_INTERVAL_MS, // How often to check
    } = options;

    const startTime = Date.now();
    let lastUpdateTime = serverState.lastDiagnosticUpdate.get(fileUri) || 0;

    return new Promise((resolve) => {
      const checkIdle = () => {
        const now = Date.now();
        const currentUpdateTime = serverState.lastDiagnosticUpdate.get(fileUri) || 0;

        // Check if we've exceeded max wait time
        if (now - startTime >= maxWaitTime) {
          logDebugMessage('DiagnosticService', `Max wait time reached for ${fileUri}`);
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
          logDebugMessage('DiagnosticService', `Diagnostics idle for ${fileUri}`);
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

import { readFileSync } from 'node:fs';
import * as DiagnosticMethods from '../lsp-methods/diagnostic-methods.js';
import type { DiagnosticMethodsContext, ServerState } from '../lsp-types.js';
import type { LSPProtocol } from '../lsp/protocol.js';
import type { Diagnostic } from '../types.js';

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
    const context: DiagnosticMethodsContext = {
      getServer: this.getServer,
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (process, method, params, timeout) =>
        this.protocol.sendRequest(process, method, params, timeout),
      sendNotification: (process, method, params) =>
        this.protocol.sendNotification(process, method, params),
      waitForDiagnosticsIdle: this.waitForDiagnosticsIdle.bind(this),
    };
    return DiagnosticMethods.getDiagnostics(context, filePath);
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

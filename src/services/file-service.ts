import { readFileSync } from 'node:fs';
import { capabilityManager } from '../capability-manager.js';
import type { ServerState } from '../lsp-types.js';
import type { LSPProtocol } from '../lsp/protocol.js';
import { pathToUri } from '../path-utils.js';
import type {
  CodeAction,
  Diagnostic,
  DocumentLink,
  FoldingRange,
  Position,
  Range,
  TextEdit,
} from '../types.js';

/**
 * Service for file-related LSP operations
 * Handles formatting, code actions, document links, and file synchronization
 */
export class FileService {
  constructor(
    private getServer: (filePath: string) => Promise<ServerState>,
    private protocol: LSPProtocol
  ) {}

  /**
   * Format document
   */
  async formatDocument(
    filePath: string,
    options?: {
      tabSize?: number;
      insertSpaces?: boolean;
      trimTrailingWhitespace?: boolean;
      insertFinalNewline?: boolean;
      trimFinalNewlines?: boolean;
    }
  ): Promise<TextEdit[]> {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error('Server not initialized');
    }

    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);

    const formattingOptions = {
      tabSize: options?.tabSize || 2,
      insertSpaces: options?.insertSpaces !== false,
      ...(options?.trimTrailingWhitespace !== undefined && {
        trimTrailingWhitespace: options.trimTrailingWhitespace,
      }),
      ...(options?.insertFinalNewline !== undefined && {
        insertFinalNewline: options.insertFinalNewline,
      }),
      ...(options?.trimFinalNewlines !== undefined && {
        trimFinalNewlines: options.trimFinalNewlines,
      }),
    };

    const result = await this.protocol.sendRequest(serverState.process, 'textDocument/formatting', {
      textDocument: { uri: fileUri },
      options: formattingOptions,
    });

    return Array.isArray(result) ? result : [];
  }

  /**
   * Get code actions for range
   */
  async getCodeActions(
    filePath: string,
    range?: Range,
    context?: { diagnostics?: Diagnostic[] }
  ): Promise<CodeAction[]> {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error('Server not initialized');
    }

    await this.ensureFileOpen(serverState, filePath);
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
      diagnostics: context?.diagnostics || diagnostics,
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
      const result = await this.protocol.sendRequest(
        serverState.process,
        'textDocument/codeAction',
        {
          textDocument: { uri: fileUri },
          range: requestRange,
          context: codeActionContext,
        }
      );

      process.stderr.write(`[DEBUG getCodeActions] Raw result: ${JSON.stringify(result)}\n`);

      if (!result) return [];
      if (Array.isArray(result)) return result.filter((action) => action != null);
      return [];
    } catch (error) {
      process.stderr.write(`[DEBUG getCodeActions] Error: ${error}\n`);
      return [];
    }
  }

  /**
   * Get folding ranges
   */
  async getFoldingRanges(filePath: string): Promise<FoldingRange[]> {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error('Server not initialized');
    }

    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);

    process.stderr.write(`[DEBUG getFoldingRanges] Requesting folding ranges for: ${filePath}\n`);

    const result = await this.protocol.sendRequest(
      serverState.process,
      'textDocument/foldingRange',
      {
        textDocument: { uri: fileUri },
      }
    );

    process.stderr.write(
      `[DEBUG getFoldingRanges] Result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : 'N/A'}\n`
    );

    if (Array.isArray(result)) {
      return result as FoldingRange[];
    }

    return [];
  }

  /**
   * Get document links
   */
  async getDocumentLinks(filePath: string): Promise<DocumentLink[]> {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error('Server not initialized');
    }

    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);

    process.stderr.write(`[DEBUG getDocumentLinks] Requesting document links for: ${filePath}\n`);

    const result = await this.protocol.sendRequest(
      serverState.process,
      'textDocument/documentLink',
      {
        textDocument: { uri: fileUri },
      }
    );

    process.stderr.write(
      `[DEBUG getDocumentLinks] Result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : 'N/A'}\n`
    );

    if (Array.isArray(result)) {
      return result as DocumentLink[];
    }

    return [];
  }

  /**
   * Apply workspace edit
   */
  async applyWorkspaceEdit(edit: {
    changes?: Record<string, TextEdit[]>;
    documentChanges?: Array<{
      textDocument: { uri: string; version?: number };
      edits: TextEdit[];
    }>;
  }): Promise<{ applied: boolean; failureReason?: string }> {
    try {
      if (edit.changes) {
        for (const [uri, edits] of Object.entries(edit.changes)) {
          const filePath = uri.replace('file://', '');
          await this.applyTextEdits(filePath, edits);
        }
      }

      if (edit.documentChanges) {
        for (const change of edit.documentChanges) {
          const filePath = change.textDocument.uri.replace('file://', '');
          await this.applyTextEdits(filePath, change.edits);
        }
      }

      return { applied: true };
    } catch (error) {
      return {
        applied: false,
        failureReason: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Rename file
   */
  async renameFile(oldPath: string, newPath: string): Promise<void> {
    // This would typically involve file system operations
    // For now, just notify LSP servers about the change
    try {
      // Get all active servers that might be interested
      const serverConfigs = new Map();
      // Implementation would check which servers handle these file types

      // Send willRename notification to interested servers
      for (const serverState of serverConfigs.values()) {
        this.protocol.sendNotification(serverState.process, 'workspace/willRenameFiles', {
          files: [
            {
              oldUri: `file://${oldPath}`,
              newUri: `file://${newPath}`,
            },
          ],
        });
      }
    } catch (error) {
      process.stderr.write(`[ERROR renameFile] ${error}\n`);
    }
  }

  /**
   * Ensure file is open in LSP server
   */
  async ensureFileOpen(serverState: ServerState, filePath: string): Promise<void> {
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

  /**
   * Apply text edits to a file
   */
  private async applyTextEdits(filePath: string, edits: TextEdit[]): Promise<void> {
    if (edits.length === 0) return;

    try {
      const fileContent = readFileSync(filePath, 'utf-8');
      const lines = fileContent.split('\n');

      // Sort edits in reverse order by position to avoid offset issues
      const sortedEdits = [...edits].sort((a, b) => {
        if (a.range.start.line !== b.range.start.line) {
          return b.range.start.line - a.range.start.line;
        }
        return b.range.start.character - a.range.start.character;
      });

      // Apply edits
      for (const edit of sortedEdits) {
        const startLine = edit.range.start.line;
        const startChar = edit.range.start.character;
        const endLine = edit.range.end.line;
        const endChar = edit.range.end.character;

        if (startLine === endLine) {
          // Single line edit
          const line = lines[startLine];
          if (line !== undefined) {
            lines[startLine] =
              line.substring(0, startChar) + edit.newText + line.substring(endChar);
          }
        } else {
          // Multi-line edit
          const newLines = edit.newText.split('\n');
          const startLineText = lines[startLine];
          const endLineText = lines[endLine];
          if (startLineText !== undefined && endLineText !== undefined) {
            const firstLine = startLineText.substring(0, startChar) + newLines[0];
            const lastLine = newLines[newLines.length - 1] + endLineText.substring(endChar);

            // Replace the range with new content
            const replacementLines = [firstLine, ...newLines.slice(1, -1), lastLine];
            lines.splice(startLine, endLine - startLine + 1, ...replacementLines);
          }
        }
      }

      // This would normally write back to the file
      // For now, just log what would happen
      process.stderr.write(
        `[DEBUG applyTextEdits] Would apply ${edits.length} edits to ${filePath}\n`
      );
    } catch (error) {
      throw new Error(`Failed to apply text edits to ${filePath}: ${error}`);
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

  /**
   * Synchronize file content with LSP server after external modifications
   * This should be called after any disk writes to keep the LSP server in sync
   */
  async syncFileContent(filePath: string): Promise<void> {
    try {
      const serverState = await this.getServer(filePath);

      // If file is not already open in the LSP server, open it first
      if (!serverState.openFiles.has(filePath)) {
        process.stderr.write(
          `[DEBUG syncFileContent] File not open, opening it first: ${filePath}\n`
        );
        await this.ensureFileOpen(serverState, filePath);
      }

      process.stderr.write(`[DEBUG syncFileContent] Syncing file: ${filePath}\n`);

      const fileContent = readFileSync(filePath, 'utf-8');
      const uri = pathToUri(filePath);

      // Increment version and send didChange notification
      const version = (serverState.fileVersions.get(filePath) || 1) + 1;
      serverState.fileVersions.set(filePath, version);

      await this.protocol.sendNotification(serverState.process, 'textDocument/didChange', {
        textDocument: {
          uri,
          version,
        },
        contentChanges: [
          {
            text: fileContent,
          },
        ],
      });

      process.stderr.write(
        `[DEBUG syncFileContent] File synced with version ${version}: ${filePath}\n`
      );
    } catch (error) {
      process.stderr.write(`[DEBUG syncFileContent] Failed to sync file ${filePath}: ${error}\n`);
      // Don't throw - syncing is best effort
    }
  }
}

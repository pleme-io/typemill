import { existsSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { DIFF_DELETE, DIFF_EQUAL, DIFF_INSERT, diff_match_patch } from 'diff-match-patch';
import { logDebugMessage } from '../core/diagnostics/debug-logger.js';
import { handleFileSystemError, logError } from '../core/diagnostics/error-utils.js';
import { pathToUri, uriToPath } from '../core/file-operations/path-utils.js';
import type { CodeAction, Diagnostic, Range, TextEdit } from '../types.js';
import type { ServiceContext } from './service-context.js';

// File service constants
const MAX_LINE_NUMBER = 999999; // Maximum line number for file operations

/**
 * Service for file-related LSP operations
 * Handles formatting, code actions, document links, and file synchronization
 */
export class FileService {
  private trackedFiles: Set<string> = new Set();
  private fileContentCache: Map<string, string> = new Map();

  constructor(private context: ServiceContext) {}

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
    const serverState = await this.context.prepareFile(filePath);
    if (!serverState.initialized) {
      throw new Error('Server not initialized for formatting document');
    }
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

    const result = await this.context.protocol.sendRequest(
      serverState.process,
      'textDocument/formatting',
      {
        textDocument: { uri: fileUri },
        options: formattingOptions,
      }
    );

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
    const serverState = await this.context.prepareFile(filePath);
    if (!serverState.initialized) {
      throw new Error('Server not initialized for getting code actions');
    }
    const fileUri = pathToUri(filePath);

    // Get current diagnostics for the file to provide context
    const diagnostics = serverState.diagnostics.get(fileUri) || [];

    // Create a proper range - use a smaller, more realistic range
    const requestRange = range || {
      start: { line: 0, character: 0 },
      end: { line: Math.min(100, MAX_LINE_NUMBER), character: 0 },
    };

    // Ensure context includes diagnostics and only property
    const codeActionContext = {
      diagnostics: context?.diagnostics || diagnostics,
      only: undefined, // Don't filter by specific code action kinds
    };

    logDebugMessage('FileService', 'Request params:', {
      textDocument: { uri: fileUri },
      range: requestRange,
      context: codeActionContext,
    });

    try {
      const result = await this.context.protocol.sendRequest(
        serverState.process,
        'textDocument/codeAction',
        {
          textDocument: { uri: fileUri },
          range: requestRange,
          context: codeActionContext,
        }
      );

      logDebugMessage('FileService', 'Raw result:', result);

      if (!result) return [];
      if (Array.isArray(result)) return result.filter((action) => action != null);
      return [];
    } catch (error) {
      logError('FileService', 'Failed to get code actions', error, {
        filePath,
        range,
      });
      return [];
    }
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
          const filePath = uriToPath(uri);
          await this.applyTextEdits(filePath, edits);
        }
      }

      if (edit.documentChanges) {
        for (const change of edit.documentChanges) {
          const filePath = uriToPath(change.textDocument.uri);
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
   * Open a file in the LSP server
   * This ensures the file is tracked and ready for LSP operations
   */
  async openFile(filePath: string, skipPredictiveLoading = false): Promise<void> {
    try {
      const _serverState = await this.context.prepareFile(filePath);

      // Trigger predictive loading if enabled and we have a predictive loader
      // Skip if explicitly requested (to avoid infinite recursion)
      if (
        !skipPredictiveLoading &&
        this.context.config?.serverOptions?.enablePredictiveLoading !== false && // Default to true
        this.context.predictiveLoader
      ) {
        // Run predictive loading in the background
        this.context.predictiveLoader.preloadImports(filePath).catch((err: any) => {
          this.context.logger?.warn('Predictive loading failed', {
            file: filePath,
            error: err.message,
          });
        });
      }
    } catch (error) {
      logError('FileService.openFile', error, { filePath });
      throw error;
    }
  }

  /**
   * Internal method to open a file without triggering predictive loading
   * Used by the predictive loader itself to avoid infinite recursion
   */
  async openFileInternal(filePath: string): Promise<void> {
    return this.openFile(filePath, true);
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
        this.context.protocol.sendNotification(serverState.process, 'workspace/willRenameFiles', {
          files: [
            {
              oldUri: pathToUri(oldPath),
              newUri: pathToUri(newPath),
            },
          ],
        });
      }
    } catch (error) {
      logDebugMessage('FileService', `ERROR renameFile: ${error}`);
    }
  }

  /**
   * Ensure file is open in LSP server
   */
  async ensureFileOpen(
    serverState: import('../lsp/types.js').ServerState,
    filePath: string
  ): Promise<void> {
    if (serverState.openFiles.has(filePath)) {
      return;
    }

    try {
      const fileContent = readFileSync(filePath, 'utf-8');

      this.context.protocol.sendNotification(serverState.process, 'textDocument/didOpen', {
        textDocument: {
          uri: pathToUri(filePath),
          languageId: this.context.getLanguageId(filePath),
          version: 1,
          text: fileContent,
        },
      });

      serverState.openFiles.add(filePath);
    } catch (error) {
      handleFileSystemError(error, filePath, 'open file for LSP server');
    }
  }

  /**
   * Apply text edits to a file
   */
  private async applyTextEdits(filePath: string, edits: TextEdit[]): Promise<void> {
    if (edits.length === 0) return;

    try {
      const originalContent = readFileSync(filePath, 'utf-8');

      // Sort edits in reverse order (from end to beginning of file)
      // This prevents earlier edits from affecting the positions of later edits
      const sortedEdits = [...edits].sort((a, b) => {
        // First sort by line (reverse)
        if (a.range.start.line !== b.range.start.line) {
          return b.range.start.line - a.range.start.line;
        }
        // Then by character (reverse)
        return b.range.start.character - a.range.start.character;
      });

      // Apply edits by working directly on the lines array
      const lines = originalContent.split('\n');

      for (const edit of sortedEdits) {
        const startLine = edit.range.start.line;
        const startChar = edit.range.start.character;
        const endLine = edit.range.end.line;
        const endChar = edit.range.end.character;

        // Validate line indices
        if (
          startLine < 0 ||
          startLine >= lines.length ||
          endLine < 0 ||
          endLine >= lines.length ||
          startLine > endLine
        ) {
          logDebugMessage(
            'FileService',
            `WARNING applyTextEdits - Invalid range in ${filePath}: ${startLine}:${startChar}-${endLine}:${endChar}`
          );
          continue;
        }

        if (startLine === endLine) {
          // Single-line edit
          const line = lines[startLine];
          if (line === undefined) continue;

          // Validate character indices
          const safeStartChar = Math.max(0, Math.min(startChar, line.length));
          const safeEndChar = Math.max(safeStartChar, Math.min(endChar, line.length));

          lines[startLine] =
            line.substring(0, safeStartChar) + edit.newText + line.substring(safeEndChar);

          logDebugMessage(
            'FileService',
            `Single-line edit at ${startLine}:${safeStartChar}-${safeEndChar} -> "${edit.newText}"`
          );
        } else {
          // Multi-line edit
          const startLineContent = lines[startLine] || '';
          const endLineContent = lines[endLine] || '';

          const safeStartChar = Math.max(0, Math.min(startChar, startLineContent.length));
          const safeEndChar = Math.max(0, Math.min(endChar, endLineContent.length));

          const newLines = edit.newText.split('\n');

          // Build the replacement
          const firstLine = startLineContent.substring(0, safeStartChar) + newLines[0];
          const lastLine =
            (newLines[newLines.length - 1] || '') + endLineContent.substring(safeEndChar);

          // Replace the range of lines
          const replacement =
            newLines.length === 1
              ? [
                  firstLine.substring(0, safeStartChar) +
                    edit.newText +
                    endLineContent.substring(safeEndChar),
                ]
              : [firstLine, ...newLines.slice(1, -1), lastLine];

          lines.splice(startLine, endLine - startLine + 1, ...replacement);

          logDebugMessage(
            'FileService',
            `Multi-line edit at ${startLine}:${safeStartChar}-${endLine}:${safeEndChar} -> "${edit.newText}"`
          );
        }
      }

      // Write the modified content back to the file
      const modifiedContent = lines.join('\n');
      writeFileSync(filePath, modifiedContent, 'utf-8');

      logDebugMessage('FileService', `Applied ${edits.length} edits to ${filePath}`);
    } catch (error) {
      throw new Error(`Failed to apply text edits to ${filePath}: ${error}`);
    }
  }

  // getLanguageId() method removed - provided by ServiceContext
  // This eliminates ~20 lines of duplicated code from this service

  /**
   * Synchronize file content with LSP server using delta updates
   * This should be called after any disk writes to keep the LSP server in sync
   */
  private async syncFileDeltas(
    filePath: string,
    oldContent: string,
    newContent: string
  ): Promise<void> {
    try {
      const serverState = await this.context.getServer(filePath);
      if (!serverState.openFiles.has(filePath)) {
        await this.context.ensureFileOpen(serverState, filePath);
        this.fileContentCache.set(filePath, newContent);
        return;
      }

      const dmp = new diff_match_patch();
      const diffs = dmp.diff_main(oldContent, newContent);
      dmp.diff_cleanupSemantic(diffs);

      const changes: TextDocumentContentChangeEvent[] = [];
      let line = 0;
      let char = 0;

      for (const [op, text] of diffs) {
        const start = { line, character: char };
        const lines = text.split('\n');
        const numLines = lines.length - 1;
        const lastLineLength = lines[numLines]?.length ?? 0;

        if (op === DIFF_EQUAL) {
          if (numLines > 0) {
            line += numLines;
            char = lastLineLength;
          } else {
            char += lastLineLength;
          }
        } else if (op === DIFF_INSERT) {
          changes.push({ range: { start, end: start }, text });
          if (numLines > 0) {
            line += numLines;
            char = lastLineLength;
          } else {
            char += lastLineLength;
          }
        } else if (op === DIFF_DELETE) {
          const endLine = line + numLines;
          const endChar = numLines > 0 ? lastLineLength : char + lastLineLength;
          changes.push({ range: { start, end: { line: endLine, character: endChar } }, text: '' });
        }
      }

      if (changes.length === 0) return;

      const uri = pathToUri(filePath);
      const version = (serverState.fileVersions.get(filePath) || 1) + 1;
      serverState.fileVersions.set(filePath, version);

      await this.context.protocol.sendNotification(serverState.process, 'textDocument/didChange', {
        textDocument: { uri, version },
        contentChanges: changes,
      });

      this.fileContentCache.set(filePath, newContent);
      logDebugMessage(
        'FileService',
        `File synced with ${changes.length} deltas (v${version}): ${filePath}`
      );
    } catch (error) {
      logDebugMessage('FileService', `Failed to sync file deltas for ${filePath}: ${error}`);
    }
  }

  /**
   * Get list of tracked files
   * Used by TransactionManager for capturing state
   */
  getTrackedFiles(): string[] {
    return Array.from(this.trackedFiles);
  }

  /**
   * Track a file for transaction management
   */
  trackFile(filePath: string): void {
    this.trackedFiles.add(filePath);
  }

  /**
   * Read file content safely
   * Returns null if file doesn't exist
   */
  async readFile(filePath: string): Promise<string | null> {
    try {
      if (!existsSync(filePath)) {
        return null;
      }
      const content = readFileSync(filePath, 'utf-8');
      this.fileContentCache.set(filePath, content);
      return content;
    } catch (error) {
      logDebugMessage('FileService', `Failed to read file ${filePath}: ${error}`);
      return null;
    }
  }

  /**
   * Write file content
   */
  async writeFile(filePath: string, content: string): Promise<void> {
    try {
      const oldContent = this.fileContentCache.get(filePath) ?? '';
      writeFileSync(filePath, content, 'utf-8');
      this.trackFile(filePath);
      // Sync with LSP server using deltas
      await this.syncFileDeltas(filePath, oldContent, content);
    } catch (error) {
      throw new Error(`Failed to write file ${filePath}: ${error}`);
    }
  }

  /**
   * Delete file
   */
  async deleteFile(filePath: string): Promise<void> {
    try {
      if (existsSync(filePath)) {
        unlinkSync(filePath);
        this.trackedFiles.delete(filePath);
      }
    } catch (error) {
      throw new Error(`Failed to delete file ${filePath}: ${error}`);
    }
  }
}

import {
  existsSync,
  lstatSync,
  readFileSync,
  realpathSync,
  renameSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { readdir } from 'node:fs/promises';
import { dirname, extname, relative, resolve } from 'node:path';
import type { LSPClient } from '../../../../@codeflow/features/src/lsp/lsp-client.js';
import { astService } from '../../services/ast-service.js';
import { pathManager } from '../../utils/platform/path-manager.js';
import { applyImportPathUpdates, findImportUpdatesForRename } from '../ast/ast-editor.js';
import { type ImportMapping, rewriteImports } from '../ast/language-rewriters.js';
import { logDebugMessage } from '../diagnostics/debug-logger.js';
import { pathToUri, uriToPath } from './path-utils.js';

export interface TextEdit {
  range: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
  newText: string;
}

export interface WorkspaceEdit {
  changes?: Record<string, TextEdit[]>;
}

export interface WorkspaceEditPreview {
  summary: string;
  totalEdits: number;
  filesAffected: number;
  details: Array<{
    filePath: string;
    editCount: number;
    preview: string;
  }>;
}

export interface ApplyEditResult {
  success: boolean;
  filesModified: string[];
  backupFiles: string[];
  error?: string;
}

interface FileBackup {
  originalPath: string; // The path that was requested (could be symlink)
  targetPath: string; // The actual file path (resolved symlink target or same as originalPath)
  originalContent: string;
}

/**
 * Apply a workspace edit to files on disk
 * @param workspaceEdit The edit to apply (from LSP rename operation)
 * @param options Configuration options
 * @returns Result indicating success and modified files
 */
export async function applyWorkspaceEdit(
  workspaceEdit: WorkspaceEdit,
  options: {
    validateBeforeApply?: boolean;
    createBackupFiles?: boolean;
    lspClient?: LSPClient;
  } = {}
): Promise<ApplyEditResult> {
  const { validateBeforeApply = true, createBackupFiles = false, lspClient } = options;

  const backups: FileBackup[] = [];
  const filesModified: string[] = [];
  const backupFilePaths: string[] = [];

  if (!workspaceEdit.changes || Object.keys(workspaceEdit.changes).length === 0) {
    return {
      success: true,
      filesModified: [],
      backupFiles: [],
    };
  }

  try {
    // Pre-flight checks
    for (const [uri, _edits] of Object.entries(workspaceEdit.changes)) {
      const filePath = uriToPath(uri);

      // Check file exists
      if (!existsSync(filePath)) {
        throw new Error(`File does not exist: ${filePath}`);
      }

      // Check if it's a symlink and validate the target
      const stats = lstatSync(filePath);
      if (stats.isSymbolicLink()) {
        // For symlinks, validate that the target exists and is a file
        try {
          const realPath = realpathSync(filePath);
          const targetStats = statSync(realPath);
          if (!targetStats.isFile()) {
            throw new Error(`Symlink target is not a file: ${realPath}`);
          }
        } catch (error) {
          throw new Error(`Cannot resolve symlink ${filePath}: ${error}`);
        }
      } else if (!stats.isFile()) {
        // For non-symlinks, check it's a regular file
        throw new Error(`Not a file: ${filePath}`);
      }

      // Try to read the file to ensure we have permissions
      try {
        readFileSync(filePath, 'utf-8');
      } catch (error) {
        throw new Error(`Cannot read file: ${filePath} - ${error}`);
      }
    }

    // Process each file
    for (const [uri, edits] of Object.entries(workspaceEdit.changes)) {
      const originalPath = uriToPath(uri);

      // Resolve symlinks to their actual target
      let targetPath = originalPath;
      const originalStats = lstatSync(originalPath);
      if (originalStats.isSymbolicLink()) {
        targetPath = realpathSync(originalPath);
        logDebugMessage(
          'FileEditor',
          `Editing symlink target: ${targetPath} (via ${originalPath})`
        );
      }

      // Read content from the actual file (symlink target or regular file)
      const originalContent = readFileSync(targetPath, 'utf-8');

      // Always track original content for rollback
      const backup: FileBackup = {
        originalPath: originalPath, // The requested path (could be symlink)
        targetPath: targetPath, // The actual file to restore
        originalContent,
      };

      backups.push(backup);

      // Create backup file if createBackupFiles is true
      if (createBackupFiles) {
        const backupPath = `${originalPath}.bak`;
        writeFileSync(backupPath, originalContent, 'utf-8');
        backupFilePaths.push(backupPath);
      }

      // Apply edits to the file content
      const modifiedContent = applyEditsToContent(originalContent, edits, validateBeforeApply);

      // Write the modified content atomically to the target location
      const tempPath = `${targetPath}.tmp-${process.pid}-${Date.now()}-${Math.random().toString(36).slice(2)}`;
      writeFileSync(tempPath, modifiedContent, 'utf-8');

      // Atomic rename to replace the target file (not the symlink)
      try {
        renameSync(tempPath, targetPath);
      } catch (error) {
        // Clean up temp file if rename failed
        try {
          if (existsSync(tempPath)) {
            unlinkSync(tempPath);
          }
        } catch {}
        throw error;
      }

      // Report the original path as modified (what the user requested)
      filesModified.push(originalPath);

      // Sync the file with LSP server if client is provided
      // Use the original path (not target) for LSP sync since LSP tracks by requested path
      if (lspClient) {
        await lspClient.syncFileContent(originalPath);
      }
    }

    return {
      success: true,
      filesModified,
      backupFiles: backupFilePaths,
    };
  } catch (error) {
    // Rollback: restore original files from backups
    for (const backup of backups) {
      try {
        // Restore to the target path (the actual file, not the symlink)
        writeFileSync(backup.targetPath, backup.originalContent, 'utf-8');
      } catch (rollbackError) {
        console.error(`Failed to rollback ${backup.targetPath}:`, rollbackError);
      }
    }

    return {
      success: false,
      filesModified: [],
      backupFiles: backupFilePaths,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

/**
 * Apply text edits to file content
 * @param content Original file content
 * @param edits List of edits to apply
 * @param validate Whether to validate edit positions
 * @returns Modified content
 */
function applyEditsToContent(content: string, edits: TextEdit[], validate: boolean): string {
  // Detect and preserve line ending style
  const lineEnding = content.includes('\r\n') ? '\r\n' : '\n';

  // Split content into lines for easier manipulation
  // Handle both LF and CRLF
  const lines = content.split(/\r?\n/);

  // Sort edits in reverse order (bottom to top, right to left)
  // This ensures that earlier edits don't affect the positions of later edits
  const sortedEdits = [...edits].sort((a, b) => {
    if (a.range.start.line !== b.range.start.line) {
      return b.range.start.line - a.range.start.line;
    }
    return b.range.start.character - a.range.start.character;
  });

  for (const edit of sortedEdits) {
    const { start, end } = edit.range;

    // Validate edit positions if requested
    if (validate) {
      if (start.line < 0 || start.line >= lines.length) {
        throw new Error(`Invalid start line ${start.line} (file has ${lines.length} lines)`);
      }
      if (end.line < 0 || end.line > lines.length) {
        throw new Error(`Invalid end line ${end.line} (file has ${lines.length} lines)`);
      }

      // Validate start position is before end position
      if (start.line > end.line || (start.line === end.line && start.character > end.character)) {
        throw new Error(
          `Invalid range: start (${start.line}:${start.character}) is after end (${end.line}:${end.character})`
        );
      }

      // Validate character bounds for start line
      const startLine = lines[start.line];
      if (startLine !== undefined) {
        if (start.character < 0 || start.character > startLine.length) {
          throw new Error(
            `Invalid start character ${start.character} on line ${start.line} (line has ${startLine.length} characters)`
          );
        }
      }

      // Validate character bounds for end line
      const endLine = lines[end.line];
      if (endLine !== undefined) {
        if (end.character < 0 || end.character > endLine.length) {
          throw new Error(
            `Invalid end character ${end.character} on line ${end.line} (line has ${endLine.length} characters)`
          );
        }
      }
    }

    // Apply the edit
    if (start.line === end.line) {
      // Single line edit
      const line = lines[start.line];
      if (line !== undefined) {
        lines[start.line] =
          line.substring(0, start.character) + edit.newText + line.substring(end.character);
      }
    } else {
      // Multi-line edit
      const startLine = lines[start.line];
      const endLine = lines[end.line];

      if (startLine !== undefined && endLine !== undefined) {
        // Combine the parts with the new text
        const newLine =
          startLine.substring(0, start.character) + edit.newText + endLine.substring(end.character);

        // Replace the affected lines
        lines.splice(start.line, end.line - start.line + 1, newLine);
      }
    }
  }

  return lines.join(lineEnding);
}

// Removed unused cleanupBackups function - backup cleanup is handled inline

/**
 * Find all TypeScript/JavaScript files that might import the target file
 * @param rootDir Root directory to search from
 * @param targetPath Path to the file being renamed
 * @param useGitignore Whether to respect gitignore patterns (default: true)
 * @returns Array of file paths that might import the target
 */
export async function findPotentialImporters(
  rootDir: string,
  targetPath: string,
  useGitignore = true
): Promise<string[]> {
  const files: string[] = [];
  const extensions = new Set(['ts', 'tsx', 'js', 'jsx', 'mjs', 'cjs']);

  // Load gitignore patterns if requested
  let ignoreFilter: ReturnType<typeof import('ignore')> | null = null;
  if (useGitignore) {
    const { loadGitignore } = await import('./scanner.js');
    ignoreFilter = await loadGitignore(rootDir);
  }

  async function* getFiles(dir: string, baseDir: string = dir): AsyncGenerator<string> {
    const entries = await readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = resolve(dir, entry.name);
      const relativePath = relative(baseDir, fullPath);

      // Skip ignored paths if using gitignore
      if (ignoreFilter?.ignores(pathManager.normalizePosix(relativePath))) {
        continue;
      }

      if (entry.isDirectory() && !entry.name.startsWith('.')) {
        yield* getFiles(fullPath, baseDir);
      } else if (entry.isFile()) {
        const ext = extname(entry.name).slice(1);
        if (extensions.has(ext)) {
          yield fullPath;
        }
      }
    }
  }

  for await (const file of getFiles(rootDir, rootDir)) {
    if (file !== targetPath) {
      // Don't check the file being renamed
      files.push(file);
    }
  }

  return files;
}

/**
 * Find import statements in a file that reference the target path
 * @param filePath Path to the file to scan
 * @param oldTargetPath Original path of the file being renamed
 * @param newTargetPath New path for the file being renamed
 * @returns Array of text edits to update the imports
 */
export async function findImportsInFile(
  filePath: string,
  oldTargetPath: string,
  newTargetPath: string
): Promise<TextEdit[]> {
  const content = readFileSync(filePath, 'utf-8');
  const fileExt = extname(filePath).toLowerCase();

  // For TypeScript/JavaScript files, use AST-based approach
  if (['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'].includes(fileExt)) {
    const updates = findImportUpdatesForRename(filePath, content, oldTargetPath, newTargetPath);

    // Convert ImportPathUpdate to TextEdit format
    const edits: TextEdit[] = [];
    if (updates.length > 0) {
      // Apply the updates and extract diffs as TextEdits
      const result = applyImportPathUpdates(filePath, content, updates);
      if (result.success && result.content) {
        // Generate text edits from the diff
        const lines = content.split('\n');
        const newLines = result.content.split('\n');

        // Simple line-by-line comparison for generating edits
        for (let i = 0; i < lines.length; i++) {
          if (lines[i] !== newLines[i]) {
            edits.push({
              range: {
                start: { line: i, character: 0 },
                end: { line: i, character: lines[i].length },
              },
              newText: newLines[i],
            });
          }
        }
      }
    }
    return edits;
  }

  // For other languages, use language-specific rewriters
  const edits: TextEdit[] = [];
  const lines = content.split('\n');
  const fileDir = dirname(filePath);

  // Calculate relative paths
  const oldRelative = pathManager.normalizePosix(relative(fileDir, oldTargetPath));
  const newRelative = pathManager.normalizePosix(relative(fileDir, newTargetPath));

  // Create import mapping
  const mappings: ImportMapping[] = [
    {
      oldPath: !oldRelative.startsWith('.') ? `./${oldRelative}` : oldRelative,
      newPath: !newRelative.startsWith('.') ? `./${newRelative}` : newRelative,
    },
  ];

  // Apply language-specific rewriter
  const result = rewriteImports(filePath, content, mappings);

  if (result.success && result.content) {
    // Generate text edits from the rewritten content
    const newLines = result.content.split('\n');
    for (let i = 0; i < lines.length; i++) {
      if (lines[i] !== newLines[i]) {
        edits.push({
          range: {
            start: { line: i, character: 0 },
            end: { line: i, character: lines[i].length },
          },
          newText: newLines[i],
        });
      }
    }
  }

  return edits;
}

/**
 * Update relative imports inside a file that has been moved to a new location
 * @param oldPath Original path of the file before moving
 * @param newPath New path where the file has been moved
 */
async function updateImportsInMovedFile(oldPath: string, newPath: string): Promise<void> {
  const content = readFileSync(newPath, 'utf-8');
  const fileExt = extname(newPath).toLowerCase();

  // For TypeScript/JavaScript files, use AST-based approach
  if (['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'].includes(fileExt)) {
    try {
      // Get all imports in the file
      const imports = await astService.getImports(newPath);
      const updates: Array<{ oldPath: string; newPath: string }> = [];

      // Process each relative import
      for (const importPath of imports) {
        if (importPath.startsWith('.')) {
          // Resolve to absolute path from OLD location
          const targetFile = resolve(dirname(oldPath), importPath);

          // Check if target exists, handling extension variations
          let resolvedTarget = targetFile;
          if (!existsSync(targetFile)) {
            const baseTarget = targetFile.replace(/\.(js|mjs|cjs)$/, '');
            for (const ext of ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs']) {
              if (existsSync(baseTarget + ext)) {
                resolvedTarget = baseTarget + ext;
                break;
              }
            }
            // Check for index files
            if (!existsSync(resolvedTarget)) {
              for (const indexExt of ['/index.ts', '/index.tsx', '/index.js', '/index.jsx']) {
                if (existsSync(targetFile + indexExt)) {
                  resolvedTarget = targetFile + indexExt;
                  break;
                }
              }
            }
          }

          // Calculate new relative path from NEW location
          let newImportPath = pathManager.normalizePosix(
            relative(dirname(newPath), resolvedTarget)
          );

          // Preserve .js extension if original had it
          if (importPath.endsWith('.js') && !newImportPath.endsWith('.js')) {
            newImportPath = newImportPath.replace(/\.(ts|tsx)$/, '.js');
          }

          // Handle directory imports
          if (!importPath.endsWith('/index') && newImportPath.endsWith('/index')) {
            newImportPath = newImportPath.substring(0, newImportPath.length - 6);
          }

          // Add ./ prefix if needed
          if (!newImportPath.startsWith('.') && !newImportPath.startsWith('/')) {
            newImportPath = './' + newImportPath;
          }

          // Only add update if path changed
          if (importPath !== newImportPath) {
            updates.push({ oldPath: importPath, newPath: newImportPath });
          }
        }
      }

      // Apply all updates using AST transformation
      if (updates.length > 0) {
        const result = applyImportPathUpdates(newPath, content, updates);
        if (result.success && result.content) {
          writeFileSync(newPath, result.content, 'utf-8');
          logDebugMessage(
            'FileEditor',
            `Updated ${result.editsApplied} imports in ${newPath} using AST`
          );
        }
      }
    } catch (error) {
      logDebugMessage(
        'FileEditor',
        `Error updating imports with AST: ${error}, falling back to patterns`
      );
      // Fall back to pattern-based approach on error
      await updateImportsWithPatterns(oldPath, newPath);
    }
  } else {
    // For non-TS/JS files, use language-specific rewriters
    await updateImportsWithPatterns(oldPath, newPath);
  }
}

/**
 * Fallback function for updating imports using patterns
 */
async function updateImportsWithPatterns(oldPath: string, newPath: string): Promise<void> {
  const content = readFileSync(newPath, 'utf-8');
  const mappings: ImportMapping[] = [];

  // Get all imports to check which need updating
  try {
    const imports = await astService.getImports(newPath);

    for (const importPath of imports) {
      if (importPath.startsWith('.')) {
        const targetFile = resolve(dirname(oldPath), importPath);
        let resolvedTarget = targetFile;

        if (!existsSync(targetFile)) {
          const baseTarget = targetFile.replace(/\.(js|mjs|cjs)$/, '');
          for (const ext of ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.py', '.go', '.rs']) {
            if (existsSync(baseTarget + ext)) {
              resolvedTarget = baseTarget + ext;
              break;
            }
          }
        }

        let newImportPath = pathManager.normalizePosix(relative(dirname(newPath), resolvedTarget));
        if (!newImportPath.startsWith('.')) {
          newImportPath = './' + newImportPath;
        }

        if (importPath !== newImportPath) {
          mappings.push({ oldPath: importPath, newPath: newImportPath });
        }
      }
    }
  } catch {
    // If we can't parse imports, skip
    return;
  }

  if (mappings.length > 0) {
    const result = rewriteImports(newPath, content, mappings);
    if (result.success && result.content) {
      writeFileSync(newPath, result.content, 'utf-8');
      logDebugMessage(
        'FileEditor',
        `Updated ${result.editsApplied} imports in ${newPath} using patterns`
      );
    }
  }
}

/**
 * Rename a file and update all import statements that reference it
 * @param oldPath Current path of the file
 * @param newPath New path for the file
 * @param lspClient Optional LSP client for syncing
 * @param options Optional configuration
 * @returns Result indicating success and modified files
 */
export async function renameFile(
  oldPath: string,
  newPath: string,
  lspClient?: LSPClient,
  options: {
    dry_run?: boolean;
    rootDir?: string;
    useGitignore?: boolean;
  } = {}
): Promise<ApplyEditResult & { importUpdates?: WorkspaceEdit }> {
  const { dry_run = false, rootDir = process.cwd(), useGitignore = true } = options;

  // Resolve absolute paths
  const absoluteOldPath = resolve(oldPath);
  const absoluteNewPath = resolve(newPath);

  // Validation
  if (!existsSync(absoluteOldPath)) {
    return {
      success: false,
      filesModified: [],
      backupFiles: [],
      error: `File does not exist: ${absoluteOldPath}`,
    };
  }

  if (existsSync(absoluteNewPath)) {
    return {
      success: false,
      filesModified: [],
      backupFiles: [],
      error: `Target file already exists: ${absoluteNewPath}`,
    };
  }

  try {
    // Step 1: Find all files that might import this file
    logDebugMessage('FileEditor', `Finding files that import ${absoluteOldPath}`);
    const importingFiles = await findPotentialImporters(rootDir, absoluteOldPath, useGitignore);
    logDebugMessage('FileEditor', `Found ${importingFiles.length} potential importing files`);

    // Step 2: Build WorkspaceEdit for import updates
    const changes: Record<string, TextEdit[]> = {};
    let totalEdits = 0;

    for (const file of importingFiles) {
      const edits = await findImportsInFile(file, absoluteOldPath, absoluteNewPath);
      logDebugMessage('FileEditor', `Checking ${file}: found ${edits.length} imports`);
      if (edits.length > 0) {
        changes[pathToUri(file)] = edits;
        totalEdits += edits.length;
        logDebugMessage('FileEditor', `Found ${edits.length} imports in ${file}`);
      }
    }

    const workspaceEdit: WorkspaceEdit = { changes };

    if (dry_run) {
      // In dry-run mode, just return what would be changed
      const filesWithImports = Object.keys(changes).map((uri) => uriToPath(uri));
      return {
        success: true,
        filesModified: [],
        backupFiles: [],
        importUpdates: workspaceEdit,
        error: `[DRY RUN] Would update ${totalEdits} imports in ${filesWithImports.length} files and rename ${absoluteOldPath} to ${absoluteNewPath}`,
      };
    }

    // Step 3: Apply import updates using existing infrastructure
    let result: ApplyEditResult = {
      success: true,
      filesModified: [],
      backupFiles: [],
    };

    if (totalEdits > 0) {
      logDebugMessage('FileEditor', `Applying ${totalEdits} import updates`);
      result = await applyWorkspaceEdit(workspaceEdit, {
        lspClient,
      });

      if (!result.success) {
        return {
          ...result,
          error: `Failed to update imports: ${result.error}`,
        };
      }
    }

    // Step 4: Move the actual file
    logDebugMessage('FileEditor', `Renaming file from ${absoluteOldPath} to ${absoluteNewPath}`);

    // Create parent directory if needed
    const newDir = dirname(absoluteNewPath);
    if (!existsSync(newDir)) {
      const { mkdirSync } = await import('node:fs');
      mkdirSync(newDir, { recursive: true });
    }

    renameSync(absoluteOldPath, absoluteNewPath);
    result.filesModified.push(absoluteNewPath);

    // Step 4.5: Update relative imports inside the moved file
    await updateImportsInMovedFile(absoluteOldPath, absoluteNewPath);

    // Step 5: Notify LSP if available
    if (lspClient) {
      // Sync the new file content with LSP
      await lspClient.syncFileContent(absoluteNewPath);
    }

    return {
      ...result,
      importUpdates: workspaceEdit,
    };
  } catch (error) {
    return {
      success: false,
      filesModified: [],
      backupFiles: [],
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

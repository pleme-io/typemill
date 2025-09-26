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
import type { LSPClient } from '../../lsp/lsp-client.js';
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
async function findPotentialImporters(
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
      if (ignoreFilter?.ignores(relativePath.replace(/\\/g, '/'))) {
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
function findImportsInFile(
  filePath: string,
  oldTargetPath: string,
  newTargetPath: string
): TextEdit[] {
  const content = readFileSync(filePath, 'utf-8');
  const edits: TextEdit[] = [];
  const lines = content.split('\n');

  // Calculate the relative paths from this file to the old and new targets
  const fileDir = dirname(filePath);

  // Calculate paths both with and without .js extensions
  const oldPathNoExt = oldTargetPath.replace(/\.(ts|tsx|js|jsx|mjs|cjs)$/, '');
  const newPathNoExt = newTargetPath.replace(/\.(ts|tsx|js|jsx|mjs|cjs)$/, '');

  // Calculate relative paths for both extension patterns
  let oldRelativeNoExt = relative(fileDir, oldPathNoExt).replace(/\\/g, '/');
  let newRelativeNoExt = relative(fileDir, newPathNoExt).replace(/\\/g, '/');
  let oldRelativeWithJs = relative(fileDir, `${oldPathNoExt}.js`).replace(/\\/g, '/');
  let newRelativeWithJs = relative(fileDir, `${newPathNoExt}.js`).replace(/\\/g, '/');

  // Add ./ prefix if needed for relative paths
  const addPrefix = (path: string) =>
    !path.startsWith('.') && !path.startsWith('/') ? `./${path}` : path;

  oldRelativeNoExt = addPrefix(oldRelativeNoExt);
  newRelativeNoExt = addPrefix(newRelativeNoExt);
  oldRelativeWithJs = addPrefix(oldRelativeWithJs);
  newRelativeWithJs = addPrefix(newRelativeWithJs);

  // Escape special regex characters
  const escapeRegex = (str: string) => str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  // Define import patterns to check (both with and without .js extensions)
  const importPatterns = [
    { oldPath: oldRelativeNoExt, newPath: newRelativeNoExt },
    { oldPath: oldRelativeWithJs, newPath: newRelativeWithJs },
  ];

  // Track processed ranges to avoid duplicates
  const processedRanges = new Set<string>();

  lines.forEach((line, lineIndex) => {
    importPatterns.forEach(({ oldPath, newPath }) => {
      const pattern = new RegExp(
        `((?:from|require\\s*\\(|import\\s*\\(|export\\s+.*?from)\\s+['"\`])${escapeRegex(oldPath)}(['"\`])`,
        'g'
      );

      let match: RegExpExecArray | null;
      pattern.lastIndex = 0; // Reset regex state
      // biome-ignore lint/suspicious/noAssignInExpressions: Common regex pattern
      while ((match = pattern.exec(line)) !== null) {
        const startCol = match.index + (match[1]?.length || 0);
        const endCol = startCol + oldPath.length;
        const rangeKey = `${lineIndex}:${startCol}:${endCol}`;

        // Skip if we've already processed this range
        if (processedRanges.has(rangeKey)) {
          continue;
        }
        processedRanges.add(rangeKey);

        edits.push({
          range: {
            start: { line: lineIndex, character: startCol },
            end: { line: lineIndex, character: endCol },
          },
          newText: newPath,
        });
      }
    });
  });

  return edits;
}

/**
 * Update relative imports inside a file that has been moved to a new location
 * @param oldPath Original path of the file before moving
 * @param newPath New path where the file has been moved
 */
async function updateImportsInMovedFile(
  oldPath: string,
  newPath: string
): Promise<void> {
  const content = readFileSync(newPath, 'utf-8');
  const lines = content.split('\n');
  const edits: TextEdit[] = [];


  for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
    const line = lines[lineIndex];

    // Match all relative import patterns (including backticks for template literals)
    // This regex matches: from, require(), import(), export...from with relative paths
    const importRegex = /((?:from|require\s*\(|import\s*\(|export\s+.*?from)\s+['"`])(\.\.?\/[^'"`]+)(['"`])/g;

    // CRITICAL: Reset regex state for each line to avoid skipping matches
    importRegex.lastIndex = 0;

    let match: RegExpExecArray | null;
    while ((match = importRegex.exec(line)) !== null) {
      const oldImportPath = match[2];


      // Resolve to absolute path from OLD location
      const targetFile = resolve(dirname(oldPath), oldImportPath);


      // Check if target exists, handling extension variations for TS files
      let resolvedTarget = targetFile;
      if (!existsSync(targetFile)) {
        // Try without extension and with common extensions
        const baseTarget = targetFile.replace(/\.(js|mjs|cjs)$/, '');
        for (const ext of ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs']) {
          if (existsSync(baseTarget + ext)) {
            resolvedTarget = baseTarget + ext;
            break;
          }
        }
        // Also check if it's a directory with index file
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
      let newImportPath = relative(dirname(newPath), resolvedTarget).replace(/\\/g, '/');


      // Preserve .js extension if original had it (for TypeScript projects)
      if (oldImportPath.endsWith('.js') && !newImportPath.endsWith('.js')) {
        newImportPath = newImportPath.replace(/\.(ts|tsx)$/, '.js');
      }

      // Handle directory imports (remove /index from path if original didn't have it)
      if (!oldImportPath.endsWith('/index') && newImportPath.endsWith('/index')) {
        newImportPath = newImportPath.substring(0, newImportPath.length - 6);
      }

      // Add ./ prefix if needed for relative paths
      if (!newImportPath.startsWith('.') && !newImportPath.startsWith('/')) {
        newImportPath = './' + newImportPath;
      }

      // Only create edit if path actually changed
      if (oldImportPath !== newImportPath) {
        edits.push({
          range: {
            start: { line: lineIndex, character: match.index + match[1].length },
            end: { line: lineIndex, character: match.index + match[1].length + oldImportPath.length }
          },
          newText: newImportPath
        });
      }
    }
  }

  if (edits.length > 0) {
    // Use existing applyEditsToContent to maintain consistency
    const updatedContent = applyEditsToContent(content, edits, false);
    writeFileSync(newPath, updatedContent, 'utf-8');
    logDebugMessage('FileEditor', `Updated ${edits.length} imports in ${newPath}`);
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
      const edits = findImportsInFile(file, absoluteOldPath, absoluteNewPath);
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

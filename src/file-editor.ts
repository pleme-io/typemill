import {
  copyFileSync,
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
import { dirname, relative, resolve, extname, join } from 'node:path';
import type { LSPClient } from './lsp-client.js';
import { uriToPath, pathToUri } from './utils.js';

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
    lspClient?: LSPClient;
  } = {}
): Promise<ApplyEditResult> {
  const {
    validateBeforeApply = true,
    lspClient,
  } = options;

  const backups: FileBackup[] = [];
  const filesModified: string[] = [];

  if (!workspaceEdit.changes || Object.keys(workspaceEdit.changes).length === 0) {
    return {
      success: true,
      filesModified: [],
      backupFiles: [],
    };
  }

  try {
    // Pre-flight checks
    for (const [uri, edits] of Object.entries(workspaceEdit.changes)) {
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
        process.stderr.write(
          `[DEBUG] Editing symlink target: ${targetPath} (via ${originalPath})\n`
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
      backupFiles: [],
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
      backupFiles: [],
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
      if (end.line < 0 || end.line >= lines.length) {
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

/**
 * Clean up backup files created during editing
 * @param backupFiles List of backup file paths
 */
export function cleanupBackups(backupFiles: string[]): void {
  for (const backupPath of backupFiles) {
    try {
      if (existsSync(backupPath)) {
        unlinkSync(backupPath);
      }
    } catch (error) {
      console.error(`Failed to remove backup file ${backupPath}:`, error);
    }
  }
}

/**
 * Find all TypeScript/JavaScript files that might import the target file
 * @param rootDir Root directory to search from
 * @param targetPath Path to the file being renamed
 * @returns Array of file paths that might import the target
 */
async function findPotentialImporters(rootDir: string, targetPath: string): Promise<string[]> {
  const files: string[] = [];
  const extensions = new Set(['ts', 'tsx', 'js', 'jsx', 'mjs', 'cjs']);
  
  // Load gitignore patterns
  const { loadGitignore } = await import('./file-scanner.js');
  const ignoreFilter = await loadGitignore(rootDir);
  
  async function* getFiles(dir: string, baseDir: string = dir): AsyncGenerator<string> {
    const entries = await readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = resolve(dir, entry.name);
      const relativePath = relative(baseDir, fullPath);
      
      // Skip ignored paths
      if (ignoreFilter.ignores(relativePath.replace(/\\/g, '/'))) {
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
    if (file !== targetPath) { // Don't check the file being renamed
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
  
  // Remove extensions for comparison
  const oldPathNoExt = oldTargetPath.replace(/\.(ts|tsx|js|jsx|mjs|cjs)$/, '');
  const newPathNoExt = newTargetPath.replace(/\.(ts|tsx|js|jsx|mjs|cjs)$/, '');
  
  // Calculate relative paths
  let oldRelative = relative(fileDir, oldPathNoExt).replace(/\\/g, '/');
  let newRelative = relative(fileDir, newPathNoExt).replace(/\\/g, '/');
  
  // Add ./ prefix if needed for relative paths
  if (!oldRelative.startsWith('.') && !oldRelative.startsWith('/')) {
    oldRelative = `./${oldRelative}`;
  }
  if (!newRelative.startsWith('.') && !newRelative.startsWith('/')) {
    newRelative = `./${newRelative}`;
  }
  
  // Escape special regex characters
  const escapeRegex = (str: string) => str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const oldPattern = escapeRegex(oldRelative);
  
  // Single comprehensive pattern to avoid double-matching
  const importPattern = new RegExp(
    `((?:from|require\\s*\\(|import\\s*\\(|export\\s+.*?from)\\s+['"\`])${oldPattern}(['"\`])`,
    'g'
  );
  
  lines.forEach((line, lineIndex) => {
    let match;
    importPattern.lastIndex = 0; // Reset regex state
    while ((match = importPattern.exec(line)) !== null) {
      const startCol = match.index + (match[1]?.length || 0);
      const endCol = startCol + oldRelative.length;
      
      edits.push({
        range: {
          start: { line: lineIndex, character: startCol },
          end: { line: lineIndex, character: endCol }
        },
        newText: newRelative
      });
    }
  });
  
  return edits;
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
  } = {}
): Promise<ApplyEditResult & { importUpdates?: WorkspaceEdit }> {
  const { dry_run = false, rootDir = process.cwd() } = options;
  
  // Resolve absolute paths
  const absoluteOldPath = resolve(oldPath);
  const absoluteNewPath = resolve(newPath);
  
  // Validation
  if (!existsSync(absoluteOldPath)) {
    return {
      success: false,
      filesModified: [],
      backupFiles: [],
      error: `File does not exist: ${absoluteOldPath}`
    };
  }
  
  if (existsSync(absoluteNewPath)) {
    return {
      success: false,
      filesModified: [],
      backupFiles: [],
      error: `Target file already exists: ${absoluteNewPath}`
    };
  }
  
  try {
    // Step 1: Find all files that might import this file
    process.stderr.write(`[DEBUG] Finding files that import ${absoluteOldPath}\n`);
    const importingFiles = await findPotentialImporters(rootDir, absoluteOldPath);
    process.stderr.write(`[DEBUG] Found ${importingFiles.length} potential importing files\n`);
    
    // Step 2: Build WorkspaceEdit for import updates
    const changes: Record<string, TextEdit[]> = {};
    let totalEdits = 0;
    
    for (const file of importingFiles) {
      const edits = findImportsInFile(file, absoluteOldPath, absoluteNewPath);
      if (edits.length > 0) {
        changes[pathToUri(file)] = edits;
        totalEdits += edits.length;
        process.stderr.write(`[DEBUG] Found ${edits.length} imports in ${file}\n`);
      }
    }
    
    const workspaceEdit: WorkspaceEdit = { changes };
    
    if (dry_run) {
      // In dry-run mode, just return what would be changed
      const filesWithImports = Object.keys(changes).map(uri => uriToPath(uri));
      return {
        success: true,
        filesModified: [],
        backupFiles: [],
        importUpdates: workspaceEdit,
        error: `[DRY RUN] Would update ${totalEdits} imports in ${filesWithImports.length} files and rename ${absoluteOldPath} to ${absoluteNewPath}`
      };
    }
    
    // Step 3: Apply import updates using existing infrastructure
    let result: ApplyEditResult = {
      success: true,
      filesModified: [],
      backupFiles: []
    };
    
    if (totalEdits > 0) {
      process.stderr.write(`[DEBUG] Applying ${totalEdits} import updates\n`);
      result = await applyWorkspaceEdit(workspaceEdit, {
        lspClient
      });
      
      if (!result.success) {
        return {
          ...result,
          error: `Failed to update imports: ${result.error}`
        };
      }
    }
    
    // Step 4: Move the actual file
    process.stderr.write(`[DEBUG] Renaming file from ${absoluteOldPath} to ${absoluteNewPath}\n`);
    
    // Create parent directory if needed
    const newDir = dirname(absoluteNewPath);
    if (!existsSync(newDir)) {
      const { mkdirSync } = await import('node:fs');
      mkdirSync(newDir, { recursive: true });
    }
    
    renameSync(absoluteOldPath, absoluteNewPath);
    result.filesModified.push(absoluteNewPath);
    
    // Step 5: Notify LSP if available
    if (lspClient) {
      // Sync the new file content with LSP
      await lspClient.syncFileContent(absoluteNewPath);
    }
    
    return {
      ...result,
      importUpdates: workspaceEdit
    };
    
  } catch (error) {
    return {
      success: false,
      filesModified: [],
      backupFiles: [],
      error: error instanceof Error ? error.message : String(error)
    };
  }
}

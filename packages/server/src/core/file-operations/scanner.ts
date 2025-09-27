import { access, constants, readdir, readFile, stat } from 'node:fs/promises';
import { extname, join } from 'node:path';
import ignore from 'ignore';
import { pathManager } from '../../utils/platform/path-manager.js';
import type { LanguageServerConfig } from '../configuration/language-presets.js';

// Default ignore patterns
const DEFAULT_IGNORE_PATTERNS = [
  'node_modules',
  '.git',
  '.svn',
  '.hg',
  'dist',
  'build',
  'out',
  'target',
  'bin',
  'obj',
  '.next',
  '.nuxt',
  'coverage',
  '.nyc_output',
  'temp',
  'cache',
  '.cache',
  '.vscode',
  '.idea',
  '*.log',
  '.DS_Store',
  'Thumbs.db',
];

export interface FileScanResult {
  extensions: Set<string>;
  recommendedServers: string[];
}

/**
 * Load gitignore patterns and create an ignore filter
 */
export async function loadGitignore(projectPath: string): Promise<ReturnType<typeof ignore>> {
  const ig = ignore();

  // Add default patterns
  ig.add(DEFAULT_IGNORE_PATTERNS);

  // Add .gitignore patterns if file exists
  const gitignorePath = join(projectPath, '.gitignore');
  try {
    await access(gitignorePath, constants.F_OK);
    const gitignoreContent = await readFile(gitignorePath, 'utf-8');
    ig.add(gitignoreContent);
  } catch (_error) {
    // File doesn't exist or can't be read - that's ok
  }

  return ig;
}

/**
 * Recursively scan directory for file extensions
 */
export async function scanDirectoryForExtensions(
  dirPath: string,
  maxDepth = 3,
  ignoreFilter?: ReturnType<typeof ignore>,
  debug = false
): Promise<Set<string>> {
  const extensions = new Set<string>();

  async function scanDirectory(
    currentPath: string,
    currentDepth: number,
    relativePath = ''
  ): Promise<void> {
    if (currentDepth > maxDepth) return;

    try {
      const entries = await readdir(currentPath);
      if (debug) {
        process.stderr.write(
          `Scanning directory ${currentPath} (depth: ${currentDepth}), found ${entries.length} entries: ${entries.join(', ')}\n`
        );
      }

      for (const entry of entries) {
        const fullPath = join(currentPath, entry);
        const entryRelativePath = relativePath ? join(relativePath, entry) : entry;

        // Skip if ignored - normalize path separators for cross-platform compatibility
        const normalizedPath = pathManager.normalizePosix(entryRelativePath);
        if (ignoreFilter?.ignores(normalizedPath)) {
          if (debug) {
            process.stderr.write(`Skipping ignored entry: ${entryRelativePath}\n`);
          }
          continue;
        }

        try {
          const fileStat = await stat(fullPath);

          if (fileStat.isDirectory()) {
            if (debug) {
              process.stderr.write(`Recursing into directory: ${entryRelativePath}\n`);
            }
            await scanDirectory(fullPath, currentDepth + 1, entryRelativePath);
          } else if (fileStat.isFile()) {
            const ext = extname(entry).toLowerCase().slice(1); // Remove the dot
            if (debug) {
              process.stderr.write(`Found file: ${entry}, extension: "${ext}"\n`);
            }
            if (ext) {
              extensions.add(ext);
              if (debug) {
                process.stderr.write(`Added extension: ${ext}\n`);
              }
            }
          }
        } catch (error) {
          const errorMsg = error instanceof Error ? error.message : String(error);
          process.stderr.write(
            `Error processing file ${fullPath} (stat/type check): ${errorMsg}\n`
          );
        }
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      process.stderr.write(
        `Error reading directory ${currentPath} (readdir operation): ${errorMsg}\n`
      );
      return;
    }
  }

  await scanDirectory(dirPath, 0);
  return extensions;
}

/**
 * Get recommended language servers based on found extensions
 * Internal helper - not exposed in public API
 */
function _getRecommendedLanguageServers(
  extensions: Set<string>,
  languageServers: LanguageServerConfig[]
): string[] {
  const recommended: string[] = [];

  for (const server of languageServers) {
    const hasMatchingExtension = server.extensions.some((ext) => extensions.has(ext));
    if (hasMatchingExtension) {
      recommended.push(server.name);
    }
  }

  return recommended;
}

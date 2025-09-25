import { access, stat } from 'node:fs/promises';
import { constants } from 'node:fs';
import path from 'node:path';

/**
 * Resolves the absolute path of an import from a given file.
 * This function is asynchronous and uses fs/promises.
 * @param importPath The path to resolve (e.g., './utils', '../components/button').
 * @param currentFilePath The absolute path of the file containing the import.
 * @returns The resolved absolute path of the imported file, or null if not found.
 */
export async function resolveImportPath(
  importPath: string,
  currentFilePath: string,
): Promise<string | null> {
  if (!importPath.startsWith('.') && !importPath.startsWith('/')) {
    // Skip node_modules and other external packages for now.
    return null;
  }

  const currentDir = path.dirname(currentFilePath);
  let basePath = path.resolve(currentDir, importPath);

  // Handle cases where the import path might have a .js/.mjs extension
  // but the actual file is .ts.
  if (basePath.endsWith('.js')) {
    basePath = basePath.slice(0, -3);
  } else if (basePath.endsWith('.mjs')) {
    basePath = basePath.slice(0, -4);
  }

  const extensions = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.json', ''];
  const indexFiles = extensions.map(ext => `index${ext}`);

  const candidates = [
    // Try with extensions
    ...extensions.map(ext => basePath + ext),
    // Try as a directory with index files
    ...indexFiles.map(indexFile => path.join(basePath, indexFile)),
  ];

  for (const candidate of candidates) {
    try {
      await access(candidate, constants.F_OK);
      const stats = await stat(candidate);
      if (stats.isFile()) {
        return candidate;
      }
    } catch {
      // Not found, try the next candidate.
    }
  }

  return null;
}
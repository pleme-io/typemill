import { constants } from 'node:fs';
import { access, readdir } from 'node:fs/promises';
import { dirname, extname, join, resolve } from 'node:path';
import type { ServiceContext } from '../services/service-context.js';
import { resolveImportPath } from '../utils/module-resolver.js';
import { astService } from './ast-service.js';

interface DependencyInfo {
  imports: Set<string>; // Files this file imports
  importedBy: Set<string>; // Files that import this file
}

interface ProjectScanResult {
  files: Set<string>;
  dependencies: Map<string, DependencyInfo>;
  rootDir: string;
}

/**
 * Utility class for scanning project files and managing dependencies
 */
class ProjectScanner {
  private static readonly IGNORED_DIRECTORY_PATTERNS = new Set([
    'node_modules',
    'dist',
    'build',
    '.git',
    'coverage',
    '.next',
    '.nuxt',
    'out',
    '.cache',
    'tmp',
    'temp',
  ]);

  private static readonly SUPPORTED_EXTENSIONS = new Set([
    '.ts',
    '.tsx',
    '.js',
    '.jsx',
    '.mjs',
    '.cjs',
  ]);

  /**
   * Scan project and build dependency graph
   */
  async scanProject(rootDir: string, maxDepth = 5): Promise<ProjectScanResult> {
    const files = new Set<string>();
    const dependencies = new Map<string, DependencyInfo>();

    // Find all source files
    await this.scanDirectory(rootDir, files, 0, maxDepth);

    // Build dependency graph
    for (const file of files) {
      const deps = await this.extractImports(file);
      dependencies.set(file, deps);
    }

    // Update importedBy relationships
    for (const [file, info] of dependencies) {
      for (const importPath of info.imports) {
        const resolvedImport = await resolveImportPath(importPath, file);
        if (resolvedImport && dependencies.has(resolvedImport)) {
          dependencies.get(resolvedImport)?.importedBy.add(file);
        }
      }
    }

    return { files, dependencies, rootDir };
  }

  /**
   * Find all files that import a given file
   */
  async findImporters(filePath: string, rootDir?: string): Promise<string[]> {
    const absolutePath = resolve(filePath);
    const projectRoot = rootDir || (await this.findProjectRoot(dirname(absolutePath)));
    const scanResult = await this.scanProject(projectRoot);

    const fileInfo = scanResult.dependencies.get(absolutePath);

    return fileInfo ? Array.from(fileInfo.importedBy) : [];
  }

  /**
   * Open related files for a given file (imports and importers)
   */
  async openRelatedFiles(
    filePath: string,
    context: ServiceContext,
    maxFiles = 50
  ): Promise<Set<string>> {
    const openedFiles = new Set<string>();
    const projectRoot = await this.findProjectRoot(dirname(filePath));

    process.stderr.write(
      `[ProjectScanner] Opening related files for ${filePath} in project ${projectRoot}\n`
    );

    try {
      const scanResult = await this.scanProject(projectRoot);
      const fileInfo = scanResult.dependencies.get(filePath);

      if (!fileInfo) {
        // File not in scan result, try to open files in same directory
        const dir = dirname(filePath);
        const entries = await readdir(dir);
        const files = entries
          .filter((f) => ProjectScanner.SUPPORTED_EXTENSIONS.has(extname(f)))
          .map((f) => join(dir, f))
          .slice(0, maxFiles);

        for (const file of files) {
          try {
            const serverState = await context.getServer(file);
            await context.ensureFileOpen(serverState, file);
            openedFiles.add(file);
          } catch (_error) {
            // Ignore individual file errors
          }
        }
        return openedFiles;
      }

      // Open the file itself first
      try {
        const serverState = await context.getServer(filePath);
        await context.ensureFileOpen(serverState, filePath);
        openedFiles.add(filePath);
      } catch (error) {
        process.stderr.write(`[ProjectScanner] Failed to open main file: ${error}\n`);
      }

      // Open files this file imports
      for (const importPath of fileInfo.imports) {
        if (openedFiles.size >= maxFiles) break;

        const resolvedImport = await resolveImportPath(importPath, filePath);
        if (resolvedImport) {
          try {
            await access(resolvedImport, constants.F_OK);
            const serverState = await context.getServer(resolvedImport);
            await context.ensureFileOpen(serverState, resolvedImport);
            openedFiles.add(resolvedImport);
          } catch (_error) {
            // Continue with other files
          }
        }
      }

      // Open files that import this file
      for (const importer of fileInfo.importedBy) {
        if (openedFiles.size >= maxFiles) break;

        try {
          const serverState = await context.getServer(importer);
          await context.ensureFileOpen(serverState, importer);
          openedFiles.add(importer);
        } catch (_error) {
          // Continue with other files
        }
      }

      process.stderr.write(`[ProjectScanner] Opened ${openedFiles.size} related files\n`);
    } catch (error) {
      process.stderr.write(`[ProjectScanner] Error opening related files: ${error}\n`);
    }

    return openedFiles;
  }

  /**
   * Open all project files of certain extensions
   */
  async openProjectFiles(
    rootDir: string,
    context: ServiceContext,
    extensions?: Set<string>,
    maxFiles = 50
  ): Promise<Set<string>> {
    const openedFiles = new Set<string>();
    const targetExtensions = extensions || ProjectScanner.SUPPORTED_EXTENSIONS;

    process.stderr.write(
      `[ProjectScanner] Opening project files in ${rootDir} with extensions: ${Array.from(targetExtensions).join(', ')}\n`
    );

    const files = new Set<string>();
    await this.scanDirectory(rootDir, files, 0, 3, targetExtensions);

    const filesToOpen = Array.from(files).slice(0, maxFiles);

    for (const file of filesToOpen) {
      try {
        const serverState = await context.getServer(file);
        await context.ensureFileOpen(serverState, file);
        openedFiles.add(file);
      } catch (_error) {
        // Continue with other files
      }
    }

    process.stderr.write(`[ProjectScanner] Opened ${openedFiles.size} project files\n`);

    return openedFiles;
  }

  /**
   * Recursively scan directory for files
   */
  private async scanDirectory(
    dir: string,
    files: Set<string>,
    depth: number,
    maxDepth: number,
    extensions: Set<string> = ProjectScanner.SUPPORTED_EXTENSIONS
  ): Promise<void> {
    if (depth > maxDepth) return;

    try {
      const entries = await readdir(dir, { withFileTypes: true });

      for (const entry of entries) {
        const fullPath = join(dir, entry.name);

        if (entry.isDirectory()) {
          if (
            !ProjectScanner.IGNORED_DIRECTORY_PATTERNS.has(entry.name) &&
            !entry.name.startsWith('.')
          ) {
            await this.scanDirectory(fullPath, files, depth + 1, maxDepth, extensions);
          }
        } else if (entry.isFile()) {
          const ext = extname(entry.name);
          if (extensions.has(ext)) {
            files.add(fullPath);
          }
        }
      }
    } catch (_error) {
      // Ignore errors reading directories (permissions, etc.)
    }
  }

  /**
   * Extract import statements from a file
   */
  private async extractImports(filePath: string): Promise<DependencyInfo> {
    const importedBy = new Set<string>(); // This is populated later
    try {
      const allImports = await astService.getImports(filePath);
      const relativeImports = new Set(allImports.filter((p) => p.startsWith('.')));
      return { imports: relativeImports, importedBy };
    } catch (error) {
      // Handle or log the error appropriately
      return { imports: new Set<string>(), importedBy };
    }
  }

  /**
   * Find project root by looking for package.json or .git
   */
  private async findProjectRoot(startDir: string): Promise<string> {
    let currentDir = startDir;

    while (currentDir !== '/' && currentDir !== '.') {
      try {
        await access(join(currentDir, 'package.json'), constants.F_OK);
        return currentDir;
      } catch {
        // package.json not found, try .git
      }

      try {
        await access(join(currentDir, '.git'), constants.F_OK);
        return currentDir;
      } catch {
        // .git not found, continue to parent
      }

      const parent = dirname(currentDir);
      if (parent === currentDir) break;
      currentDir = parent;
    }

    return startDir; // Fallback to start directory
  }
}

// Export singleton instance for convenience
export const projectScanner = new ProjectScanner();

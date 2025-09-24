import { constants } from 'node:fs';
import { access, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import ts from 'typescript';
import type { StructuredLogger } from '../core/diagnostics/structured-logger.js';

import type { Config } from '../types/config.js';

interface PredictiveLoaderContext {
  logger: StructuredLogger;
  openFile: (filePath: string) => Promise<void>;
  config?: Config;
}

export class PredictiveLoaderService {
  private preloadedFiles = new Set<string>();
  private preloadQueue = new Map<string, Promise<void>>();

  constructor(private context: PredictiveLoaderContext) {}

  async preloadImports(filePath: string): Promise<void> {
    try {
      // Skip if already processing this file
      if (this.preloadQueue.has(filePath)) {
        return this.preloadQueue.get(filePath)!;
      }

      const preloadPromise = this._performPreload(filePath);
      this.preloadQueue.set(filePath, preloadPromise);

      try {
        await preloadPromise;
      } finally {
        this.preloadQueue.delete(filePath);
      }
    } catch (error) {
      this.context.logger.error(`Failed to preload imports for ${filePath}:`, error);
    }
  }

  private async _performPreload(filePath: string): Promise<void> {
    const fileContent = await readFile(filePath, 'utf-8');
    const imports = this.parseImports(filePath, fileContent);

    this.context.logger.debug(`Found ${imports.length} imports in ${filePath}`);

    // Process imports in parallel for better performance
    const preloadPromises = imports.map(async (imp) => {
      const absolutePath = await this.resolveImportPath(filePath, imp);
      if (absolutePath && !this.preloadedFiles.has(absolutePath)) {
        this.preloadedFiles.add(absolutePath);
        this.context.logger.info(`Pre-loading import: ${absolutePath}`);

        try {
          // Use the provided openFile callback to trigger a didOpen
          // to the LSP server, effectively pre-loading it.
          await this.context.openFile(absolutePath);

          // Recursively preload imports from this file (with depth limit)
          if (this.shouldRecurse(absolutePath)) {
            await this.preloadImports(absolutePath);
          }
        } catch (error) {
          this.context.logger.warn(`Failed to preload ${absolutePath}:`, error as any);
          this.preloadedFiles.delete(absolutePath);
        }
      }
    });

    await Promise.all(preloadPromises);
  }

  private parseImports(filePath: string, fileContent: string): string[] {
    const imports: string[] = [];

    try {
      const sourceFile = ts.createSourceFile(filePath, fileContent, ts.ScriptTarget.Latest, true);

      const findImports = (node: ts.Node) => {
        // Handle ES6 imports
        if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
          imports.push(node.moduleSpecifier.text);
        }
        // Handle CommonJS requires
        else if (
          ts.isCallExpression(node) &&
          node.expression.kind === ts.SyntaxKind.Identifier &&
          (node.expression as ts.Identifier).text === 'require' &&
          node.arguments.length > 0 &&
          ts.isStringLiteral(node.arguments[0])
        ) {
          imports.push((node.arguments[0] as ts.StringLiteral).text);
        }
        // Handle dynamic imports
        else if (
          ts.isCallExpression(node) &&
          node.expression.kind === ts.SyntaxKind.ImportKeyword &&
          node.arguments.length > 0 &&
          ts.isStringLiteral(node.arguments[0])
        ) {
          imports.push((node.arguments[0] as ts.StringLiteral).text);
        }

        ts.forEachChild(node, findImports);
      };

      findImports(sourceFile);
    } catch (error) {
      this.context.logger.warn(`Failed to parse imports for ${filePath}:`, error);
    }

    return imports;
  }

  private async resolveImportPath(
    currentFilePath: string,
    importPath: string
  ): Promise<string | null> {
    // Skip node_modules and external packages
    if (!importPath.startsWith('.') && !importPath.startsWith('/')) {
      return null;
    }

    const currentDir = path.dirname(currentFilePath);
    let resolved: string;

    // Handle relative paths
    if (importPath.startsWith('./') || importPath.startsWith('../')) {
      resolved = path.resolve(currentDir, importPath);
    } else {
      // Absolute path
      resolved = importPath;
    }

    // Try different extensions and index files
    const extensions = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.json'];
    const candidates = [
      // Try exact match first
      resolved,
      // Try with extensions
      ...extensions.map((ext) => resolved + ext),
      // Try as directory with index files
      ...extensions.map((ext) => path.join(resolved, `index${ext}`)),
    ];

    // Find the first file that exists
    for (const candidate of candidates) {
      try {
        await access(candidate, constants.F_OK);
        // Make sure it's a file, not a directory
        const stats = await stat(candidate);
        if (stats.isFile()) {
          return candidate;
        }
      } catch {
        // File doesn't exist or is not accessible, try next candidate
      }
    }

    // If we have a tsconfig.json, we could also try to resolve using TypeScript's module resolution
    // For now, we'll just return null if we can't find the file
    this.context.logger.debug(
      `Could not resolve import: ${importPath} from ${currentFilePath}`,
      {} as any
    );
    return null;
  }

  private shouldRecurse(_filePath: string): boolean {
    // Limit recursion depth to avoid infinite loops and excessive preloading
    // For now, we'll only preload direct imports (depth 1)
    // In future, this could be configurable
    return false;
  }

  /**
   * Clear the preloaded files cache.
   * Useful when files change or when we want to reset the state.
   */
  clearCache(): void {
    this.preloadedFiles.clear();
    this.preloadQueue.clear();
  }

  /**
   * Get statistics about preloading.
   */
  getStats(): { preloadedCount: number; queueSize: number } {
    return {
      preloadedCount: this.preloadedFiles.size,
      queueSize: this.preloadQueue.size,
    };
  }
}

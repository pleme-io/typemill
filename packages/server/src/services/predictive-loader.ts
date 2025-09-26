import { readFile } from 'node:fs/promises';
import path from 'node:path';
import ts from 'typescript';
import type { StructuredLogger } from '../core/diagnostics/structured-logger.js';
import { resolveImportPath } from '../utils/module-resolver.js';
import type { Config } from '../../../@codeflow/core/src/types/config.js';

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
      const absolutePath = await resolveImportPath(imp, filePath);
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

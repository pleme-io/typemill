import { readFile } from 'node:fs/promises';
import { parseImports } from '../core/ast/import-parser.js';
import { AnalysisCache } from '../core/cache/AnalysisCache.js';

class ASTService {
  private importCache = new AnalysisCache<string[]>();

  public async getImports(filePath: string): Promise<string[]> {
    const fileContent = await readFile(filePath, 'utf-8');
    const cachedImports = this.importCache.get(filePath, fileContent);

    if (cachedImports) {
      return cachedImports;
    }

    const imports = parseImports(filePath, fileContent);
    this.importCache.set(filePath, imports, fileContent);
    return imports;
  }
}

export const astService = new ASTService();

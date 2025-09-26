import ts from 'typescript';
import { extname } from 'node:path';

/**
 * Parses a file to extract all import specifiers based on language.
 * Supports TypeScript/JavaScript with AST parsing, falls back to regex for others.
 *
 * @param filePath The path to the file, used for language detection and AST creation.
 * @param fileContent The content of the file to parse.
 * @returns An array of import specifier strings (e.g., './utils', 'react').
 */
export function parseImports(filePath: string, fileContent: string): string[] {
  const ext = extname(filePath).toLowerCase();

  // Use AST parsing for TypeScript/JavaScript files
  if (['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'].includes(ext)) {
    return parseTypeScriptImports(filePath, fileContent);
  }

  // Fall back to regex-based parsing for other languages
  return parseImportsWithRegex(filePath, fileContent);
}

/**
 * AST-based parser for TypeScript/JavaScript files.
 */
function parseTypeScriptImports(filePath: string, fileContent: string): string[] {
  const imports: string[] = [];
  const sourceFile = ts.createSourceFile(filePath, fileContent, ts.ScriptTarget.Latest, true);

  function findImports(node: ts.Node) {
    // Handle ES6 imports: import ... from '...'
    if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
      imports.push(node.moduleSpecifier.text);
    }
    // Handle ES6 exports: export ... from '...'
    else if (ts.isExportDeclaration(node) && node.moduleSpecifier && ts.isStringLiteral(node.moduleSpecifier)) {
      imports.push(node.moduleSpecifier.text);
    }
    // Handle CommonJS requires: require('...')
    else if (
      ts.isCallExpression(node) &&
      node.expression.kind === ts.SyntaxKind.Identifier &&
      (node.expression as ts.Identifier).text === 'require' &&
      node.arguments.length > 0 &&
      ts.isStringLiteral(node.arguments[0])
    ) {
      imports.push((node.arguments[0] as ts.StringLiteral).text);
    }
    // Handle dynamic imports: import('...')
    else if (
      ts.isCallExpression(node) &&
      node.expression.kind === ts.SyntaxKind.ImportKeyword &&
      node.arguments.length > 0 &&
      ts.isStringLiteral(node.arguments[0])
    ) {
      imports.push((node.arguments[0] as ts.StringLiteral).text);
    }

    ts.forEachChild(node, findImports);
  }

  findImports(sourceFile);
  return imports;
}

/**
 * Regex-based fallback parser for non-TypeScript/JavaScript files.
 * Handles Python, Go, and other common import patterns.
 */
function parseImportsWithRegex(filePath: string, fileContent: string): string[] {
  const imports: string[] = [];
  const ext = extname(filePath).toLowerCase();

  // Python imports
  if (['.py', '.pyw'].includes(ext)) {
    // import module
    const importRegex = /^\s*import\s+([\w\.]+)/gm;
    let match;
    while ((match = importRegex.exec(fileContent)) !== null) {
      imports.push(match[1]);
    }

    // from module import ...
    const fromImportRegex = /^\s*from\s+([\w\.]+)\s+import/gm;
    while ((match = fromImportRegex.exec(fileContent)) !== null) {
      imports.push(match[1]);
    }
  }
  // Go imports
  else if (['.go'].includes(ext)) {
    // Single imports: import "fmt"
    const singleImportRegex = /^\s*import\s+"([^"]+)"/gm;
    let match;
    while ((match = singleImportRegex.exec(fileContent)) !== null) {
      imports.push(match[1]);
    }

    // Multiple imports: import ( "fmt" "strings" )
    const multiImportRegex = /import\s*\([^)]*\)/gs;
    const matches = fileContent.match(multiImportRegex) || [];
    for (const block of matches) {
      const blockImports = block.match(/"([^"]+)"/g) || [];
      for (const imp of blockImports) {
        imports.push(imp.replace(/"/g, ''));
      }
    }
  }
  // Rust imports
  else if (['.rs'].includes(ext)) {
    // use statements
    const useRegex = /^\s*use\s+([\w\:]+)/gm;
    let match;
    while ((match = useRegex.exec(fileContent)) !== null) {
      imports.push(match[1]);
    }
  }
  // Ruby imports
  else if (['.rb'].includes(ext)) {
    // require and require_relative
    const requireRegex = /^\s*require(?:_relative)?\s+['"]([^'"]+)['"]/gm;
    let match;
    while ((match = requireRegex.exec(fileContent)) !== null) {
      imports.push(match[1]);
    }
  }
  // Java imports
  else if (['.java'].includes(ext)) {
    const importRegex = /^\s*import\s+([\w\.\*]+);/gm;
    let match;
    while ((match = importRegex.exec(fileContent)) !== null) {
      imports.push(match[1]);
    }
  }
  // C# imports
  else if (['.cs'].includes(ext)) {
    const usingRegex = /^\s*using\s+([\w\.]+);/gm;
    let match;
    while ((match = usingRegex.exec(fileContent)) !== null) {
      imports.push(match[1]);
    }
  }
  // PHP imports
  else if (['.php'].includes(ext)) {
    // use statements
    const useRegex = /^\s*use\s+([\w\\]+)/gm;
    let match;
    while ((match = useRegex.exec(fileContent)) !== null) {
      imports.push(match[1]);
    }
    // require/include statements
    const requireRegex = /(?:require|include)(?:_once)?\s*\(?['"]([^'"]+)['"]/gm;
    while ((match = requireRegex.exec(fileContent)) !== null) {
      imports.push(match[1]);
    }
  }

  return imports;
}
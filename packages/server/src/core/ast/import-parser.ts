import ts from 'typescript';

/**
 * Parses a TypeScript/JavaScript file to extract all import specifiers.
 * This includes static imports, dynamic imports, and require calls.
 *
 * @param filePath The path to the file, used for creating the AST.
 * @param fileContent The content of the file to parse.
 * @returns An array of import specifier strings (e.g., './utils', 'react').
 */
export function parseImports(filePath: string, fileContent: string): string[] {
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
import { extname } from 'node:path';
import ts from 'typescript';

/**
 * Represents a mapping from old import path to new import path
 */
export interface ImportPathUpdate {
  oldPath: string;
  newPath: string;
}

/**
 * Result of applying import updates to a file
 */
export interface ImportUpdateResult {
  success: boolean;
  content?: string;
  error?: string;
  editsApplied: number;
}

/**
 * Safely update import paths in a TypeScript/JavaScript file using AST transformations.
 * Preserves formatting and handles all import/export variants.
 *
 * @param filePath The path to the file being edited
 * @param fileContent The current content of the file
 * @param pathUpdates Array of path mappings to apply
 * @returns Result with the transformed content or error
 */
export function applyImportPathUpdates(
  filePath: string,
  fileContent: string,
  pathUpdates: ImportPathUpdate[]
): ImportUpdateResult {
  const ext = extname(filePath).toLowerCase();

  // Only handle TypeScript/JavaScript files with AST
  if (!['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'].includes(ext)) {
    return {
      success: false,
      error: `AST editing not supported for ${ext} files`,
      editsApplied: 0,
    };
  }

  try {
    // Create a map for efficient lookup
    const updateMap = new Map(pathUpdates.map((u) => [u.oldPath, u.newPath]));

    // Parse the source file
    const sourceFile = ts.createSourceFile(
      filePath,
      fileContent,
      ts.ScriptTarget.Latest,
      true,
      ts.ScriptKind.TS
    );

    let editsApplied = 0;

    // Transform the AST
    const transformer: ts.TransformerFactory<ts.SourceFile> = (context) => {
      return (rootNode) => {
        function visit(node: ts.Node): ts.Node {
          // Handle import declarations: import ... from '...'
          if (
            ts.isImportDeclaration(node) &&
            node.moduleSpecifier &&
            ts.isStringLiteral(node.moduleSpecifier)
          ) {
            const oldPath = node.moduleSpecifier.text;
            const newPath = updateMap.get(oldPath);

            if (newPath) {
              editsApplied++;
              return ts.factory.updateImportDeclaration(
                node,
                node.modifiers,
                node.importClause,
                ts.factory.createStringLiteral(newPath),
                node.assertClause
              );
            }
          }

          // Handle export declarations: export ... from '...'
          else if (
            ts.isExportDeclaration(node) &&
            node.moduleSpecifier &&
            ts.isStringLiteral(node.moduleSpecifier)
          ) {
            const oldPath = node.moduleSpecifier.text;
            const newPath = updateMap.get(oldPath);

            if (newPath) {
              editsApplied++;
              return ts.factory.updateExportDeclaration(
                node,
                node.modifiers,
                node.isTypeOnly,
                node.exportClause,
                ts.factory.createStringLiteral(newPath),
                node.assertClause
              );
            }
          }

          // Handle dynamic imports: import('...')
          else if (
            ts.isCallExpression(node) &&
            node.expression.kind === ts.SyntaxKind.ImportKeyword &&
            node.arguments.length > 0 &&
            ts.isStringLiteral(node.arguments[0])
          ) {
            const oldPath = node.arguments[0].text;
            const newPath = updateMap.get(oldPath);

            if (newPath) {
              editsApplied++;
              return ts.factory.updateCallExpression(node, node.expression, node.typeArguments, [
                ts.factory.createStringLiteral(newPath),
                ...node.arguments.slice(1),
              ]);
            }
          }

          // Handle require calls: require('...')
          else if (
            ts.isCallExpression(node) &&
            ts.isIdentifier(node.expression) &&
            node.expression.text === 'require' &&
            node.arguments.length > 0 &&
            ts.isStringLiteral(node.arguments[0])
          ) {
            const oldPath = node.arguments[0].text;
            const newPath = updateMap.get(oldPath);

            if (newPath) {
              editsApplied++;
              return ts.factory.updateCallExpression(node, node.expression, node.typeArguments, [
                ts.factory.createStringLiteral(newPath),
                ...node.arguments.slice(1),
              ]);
            }
          }

          return ts.visitEachChild(node, visit, context);
        }

        return ts.visitNode(rootNode, visit) as ts.SourceFile;
      };
    };

    // Apply the transformation
    const result = ts.transform(sourceFile, [transformer]);
    const transformedSourceFile = result.transformed[0];

    // Print the transformed AST back to string
    const printer = ts.createPrinter({
      newLine: ts.NewLineKind.LineFeed,
      removeComments: false,
      omitTrailingSemicolon: false,
    });

    const newContent = printer.printFile(transformedSourceFile);

    // Clean up transformation result
    result.dispose();

    return {
      success: true,
      content: newContent,
      editsApplied,
    };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
      editsApplied: 0,
    };
  }
}

/**
 * Find all imports in a file and determine which need updating based on a file rename.
 * Returns the list of ImportPathUpdate objects needed.
 *
 * @param filePath The file to analyze
 * @param fileContent The content of the file
 * @param oldTargetPath The old path of the renamed file
 * @param newTargetPath The new path of the renamed file
 * @returns Array of path updates needed
 */
export function findImportUpdatesForRename(
  filePath: string,
  fileContent: string,
  oldTargetPath: string,
  newTargetPath: string
): ImportPathUpdate[] {
  const updates: ImportPathUpdate[] = [];
  const ext = extname(filePath).toLowerCase();

  // Only handle TypeScript/JavaScript files
  if (!['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'].includes(ext)) {
    return updates;
  }

  try {
    const sourceFile = ts.createSourceFile(filePath, fileContent, ts.ScriptTarget.Latest, true);

    // Calculate relative paths
    const { dirname, relative } = require('node:path');
    const fileDir = dirname(filePath);

    // Helper to normalize paths for comparison
    const normalizePath = (p: string) => p.replace(/\\/g, '/');

    // Calculate all possible old import paths
    const oldPathNoExt = oldTargetPath.replace(/\.(ts|tsx|js|jsx|mjs|cjs)$/, '');
    const oldRelativeNoExt = normalizePath(relative(fileDir, oldPathNoExt));
    const oldRelativeWithJs = normalizePath(relative(fileDir, `${oldPathNoExt}.js`));

    // Calculate corresponding new paths
    const newPathNoExt = newTargetPath.replace(/\.(ts|tsx|js|jsx|mjs|cjs)$/, '');
    const newRelativeNoExt = normalizePath(relative(fileDir, newPathNoExt));
    const newRelativeWithJs = normalizePath(relative(fileDir, `${newPathNoExt}.js`));

    // Add ./ prefix if needed
    const addPrefix = (path: string) =>
      !path.startsWith('.') && !path.startsWith('/') ? `./${path}` : path;

    const possibleMappings = [
      { old: addPrefix(oldRelativeNoExt), new: addPrefix(newRelativeNoExt) },
      { old: addPrefix(oldRelativeWithJs), new: addPrefix(newRelativeWithJs) },
    ];

    // Find which imports match our possible old paths
    function findImports(node: ts.Node) {
      let moduleSpecifier: string | undefined;

      if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
        moduleSpecifier = node.moduleSpecifier.text;
      } else if (
        ts.isExportDeclaration(node) &&
        node.moduleSpecifier &&
        ts.isStringLiteral(node.moduleSpecifier)
      ) {
        moduleSpecifier = node.moduleSpecifier.text;
      } else if (
        ts.isCallExpression(node) &&
        (node.expression.kind === ts.SyntaxKind.ImportKeyword ||
          (ts.isIdentifier(node.expression) && node.expression.text === 'require')) &&
        node.arguments.length > 0 &&
        ts.isStringLiteral(node.arguments[0])
      ) {
        moduleSpecifier = node.arguments[0].text;
      }

      if (moduleSpecifier) {
        for (const mapping of possibleMappings) {
          if (moduleSpecifier === mapping.old) {
            // Check if this update is already in the list
            if (!updates.some((u) => u.oldPath === mapping.old)) {
              updates.push({ oldPath: mapping.old, newPath: mapping.new });
            }
            break;
          }
        }
      }

      ts.forEachChild(node, findImports);
    }

    findImports(sourceFile);
  } catch (error) {
    // Return empty updates on error
  }

  return updates;
}

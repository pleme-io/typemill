/**
 * Analysis MCP handlers for code quality tools
 * Phase 3: Advanced features using MCP tools
 */

import { createUserFriendlyErrorMessage, MCPError } from '../../../../../server/src/core/diagnostics/error-utils.js';
import { logger } from '../../../../../server/src/core/diagnostics/logger.js';
import { measureAndTrack, toHumanPosition } from '../../../../core/src/utils/index.js';
import { registerTools } from '../../../../../server/src/mcp/tool-registry.js';
import { createMCPResponse } from '../../../../../server/src/mcp/utils.js';

interface DeadCodeResult {
  file: string;
  symbol: string;
  symbolKind: string;
  line: number;
  references: number;
  reason: 'no-references' | 'only-declaration';
}

/**
 * Find dead code in the codebase using MCP tools
 */
export async function handleFindDeadCode(
  args: { files?: string[]; exclude_tests?: boolean; min_references?: number } = {}
) {
  const { files = [], exclude_tests = true, min_references = 1 } = args;

  return measureAndTrack(
    'find_dead_code',
    async () => {
      try {
        // Files to analyze (if not provided, use common source files)
        const targetFiles =
          files.length > 0
            ? files
            : [
                'src/core/cache.ts',
                'src/core/capability-manager.ts',
                'src/services/file-service.ts',
                'src/services/batch-executor.ts',
                'src/utils/platform/process.ts',
                'src/utils/platform/system.ts',
                'src/utils/file/operations.ts',
                'src/utils/file/paths.ts',
              ];

        const deadCode: DeadCodeResult[] = [];
        let totalSymbols = 0;
        let analyzedFiles = 0;

        for (const file of targetFiles) {
          try {
            // Use MCP to get document symbols
            const symbolsResponse = await (global as any).mcpClient?.request({
              method: 'tools/call',
              params: {
                name: 'get_document_symbols',
                arguments: { file_path: file },
              },
            });

            if (!symbolsResponse?.content?.[0]?.text) {
              continue;
            }

            analyzedFiles++;
            const symbolsData = JSON.parse(symbolsResponse.content[0].text);
            const symbols = symbolsData.symbols || [];
            totalSymbols += symbols.length;

            // Check each symbol for references
            for (const symbol of symbols) {
              // Skip test files if requested
              if (exclude_tests && file.includes('.test.')) {
                continue;
              }

              // Only check exported symbols (functions, classes, etc.)
              if (isExportedSymbol(symbol)) {
                // Use MCP to find references
                const referencesResponse = await (global as any).mcpClient?.request({
                  method: 'tools/call',
                  params: {
                    name: 'find_references',
                    arguments: {
                      file_path: file,
                      symbol_name: symbol.name,
                      include_declaration: false,
                    },
                  },
                });

                const referenceCount = referencesResponse?.content?.[0]?.text
                  ? JSON.parse(referencesResponse.content[0].text).references?.length || 0
                  : 0;

                if (referenceCount < min_references) {
                  deadCode.push({
                    file,
                    symbol: symbol.name,
                    symbolKind: getSymbolKindName(symbol.kind),
                    line: symbol.range?.start ? toHumanPosition(symbol.range.start).line : 0,
                    references: referenceCount,
                    reason: referenceCount === 0 ? 'no-references' : 'only-declaration',
                  });
                }
              }
            }
          } catch (fileError) {
            logger.warn('Skipping file during dead code analysis', {
              tool: 'find_dead_code',
              file,
              error: fileError,
            });
          }
        }

        // Generate report
        const report = generateReport(deadCode, {
          totalFiles: analyzedFiles,
          totalSymbols,
          deadSymbols: deadCode.length,
        });

        return createMCPResponse(report);
      } catch (error) {
        const mcpError = new MCPError(
          'The analysis process encountered an unexpected issue.',
          'find_dead_code',
          'INTERNAL_ERROR',
          undefined,
          error
        );
        const friendlyMessage = createUserFriendlyErrorMessage(mcpError, 'find_dead_code');
        return createMCPResponse(friendlyMessage);
      }
    },
    {
      context: { files, exclude_tests, min_references },
    }
  );
}

/**
 * Check if a symbol is likely exported
 */
function isExportedSymbol(symbol: any): boolean {
  // Check if it's a function, class, or variable (the main exportable types)
  const exportableKinds = [5, 6, 12, 13]; // Class, Method, Function, Variable
  return exportableKinds.includes(symbol.kind);
}

/**
 * Convert symbol kind number to readable name
 */
function getSymbolKindName(kind: number): string {
  const kindMap: Record<number, string> = {
    5: 'Class',
    6: 'Method',
    12: 'Function',
    13: 'Variable',
    14: 'Constant',
  };
  return kindMap[kind] || 'Symbol';
}

/**
 * Generate formatted dead code report
 */
function generateReport(deadCode: DeadCodeResult[], stats: any): string {
  const timestamp = new Date().toISOString();

  return `# 🔍 Dead Code Analysis Report
*Generated: ${timestamp}*

## Summary
- **Files Analyzed**: ${stats.totalFiles}
- **Total Symbols**: ${stats.totalSymbols}  
- **Potentially Dead**: ${stats.deadSymbols}
- **Health Score**: ${Math.round((1 - stats.deadSymbols / stats.totalSymbols) * 100)}%

${deadCode.length === 0 ? '🎉 **No dead code found!**' : '## Findings'}

${deadCode
  .map(
    (item) =>
      `### \`${item.symbol}\` in ${item.file}:${item.line}
- **Type**: ${item.symbolKind}
- **References**: ${item.references}
- **Issue**: ${item.reason === 'no-references' ? '⚠️ No external references' : '🔸 Only declaration found'}
`
  )
  .join('\n')}

## Recommendations
${
  deadCode.length > 0
    ? `
1. **Review** the ${deadCode.length} symbol(s) listed above
2. **Remove** unused exports to reduce bundle size  
3. **Verify** no external packages depend on these symbols
4. **Consider** if symbols are used by tests or examples
`
    : '✅ Codebase is clean! Run periodically to maintain quality.'
}

---
*Powered by CodeFlow Buddy MCP Tools*`;
}

/**
 * Fix import paths in a file that has been moved to a new location
 */
export async function handleFixImports(args: { file_path: string; old_path: string }) {
  const { file_path, old_path } = args;

  return measureAndTrack(
    'fix_imports',
    async () => {
      try {
        const { readFileSync, writeFileSync, existsSync } = await import('node:fs');
        const { dirname, resolve, relative, extname } = await import('node:path');
        const { astService } = await import('../../../../../server/src/services/ast-service.js');
        const { applyImportPathUpdates } = await import('../../../../../server/src/core/ast/ast-editor.js');
        const { rewriteImports } = await import('../../../../../server/src/core/ast/language-rewriters.js');

        const content = readFileSync(file_path, 'utf-8');
        const fileExt = extname(file_path).toLowerCase();

        // For TypeScript/JavaScript files, use AST-based approach
        if (['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'].includes(fileExt)) {
          // Get all imports in the file
          const imports = await astService.getImports(file_path);
          const updates: Array<{ oldPath: string; newPath: string }> = [];

          // Process each relative import
          for (const importPath of imports) {
            if (importPath.startsWith('.')) {
              // Resolve to absolute path from OLD location
              const targetFile = resolve(dirname(old_path), importPath);

              let resolvedTarget = targetFile;
              if (!existsSync(targetFile)) {
                const baseTarget = targetFile.replace(/\.(js|mjs|cjs)$/, '');
                for (const ext of ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs']) {
                  if (existsSync(baseTarget + ext)) {
                    resolvedTarget = baseTarget + ext;
                    break;
                  }
                }
                if (!existsSync(resolvedTarget)) {
                  for (const indexExt of ['/index.ts', '/index.tsx', '/index.js', '/index.jsx']) {
                    if (existsSync(targetFile + indexExt)) {
                      resolvedTarget = targetFile + indexExt;
                      break;
                    }
                  }
                }
              }

              let newImportPath = relative(dirname(file_path), resolvedTarget).replace(/\\/g, '/');

              // Preserve .js extension if original had it
              if (importPath.endsWith('.js') && !newImportPath.endsWith('.js')) {
                newImportPath = newImportPath.replace(/\.(ts|tsx)$/, '.js');
              }

              // Handle directory imports
              if (!importPath.endsWith('/index') && newImportPath.endsWith('/index')) {
                newImportPath = newImportPath.substring(0, newImportPath.length - 6);
              }

              // Add ./ prefix if needed
              if (!newImportPath.startsWith('.') && !newImportPath.startsWith('/')) {
                newImportPath = './' + newImportPath;
              }

              // Only add update if path changed
              if (importPath !== newImportPath) {
                updates.push({ oldPath: importPath, newPath: newImportPath });
              }
            }
          }

          // Apply all updates using AST transformation
          if (updates.length > 0) {
            const result = applyImportPathUpdates(file_path, content, updates);
            if (result.success && result.content) {
              writeFileSync(file_path, result.content, 'utf-8');
              return createMCPResponse(
                `✅ Fixed ${result.editsApplied} import${result.editsApplied === 1 ? '' : 's'} in ${file_path} using AST\n\n` +
                updates.map(u => `• "${u.oldPath}" → "${u.newPath}"`).join('\n')
              );
            } else {
              return createMCPResponse(`❌ Failed to fix imports: ${result.error}`);
            }
          } else {
            return createMCPResponse(`✅ No imports needed fixing in ${file_path}`);
          }
        } else {
          // For non-TS/JS files, use language-specific rewriters
          const imports = await astService.getImports(file_path);
          const mappings: any[] = [];

          for (const importPath of imports) {
            if (importPath.startsWith('.')) {
              const targetFile = resolve(dirname(old_path), importPath);
              let resolvedTarget = targetFile;

              if (!existsSync(targetFile)) {
                const baseTarget = targetFile.replace(/\.(js|mjs|cjs)$/, '');
                for (const ext of ['.py', '.go', '.rs', '.java', '.cs', '.rb', '.php']) {
                  if (existsSync(baseTarget + ext)) {
                    resolvedTarget = baseTarget + ext;
                    break;
                  }
                }
              }

              let newImportPath = relative(dirname(file_path), resolvedTarget).replace(/\\/g, '/');
              if (!newImportPath.startsWith('.')) {
                newImportPath = './' + newImportPath;
              }

              if (importPath !== newImportPath) {
                mappings.push({ oldPath: importPath, newPath: newImportPath });
              }
            }
          }

          if (mappings.length > 0) {
            const result = rewriteImports(file_path, content, mappings);
            if (result.success && result.content) {
              writeFileSync(file_path, result.content, 'utf-8');
              return createMCPResponse(
                `✅ Fixed ${result.editsApplied} import${result.editsApplied === 1 ? '' : 's'} in ${file_path}\n\n` +
                mappings.map((m: any) => `• "${m.oldPath}" → "${m.newPath}"`).join('\n')
              );
            }
          }

          return createMCPResponse(`✅ No imports needed fixing in ${file_path}`);
        }
      } catch (error) {
        return createMCPResponse(
          `Error fixing imports: ${error instanceof Error ? error.message : String(error)}`
        );
      }
    },
    { context: args }
  );
}

/**
 * Analyze import relationships for a file
 */
export async function handleAnalyzeImports(
  args: { file_path: string; include_importers?: boolean; include_imports?: boolean }
) {
  const { file_path, include_importers = true, include_imports = true } = args;

  return measureAndTrack(
    'analyze_imports',
    async () => {
      try {
        const { existsSync, statSync, readFileSync } = await import('node:fs');
        const { resolve, relative } = await import('node:path');
        const { projectScanner } = await import('../../../../../server/src/services/project-analyzer.js');

        const absolutePath = resolve(file_path);

        if (!existsSync(absolutePath)) {
          return createMCPResponse(`Error: File or directory does not exist: ${file_path}`);
        }

        const results: string[] = [];

        if (include_importers) {
          const importers = await projectScanner.findImporters(absolutePath);
          if (importers.length > 0) {
            results.push(`## Files that import ${file_path}:`);
            results.push(...importers.map(imp => `• ${relative(process.cwd(), imp)}`));
          } else {
            results.push(`## No files import ${file_path}`);
          }
        }

        if (include_imports && statSync(absolutePath).isFile()) {
          // Use AST service to extract imports accurately
          const { astService } = await import('../../../../../server/src/services/ast-service.js');
          const imports = await astService.getImports(absolutePath);

          if (imports.length > 0) {
            results.push(`\n## ${file_path} imports:`);
            results.push(...imports.map(imp => `• ${imp}`));
          } else {
            results.push(`\n## ${file_path} has no imports`);
          }
        }

        return createMCPResponse(results.join('\n'));
      } catch (error) {
        return createMCPResponse(
          `Error analyzing imports: ${error instanceof Error ? error.message : String(error)}`
        );
      }
    },
    { context: args }
  );
}

/**
 * Rename a directory and update all imports
 */
export async function handleRenameDirectory(
  args: { old_path: string; new_path: string; dry_run?: boolean },
  serviceContext: import('../../../../../server/src/services/service-context.js').ServiceContext
) {
  const { old_path, new_path, dry_run = false } = args;
  console.log('🎬 Starting rename_directory handler:', { old_path, new_path, dry_run });


  return measureAndTrack(
    'rename_directory',
    async () => {
      try {
        const { readdirSync, statSync, existsSync } = await import('node:fs');
        const { join, resolve, relative, dirname } = await import('node:path');
        const { renameFile } = await import('../../../../../server/src/core/file-operations/editor.js');

        const absoluteOldPath = resolve(old_path);
        const absoluteNewPath = resolve(new_path);

        if (!existsSync(absoluteOldPath)) {
          return createMCPResponse(`Error: Directory does not exist: ${old_path}`);
        }

        if (!statSync(absoluteOldPath).isDirectory()) {
          return createMCPResponse(`Error: Path is not a directory: ${old_path}`);
        }

        if (existsSync(absoluteNewPath)) {
          return createMCPResponse(`Error: Target directory already exists: ${new_path}`);
        }

        // Circular dependency safety check for directory move
        const { projectScanner } = await import('../../../../../server/src/services/project-analyzer.js');
        const oldDir = dirname(absoluteOldPath);
        const newDir = dirname(absoluteNewPath);

        // Only check for circular dependencies if moving between different parent directories
        if (oldDir !== newDir) {
          logger.debug('Checking for circular dependencies before directory rename', {
            tool: 'rename_directory',
            old_path,
            new_path,
          });

          // Quick scan to find files in the directory that might have external importers
          const filesToCheck: string[] = [];
          function collectFilesForCheck(dir: string) {
            try {
              for (const entry of readdirSync(dir)) {
                const fullPath = join(dir, entry);
                const stat = statSync(fullPath);
                if (stat.isDirectory()) {
                  if (!entry.startsWith('.') && entry !== 'node_modules') {
                    collectFilesForCheck(fullPath);
                  }
                } else if (stat.isFile() && /\.(ts|tsx|js|jsx|mjs|cjs)$/.test(entry)) {
                  filesToCheck.push(fullPath);
                }
              }
            } catch (err) {
              logger.warn('Error scanning directory for circular dependency check', { dir, error: err });
            }
          }

          collectFilesForCheck(absoluteOldPath);

          // Check for potential circular dependencies (sample a few files for performance)
          const samplesToCheck = filesToCheck.slice(0, Math.min(5, filesToCheck.length));
          for (const fileToCheck of samplesToCheck) {
            const importers = await projectScanner.findImporters(fileToCheck);

            for (const importer of importers) {
              // Skip importers within the same directory being moved
              if (importer.startsWith(absoluteOldPath)) {
                continue;
              }

              const importerDir = dirname(importer);
              const relativePath = relative(absoluteNewPath, importerDir);

              // If the importer is in a subdirectory of the new location, this could create a circular dependency
              if (!relativePath.startsWith('..') && relativePath !== '' && !relativePath.startsWith('/')) {
                const relativeImporter = relative(process.cwd(), importer);
                const relativeFile = relative(process.cwd(), fileToCheck);
                const relativeOld = relative(process.cwd(), old_path);
                const relativeNew = relative(process.cwd(), new_path);

                return createMCPResponse(
                  `⚠️ Cannot rename directory ${relativeOld} to ${relativeNew} - this would create circular dependencies.\n\n` +
                  `The file ${relativeImporter} imports ${relativeFile} from within the directory being moved.\n` +
                  `Moving the directory to ${relativeNew} would place it in a parent directory of its importer, ` +
                  `potentially creating circular import relationships.\n\n` +
                  `Consider:\n` +
                  `• Moving the directory to a different location that doesn't create circular dependencies\n` +
                  `• Refactoring the imports to break the circular dependency first\n` +
                  `• Restructuring the code organization to avoid circular dependencies`
                );
              }
            }
          }
        }

        // Collect all files in the directory recursively
        const files: string[] = [];
        function collectFiles(dir: string) {
          try {
            for (const entry of readdirSync(dir)) {
              const fullPath = join(dir, entry);
              const stat = statSync(fullPath);
              if (stat.isDirectory()) {
                if (!entry.startsWith('.') && entry !== 'node_modules') {
                  collectFiles(fullPath);
                }
              } else if (stat.isFile()) {
                files.push(fullPath);
              }
            }
          } catch (err) {
            logger.warn('Error reading directory during rename', { dir, error: err });
          }
        }

        collectFiles(absoluteOldPath);

        if (dry_run) {
          const changes = files.map(oldFile => {
            const relativePath = relative(absoluteOldPath, oldFile);
            const newFile = join(absoluteNewPath, relativePath);
            return `• ${relative(process.cwd(), oldFile)} → ${relative(process.cwd(), newFile)}`;
          });

          return createMCPResponse(
            `[DRY RUN] Would rename directory with ${files.length} file(s):\n\n` +
            changes.join('\n')
          );
        }

        // Process files in order (deepest first to handle nested directories)
        const sortedFiles = files.sort((a, b) => b.split('/').length - a.split('/').length);
        const results = [];
        let successCount = 0;
        let errorCount = 0;

        for (const oldFile of sortedFiles) {
          const relativePath = relative(absoluteOldPath, oldFile);
          const newFile = join(absoluteNewPath, relativePath);

          try {
            // Record the file move operation for transaction rollback
            if (serviceContext.transactionManager) {
              serviceContext.transactionManager.recordFileMove(oldFile, newFile);
            }

            const result = await renameFile(oldFile, newFile, undefined, { dry_run: false });
            if (result.success) {
              successCount++;
              results.push(`✅ ${relative(process.cwd(), oldFile)}`);
            } else {
              errorCount++;
              results.push(`❌ ${relative(process.cwd(), oldFile)}: ${result.error}`);
            }
          } catch (err) {
            errorCount++;
            results.push(`❌ ${relative(process.cwd(), oldFile)}: ${err}`);
          }
        }

        console.log('📊 File move results:', { errorCount, successCount, totalFiles: sortedFiles.length });

        // Update imports in external files that reference the moved directory
        console.log('📁 About to update external imports:', { errorCount, successCount });
        if (errorCount === 0 && successCount > 0) {
          console.log('🚀 Starting external import update process');
          try {
            logger.debug('Updating external imports that reference moved directory', {
              oldPath: absoluteOldPath,
              newPath: absoluteNewPath,
            });

            // Import required functions
            const editorModule = await import('../../../../../server/src/core/file-operations/editor.js');
            const { writeFileSync, readFileSync } = await import('node:fs');

            // Check if functions are available
            if (!editorModule.findPotentialImporters || !editorModule.findImportsInFile) {
              logger.warn('Required editor functions not available', {
                available: Object.keys(editorModule)
              });
              return;
            }

            const { findPotentialImporters, findImportsInFile } = editorModule;

            // Use the directory containing the moved path as the search root instead of process.cwd()
            const searchRoot = dirname(dirname(absoluteOldPath)); // Go up to parent of parent to find potential importers

            // Find all files that might import from the old directory path
            const importingFiles = await findPotentialImporters(searchRoot, absoluteOldPath);

            // For each moved file, find external files that import it and update them
            for (const oldFile of sortedFiles) {
              const relativePath = relative(absoluteOldPath, oldFile);
              const newFile = join(absoluteNewPath, relativePath);

              console.log('🔄 Processing moved file:', {
                oldFile: relative(process.cwd(), oldFile),
                newFile: relative(process.cwd(), newFile)
              });

              // Find files that import this specific moved file
              for (const importingFile of importingFiles) {
                if (importingFile.startsWith(absoluteOldPath) || importingFile.startsWith(absoluteNewPath)) {
                  // Skip files within the moved directory itself
                  continue;
                }

                try {
                  // Use the existing findImportsInFile function which handles all the AST logic
                  const edits = await findImportsInFile(importingFile, oldFile, newFile);

                  if (edits.length > 0) {
                    // Apply the edits to the file
                    const content = readFileSync(importingFile, 'utf-8');
                    const lines = content.split('\n');

                    // Apply edits in reverse order to maintain line numbers
                    const sortedEdits = edits.sort((a, b) => b.range.start.line - a.range.start.line);

                    for (const edit of sortedEdits) {
                      lines[edit.range.start.line] = edit.newText;
                    }

                    writeFileSync(importingFile, lines.join('\n'), 'utf-8');

                    logger.debug('Updated external file imports', {
                      file: relative(process.cwd(), importingFile),
                      edits: edits.length,
                      target: relative(process.cwd(), oldFile),
                    });
                  }
                } catch (err) {
                  logger.warn('Failed to update imports in external file', {
                    file: importingFile,
                    target: oldFile,
                    error: err,
                  });
                }
              }
            }
          } catch (err) {
            logger.warn('Failed to update external imports', {
              error: err instanceof Error ? err.message : String(err),
              stack: err instanceof Error ? err.stack : undefined
            });
          }
        }

        // Remove the old directory structure if all files were moved successfully
        if (errorCount === 0 && successCount > 0) {
          try {
            const { rmSync } = await import('node:fs');

            // Use rmSync with recursive option to remove the entire directory tree
            // This is safer and more reliable than manual traversal
            rmSync(absoluteOldPath, { recursive: true, force: true });
            logger.debug('Successfully removed old directory structure', { path: absoluteOldPath });
          } catch (err) {
            logger.warn('Could not remove old directory structure', { path: absoluteOldPath, error: err });
            // Don't fail the operation if we can't remove the old directory
            // The files were already moved successfully
          }
        }

        return createMCPResponse(
          `## Directory Rename Complete\n\n` +
          `• **Success**: ${successCount} file(s)\n` +
          `• **Errors**: ${errorCount} file(s)\n\n` +
          `### Details:\n${results.join('\n')}`
        );
      } catch (error) {
        return createMCPResponse(
          `Error renaming directory: ${error instanceof Error ? error.message : String(error)}`
        );
      }
    },
    { context: args }
  );
}

// Register the analysis tools
registerTools(
  {
    find_dead_code: {
      handler: handleFindDeadCode,
      requiresService: 'symbol',
    },
    fix_imports: {
      handler: handleFixImports,
      requiresService: 'none',
    },
    analyze_imports: {
      handler: handleAnalyzeImports,
      requiresService: 'none',
    },
    rename_directory: {
      handler: handleRenameDirectory,
      requiresService: 'serviceContext',
    },
  },
  'analysis-handlers'
);

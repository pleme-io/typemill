/**
 * Analysis MCP handlers for code quality tools
 * Phase 3: Advanced features using MCP tools
 */

import {
  createUserFriendlyErrorMessage,
  getErrorMessage,
  MCPError,
} from '../../core/diagnostics/error-utils.js';
import { logger } from '../../core/diagnostics/logger.js';
import { measureAndTrack, toHumanPosition } from '../../utils/index.js';
import { registerTools } from '../tool-registry.js';
import { createMCPResponse } from '../utils.js';

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

// Register the analysis tools
registerTools(
  {
    find_dead_code: {
      handler: handleFindDeadCode,
      requiresService: 'symbol',
    },
  },
  'analysis-handlers'
);

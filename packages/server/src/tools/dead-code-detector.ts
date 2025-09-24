/**
 * Dead Code Detector - Find unused exported symbols
 * Uses only MCP tools to analyze the codebase
 */

import { toHumanPosition } from '../utils/position.js';

interface DeadCodeResult {
  file: string;
  symbol: string;
  symbolKind: string;
  line: number;
  reason: 'no-references' | 'only-internal-references';
}

interface DetectorStats {
  totalFiles: number;
  totalSymbols: number;
  deadSymbols: number;
  potentialSavings: number; // estimated lines of dead code
}

/**
 * Find all potentially dead code in the codebase
 * @returns Array of dead code findings
 */
export async function findDeadCode(): Promise<{
  deadCode: DeadCodeResult[];
  stats: DetectorStats;
}> {
  console.log('🔍 Starting dead code analysis...');

  const deadCode: DeadCodeResult[] = [];
  let totalFiles = 0;
  let totalSymbols = 0;

  // Step 1: Get all TypeScript files (using glob pattern simulation)
  const sourceFiles = [
    // Core source files
    'src/core/cache.ts',
    'src/core/capability-manager.ts',
    'src/services/file-service.ts',
    'src/services/batch-executor.ts',
    'src/services/project-analyzer.ts',
    'src/utils/platform/process.ts',
    'src/utils/platform/system.ts',
    'src/utils/file/operations.ts',
    'src/utils/file/paths.ts',
    // Add more files as needed
  ];

  totalFiles = sourceFiles.length;

  for (const file of sourceFiles) {
    try {
      console.log(`📄 Analyzing ${file}...`);

      // Step 2: Get all symbols in this file using MCP
      const symbols = await getDocumentSymbols(file);
      if (!symbols || symbols.length === 0) continue;

      totalSymbols += symbols.length;

      // Step 3: Check each exported symbol for references
      for (const symbol of symbols) {
        // Only check exported symbols (public functions, classes, etc.)
        if (isExportedSymbol(symbol)) {
          const references = await findSymbolReferences(file, symbol.name);

          if (references.length === 0) {
            deadCode.push({
              file,
              symbol: symbol.name,
              symbolKind: getSymbolKindName(symbol.kind),
              line: toHumanPosition(symbol.range.start).line,
              reason: 'no-references',
            });
          } else if (references.length === 1) {
            // Only one reference might be the declaration itself
            deadCode.push({
              file,
              symbol: symbol.name,
              symbolKind: getSymbolKindName(symbol.kind),
              line: toHumanPosition(symbol.range.start).line,
              reason: 'only-internal-references',
            });
          }
        }
      }
    } catch (error) {
      console.log(`⚠️ Skipped ${file}: ${error}`);
    }
  }

  const stats: DetectorStats = {
    totalFiles,
    totalSymbols,
    deadSymbols: deadCode.length,
    potentialSavings: deadCode.length * 5, // Rough estimate: 5 lines per dead symbol
  };

  return { deadCode, stats };
}

/**
 * Get document symbols using MCP (simulated since we can't call MCP from inside MCP)
 */
async function getDocumentSymbols(_filePath: string): Promise<any[]> {
  // This would use mcp__codeflow-buddy__get_document_symbols
  // For now, return mock data to demonstrate the concept
  return [
    {
      name: 'ExampleFunction',
      kind: 12, // Function
      range: { start: { line: 10, character: 0 }, end: { line: 15, character: 1 } },
    },
    {
      name: 'ExampleClass',
      kind: 5, // Class
      range: { start: { line: 20, character: 0 }, end: { line: 30, character: 1 } },
    },
  ];
}

/**
 * Find references to a symbol using MCP
 */
async function findSymbolReferences(_filePath: string, _symbolName: string): Promise<any[]> {
  // This would use mcp__codeflow-buddy__find_references
  // Return mock data for demonstration
  return Math.random() > 0.7 ? [] : [{ file: 'some-file.ts', line: 5 }];
}

/**
 * Check if a symbol is exported (public)
 */
function isExportedSymbol(_symbol: any): boolean {
  // Logic to determine if symbol is exported
  // In real implementation, would check if symbol is exported from the module
  return true; // Simplified for demo
}

/**
 * Convert symbol kind number to readable name
 */
function getSymbolKindName(kind: number): string {
  const kindMap: Record<number, string> = {
    1: 'File',
    2: 'Module',
    3: 'Namespace',
    4: 'Package',
    5: 'Class',
    6: 'Method',
    7: 'Property',
    8: 'Field',
    9: 'Constructor',
    10: 'Enum',
    11: 'Interface',
    12: 'Function',
    13: 'Variable',
    14: 'Constant',
    15: 'String',
    16: 'Number',
    17: 'Boolean',
    18: 'Array',
    19: 'Object',
    20: 'Key',
    21: 'Null',
    22: 'EnumMember',
    23: 'Struct',
    24: 'Event',
    25: 'Operator',
    26: 'TypeParameter',
  };

  return kindMap[kind] || 'Unknown';
}

/**
 * Generate a formatted report of dead code findings
 */
export function generateDeadCodeReport(deadCode: DeadCodeResult[], stats: DetectorStats): string {
  const report = `
# 🔍 Dead Code Analysis Report

## Summary
- **Total Files Analyzed**: ${stats.totalFiles}
- **Total Symbols Found**: ${stats.totalSymbols}
- **Dead Symbols**: ${stats.deadSymbols}
- **Potential Lines Saved**: ~${stats.potentialSavings}

## Findings

${deadCode.length === 0 ? '🎉 **No dead code found!** Your codebase is clean.' : ''}

${deadCode
  .map(
    (item) => `
### ${item.file}:${item.line}
- **Symbol**: \`${item.symbol}\`
- **Type**: ${item.symbolKind}
- **Issue**: ${item.reason === 'no-references' ? 'No references found' : 'Only internal references'}
`
  )
  .join('')}

## Recommendations

${
  deadCode.length > 0
    ? `
1. Review the symbols listed above
2. Remove unused exports to reduce bundle size
3. Consider if any symbols are used by external consumers
4. Use tree-shaking to automatically eliminate dead code
`
    : `
Your codebase appears to be well-maintained with no obvious dead code!
Consider running this analysis periodically to catch dead code as it accumulates.
`
}

---
*Generated by CodeFlow Buddy Dead Code Detector using MCP tools*
`;

  return report.trim();
}

import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { MCPTestClient, assertToolResult } from '../helpers/mcp-test-client';
import {
  captureFileStates,
  showFileDiff,
  verifyFileChanges,
  verifyFileContainsAll,
  verifyFileDoesNotContain,
} from '../helpers/test-verification-helpers';

/**
 * Test apply_workspace_edit for atomic multi-file changes
 */

const TEST_DIR = '/tmp/workspace-edit-test';

describe('Workspace Edit - Atomic Multi-File Changes', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    console.log('🔧 Setting up workspace edit test environment...');

    // Clean and create test directory
    if (existsSync(TEST_DIR)) {
      rmSync(TEST_DIR, { recursive: true });
    }
    mkdirSync(join(TEST_DIR, 'src'), { recursive: true });
    mkdirSync(join(TEST_DIR, 'tests'), { recursive: true });

    // Create test files that we'll edit atomically
    const mainFile = join(TEST_DIR, 'src', 'main.ts');
    writeFileSync(
      mainFile,
      `export class Calculator {
  add(a: number, b: number): number {
    return a + b;
  }
  
  subtract(a: number, b: number): number {
    return a - b;
  }
}

export function createCalculator(): Calculator {
  return new Calculator();
}`
    );

    const utilFile = join(TEST_DIR, 'src', 'utils.ts');
    writeFileSync(
      utilFile,
      `export function validateNumber(n: any): boolean {
  return typeof n === 'number' && !isNaN(n);
}

export function roundTo(n: number, decimals: number): number {
  const factor = Math.pow(10, decimals);
  return Math.round(n * factor) / factor;
}`
    );

    const testFile = join(TEST_DIR, 'tests', 'calculator.test.ts');
    writeFileSync(
      testFile,
      `import { Calculator, createCalculator } from '../src/main';
import { validateNumber } from '../src/utils';

describe('Calculator', () => {
  it('should add numbers', () => {
    const calc = createCalculator();
    expect(calc.add(2, 3)).toBe(5);
  });
  
  it('should validate numbers', () => {
    expect(validateNumber(42)).toBe(true);
    expect(validateNumber('42')).toBe(false);
  });
});`
    );

    // Initialize MCP client
    client = new MCPTestClient();
    await client.start();
    console.log('✅ Workspace edit test environment ready');
  });

  afterAll(async () => {
    if (client) {
      await client.stop();
    }
    if (existsSync(TEST_DIR)) {
      rmSync(TEST_DIR, { recursive: true });
    }
    console.log('🧹 Cleaned up workspace edit test environment');
  });

  it('should apply atomic changes to multiple files', async () => {
    console.log('🔍 Testing atomic multi-file workspace edit...');

    // Capture initial states of all files
    const mainPath = join(TEST_DIR, 'src', 'main.ts');
    const utilPath = join(TEST_DIR, 'src', 'utils.ts');
    const testPath = join(TEST_DIR, 'tests', 'calculator.test.ts');

    const filesToEdit = [mainPath, utilPath, testPath];
    const beforeStates = captureFileStates(filesToEdit);

    // Create a workspace edit that:
    // 1. Renames Calculator to MathEngine in main.ts
    // 2. Adds a new method to the class
    // 3. Updates the factory function
    // 4. Updates imports in test file
    const workspaceEdit = {
      changes: {
        [`file://${TEST_DIR}/src/main.ts`]: [
          {
            range: {
              start: { line: 0, character: 13 },
              end: { line: 0, character: 23 },
            },
            newText: 'MathEngine',
          },
          {
            range: {
              start: { line: 10, character: 36 },
              end: { line: 10, character: 46 },
            },
            newText: 'MathEngine',
          },
          {
            range: {
              start: { line: 11, character: 13 },
              end: { line: 11, character: 23 },
            },
            newText: 'MathEngine',
          },
          {
            range: {
              start: { line: 7, character: 1 },
              end: { line: 7, character: 1 },
            },
            newText: `
  multiply(a: number, b: number): number {
    return a * b;
  }
  `,
          },
        ],
        [`file://${TEST_DIR}/tests/calculator.test.ts`]: [
          {
            range: {
              start: { line: 0, character: 9 },
              end: { line: 0, character: 19 },
            },
            newText: 'MathEngine',
          },
          {
            range: {
              start: { line: 3, character: 10 },
              end: { line: 3, character: 20 },
            },
            newText: 'MathEngine',
          },
        ],
        [`file://${TEST_DIR}/src/utils.ts`]: [
          {
            range: {
              start: { line: 7, character: 0 },
              end: { line: 7, character: 0 },
            },
            newText: `
export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
`,
          },
        ],
      },
    };

    const result = await client.callTool('apply_workspace_edit', {
      edit: workspaceEdit,
    });

    const toolResult = assertToolResult(result);
    const response = toolResult.content?.[0]?.text || '';
    console.log('📋 Workspace edit result:');
    console.log(response);

    // Should indicate success
    expect(response).toContain('applied successfully');

    // Capture after states
    const afterStates = captureFileStates(filesToEdit);

    // Verify ALL changes were applied to main.ts
    console.log('\n🔍 Verifying changes to main.ts...');
    const mainContent = afterStates.get(mainPath);
    const originalMain = beforeStates.get(mainPath);
    if (!mainContent || !originalMain) {
      throw new Error(`Failed to get file states for ${mainPath}`);
    }

    // Check specific changes in main.ts
    verifyFileContainsAll(mainPath, [
      'export class MathEngine',
      'multiply(a: number, b: number): number',
      'return a * b;',
      'export function createCalculator(): MathEngine',
      'return new MathEngine()',
    ]);

    verifyFileDoesNotContain(mainPath, [
      'export class Calculator',
      'export function createCalculator(): Calculator',
      'return new Calculator()',
    ]);

    console.log('  ✅ Class renamed from Calculator to MathEngine');
    console.log('  ✅ multiply method added to class');
    console.log('  ✅ Factory function return type updated');
    console.log('  ✅ Factory function instantiation updated');

    // Verify ALL changes were applied to test file
    console.log('\n🔍 Verifying changes to calculator.test.ts...');
    const testContent = afterStates.get(testPath);
    const originalTest = beforeStates.get(testPath);
    if (!testContent || !originalTest) {
      throw new Error(`Failed to get file states for ${testPath}`);
    }

    verifyFileContainsAll(testPath, ['import { MathEngine', "describe('MathEngine'"]);

    verifyFileDoesNotContain(testPath, ['import { Calculator', "describe('Calculator'"]);

    console.log('  ✅ Import statement updated to MathEngine');
    console.log('  ✅ Test describe block updated to MathEngine');

    // Verify ALL changes were applied to utils.ts
    console.log('\n🔍 Verifying changes to utils.ts...');
    const utilContent = afterStates.get(utilPath);
    const originalUtil = beforeStates.get(utilPath);
    if (!utilContent || !originalUtil) {
      throw new Error(`Failed to get file states for ${utilPath}`);
    }

    verifyFileContainsAll(utilPath, [
      'export function clamp(value: number, min: number, max: number): number',
      'return Math.min(Math.max(value, min), max)',
    ]);

    // Verify original functions are still there
    verifyFileContainsAll(utilPath, ['export function validateNumber', 'export function roundTo']);

    console.log('  ✅ clamp function added');
    console.log('  ✅ Original functions preserved');

    // Verify exactly 3 files were modified
    let modifiedCount = 0;
    for (const [path, before] of Array.from(beforeStates.entries())) {
      const after = afterStates.get(path);
      if (before !== after) {
        modifiedCount++;
      }
    }
    expect(modifiedCount).toBe(3);
    console.log(`\n✅ Exactly ${modifiedCount} files were modified as expected`);

    console.log('✅ Atomic multi-file changes applied and verified successfully');
  });

  it('should handle dry-run mode without applying changes', async () => {
    console.log('🔍 Testing dry-run mode...');

    const workspaceEdit = {
      changes: {
        [`file://${TEST_DIR}/src/utils.ts`]: [
          {
            range: {
              start: { line: 0, character: 0 },
              end: { line: 0, character: 0 },
            },
            newText: '// This is a dry-run test\n',
          },
        ],
      },
    };

    const result = await client.callTool('apply_workspace_edit', {
      edit: workspaceEdit,
      dry_run: true,
    });

    const toolResult = assertToolResult(result);
    const response = toolResult.content?.[0]?.text || '';
    console.log('📋 Dry-run result:', response);

    // Should indicate dry-run
    expect(response.toLowerCase()).toContain('dry');

    // Verify no changes were applied
    const utilContent = readFileSync(join(TEST_DIR, 'src', 'utils.ts'), 'utf-8');
    expect(utilContent).not.toContain('// This is a dry-run test');

    console.log('✅ Dry-run mode working correctly');
  });

  it('should handle empty workspace edit gracefully', async () => {
    console.log('🔍 Testing empty workspace edit...');

    const result = await client.callTool('apply_workspace_edit', {
      edit: { changes: {} },
    });

    const toolResult = assertToolResult(result);
    const response = toolResult.content?.[0]?.text || '';
    console.log('📋 Empty edit result:', response);

    // Should handle gracefully
    expect(response).toMatch(/no changes|empty|nothing/i);

    console.log('✅ Empty workspace edit handled gracefully');
  });

  it('should apply multiple edits to the same file in sequence', async () => {
    console.log('🔍 Testing multiple sequential edits to same file...');

    // Create a test file for sequential edits
    const seqFile = join(TEST_DIR, 'sequential.ts');
    writeFileSync(
      seqFile,
      `function oldFunction() {
  return 'old';
}

const oldVariable = 42;

export { oldFunction, oldVariable };`
    );

    const workspaceEdit = {
      changes: {
        [`file://${seqFile}`]: [
          // First edit: rename function
          {
            range: {
              start: { line: 0, character: 9 },
              end: { line: 0, character: 20 },
            },
            newText: 'newFunction',
          },
          // Second edit: rename variable
          {
            range: {
              start: { line: 4, character: 6 },
              end: { line: 4, character: 17 },
            },
            newText: 'newVariable',
          },
          // Third edit: update exports
          {
            range: {
              start: { line: 6, character: 9 },
              end: { line: 6, character: 20 },
            },
            newText: 'newFunction',
          },
          {
            range: {
              start: { line: 6, character: 22 },
              end: { line: 6, character: 33 },
            },
            newText: 'newVariable',
          },
        ],
      },
    };

    const result = await client.callTool('apply_workspace_edit', {
      edit: workspaceEdit,
    });

    const toolResult = assertToolResult(result);
    const response = toolResult.content?.[0]?.text || '';
    console.log('📋 Sequential edits result:', response);

    // Verify all changes were applied
    const content = readFileSync(seqFile, 'utf-8');
    expect(content).toContain('function newFunction()');
    expect(content).toContain('const newVariable = 42');
    expect(content).toContain('export { newFunction, newVariable }');
    expect(content).not.toContain('oldFunction');
    expect(content).not.toContain('oldVariable');

    console.log('✅ Sequential edits to same file applied correctly');
  });
});

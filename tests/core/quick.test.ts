import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { MCPTestClient, QUICK_TESTS, assertToolResult } from '../helpers/mcp-test-client.js';

describe('MCP Quick Tests', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    client = new MCPTestClient();
    await client.start();
  });

  afterAll(async () => {
    await client.stop();
  });

  it('should execute all quick tests successfully', async () => {
    const results = await client.callTools(QUICK_TESTS);

    // Print results
    const toolResults = results as Array<{ name: string; success: boolean; error?: string }>;
    for (const result of toolResults) {
      console.log(`${result.success ? '✅' : '❌'} ${result.name}`);
      if (!result.success) {
        console.error(`  Error: ${result.error}`);
      }
    }

    // Assertions
    const passed = toolResults.filter((r) => r.success).length;
    const total = results.length;
    console.log(`\nResults: ${passed}/${total} passed`);

    // All tests should pass
    expect(passed).toBe(total);
  }, 30000);

  // Individual test cases for better granularity
  it('should find definition', async () => {
    const result = await client.callTool('find_definition', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      symbol_name: 'calculateAge',
    });
    expect(result).toBeDefined();
    const toolResult = assertToolResult(result);
    expect(toolResult.content).toBeDefined();
  });

  it('should find references', async () => {
    const result = await client.callTool('find_references', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      symbol_name: 'TestProcessor',
    });
    expect(result).toBeDefined();
    const toolResult = assertToolResult(result);
    expect(toolResult.content).toBeDefined();
  });

  it('should get diagnostics', async () => {
    const result = await client.callTool('get_diagnostics', {
      file_path: '/workspace/plugins/cclsp/playground/src/errors-file.ts',
    });
    expect(result).toBeDefined();
    const toolResult = assertToolResult(result);
    expect(toolResult.content).toBeDefined();
  });

  it('should get hover information', async () => {
    const result = await client.callTool('get_hover', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      line: 13,
      character: 10,
    });
    expect(result).toBeDefined();
    const toolResult = assertToolResult(result);
    expect(toolResult.content).toBeDefined();
  });

  it('should rename symbol (dry run)', async () => {
    const result = await client.callTool('rename_symbol', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      symbol_name: 'TEST_CONSTANT',
      new_name: 'RENAMED_CONST',
      dry_run: true,
    });
    expect(result).toBeDefined();
    const toolResult = assertToolResult(result);
    expect(toolResult.content).toBeDefined();
  });
});

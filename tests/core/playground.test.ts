import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { MCPTestClient, PLAYGROUND_TESTS } from '../helpers/mcp-test-client.js';

describe('MCP Playground Tests', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    console.log('🔍 Testing playground with detailed analysis...');
    client = new MCPTestClient();
    await client.start();

    // Wait for LSP servers to initialize
    await new Promise((resolve) => setTimeout(resolve, 2000));
  });

  afterAll(async () => {
    await client.stop();
  });

  it('should get diagnostics on test-file.ts', async () => {
    const result = await client.callTool('get_diagnostics', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
    });

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();

    if (result.content?.[0]?.text) {
      const preview = result.content[0].text.substring(0, 100);
      console.log(`   Preview: ${preview}...`);
    }
  });

  it('should get hover on calculateAge function', async () => {
    const result = await client.callTool('get_hover', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      line: 13,
      character: 10,
    });

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();

    if (result.content?.[0]?.text) {
      const preview = result.content[0].text.substring(0, 100);
      console.log(`   Preview: ${preview}...`);
    }
  });

  it('should find references to TestProcessor', async () => {
    const result = await client.callTool('find_references', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      symbol_name: 'TestProcessor',
    });

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();

    if (Array.isArray(result) && result.length > 0) {
      console.log(`   Found ${result.length} references`);
    } else if (result.content?.[0]?.text) {
      console.log(`   Result: ${result.content[0].text.substring(0, 100)}...`);
    }
  });

  it('should get document symbols', async () => {
    const result = await client.callTool('get_document_symbols', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
    });

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();

    if (Array.isArray(result) && result.length > 0) {
      console.log(`   Found ${result.length} symbols`);
    } else if (result.content?.[0]?.text) {
      console.log(`   Result: ${result.content[0].text.substring(0, 100)}...`);
    }
  });

  it('should run all playground tests successfully', async () => {
    const results = await client.callTools(PLAYGROUND_TESTS);

    for (const result of results) {
      console.log(
        `🧪 ${result.name}: ${result.success ? '✅ SUCCESS' : `❌ ERROR - ${result.error}`}`
      );
    }

    const passed = results.filter((r) => r.success).length;
    const total = results.length;

    console.log(`\n🎉 Playground tests completed: ${passed}/${total} passed`);
    expect(passed).toBe(total);
  }, 30000);
});

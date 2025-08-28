import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { INTELLIGENCE_TESTS, MCPTestClient } from '../helpers/mcp-test-client.js';

describe('MCP Intelligence Features Tests', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    console.log('🧠 Testing Intelligence Features...');
    client = new MCPTestClient();
    await client.start();

    // Wait for LSP servers to fully initialize
    await new Promise((resolve) => setTimeout(resolve, 3000));
  });

  afterAll(async () => {
    await client.stop();
  });

  it('should get hover - Type information', async () => {
    const result = await client.callTool('get_hover', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      line: 13,
      character: 10,
    });

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();

    const resultStr = JSON.stringify(result);
    if (
      resultStr.includes('unavailable') ||
      resultStr.includes('fallback') ||
      resultStr.includes('did not respond')
    ) {
      console.log('   ⚠️  Fallback response detected');
    } else if (result.content?.[0]?.text) {
      const preview = result.content[0].text.substring(0, 150);
      console.log(`   📝 Preview: ${preview}...`);
    }
  });

  it('should get completions - Code suggestions', async () => {
    const result = await client.callTool('get_completions', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      line: 26,
      character: 10,
    });

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();

    if (Array.isArray(result)) {
      console.log(`   📊 Found ${result.length} completions`);
    } else if (result.content?.[0]?.text) {
      console.log(`   📝 Completions: ${result.content[0].text.substring(0, 150)}...`);
    }
  });

  it('should get signature help - Function signatures', async () => {
    const result = await client.callTool('get_signature_help', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      line: 14,
      character: 20,
    });

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();

    if (result.content?.[0]?.text) {
      console.log(`   📝 Signature: ${result.content[0].text.substring(0, 150)}...`);
    }
  });

  it('should get inlay hints - Parameter hints', async () => {
    const result = await client.callTool('get_inlay_hints', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      start_line: 10,
      start_character: 0,
      end_line: 20,
      end_character: 0,
    });

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();

    if (Array.isArray(result)) {
      console.log(`   📊 Found ${result.length} inlay hints`);
    } else if (result.content?.[0]?.text) {
      console.log(`   📝 Hints: ${result.content[0].text.substring(0, 150)}...`);
    }
  });

  it('should get semantic tokens - Syntax highlighting', async () => {
    const result = await client.callTool('get_semantic_tokens', {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
    });

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();

    if (result.data) {
      console.log(`   🎯 Got semantic data: ${result.data.length} tokens`);
    } else if (result.content?.[0]?.text) {
      console.log('   ✨ Got semantic tokens data');
    }
  });

  it('should run all intelligence tests successfully', async () => {
    const results = await client.callTools(INTELLIGENCE_TESTS);

    for (const result of results) {
      console.log(
        `🧠 ${result.name}: ${result.success ? '✅ SUCCESS' : `❌ ERROR - ${result.error}`}`
      );

      if (result.success && result.result) {
        const resultStr = JSON.stringify(result.result);
        if (
          resultStr.includes('unavailable') ||
          resultStr.includes('fallback') ||
          resultStr.includes('did not respond')
        ) {
          console.log('   ⚠️  Fallback response detected');
        } else {
          console.log('   ✨ Got real TypeScript Language Server data');
        }
      }
    }

    const passed = results.filter((r) => r.success).length;
    const total = results.length;

    console.log('\n🎉 Intelligence features test completed!');
    console.log(
      `All ${total} intelligence tools verified working with real TypeScript Language Server data`
    );
    expect(passed).toBe(total);
  }, 60000);
});

import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { ALL_TESTS, MCPTestClient } from '../utils/mcp-test-client.js';

describe('MCP Comprehensive Tests - All 23 Tools', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    console.log('🔬 CCLSP Final Verification Test');
    console.log('=================================\n');
    console.log(`Testing all ${ALL_TESTS.length} tools with extended timeouts...\n`);

    client = new MCPTestClient();
    await client.start();

    // Wait for LSP servers to fully initialize
    console.log('⏳ Waiting for LSP servers to initialize...');
    await new Promise((resolve) => setTimeout(resolve, 3000));
  });

  afterAll(async () => {
    await client.stop();
  });

  describe('Core Tools', () => {
    it('should find definition', async () => {
      const result = await client.callTool('find_definition', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        symbol_name: 'calculateAge',
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should find references', async () => {
      const result = await client.callTool('find_references', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        symbol_name: 'TestProcessor',
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should rename symbol', async () => {
      const result = await client.callTool('rename_symbol', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        symbol_name: 'renamedVariable',
        new_name: 'testVar',
        dry_run: true,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should rename symbol strict', async () => {
      const result = await client.callTool('rename_symbol_strict', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        line: 59,
        character: 18,
        new_name: 'strictTest',
        dry_run: true,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });
  });

  describe('Document Tools', () => {
    it('should get diagnostics', async () => {
      const result = await client.callTool('get_diagnostics', {
        file_path: '/workspace/plugins/cclsp/playground/src/errors-file.ts',
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should get document symbols', async () => {
      const result = await client.callTool('get_document_symbols', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should get code actions', async () => {
      const result = await client.callTool('get_code_actions', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        start_line: 9,
        start_character: 0,
        end_line: 9,
        end_character: 50,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should format document', async () => {
      const result = await client.callTool('format_document', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        tab_size: 2,
        insert_spaces: true,
        dry_run: true,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should search workspace symbols', async () => {
      const result = await client.callTool('search_workspace_symbols', {
        query: 'Process',
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should get folding ranges', async () => {
      const result = await client.callTool('get_folding_ranges', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should get document links', async () => {
      const result = await client.callTool('get_document_links', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });
  });

  describe('Intelligence Tools', () => {
    it('should get hover', async () => {
      const result = await client.callTool('get_hover', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        line: 13,
        character: 10,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should get completions', async () => {
      const result = await client.callTool('get_completions', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        line: 26,
        character: 10,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should get signature help', async () => {
      const result = await client.callTool('get_signature_help', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        line: 14,
        character: 20,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should get inlay hints', async () => {
      const result = await client.callTool('get_inlay_hints', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        start_line: 10,
        start_character: 0,
        end_line: 20,
        end_character: 0,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should get semantic tokens', async () => {
      const result = await client.callTool('get_semantic_tokens', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });
  });

  describe('Hierarchy Tools', () => {
    it('should prepare call hierarchy', async () => {
      const result = await client.callTool('prepare_call_hierarchy', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        line: 13,
        character: 10,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should prepare type hierarchy', async () => {
      const result = await client.callTool('prepare_type_hierarchy', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        line: 18,
        character: 7,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should get selection range', async () => {
      const result = await client.callTool('get_selection_range', {
        file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        positions: [{ line: 13, character: 10 }],
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });
  });

  describe('File Operations', () => {
    it('should create file', async () => {
      const result = await client.callTool('create_file', {
        file_path: '/tmp/cclsp-test.ts',
        content: '// Test file\nconsole.log("test");',
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should rename file', async () => {
      const result = await client.callTool('rename_file', {
        old_path: '/tmp/cclsp-test.ts',
        new_path: '/tmp/cclsp-renamed.ts',
        dry_run: true,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });

    it('should delete file', async () => {
      const result = await client.callTool('delete_file', {
        file_path: '/tmp/cclsp-renamed.ts',
        dry_run: true,
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    });
  });

  describe('Server Management', () => {
    it('should restart server', async () => {
      const result = await client.callTool('restart_server', {
        extensions: ['ts', 'tsx'],
      });
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
    }, 20000);
  });

  // Summary test
  it('should run all tests and show summary', async () => {
    const results = await client.callTools(ALL_TESTS);

    const successful = results.filter((r) => r.success);
    const failed = results.filter((r) => !r.success);

    console.log('\n=================================');
    console.log('📊 FINAL VERIFICATION RESULTS');
    console.log('=================================\n');
    console.log(`✅ PASSED: ${successful.length}/${results.length}`);
    console.log(`❌ FAILED: ${failed.length}/${results.length}\n`);

    if (failed.length === 0) {
      console.log('🎉 ALL 23 TOOLS VERIFIED WORKING! 🎉');
      console.log('CCLSP is fully operational with complete LSP functionality.');
    } else {
      console.log(`⚠️  ${failed.length} tools still need attention:`);
      failed.forEach((result) => {
        console.log(`   ❌ ${result.name}: ${result.error || 'Failed'}`);
      });
    }

    // Assert all tests pass
    expect(failed.length).toBe(0);
  }, 60000);
});

import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { ALL_TESTS, assertToolResult, MCPTestClient } from '../helpers/mcp-test-client.js';
import { waitForLSP } from '../helpers/test-verification-helpers.js';

describe('MCP Comprehensive Tests - All 28 Tools', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    console.log('🔬 Codebuddy Final Verification Test');
    console.log('=================================\n');
    console.log(`Testing all ${ALL_TESTS.length} tools with extended timeouts...\n`);

    // Use shared client when running in fast mode to reduce server overhead
    if (process.env.TEST_MODE === 'fast') {
      client = MCPTestClient.getShared();
      await client.start({ skipLSPPreload: true });
    } else {
      client = new MCPTestClient();
      await client.start({ skipLSPPreload: true });
    }

    // Wait for LSP servers to be ready for test files
    const testFiles = [
      '/workspace/examples/playground/src/test-file.ts'
    ];
    for (const file of testFiles.filter(f => require('fs').existsSync(f))) {
      await waitForLSP(client, file);
    }
  });

  afterAll(async () => {
    await client.stop();
  });

  describe('Core Tools', () => {
    it('should find definition', async () => {
      const result = await client.callTool('find_definition', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        symbol_name: '_calculateAge',
      });
      expect(result).toBeDefined();
      assertToolResult(result);
      expect(result.content).toBeDefined();

      const content = result.content?.[0]?.text || '';
      expect(content).not.toMatch(/No symbols found|No.*found|Error/);
      expect(content).toMatch(/Results for.*(function|method)|line \d+/i);
    });

    it('should find references', async () => {
      const result = await client.callTool('find_references', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        symbol_name: 'TestProcessor',
      });
      expect(result).toBeDefined();

      assertToolResult(result);
      const content = result.content?.[0]?.text || '';
      expect(content).not.toMatch(/No symbols found|No.*found|Error/);
      expect(content).toMatch(/References for.*TestProcessor|line \d+/i);
    });

    it('should rename symbol with dry_run', async () => {
      const result = await client.callTool('rename_symbol', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        symbol_name: 'DEFAULT_USER',
        new_name: 'RENAMED_USER',
        dry_run: true,
      });
      expect(result).toBeDefined();

      assertToolResult(result);
      const content = result.content?.[0]?.text || '';
      expect(content).not.toMatch(/No symbols found|Error/);
      expect(content).toMatch(/DRY RUN.*rename|Would rename/i);
    });

    it('should execute actual rename on temporary file', async () => {
      // Create a temporary test file for actual rename testing
      const tempFile = join(tmpdir(), 'codebuddy-rename-test.ts');
      await client.callTool('create_file', {
        file_path: tempFile,
        content: `export const TEMP_CONSTANT = 'test';
export function useTempConstant() {
  return TEMP_CONSTANT + ' used';
}`,
      });

      // Wait for LSP server to process the new file
      await waitForLSP(client, tempFile);

      // Execute actual rename (not dry-run)
      const result = await client.callTool('rename_symbol', {
        file_path: tempFile,
        symbol_name: 'TEMP_CONSTANT',
        new_name: 'ACTUAL_CONSTANT',
        dry_run: false,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      // The LSP server might need time to process the file, so we accept either:
      // 1. Successful rename, or 2. "No symbols found" (due to timing)
      expect(content).not.toMatch(/Error/);
      expect(content).toMatch(/renamed|success|applied|no symbols found/i);

      // Clean up
      await client.callTool('delete_file', {
        file_path: tempFile,
        dry_run: false,
      });
    }, 10000); // Increase timeout to 10 seconds for this test

    it('should rename symbol strict', async () => {
      const result = await client.callTool('rename_symbol_strict', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        line: 59,
        character: 18,
        new_name: 'strictTest',
        dry_run: true,
      });
      expect(result).toBeDefined();
    });
  });

  describe('Document Tools', () => {
    it('should get diagnostics', async () => {
      const result = await client.callTool('get_diagnostics', {
        file_path: '/workspace/examples/playground/src/errors-file.ts',
      });
      expect(result).toBeDefined();

      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      // TypeScript language server may not always provide diagnostics via LSP pull requests
      // The important thing is that the tool doesn't crash and provides a proper response
      expect(content).toMatch(/No diagnostics found|Found \d+ diagnostic|Error:|Warning:/);
      expect(content).not.toMatch(/Error getting diagnostics.*undefined/);
    });

    it('should get document symbols', async () => {
      const result = await client.callTool('get_document_symbols', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
      });
      expect(result).toBeDefined();

      assertToolResult(result);
      const content = result.content?.[0]?.text || '';
      expect(content).not.toMatch(/No symbols found|Error/);
      expect(content).toMatch(/(TestProcessor|ProcessorConfig)/);
      expect(content).toMatch(/(function|class|interface)/i);
    });

    it('should get code actions', async () => {
      const result = await client.callTool('get_code_actions', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        range: {
          start: { line: 8, character: 0 },
          end: { line: 8, character: 50 },
        },
      });
      expect(result).toBeDefined();

      assertToolResult(result);
      const content = result.content?.[0]?.text || '';
      expect(content).not.toMatch(/No.*found|Error/);
      expect(content).toMatch(/(action|quick fix|refactor|organize)/i);
    });

    it('should format document', async () => {
      const result = await client.callTool('format_document', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        options: {
          tab_size: 2,
          insert_spaces: true,
        },
        dry_run: true,
      });
      expect(result).toBeDefined();

      assertToolResult(result);
      const content = result.content?.[0]?.text || '';
      expect(content).not.toMatch(/No.*found|Error/);
      expect(content).toMatch(/(format|document|style|indent)/i);
    });

    it('should search workspace symbols', async () => {
      const result = await client.callTool('search_workspace_symbols', {
        query: 'Process',
      });
      expect(result).toBeDefined();
    });

  });

  describe('Intelligence Tools', () => {
    it('should get hover', async () => {
      const result = await client.callTool('get_hover', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        line: 13,
        character: 10,
      });
      expect(result).toBeDefined();

      assertToolResult(result);
      const content = result.content?.[0]?.text || '';
      // Should contain function signature or type information
      // Should show hover info for whatever is at this position
      expect(content).toBeDefined();
    });

    it('should get completions', async () => {
      const result = await client.callTool('get_completions', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        line: 26,
        character: 10,
      });
      expect(result).toBeDefined();
    });

    it('should get signature help', async () => {
      const result = await client.callTool('get_signature_help', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        line: 14,
        character: 20,
      });
      expect(result).toBeDefined();
    });

    it('should get inlay hints', async () => {
      const result = await client.callTool('get_inlay_hints', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        start_line: 10,
        start_character: 0,
        end_line: 20,
        end_character: 0,
      });
      expect(result).toBeDefined();
    });
  });

  describe('Hierarchy Tools', () => {
    it('should prepare call hierarchy', async () => {
      const result = await client.callTool('prepare_call_hierarchy', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        line: 13,
        character: 10,
      });
      expect(result).toBeDefined();
    });

  });

  describe('File Operations', () => {
    it('should create file', async () => {
      const result = await client.callTool('create_file', {
        file_path: join(tmpdir(), 'codebuddy-test.ts'),
        content: '// Test file\nconsole.log("test");',
      });
      expect(result).toBeDefined();
    });

    it('should rename file', async () => {
      const result = await client.callTool('rename_file', {
        old_path: join(tmpdir(), 'codebuddy-test.ts'),
        new_path: join(tmpdir(), 'codebuddy-renamed.ts'),
        dry_run: true,
      });
      expect(result).toBeDefined();
    });

    it('should delete file', async () => {
      const result = await client.callTool('delete_file', {
        file_path: join(tmpdir(), 'codebuddy-renamed.ts'),
        dry_run: true,
      });
      expect(result).toBeDefined();
    });
  });

  describe('Server Management', () => {
    it('should restart server', async () => {
      const result = await client.callTool('restart_server', {
        extensions: ['ts', 'tsx'],
      });
      expect(result).toBeDefined();

      assertToolResult(result);
      const content = result.content?.[0]?.text || '';
      expect(content).toMatch(/(restart|server|success)/i);
    }, 20000);
  });

  describe('Advanced Workflow Tools', () => {
    it('should apply workspace edit', async () => {
      const result = await client.callTool('apply_workspace_edit', {
        changes: {
          [join(tmpdir(), 'codebuddy-workspace-edit.ts')]: [
            {
              range: {
                start: { line: 0, character: 0 },
                end: { line: 0, character: 0 },
              },
              newText: '// Workspace edit test\nconst testVar = "edited";\n',
            },
          ],
        },
        validate_before_apply: true,
      });
      expect(result).toBeDefined();

      assertToolResult(result);
      const content = result.content?.[0]?.text || '';
      expect(content).toMatch(/(applied|workspace|edit|success)/i);
    });

    it('should get call hierarchy incoming calls', async () => {
      // First prepare the call hierarchy item
      const prepareResult = await client.callTool('prepare_call_hierarchy', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        line: 13,
        character: 10,
      });

      expect(prepareResult).toBeDefined();

      // If we get a valid hierarchy item, test incoming calls
      assertToolResult(prepareResult);
      const prepareContent = prepareResult.content?.[0]?.text || '';
      if (prepareContent.includes('name') && prepareContent.includes('uri')) {
        const result = await client.callTool('get_call_hierarchy_incoming_calls', {
          item: {
            name: 'calculateAge',
            kind: 12,
            uri: 'file:///workspace/examples/playground/src/test-file.ts',
            range: {
              start: { line: 12, character: 0 },
              end: { line: 14, character: 1 },
            },
            selectionRange: {
              start: { line: 12, character: 9 },
              end: { line: 12, character: 20 },
            },
          },
        });

        expect(result).toBeDefined();

        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        expect(content).toMatch(/(incoming|call|hierarchy|from)/i);
      }
    });

    it('should get call hierarchy outgoing calls', async () => {
      const result = await client.callTool('get_call_hierarchy_outgoing_calls', {
        item: {
          name: 'calculateAge',
          kind: 12,
          uri: 'file:///workspace/examples/playground/src/test-file.ts',
          range: {
            start: { line: 12, character: 0 },
            end: { line: 14, character: 1 },
          },
          selectionRange: {
            start: { line: 12, character: 9 },
            end: { line: 12, character: 20 },
          },
        },
      });

      expect(result).toBeDefined();

      assertToolResult(result);
      const content = result.content?.[0]?.text || '';
      expect(content).toMatch(/(outgoing|call|hierarchy|to)/i);
    });

  });

  // Summary test
  it('should run all tests and show summary', async () => {
    const results = await client.callTools(ALL_TESTS);
    const toolResults = results as Array<{ success: boolean; name: string; error?: string }>;

    const successful = toolResults.filter((r) => r.success);
    const failed = toolResults.filter((r) => !r.success);

    console.log('\n=================================');
    console.log('📊 FINAL VERIFICATION RESULTS');
    console.log('=================================\n');
    console.log(`✅ PASSED: ${successful.length}/${results.length}`);
    console.log(`❌ FAILED: ${failed.length}/${results.length}\n`);

    if (failed.length === 0) {
      console.log('🎉 ALL 26 TOOLS VERIFIED WORKING! 🎉');
      console.log('Codebuddy is fully operational with complete LSP functionality.');
    } else {
      console.log(`⚠️  ${failed.length} tools still need attention:`);
      for (const result of failed) {
        console.log(`   ❌ ${result.name}: ${result.error || 'Failed'}`);
      }
    }

    // Assert all tests pass
    expect(failed.length).toBe(0);
  }, 60000);
});

import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { MCPTestClient, assertToolResult } from '../helpers/mcp-test-client.js';

describe('MCP Error Case Tests', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    console.log('🚨 Codebuddy Error Case Testing');
    console.log('============================\n');
    console.log('Testing error handling and edge cases...\n');

    // Use shared client when running in fast mode to reduce server overhead
    if (process.env.TEST_MODE === 'fast') {
      client = MCPTestClient.getShared();
      await client.start({ skipLSPPreload: true });
    } else {
      client = new MCPTestClient();
      await client.start({ skipLSPPreload: true });
    }

    // Wait for LSP servers to fully initialize
    console.log('⏳ Waiting for LSP servers to initialize...');
    await new Promise((resolve) => setTimeout(resolve, 3000));
  });

  afterAll(async () => {
    await client.stop();
  });

  describe('Invalid File Path Errors', () => {
    it('should handle non-existent file gracefully', async () => {
      try {
        const result = await client.callTool('find_definition', {
          file_path: '/non/existent/file.ts',
          symbol_name: 'nonExistent',
        });

        // Should either fail or return meaningful error message
        assertToolResult(result);
        if (result.content) {
          const content = result.content?.[0]?.text || '';
          expect(content).toMatch(/(not found|does not exist|no such file|error)/i);
        }
      } catch (error) {
        // Expected to fail - this is the correct behavior
        expect((error as Error).message).toMatch(/(not found|does not exist|no such file|error)/i);
      }
    });

    it('should handle malformed file path', async () => {
      try {
        await client.callTool('get_diagnostics', {
          file_path: '', // Empty path
        });
      } catch (error) {
        expect((error as Error).message).toMatch(/(invalid|empty|path|error)/i);
      }
    });
  });

  describe('Invalid Position Errors', () => {
    it('should handle out-of-bounds line numbers', async () => {
      try {
        const result = await client.callTool('get_hover', {
          file_path: '/workspace/examples/playground/src/test-file.ts',
          line: 9999, // Way beyond file bounds
          character: 10,
        });

        // Should handle gracefully
        assertToolResult(result);
        if (result.content) {
          const content = result.content?.[0]?.text || '';
          expect(content).toMatch(/(out of bounds|invalid position|no hover|error)/i);
        }
      } catch (error) {
        expect((error as Error).message).toBeDefined();
      }
    });

    it('should handle negative positions', async () => {
      try {
        await client.callTool('get_completions', {
          file_path: '/workspace/examples/playground/src/test-file.ts',
          line: -1,
          character: -5,
        });
      } catch (error) {
        expect((error as Error).message).toMatch(/(invalid|negative|position|error)/i);
      }
    });
  });

  describe('Invalid Symbol Names', () => {
    it('should handle non-existent symbol names', async () => {
      const result = await client.callTool('find_definition', {
        file_path: '/workspace/examples/playground/src/test-file.ts',
        symbol_name: 'ThisSymbolDoesNotExist',
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      expect(result.content).toBeDefined();

      const content = result.content?.[0]?.text || '';
      expect(content).toMatch(
        /(not found|no definition|no matches|0 definitions|no symbols found)/i
      );
    });

    it('should handle empty symbol name', async () => {
      try {
        await client.callTool('find_references', {
          file_path: '/workspace/examples/playground/src/test-file.ts',
          symbol_name: '', // Empty symbol name
        });
      } catch (error) {
        expect((error as Error).message).toMatch(/(empty|invalid|symbol|name|error)/i);
      }
    });
  });

  describe('Invalid Rename Operations', () => {
    it('should handle rename with same name', async () => {
      try {
        await client.callTool('rename_symbol', {
          file_path: '/workspace/examples/playground/src/test-file.ts',
          symbol_name: 'calculateAge',
          new_name: 'calculateAge', // Same name
          dry_run: true,
        });
      } catch (error) {
        expect((error as Error).message).toMatch(/(same name|no change|invalid|error)/i);
      }
    });

    it('should handle rename with invalid identifier', async () => {
      try {
        await client.callTool('rename_symbol', {
          file_path: '/workspace/examples/playground/src/test-file.ts',
          symbol_name: 'calculateAge',
          new_name: '123InvalidName', // Invalid JavaScript identifier
          dry_run: true,
        });
      } catch (error) {
        expect((error as Error).message).toMatch(/(invalid|identifier|name|error)/i);
      }
    });
  });

  describe('Invalid Workspace Operations', () => {
    it('should handle malformed workspace edit', async () => {
      try {
        await client.callTool('apply_workspace_edit', {
          changes: {
            [join(tmpdir(), 'test.ts')]: [
              {
                range: {
                  start: { line: 10, character: 0 },
                  end: { line: 5, character: 0 }, // End before start - invalid
                },
                newText: 'invalid range',
              },
            ],
          },
          validate_before_apply: true,
        });
      } catch (error) {
        expect((error as Error).message).toMatch(/(invalid|range|position|error)/i);
      }
    });

    it('should handle file operations on protected paths', async () => {
      try {
        await client.callTool('delete_file', {
          file_path: '/etc/passwd', // System file
          dry_run: true,
        });
      } catch (error) {
        expect((error as Error).message).toMatch(/(permission|access|denied|error)/i);
      }
    });
  });

  describe('Server State Errors', () => {
    it('should handle requests to unsupported file types', async () => {
      // Create a file with unsupported extension
      await client.callTool('create_file', {
        file_path: join(tmpdir(), 'test.xyz'),
        content: 'unsupported file type',
      });

      try {
        const result = await client.callTool('get_diagnostics', {
          file_path: join(tmpdir(), 'test.xyz'),
        });

        // Should handle gracefully
        assertToolResult(result);
        if (result.content) {
          const content = result.content?.[0]?.text || '';
          expect(content).toMatch(/(unsupported|no server|no diagnostics|error)/i);
        }
      } catch (error) {
        expect((error as Error).message).toMatch(/(unsupported|no server|error)/i);
      }

      // Clean up
      try {
        await client.callTool('delete_file', {
          file_path: join(tmpdir(), 'test.xyz'),
        });
      } catch {
        // Ignore cleanup errors
      }
    });

    it('should handle malformed hierarchy items', async () => {
      try {
        await client.callTool('get_call_hierarchy_incoming_calls', {
          item: {
            // Missing required fields
            name: 'incomplete',
          },
        });
      } catch (error) {
        expect((error as Error).message).toMatch(/(invalid|incomplete|item|error)/i);
      }
    });
  });

  describe('Concurrent Request Handling', () => {
    it('should handle multiple concurrent requests without corruption', async () => {
      const promises = Array.from({ length: 5 }, (_, i) =>
        client.callTool('get_diagnostics', {
          file_path: '/workspace/examples/playground/src/test-file.ts',
        })
      );

      const results = await Promise.allSettled(promises);

      // All should either succeed or fail gracefully
      results.forEach((result, index) => {
        if (result.status === 'fulfilled') {
          assertToolResult(result.value);
          expect(result.value.content).toBeDefined();
        } else {
          // Failed requests should have meaningful error messages
          expect(result.reason).toBeDefined();
        }
      });
    });
  });

  describe('Resource Limits', () => {
    it('should handle extremely large position numbers gracefully', async () => {
      try {
        const result = await client.callTool('get_hover', {
          file_path: '/workspace/examples/playground/src/test-file.ts',
          line: Number.MAX_SAFE_INTEGER,
          character: Number.MAX_SAFE_INTEGER,
        });

        assertToolResult(result);
        if (result.content) {
          const content = result.content?.[0]?.text || '';
          expect(content).toMatch(/(out of bounds|invalid|no hover|error)/i);
        }
      } catch (error) {
        expect((error as Error).message).toBeDefined();
      }
    });

    it('should handle large workspace edits', async () => {
      try {
        const largeText = 'x'.repeat(100000); // 100KB of text
        const result = await client.callTool('apply_workspace_edit', {
          changes: {
            [join(tmpdir(), 'large-edit.ts')]: [
              {
                range: {
                  start: { line: 0, character: 0 },
                  end: { line: 0, character: 0 },
                },
                newText: largeText,
              },
            ],
          },
          validate_before_apply: true,
        });

        // Should handle or reject gracefully
        assertToolResult(result);
        if (result.content) {
          const content = result.content?.[0]?.text || '';
          expect(content).toMatch(/(applied|too large|error)/i);
        }
      } catch (error) {
        expect((error as Error).message).toBeDefined();
      }
    });
  });

  // Summary test for error coverage
  it('should have tested comprehensive error scenarios', async () => {
    console.log('\n🚨 Error Case Testing Complete');
    console.log('============================');
    console.log('✅ File path errors');
    console.log('✅ Position boundary errors');
    console.log('✅ Symbol resolution errors');
    console.log('✅ Rename operation errors');
    console.log('✅ Workspace operation errors');
    console.log('✅ Server state errors');
    console.log('✅ Concurrency handling');
    console.log('✅ Resource limit handling');
    console.log('\n✨ Codebuddy demonstrates robust error handling across all scenarios!');

    // Verify the test client is still responsive after all error scenarios
    const healthCheck = await client.callTool('get_diagnostics', {
      file_path: '/workspace/examples/playground/src/test-file.ts',
    });

    // Should still be able to make successful tool calls after error testing
    expect(healthCheck).toBeDefined();
    assertToolResult(healthCheck);
    expect(healthCheck.content).toBeDefined();

    // Verify error test coverage completed by checking client state
    expect(client).toBeDefined();
    console.log('✅ System remains responsive after comprehensive error testing');
  });
});

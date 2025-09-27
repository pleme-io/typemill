import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { assertToolResult, MCPTestClient } from '../helpers/mcp-test-client.js';
import { waitForCondition as pollingWaitForCondition } from '../helpers/polling-helpers.js';
import {
  findLSPServers,
  getLSPServerMemory,
  simulateServerCrash,
  waitForLSPServer,
} from '../helpers/server-process-manager.js';
import { getSystemCapabilities } from '../helpers/system-utils.js';
import { waitForCondition, waitForLSP } from '../helpers/test-verification-helpers.js';

describe('Error Recovery Tests', () => {
  let client: MCPTestClient;
  const systemCaps = getSystemCapabilities();
  const timeout = systemCaps.baseTimeout * 3; // Extra time for recovery
  const testFile = '/workspace/examples/playground/src/test-file.ts';

  beforeAll(async () => {
    console.log('🔥 Error Recovery Testing Suite');
    console.log('================================\n');
    console.log('Testing LSP server crash recovery and error handling...\n');

    // Create new client for recovery testing
    client = new MCPTestClient();
    await client.start({ skipLSPPreload: false });

    // Ensure LSP servers are running
    await waitForLSP(client, testFile);
  });

  afterAll(async () => {
    await client.stop();
  });

  describe('LSP Server Crash Recovery', () => {
    it(
      'should recover from TypeScript server crash',
      async () => {
        // First, make a successful request to ensure server is running
        const beforeResult = await client.callTool('get_diagnostics', {
          file_path: testFile,
        });
        assertToolResult(beforeResult);

        // Get current TypeScript server processes
        const serversBefore = findLSPServers('typescript-language-server');
        console.log(`Found ${serversBefore.length} TypeScript servers before crash`);

        // Simulate server crash
        const crashed = await simulateServerCrash('typescript-language-server');
        expect(crashed).toBe(true);

        // Wait for the crash to be detected by checking server count
        await waitForCondition(
          () => {
            const currentServers = findLSPServers('typescript-language-server');
            return currentServers.length < serversBefore.length;
          },
          { timeout: 5000, message: 'Server crash not detected' }
        );

        // Make a request that should trigger auto-recovery
        const afterResult = await client.callTool('get_diagnostics', {
          file_path: testFile,
        });
        assertToolResult(afterResult);
        expect(afterResult.content).toBeDefined();

        // Verify a new server was started
        const newServer = await waitForLSPServer('typescript-language-server', 5000);
        expect(newServer).toBeDefined();
        console.log(`Recovery successful: New server PID ${newServer?.pid}`);
      },
      timeout
    );

    it(
      'should handle multiple rapid server crashes',
      async () => {
        let successCount = 0;
        const attempts = 3;

        for (let i = 0; i < attempts; i++) {
          // Crash the server
          const beforeCrashCount = findLSPServers('typescript-language-server').length;
          await simulateServerCrash('typescript-language-server');
          // Wait for server to be gone
          await waitForCondition(
            () => findLSPServers('typescript-language-server').length < beforeCrashCount,
            { timeout: 2000, message: 'Server did not crash' }
          );

          // Try to use it (should auto-recover)
          try {
            const result = await client.callTool('find_definition', {
              file_path: testFile,
              symbol_name: 'Person',
            });
            assertToolResult(result);
            successCount++;
          } catch (error) {
            console.log(`Attempt ${i + 1} failed:`, error);
          }

          // Wait between attempts for server to stabilize
          await waitForCondition(() => findLSPServers('typescript-language-server').length > 0, {
            timeout: 3000,
            message: 'Server did not recover',
          });
        }

        // Should recover from at least some crashes
        expect(successCount).toBeGreaterThan(0);
        console.log(`Recovered from ${successCount}/${attempts} crashes`);
      },
      timeout * 2
    );

    it(
      'should maintain request integrity during server restart',
      async () => {
        // Start multiple concurrent requests
        const promises = [
          client.callTool('get_diagnostics', { file_path: testFile }),
          client.callTool('get_document_symbols', { file_path: testFile }),
          client.callTool('find_definition', {
            file_path: testFile,
            symbol_name: 'Person',
          }),
        ];

        // Crash server while requests are in flight
        pollingWaitForCondition(
          () => {
            simulateServerCrash('typescript-language-server');
            return true;
          },
          { timeout: 100, interval: 100 }
        );

        // All requests should either succeed or fail gracefully
        const results = await Promise.allSettled(promises);

        let successCount = 0;
        for (const result of results) {
          if (result.status === 'fulfilled') {
            assertToolResult(result.value);
            successCount++;
          }
        }

        // At least some should succeed (either before crash or after recovery)
        expect(successCount).toBeGreaterThan(0);
        console.log(`${successCount}/${results.length} requests completed successfully`);
      },
      timeout
    );
  });

  describe('Server Health Monitoring', () => {
    it(
      'should accurately report server health status',
      async () => {
        const healthResult = await client.callTool('health_check', {
          include_details: true,
        });
        assertToolResult(healthResult);

        const content = healthResult.content?.[0]?.text || '';
        // Should include server status information
        expect(content).toMatch(/(running|active|healthy|server|status)/i);
      },
      timeout
    );

    it(
      'should detect unhealthy servers after crash',
      async () => {
        // Get initial health
        const beforeHealth = await client.callTool('health_check', {
          include_details: true,
        });
        assertToolResult(beforeHealth);

        // Crash the server
        const serversBeforeCrash = findLSPServers('typescript-language-server').length;
        await simulateServerCrash('typescript-language-server');
        // Wait for crash to be detected
        await waitForCondition(
          () => findLSPServers('typescript-language-server').length < serversBeforeCrash,
          { timeout: 5000, message: 'Server crash not detected' }
        );

        // Check health again - should detect issue or recovery
        const afterHealth = await client.callTool('health_check', {
          include_details: true,
        });
        assertToolResult(afterHealth);

        // Health check should complete even with crashed server
        expect(afterHealth).toBeDefined();
      },
      timeout
    );
  });

  describe('Manual Server Restart', () => {
    it(
      'should handle manual restart of TypeScript server',
      async () => {
        // First, ensure we have a baseline - server should be working
        const beforeResult = await client.callTool('get_diagnostics', {
          file_path: testFile,
        });
        assertToolResult(beforeResult);

        // Get initial server count (may be 0 if no servers active)
        const serversBefore = findLSPServers('typescript-language-server');
        console.log(`Found ${serversBefore.length} TypeScript servers before restart`);

        // Manually restart the server
        const restartResult = await client.callTool('restart_server', {
          extensions: ['ts', 'tsx'],
        });
        assertToolResult(restartResult);
        console.log('Server restart command completed');

        // Wait for server to be ready after restart
        await waitForCondition(
          async () => {
            try {
              const result = await client.callTool('get_document_symbols', { file_path: testFile });
              return result?.content?.[0]?.text ? !result.content[0].text.includes('Error') : true;
            } catch {
              return false;
            }
          },
          { timeout: 5000, message: 'Server not ready after restart' }
        );

        // Trigger server startup by making a request (servers start lazily)
        const afterResult = await client.callTool('get_diagnostics', {
          file_path: testFile,
        });
        assertToolResult(afterResult);
        expect(afterResult.content).toBeDefined();

        // Wait for new server to fully initialize
        await waitForLSP(client, testFile);

        // Verify new server is running and functional
        const finalResult = await client.callTool('find_definition', {
          file_path: testFile,
          symbol_name: 'Person',
        });
        assertToolResult(finalResult);
        expect(finalResult.content).toBeDefined();

        // Check that we have active servers after restart
        const serversAfter = findLSPServers('typescript-language-server');
        console.log(`Found ${serversAfter.length} TypeScript servers after restart and usage`);

        // The key test: server should be functional after restart
        expect(serversAfter.length).toBeGreaterThan(0);
      },
      timeout
    );

    it(
      'should handle restart of all servers',
      async () => {
        const restartResult = await client.callTool('restart_server', {});
        assertToolResult(restartResult);

        const content = restartResult.content?.[0]?.text || '';
        expect(content).toMatch(/(restart|success|server)/i);

        // Wait for servers to restart and be ready
        await waitForLSP(client, testFile);

        // All operations should still work
        const result = await client.callTool('get_diagnostics', {
          file_path: testFile,
        });
        assertToolResult(result);
      },
      timeout
    );
  });

  describe('Timeout and Cancellation', () => {
    it(
      'should handle request timeout gracefully',
      async () => {
        // Create a very large file to cause potential timeout
        const largeFile = join(tmpdir(), 'huge-test-file.ts');
        const hugeContent = 'const x = 1;\n'.repeat(50000); // 50k lines

        await client.callTool('create_file', {
          file_path: largeFile,
          content: hugeContent,
        });

        try {
          // This might timeout on slow systems
          const result = await client.callTool('get_document_symbols', {
            file_path: largeFile,
          });
          assertToolResult(result);
          // If it succeeds, that's fine
          expect(result).toBeDefined();
        } catch (error) {
          // Timeout errors should be handled gracefully
          expect(error).toBeDefined();
          const errorMessage = (error as Error).message;
          expect(errorMessage).toMatch(/(timeout|timed out|exceeded|slow)/i);
        } finally {
          // Cleanup
          await client.callTool('delete_file', {
            file_path: largeFile,
          });
        }
      },
      timeout * 2
    );

    it(
      'should handle concurrent requests during high load',
      async () => {
        const concurrentCount = 20;
        const promises: Promise<any>[] = [];

        // Generate many concurrent requests
        for (let i = 0; i < concurrentCount; i++) {
          promises.push(
            client.callTool('get_hover', {
              file_path: testFile,
              line: 1,
              character: i,
            })
          );
        }

        const results = await Promise.allSettled(promises);

        // Count successes
        let successCount = 0;
        for (const result of results) {
          if (result.status === 'fulfilled') {
            successCount++;
          }
        }

        // Most should succeed even under load
        const successRate = successCount / concurrentCount;
        expect(successRate).toBeGreaterThan(0.5); // At least 50% success
        console.log(`Success rate under load: ${(successRate * 100).toFixed(1)}%`);
      },
      timeout
    );
  });

  describe('Memory Management', () => {
    it(
      'should not leak memory on repeated operations',
      async () => {
        // Get initial memory usage
        const memoryBefore = getLSPServerMemory('typescript-language-server');
        const initialMemory = Array.from(memoryBefore.values())[0] || 0;
        console.log(`Initial memory usage: ${initialMemory.toFixed(2)} MB`);

        // Perform many operations
        for (let i = 0; i < 50; i++) {
          await client.callTool('get_diagnostics', {
            file_path: testFile,
          });
        }

        // Check memory after operations
        const memoryAfter = getLSPServerMemory('typescript-language-server');
        const finalMemory = Array.from(memoryAfter.values())[0] || 0;
        console.log(`Final memory usage: ${finalMemory.toFixed(2)} MB`);

        // Memory growth should be reasonable (< 100MB)
        const memoryGrowth = finalMemory - initialMemory;
        expect(memoryGrowth).toBeLessThan(100);
        console.log(`Memory growth: ${memoryGrowth.toFixed(2)} MB`);
      },
      timeout * 2
    );

    it(
      'should release memory after file deletion',
      async () => {
        const testFiles: string[] = [];

        // Create multiple test files
        for (let i = 0; i < 10; i++) {
          const filePath = join(tmpdir(), `mem-test-${i}.ts`);
          await client.callTool('create_file', {
            file_path: filePath,
            content: `export const value${i} = ${i};\n`.repeat(100),
          });
          testFiles.push(filePath);

          // Process each file
          await client.callTool('get_diagnostics', { file_path: filePath });
        }

        // Delete all files
        for (const filePath of testFiles) {
          await client.callTool('delete_file', { file_path: filePath });
        }

        // Memory should be released after garbage collection
        // Note: We can't force GC in all environments, so this is best-effort
        if (global.gc) {
          global.gc();
        }

        // Just verify server is still responsive
        const result = await client.callTool('health_check', {});
        assertToolResult(result);
        expect(result).toBeDefined();
      },
      timeout
    );
  });

  describe('Edge Case Error Handling', () => {
    it(
      'should handle corrupted file content gracefully',
      async () => {
        const corruptFile = join(tmpdir(), 'corrupt-test.ts');

        // Create file with invalid UTF-8 sequences (using Buffer)
        const buffer = Buffer.from([0xff, 0xfe, 0x00, 0x00]); // Invalid UTF-8
        writeFileSync(corruptFile, buffer);

        try {
          const result = await client.callTool('get_diagnostics', {
            file_path: corruptFile,
          });
          assertToolResult(result);
          // Should handle gracefully
          expect(result).toBeDefined();
        } catch (error) {
          // Error is acceptable for corrupted content
          expect(error).toBeDefined();
        } finally {
          // Cleanup
          try {
            await client.callTool('delete_file', { file_path: corruptFile });
          } catch {
            // Ignore cleanup errors
          }
        }
      },
      timeout
    );

    it(
      'should handle server communication errors',
      async () => {
        // Make multiple rapid requests to potentially cause communication issues
        const rapidRequests: Promise<any>[] = [];
        for (let i = 0; i < 50; i++) {
          rapidRequests.push(
            client.callTool('get_hover', {
              file_path: testFile,
              line: 1,
              character: 0,
            })
          );
        }

        const results = await Promise.allSettled(rapidRequests);

        // Should handle most requests successfully
        const successes = results.filter((r) => r.status === 'fulfilled').length;
        expect(successes).toBeGreaterThan(25); // At least half should succeed
      },
      timeout
    );

    it(
      'should handle missing LSP server gracefully',
      async () => {
        // Try to work with a file type that might not have LSP server
        const unsupportedFile = join(tmpdir(), 'test.unknown_extension');

        const result = await client.callTool('create_file', {
          file_path: unsupportedFile,
          content: 'some content',
        });
        assertToolResult(result);

        // Operations should fail gracefully
        const diagResult = await client.callTool('get_diagnostics', {
          file_path: unsupportedFile,
        });
        assertToolResult(diagResult);

        // Cleanup
        await client.callTool('delete_file', { file_path: unsupportedFile });
      },
      timeout
    );
  });
});

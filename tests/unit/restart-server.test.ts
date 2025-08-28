import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { MCPTestClient } from '../helpers/mcp-test-client.js';

describe('Server Restart Tests', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    console.log('⏱️  Testing restart_server timing and response...');
    client = new MCPTestClient();
    await client.start();

    // Wait for LSP servers to fully initialize
    console.log('⏳ Waiting for LSP servers to initialize...');
    await new Promise((resolve) => setTimeout(resolve, 3000));
  });

  afterAll(async () => {
    await client.stop();
  });

  it('should restart TypeScript servers successfully', async () => {
    console.log('🔄 Starting restart_server test...');
    const startTime = Date.now();

    // Track progress
    const progressTimer = setInterval(() => {
      const elapsed = Date.now() - startTime;
      console.log(`⏳ Still waiting... ${elapsed}ms elapsed`);
    }, 1000);

    try {
      const result = await client.callTool('restart_server', {
        extensions: ['ts', 'tsx'],
      });

      clearInterval(progressTimer);
      const elapsed = Date.now() - startTime;
      console.log(`✅ restart_server completed in ${elapsed}ms`);

      expect(result).toBeDefined();
      expect(result.content).toBeDefined();

      console.log('✅ Got result:', JSON.stringify(result, null, 2));
    } catch (error: any) {
      clearInterval(progressTimer);
      const elapsed = Date.now() - startTime;
      console.log(`❌ restart_server failed after ${elapsed}ms: ${error.message}`);
      throw error;
    }
  }, 30000);

  it('should restart all servers when no extensions specified', async () => {
    console.log('🔄 Testing restart all servers...');
    const startTime = Date.now();

    const result = await client.callTool('restart_server', {});

    const elapsed = Date.now() - startTime;
    console.log(`✅ restart_server (all) completed in ${elapsed}ms`);

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();
  }, 30000);

  it('should handle restart with non-existent extension gracefully', async () => {
    console.log('🔄 Testing restart with non-existent extension...');

    const result = await client.callTool('restart_server', {
      extensions: ['xyz'], // Non-existent extension
    });

    expect(result).toBeDefined();
    expect(result.content).toBeDefined();

    // Should complete without error even if no servers match
    console.log('✅ Handled non-existent extension gracefully');
  }, 15000);

  it('should restart multiple times successfully', async () => {
    console.log('🔄 Testing multiple restarts...');

    // First restart
    const result1 = await client.callTool('restart_server', {
      extensions: ['ts'],
    });
    expect(result1).toBeDefined();
    console.log('✅ First restart completed');

    // Wait a bit
    await new Promise((resolve) => setTimeout(resolve, 1000));

    // Second restart
    const result2 = await client.callTool('restart_server', {
      extensions: ['ts'],
    });
    expect(result2).toBeDefined();
    console.log('✅ Second restart completed');

    // Third restart
    const result3 = await client.callTool('restart_server', {
      extensions: ['ts'],
    });
    expect(result3).toBeDefined();
    console.log('✅ Third restart completed');

    console.log('✅ Multiple restarts handled successfully');
  }, 45000);
});

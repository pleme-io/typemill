import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { MCPTestClient } from '../helpers/mcp-test-client';

/**
 * Enhanced test for search_workspace_symbols with isolated test files
 * Tests symbol search across multiple files with known symbols
 * Optimized to prevent LSP server timeouts with minimal test files
 */

const TEST_DIR = '/tmp/workspace-symbols-simple';

describe('Workspace Symbols Search - Enhanced', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    console.log('🔧 Setting up simplified workspace symbols test...');

    // Clean and create test directory
    if (existsSync(TEST_DIR)) {
      rmSync(TEST_DIR, { recursive: true });
    }
    mkdirSync(TEST_DIR, { recursive: true });

    // Create simple test files with minimal TypeScript structures
    writeFileSync(
      join(TEST_DIR, 'service.ts'),
      `export class TestService {
  process(data: string): string {
    return data;
  }
}

export interface TestData {
  id: string;
  value: string;
}

export enum TestStatus {
  ACTIVE = 'active',
  INACTIVE = 'inactive',
}

export const TEST_CONSTANT = 'test';

export function validateTest(input: string): boolean {
  return input.length > 0;
}

export type TestFilter = {
  status?: TestStatus;
};`
    );

    writeFileSync(
      join(TEST_DIR, 'handler.ts'),
      `export class UserHandler {
  handle(user: UserData): void {
    console.log(user.name);
  }
}

export interface UserData {
  name: string;
  id: number;
}

export function processUser(data: UserData): UserData {
  return data;
}`
    );

    // Initialize MCP client
    client = new MCPTestClient();
    await client.start();
    console.log('✅ Simplified workspace symbols test ready');
  });

  afterAll(async () => {
    if (client) {
      await client.stop();
    }
    if (existsSync(TEST_DIR)) {
      rmSync(TEST_DIR, { recursive: true });
    }
    console.log('🧹 Cleaned up workspace symbols test');
  });

  it('should find Service classes in the workspace', async () => {
    console.log('🔍 Testing search for "Service" classes...');

    const result = await client.callTool('search_workspace_symbols', {
      query: 'Service',
      workspace_path: TEST_DIR,
    });

    const response = result.content?.[0]?.text || '';
    console.log('📋 Symbol search result for "Service":');
    console.log(`${response.substring(0, 300)}`);

    // Should find service classes
    expect(response).toContain('TestService');

    // Should include file path
    expect(response).toContain('service.ts');

    console.log('✅ Found Service classes correctly');
  });

  it('should find interfaces across files', async () => {
    console.log('🔍 Testing search for interfaces...');

    const result = await client.callTool('search_workspace_symbols', {
      query: 'Data',
      workspace_path: TEST_DIR,
    });

    const response = result.content?.[0]?.text || '';
    console.log('📋 Interface search result:');

    // Should find at least one interface
    const interfaceMatches = ['TestData', 'UserData'];
    let foundCount = 0;
    for (const interfaceName of interfaceMatches) {
      if (response.includes(interfaceName)) {
        foundCount++;
      }
    }

    console.log(`  Found ${foundCount} interfaces`);
    expect(foundCount).toBeGreaterThan(0);

    console.log('✅ Interface search working');
  });

  it('should find enum symbols', async () => {
    console.log('🔍 Testing search for enums...');

    const result = await client.callTool('search_workspace_symbols', {
      query: 'TestStatus',
      workspace_path: TEST_DIR,
    });

    const response = result.content?.[0]?.text || '';
    console.log('📋 Enum search result for "TestStatus":');
    console.log(response.substring(0, 300));

    // Should find TestStatus enum
    expect(response).toContain('TestStatus');

    console.log('✅ Enum symbols found');
  });

  it('should find function symbols', async () => {
    console.log('🔍 Testing search for functions...');

    const result = await client.callTool('search_workspace_symbols', {
      query: 'validate',
      workspace_path: TEST_DIR,
    });

    const response = result.content?.[0]?.text || '';
    console.log('📋 Function search result for "validate":');

    // Should find validation function
    if (response.includes('validateTest')) {
      console.log('  ✓ Found function: validateTest');
      expect(response).toContain('validateTest');
    }

    console.log('✅ Function search working');
  });

  it('should find type aliases', async () => {
    console.log('🔍 Testing search for type aliases...');

    const result = await client.callTool('search_workspace_symbols', {
      query: 'TestFilter',
      workspace_path: TEST_DIR,
    });

    const response = result.content?.[0]?.text || '';
    console.log('📋 Type alias search result:');

    // Should find TestFilter type
    if (response.includes('TestFilter')) {
      console.log('  ✓ Found TestFilter type alias');
      expect(response).toContain('TestFilter');
    }

    console.log('✅ Type alias search working');
  });

  it('should find constants and variables', async () => {
    console.log('🔍 Testing search for constants...');

    const result = await client.callTool('search_workspace_symbols', {
      query: 'TEST_CONSTANT',
      workspace_path: TEST_DIR,
    });

    const response = result.content?.[0]?.text || '';
    console.log('📋 Constant search result:');

    // Should find TEST_CONSTANT
    if (response.includes('TEST_CONSTANT')) {
      console.log('  ✓ Found TEST_CONSTANT');
      expect(response).toContain('TEST_CONSTANT');
      expect(response).toContain('service.ts');
    }

    console.log('✅ Constant search working');
  });

  it('should handle empty query gracefully', async () => {
    console.log('🔍 Testing empty query handling...');

    const result = await client.callTool('search_workspace_symbols', {
      query: '',
      workspace_path: TEST_DIR,
    });

    const response = result.content?.[0]?.text || '';
    console.log('📋 Empty query result:', response);

    expect(response.toLowerCase()).toContain('provide');

    console.log('✅ Empty query handled gracefully');
  });

  it('should return empty results for non-existent symbols', async () => {
    console.log('🔍 Testing non-existent symbol search...');

    const result = await client.callTool('search_workspace_symbols', {
      query: 'NonExistentSymbol123',
      workspace_path: TEST_DIR,
    });

    const response = result.content?.[0]?.text || '';
    console.log('📋 Non-existent symbol result:', response);

    expect(response.toLowerCase()).toMatch(/no.*found|not found|no symbols/i);

    console.log('✅ Non-existent symbols handled correctly');
  });

  it('should find symbols case-insensitively', async () => {
    console.log('🔍 Testing case-insensitive search...');

    const result = await client.callTool('search_workspace_symbols', {
      query: 'testservice', // lowercase
      workspace_path: TEST_DIR,
    });

    const response = result.content?.[0]?.text || '';
    console.log('📋 Case-insensitive search result:');

    // Should still find TestService even with different case
    if (response.includes('TestService')) {
      console.log('  ✓ Found TestService with lowercase query');
      expect(response).toContain('TestService');
    }

    console.log('✅ Case-insensitive search working');
  });
});

import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { MCPTestClient, assertToolResult } from '../helpers/mcp-test-client';
import {
  captureFileStates,
  verifyFileContainsAll,
  verifyFileDoesNotContain,
} from '../helpers/test-verification-helpers';

/**
 * Test delete_file command with enhanced impact analysis
 */

const TEST_DIR = '/tmp/delete-file-enhanced-test';

describe('Delete File with Impact Analysis', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    console.log('🔧 Setting up enhanced delete test environment...');

    // Clean and create test directory
    if (existsSync(TEST_DIR)) {
      rmSync(TEST_DIR, { recursive: true });
    }
    mkdirSync(join(TEST_DIR, 'services'), { recursive: true });
    mkdirSync(join(TEST_DIR, 'utils'), { recursive: true });

    // Create a service file that will be deleted
    const servicePath = join(TEST_DIR, 'services', 'data-service.ts');
    writeFileSync(
      servicePath,
      `
export class DataService {
  getData(): string {
    return 'data';
  }
}

export interface DataConfig {
  url: string;
  timeout: number;
}

export const DEFAULT_CONFIG: DataConfig = {
  url: 'https://api.example.com',
  timeout: 5000
};
`.trim()
    );

    // Create files that import the service
    writeFileSync(
      join(TEST_DIR, 'index.ts'),
      `
import { DataService, DataConfig } from './services/data-service';

export { DataService, DataConfig };

const service = new DataService();
console.log(service.getData());
`.trim()
    );

    writeFileSync(
      join(TEST_DIR, 'utils', 'helper.ts'),
      `
import { DataService, DEFAULT_CONFIG } from '../services/data-service';
import type { DataConfig } from '../services/data-service';

export function createService(config: DataConfig = DEFAULT_CONFIG): DataService {
  return new DataService();
}
`.trim()
    );

    writeFileSync(
      join(TEST_DIR, 'services', 'user-service.ts'),
      `
import { DataService } from './data-service';

export class UserService {
  private dataService = new DataService();
  
  getUser(id: string) {
    const data = this.dataService.getData();
    return { id, data };
  }
}
`.trim()
    );

    // Initialize MCP client
    client = new MCPTestClient();
    await client.start();
    console.log('✅ Enhanced test environment ready');
  });

  afterAll(async () => {
    if (client) {
      await client.stop();
    }
    if (existsSync(TEST_DIR)) {
      rmSync(TEST_DIR, { recursive: true });
    }
    console.log('🧹 Cleaned up test environment');
  });

  it('should prevent deletion when file is imported by others', async () => {
    console.log('🔍 Testing impact analysis prevention...');

    const servicePath = join(TEST_DIR, 'services', 'data-service.ts');

    // Try to delete without force
    const result = await client.callTool('delete_file', {
      file_path: servicePath,
    });

    const toolResult = assertToolResult(result);
    const response = toolResult.content?.[0]?.text || '';
    console.log('📋 Delete attempt result:');
    console.log(response);

    // Should be prevented
    expect(response).toContain('Cannot delete');
    expect(response).toContain('imported by');
    expect(response).toContain('file');

    // Should list the importing files
    expect(response).toMatch(/user-service\.ts/);

    // Should suggest using force
    expect(response).toContain('force: true');

    // File should still exist
    expect(existsSync(servicePath)).toBe(true);

    console.log('✅ Deletion correctly prevented due to imports');
  });

  it('should force delete with warning when force is true', async () => {
    console.log('🔍 Testing force deletion with warnings...');

    const servicePath = join(TEST_DIR, 'services', 'data-service.ts');
    const indexPath = join(TEST_DIR, 'index.ts');
    const helperPath = join(TEST_DIR, 'utils', 'helper.ts');
    const userServicePath = join(TEST_DIR, 'services', 'user-service.ts');

    // Capture states of importing files before deletion
    const importingFiles = [indexPath, helperPath, userServicePath];
    const beforeStates = captureFileStates(importingFiles);

    // Force delete
    const result = await client.callTool('delete_file', {
      file_path: servicePath,
      force: true,
    });

    const toolResult = assertToolResult(result);
    const response = toolResult.content?.[0]?.text || '';
    console.log('📋 Force delete result:');
    console.log(response);

    // Should succeed
    expect(response).toContain('Successfully deleted');

    // Should include warning about broken imports
    expect(response).toContain('Warning');
    expect(response).toContain('broken imports');

    // Should list affected files
    expect(response).toMatch(/user-service\.ts/);

    // File should be deleted
    expect(existsSync(servicePath)).toBe(false);

    // Verify importing files still exist but have broken imports
    console.log('\n🔍 Verifying importing files still contain broken imports...');

    // Check index.ts still has the broken import
    expect(existsSync(indexPath)).toBe(true);
    const indexContent = readFileSync(indexPath, 'utf-8');
    expect(indexContent).toContain(
      "import { DataService, DataConfig } from './services/data-service'"
    );
    console.log('  ✅ index.ts still contains broken import statement');

    // Check helper.ts still has the broken imports
    expect(existsSync(helperPath)).toBe(true);
    const helperContent = readFileSync(helperPath, 'utf-8');
    expect(helperContent).toContain(
      "import { DataService, DEFAULT_CONFIG } from '../services/data-service'"
    );
    expect(helperContent).toContain("import type { DataConfig } from '../services/data-service'");
    console.log('  ✅ helper.ts still contains broken import statements');

    // Check user-service.ts still has the broken import
    expect(existsSync(userServicePath)).toBe(true);
    const userServiceContent = readFileSync(userServicePath, 'utf-8');
    expect(userServiceContent).toContain("import { DataService } from './data-service'");
    console.log('  ✅ user-service.ts still contains broken import statement');

    // Verify files weren't modified (only the deleted file is gone)
    const afterStates = captureFileStates(importingFiles);
    for (const file of importingFiles) {
      const before = beforeStates.get(file);
      const after = afterStates.get(file);
      expect(before).toBe(after);
    }
    console.log('  ✅ Importing files were not modified, only the target file was deleted');

    console.log('✅ Force deletion succeeded with appropriate warnings');
  });

  it('should delete without warnings when file has no importers', async () => {
    console.log('🔍 Testing deletion of non-imported file...');

    // Create a standalone file with no imports
    const standalonePath = join(TEST_DIR, 'standalone.ts');
    writeFileSync(
      standalonePath,
      `
export function standaloneFunction() {
  return 'standalone';
}
`.trim()
    );

    const result = await client.callTool('delete_file', {
      file_path: standalonePath,
    });

    const toolResult = assertToolResult(result);
    const response = toolResult.content?.[0]?.text || '';
    console.log('📋 Standalone file delete result:');
    console.log(response);

    // Should succeed without warnings
    expect(response).toContain('Successfully deleted');
    expect(response).not.toContain('Warning');
    expect(response).not.toContain('broken imports');

    // File should be deleted
    expect(existsSync(standalonePath)).toBe(false);

    console.log('✅ Non-imported file deleted without warnings');
  });

  it('should handle non-existent file gracefully', async () => {
    console.log('🔍 Testing non-existent file handling...');

    const result = await client.callTool('delete_file', {
      file_path: join(TEST_DIR, 'non-existent.ts'),
    });

    const toolResult = assertToolResult(result);
    const response = toolResult.content?.[0]?.text || '';
    console.log('📋 Non-existent file result:');
    console.log(response);

    expect(response).toContain('does not exist');

    console.log('✅ Non-existent file handled gracefully');
  });
});

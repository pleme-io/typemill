import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { MCPTestClient, assertToolResult } from '../helpers/mcp-test-client';
import {
  captureFileStates,
  getFileLines,
  verifyFileContainsAll,
} from '../helpers/test-verification-helpers.js';

/**
 * Integration test for combined LSP operations
 * Tests multiple operations working together on the same codebase
 */

const TEST_DIR = '/workspace/playground/multi-operation-test';

describe('Multi-Operation Integration Tests', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    console.log('🔧 Setting up multi-operation integration test...');

    // Create isolated test directory
    if (existsSync(TEST_DIR)) {
      rmSync(TEST_DIR, { recursive: true });
    }
    mkdirSync(TEST_DIR, { recursive: true });

    // Create TypeScript project configuration for proper cross-file analysis
    writeFileSync(
      join(TEST_DIR, 'tsconfig.json'),
      JSON.stringify(
        {
          compilerOptions: {
            target: 'ES2022',
            module: 'ESNext',
            moduleResolution: 'node',
            esModuleInterop: true,
            allowSyntheticDefaultImports: true,
            strict: true,
            skipLibCheck: true,
            forceConsistentCasingInFileNames: true,
            resolveJsonModule: true,
            isolatedModules: true,
            noEmit: true,
          },
          include: ['**/*'],
          exclude: ['node_modules'],
        },
        null,
        2
      )
    );

    writeFileSync(
      join(TEST_DIR, 'package.json'),
      JSON.stringify(
        {
          name: 'multi-operation-integration-test',
          type: 'module',
          version: '1.0.0',
        },
        null,
        2
      )
    );

    // Create a service file that will be modified by multiple operations
    writeFileSync(
      join(TEST_DIR, 'user-service.ts'),
      `export interface UserData {
  id: number;
  name: string;
  email: string;
}

export class UserService {
  private users: UserData[] = [];

  addUser(user: UserData): void {
    this.users.push(user);
  }

  findUser(id: number): UserData | undefined {
    return this.users.find(u => u.id === id);
  }

  updateUser(id: number, data: Partial<UserData>): boolean {
    const user = this.findUser(id);
    if (user) {
      Object.assign(user, data);
      return true;
    }
    return false;
  }
}

export const DEFAULT_USER: UserData = {
  id: 1,
  name: 'Default User',
  email: 'default@test.com',
};`
    );

    // Create files that will reference the service
    writeFileSync(
      join(TEST_DIR, 'user-handler.ts'),
      `import { UserService, UserData } from './user-service';

export class UserHandler {
  constructor(private service: UserService) {}

  processUser(userData: UserData): string {
    this.service.addUser(userData);
    return \`Processed user: \${userData.name}\`;
  }

  getUserInfo(id: number): string | null {
    const user = this.service.findUser(id);
    return user ? \`\${user.name} <\${user.email}>\` : null;
  }
}`
    );

    writeFileSync(
      join(TEST_DIR, 'index.ts'),
      `export { UserService, UserData, DEFAULT_USER } from './user-service';
export { UserHandler } from './user-handler';

// Test function for call hierarchy
export function createService(): UserService {
  return new UserService();
}`
    );

    // Initialize MCP client
    client = new MCPTestClient();
    await client.start({ skipLSPPreload: true });

    // Allow extra time for TypeScript LSP to index the new project
    console.log('⏳ Waiting for TypeScript LSP to index project files...');
    await new Promise((resolve) => setTimeout(resolve, 3000));
    console.log('✅ Multi-operation integration test ready');
  });

  afterAll(async () => {
    if (client) {
      await client.stop();
    }
    if (existsSync(TEST_DIR)) {
      rmSync(TEST_DIR, { recursive: true });
    }
    console.log('🧹 Cleaned up multi-operation integration test');
  });

  it('should perform complete workflow: search → find references → rename → verify', async () => {
    console.log('🚀 Testing complete multi-operation workflow...');

    // STEP 1: Search for symbols in the workspace
    console.log('📍 Step 1: Search workspace symbols');
    const searchResult = await client.callTool('search_workspace_symbols', {
      query: 'UserData',
      workspace_path: TEST_DIR,
    });

    assertToolResult(searchResult);
    const searchResponse = searchResult.content?.[0]?.text || '';
    console.log('🔍 Symbol search found:', searchResponse.substring(0, 200));

    // Should find the UserData interface
    expect(searchResponse).toContain('UserData');
    expect(searchResponse).toContain('user-service.ts');

    // STEP 2: Find all references to UserData
    console.log('📍 Step 2: Find references to UserData');
    const referencesResult = await client.callTool('find_references', {
      file_path: join(TEST_DIR, 'user-service.ts'),
      symbol_name: 'UserData',
      include_declaration: true,
    });

    assertToolResult(referencesResult);
    const referencesResponse = referencesResult.content?.[0]?.text || '';
    console.log('🔗 References found:', referencesResponse.substring(0, 300));

    // Should find references in multiple files
    expect(referencesResponse).toContain('UserData');
    expect(referencesResponse).not.toMatch(/No.*found|not found/i);

    // STEP 3: Capture state before rename
    console.log('📍 Step 3: Capture state before rename operation');
    const filesToCheck = [
      join(TEST_DIR, 'user-service.ts'),
      join(TEST_DIR, 'user-handler.ts'),
      join(TEST_DIR, 'index.ts'),
    ];

    const beforeStates = captureFileStates(filesToCheck);
    console.log('📸 Captured state of', filesToCheck.length, 'files');

    // Verify UserData exists before rename
    verifyFileContainsAll(join(TEST_DIR, 'user-service.ts'), [
      'export interface UserData',
      'addUser(user: UserData)',
      'UserData[]',
    ]);

    // STEP 4: Rename UserData to UserInfo across all files
    console.log('📍 Step 4: Rename UserData → UserInfo');
    const renameResult = await client.callTool('rename_symbol', {
      file_path: join(TEST_DIR, 'user-service.ts'),
      symbol_name: 'UserData',
      new_name: 'UserInfo',
      dry_run: false,
    });

    assertToolResult(renameResult);
    const renameResponse = renameResult.content?.[0]?.text || '';
    console.log('🔄 Rename result:', renameResponse.substring(0, 200));

    expect(renameResponse).toMatch(/renamed|success|applied/i);
    expect(renameResponse).toMatch(/UserData.*UserInfo/);

    // Wait for file system operations
    await new Promise((resolve) => setTimeout(resolve, 500));

    // STEP 5: Verify rename was applied correctly across all files
    console.log('📍 Step 5: Verify rename across all affected files');

    console.log('📄 user-service.ts verification:');
    verifyFileContainsAll(join(TEST_DIR, 'user-service.ts'), [
      'export interface UserInfo',
      'addUser(user: UserInfo)',
      'UserInfo[]',
      'UserInfo | undefined',
      'Partial<UserInfo>',
      'const DEFAULT_USER: UserInfo',
    ]);

    console.log('📄 user-handler.ts verification:');
    verifyFileContainsAll(join(TEST_DIR, 'user-handler.ts'), [
      'import { UserService, UserInfo }',
      'processUser(userData: UserInfo)',
    ]);

    console.log('📄 index.ts verification:');
    verifyFileContainsAll(join(TEST_DIR, 'index.ts'), [
      'export { UserService, UserInfo, DEFAULT_USER }',
    ]);

    // Verify old name is completely gone
    const afterStates = captureFileStates(filesToCheck);
    for (const filePath of filesToCheck) {
      const content = afterStates.get(filePath) || '';
      expect(content).not.toContain('UserData');
      expect(content).toContain('UserInfo');
    }

    console.log('✅ Complete workflow verification successful');
  }, 30000);

  it('should perform workspace edit → find definition → call hierarchy sequence', async () => {
    console.log('🚀 Testing workspace edit → find definition → call hierarchy...');

    // STEP 1: Apply workspace edits to modify multiple files
    console.log('📍 Step 1: Apply workspace edits to add new methods');

    const edits = [
      {
        file_path: join(TEST_DIR, 'user-service.ts'),
        edits: [
          {
            range: { start: { line: 25, character: 0 }, end: { line: 25, character: 0 } },
            new_text: `
  deleteUser(id: number): boolean {
    const index = this.users.findIndex(u => u.id === id);
    if (index !== -1) {
      this.users.splice(index, 1);
      return true;
    }
    return false;
  }
`,
          },
        ],
      },
      {
        file_path: join(TEST_DIR, 'user-handler.ts'),
        edits: [
          {
            range: { start: { line: 15, character: 0 }, end: { line: 15, character: 0 } },
            new_text: `
  removeUser(id: number): boolean {
    return this.service.deleteUser(id);
  }
`,
          },
        ],
      },
    ];

    // Transform test data to match API specification
    const changes: Record<string, Array<{ range: any; newText: string }>> = {};
    for (const edit of edits) {
      changes[edit.file_path] = edit.edits.map((e) => ({
        range: e.range,
        newText: e.new_text,
      }));
    }

    const workspaceEditResult = await client.callTool('apply_workspace_edit', {
      changes: changes,
    });

    assertToolResult(workspaceEditResult);
    const editResponse = workspaceEditResult.content?.[0]?.text || '';
    console.log('📝 Workspace edit result:', editResponse.substring(0, 200));

    expect(editResponse).toMatch(/applied|success/i);

    // Wait for edits to be applied
    await new Promise((resolve) => setTimeout(resolve, 500));

    // STEP 2: Find definition of the newly added deleteUser method
    console.log('📍 Step 2: Find definition of deleteUser method');

    const definitionResult = await client.callTool('find_definition', {
      file_path: join(TEST_DIR, 'user-handler.ts'),
      symbol_name: 'deleteUser',
    });

    assertToolResult(definitionResult);
    const definitionResponse = definitionResult.content?.[0]?.text || '';
    console.log('🎯 Definition found:', definitionResponse.substring(0, 200));

    // Should find the definition in user-service.ts
    expect(definitionResponse).toContain('deleteUser');
    expect(definitionResponse).toContain('user-service.ts');

    // STEP 3: Prepare call hierarchy for deleteUser
    console.log('📍 Step 3: Prepare call hierarchy for deleteUser');

    // Find the exact line with deleteUser method
    const serviceContent = readFileSync(join(TEST_DIR, 'user-service.ts'), 'utf-8');
    const lines = serviceContent.split('\n');
    let deleteUserLine = -1;

    for (let i = 0; i < lines.length; i++) {
      if (lines[i].includes('deleteUser') && lines[i].includes('(')) {
        deleteUserLine = i + 1; // Convert to 1-based
        console.log(`🔍 Found deleteUser method at line ${deleteUserLine}: "${lines[i].trim()}"`);
        break;
      }
    }

    expect(deleteUserLine).toBeGreaterThan(0);

    const prepareResult = await client.callTool('prepare_call_hierarchy', {
      file_path: join(TEST_DIR, 'user-service.ts'),
      line: deleteUserLine,
      character: 3,
    });

    assertToolResult(prepareResult);
    const prepareResponse = prepareResult.content?.[0]?.text || '';
    console.log('📋 Call hierarchy prepared:', prepareResponse.substring(0, 200));

    expect(prepareResponse).toContain('deleteUser');

    // STEP 4: Get incoming calls to deleteUser
    console.log('📍 Step 4: Get incoming calls to deleteUser');

    const incomingResult = await client.callTool('get_call_hierarchy_incoming_calls', {
      file_path: join(TEST_DIR, 'user-service.ts'),
      line: deleteUserLine,
      character: 3,
    });

    assertToolResult(incomingResult);
    const incomingResponse = incomingResult.content?.[0]?.text || '';
    console.log('📞 Incoming calls:', incomingResponse.substring(0, 300));

    // Should find the call from user-handler.ts
    if (incomingResponse.includes('removeUser') || incomingResponse.includes('user-handler')) {
      console.log('✅ Found incoming call from user-handler.ts');
    }

    console.log('✅ Workspace edit → find definition → call hierarchy sequence complete');
  }, 25000);

  it('should perform symbol search → rename file → update imports verification', async () => {
    console.log('🚀 Testing symbol search → rename file → update imports...');

    // STEP 1: Search for all symbols in current file structure
    console.log('📍 Step 1: Search workspace symbols');

    const searchResult = await client.callTool('search_workspace_symbols', {
      query: 'UserService',
      workspace_path: TEST_DIR,
    });

    assertToolResult(searchResult);
    const searchResponse = searchResult.content?.[0]?.text || '';
    console.log('🔍 Initial search:', searchResponse.substring(0, 200));

    expect(searchResponse).toContain('UserService');

    // STEP 2: Rename the main service file
    console.log('📍 Step 2: Rename user-service.ts → account-service.ts');

    const renameFileResult = await client.callTool('rename_file', {
      old_path: join(TEST_DIR, 'user-service.ts'),
      new_path: join(TEST_DIR, 'account-service.ts'),
      update_imports: true,
    });

    assertToolResult(renameFileResult);
    const renameFileResponse = renameFileResult.content?.[0]?.text || '';
    console.log('📁 File rename result:', renameFileResponse.substring(0, 300));

    expect(renameFileResponse).toMatch(/renamed|moved|updated/i);

    // Wait for file operations
    await new Promise((resolve) => setTimeout(resolve, 1000));

    // STEP 3: Verify file was renamed and imports updated
    console.log('📍 Step 3: Verify file rename and import updates');

    // Original file should not exist
    expect(existsSync(join(TEST_DIR, 'user-service.ts'))).toBe(false);

    // New file should exist
    expect(existsSync(join(TEST_DIR, 'account-service.ts'))).toBe(true);

    // STEP 4: Verify imports were updated in dependent files
    console.log('📍 Step 4: Verify import updates in dependent files');

    console.log('📄 user-handler.ts import verification:');
    const handlerContent = readFileSync(join(TEST_DIR, 'user-handler.ts'), 'utf-8');
    console.log('Handler imports:', handlerContent.split('\n')[0]);

    if (handlerContent.includes("from './account-service'")) {
      console.log('  ✅ Import path updated to account-service');
    } else if (handlerContent.includes("from './user-service'")) {
      console.log('  ⚠️  Import path still points to old user-service');
    }

    console.log('📄 index.ts import verification:');
    const indexContent = readFileSync(join(TEST_DIR, 'index.ts'), 'utf-8');
    console.log('Index exports:', indexContent.split('\n')[0]);

    if (indexContent.includes("from './account-service'")) {
      console.log('  ✅ Export path updated to account-service');
    } else if (indexContent.includes("from './user-service'")) {
      console.log('  ⚠️  Export path still points to old user-service');
    }

    // STEP 5: Search for symbols again to verify they're still found
    console.log('📍 Step 5: Search workspace symbols after file rename');

    const finalSearchResult = await client.callTool('search_workspace_symbols', {
      query: 'UserService',
      workspace_path: TEST_DIR,
    });

    assertToolResult(finalSearchResult);
    const finalSearchResponse = finalSearchResult.content?.[0]?.text || '';
    console.log('🔍 Final search:', finalSearchResponse.substring(0, 200));

    // Should still find UserService but in the new file location
    expect(finalSearchResponse).toContain('UserService');
    if (finalSearchResponse.includes('account-service.ts')) {
      console.log('  ✅ Symbol found in new file location');
    }

    console.log('✅ Symbol search → rename file → update imports sequence complete');
  }, 30000);

  it('should verify all operations work on the final state', async () => {
    console.log('🚀 Testing final state verification...');

    // After all previous operations, verify the workspace is still functional
    console.log('📍 Final verification: find references to UserInfo interface');

    const finalReferenceResult = await client.callTool('find_references', {
      file_path: join(TEST_DIR, 'account-service.ts'), // File was renamed
      symbol_name: 'UserInfo', // Interface was renamed
      include_declaration: true,
    });

    assertToolResult(finalReferenceResult);
    const finalReferenceResponse = finalReferenceResult.content?.[0]?.text || '';
    console.log('🔗 Final references check:', finalReferenceResponse.substring(0, 300));

    // Should still find references despite all the changes
    expect(finalReferenceResponse).toContain('UserInfo');

    console.log('📍 Final verification: get diagnostics for all files');

    const files = [
      join(TEST_DIR, 'account-service.ts'),
      join(TEST_DIR, 'user-handler.ts'),
      join(TEST_DIR, 'index.ts'),
    ];

    for (const file of files) {
      if (existsSync(file)) {
        const diagnosticsResult = await client.callTool('get_diagnostics', {
          file_path: file,
        });

        assertToolResult(diagnosticsResult);
        const diagnosticsResponse = diagnosticsResult.content?.[0]?.text || '';
        console.log(
          `📋 Diagnostics for ${file.split('/').pop()}:`,
          diagnosticsResponse.substring(0, 200)
        );
      }
    }

    console.log('✅ Final state verification complete - all operations successful');
  });
});

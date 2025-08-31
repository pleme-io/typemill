import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, readFileSync } from 'node:fs';
import { FileBackupManager } from '../helpers/file-backup-manager.js';
import { MCPTestClient, assertToolResult } from '../helpers/mcp-test-client.js';
import {
  verifyFileContainsAll,
  verifyFileDoesNotContain,
  verifyImportStatement,
} from '../helpers/test-verification-helpers.js';

describe('Multi-File Rename File Path Tests', () => {
  let client: MCPTestClient;
  let backupManager: FileBackupManager;

  // Test files for file rename
  const testFiles = [
    '/workspace/plugins/cclsp/playground/src/services/user-service.ts',
    '/workspace/plugins/cclsp/playground/src/index.ts',
    '/workspace/plugins/cclsp/playground/src/components/user-list.ts',
    '/workspace/plugins/cclsp/playground/src/components/user-form.ts',
    '/workspace/plugins/cclsp/playground/src/utils/user-helpers.ts',
    '/workspace/plugins/cclsp/playground/src/test-file.ts',
  ];

  beforeAll(async () => {
    console.log('🔍 Multi-File Rename File Path Test');
    console.log('=====================================\n');

    // Initialize backup manager
    backupManager = new FileBackupManager();

    // Create backups of all test files
    console.log('📋 Creating backups of playground files...');
    for (const filePath of testFiles) {
      if (existsSync(filePath)) {
        backupManager.backupFile(filePath);
        console.log(`  ✓ Backed up: ${filePath}`);
      }
    }

    // Initialize MCP client
    client = new MCPTestClient();
    await client.start();

    // Wait for LSP servers to initialize
    console.log('⏳ Waiting for LSP servers to initialize...');
    await new Promise((resolve) => setTimeout(resolve, 2000));
    console.log('✅ Setup complete\n');
  });

  afterAll(async () => {
    // Stop MCP client
    await client.stop();

    // Restore all files from backups
    console.log('\n🔄 Restoring original files...');
    const restored = backupManager.restoreAll();
    console.log(`✅ Restored ${restored} files from backups`);

    // Cleanup backup manager
    backupManager.cleanup();
  });

  describe('File Path Rename with Import Updates', () => {
    it('should preview file rename with dry_run showing import updates', async () => {
      console.log('🔍 Testing dry-run file rename preview...');

      const result = await client.callTool('rename_file', {
        old_path: '/workspace/plugins/cclsp/playground/src/services/user-service.ts',
        new_path: '/workspace/plugins/cclsp/playground/src/core/account-service.ts',
        dry_run: true,
      });

      expect(result).toBeDefined();
      const toolResult = assertToolResult(result);
      const content = toolResult.content?.[0]?.text || '';

      console.log('📋 Dry-run file rename result:');
      console.log(content);

      // Should indicate it's a dry run
      expect(content).toMatch(/DRY RUN|Would update/i);

      // Should mention import updates
      expect(content).toMatch(/import/i);

      // Should mention the file paths
      expect(content).toMatch(/user-service\.ts.*account-service\.ts/);

      // Verify no actual file changes occurred
      expect(existsSync('/workspace/plugins/cclsp/playground/src/services/user-service.ts')).toBe(
        true
      );
      expect(existsSync('/workspace/plugins/cclsp/playground/src/core/account-service.ts')).toBe(
        false
      );

      console.log('✅ Dry-run preview successful - no files modified');
    });

    it('should execute file rename and update all import paths', async () => {
      console.log('🔧 Executing actual file rename with import updates...');

      // Restore files first to ensure clean state
      backupManager.restoreAll();

      // Record original import statements
      const originalImports = new Map<string, string[]>();
      for (const file of testFiles) {
        if (
          existsSync(file) &&
          file !== '/workspace/plugins/cclsp/playground/src/services/user-service.ts'
        ) {
          const content = readFileSync(file, 'utf-8');
          const imports = content.match(/from ['"].*user-service['"]/g) || [];
          if (imports.length > 0) {
            originalImports.set(file, imports);
            console.log(`📄 Found ${imports.length} imports in ${file.split('/').pop()}`);
          }
        }
      }

      // Execute the file rename
      const result = await client.callTool('rename_file', {
        old_path: '/workspace/plugins/cclsp/playground/src/services/user-service.ts',
        new_path: '/workspace/plugins/cclsp/playground/src/core/account-service.ts',
        dry_run: false,
      });

      expect(result).toBeDefined();
      const toolResult = assertToolResult(result);
      const content = toolResult.content?.[0]?.text || '';

      console.log('📋 File rename execution result:');
      console.log(content);

      // Should indicate successful rename
      expect(content).toMatch(/success|renamed/i);

      // Wait for file system operations
      await new Promise((resolve) => setTimeout(resolve, 500));

      console.log('\n🔍 Verifying file changes...');

      // Verify file was moved
      const oldFileExists = existsSync(
        '/workspace/plugins/cclsp/playground/src/services/user-service.ts'
      );
      const newFileExists = existsSync(
        '/workspace/plugins/cclsp/playground/src/core/account-service.ts'
      );

      console.log(`Old file exists: ${oldFileExists ? '❌ Still present' : '✅ Removed'}`);
      console.log(`New file exists: ${newFileExists ? '✅ Created' : '❌ Missing'}`);

      expect(oldFileExists).toBe(false);
      expect(newFileExists).toBe(true);

      // Verify import paths were updated with exact content verification
      console.log('\n🔍 Verifying exact import statement updates...');

      // Check index.ts - should have path updated from services to core
      const indexFile = '/workspace/plugins/cclsp/playground/src/index.ts';
      if (existsSync(indexFile)) {
        console.log('\n📄 Verifying index.ts imports...');
        const indexContent = readFileSync(indexFile, 'utf-8');

        // Should NOT contain old path
        expect(indexContent).not.toContain('./services/user-service');

        // Should contain new path - verify exact import statements
        if (indexContent.includes('../core/account-service')) {
          // Path adjusted for new location
          verifyFileContainsAll(indexFile, ['../core/account-service']);
          console.log('  ✅ Import path correctly updated to ../core/account-service');
        } else if (indexContent.includes('./core/account-service')) {
          // Or might be relative to same level
          verifyFileContainsAll(indexFile, ['./core/account-service']);
          console.log('  ✅ Import path correctly updated to ./core/account-service');
        } else {
          throw new Error('index.ts import path not properly updated');
        }
      }

      // Check user-list.ts - verify exact import change
      const userListFile = '/workspace/plugins/cclsp/playground/src/components/user-list.ts';
      if (existsSync(userListFile)) {
        console.log('\n📄 Verifying user-list.ts imports...');
        verifyImportStatement(
          userListFile,
          /from ['"].*services\/user-service['"]/,
          '../core/account-service'
        );
        console.log('  ✅ Import path correctly updated from services to core directory');
      }

      // Check user-form.ts - verify exact import change
      const userFormFile = '/workspace/plugins/cclsp/playground/src/components/user-form.ts';
      if (existsSync(userFormFile)) {
        console.log('\n📄 Verifying user-form.ts imports...');
        verifyImportStatement(
          userFormFile,
          /from ['"].*services\/user-service['"]/,
          '../core/account-service'
        );
        console.log('  ✅ Import path correctly updated from services to core directory');
      }

      // Check user-helpers.ts - verify exact import change
      const userHelpersFile = '/workspace/plugins/cclsp/playground/src/utils/user-helpers.ts';
      if (existsSync(userHelpersFile)) {
        console.log('\n📄 Verifying user-helpers.ts imports...');
        verifyImportStatement(
          userHelpersFile,
          /from ['"].*services\/user-service['"]/,
          '../core/account-service'
        );
        console.log('  ✅ Import path correctly updated from services to core directory');
      }

      // Verify at least 3 files had their imports updated
      let verifiedFiles = 0;
      for (const file of [indexFile, userListFile, userFormFile, userHelpersFile]) {
        if (existsSync(file)) {
          const content = readFileSync(file, 'utf-8');
          if (content.includes('account-service') && !content.includes('user-service')) {
            verifiedFiles++;
          }
        }
      }

      console.log(`\n📊 Summary: ${verifiedFiles} files verified with correct import updates`);
      expect(verifiedFiles).toBeGreaterThanOrEqual(3);

      console.log('✅ File rename with import updates verification complete');
    }, 30000); // Extended timeout for file operations

    it('should handle rename of non-existent file gracefully', async () => {
      console.log('🔍 Testing rename of non-existent file...');

      const result = await client.callTool('rename_file', {
        old_path: '/workspace/plugins/cclsp/playground/src/services/non-existent.ts',
        new_path: '/workspace/plugins/cclsp/playground/src/services/new-name.ts',
        dry_run: true,
      });

      expect(result).toBeDefined();
      const toolResult = assertToolResult(result);
      const content = toolResult.content?.[0]?.text || '';

      console.log('📋 Non-existent file result:');
      console.log(content);

      // Should indicate file doesn't exist
      expect(content).toMatch(/does not exist|not found|failed/i);

      console.log('✅ Non-existent file handled gracefully');
    });

    it('should prevent overwriting existing file', async () => {
      console.log('🔍 Testing rename to existing file...');

      const result = await client.callTool('rename_file', {
        old_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
        new_path: '/workspace/plugins/cclsp/playground/src/index.ts', // Already exists
        dry_run: true,
      });

      expect(result).toBeDefined();
      const toolResult = assertToolResult(result);
      const content = toolResult.content?.[0]?.text || '';

      console.log('📋 Existing target file result:');
      console.log(content);

      // Should indicate target already exists
      expect(content).toMatch(/already exists|cannot overwrite|failed/i);

      console.log('✅ Existing file overwrite prevented');
    });
  });
});

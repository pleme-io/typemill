
import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { assertToolResult, MCPTestClient } from '../helpers/mcp-test-client.js';
import { waitForLSP } from '../helpers/test-verification-helpers.js';

describe.skip('Batch Directory Move Integration Tests', () => {
  let client: MCPTestClient;
  const TEST_DIR = join(tmpdir(), 'batch-dir-move-test');
  const initialFileStates = new Map<string, string>();

  const filePaths = {
    tsconfig: join(TEST_DIR, 'tsconfig.json'),
    pkgJson: join(TEST_DIR, 'package.json'),
    main: join(TEST_DIR, 'main.ts'),
    dirA_file: join(TEST_DIR, 'src', 'dir_a', 'file_a.ts'),
    dirB_file: join(TEST_DIR, 'src', 'dir_b', 'file_b.ts'),
    dirC_blockingFile: join(TEST_DIR, 'dest', 'dir_c', 'file_c.ts'),
    srcDir: join(TEST_DIR, 'src'),
    destDir: join(TEST_DIR, 'dest'),
    dirA: join(TEST_DIR, 'src', 'dir_a'),
    dirB: join(TEST_DIR, 'src', 'dir_b'),
    dirC: join(TEST_DIR, 'src', 'dir_c'),
    destDirA: join(TEST_DIR, 'dest', 'dir_a'),
    destDirB: join(TEST_DIR, 'dest', 'dir_b'),
    destDirC: join(TEST_DIR, 'dest', 'dir_c'),
  };

  beforeAll(async () => {
    console.log('🧪 Batch Directory Move Integration Test');
    console.log('========================================');

    // Create isolated test directory
    rmSync(TEST_DIR, { recursive: true, force: true });
    mkdirSync(TEST_DIR, { recursive: true });
    mkdirSync(filePaths.srcDir, { recursive: true });
    mkdirSync(filePaths.destDir, { recursive: true });
    mkdirSync(filePaths.dirA, { recursive: true });
    mkdirSync(filePaths.dirB, { recursive: true });
    mkdirSync(filePaths.dirC, { recursive: true });

    // --- Create Project Files ---
    writeFileSync(filePaths.tsconfig, '{ "compilerOptions": { "module": "ESNext", "moduleResolution": "node" } }');
    writeFileSync(filePaths.pkgJson, '{ "name": "test-project", "type": "module" }');
    writeFileSync(filePaths.dirA_file, "export const A = 'A';");
    writeFileSync(filePaths.dirB_file, "export const B = 'B';");
    writeFileSync(join(filePaths.dirC, 'file_c.ts'), "export const C = 'C';");
    writeFileSync(filePaths.main, `import { A } from './src/dir_a/file_a';
import { B } from './src/dir_b/file_b';
console.log(A, B);`);

    // Store initial states for rollback verification
    initialFileStates.set(filePaths.main, readFileSync(filePaths.main, 'utf-8'));
    initialFileStates.set(filePaths.dirA_file, readFileSync(filePaths.dirA_file, 'utf-8'));
    initialFileStates.set(filePaths.dirB_file, readFileSync(filePaths.dirB_file, 'utf-8'));

    client = new MCPTestClient();
    await client.start({ skipLSPPreload: true });
    await waitForLSP(client, filePaths.main);
    console.log('✅ Setup complete');
  });

  afterAll(async () => {
    await client.stop();
    rmSync(TEST_DIR, { recursive: true, force: true });
    console.log('✅ Cleanup complete');
  });

  it('should atomically fail and roll back all directory moves if one operation fails', async () => {
    console.log('💥 Testing atomic rollback of batch directory moves...');

    // --- Engineer a failure ---
    // Create a file where one of the directories is supposed to be moved, causing a collision.
    mkdirSync(filePaths.destDirC, { recursive: true });
    writeFileSync(filePaths.dirC_blockingFile, 'BLOCKING FILE');

    // --- Attempt the batch move ---
    const result = await client.callTool('batch_execute', {
      operations: [
        { tool: 'rename_directory', args: { old_path: filePaths.dirA, new_path: filePaths.destDirA }, id: 'move-a' },
        { tool: 'rename_directory', args: { old_path: filePaths.dirB, new_path: filePaths.destDirB }, id: 'move-b' },
        { tool: 'rename_directory', args: { old_path: filePaths.dirC, new_path: filePaths.destDirC }, id: 'move-c-fail' }, // This will fail
      ],
      options: { atomic: true, stop_on_error: true },
    });

    assertToolResult(result);
    const response = result.content[0]?.text || '';

    // --- Verify Failure and Rollback ---
    console.log('🔍 Verifying failure report...');
    expect(response).toContain('Batch Execution Results');
    expect(response).toContain('❌ Execution Results (with errors)');
    expect(response).toContain('Error: Target directory already exists');
    expect(response).toContain('Rolling back atomic transaction - all operations have been reverted');

    console.log('🔍 Verifying file system has been rolled back...');
    // 1. Original directories should still exist
    expect(existsSync(filePaths.dirA)).toBe(true);
    expect(existsSync(filePaths.dirB)).toBe(true);
    expect(existsSync(filePaths.dirC)).toBe(true);

    // 2. Destination directories should NOT exist (except the one we manually created)
    expect(existsSync(filePaths.destDirA)).toBe(false);
    expect(existsSync(filePaths.destDirB)).toBe(false);
    expect(existsSync(filePaths.destDirC)).toBe(true); // The blocking dir still exists

    // 3. File contents should be in their original state
    expect(readFileSync(filePaths.main, 'utf-8')).toBe(initialFileStates.get(filePaths.main));
    expect(readFileSync(filePaths.dirA_file, 'utf-8')).toBe(initialFileStates.get(filePaths.dirA_file));
    expect(readFileSync(filePaths.dirB_file, 'utf-8')).toBe(initialFileStates.get(filePaths.dirB_file));
    expect(readFileSync(filePaths.dirC_blockingFile, 'utf-8')).toBe('BLOCKING FILE');

    console.log('✅ Atomic rollback verified successfully.');
  }, 30000);

  it('should successfully execute a batch of directory moves and update imports', async () => {
    console.log('🚀 Testing successful batch directory move...');

    // Clean up from previous test - make sure all dest directories are removed
    rmSync(filePaths.destDirA, { recursive: true, force: true });
    rmSync(filePaths.destDirB, { recursive: true, force: true });
    rmSync(filePaths.destDirC, { recursive: true, force: true });

    const result = await client.callTool('batch_execute', {
        operations: [
          { tool: 'rename_directory', args: { old_path: filePaths.dirA, new_path: filePaths.destDirA }, id: 'move-a' },
          { tool: 'rename_directory', args: { old_path: filePaths.dirB, new_path: filePaths.destDirB }, id: 'move-b' },
        ],
        options: { atomic: true, stop_on_error: true },
      });

      assertToolResult(result);
      const response = result.content[0]?.text || '';

      console.log('🔍 Verifying success report...');
      expect(response).toContain('🎉 All operations completed successfully!');

      console.log('🔍 Verifying file system changes...');
      // 1. Original directories should be gone
      expect(existsSync(filePaths.dirA)).toBe(false);
      expect(existsSync(filePaths.dirB)).toBe(false);

      // 2. Destination directories should exist
      expect(existsSync(filePaths.destDirA)).toBe(true);
      expect(existsSync(filePaths.destDirB)).toBe(true);

      console.log('🔍 Verifying import updates...');
      const updatedMainContent = readFileSync(filePaths.main, 'utf-8');
      expect(updatedMainContent).toContain('from "./dest/dir_a/file_a"');
      expect(updatedMainContent).toContain('from "./dest/dir_b/file_b"');
      expect(updatedMainContent).not.toContain("from './src/dir_a/file_a'");
      expect(updatedMainContent).not.toContain("from './src/dir_b/file_b'");

      console.log('✅ Successful batch directory move verified.');
  }, 30000);
});

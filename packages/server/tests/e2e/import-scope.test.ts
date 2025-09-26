
import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { assertToolResult, MCPTestClient } from '../helpers/mcp-test-client.js';
import { waitForLSP } from '../helpers/test-verification-helpers.js';

describe('Import Scope Integration Test', () => {
  let client: MCPTestClient;
  const TEST_DIR = join(tmpdir(), 'import-scope-test');

  const filePaths = {
    tsconfig: join(TEST_DIR, 'tsconfig.json'),
    pkgJson: join(TEST_DIR, 'package.json'),
    index: join(TEST_DIR, 'index.ts'),
    util: join(TEST_DIR, 'src', 'utils', 'util.ts'),
    srcDir: join(TEST_DIR, 'src'),
    utilsDir: join(TEST_DIR, 'src', 'utils'),
    libDir: join(TEST_DIR, 'lib'),
    destUtilsDir: join(TEST_DIR, 'lib', 'utils'),
  };

  beforeAll(async () => {
    console.log('🧪 Import Scope Integration Test');
    console.log('================================');

    // Create isolated test directory
    rmSync(TEST_DIR, { recursive: true, force: true });
    mkdirSync(TEST_DIR, { recursive: true });
    mkdirSync(filePaths.srcDir, { recursive: true });
    mkdirSync(filePaths.utilsDir, { recursive: true });
    mkdirSync(filePaths.libDir, { recursive: true });

    // --- Create Project Files ---
    writeFileSync(filePaths.tsconfig, '{ "compilerOptions": { "module": "ESNext", "moduleResolution": "node" } }');
    writeFileSync(filePaths.pkgJson, '{ "name": "test-project", "type": "module" }');
    writeFileSync(filePaths.util, "export const HELLO = 'WORLD';");

    // index.ts is in the root, importing from a nested directory.
    // This is the exact scenario that failed for Bob.
    writeFileSync(filePaths.index, `import { HELLO } from './src/utils/util';
console.log(HELLO);`);

    client = new MCPTestClient();
    await client.start({ skipLSPPreload: true });
    await waitForLSP(client, filePaths.index);
    console.log('✅ Setup complete');
  });

  afterAll(async () => {
    await client.stop();
    rmSync(TEST_DIR, { recursive: true, force: true });
    console.log('✅ Cleanup complete');
  });

  it('should update imports in parent directories when a nested directory is moved', async () => {
    console.log('🚀 Testing import updates in parent-level files...');

    const initialIndexContent = readFileSync(filePaths.index, 'utf-8');
    expect(initialIndexContent).toContain('./src/utils/util');

    // --- Execute the move ---
    // Moving src/utils -> lib/utils
    const result = await client.callTool('rename_directory', {
      old_path: filePaths.utilsDir,
      new_path: filePaths.destUtilsDir,
    });

    assertToolResult(result);
    const response = result.content[0]?.text || '';

    // --- Verify Success ---
    console.log('🔍 Verifying success report...');
    expect(response).toContain('Directory Rename Complete');
    expect(response).toContain('Success**: 1 file(s)');

    console.log('🔍 Verifying file system changes...');
    expect(existsSync(filePaths.utilsDir)).toBe(false);
    expect(existsSync(filePaths.destUtilsDir)).toBe(true);

    console.log('🔍 Verifying import update in parent file (index.ts)...');
    const updatedIndexContent = readFileSync(filePaths.index, 'utf-8');
    expect(updatedIndexContent).toContain('from "./lib/utils/util"');
    expect(updatedIndexContent).not.toContain("from './src/utils/util'");

    console.log('✅ Parent directory import updated successfully.');
  }, 30000);
});

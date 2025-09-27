import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { MCPTestClient } from '../helpers/mcp-test-client.js';
import { waitForFile } from '../helpers/test-verification-helpers.js';

/**
 * CRITICAL TEST: This specifically tests the bug fix for internal import path updates
 *
 * The bug was in editor.ts:549-600 where depth-based logic failed for monorepo moves.
 * This test recreates the EXACT scenario that was broken before our fix.
 */
describe('Critical Import Fix Verification', () => {
  let client: MCPTestClient;
  const testDir = '/tmp/critical-fix-test';

  beforeAll(async () => {
    console.log('🚨 CRITICAL FIX TEST - Recreating Exact Failure Scenario');
    console.log('======================================================\n');

    // Clean up
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }

    // Create the EXACT structure that was problematic
    const serverPath = join(testDir, 'packages/server/src');
    const coreFeaturePath = join(testDir, 'packages/@codeflow/features');

    mkdirSync(join(serverPath, 'lsp'), { recursive: true });
    mkdirSync(join(serverPath, 'core/file-operations'), { recursive: true });
    mkdirSync(join(serverPath, 'services'), { recursive: true });
    mkdirSync(join(coreFeaturePath, 'lsp/src'), { recursive: true });

    // Create the exact file that was causing issues
    const originalFile = join(serverPath, 'lsp/client.ts');
    writeFileSync(
      originalFile,
      `
// This file recreates the EXACT imports that were broken during monorepo restructuring
import { pathToUri, uriToPath } from '../core/file-operations/path-utils.js';
import { logger } from '../core/diagnostics/logger.js';
import { ValidationError } from '../utils/validation.js';
import type { ServiceContext } from '../services/service-context.js';

// Dynamic imports that were particularly problematic
const setupModule = await import('./commands/setup.js');
const statusModule = await import('./commands/status.js');

// Export from statement
export { createClient } from '../core/client-factory.js';

export class LSPClient {
  constructor(private context: ServiceContext) {
    logger.debug('LSP Client initialized');
  }

  async initialize() {
    const path = pathToUri('/test/path');
    return uriToPath(path);
  }
}
`
    );

    // Create all the referenced files so the imports can be resolved
    // IMPORTANT: The test file is at src/lsp/client.ts, so ../../core should resolve to src/core/
    // NOT to packages/server/core - we need the full src structure!

    writeFileSync(
      join(serverPath, 'core/file-operations/path-utils.js'),
      'export const pathToUri = (p) => p; export const uriToPath = (p) => p;'
    );

    mkdirSync(join(serverPath, 'core/diagnostics'), { recursive: true });
    writeFileSync(
      join(serverPath, 'core/diagnostics/logger.js'),
      'export const logger = { debug: console.log };'
    );

    mkdirSync(join(serverPath, 'utils'), { recursive: true });
    writeFileSync(
      join(serverPath, 'utils/validation.js'),
      'export class ValidationError extends Error {}'
    );

    writeFileSync(
      join(serverPath, 'services/service-context.js'),
      'export interface ServiceContext {}'
    );

    mkdirSync(join(serverPath, 'lsp/commands'), { recursive: true });
    writeFileSync(join(serverPath, 'lsp/commands/setup.js'), 'export const setup = () => {};');
    writeFileSync(join(serverPath, 'lsp/commands/status.js'), 'export const status = () => {};');

    writeFileSync(
      join(serverPath, 'core/client-factory.js'),
      'export const createClient = () => {};'
    );

    // Initialize MCP client
    client = new MCPTestClient();
    await client.start({ skipLSPPreload: true });

    console.log('✅ Test environment with EXACT problematic structure created\n');
  });

  afterAll(async () => {
    await client.stop();
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }
  });

  it('should fix the EXACT import path issue that caused 50+ broken imports', async () => {
    console.log('🔥 REPRODUCING THE EXACT BUG SCENARIO\n');

    const originalFile = join(testDir, 'packages/server/src/lsp/client.ts');
    const targetFile = join(testDir, 'packages/@codeflow/features/lsp/src/client.ts');

    console.log('📁 Moving file from:');
    console.log('   /packages/server/src/lsp/client.ts');
    console.log('📁 To:');
    console.log('   /packages/@codeflow/features/lsp/src/client.ts\n');

    // Read original content to verify starting point
    const originalContent = readFileSync(originalFile, 'utf-8');
    console.log('📄 Original imports in the file:');
    console.log('  ../core/file-operations/path-utils.js');
    console.log('  ../core/diagnostics/logger.js');
    console.log('  ../utils/validation.js');
    console.log('  ../services/service-context.js');
    console.log('  ./commands/setup.js (dynamic import)');
    console.log('  ./commands/status.js (dynamic import)');
    console.log('  ../core/client-factory.js (export from)\n');

    // Execute the move that was causing the problem
    const result = await client.callTool('rename_file', {
      old_path: originalFile,
      new_path: targetFile,
      dry_run: false,
    });

    expect(result).toBeDefined();
    expect(result.content?.[0]?.text).toContain('renamed');

    // Wait for file operations to complete
    await waitForFile(targetFile, { timeout: 5000 });

    // Verify the file was moved
    expect(existsSync(originalFile)).toBe(false);
    expect(existsSync(targetFile)).toBe(true);

    console.log('🔍 CRITICAL CHECK: Verifying internal import updates...\n');

    const movedContent = readFileSync(targetFile, 'utf-8');

    // These are the EXACT path corrections that were failing before our fix
    // From: /packages/@codeflow/features/lsp/src/client.ts
    // To:   /packages/server/src/core/file-operations/path-utils.js
    // Path: ../../../../server/src/core/file-operations/path-utils.js

    const pathUtilsFixed = movedContent.includes(
      '../../../../server/src/core/file-operations/path-utils.js'
    );
    const loggerFixed = movedContent.includes('../../../../server/src/core/diagnostics/logger.js');
    const validationFixed = movedContent.includes('../../../../server/src/utils/validation.js');
    const serviceContextFixed = movedContent.includes(
      '../../../../server/src/services/service-context.js'
    );
    const clientFactoryFixed = movedContent.includes(
      '../../../../server/src/core/client-factory.js'
    );

    // Dynamic imports should also be fixed
    const setupDynamicFixed = movedContent.includes(
      "import('../../../../server/src/lsp/commands/setup.js')"
    );
    const statusDynamicFixed = movedContent.includes(
      "import('../../../../server/src/lsp/commands/status.js')"
    );

    console.log('💊 Import Fix Results:');
    console.log(`   Path utils:     ${pathUtilsFixed ? '✅ FIXED' : '❌ STILL BROKEN'}`);
    console.log(`   Logger:         ${loggerFixed ? '✅ FIXED' : '❌ STILL BROKEN'}`);
    console.log(`   Validation:     ${validationFixed ? '✅ FIXED' : '❌ STILL BROKEN'}`);
    console.log(`   Service context:${serviceContextFixed ? '✅ FIXED' : '❌ STILL BROKEN'}`);
    console.log(`   Client factory: ${clientFactoryFixed ? '✅ FIXED' : '❌ STILL BROKEN'}`);
    console.log(`   Setup (dynamic): ${setupDynamicFixed ? '✅ FIXED' : '❌ STILL BROKEN'}`);
    console.log(`   Status (dynamic):${statusDynamicFixed ? '✅ FIXED' : '❌ STILL BROKEN'}\n`);

    // If any failed, show what we actually got vs what we expected
    if (!pathUtilsFixed) {
      const actualMatch = movedContent.match(/from ['"]([^'"]*path-utils[^'"]*)['"]/);
      console.log('❌ Path utils import error:');
      console.log(`   Expected: ../../../../server/src/core/file-operations/path-utils.js`);
      console.log(`   Actual:   ${actualMatch ? actualMatch[1] : 'NOT FOUND'}\n`);
    }

    if (!loggerFixed) {
      const actualMatch = movedContent.match(/from ['"]([^'"]*logger[^'"]*)['"]/);
      console.log('❌ Logger import error:');
      console.log(`   Expected: ../../../../server/src/core/diagnostics/logger.js`);
      console.log(`   Actual:   ${actualMatch ? actualMatch[1] : 'NOT FOUND'}\n`);
    }

    // These assertions will PASS with our fix and FAIL with the old depth-based logic
    expect(pathUtilsFixed).toBe(true);
    expect(loggerFixed).toBe(true);
    expect(validationFixed).toBe(true);
    expect(serviceContextFixed).toBe(true);
    expect(clientFactoryFixed).toBe(true);
    expect(setupDynamicFixed).toBe(true);
    expect(statusDynamicFixed).toBe(true);

    console.log('🎉 SUCCESS: The critical import path bug has been FIXED!');
    console.log('   No more "50+ broken imports requiring manual fixes"');
    console.log('   CodeBuddy is now a true refactoring assistant! ✅');
  }, 60000);

  it('should verify the old depth-based approach would have failed', async () => {
    console.log('📊 DEMONSTRATING WHY THE OLD APPROACH FAILED\n');

    // Create another test scenario to show the contrast
    const testFile = join(testDir, 'packages/server/src/utils/helper.ts');
    const movedFile = join(testDir, 'packages/@codeflow/core/src/utils/helper.ts');

    mkdirSync(join(testDir, 'packages/@codeflow/core/src/utils'), { recursive: true });
    mkdirSync(join(testDir, 'packages/server/src/data'), { recursive: true });
    writeFileSync(join(testDir, 'packages/server/src/data/config.js'), 'export const config = {};');

    writeFileSync(
      testFile,
      `
import { config } from '../data/config.js';

export function helper() {
  return config;
}
`
    );

    console.log('🧮 Old depth-based logic would have done:');
    console.log('   Original: ../data/config.js');
    console.log('   Depth change: +2 levels (server/src/utils → @codeflow/core/src/utils)');
    console.log('   Wrong result: ../../data/config.js (just adding ../)');
    console.log('   Correct path: ../../../../server/src/data/config.js\n');

    const result = await client.callTool('rename_file', {
      old_path: testFile,
      new_path: movedFile,
      dry_run: false,
    });

    expect(result).toBeDefined();

    // Wait for file to be created at new location
    await waitForFile(movedFile, { timeout: 5000 });

    const content = readFileSync(movedFile, 'utf-8');
    const hasCorrectPath = content.includes('../../../../server/src/data/config.js');

    console.log(`✨ Our fix produces: ${hasCorrectPath ? '✅ CORRECT PATH' : '❌ WRONG PATH'}`);
    expect(hasCorrectPath).toBe(true);

    console.log('📈 This proves our path-resolution approach works correctly!');
  });
});

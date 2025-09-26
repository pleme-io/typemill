import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { MCPTestClient } from '../helpers/mcp-test-client.js';
import { waitForCondition } from '../helpers/polling-helpers.js';

describe('Import Path Update Tests - Critical Fix Verification', () => {
  let client: MCPTestClient;
  const testDir = '/tmp/import-test';
  const originalFile = join(testDir, 'src/services/user-service.ts');
  const movedFile = join(testDir, 'src/core/features/user/user-service.ts');

  beforeAll(async () => {
    console.log('🔍 Import Path Update Test - Verifying Critical Fix');
    console.log('===================================================\n');

    // Clean up any existing test directory
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }

    // Create test directory structure
    mkdirSync(join(testDir, 'src/services'), { recursive: true });
    mkdirSync(join(testDir, 'src/utils'), { recursive: true });
    mkdirSync(join(testDir, 'src/models'), { recursive: true });

    // Create test files with various import patterns

    // File with imports that need updating when moved
    writeFileSync(originalFile, `
import { helper } from '../../utils/helper.js';
import { User } from '../../models/user.js';
import { config } from '../../config/settings.js';
const validator = await import('../../utils/validator.js');
export { formatUser } from '../../utils/formatter.js';

export class UserService {
  private data = require('../../data/users.json');

  async getUser(id: string) {
    const user = new User(id);
    return helper.process(user);
  }
}
`);

    // Create the imported files so they exist
    writeFileSync(join(testDir, 'src/utils/helper.js'), 'export const helper = { process: (x) => x };');
    writeFileSync(join(testDir, 'src/utils/validator.js'), 'export const validate = (x) => true;');
    writeFileSync(join(testDir, 'src/utils/formatter.js'), 'export const formatUser = (x) => x;');
    writeFileSync(join(testDir, 'src/models/user.js'), 'export class User { constructor(id) { this.id = id; } }');
    mkdirSync(join(testDir, 'src/config'), { recursive: true });
    writeFileSync(join(testDir, 'src/config/settings.js'), 'export const config = {};');
    mkdirSync(join(testDir, 'src/data'), { recursive: true });
    writeFileSync(join(testDir, 'src/data/users.json'), '[]');

    // Initialize MCP client
    client = new MCPTestClient();
    await client.start({ skipLSPPreload: true });

    console.log('✅ Test environment setup complete\n');
  });

  afterAll(async () => {
    await client.stop();

    // Clean up test directory
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }

    console.log('✅ Cleanup complete');
  });

  describe('Internal Import Updates in Moved Files', () => {
    it('should update all import paths WITHIN a moved file to maintain correct references', async () => {
      console.log('🔧 Testing internal import path updates in moved file...\n');

      // Verify original file exists and has expected imports
      expect(existsSync(originalFile)).toBe(true);
      const originalContent = readFileSync(originalFile, 'utf-8');

      console.log('📄 Original file imports:');
      console.log('  - ../../utils/helper.js');
      console.log('  - ../../models/user.js');
      console.log('  - ../../config/settings.js');
      console.log('  - ../../utils/validator.js (dynamic)');
      console.log('  - ../../utils/formatter.js (export from)');
      console.log('  - ../../data/users.json (require)\n');

      // Move the file to a deeper nested location
      const result = await client.callTool('rename_file', {
        old_path: originalFile,
        new_path: movedFile,
        dry_run: false,
      });

      expect(result).toBeDefined();
      expect(result.content?.[0]?.text).toContain('renamed');

      // Wait for file operations
      await waitForCondition(() => existsSync(movedFile) && !existsSync(originalFile), { timeout: 500, interval: 100 });

      // Verify file was moved
      expect(existsSync(originalFile)).toBe(false);
      expect(existsSync(movedFile)).toBe(true);

      // Read the moved file and check if imports were updated
      const movedContent = readFileSync(movedFile, 'utf-8');

      console.log('🔍 Verifying import path updates in moved file...\n');

      // The file moved from src/services to src/core/features/user (2 levels deeper)
      // So imports should change from ../../ to ../../../../

      // Check static imports
      const hasCorrectHelperImport = movedContent.includes("from '../../../../utils/helper.js'") ||
                                     movedContent.includes('from "../../../../utils/helper.js"');
      const hasCorrectUserImport = movedContent.includes("from '../../../../models/user.js'") ||
                                   movedContent.includes('from "../../../../models/user.js"');
      const hasCorrectConfigImport = movedContent.includes("from '../../../../config/settings.js'") ||
                                     movedContent.includes('from "../../../../config/settings.js"');

      // Check dynamic import
      const hasCorrectDynamicImport = movedContent.includes("import('../../../../utils/validator.js')") ||
                                      movedContent.includes('import("../../../../utils/validator.js")');

      // Check export from
      const hasCorrectExportFrom = movedContent.includes("from '../../../../utils/formatter.js'") ||
                                   movedContent.includes('from "../../../../utils/formatter.js"');

      // Check require
      const hasCorrectRequire = movedContent.includes("require('../../../../data/users.json')") ||
                               movedContent.includes('require("../../../../data/users.json")');

      // Log actual vs expected for debugging
      if (!hasCorrectHelperImport) {
        console.log('❌ Helper import not updated correctly');
        console.log('   Expected: ../../../../utils/helper.js');
        const match = movedContent.match(/from ['"]([^'"]*helper[^'"]*)['"]/);
        if (match) console.log(`   Found: ${match[1]}`);
      } else {
        console.log('✅ Helper import updated correctly');
      }

      if (!hasCorrectUserImport) {
        console.log('❌ User model import not updated correctly');
        console.log('   Expected: ../../../../models/user.js');
        const match = movedContent.match(/from ['"]([^'"]*user[^'"]*)['"]/);
        if (match) console.log(`   Found: ${match[1]}`);
      } else {
        console.log('✅ User model import updated correctly');
      }

      if (!hasCorrectConfigImport) {
        console.log('❌ Config import not updated correctly');
        console.log('   Expected: ../../../../config/settings.js');
        const match = movedContent.match(/from ['"]([^'"]*settings[^'"]*)['"]/);
        if (match) console.log(`   Found: ${match[1]}`);
      } else {
        console.log('✅ Config import updated correctly');
      }

      if (!hasCorrectDynamicImport) {
        console.log('❌ Dynamic import not updated correctly');
        console.log('   Expected: import("../../../../utils/validator.js")');
        const match = movedContent.match(/import\(['"]([^'"]*validator[^'"]*)['"]\)/);
        if (match) console.log(`   Found: import("${match[1]}")`);
      } else {
        console.log('✅ Dynamic import updated correctly');
      }

      if (!hasCorrectExportFrom) {
        console.log('❌ Export from not updated correctly');
        console.log('   Expected: from "../../../../utils/formatter.js"');
        const match = movedContent.match(/export.*from ['"]([^'"]*formatter[^'"]*)['"]/);
        if (match) console.log(`   Found: ${match[1]}`);
      } else {
        console.log('✅ Export from statement updated correctly');
      }

      if (!hasCorrectRequire) {
        console.log('❌ Require statement not updated correctly');
        console.log('   Expected: require("../../../../data/users.json")');
        const match = movedContent.match(/require\(['"]([^'"]*users\.json[^'"]*)['"]\)/);
        if (match) console.log(`   Found: require("${match[1]}")`);
      } else {
        console.log('✅ Require statement updated correctly');
      }

      // Assert all imports were updated correctly
      expect(hasCorrectHelperImport).toBe(true);
      expect(hasCorrectUserImport).toBe(true);
      expect(hasCorrectConfigImport).toBe(true);
      expect(hasCorrectDynamicImport).toBe(true);
      expect(hasCorrectExportFrom).toBe(true);
      expect(hasCorrectRequire).toBe(true);

      console.log('\n✅ All import paths within moved file were updated correctly!');
    }, 30000);

    it('should handle lateral moves across monorepo structure correctly', async () => {
      console.log('🔧 Testing lateral move across monorepo packages...\n');

      // Create a monorepo-like structure
      const monoRepoDir = '/tmp/monorepo-test';
      if (existsSync(monoRepoDir)) {
        rmSync(monoRepoDir, { recursive: true, force: true });
      }

      // Create structure: packages/server/src/lsp/client.ts -> packages/@scope/features/lsp/src/client.ts
      const serverFile = join(monoRepoDir, 'packages/server/src/lsp/client.ts');
      const scopedFile = join(monoRepoDir, 'packages/@scope/features/lsp/src/client.ts');

      mkdirSync(dirname(serverFile), { recursive: true });
      mkdirSync(join(monoRepoDir, 'packages/server/src/core'), { recursive: true });

      // Create file with imports that correctly reference the actual file locations
      writeFileSync(serverFile, `
import { pathUtils } from '../core/file-operations/path-utils.js';
import { logger } from '../core/diagnostics/logger.js';
import { config } from '../../../shared/config.js';

export class LSPClient {
  constructor() {
    logger.info('LSP Client initialized');
  }
}
`);

      // Create the referenced files
      mkdirSync(join(monoRepoDir, 'packages/server/src/core/file-operations'), { recursive: true });
      writeFileSync(join(monoRepoDir, 'packages/server/src/core/file-operations/path-utils.js'),
        'export const pathUtils = {};');

      mkdirSync(join(monoRepoDir, 'packages/server/src/core/diagnostics'), { recursive: true });
      writeFileSync(join(monoRepoDir, 'packages/server/src/core/diagnostics/logger.js'),
        'export const logger = { info: console.log };');

      mkdirSync(join(monoRepoDir, 'packages/shared'), { recursive: true });
      writeFileSync(join(monoRepoDir, 'packages/shared/config.js'),
        'export const config = {};');

      console.log('📁 Moving from: packages/server/src/lsp/');
      console.log('📁 Moving to:   packages/@scope/features/lsp/src/\n');

      // Perform the move
      const result = await client.callTool('rename_file', {
        old_path: serverFile,
        new_path: scopedFile,
        dry_run: false,
      });

      expect(result).toBeDefined();
      await waitForCondition(() => existsSync(scopedFile) && !existsSync(serverFile), { timeout: 500, interval: 100 });

      // Verify file was moved
      expect(existsSync(serverFile)).toBe(false);
      expect(existsSync(scopedFile)).toBe(true);

      const movedContent = readFileSync(scopedFile, 'utf-8');

      // The correct paths after move should be:
      // From packages/@scope/features/lsp/src/client.ts to:
      // - packages/server/src/core/file-operations/path-utils.js = ../../../../server/src/core/file-operations/path-utils.js
      // - packages/server/src/core/diagnostics/logger.js = ../../../../server/src/core/diagnostics/logger.js
      // - packages/shared/config.js = ../../../../shared/config.js

      const correctPathUtils = movedContent.includes('../../../../server/src/core/file-operations/path-utils.js');
      const correctLogger = movedContent.includes('../../../../server/src/core/diagnostics/logger.js');
      const correctConfig = movedContent.includes('../../../../shared/config.js');

      console.log('Import updates:');
      console.log(`  Path utils: ${correctPathUtils ? '✅' : '❌'} (should be ../../../../server/src/core/file-operations/path-utils.js)`);
      console.log(`  Logger:     ${correctLogger ? '✅' : '❌'} (should be ../../../../server/src/core/diagnostics/logger.js)`);
      console.log(`  Config:     ${correctConfig ? '✅' : '❌'} (should be ../../../../shared/config.js)`);

      expect(correctPathUtils).toBe(true);
      expect(correctLogger).toBe(true);
      expect(correctConfig).toBe(true);

      // Cleanup
      rmSync(monoRepoDir, { recursive: true, force: true });

      console.log('\n✅ Lateral monorepo move handled correctly!');
    }, 30000);
  });

  describe('New Import Fix Tools', () => {
    it('should fix imports using the fix_imports tool', async () => {
      console.log('🔧 Testing fix_imports tool...\n');

      const brokenFile = join(testDir, 'broken.ts');
      mkdirSync(dirname(brokenFile), { recursive: true });

      // Create a file with broken imports (as if it was moved without updating imports)
      writeFileSync(brokenFile, `
import { helper } from '../utils/helper.js';  // This path is wrong after move
import { User } from '../models/user.js';      // This too

export function test() {
  return helper.process(new User('123'));
}
`);

      // Use fix_imports to repair the paths (assuming file was moved from src/ to src/features/)
      const result = await client.callTool('fix_imports', {
        file_path: brokenFile,
        old_path: join(testDir, 'src/broken.ts'),  // Where it used to be
      });

      expect(result).toBeDefined();
      const response = result.content?.[0]?.text || '';

      console.log('Fix imports result:');
      console.log(response);

      expect(response).toContain('Fixed');

      // Verify the imports were fixed
      const fixedContent = readFileSync(brokenFile, 'utf-8');
      expect(fixedContent).toContain('./utils/helper.js');  // Should now be relative to testDir
      expect(fixedContent).toContain('./models/user.js');

      console.log('✅ fix_imports tool working correctly');
    });

    it('should analyze import relationships with analyze_imports', async () => {
      console.log('🔧 Testing analyze_imports tool...\n');

      const result = await client.callTool('analyze_imports', {
        file_path: join(testDir, 'src/utils/helper.js'),
      });

      expect(result).toBeDefined();
      const response = result.content?.[0]?.text || '';

      console.log('Analyze imports result:');
      console.log(response);

      // Should show what imports this file (if any)
      expect(response).toMatch(/import|no files import/i);

      console.log('✅ analyze_imports tool working correctly');
    });

    it('should rename directory with all files using rename_directory', async () => {
      console.log('🔧 Testing rename_directory tool...\n');

      // First do a dry run
      const dryResult = await client.callTool('rename_directory', {
        old_path: join(testDir, 'src/utils'),
        new_path: join(testDir, 'src/utilities'),
        dry_run: true,
      });

      expect(dryResult).toBeDefined();
      const dryResponse = dryResult.content?.[0]?.text || '';

      console.log('Dry run result:');
      console.log(dryResponse);

      expect(dryResponse).toContain('DRY RUN');
      expect(dryResponse).toMatch(/would rename/i);

      // Verify no actual changes occurred
      expect(existsSync(join(testDir, 'src/utils'))).toBe(true);
      expect(existsSync(join(testDir, 'src/utilities'))).toBe(false);

      console.log('✅ rename_directory dry run working correctly');
    });
  });
});
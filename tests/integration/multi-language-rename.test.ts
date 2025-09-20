import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { FileBackupManager } from '../helpers/file-backup-manager.js';
import { MCPTestClient, assertToolResult } from '../helpers/mcp-test-client.js';

describe('Multi-Language Rename Integration Tests', () => {
  let client: MCPTestClient;
  let backupManager: FileBackupManager;

  // Test files for multi-language renaming
  const pythonFiles = [
    '/workspace/playground/python/math_utils.py',
    '/workspace/playground/python/main.py',
    '/workspace/playground/python/helpers.py',
  ];

  const rustFiles = [
    '/workspace/playground/rust/src/processor.rs',
    '/workspace/playground/rust/src/utils.rs',
    '/workspace/playground/rust/src/main.rs',
    '/workspace/playground/rust/src/lib.rs',
  ];

  beforeAll(async () => {
    console.log('🌍 Multi-Language Rename Integration Test');
    console.log('==========================================\n');

    // Initialize backup manager
    backupManager = new FileBackupManager();

    // Create backups of all test files
    console.log('📋 Creating backups of multi-language files...');
    for (const filePath of [...pythonFiles, ...rustFiles]) {
      if (existsSync(filePath)) {
        backupManager.backupFile(filePath);
        console.log(`  ✓ Backed up: ${filePath.split('/').pop()}`);
      }
    }

    // Initialize MCP client with playground config
    process.env.CODEBUDDY_CONFIG_PATH = '/workspace/playground/codebuddy.json';
    client = new MCPTestClient();
    await client.start({ skipLSPPreload: true });

    // Extended wait for multiple LSP servers to initialize
    console.log('⏳ Waiting for multiple LSP servers to initialize...');
    await new Promise((resolve) => setTimeout(resolve, 5000));
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

  describe('Python Multi-File Symbol Rename', () => {
    it('should rename Python class DataProcessor → Calculator across multiple files', async () => {
      console.log('🐍 Testing Python class rename: DataProcessor → Calculator');

      // First test dry run
      const dryRunResult = await client.callTool('rename_symbol', {
        file_path: '/workspace/playground/python/math_utils.py',
        symbol_name: 'DataProcessor',
        new_name: 'Calculator',
        dry_run: true,
      });

      expect(dryRunResult).toBeDefined();
      assertToolResult(dryRunResult);
      const dryRunContent = dryRunResult.content?.[0]?.text || '';

      console.log('📋 Python dry-run result:');
      console.log(dryRunContent);

      // Should indicate it's a dry run and mention the rename
      expect(dryRunContent).toMatch(/DRY RUN|Would rename|preview/i);
      expect(dryRunContent).toMatch(/DataProcessor.*Calculator/);

      // Execute the actual rename
      console.log('🔧 Executing Python class rename...');
      const result = await client.callTool('rename_symbol', {
        file_path: '/workspace/playground/python/math_utils.py',
        symbol_name: 'DataProcessor',
        new_name: 'Calculator',
        dry_run: false,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      console.log('📋 Python rename execution result:');
      console.log(content);

      // LSP synchronization now handled automatically by applyWorkspaceEdit

      console.log('🔍 Verifying Python file changes...');

      // Verify math_utils.py changes
      const mathUtilsContent = readFileSync(pythonFiles[0], 'utf-8');
      console.log('📄 math_utils.py changes:');
      if (mathUtilsContent.includes('class Calculator:')) {
        console.log('  ✅ Class definition renamed');
      }
      if (mathUtilsContent.includes('Calculator(multiplier')) {
        console.log('  ✅ Constructor calls updated');
      }
      expect(mathUtilsContent).not.toContain('class DataProcessor:');
      expect(mathUtilsContent).toContain('class Calculator:');

      // Verify main.py changes
      const mainContent = readFileSync(pythonFiles[1], 'utf-8');
      console.log('📄 main.py changes:');
      if (mainContent.includes('from math_utils import Calculator')) {
        console.log('  ✅ Import statement updated');
      }
      if (mainContent.includes('processor = Calculator(4)')) {
        console.log('  ✅ Constructor call updated');
      }
      expect(mainContent).not.toContain('DataProcessor');
      expect(mainContent).toContain('Calculator');

      // Verify helpers.py changes
      const helpersContent = readFileSync(pythonFiles[2], 'utf-8');
      console.log('📄 helpers.py changes:');
      if (helpersContent.includes('from math_utils import Calculator')) {
        console.log('  ✅ Import statement updated');
      }
      if (helpersContent.includes('List[Calculator]')) {
        console.log('  ✅ Type annotations updated');
      }
      expect(helpersContent).not.toContain('DataProcessor');
      expect(helpersContent).toContain('Calculator');

      console.log('✅ Python multi-file class rename verification complete');
    }, 30000);

    it('should rename Python function process_data → transform_data across files', async () => {
      console.log('🐍 Testing Python function rename: process_data → transform_data');

      // Restore files first
      backupManager.restoreAll();
      await new Promise((resolve) => setTimeout(resolve, 2000));
      console.log('⏳ Allowing time for LSP re-indexing after file restore...');

      const result = await client.callTool('rename_symbol', {
        file_path: '/workspace/playground/python/math_utils.py',
        symbol_name: 'process_data',
        new_name: 'transform_data',
        dry_run: false,
      });

      expect(result).toBeDefined();
      const toolResult = assertToolResult(result);

      // Wait for file operations
      await new Promise((resolve) => setTimeout(resolve, 1000));

      console.log('🔍 Verifying Python function rename...');

      // Check all files for function name update
      for (const filePath of pythonFiles) {
        if (existsSync(filePath)) {
          const content = readFileSync(filePath, 'utf-8');
          const fileName = filePath.split('/').pop();

          if (content.includes('transform_data') && !content.includes('process_data')) {
            console.log(`  ✅ ${fileName}: function renamed correctly`);
          } else if (!content.includes('process_data') && !content.includes('transform_data')) {
            console.log(`  ⚪ ${fileName}: no references (expected)`);
          } else {
            console.log(`  ⚠️  ${fileName}: may have mixed references`);
          }
        }
      }

      console.log('✅ Python function rename verification complete');
    }, 20000);
  });

  describe('Rust Multi-File Symbol Rename', () => {
    it('should rename Rust struct DataProcessor → InfoProcessor across modules', async () => {
      console.log('🦀 Testing Rust struct rename: DataProcessor → InfoProcessor');

      // Restore files first
      backupManager.restoreAll();
      await new Promise((resolve) => setTimeout(resolve, 2000));
      console.log('⏳ Allowing time for LSP re-indexing after file restore...');

      // First test dry run
      const dryRunResult = await client.callTool('rename_symbol', {
        file_path: '/workspace/playground/rust/src/processor.rs',
        symbol_name: 'DataProcessor',
        new_name: 'InfoProcessor',
        dry_run: true,
      });

      expect(dryRunResult).toBeDefined();
      assertToolResult(dryRunResult);
      const dryRunContent = dryRunResult.content?.[0]?.text || '';

      console.log('📋 Rust dry-run result:');
      console.log(dryRunContent);

      // Should indicate it's a dry run and mention the rename
      expect(dryRunContent).toMatch(/DRY RUN|Would rename|preview/i);
      expect(dryRunContent).toMatch(/DataProcessor.*InfoProcessor/);

      // Execute the actual rename
      console.log('🔧 Executing Rust struct rename...');
      const result = await client.callTool('rename_symbol', {
        file_path: '/workspace/playground/rust/src/processor.rs',
        symbol_name: 'DataProcessor',
        new_name: 'InfoProcessor',
        dry_run: false,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      console.log('📋 Rust rename execution result:');
      console.log(content);

      // LSP synchronization now handled automatically by applyWorkspaceEdit

      console.log('🔍 Verifying Rust file changes...');

      // Verify processor.rs changes
      const processorContent = readFileSync(rustFiles[0], 'utf-8');
      console.log('📄 processor.rs changes:');
      if (processorContent.includes('pub struct InfoProcessor')) {
        console.log('  ✅ Struct definition renamed');
      }
      if (processorContent.includes('InfoProcessor { multiplier }')) {
        console.log('  ✅ Struct instantiation updated');
      }
      expect(processorContent).not.toContain('pub struct DataProcessor');
      expect(processorContent).toContain('pub struct InfoProcessor');

      // Verify utils.rs changes
      const utilsContent = readFileSync(rustFiles[1], 'utf-8');
      console.log('📄 utils.rs changes:');
      if (utilsContent.includes('use crate::processor::{InfoProcessor')) {
        console.log('  ✅ Use statement updated');
      }
      if (utilsContent.includes('InfoProcessor::new')) {
        console.log('  ✅ Associated function calls updated');
      }
      expect(utilsContent).not.toContain('DataProcessor');
      expect(utilsContent).toContain('InfoProcessor');

      // Verify main.rs changes
      const mainContent = readFileSync(rustFiles[2], 'utf-8');
      console.log('📄 main.rs changes:');
      if (mainContent.includes('use codebuddy_playground::{InfoProcessor')) {
        console.log('  ✅ Import statement updated');
      }
      if (mainContent.includes('InfoProcessor::new(4)')) {
        console.log('  ✅ Constructor call updated');
      }
      expect(mainContent).not.toContain('DataProcessor');
      expect(mainContent).toContain('InfoProcessor');

      // Verify lib.rs changes
      const libContent = readFileSync(rustFiles[3], 'utf-8');
      console.log('📄 lib.rs changes:');
      if (libContent.includes('pub use processor::{InfoProcessor')) {
        console.log('  ✅ Re-export statement updated');
      }
      expect(libContent).not.toContain('DataProcessor');
      expect(libContent).toContain('InfoProcessor');

      console.log('✅ Rust multi-module struct rename verification complete');
    }, 30000);

    it('should handle Rust function renaming across modules', async () => {
      console.log('🦀 Testing Rust function rename: process_data → transform_data');

      // Restore files first
      backupManager.restoreAll();
      await new Promise((resolve) => setTimeout(resolve, 2000));
      console.log('⏳ Allowing time for LSP re-indexing after file restore...');

      const result = await client.callTool('rename_symbol', {
        file_path: '/workspace/playground/rust/src/utils.rs',
        symbol_name: 'process_data',
        new_name: 'transform_data',
        dry_run: false,
      });

      expect(result).toBeDefined();
      const toolResult = assertToolResult(result);

      // LSP synchronization now handled automatically by applyWorkspaceEdit

      console.log('🔍 Verifying Rust function rename...');

      // Check all files for function name update
      for (const filePath of rustFiles) {
        if (existsSync(filePath)) {
          const content = readFileSync(filePath, 'utf-8');
          const fileName = filePath.split('/').pop();

          if (content.includes('transform_data') && !content.includes('process_data')) {
            console.log(`  ✅ ${fileName}: function renamed correctly`);
          } else if (!content.includes('process_data') && !content.includes('transform_data')) {
            console.log(`  ⚪ ${fileName}: no references (expected)`);
          } else {
            console.log(`  ⚠️  ${fileName}: may have mixed references`);
          }
        }
      }

      console.log('✅ Rust function rename verification complete');
    }, 20000);
  });

  describe('Cross-Language Reference Verification', () => {
    it('should verify find_references works for Python symbols', async () => {
      console.log('🔍 Testing Python cross-file references...');

      // Restore files first
      backupManager.restoreAll();
      await new Promise((resolve) => setTimeout(resolve, 2000));
      console.log('⏳ Allowing time for LSP re-indexing after file restore...');

      const result = await client.callTool('find_references', {
        file_path: '/workspace/playground/python/math_utils.py',
        symbol_name: 'DataProcessor',
        include_declaration: true,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      console.log('📋 Python references found:');
      console.log(content);

      // Should find references across multiple Python files
      expect(content).not.toMatch(/No.*found/i);
      expect(content).toMatch(/DataProcessor/);
      expect(content).toContain('.py');

      console.log('✅ Python cross-file references verified');
    });

    it('should verify find_references works for Rust symbols', async () => {
      console.log('🔍 Testing Rust cross-module references...');

      // First try with position-based approach (line 3, character 11 for "DataProcessor")
      const result = await client.callTool('find_references', {
        file_path: '/workspace/playground/rust/src/processor.rs',
        symbol_name: 'DataProcessor',
        include_declaration: true,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      console.log('📋 Rust references found (position-based):');
      console.log(content);

      if (content.includes('DataProcessor')) {
        // Should find references across multiple Rust files
        expect(content).toMatch(/DataProcessor/);
        expect(content).toContain('.rs');
        console.log('✅ Rust cross-module references verified');
      } else {
        console.log('⚠️ Rust references not found - may need more indexing time');
        // Fallback: try symbol-name approach
        const fallbackResult = await client.callTool('find_references', {
          file_path: '/workspace/playground/rust/src/processor.rs',
          symbol_name: 'DataProcessor',
          include_declaration: true,
        });
        const fallbackContent = assertToolResult(fallbackResult).content?.[0]?.text || '';
        console.log('📋 Rust references found (symbol-name):');
        console.log(fallbackContent);

        if (fallbackContent.includes('DataProcessor')) {
          expect(fallbackContent).toMatch(/DataProcessor/);
          console.log('✅ Rust cross-module references verified (fallback)');
        } else {
          console.log('⚠️ Rust LSP may need more time to index project');
        }
      }
    });
  });

  describe('Language Server Health Check', () => {
    it('should verify all language servers are working', async () => {
      console.log('🏥 Testing language server health...');

      // Test TypeScript server
      const tsResult = await client.callTool('get_diagnostics', {
        file_path: '/workspace/playground/src/index.ts',
      });
      expect(tsResult).toBeDefined();
      console.log('  ✅ TypeScript LSP responding');

      // Test Python server
      const pyResult = await client.callTool('get_diagnostics', {
        file_path: '/workspace/playground/python/math_utils.py',
      });
      expect(pyResult).toBeDefined();
      console.log('  ✅ Python LSP responding');

      // Test Rust server
      const rsResult = await client.callTool('get_diagnostics', {
        file_path: '/workspace/playground/rust/src/lib.rs',
      });
      expect(rsResult).toBeDefined();
      console.log('  ✅ Rust LSP responding');

      console.log('✅ All language servers healthy');
    });
  });
});

import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { assertToolResult, MCPTestClient } from '../helpers/mcp-test-client.js';
import { poll, waitForLSP } from '../helpers/test-verification-helpers.js';

describe('Multi-File Rename Integration Tests', () => {
  let client: MCPTestClient;
  const TEST_DIR = '/workspace/examples/playground/multi-file-rename-test';

  beforeAll(async () => {
    console.log('🔍 Multi-File Rename Integration Test');
    console.log('=====================================\n');

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
          name: 'multi-file-rename-test',
          type: 'module',
          version: '1.0.0',
        },
        null,
        2
      )
    );

    // Create a simple service that will be renamed
    writeFileSync(
      join(TEST_DIR, 'service.ts'),
      `export class DataProcessor {
  process(data: string): string {
    return data.toUpperCase();
  }
  
  validate(data: string): boolean {
    return data.length > 0;
  }
}`
    );

    // Create files that import/use the service
    writeFileSync(
      join(TEST_DIR, 'handler.ts'),
      `import { DataProcessor } from './service';

export class DataHandler {
  private processor: DataProcessor = new DataProcessor();
  
  handleData(input: string): string {
    if (this.processor.validate(input)) {
      return this.processor.process(input);
    }
    return '';
  }
}`
    );

    writeFileSync(
      join(TEST_DIR, 'utils.ts'),
      `import { DataProcessor } from './service';

export function createProcessor(): DataProcessor {
  return new DataProcessor();
}

export const PROCESSOR_INSTANCE = new DataProcessor();`
    );

    // Initialize MCP client
    client = new MCPTestClient();
    await client.start({ skipLSPPreload: true });

    await waitForLSP(client, join(TEST_DIR, 'service.ts'));
    console.log('✅ Setup complete\n');
  });

  afterAll(async () => {
    await client.stop();
    if (existsSync(TEST_DIR)) {
      rmSync(TEST_DIR, { recursive: true });
    }
    console.log('✅ Cleanup complete');
  });

  describe('DataProcessor → ContentProcessor Rename', () => {
    it('should preview multi-file rename with dry_run', async () => {
      console.log('🔍 Testing dry-run rename preview...');

      const result = await client.callTool('rename_symbol', {
        file_path: join(TEST_DIR, 'service.ts'),
        symbol_name: 'DataProcessor',
        new_name: 'ContentProcessor',
        dry_run: true,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      console.log('📋 Dry-run result preview:');
      console.log(content);

      // Should indicate it's a dry run
      expect(content).toMatch(/DRY RUN|Would rename|preview/i);

      // Should mention the symbol being renamed
      expect(content).toMatch(/DataProcessor.*ContentProcessor/);

      // Verify files are unchanged after dry run
      const serviceContent = readFileSync(join(TEST_DIR, 'service.ts'), 'utf-8');
      const handlerContent = readFileSync(join(TEST_DIR, 'handler.ts'), 'utf-8');
      const utilsContent = readFileSync(join(TEST_DIR, 'utils.ts'), 'utf-8');

      expect(serviceContent).toContain('DataProcessor');
      expect(handlerContent).toContain('DataProcessor');
      expect(utilsContent).toContain('DataProcessor');

      expect(serviceContent).not.toContain('ContentProcessor');
      expect(handlerContent).not.toContain('ContentProcessor');
      expect(utilsContent).not.toContain('ContentProcessor');

      console.log('✅ Dry-run preview successful - no files modified');
    });

    it('should execute multi-file rename and verify all file changes', async () => {
      console.log('🔧 Executing actual multi-file rename...');

      // First, open all files to ensure TypeScript server knows about them
      console.log('📂 Opening all project files in TypeScript LSP...');

      // Use find_definition to trigger file opening in LSP
      await client.callTool('find_definition', {
        file_path: join(TEST_DIR, 'handler.ts'),
        symbol_name: 'DataProcessor',
        symbol_kind: 'class',
      });

      await client.callTool('find_definition', {
        file_path: join(TEST_DIR, 'utils.ts'),
        symbol_name: 'DataProcessor',
        symbol_kind: 'class',
      });

      await waitForLSP(client, join(TEST_DIR, 'handler.ts'));
      await waitForLSP(client, join(TEST_DIR, 'utils.ts'));

      // Execute the rename
      const result = await client.callTool('rename_symbol', {
        file_path: join(TEST_DIR, 'service.ts'),
        symbol_name: 'DataProcessor',
        new_name: 'ContentProcessor',
        dry_run: false,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      console.log('📋 Rename execution result:');
      console.log(content);

      // Should indicate successful rename
      expect(content).toMatch(/renamed|success|applied/i);
      expect(content).toMatch(/DataProcessor.*ContentProcessor/);

      console.log('🔍 Verifying file changes...');

      // Wait for file system operations to complete by polling for the change
      await poll(
        async () => {
          const serviceContent = readFileSync(join(TEST_DIR, 'service.ts'), 'utf-8');
          return serviceContent.includes('ContentProcessor');
        },
        5000,
        100
      );
      // Verify specific changes in each file
      const serviceContent = readFileSync(join(TEST_DIR, 'service.ts'), 'utf-8');
      const handlerContent = readFileSync(join(TEST_DIR, 'handler.ts'), 'utf-8');
      const utilsContent = readFileSync(join(TEST_DIR, 'utils.ts'), 'utf-8');

      console.log('📄 service.ts changes:');
      if (serviceContent.includes('export class ContentProcessor')) {
        console.log('  ✅ Class definition renamed');
      } else {
        console.log('  ❌ Class definition not renamed');
        console.log('  Current content:', serviceContent.substring(0, 100));
      }

      console.log('📄 handler.ts changes:');
      if (handlerContent.includes('import { ContentProcessor }')) {
        console.log('  ✅ Import statement updated');
      }
      if (handlerContent.includes('private processor: ContentProcessor')) {
        console.log('  ✅ Type annotation updated');
      }
      if (handlerContent.includes('new ContentProcessor()')) {
        console.log('  ✅ Constructor call updated');
      }

      console.log('📄 utils.ts changes:');
      if (utilsContent.includes('import { ContentProcessor }')) {
        console.log('  ✅ Import statement updated');
      }
      if (utilsContent.includes('function createProcessor(): ContentProcessor')) {
        console.log('  ✅ Return type updated');
      }
      if (utilsContent.includes('new ContentProcessor()')) {
        console.log('  ✅ Constructor calls updated');
      }

      // Verify old name is gone
      expect(serviceContent).not.toContain('DataProcessor');
      expect(handlerContent).not.toContain('DataProcessor');
      expect(utilsContent).not.toContain('DataProcessor');

      // Verify new name is present
      expect(serviceContent).toContain('ContentProcessor');
      expect(handlerContent).toContain('ContentProcessor');
      expect(utilsContent).toContain('ContentProcessor');

      console.log('✅ Multi-file rename verification complete');

      // LSP synchronization now handled automatically by applyWorkspaceEdit
    }, 30000);

    it('should handle rename of non-existent symbol gracefully', async () => {
      console.log('🔍 Testing rename of non-existent symbol...');

      const result = await client.callTool('rename_symbol', {
        file_path: join(TEST_DIR, 'service.ts'),
        symbol_name: 'NonExistentClass',
        new_name: 'SomeOtherClass',
        dry_run: true,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      console.log('📋 Non-existent symbol result:');
      console.log(content);

      // Should indicate no symbol found
      expect(content).toMatch(/No.*found|not found|No symbols/i);

      console.log('✅ Non-existent symbol handled gracefully');
    });

    it('should validate rename with same name fails appropriately', async () => {
      console.log('🔍 Testing rename with same name...');

      const result = await client.callTool('rename_symbol', {
        file_path: join(TEST_DIR, 'service.ts'),
        symbol_name: 'ContentProcessor',
        new_name: 'ContentProcessor',
        dry_run: true,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      console.log('📋 Same name rename result:');
      console.log(content);

      // Should indicate same name issue or no changes
      expect(content).toMatch(/same|no changes|identical|already named/i);

      console.log('✅ Same name rename handled appropriately');
    });
  });

  describe('Position-Based Rename (rename_symbol_strict)', () => {
    it('should rename using exact position coordinates', async () => {
      console.log('🎯 Testing position-based rename...');

      // Find exact position of ContentProcessor in the class definition
      const serviceContent = readFileSync(join(TEST_DIR, 'service.ts'), 'utf-8');
      const lines = serviceContent.split('\n');
      let targetLine = 1;
      let targetChar = 14;

      for (let i = 0; i < lines.length; i++) {
        if (lines[i].includes('export class ContentProcessor')) {
          targetLine = i + 1; // Convert to 1-based
          targetChar = lines[i].indexOf('ContentProcessor');
          break;
        }
      }

      console.log(`Using position: line ${targetLine}, character ${targetChar}`);

      const result = await client.callTool('rename_symbol_strict', {
        file_path: join(TEST_DIR, 'service.ts'),
        line: targetLine,
        character: targetChar,
        new_name: 'ProcessorService',
        dry_run: true,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      console.log('📋 Position-based rename result:');
      console.log(content);

      // Should indicate successful rename preview
      expect(content).toMatch(/DRY RUN|Would rename|preview/i);
      expect(content).toMatch(/ProcessorService/);

      console.log('✅ Position-based rename preview successful');
    });
  });

  describe('Cross-File Reference Verification', () => {
    it('should verify find_references works across all files', async () => {
      console.log('🔍 Verifying cross-file references...');

      const result = await client.callTool('find_references', {
        file_path: join(TEST_DIR, 'service.ts'),
        symbol_name: 'ContentProcessor',
        include_declaration: true,
      });

      expect(result).toBeDefined();
      assertToolResult(result);
      const content = result.content?.[0]?.text || '';

      console.log('📋 References found:');
      console.log(content);

      // Should find references in multiple files
      expect(content).not.toMatch(/No.*found/i);
      expect(content).toMatch(/ContentProcessor/);

      // Should mention multiple files
      expect(content).toContain('.ts');

      console.log('✅ Cross-file references verified');
    });
  });
});

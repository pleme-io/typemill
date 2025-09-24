import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { assertToolResult, MCPTestClient } from '../helpers/mcp-test-client.js';

describe('Atomic Refactoring Integration Tests', () => {
  let client: MCPTestClient;
  const TEST_DIR = join(tmpdir(), 'atomic-refactoring-test');

  beforeAll(async () => {
    console.log('🔍 Atomic Refactoring Integration Test');
    console.log('======================================\n');

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
          name: 'atomic-refactoring-test',
          type: 'module',
          version: '1.0.0',
        },
        null,
        2
      )
    );

    // Create source structure that we'll be moving
    mkdirSync(join(TEST_DIR, 'src'), { recursive: true });
    mkdirSync(join(TEST_DIR, 'lib'), { recursive: true });

    // Create a utility service in src/
    writeFileSync(
      join(TEST_DIR, 'src', 'utils.ts'),
      `export class StringUtils {
  static capitalize(text: string): string {
    return text.charAt(0).toUpperCase() + text.slice(1);
  }

  static reverse(text: string): string {
    return text.split('').reverse().join('');
  }
}`
    );

    // Create a data service in src/
    writeFileSync(
      join(TEST_DIR, 'src', 'data-service.ts'),
      `import { StringUtils } from './utils';

export class DataService {
  formatData(data: string): string {
    return StringUtils.capitalize(StringUtils.reverse(data));
  }

  processItems(items: string[]): string[] {
    return items.map(item => this.formatData(item));
  }
}`
    );

    // Create main file that imports both services
    writeFileSync(
      join(TEST_DIR, 'main.ts'),
      `import { DataService } from './src/data-service';
import { StringUtils } from './src/utils';

const service = new DataService();
const processed = service.processItems(['hello', 'world']);
const manual = StringUtils.capitalize('test');

console.log('Processed:', processed);
console.log('Manual:', manual);
`
    );

    // Create another consumer in lib/
    writeFileSync(
      join(TEST_DIR, 'lib', 'consumer.ts'),
      `import { StringUtils } from '../src/utils';
import { DataService } from '../src/data-service';

export class Consumer {
  private dataService = new DataService();

  consume(input: string): string {
    const reversed = StringUtils.reverse(input);
    return this.dataService.formatData(reversed);
  }
}`
    );

    // Initialize MCP client
    client = new MCPTestClient();
    await client.start({ skipLSPPreload: true });

    // Allow extra time for TypeScript LSP to index the new project
    console.log('⏳ Waiting for TypeScript LSP to index project files...');
    await new Promise((resolve) => setTimeout(resolve, 3000));
    console.log('✅ Setup complete\n');
  });

  afterAll(async () => {
    await client.stop();
    if (existsSync(TEST_DIR)) {
      rmSync(TEST_DIR, { recursive: true });
    }
    console.log('✅ Cleanup complete');
  });

  describe('Impact Analysis', () => {
    it('should analyze impact of moving files from src/ to lib/', async () => {
      console.log('🔍 Testing impact analysis for file moves...');

      const result = await client.callTool('analyze_refactor_impact', {
        operations: [
          {
            type: 'move_file',
            old_path: join(TEST_DIR, 'src', 'utils.ts'),
            new_path: join(TEST_DIR, 'lib', 'utils.ts'),
          },
          {
            type: 'move_file',
            old_path: join(TEST_DIR, 'src', 'data-service.ts'),
            new_path: join(TEST_DIR, 'lib', 'data-service.ts'),
          },
        ],
        include_recommendations: true,
      });

      assertToolResult(result);
      const response = result.content[0]?.text || '';

      // Verify impact analysis content
      expect(response).toContain('Refactoring Impact Analysis');
      expect(response).toContain('**Operations**: 2');
      expect(response).toContain('Estimated file changes');
      expect(response).toContain('Risk assessment');

      // Should detect dependent files
      expect(response).toContain('main.ts');
      expect(response).toContain('consumer.ts');

      console.log('✅ Impact analysis complete');
    });
  });

  describe('Batch Move Files - Success Case', () => {
    it('should successfully move files and update all imports atomically', async () => {
      console.log('🔍 Testing successful atomic batch move...');

      // Verify initial state
      expect(existsSync(join(TEST_DIR, 'src', 'utils.ts'))).toBe(true);
      expect(existsSync(join(TEST_DIR, 'src', 'data-service.ts'))).toBe(true);
      expect(existsSync(join(TEST_DIR, 'lib', 'utils.ts'))).toBe(false);
      expect(existsSync(join(TEST_DIR, 'lib', 'data-service.ts'))).toBe(false);

      // Read original content to verify it doesn't change
      const _originalMainContent = readFileSync(join(TEST_DIR, 'main.ts'), 'utf-8');
      const _originalConsumerContent = readFileSync(join(TEST_DIR, 'lib', 'consumer.ts'), 'utf-8');

      // Execute atomic move
      const result = await client.callTool('batch_move_files', {
        moves: [
          {
            old_path: join(TEST_DIR, 'src', 'utils.ts'),
            new_path: join(TEST_DIR, 'lib', 'utils.ts'),
          },
          {
            old_path: join(TEST_DIR, 'src', 'data-service.ts'),
            new_path: join(TEST_DIR, 'lib', 'data-service.ts'),
          },
        ],
        dry_run: false,
        strategy: 'safe',
      });

      assertToolResult(result);
      const response = result.content[0]?.text || '';

      // Verify success message
      expect(response).toContain('Batch Move Results');
      expect(response).toContain('All operations completed successfully');
      expect(response).toContain('**Successful moves**: 2');

      // Verify files were moved
      expect(existsSync(join(TEST_DIR, 'src', 'utils.ts'))).toBe(false);
      expect(existsSync(join(TEST_DIR, 'src', 'data-service.ts'))).toBe(false);
      expect(existsSync(join(TEST_DIR, 'lib', 'utils.ts'))).toBe(true);
      expect(existsSync(join(TEST_DIR, 'lib', 'data-service.ts'))).toBe(true);

      // Verify imports were updated correctly
      const updatedMainContent = readFileSync(join(TEST_DIR, 'main.ts'), 'utf-8');
      const updatedConsumerContent = readFileSync(join(TEST_DIR, 'lib', 'consumer.ts'), 'utf-8');

      // Main.ts should have updated imports
      expect(updatedMainContent).toContain("from './lib/data-service'");
      expect(updatedMainContent).toContain("from './lib/utils'");
      expect(updatedMainContent).not.toContain("from './src/data-service'");
      expect(updatedMainContent).not.toContain("from './src/utils'");

      // Consumer.ts should have updated relative imports
      expect(updatedConsumerContent).toContain("from './utils'");
      expect(updatedConsumerContent).toContain("from './data-service'");
      expect(updatedConsumerContent).not.toContain("from '../src/utils'");
      expect(updatedConsumerContent).not.toContain("from '../src/data-service'");

      // Verify file content integrity (code itself should be unchanged)
      const movedUtilsContent = readFileSync(join(TEST_DIR, 'lib', 'utils.ts'), 'utf-8');
      const movedDataServiceContent = readFileSync(
        join(TEST_DIR, 'lib', 'data-service.ts'),
        'utf-8'
      );

      expect(movedUtilsContent).toContain('export class StringUtils');
      expect(movedUtilsContent).toContain('capitalize(text: string)');
      expect(movedDataServiceContent).toContain('export class DataService');
      expect(movedDataServiceContent).toContain("from './utils'"); // Internal import updated

      console.log('✅ Atomic batch move successful');
    });
  });

  describe('Batch Move Files - Preview Case', () => {
    it('should preview changes without modifying any files', async () => {
      console.log('🔍 Testing dry-run preview...');

      // Reset test state by moving files back to src/
      mkdirSync(join(TEST_DIR, 'src'), { recursive: true });

      // Move files back manually for this test
      if (existsSync(join(TEST_DIR, 'lib', 'utils.ts'))) {
        const utilsContent = readFileSync(join(TEST_DIR, 'lib', 'utils.ts'), 'utf-8');
        writeFileSync(join(TEST_DIR, 'src', 'utils.ts'), utilsContent);
        rmSync(join(TEST_DIR, 'lib', 'utils.ts'));
      }

      if (existsSync(join(TEST_DIR, 'lib', 'data-service.ts'))) {
        const dataServiceContent = readFileSync(join(TEST_DIR, 'lib', 'data-service.ts'), 'utf-8');
        // Fix the internal import back to relative
        const fixedContent = dataServiceContent.replace("from './utils'", "from './utils'");
        writeFileSync(join(TEST_DIR, 'src', 'data-service.ts'), fixedContent);
        rmSync(join(TEST_DIR, 'lib', 'data-service.ts'));
      }

      // Wait for LSP to re-index
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Get initial file states
      const initialUtilsExists = existsSync(join(TEST_DIR, 'src', 'utils.ts'));
      const initialDataServiceExists = existsSync(join(TEST_DIR, 'src', 'data-service.ts'));
      const initialMainContent = readFileSync(join(TEST_DIR, 'main.ts'), 'utf-8');

      // Execute dry run
      const result = await client.callTool('batch_move_files', {
        moves: [
          {
            old_path: join(TEST_DIR, 'src', 'utils.ts'),
            new_path: join(TEST_DIR, 'lib', 'utils.ts'),
          },
          {
            old_path: join(TEST_DIR, 'src', 'data-service.ts'),
            new_path: join(TEST_DIR, 'lib', 'data-service.ts'),
          },
        ],
        dry_run: true,
        strategy: 'safe',
      });

      assertToolResult(result);
      const response = result.content[0]?.text || '';

      // Verify preview content
      expect(response).toContain('Batch Move Preview (DRY RUN)');
      expect(response).toContain('**Operations**: 2');
      expect(response).toContain('Import updates');
      expect(response).toContain('File Operations');
      expect(response).toContain('Ready to move');

      // Verify NO files were actually moved
      expect(existsSync(join(TEST_DIR, 'src', 'utils.ts'))).toBe(initialUtilsExists);
      expect(existsSync(join(TEST_DIR, 'src', 'data-service.ts'))).toBe(initialDataServiceExists);
      expect(existsSync(join(TEST_DIR, 'lib', 'utils.ts'))).toBe(false);
      expect(existsSync(join(TEST_DIR, 'lib', 'data-service.ts'))).toBe(false);

      // Verify NO imports were modified
      const finalMainContent = readFileSync(join(TEST_DIR, 'main.ts'), 'utf-8');
      expect(finalMainContent).toBe(initialMainContent);

      console.log('✅ Dry-run preview completed without file modifications');
    });
  });

  describe('Batch Move Files - Rollback Case', () => {
    it('should rollback all changes when a move fails', async () => {
      console.log('🔍 Testing atomic rollback on failure...');

      // Ensure we have the source files
      const srcUtilsPath = join(TEST_DIR, 'src', 'utils.ts');
      const srcDataServicePath = join(TEST_DIR, 'src', 'data-service.ts');
      const targetUtilsPath = join(TEST_DIR, 'lib', 'utils.ts');
      const targetDataServicePath = join(TEST_DIR, 'lib', 'data-service.ts');

      // Verify initial state
      expect(existsSync(srcUtilsPath)).toBe(true);
      expect(existsSync(srcDataServicePath)).toBe(true);

      // Capture initial file states
      const initialMainContent = readFileSync(join(TEST_DIR, 'main.ts'), 'utf-8');
      const initialConsumerContent = readFileSync(join(TEST_DIR, 'lib', 'consumer.ts'), 'utf-8');
      const initialUtilsContent = readFileSync(srcUtilsPath, 'utf-8');
      const initialDataServiceContent = readFileSync(srcDataServicePath, 'utf-8');

      // Engineer a failure: Create a file at one of the target destinations
      // to cause a collision during the atomic operation
      mkdirSync(join(TEST_DIR, 'lib'), { recursive: true });
      writeFileSync(targetDataServicePath, 'BLOCKING FILE CONTENT'); // This will cause a collision

      // Attempt the batch move (should fail due to file collision)
      const result = await client.callTool('batch_move_files', {
        moves: [
          {
            old_path: srcUtilsPath,
            new_path: targetUtilsPath,
          },
          {
            old_path: srcDataServicePath,
            new_path: targetDataServicePath, // This will fail due to existing file
          },
        ],
        dry_run: false,
        strategy: 'safe', // Safe strategy should abort on any failure
      });

      assertToolResult(result);
      const response = result.content[0]?.text || '';

      // Verify failure is reported
      expect(response).toContain('Validation Failed');
      expect(response).toContain('Target file already exists');

      // Critical verification: Ensure complete rollback
      // 1. Source files should still exist in original locations
      expect(existsSync(srcUtilsPath)).toBe(true);
      expect(existsSync(srcDataServicePath)).toBe(true);

      // 2. Target locations should not have our moved files (except the blocking file we created)
      expect(existsSync(targetUtilsPath)).toBe(false);
      const targetDataServiceContent = readFileSync(targetDataServicePath, 'utf-8');
      expect(targetDataServiceContent).toBe('BLOCKING FILE CONTENT'); // Still the blocking file

      // 3. All import files should have their original content (no partial updates)
      const finalMainContent = readFileSync(join(TEST_DIR, 'main.ts'), 'utf-8');
      const finalConsumerContent = readFileSync(join(TEST_DIR, 'lib', 'consumer.ts'), 'utf-8');
      const finalUtilsContent = readFileSync(srcUtilsPath, 'utf-8');
      const finalDataServiceContent = readFileSync(srcDataServicePath, 'utf-8');

      expect(finalMainContent).toBe(initialMainContent);
      expect(finalConsumerContent).toBe(initialConsumerContent);
      expect(finalUtilsContent).toBe(initialUtilsContent);
      expect(finalDataServiceContent).toBe(initialDataServiceContent);

      // Clean up the blocking file for other tests
      rmSync(targetDataServicePath);

      console.log('✅ Atomic rollback verified - all files restored to original state');
    });
  });

  describe('Complex Operation Preview', () => {
    it('should preview complex batch operations with detailed analysis', async () => {
      console.log('🔍 Testing complex operation preview...');

      const result = await client.callTool('preview_batch_operation', {
        operations: [
          {
            type: 'move_file',
            old_path: join(TEST_DIR, 'src', 'utils.ts'),
            new_path: join(TEST_DIR, 'lib', 'utilities.ts'), // Different name
          },
          {
            type: 'move_file',
            old_path: join(TEST_DIR, 'src', 'data-service.ts'),
            new_path: join(TEST_DIR, 'services', 'data-processor.ts'), // Different directory and name
          },
        ],
        detailed: true,
      });

      assertToolResult(result);
      const response = result.content[0]?.text || '';

      // Verify preview structure
      expect(response).toContain('Batch Operation Preview');
      expect(response).toContain('**Operations**: 2');
      expect(response).toContain('**Preview mode**: Detailed');
      expect(response).toContain('Operations Preview');
      expect(response).toContain('Summary');

      // Should show operation details
      expect(response).toContain('move_file');
      expect(response).toContain('utilities.ts');
      expect(response).toContain('data-processor.ts');

      console.log('✅ Complex operation preview complete');
    });
  });
});

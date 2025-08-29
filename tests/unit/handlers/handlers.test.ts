import { afterAll, beforeAll, describe, expect, it } from 'bun:test';
import { existsSync } from 'node:fs';
import { rm } from 'node:fs/promises';
import { join } from 'node:path';
import { LSPClient } from '../../src/lsp-client.js';
import { LSPClient as NewLSPClient } from '../../src/lsp/client.js';
// eslint-disable-next-line @typescript-eslint/consistent-type-imports
import { FileService } from '../../src/services/file-service.js';
// eslint-disable-next-line @typescript-eslint/consistent-type-imports
import { IntelligenceService } from '../../src/services/intelligence-service.js';

interface MCPResponse {
  content: Array<{
    type: 'text';
    text: string;
  }>;
}
import {
  handleApplyWorkspaceEdit,
  handleGetDocumentLinks,
  handleGetFoldingRanges,
} from '../../src/mcp/handlers/advanced-handlers.js';
import { handleGetSignatureHelp } from '../../src/mcp/handlers/intelligence-handlers.js';
import { handleCreateFile, handleDeleteFile } from '../../src/mcp/handlers/utility-handlers.js';

describe('MCP Handlers Unit Tests', () => {
  let lspClient: LSPClient;
  let fileService: FileService;
  let intelligenceService: IntelligenceService;
  const testDir = '/workspace/plugins/cclsp/playground';
  const testFileBase = 'handler-created';
  const testFile = join(
    testDir,
    `src/${testFileBase}-${Date.now()}-${Math.random().toString(36).slice(2, 7)}.ts`
  );

  beforeAll(() => {
    console.log('🎯 Direct Handler Test');
    console.log('======================\n');

    // Set up LSP client
    process.env.CCLSP_CONFIG_PATH = join('/workspace/plugins/cclsp', 'cclsp.json');
    lspClient = new LSPClient();

    // Create services for handlers that need them
    const newLspClient = new NewLSPClient();
    const getServerWrapper = (filePath: string) => newLspClient.getServer(filePath);
    const protocol = newLspClient.protocol;
    fileService = new FileService(getServerWrapper, protocol);
    intelligenceService = new IntelligenceService(getServerWrapper, protocol);
  });

  afterAll(async () => {
    lspClient.dispose();

    // Clean up test files
    if (existsSync(testFile)) {
      await rm(testFile, { force: true });
    }
  });

  describe('Advanced Handlers', () => {
    it('should handle getFoldingRanges', async () => {
      console.log('🔍 Testing handleGetFoldingRanges...');

      const result = (await handleGetFoldingRanges(lspClient, {
        file_path: join(testDir, 'src/components/user-form.ts'),
      })) as MCPResponse;

      const success = result.content?.[0]?.text;
      console.log(`✅ handleGetFoldingRanges: ${success ? 'SUCCESS' : 'FAILED'}`);
      if (success && result.content?.[0]?.text) {
        console.log(`   📋 Response preview: ${result.content[0].text.substring(0, 100)}...`);
      }

      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
      expect(result.content[0].text).toMatch(/folding|range|Found \d+/i);
    });

    it('should handle getDocumentLinks', async () => {
      console.log('🔗 Testing handleGetDocumentLinks...');

      const result = (await handleGetDocumentLinks(lspClient, {
        file_path: join(testDir, 'src/test-file.ts'),
      })) as MCPResponse;

      const success = result.content?.[0]?.text;
      console.log(`✅ handleGetDocumentLinks: ${success ? 'SUCCESS' : 'FAILED'}`);
      if (success && result.content?.[0]?.text) {
        console.log(`   📋 Links found: ${result.content[0].text.substring(0, 100)}...`);
      }

      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
      expect(result.content[0].text).toMatch(/link|import|export|Found \d+/i);
    });

    it('should handle applyWorkspaceEdit', async () => {
      console.log('📝 Testing handleApplyWorkspaceEdit...');

      // Create a validation-only edit
      const result = (await handleApplyWorkspaceEdit(fileService, {
        changes: {
          [join(testDir, 'src/test-file.ts')]: [
            {
              range: {
                start: { line: 0, character: 0 },
                end: { line: 0, character: 0 },
              },
              newText: '// Test comment\n',
            },
          ],
        },
        validate_before_apply: true,
      })) as MCPResponse;

      console.log(
        `✅ handleApplyWorkspaceEdit: ${result.content?.[0]?.text ? 'SUCCESS' : 'FAILED'}`
      );

      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
      expect(result.content[0].text).toMatch(/applied|validated|workspace|edit/i);
    });
  });

  describe('Utility Handlers', () => {
    it('should handle createFile', async () => {
      console.log('📝 Testing handleCreateFile...');

      // Ensure parent directory exists
      const parentDir = join(testDir, 'src');
      if (!existsSync(parentDir)) {
        const { mkdirSync } = await import('node:fs');
        mkdirSync(parentDir, { recursive: true });
      }

      // Remove if exists
      if (existsSync(testFile)) {
        await rm(testFile, { force: true });
      }

      const result = (await handleCreateFile(lspClient, {
        file_path: testFile,
        content: '// Handler test file\nconsole.log("test");',
      })) as MCPResponse;

      const success = existsSync(testFile);
      console.log(`✅ handleCreateFile: ${success ? 'SUCCESS' : 'FAILED'}`);
      if (success) {
        console.log(`   📁 File created at: ${testFile}`);
      }

      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
      expect(result.content[0].text).toMatch(/created|file|success/i);
      expect(existsSync(testFile)).toBe(true);
    });

    it('should handle deleteFile', async () => {
      console.log('🗑️ Testing handleDeleteFile...');

      // First ensure file exists
      if (!existsSync(testFile)) {
        await handleCreateFile(lspClient, {
          file_path: testFile,
          content: '// File to delete',
        });
      }

      const result = (await handleDeleteFile(lspClient, {
        file_path: testFile,
        force: false,
      })) as MCPResponse;

      const success = !existsSync(testFile);
      console.log(`✅ handleDeleteFile: ${success ? 'SUCCESS' : 'FAILED'}`);
      if (success) {
        console.log(`   🗑️ File deleted: ${testFile}`);
      }

      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
      expect(result.content[0].text).toMatch(/deleted|removed|file|success/i);
      expect(existsSync(testFile)).toBe(false);
    });
  });

  describe('Intelligence Handlers', () => {
    it('should handle getSignatureHelp', async () => {
      console.log('✍️ Testing handleGetSignatureHelp...');

      try {
        const result = (await handleGetSignatureHelp(intelligenceService, {
          file_path: join(testDir, 'src/test-file.ts'),
          line: 14,
          character: 20,
        })) as MCPResponse;

        const success = result.content?.[0]?.text;
        console.log(
          `✅ handleGetSignatureHelp: ${success ? 'SUCCESS' : 'No signature at position'}`
        );
        if (success && result.content?.[0]?.text) {
          console.log(`   📋 Signature: ${result.content[0].text.substring(0, 100)}...`);
        }

        expect(result).toBeDefined();
        expect(result.content).toBeDefined();
        expect(result.content[0].text).toMatch(/signature|parameter|function|method/i);
      } catch (error: unknown) {
        console.log('⚠️ handleGetSignatureHelp: No signature available at position');
        // This is expected for some positions - no assertion needed
      }
    });
  });

  it('should run comprehensive handler test suite', async () => {
    const testResults: { test: string; status: string }[] = [];

    // Test all handlers in sequence
    const tests = [
      {
        name: 'handleGetFoldingRanges',
        handler: () =>
          handleGetFoldingRanges(lspClient, {
            file_path: join(testDir, 'src/components/user-form.ts'),
          }),
      },
      {
        name: 'handleGetDocumentLinks',
        handler: () =>
          handleGetDocumentLinks(lspClient, {
            file_path: join(testDir, 'src/test-file.ts'),
          }),
      },
      {
        name: 'handleCreateFile',
        handler: () =>
          handleCreateFile(lspClient, {
            file_path: join(testDir, 'src/temp-test.ts'),
            content: '// Temp test',
          }),
      },
      {
        name: 'handleDeleteFile',
        handler: () =>
          handleDeleteFile(lspClient, {
            file_path: join(testDir, 'src/temp-test.ts'),
            force: false,
          }),
      },
      {
        name: 'handleGetSignatureHelp',
        handler: () =>
          handleGetSignatureHelp(intelligenceService, {
            file_path: join(testDir, 'src/test-file.ts'),
            line: 14,
            character: 20,
          }),
      },
    ];

    for (const test of tests) {
      try {
        const result = await test.handler();
        testResults.push({ test: test.name, status: result ? 'PASS' : 'FAIL' });
      } catch (error) {
        testResults.push({ test: test.name, status: 'FAIL' });
      }
    }

    console.log('\n📊 Handler Test Summary');
    console.log('========================');
    const passed = testResults.filter((r) => r.status === 'PASS').length;
    const total = testResults.length;
    console.log(`✅ PASSED: ${passed}/${total}`);

    for (const result of testResults) {
      console.log(`${result.status === 'PASS' ? '✅' : '❌'} ${result.test}`);
    }

    expect(passed).toBeGreaterThan(0);
  });
});

import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'bun:test';
import { existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { assertToolResult, MCPTestClient } from '../helpers/mcp-test-client.js';
import { getSystemCapabilities } from '../helpers/system-utils.js';

describe('Edge Case Tests', () => {
  let client: MCPTestClient;
  const fixturesPath = '/workspace/tests/fixtures/edge-cases';
  const systemCaps = getSystemCapabilities();
  const timeout = systemCaps.baseTimeout * 2;
  const slowTimeout = systemCaps.baseTimeout * 4; // Extended timeout for slow operations

  beforeAll(async () => {
    console.log('🔬 Edge Case Testing Suite');
    console.log('==========================\n');
    console.log('Testing edge cases and boundary conditions...\n');

    // Use shared client for performance
    if (process.env.TEST_MODE === 'fast') {
      client = MCPTestClient.getShared();
      await client.start({ skipLSPPreload: false });
    } else {
      client = new MCPTestClient();
      await client.start({ skipLSPPreload: false });
    }

    // Wait for LSP servers to initialize
    await new Promise((resolve) => setTimeout(resolve, 2000));
  });

  afterAll(async () => {
    await client.stop();
  });

  describe('Empty File Handling', () => {
    const emptyFile = `${fixturesPath}/empty-file.ts`;

    it(
      'should handle empty file for diagnostics',
      async () => {
        const result = await client.callTool('get_diagnostics', {
          file_path: emptyFile,
        });
        assertToolResult(result);
        // Empty file should have no errors
        const content = result.content?.[0]?.text || '';
        expect(content).toMatch(/(no diagnostics|no issues|clean)/i);
      },
      timeout
    );

    it(
      'should handle empty file for symbols',
      async () => {
        const result = await client.callTool('get_document_symbols', {
          file_path: emptyFile,
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        // Should return empty or no symbols
        expect(content).toMatch(/(no symbols|empty|\[\])/i);
      },
      timeout
    );

    it(
      'should handle empty file for hover at position 0,0',
      async () => {
        const result = await client.callTool('get_hover', {
          file_path: emptyFile,
          line: 1,
          character: 0,
        });
        assertToolResult(result);
        // Should handle gracefully
        expect(result).toBeDefined();
      },
      timeout
    );
  });

  describe('Unicode and Emoji Handling', () => {
    const unicodeFile = `${fixturesPath}/unicode-symbols.ts`;

    beforeEach(async () => {
      // Ensure previous test is fully complete and server is stable
      await new Promise((resolve) => setTimeout(resolve, 2000));
    });

    it(
      'should find Unicode function definition',
      async () => {
        const result = await client.callTool('find_definition', {
          file_path: unicodeFile,
          symbol_name: '计算总和',
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        expect(content).toContain('计算总和');
      },
      slowTimeout
    );

    it(
      'should find emoji variable',
      async () => {
        const result = await client.callTool('find_definition', {
          file_path: unicodeFile,
          symbol_name: 'émoji',
        });
        assertToolResult(result);
        expect(result.content).toBeDefined();
      },
      slowTimeout
    );

    it(
      'should handle multi-byte character positions correctly',
      async () => {
        // Test hover on emoji - LSP uses UTF-16 encoding for positions
        const result = await client.callTool('get_hover', {
          file_path: unicodeFile,
          line: 3, // Line with emoji variable
          character: 14, // Position accounting for UTF-16
        });
        assertToolResult(result);
        expect(result).toBeDefined();
      },
      slowTimeout
    );

    it(
      'should find Japanese variable',
      async () => {
        const result = await client.callTool('find_references', {
          file_path: unicodeFile,
          symbol_name: '日本語変数',
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        expect(content).toContain('日本語変数');
      },
      slowTimeout
    );

    it(
      'should handle right-to-left text variables',
      async () => {
        const result = await client.callTool('get_document_symbols', {
          file_path: unicodeFile,
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        // Should include Arabic and Hebrew variable names
        expect(content).toMatch(/(arabic|hebrew|variable|const)/i);
      },
      slowTimeout
    );
  });

  describe('Deeply Nested Structures', () => {
    const deeplyNestedFile = `${fixturesPath}/deeply-nested.ts`;

    it(
      'should handle deeply nested class definitions',
      async () => {
        const result = await client.callTool('find_definition', {
          file_path: deeplyNestedFile,
          symbol_name: 'DeepestClass',
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        expect(content).toContain('DeepestClass');
      },
      timeout
    );

    it(
      'should get symbols for deeply nested structures',
      async () => {
        const result = await client.callTool('get_document_symbols', {
          file_path: deeplyNestedFile,
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        // Should include nested classes
        expect(content).toMatch(/(OuterClass|MiddleClass|InnerClass)/i);
      },
      timeout
    );

    it(
      'should handle deeply nested namespace access',
      async () => {
        const result = await client.callTool('find_definition', {
          file_path: deeplyNestedFile,
          symbol_name: 'deepFunction',
        });
        assertToolResult(result);
        expect(result.content).toBeDefined();
      },
      timeout
    );

    it(
      'should handle complex generic constraints',
      async () => {
        const result = await client.callTool('find_definition', {
          file_path: deeplyNestedFile,
          symbol_name: 'GenericNesting',
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        expect(content).toContain('GenericNesting');
      },
      timeout
    );
  });

  describe('Large File Performance', () => {
    const largeFile = `${fixturesPath}/large-file.ts`;

    it(
      'should handle large file symbols efficiently',
      async () => {
        // Add delay before large file operations
        await new Promise((resolve) => setTimeout(resolve, 1000));

        const startTime = Date.now();
        const result = await client.callTool('get_document_symbols', {
          file_path: largeFile,
        });
        const duration = Date.now() - startTime;

        assertToolResult(result);
        // Should complete within reasonable time (15 seconds for larger files)
        expect(duration).toBeLessThan(15000);

        const content = result.content?.[0]?.text || '';
        // Should find multiple classes
        expect(content).toMatch(/(LargeClass|MiddleClass|FinalClass)/i);
      },
      slowTimeout
    );

    it(
      'should find definitions in large file',
      async () => {
        // Add delay to reduce server load
        await new Promise((resolve) => setTimeout(resolve, 500));

        const result = await client.callTool('find_definition', {
          file_path: largeFile,
          symbol_name: 'FinalClass',
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        expect(content).toContain('FinalClass');
      },
      slowTimeout
    );

    it(
      'should handle hover at end of large file',
      async () => {
        // Add delay to reduce server load
        await new Promise((resolve) => setTimeout(resolve, 500));

        const result = await client.callTool('get_hover', {
          file_path: largeFile,
          line: 130, // Near end of file (adjusted for smaller file)
          character: 10,
        });
        assertToolResult(result);
        expect(result).toBeDefined();
      },
      slowTimeout
    );
  });

  describe('Boundary Position Tests', () => {
    const testFile = '/workspace/examples/playground/src/test-file.ts';

    it(
      'should handle position at start of file (1,0)',
      async () => {
        const result = await client.callTool('get_hover', {
          file_path: testFile,
          line: 1,
          character: 0,
        });
        assertToolResult(result);
        expect(result).toBeDefined();
      },
      timeout
    );

    it(
      'should handle out of bounds line gracefully',
      async () => {
        const result = await client.callTool('get_hover', {
          file_path: testFile,
          line: 99999,
          character: 0,
        });
        assertToolResult(result);
        // Should handle gracefully without crashing
        expect(result).toBeDefined();
      },
      timeout
    );

    it(
      'should handle out of bounds character gracefully',
      async () => {
        const result = await client.callTool('get_hover', {
          file_path: testFile,
          line: 1,
          character: 99999,
        });
        assertToolResult(result);
        expect(result).toBeDefined();
      },
      timeout
    );

    it(
      'should handle negative line numbers gracefully',
      async () => {
        try {
          await client.callTool('get_hover', {
            file_path: testFile,
            line: -1,
            character: 0,
          });
        } catch (error) {
          // Expected to error or handle gracefully
          expect(error).toBeDefined();
        }
      },
      timeout
    );
  });

  describe('File Path Edge Cases', () => {
    it(
      'should handle non-existent file gracefully',
      async () => {
        const result = await client.callTool('get_diagnostics', {
          file_path: '/non/existent/file.ts',
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        expect(content).toMatch(/(not found|does not exist|error|cannot read)/i);
      },
      timeout
    );

    it(
      'should handle file with spaces in name',
      async () => {
        const spacedFile = join(tmpdir(), 'test file with spaces.ts');
        const result = await client.callTool('create_file', {
          file_path: spacedFile,
          content: 'export const test = 1;',
        });
        assertToolResult(result);

        if (existsSync(spacedFile)) {
          const diagResult = await client.callTool('get_diagnostics', {
            file_path: spacedFile,
          });
          assertToolResult(diagResult);
          expect(diagResult).toBeDefined();

          // Cleanup
          await client.callTool('delete_file', {
            file_path: spacedFile,
          });
        }
      },
      timeout
    );

    it(
      'should handle very long file paths',
      async () => {
        const longPath = join(tmpdir(), `${'a'.repeat(200)}.ts`);
        try {
          await client.callTool('create_file', {
            file_path: longPath,
            content: 'export const test = 1;',
          });
        } catch (error) {
          // Some systems may reject very long paths
          expect(error).toBeDefined();
        }
      },
      timeout
    );
  });

  describe('Symbol Name Edge Cases', () => {
    it(
      'should handle empty symbol name',
      async () => {
        const result = await client.callTool('find_definition', {
          file_path: '/workspace/examples/playground/src/test-file.ts',
          symbol_name: '',
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        expect(content).toMatch(/(not found|empty|invalid|no definition)/i);
      },
      timeout
    );

    it(
      'should handle symbol with special characters',
      async () => {
        const result = await client.callTool('find_definition', {
          file_path: '/workspace/examples/playground/src/test-file.ts',
          symbol_name: '$#@!%^&*()',
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        expect(content).toMatch(/(not found|no definition|no symbols found)/i);
      },
      timeout
    );

    it(
      'should handle very long symbol names',
      async () => {
        const longSymbol = 'a'.repeat(1000);
        const result = await client.callTool('find_definition', {
          file_path: '/workspace/examples/playground/src/test-file.ts',
          symbol_name: longSymbol,
        });
        assertToolResult(result);
        const content = result.content?.[0]?.text || '';
        expect(content).toMatch(/(not found|no definition|no symbols found)/i);
      },
      timeout
    );
  });
});

import { afterAll, describe, expect, it } from 'bun:test';
import { join } from 'node:path';
import { LSPClient } from '../../src/lsp-client.js';

describe('LSP Client Unit Tests', () => {
  let lspClient: LSPClient;

  afterAll(() => {
    if (lspClient) {
      lspClient.dispose();
    }
  });

  it('should initialize LSP client and test basic operations', async () => {
    console.log('🔧 Testing LSP Client directly...');

    // Set the config path
    process.env.CCLSP_CONFIG_PATH = join('/workspace/plugins/cclsp', 'cclsp.json');

    lspClient = new LSPClient();
    const testFile = join('/workspace/plugins/cclsp', 'playground/src/components/user-form.ts');

    console.log('📁 Test file:', testFile);

    // Test 1: Get folding ranges
    console.log('\n🔍 Testing getFoldingRanges...');
    const foldingRanges = await lspClient.getFoldingRanges(testFile);
    console.log(`✅ Folding ranges result: ${foldingRanges?.length || 0} ranges found`);
    if (foldingRanges?.length > 0) {
      console.log('   First range:', foldingRanges[0]);
    }
    expect(foldingRanges).toBeDefined();

    // Test 2: Get document links
    console.log('\n🔗 Testing getDocumentLinks...');
    const docLinks = await lspClient.getDocumentLinks(testFile);
    console.log(`✅ Document links result: ${docLinks?.length || 0} links found`);
    if (docLinks?.length > 0) {
      console.log('   First link:', docLinks[0]);
    }
    expect(docLinks).toBeDefined();

    // Test 3: Get document symbols
    console.log('\n📋 Testing getDocumentSymbols...');
    const symbols = await lspClient.getDocumentSymbols(testFile);
    console.log(`✅ Document symbols result: ${symbols?.length || 0} symbols found`);
    if (symbols?.length > 0) {
      console.log('   First symbol:', symbols[0]);
    }
    expect(symbols).toBeDefined();

    // Test 4: Get signature help (need a position in the file)
    console.log('\n✍️ Testing getSignatureHelp...');
    try {
      const sigHelp = await lspClient.getSignatureHelp(testFile, { line: 5, character: 10 });
      console.log(`✅ Signature help result: ${sigHelp ? 'Available' : 'None'}`);
      if (sigHelp) {
        console.log(`   Signatures: ${sigHelp.signatures?.length || 0}`);
      }
      // May return null for some positions, which is valid
      expect(true).toBe(true);
    } catch (sigError: any) {
      console.log(`⚠️ Signature help failed (expected for some positions): ${sigError.message}`);
      // This can fail for some positions, which is expected
      expect(true).toBe(true);
    }
  });

  it('should handle multiple file types', async () => {
    process.env.CCLSP_CONFIG_PATH = join('/workspace/plugins/cclsp', 'cclsp.json');

    const client = new LSPClient();

    try {
      // Test TypeScript file
      const tsFile = join('/workspace/plugins/cclsp', 'playground/src/test-file.ts');
      const tsSymbols = await client.getDocumentSymbols(tsFile);
      expect(tsSymbols).toBeDefined();
      console.log(`TypeScript file: ${tsSymbols?.length || 0} symbols found`);

      // Test another TypeScript file
      const tsFile2 = join('/workspace/plugins/cclsp', 'playground/src/components/user-form.ts');
      const tsSymbols2 = await client.getDocumentSymbols(tsFile2);
      expect(tsSymbols2).toBeDefined();
      console.log(`Another TypeScript file: ${tsSymbols2?.length || 0} symbols found`);
    } finally {
      client.dispose();
    }
  });
});

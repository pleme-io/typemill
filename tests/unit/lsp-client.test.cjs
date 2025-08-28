// Test LSP client functionality directly
const path = require('node:path');

async function testLSPClient() {
  // Import ES modules
  const { LSPClient } = await import('./dist/src/lsp-client.js');

  console.log('🔧 Testing LSP Client directly...');

  // Set the config path
  process.env.CCLSP_CONFIG_PATH = path.join(__dirname, 'test-config.json');

  const lspClient = new LSPClient();
  const testFile = path.join(__dirname, 'playground/src/components/user-form.ts');

  try {
    console.log('📁 Test file:', testFile);

    // Test 1: Get folding ranges
    console.log('\n🔍 Testing getFoldingRanges...');
    const foldingRanges = await lspClient.getFoldingRanges(testFile);
    console.log('✅ Folding ranges result:', foldingRanges?.length || 0, 'ranges found');
    if (foldingRanges?.length > 0) {
      console.log('   First range:', foldingRanges[0]);
    }

    // Test 2: Get document links
    console.log('\n🔗 Testing getDocumentLinks...');
    const docLinks = await lspClient.getDocumentLinks(testFile);
    console.log('✅ Document links result:', docLinks?.length || 0, 'links found');
    if (docLinks?.length > 0) {
      console.log('   First link:', docLinks[0]);
    }

    // Test 3: Get document symbols
    console.log('\n📋 Testing getDocumentSymbols...');
    const symbols = await lspClient.getDocumentSymbols(testFile);
    console.log('✅ Document symbols result:', symbols?.length || 0, 'symbols found');
    if (symbols?.length > 0) {
      console.log('   First symbol:', symbols[0]);
    }

    // Test 4: Get signature help (need a position in the file)
    console.log('\n✍️ Testing getSignatureHelp...');
    try {
      const sigHelp = await lspClient.getSignatureHelp(testFile, { line: 5, character: 10 });
      console.log('✅ Signature help result:', sigHelp ? 'Available' : 'None');
      if (sigHelp) {
        console.log('   Signatures:', sigHelp.signatures?.length || 0);
      }
    } catch (sigError) {
      console.log('⚠️ Signature help failed (expected for some positions):', sigError.message);
    }
  } catch (error) {
    console.error('❌ Test failed:', error);
  } finally {
    lspClient.dispose();
  }
}

testLSPClient().catch(console.error);

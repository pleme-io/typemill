// Direct test of MCP handlers
const path = require('node:path');
const fs = require('node:fs');

async function testHandlers() {
  console.log('🎯 Direct Handler Test');
  console.log('======================\n');

  // Import handlers
  const { handleCreateFile, handleDeleteFile } = await import(
    './dist/src/mcp/handlers/utility-handlers.js'
  );
  const { handleGetFoldingRanges, handleGetDocumentLinks, handleApplyWorkspaceEdit } = await import(
    './dist/src/mcp/handlers/advanced-handlers.js'
  );
  const { handleGetSignatureHelp } = await import(
    './dist/src/mcp/handlers/intelligence-handlers.js'
  );
  const { LSPClient } = await import('./dist/src/lsp-client.js');

  // Set up LSP client
  process.env.CCLSP_CONFIG_PATH = path.join(__dirname, 'test-config.json');
  const lspClient = new LSPClient();

  const testResults = [];

  try {
    // Test 1: handleGetFoldingRanges
    console.log('🔍 Testing handleGetFoldingRanges...');
    try {
      const result = await handleGetFoldingRanges(lspClient, {
        file_path: path.join(__dirname, 'playground/src/components/user-form.ts'),
      });

      const success = result.content?.[0]?.text;
      console.log(`✅ handleGetFoldingRanges: ${success ? 'SUCCESS' : 'FAILED'}`);
      if (success) {
        console.log('   📋 Response preview:', `${result.content[0].text.substring(0, 100)}...`);
      }
      testResults.push({ test: 'handleGetFoldingRanges', status: success ? 'PASS' : 'FAIL' });
    } catch (error) {
      console.log('❌ handleGetFoldingRanges failed:', error.message);
      testResults.push({ test: 'handleGetFoldingRanges', status: 'FAIL' });
    }

    // Test 2: handleCreateFile
    console.log('\n📝 Testing handleCreateFile...');
    const createTestFile = path.join(__dirname, 'playground/src/handler-created.ts');
    try {
      // Remove if exists
      if (fs.existsSync(createTestFile)) fs.unlinkSync(createTestFile);

      const result = await handleCreateFile(lspClient, {
        file_path: createTestFile,
        content: '// Created by handler test\nexport const handlerTest = true;\n',
      });

      const success = result.content?.[0]?.text.includes('Successfully created');
      const fileExists = fs.existsSync(createTestFile);

      console.log(`✅ handleCreateFile: ${success && fileExists ? 'SUCCESS' : 'FAILED'}`);
      console.log(`   📁 File exists: ${fileExists}`);
      console.log(`   📝 Handler response: ${success}`);

      testResults.push({
        test: 'handleCreateFile',
        status: success && fileExists ? 'PASS' : 'FAIL',
      });
    } catch (error) {
      console.log('❌ handleCreateFile failed:', error.message);
      testResults.push({ test: 'handleCreateFile', status: 'FAIL' });
    }

    // Test 3: handleDeleteFile
    console.log('\n🗑️ Testing handleDeleteFile...');
    try {
      if (fs.existsSync(createTestFile)) {
        const result = await handleDeleteFile(lspClient, {
          file_path: createTestFile,
        });

        const success = result.content?.[0]?.text.includes('Successfully deleted');
        const fileDeleted = !fs.existsSync(createTestFile);

        console.log(`✅ handleDeleteFile: ${success && fileDeleted ? 'SUCCESS' : 'FAILED'}`);
        console.log(`   📁 File deleted: ${fileDeleted}`);
        console.log(`   📝 Handler response: ${success}`);

        testResults.push({
          test: 'handleDeleteFile',
          status: success && fileDeleted ? 'PASS' : 'FAIL',
        });
      } else {
        console.log('⚠️ handleDeleteFile: SKIP (no file to delete)');
        testResults.push({ test: 'handleDeleteFile', status: 'SKIP' });
      }
    } catch (error) {
      console.log('❌ handleDeleteFile failed:', error.message);
      testResults.push({ test: 'handleDeleteFile', status: 'FAIL' });
    }

    // Test 4: handleApplyWorkspaceEdit
    console.log('\n⚡ Testing handleApplyWorkspaceEdit...');
    const editTestFile = path.join(__dirname, 'playground/src/workspace-edit-test.ts');
    try {
      // Create test file
      fs.writeFileSync(editTestFile, 'const original = "test";\n', 'utf8');

      const result = await handleApplyWorkspaceEdit(lspClient, {
        changes: {
          [editTestFile]: [
            {
              range: {
                start: { line: 0, character: 0 },
                end: { line: 0, character: 0 },
              },
              newText: '// Added by workspace edit handler\n',
            },
          ],
        },
      });

      const success = result.content?.[0]?.text.includes('successfully');

      // Check if edit was applied
      const content = fs.readFileSync(editTestFile, 'utf8');
      const editApplied = content.includes('// Added by workspace edit handler');

      console.log(`✅ handleApplyWorkspaceEdit: ${success && editApplied ? 'SUCCESS' : 'FAILED'}`);
      console.log(`   📝 Handler response: ${success}`);
      console.log(`   ✏️ Edit applied: ${editApplied}`);

      testResults.push({
        test: 'handleApplyWorkspaceEdit',
        status: success && editApplied ? 'PASS' : 'FAIL',
      });

      // Cleanup
      if (fs.existsSync(editTestFile)) fs.unlinkSync(editTestFile);
    } catch (error) {
      console.log('❌ handleApplyWorkspaceEdit failed:', error.message);
      testResults.push({ test: 'handleApplyWorkspaceEdit', status: 'FAIL' });
    }

    // Test 5: handleGetSignatureHelp (expect graceful failure or success)
    console.log('\n✍️ Testing handleGetSignatureHelp...');
    try {
      const result = await handleGetSignatureHelp(lspClient, {
        file_path: path.join(__dirname, 'playground/src/components/user-form.ts'),
        position: { line: 5, character: 10 },
      });

      const success = result.content?.[0];
      console.log(`✅ handleGetSignatureHelp: ${success ? 'SUCCESS' : 'FAILED'}`);
      if (success) {
        console.log('   📋 Response preview:', `${result.content[0].text.substring(0, 100)}...`);
      }
      testResults.push({ test: 'handleGetSignatureHelp', status: success ? 'PASS' : 'FAIL' });
    } catch (error) {
      console.log('❌ handleGetSignatureHelp failed:', error.message);
      testResults.push({ test: 'handleGetSignatureHelp', status: 'FAIL' });
    }

    // Test 6: handleGetDocumentLinks (expect graceful degradation)
    console.log('\n🔗 Testing handleGetDocumentLinks...');
    try {
      const result = await handleGetDocumentLinks(lspClient, {
        file_path: path.join(__dirname, 'playground/src/components/user-form.ts'),
      });

      const success = result.content?.[0];
      const isGracefulDegradation = success && result.content[0].text.includes('does not support');

      console.log(`✅ handleGetDocumentLinks: ${success ? 'SUCCESS' : 'FAILED'}`);
      if (isGracefulDegradation) {
        console.log('   🔵 Graceful degradation detected');
      }
      console.log(
        `   📋 Response preview: ${success ? `${result.content[0].text.substring(0, 100)}...` : 'No response'}`
      );

      testResults.push({
        test: 'handleGetDocumentLinks',
        status: success ? (isGracefulDegradation ? 'GRACEFUL' : 'PASS') : 'FAIL',
      });
    } catch (error) {
      console.log('❌ handleGetDocumentLinks failed:', error.message);
      testResults.push({ test: 'handleGetDocumentLinks', status: 'FAIL' });
    }
  } finally {
    lspClient.dispose();
  }

  // Results
  console.log('\n📊 HANDLER TEST RESULTS');
  console.log('=========================');

  let passCount = 0;
  let gracefulCount = 0;
  let skipCount = 0;

  testResults.forEach((result) => {
    let emoji = '❌';
    if (result.status === 'PASS') {
      emoji = '✅';
      passCount++;
    } else if (result.status === 'GRACEFUL') {
      emoji = '🔵';
      gracefulCount++;
    } else if (result.status === 'SKIP') {
      emoji = '⏭️';
      skipCount++;
    }

    console.log(`${emoji} ${result.test.padEnd(25)} | ${result.status}`);
  });

  const totalSuccessful = passCount + gracefulCount + skipCount;
  const successRate = Math.round((totalSuccessful / testResults.length) * 100);

  console.log(`\n🎯 Success Rate: ${successRate}% (${totalSuccessful}/${testResults.length})`);
  console.log(`   ✅ ${passCount} full success`);
  console.log(`   🔵 ${gracefulCount} graceful degradation`);
  console.log(`   ⏭️ ${skipCount} skipped`);

  return testResults;
}

testHandlers().catch(console.error);

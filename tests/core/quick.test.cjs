const { spawn } = require('child_process');

const tests = [
  ['find_definition', { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts', symbol_name: 'calculateAge' }],
  ['find_references', { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts', symbol_name: 'TestProcessor' }],
  ['get_diagnostics', { file_path: '/workspace/plugins/cclsp/playground/src/errors-file.ts' }],
  ['get_hover', { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts', line: 13, character: 10 }],
  ['rename_symbol', { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts', symbol_name: 'TEST_CONSTANT', new_name: 'RENAMED_CONST', dry_run: true }]
];

async function runTest() {
  const mcp = spawn('node', ['dist/index.js'], {
    cwd: '/workspace/plugins/cclsp',
    env: { ...process.env, CCLSP_CONFIG_PATH: '/workspace/plugins/cclsp/cclsp.json' }
  });
  
  let buffer = '';
  let results = [];
  let currentTest = 0;
  let id = 1;
  
  mcp.stdout.on('data', (data) => {
    buffer += data.toString();
    const lines = buffer.split('\n');
    buffer = lines.pop() || '';
    
    for (const line of lines) {
      if (!line.trim()) continue;
      try {
        const msg = JSON.parse(line);
        if (msg.id === 1) {
          // Initialized, start tests
          setTimeout(() => runNextTest(), 500);
        } else if (msg.result || msg.error) {
          const testName = tests[currentTest - 1][0];
          results.push({
            name: testName,
            success: !!msg.result,
            error: msg.error?.message
          });
          console.log(`${msg.result ? '✅' : '❌'} ${testName}`);
          runNextTest();
        }
      } catch (e) {}
    }
  });
  
  function runNextTest() {
    if (currentTest >= tests.length) {
      console.log(`\nResults: ${results.filter(r => r.success).length}/${results.length} passed`);
      mcp.kill();
      process.exit(0);
    }
    
    const [name, params] = tests[currentTest++];
    const req = JSON.stringify({
      jsonrpc: '2.0',
      id: ++id,
      method: 'tools/call',
      params: { name, arguments: params }
    });
    mcp.stdin.write(req + '\n');
  }
  
  // Initialize
  mcp.stdin.write(JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: { protocolVersion: '0.1.0', capabilities: {}, clientInfo: { name: 'test', version: '1.0' } }
  }) + '\n');
  
  setTimeout(() => {
    console.log('Test timeout');
    mcp.kill();
    process.exit(1);
  }, 30000);
}

runTest();

#!/usr/bin/env node

const { spawn } = require('node:child_process');

console.log('🔍 Testing playground with detailed analysis...');

const mcp = spawn('node', ['dist/index.js'], {
  cwd: '/workspace/plugins/cclsp',
  env: {
    ...process.env,
    CCLSP_CONFIG_PATH: '/workspace/plugins/cclsp/cclsp.json',
  },
});

let initialized = false;
let testCount = 0;

const tests = [
  {
    name: 'diagnostics on test-file.ts',
    tool: 'get_diagnostics',
    args: { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts' },
  },
  {
    name: 'hover on calculateAge function',
    tool: 'get_hover',
    args: {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      line: 13,
      character: 10,
    },
  },
  {
    name: 'find references to TestProcessor',
    tool: 'find_references',
    args: {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      symbol_name: 'TestProcessor',
    },
  },
  {
    name: 'document symbols',
    tool: 'get_document_symbols',
    args: { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts' },
  },
];

mcp.stdout.on('data', (data) => {
  const lines = data.toString().split('\n');
  for (const line of lines) {
    if (!line.trim()) continue;
    try {
      const msg = JSON.parse(line);
      if (msg.id === 1 && !initialized) {
        initialized = true;
        console.log('✅ MCP Server initialized');
        setTimeout(() => runNextTest(), 2000);
      } else if (msg.id > 1) {
        const testIndex = msg.id - 2;
        const test = tests[testIndex];

        if (msg.result) {
          console.log(`✅ ${test.name}: SUCCESS`);
          if (msg.result.content?.[0]?.text) {
            const preview = msg.result.content[0].text.substring(0, 100);
            console.log(`   Preview: ${preview}...`);
          } else if (Array.isArray(msg.result) && msg.result.length > 0) {
            console.log(`   Found ${msg.result.length} items`);
          } else {
            console.log(`   Result: ${JSON.stringify(msg.result).substring(0, 100)}...`);
          }
        } else if (msg.error) {
          console.log(`❌ ${test.name}: ERROR - ${msg.error.message}`);
        }

        setTimeout(() => runNextTest(), 1000);
      }
    } catch (e) {}
  }
});

function runNextTest() {
  if (testCount >= tests.length) {
    console.log('\n🎉 All playground tests completed');
    mcp.kill();
    process.exit(0);
    return;
  }

  const test = tests[testCount++];
  console.log(`\n🧪 Running: ${test.name}`);

  const req = JSON.stringify({
    jsonrpc: '2.0',
    id: testCount + 1,
    method: 'tools/call',
    params: {
      name: test.tool,
      arguments: test.args,
    },
  });
  mcp.stdin.write(`${req}\n`);
}

// Initialize
mcp.stdin.write(
  `${JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: '0.1.0',
      capabilities: {},
      clientInfo: { name: 'playground-test', version: '1.0' },
    },
  })}\n`
);

setTimeout(() => {
  console.log('❌ Test timed out');
  process.exit(1);
}, 30000);

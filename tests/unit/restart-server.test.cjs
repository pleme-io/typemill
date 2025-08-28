#!/usr/bin/env node

const { spawn } = require('node:child_process');

console.log('⏱️  Testing restart_server timing and response...');

const mcp = spawn('node', ['dist/index.js'], {
  cwd: '/workspace/plugins/cclsp',
  env: {
    ...process.env,
    CCLSP_CONFIG_PATH: '/workspace/plugins/cclsp/cclsp.json',
  },
});

let initialized = false;
let startTime = 0;

mcp.stdout.on('data', (data) => {
  const lines = data.toString().split('\n');
  for (const line of lines) {
    if (!line.trim()) continue;
    try {
      const msg = JSON.parse(line);
      if (msg.id === 1 && !initialized) {
        initialized = true;
        console.log('✅ MCP Server initialized');

        // Wait a moment for LSP servers to start
        setTimeout(() => {
          console.log('🔄 Starting restart_server test...');
          startTime = Date.now();

          const req = JSON.stringify({
            jsonrpc: '2.0',
            id: 2,
            method: 'tools/call',
            params: {
              name: 'restart_server',
              arguments: {
                extensions: ['ts', 'tsx'],
              },
            },
          });
          mcp.stdin.write(`${req}\n`);

          // Track progress every second
          const progressTimer = setInterval(() => {
            const elapsed = Date.now() - startTime;
            console.log(`⏳ Still waiting... ${elapsed}ms elapsed`);
          }, 1000);

          // Give it 30 seconds maximum
          setTimeout(() => {
            clearInterval(progressTimer);
            console.log('❌ Test timed out after 30 seconds');
            mcp.kill();
            process.exit(1);
          }, 30000);
        }, 3000);
      } else if (msg.id === 2) {
        const elapsed = Date.now() - startTime;
        console.log(`✅ restart_server completed in ${elapsed}ms`);

        if (msg.result) {
          console.log('✅ Got result:', JSON.stringify(msg.result, null, 2));
        } else if (msg.error) {
          console.log('❌ Got error:', msg.error.message);
        } else {
          console.log('⚠️  Got empty response');
        }
        mcp.kill();
        process.exit(0);
      }
    } catch (e) {}
  }
});

mcp.stderr.on('data', (data) => {
  const output = data.toString();
  // Show all restart-related debug messages with timestamps
  if (output.includes('restart') || output.includes('startServer') || output.includes('DEBUG')) {
    const elapsed = startTime > 0 ? ` [+${Date.now() - startTime}ms]` : '';
    console.log(`DEBUG${elapsed}:`, output.trim());
  }
});

// Initialize
mcp.stdin.write(
  `${JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: '0.1.0',
      capabilities: {},
      clientInfo: { name: 'timing-test', version: '1.0' },
    },
  })}\n`
);

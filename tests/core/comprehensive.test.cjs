#!/usr/bin/env node

const { spawn } = require('node:child_process');

// Test all 23 MCP tools with longer timeouts
const ALL_TESTS = [
  // Core Tools (4)
  [
    'find_definition',
    {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      symbol_name: 'calculateAge',
    },
  ],
  [
    'find_references',
    {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      symbol_name: 'TestProcessor',
    },
  ],
  [
    'rename_symbol',
    {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      symbol_name: 'renamedVariable',
      new_name: 'testVar',
      dry_run: true,
    },
  ],
  [
    'rename_symbol_strict',
    {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      line: 59,
      character: 18,
      new_name: 'strictTest',
      dry_run: true,
    },
  ],

  // Document Tools (7)
  ['get_diagnostics', { file_path: '/workspace/plugins/cclsp/playground/src/errors-file.ts' }],
  ['get_document_symbols', { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts' }],
  [
    'get_code_actions',
    {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      start_line: 9,
      start_character: 0,
      end_line: 9,
      end_character: 50,
    },
  ],
  [
    'format_document',
    {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      tab_size: 2,
      insert_spaces: true,
      dry_run: true,
    },
  ],
  ['search_workspace_symbols', { query: 'Process' }],
  ['get_folding_ranges', { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts' }],
  ['get_document_links', { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts' }],

  // Intelligence Tools (5)
  [
    'get_hover',
    { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts', line: 13, character: 10 },
  ],
  [
    'get_completions',
    { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts', line: 26, character: 10 },
  ],
  [
    'get_signature_help',
    { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts', line: 14, character: 20 },
  ],
  [
    'get_inlay_hints',
    {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      start_line: 10,
      start_character: 0,
      end_line: 20,
      end_character: 0,
    },
  ],
  ['get_semantic_tokens', { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts' }],

  // Hierarchy Tools (3)
  [
    'prepare_call_hierarchy',
    { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts', line: 13, character: 10 },
  ],
  [
    'prepare_type_hierarchy',
    { file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts', line: 18, character: 7 },
  ],
  [
    'get_selection_range',
    {
      file_path: '/workspace/plugins/cclsp/playground/src/test-file.ts',
      positions: [{ line: 13, character: 10 }],
    },
  ],

  // File Operations (3)
  [
    'create_file',
    { file_path: '/tmp/cclsp-test.ts', content: '// Test file\nconsole.log("test");' },
  ],
  [
    'rename_file',
    { old_path: '/tmp/cclsp-test.ts', new_path: '/tmp/cclsp-renamed.ts', dry_run: true },
  ],
  ['delete_file', { file_path: '/tmp/cclsp-renamed.ts', dry_run: true }],

  // Server Management (1) - Test this last with longer timeout
  ['restart_server', { extensions: ['ts', 'tsx'] }],
];

class FinalVerificationTester {
  constructor() {
    this.results = [];
    this.currentTest = 0;
    this.mcpServer = null;
    this.buffer = '';
    this.initialized = false;
  }

  async start() {
    console.log('🔬 CCLSP Final Verification Test');
    console.log('=================================\n');
    console.log(`Testing all ${ALL_TESTS.length} tools with extended timeouts...\n`);

    this.mcpServer = spawn('node', ['dist/index.js'], {
      cwd: '/workspace/plugins/cclsp',
      env: {
        ...process.env,
        CCLSP_CONFIG_PATH: '/workspace/plugins/cclsp/cclsp.json',
      },
    });

    this.setupHandlers();
    await this.initialize();
    await this.runAllTests();
    this.printResults();
  }

  setupHandlers() {
    this.mcpServer.stdout.on('data', (data) => {
      this.buffer += data.toString();
      const lines = this.buffer.split('\n');
      this.buffer = lines.pop() || '';

      for (const line of lines) {
        if (!line.trim()) continue;
        try {
          const msg = JSON.parse(line);
          this.handleMessage(msg);
        } catch (e) {}
      }
    });

    this.mcpServer.stderr.on('data', (data) => {
      // Show restart debug messages
      const output = data.toString();
      if (output.includes('restartServers')) {
        console.log('DEBUG:', output.trim());
      }
    });
  }

  handleMessage(msg) {
    if (msg.id === 1) {
      this.initialized = true;
    } else if (msg.id > 1 && this.currentTest < ALL_TESTS.length) {
      const [toolName] = ALL_TESTS[this.currentTest - 1];
      const success = !!msg.result && !msg.error;
      const hasContent = this.checkHasContent(msg.result);

      this.results.push({
        tool: toolName,
        success,
        hasContent,
        error: msg.error?.message,
        response: success ? 'OK' : 'FAILED',
      });

      const status = success ? '✅' : '❌';
      const contentInfo = hasContent ? ' (with data)' : '';
      console.log(`${status} ${toolName}${contentInfo}`);

      this.runNextTest();
    }
  }

  checkHasContent(result) {
    if (!result) return false;
    return !!(
      result.content ||
      result.items ||
      result.changes ||
      result.locations ||
      result.symbols ||
      result.ranges ||
      result.actions ||
      result.diagnostics ||
      result.edit ||
      result.message ||
      (Array.isArray(result) && result.length > 0)
    );
  }

  async initialize() {
    return new Promise((resolve) => {
      const checkInit = setInterval(() => {
        if (!this.initialized) {
          this.mcpServer.stdin.write(
            `${JSON.stringify({
              jsonrpc: '2.0',
              id: 1,
              method: 'initialize',
              params: {
                protocolVersion: '0.1.0',
                capabilities: {},
                clientInfo: { name: 'final-verification', version: '1.0' },
              },
            })}\n`
          );
        } else {
          clearInterval(checkInit);
          console.log('📡 MCP Server initialized\n');
          resolve();
        }
      }, 100);
    });
  }

  async runAllTests() {
    return new Promise((resolve) => {
      this.testResolve = resolve;
      // Wait for LSP server to fully initialize
      setTimeout(() => {
        this.runNextTest();
      }, 3000);
    });
  }

  runNextTest() {
    if (this.currentTest >= ALL_TESTS.length) {
      if (this.testResolve) {
        this.testResolve();
      }
      return;
    }

    const [toolName, params] = ALL_TESTS[this.currentTest++];

    const request = JSON.stringify({
      jsonrpc: '2.0',
      id: this.currentTest + 1,
      method: 'tools/call',
      params: {
        name: toolName,
        arguments: params,
      },
    });

    this.mcpServer.stdin.write(`${request}\n`);

    // Extended timeout for restart_server
    const timeout = toolName === 'restart_server' ? 20000 : 10000;

    setTimeout(() => {
      if (this.currentTest <= ALL_TESTS.length && !this.results.find((r) => r.tool === toolName)) {
        this.results.push({
          tool: toolName,
          success: false,
          hasContent: false,
          error: 'Timeout',
          response: 'TIMEOUT',
        });
        console.log(`⏰ ${toolName} (timeout after ${timeout / 1000}s)`);
        this.runNextTest();
      }
    }, timeout);
  }

  printResults() {
    console.log('\n=================================');
    console.log('📊 FINAL VERIFICATION RESULTS');
    console.log('=================================\n');

    const successful = this.results.filter((r) => r.success);
    const failed = this.results.filter((r) => !r.success);

    console.log(`✅ PASSED: ${successful.length}/${this.results.length}`);
    console.log(`❌ FAILED: ${failed.length}/${this.results.length}\n`);

    if (failed.length === 0) {
      console.log('🎉 ALL 23 TOOLS VERIFIED WORKING! 🎉');
      console.log('CCLSP is fully operational with complete LSP functionality.');
    } else {
      console.log(`⚠️  ${failed.length} tools still need attention:`);
      failed.forEach((result) => {
        console.log(`   ❌ ${result.tool}: ${result.error || 'Failed'}`);
      });
    }

    this.mcpServer.kill();
    process.exit(failed.length > 0 ? 1 : 0);
  }
}

// Run the final verification
const tester = new FinalVerificationTester();
tester.start().catch(console.error);

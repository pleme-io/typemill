import { type ChildProcess, spawn } from 'node:child_process';
import { join } from 'node:path';

export interface MCPMessage {
  jsonrpc: '2.0';
  id?: number;
  method?: string;
  params?: unknown;
  result?: unknown;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

export interface MCPToolCall {
  name: string;
  arguments: Record<string, unknown>;
}

export interface MCPToolResult {
  content?: Array<{ text: string }>;
  [key: string]: unknown;
}

// Type assertion helper for test results
export function assertToolResult(result: unknown): MCPToolResult {
  if (result && typeof result === 'object') {
    return result as MCPToolResult;
  }
  return { content: [{ text: String(result) }] };
}

export class MCPTestClient {
  private process: ChildProcess | null = null;
  private buffer = '';
  private messageHandlers = new Map<number, (msg: MCPMessage) => void>();
  private nextId = 1;
  private initPromise: Promise<void> | null = null;

  constructor(
    private configPath = '/workspace/plugins/cclsp/cclsp.json',
    private cwd = '/workspace/plugins/cclsp'
  ) {}

  async start(): Promise<void> {
    if (this.initPromise) return this.initPromise;

    this.initPromise = new Promise((resolve, reject) => {
      this.process = spawn('node', ['dist/index.js'], {
        cwd: this.cwd,
        env: { ...process.env, CCLSP_CONFIG_PATH: this.configPath },
      });

      this.process.stdout?.on('data', (data) => {
        this.buffer += data.toString();
        this.processBuffer();
      });

      this.process.stderr?.on('data', (data) => {
        console.error('MCP stderr:', data.toString());
      });

      this.process.on('error', (err) => {
        console.error('MCP process error:', err);
        reject(err);
      });

      // Send initialize request
      const initId = this.nextId++;
      this.messageHandlers.set(initId, (msg) => {
        if (msg.error) {
          reject(new Error(`Initialization failed: ${msg.error.message}`));
        } else {
          resolve();
        }
      });

      this.sendMessage({
        jsonrpc: '2.0',
        id: initId,
        method: 'initialize',
        params: {
          protocolVersion: '0.1.0',
          capabilities: {},
          clientInfo: { name: 'test-client', version: '1.0.0' },
        },
      });

      // Timeout after 5 seconds
      setTimeout(() => {
        if (this.messageHandlers.has(initId)) {
          this.messageHandlers.delete(initId);
          reject(new Error('Initialization timeout'));
        }
      }, 5000);
    });

    return this.initPromise;
  }

  async callTool(name: string, args: Record<string, unknown>): Promise<unknown> {
    const id = this.nextId++;

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.messageHandlers.delete(id);
        reject(new Error(`Tool call timeout: ${name}`));
      }, 30000);

      this.messageHandlers.set(id, (msg) => {
        clearTimeout(timeout);
        if (msg.error) {
          reject(new Error(`Tool ${name} failed: ${msg.error.message}`));
        } else {
          resolve(msg.result);
        }
      });

      this.sendMessage({
        jsonrpc: '2.0',
        id,
        method: 'tools/call',
        params: { name, arguments: args },
      });
    });
  }

  async callTools(tools: MCPToolCall[]): Promise<unknown[]> {
    const results: unknown[] = [];
    for (const tool of tools) {
      try {
        const result = await this.callTool(tool.name, tool.arguments);
        results.push({ success: true, name: tool.name, result });
      } catch (error) {
        results.push({
          success: false,
          name: tool.name,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    }
    return results;
  }

  private sendMessage(msg: MCPMessage): void {
    if (!this.process?.stdin) {
      throw new Error('MCP process not started');
    }
    this.process.stdin.write(`${JSON.stringify(msg)}\n`);
  }

  private processBuffer(): void {
    const lines = this.buffer.split('\n');
    this.buffer = lines.pop() || '';

    for (const line of lines) {
      if (!line.trim()) continue;
      try {
        const msg = JSON.parse(line) as MCPMessage;
        if (msg.id) {
          const handler = this.messageHandlers.get(msg.id);
          if (handler) {
            this.messageHandlers.delete(msg.id);
            handler(msg);
          }
        }
      } catch (e) {
        // Ignore parse errors
      }
    }
  }

  async stop(): Promise<void> {
    return new Promise((resolve) => {
      if (!this.process) {
        resolve();
        return;
      }

      this.process.on('exit', () => resolve());
      this.process.kill();

      // Force kill after 2 seconds
      setTimeout(() => {
        this.process?.kill('SIGKILL');
        resolve();
      }, 2000);
    });
  }
}

// Test data paths
export const TEST_FILES = {
  testFile: '/workspace/plugins/cclsp/playground/src/test-file.ts',
  errorsFile: '/workspace/plugins/cclsp/playground/src/errors-file.ts',
  componentsDir: '/workspace/plugins/cclsp/playground/src/components',
  userForm: '/workspace/plugins/cclsp/playground/src/components/user-form.ts',
} as const;

// Common test tool calls
export const QUICK_TESTS: MCPToolCall[] = [
  {
    name: 'find_definition',
    arguments: { file_path: TEST_FILES.testFile, symbol_name: '_calculateAge' },
  },
  {
    name: 'find_references',
    arguments: { file_path: TEST_FILES.testFile, symbol_name: 'TestProcessor' },
  },
  {
    name: 'get_diagnostics',
    arguments: { file_path: TEST_FILES.errorsFile },
  },
  {
    name: 'get_hover',
    arguments: { file_path: TEST_FILES.testFile, line: 13, character: 10 },
  },
  {
    name: 'rename_symbol',
    arguments: {
      file_path: TEST_FILES.testFile,
      symbol_name: 'TEST_CONSTANT',
      new_name: 'RENAMED_CONSTANT',
      dry_run: true,
    },
  },
];

// Comprehensive test suite covering all 28 MCP tools
export const ALL_TESTS: MCPToolCall[] = [
  // Core Tools (4)
  {
    name: 'find_definition',
    arguments: { file_path: TEST_FILES.testFile, symbol_name: '_calculateAge' },
  },
  {
    name: 'find_references',
    arguments: { file_path: TEST_FILES.testFile, symbol_name: 'TestProcessor' },
  },
  {
    name: 'rename_symbol',
    arguments: {
      file_path: TEST_FILES.testFile,
      symbol_name: 'TEST_CONSTANT',
      new_name: 'RENAMED_CONSTANT',
      dry_run: true,
    },
  },
  {
    name: 'rename_symbol_strict',
    arguments: {
      file_path: TEST_FILES.testFile,
      line: 59,
      character: 18,
      new_name: 'strictTest',
      dry_run: true,
    },
  },

  // Document Tools (7)
  { name: 'get_diagnostics', arguments: { file_path: TEST_FILES.errorsFile } },
  { name: 'get_document_symbols', arguments: { file_path: TEST_FILES.testFile } },
  {
    name: 'get_code_actions',
    arguments: {
      file_path: TEST_FILES.testFile,
      range: {
        start: { line: 8, character: 0 },
        end: { line: 8, character: 50 },
      },
    },
  },
  {
    name: 'format_document',
    arguments: {
      file_path: TEST_FILES.testFile,
      options: {
        tab_size: 2,
        insert_spaces: true,
      },
      dry_run: true,
    },
  },
  { name: 'search_workspace_symbols', arguments: { query: 'Process' } },
  { name: 'get_folding_ranges', arguments: { file_path: TEST_FILES.testFile } },
  { name: 'get_document_links', arguments: { file_path: TEST_FILES.testFile } },

  // Intelligence Tools (5)
  {
    name: 'get_hover',
    arguments: { file_path: TEST_FILES.testFile, line: 13, character: 10 },
  },
  {
    name: 'get_completions',
    arguments: { file_path: TEST_FILES.testFile, line: 26, character: 10 },
  },
  {
    name: 'get_signature_help',
    arguments: { file_path: TEST_FILES.testFile, line: 14, character: 20 },
  },
  {
    name: 'get_inlay_hints',
    arguments: {
      file_path: TEST_FILES.testFile,
      start_line: 10,
      start_character: 0,
      end_line: 20,
      end_character: 0,
    },
  },
  { name: 'get_semantic_tokens', arguments: { file_path: TEST_FILES.testFile } },

  // Hierarchy Tools (3)
  {
    name: 'prepare_call_hierarchy',
    arguments: { file_path: TEST_FILES.testFile, line: 13, character: 10 },
  },
  {
    name: 'prepare_type_hierarchy',
    arguments: { file_path: TEST_FILES.testFile, line: 18, character: 7 },
  },
  {
    name: 'get_selection_range',
    arguments: {
      file_path: TEST_FILES.testFile,
      positions: [{ line: 13, character: 10 }],
    },
  },

  // File Operations (3)
  {
    name: 'create_file',
    arguments: { file_path: '/tmp/cclsp-test.ts', content: '// Test file\nconsole.log("test");' },
  },
  {
    name: 'rename_file',
    arguments: { old_path: '/tmp/cclsp-test.ts', new_path: '/tmp/cclsp-renamed.ts', dry_run: true },
  },
  { name: 'delete_file', arguments: { file_path: '/tmp/cclsp-renamed.ts', dry_run: true } },

  // Server Management (1) - Test this last with longer timeout
  { name: 'restart_server', arguments: { extensions: ['ts', 'tsx'] } },

  // Advanced Workflow Operations (5) - Missing tools added
  {
    name: 'apply_workspace_edit',
    arguments: {
      changes: {
        '/tmp/cclsp-workspace-edit.ts': [
          {
            range: {
              start: { line: 0, character: 0 },
              end: { line: 0, character: 0 },
            },
            newText: '// Workspace edit test\nconst testVar = "edited";\n',
          },
        ],
      },
      validate_before_apply: true,
    },
  },
  {
    name: 'get_call_hierarchy_incoming_calls',
    arguments: {
      item: {
        name: 'calculateAge',
        kind: 12, // Function kind
        uri: 'file:///workspace/plugins/cclsp/playground/src/test-file.ts',
        range: {
          start: { line: 12, character: 0 },
          end: { line: 14, character: 1 },
        },
        selectionRange: {
          start: { line: 12, character: 9 },
          end: { line: 12, character: 20 },
        },
      },
    },
  },
  {
    name: 'get_call_hierarchy_outgoing_calls',
    arguments: {
      item: {
        name: 'calculateAge',
        kind: 12, // Function kind
        uri: 'file:///workspace/plugins/cclsp/playground/src/test-file.ts',
        range: {
          start: { line: 12, character: 0 },
          end: { line: 14, character: 1 },
        },
        selectionRange: {
          start: { line: 12, character: 9 },
          end: { line: 12, character: 20 },
        },
      },
    },
  },
  {
    name: 'get_type_hierarchy_supertypes',
    arguments: {
      item: {
        name: 'TestProcessor',
        kind: 5, // Class kind
        uri: 'file:///workspace/plugins/cclsp/playground/src/test-file.ts',
        range: {
          start: { line: 17, character: 0 },
          end: { line: 41, character: 1 },
        },
        selectionRange: {
          start: { line: 17, character: 6 },
          end: { line: 17, character: 19 },
        },
      },
    },
  },
  {
    name: 'get_type_hierarchy_subtypes',
    arguments: {
      item: {
        name: 'TestProcessor',
        kind: 5, // Class kind
        uri: 'file:///workspace/plugins/cclsp/playground/src/test-file.ts',
        range: {
          start: { line: 17, character: 0 },
          end: { line: 41, character: 1 },
        },
        selectionRange: {
          start: { line: 17, character: 6 },
          end: { line: 17, character: 19 },
        },
      },
    },
  },
];

// Playground tests - specific tests for playground functionality
export const PLAYGROUND_TESTS: MCPToolCall[] = [
  {
    name: 'get_diagnostics',
    arguments: { file_path: TEST_FILES.testFile },
  },
  {
    name: 'get_hover',
    arguments: {
      file_path: TEST_FILES.testFile,
      line: 13,
      character: 10,
    },
  },
  {
    name: 'find_references',
    arguments: {
      file_path: TEST_FILES.testFile,
      symbol_name: 'TestProcessor',
    },
  },
  {
    name: 'get_document_symbols',
    arguments: { file_path: TEST_FILES.testFile },
  },
];

// Intelligence tests - focusing on code intelligence features
export const INTELLIGENCE_TESTS: MCPToolCall[] = [
  {
    name: 'get_hover',
    arguments: {
      file_path: TEST_FILES.testFile,
      line: 13,
      character: 10,
    },
  },
  {
    name: 'get_completions',
    arguments: {
      file_path: TEST_FILES.testFile,
      line: 26,
      character: 10,
    },
  },
  {
    name: 'get_signature_help',
    arguments: {
      file_path: TEST_FILES.testFile,
      line: 14,
      character: 20,
    },
  },
  {
    name: 'get_inlay_hints',
    arguments: {
      file_path: TEST_FILES.testFile,
      start_line: 10,
      start_character: 0,
      end_line: 20,
      end_character: 0,
    },
  },
  {
    name: 'get_semantic_tokens',
    arguments: { file_path: TEST_FILES.testFile },
  },
];

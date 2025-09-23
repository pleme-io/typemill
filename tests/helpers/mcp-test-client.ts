import { type ChildProcess, spawn } from 'node:child_process';
import { EventEmitter } from 'node:events';
import { getSystemCapabilities } from './system-utils.js';

// Shared server instance for test suite
let sharedServerInstance: MCPTestClient | null = null;

// System capability detection now handled by shared utility

// Define message types for better type safety
interface MCPMessage {
  jsonrpc: string;
  id?: number | string;
  method?: string;
  result?: unknown;
  error?: unknown;
  params?: unknown;
}

// A message parser for newline-delimited JSON (NDJSON) as used by MCP protocol
function createMessageParser() {
  let buffer = '';
  const subscribers = new Set<(message: MCPMessage) => void>();

  function parse() {
    const lines = buffer.split('\n');
    buffer = lines.pop() || ''; // Keep incomplete line in buffer

    for (const line of lines) {
      if (!line.trim()) continue; // Skip empty lines

      try {
        const message = JSON.parse(line) as MCPMessage;
        for (const sub of subscribers) {
          sub(message);
        }
      } catch (e) {
        // Only log errors for lines that look like they should be JSON
        // (ignore log output that starts with timestamp or other non-JSON)
        if (line.trim().startsWith('{') || line.trim().startsWith('[')) {
          console.error('Failed to parse message JSON:', e);
        }
      }
    }
  }

  return {
    append: (data: string) => {
      buffer += data;
      parse();
    },
    subscribe: (callback: (message: MCPMessage) => void) => {
      subscribers.add(callback);
      return () => subscribers.delete(callback);
    },
  };
}

export class MCPTestClient {
  private process!: ChildProcess;
  private parser = createMessageParser();
  private responseEmitter = new EventEmitter();
  private static sharedMode = process.env.TEST_SHARED_SERVER === 'true';
  private isShared = false;
  private initPromise: Promise<void> | null = null;
  public isClosed = false;
  private static requestCounter = 0;
  private static instanceId = Math.floor(Math.random() * 10000);

  constructor() {
    this.parser.subscribe((message) => {
      if (message.id !== undefined) {
        // Handle both successful results and errors
        if (message.error) {
          this.responseEmitter.emit(message.id, { error: message.error });
        } else {
          // Ensure we always emit a valid result, even if it's undefined
          // This prevents the race condition where no event is emitted at all
          this.responseEmitter.emit(message.id, message.result);
        }
      }
    });
  }

  private generateUniqueId(): string {
    // Generate truly unique ID using timestamp + counter + instance ID + random
    const timestamp = Date.now();
    const counter = ++MCPTestClient.requestCounter;
    const random = Math.floor(Math.random() * 1000);
    return `${MCPTestClient.instanceId}-${timestamp}-${counter}-${random}`;
  }

  static getShared(): MCPTestClient {
    if (!sharedServerInstance) {
      sharedServerInstance = new MCPTestClient();
      sharedServerInstance.isShared = true;
    }
    return sharedServerInstance;
  }

  async start(options?: {
    timeout?: number;
    enablePreloading?: boolean;
    minimalConfig?: boolean;
    skipLSPPreload?: boolean;
  }): Promise<void> {
    // If already started (shared mode), return existing promise
    if (this.initPromise) {
      return this.initPromise;
    }

    this.initPromise = this._doStart(options);
    return this.initPromise;
  }

  private async _doStart(options?: {
    timeout?: number;
    enablePreloading?: boolean;
    minimalConfig?: boolean;
    skipLSPPreload?: boolean;
  }): Promise<void> {
    this.process = spawn(process.execPath, ['dist/index.js', 'start'], {
      cwd: process.cwd(),
      stdio: ['pipe', 'pipe', 'pipe'],
      env: {
        ...process.env,
        TEST_MODE: getSystemCapabilities().isSlowSystem ? 'slow' : 'fast',
        SKIP_LSP_PRELOAD: options?.skipLSPPreload !== false ? 'true' : undefined,
        TEST_MINIMAL_CONFIG: options?.minimalConfig ? 'true' : 'false',
        SKIP_PID_FILE: 'true', // Skip PID file management in tests
      },
    });

    this.process.on('error', (err) => console.error('MCP process error:', err));
    this.process.stderr?.on('data', (data) => process.stderr.write(data));
    this.process.stdout?.on('data', (data) => this.parser.append(data.toString()));

    return new Promise<void>((resolve, reject) => {
      const capabilities = getSystemCapabilities();
      const startupTimeout = capabilities.baseTimeout * capabilities.timeoutMultiplier;

      const timeout = setTimeout(() => {
        reject(new Error(`Test client startup timed out after ${startupTimeout / 1000} seconds.`));
      }, startupTimeout);

      this.process.stderr?.on('data', (data) => {
        if (data.toString().includes('Codebuddy Server running on stdio')) {
          // Send initialize request immediately after server starts
          const initRequest = `${JSON.stringify({
            jsonrpc: '2.0',
            id: 'init',
            method: 'initialize',
            params: {
              protocolVersion: '2024-11-05',
              capabilities: {},
              clientInfo: { name: 'mcp-test-client', version: '1.0.0' },
            },
          })}\n`;

          this.process.stdin?.write(initRequest);

          // Wait for initialize response before resolving
          const unsubscribe = this.parser.subscribe((message) => {
            if (message.id === 'init' && message.result) {
              unsubscribe();
              clearTimeout(timeout);
              resolve();
            }
          });
        }
      });
    });
  }

  async stop(): Promise<void> {
    // Don't stop shared server instances
    if (this.isShared) {
      console.log('⚠️ Keeping shared server alive for other tests');
      return;
    }
    this.isClosed = true;
    this.process?.kill('SIGTERM');
  }

  async close(): Promise<void> {
    this.isClosed = true;
    this.process?.kill('SIGTERM');
  }

  static async cleanup(): Promise<void> {
    if (sharedServerInstance) {
      sharedServerInstance.isClosed = true;
      sharedServerInstance.process?.kill('SIGTERM');
      sharedServerInstance = null;
    }
  }

  async callTool(name: string, args: Record<string, unknown>): Promise<unknown> {
    const id = this.generateUniqueId();
    const request = {
      jsonrpc: '2.0',
      id,
      method: 'tools/call',
      params: { name, arguments: args },
    };

    // MCP uses newline-delimited JSON, not Content-Length headers
    const requestString = `${JSON.stringify(request)}\n`;

    return new Promise((resolve, reject) => {
      const capabilities = getSystemCapabilities();
      const requestTimeout = capabilities.baseTimeout * capabilities.timeoutMultiplier;

      const timeout = setTimeout(() => {
        // Clean up listener on timeout
        this.responseEmitter.removeAllListeners(id);
        reject(new Error(`Request ${name} (${id}) timed out after ${requestTimeout / 1000}s.`));
      }, requestTimeout);

      // Add defensive check for response handling
      this.responseEmitter.once(id, (result) => {
        clearTimeout(timeout);

        // Handle error responses properly
        if (result && typeof result === 'object' && 'error' in result) {
          reject(new Error(`Tool error: ${JSON.stringify(result.error)}`));
        } else if (result === undefined) {
          reject(new Error(`Received undefined result for ${name} (${id})`));
        } else {
          resolve(result);
        }
      });

      if (!this.process || !this.process.stdin) {
        clearTimeout(timeout);
        this.responseEmitter.removeAllListeners(id);
        reject(new Error('Process not started or stdin not available'));
        return;
      }

      this.process.stdin.write(requestString, (err) => {
        if (err) {
          clearTimeout(timeout);
          this.responseEmitter.removeAllListeners(id);
          reject(err);
        }
      });
    });
  }

  async forceLSPReindex(extensions: string[] = ['ts', 'py', 'rs']): Promise<void> {
    console.log('🔄 Forcing LSP re-indexing for extensions:', extensions.join(', '));
    await this.callTool('restart_server', { extensions });
    await new Promise((resolve) => setTimeout(resolve, 1500)); // Wait for restart
    console.log('✅ LSP re-indexing complete');
  }

  async callTools(
    tests: Array<{ name: string; args: Record<string, unknown> }>
  ): Promise<Array<{ name: string; success: boolean; error?: string }>> {
    const results = [];
    for (const test of tests) {
      try {
        await this.callTool(test.name, test.args);
        results.push({ name: test.name, success: true });
      } catch (error) {
        results.push({
          name: test.name,
          success: false,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    }
    return results;
  }
}

// Quick tests configuration
export const QUICK_TESTS = [
  {
    name: 'find_definition',
    args: {
      file_path: '/workspace/examples/playground/src/test-file.ts',
      symbol_name: '_calculateAge',
    },
  },
  {
    name: 'find_references',
    args: {
      file_path: '/workspace/examples/playground/src/test-file.ts',
      symbol_name: 'TestProcessor',
    },
  },
  {
    name: 'get_diagnostics',
    args: {
      file_path: '/workspace/examples/playground/src/errors-file.ts',
    },
  },
  {
    name: 'get_hover',
    args: {
      file_path: '/workspace/examples/playground/src/test-file.ts',
      line: 13,
      character: 10,
    },
  },
];

// For now, ALL_TESTS is the same as QUICK_TESTS
// TODO: Add comprehensive tests for all 28+ tools
export const ALL_TESTS = QUICK_TESTS;

export function assertToolResult(
  result: unknown
): asserts result is { content: Array<{ type: 'text'; text: string }> } {
  if (!result || typeof result !== 'object') {
    console.error('Invalid tool result - not an object:', result);
    throw new Error('Invalid tool result format: expected object');
  }

  const resultObj = result as Record<string, unknown>;
  if (!resultObj.content) {
    console.error('Invalid tool result - no content property:', result);
    throw new Error('Invalid tool result format: missing content property');
  }

  if (!Array.isArray(resultObj.content)) {
    console.error('Invalid tool result - content not array:', result);
    throw new Error('Invalid tool result format: content must be array');
  }
}

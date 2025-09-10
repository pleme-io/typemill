import { type ChildProcess, spawn } from 'node:child_process';
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

// A robust message parser that handles Content-Length headers, adapted from the project's own LSP protocol parser.
function createMessageParser() {
  let buffer = '';
  const subscribers = new Set<(message: MCPMessage) => void>();

  function parse() {
    while (true) {
      const headerEndIndex = buffer.indexOf('\r\n\r\n');
      if (headerEndIndex === -1) break;

      const headers = buffer.substring(0, headerEndIndex);
      const contentLengthMatch = headers.match(/Content-Length: (\d+)/);

      if (!contentLengthMatch) {
        // Malformed header, discard and continue
        buffer = buffer.substring(headerEndIndex + 4);
        continue;
      }

      const contentLength = Number.parseInt(contentLengthMatch[1], 10);
      const messageStartIndex = headerEndIndex + 4;

      if (buffer.length < messageStartIndex + contentLength) break;

      const messageContent = buffer.substring(messageStartIndex, messageStartIndex + contentLength);
      buffer = buffer.substring(messageStartIndex + contentLength);

      try {
        const message = JSON.parse(messageContent) as MCPMessage;
        for (const sub of subscribers) {
          sub(message);
        }
      } catch (e) {
        console.error('Failed to parse message JSON:', e);
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
  private responseEmitter = new (require('node:events').EventEmitter)();
  private static sharedMode = process.env.TEST_SHARED_SERVER === 'true';
  private isShared = false;
  private initPromise: Promise<void> | null = null;
  public isClosed = false;

  constructor() {
    this.parser.subscribe((message) => {
      if (message.id !== undefined) {
        // Handle both successful results and errors
        if (message.error) {
          this.responseEmitter.emit(message.id, { error: message.error });
        } else {
          this.responseEmitter.emit(message.id, message.result);
        }
      }
    });
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
    this.process = spawn(process.execPath, ['dist/index.js'], {
      cwd: process.cwd(),
      stdio: ['pipe', 'pipe', 'pipe'],
      env: {
        ...process.env,
        TEST_MODE: getSystemCapabilities().isSlowSystem ? 'slow' : 'fast',
        SKIP_LSP_PRELOAD: options?.skipLSPPreload ? 'true' : 'false',
        TEST_MINIMAL_CONFIG: options?.minimalConfig ? 'true' : 'false',
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
          clearTimeout(timeout);
          resolve();
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
    const id = Math.floor(Math.random() * 100000);
    const request = {
      jsonrpc: '2.0',
      id,
      method: 'tools/call',
      params: { name, arguments: args },
    };

    const requestString = `Content-Length: ${Buffer.byteLength(JSON.stringify(request))}\r\n\r\n${JSON.stringify(request)}`;

    return new Promise((resolve, reject) => {
      const capabilities = getSystemCapabilities();
      const requestTimeout = capabilities.baseTimeout * capabilities.timeoutMultiplier;

      const timeout = setTimeout(() => {
        reject(new Error(`Request ${name} (${id}) timed out after ${requestTimeout / 1000}s.`));
      }, requestTimeout);

      this.responseEmitter.once(id, (result) => {
        clearTimeout(timeout);
        resolve(result);
      });

      if (!this.process || !this.process.stdin) {
        clearTimeout(timeout);
        reject(new Error('Process not started or stdin not available'));
        return;
      }

      this.process.stdin.write(requestString, (err) => {
        if (err) {
          clearTimeout(timeout);
          reject(err);
        }
      });
    });
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
      file_path: '/workspace/plugins/codebuddy/playground/src/test-file.ts',
      symbol_name: '_calculateAge',
    },
  },
  {
    name: 'find_references',
    args: {
      file_path: '/workspace/plugins/codebuddy/playground/src/test-file.ts',
      symbol_name: 'TestProcessor',
    },
  },
  {
    name: 'get_diagnostics',
    args: {
      file_path: '/workspace/plugins/codebuddy/playground/src/errors-file.ts',
    },
  },
  {
    name: 'get_hover',
    args: {
      file_path: '/workspace/plugins/codebuddy/playground/src/test-file.ts',
      line: 13,
      character: 10,
    },
  },
];

export function assertToolResult(
  result: unknown
): asserts result is { content: Array<{ type: 'text'; text: string }> } {
  if (!result || !result.content || !Array.isArray(result.content)) {
    console.error('Invalid tool result:', result);
    throw new Error('Invalid tool result format');
  }
}

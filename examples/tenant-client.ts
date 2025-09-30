/**
 * Modern CodeBuddy MCP Client Example
 *
 * This example demonstrates how to connect to CodeBuddy's WebSocket server
 * using the Model Context Protocol (MCP) format.
 *
 * The MCP protocol uses JSON-RPC 2.0 style messages for all communication.
 *
 * For the server implementation, see:
 * - rust/crates/cb-transport/src/ws.rs (WebSocket transport)
 * - rust/crates/cb-core/src/model/mcp.rs (MCP types)
 */

import WebSocket from 'ws';

// MCP Message Types (TypeScript interfaces matching Rust types)

interface McpRequest {
  jsonrpc: '2.0';
  id?: string | number;
  method: string;
  params?: unknown;
}

interface McpResponse {
  jsonrpc: '2.0';
  id?: string | number;
  result?: unknown;
  error?: McpError;
}

interface McpError {
  code: number;
  message: string;
  data?: unknown;
}

interface McpToolCall {
  name: string;
  arguments?: Record<string, unknown>;
}

interface InitializeParams {
  protocolVersion: string;
  capabilities: {
    roots?: { listChanged?: boolean };
    sampling?: Record<string, unknown>;
  };
  clientInfo: {
    name: string;
    version: string;
  };
}

interface InitializeResult {
  protocolVersion: string;
  capabilities: {
    logging?: Record<string, unknown>;
    prompts?: { listChanged?: boolean };
    resources?: { subscribe?: boolean; listChanged?: boolean };
    tools?: { listChanged?: boolean };
  };
  serverInfo: {
    name: string;
    version: string;
  };
}

/**
 * CodeBuddy MCP client for WebSocket connections
 */
class CodeBuddyMcpClient {
  private ws: WebSocket;
  private requestId = 1;
  private pendingRequests = new Map<string | number, {
    resolve: (value: unknown) => void;
    reject: (error: Error) => void;
  }>();

  constructor(serverUrl: string = 'ws://localhost:3000') {
    this.ws = new WebSocket(serverUrl);

    this.ws.on('open', () => {
      console.log('Connected to CodeBuddy MCP server');
    });

    this.ws.on('message', (data) => {
      this.handleMessage(data.toString());
    });

    this.ws.on('error', (error) => {
      console.error('WebSocket error:', error);
    });

    this.ws.on('close', () => {
      console.log('Disconnected from CodeBuddy MCP server');
      // Reject all pending requests
      this.pendingRequests.forEach(({ reject }) => {
        reject(new Error('Connection closed'));
      });
      this.pendingRequests.clear();
    });
  }

  /**
   * Wait for connection to be established
   */
  async waitForConnection(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws.readyState === WebSocket.OPEN) {
        resolve();
        return;
      }

      const onOpen = () => {
        this.ws.off('error', onError);
        resolve();
      };

      const onError = (error: Error) => {
        this.ws.off('open', onOpen);
        reject(error);
      };

      this.ws.once('open', onOpen);
      this.ws.once('error', onError);
    });
  }

  /**
   * Handle incoming MCP messages
   */
  private handleMessage(data: string): void {
    try {
      const message = JSON.parse(data) as McpResponse;

      if (message.id !== undefined && this.pendingRequests.has(message.id)) {
        const { resolve, reject } = this.pendingRequests.get(message.id)!;
        this.pendingRequests.delete(message.id);

        if (message.error) {
          reject(new Error(`MCP Error ${message.error.code}: ${message.error.message}`));
        } else {
          resolve(message.result);
        }
      }
    } catch (error) {
      console.error('Failed to parse message:', error);
    }
  }

  /**
   * Send an MCP request and wait for response
   */
  private async sendRequest<T = unknown>(method: string, params?: unknown): Promise<T> {
    const id = this.requestId++;
    const request: McpRequest = {
      jsonrpc: '2.0',
      id,
      method,
      params,
    };

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(id, { resolve, reject });

      this.ws.send(JSON.stringify(request), (error) => {
        if (error) {
          this.pendingRequests.delete(id);
          reject(error);
        }
      });

      // Timeout after 30 seconds
      setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id);
          reject(new Error('Request timeout'));
        }
      }, 30000);
    });
  }

  /**
   * Initialize MCP session
   */
  async initialize(clientName: string = 'example-client', clientVersion: string = '1.0.0'): Promise<InitializeResult> {
    const params: InitializeParams = {
      protocolVersion: '2025-06-18',
      capabilities: {
        roots: { listChanged: true },
        sampling: {},
      },
      clientInfo: {
        name: clientName,
        version: clientVersion,
      },
    };

    return this.sendRequest<InitializeResult>('initialize', params);
  }

  /**
   * List available tools
   */
  async listTools(): Promise<{ tools: Array<{ name: string; description: string; inputSchema?: unknown }> }> {
    return this.sendRequest('tools/list');
  }

  /**
   * Call a tool (e.g., LSP operations like find_definition, rename_symbol, etc.)
   */
  async callTool<T = unknown>(toolCall: McpToolCall): Promise<T> {
    return this.sendRequest<T>('tools/call', toolCall);
  }

  /**
   * Find definition of a symbol
   */
  async findDefinition(filePath: string, symbolName: string): Promise<unknown> {
    return this.callTool({
      name: 'find_definition',
      arguments: {
        file_path: filePath,
        symbol_name: symbolName,
      },
    });
  }

  /**
   * Find all references to a symbol
   */
  async findReferences(filePath: string, symbolName: string, includeDeclaration: boolean = true): Promise<unknown> {
    return this.callTool({
      name: 'find_references',
      arguments: {
        file_path: filePath,
        symbol_name: symbolName,
        include_declaration: includeDeclaration,
      },
    });
  }

  /**
   * Rename a symbol across the workspace
   */
  async renameSymbol(filePath: string, symbolName: string, newName: string, dryRun: boolean = false): Promise<unknown> {
    return this.callTool({
      name: 'rename_symbol',
      arguments: {
        file_path: filePath,
        symbol_name: symbolName,
        new_name: newName,
        dry_run: dryRun,
      },
    });
  }

  /**
   * Get hover information for a symbol
   */
  async getHover(filePath: string, line: number, character: number): Promise<unknown> {
    return this.callTool({
      name: 'get_hover',
      arguments: {
        file_path: filePath,
        line,
        character,
      },
    });
  }

  /**
   * Get document symbols (outline)
   */
  async getDocumentSymbols(filePath: string): Promise<unknown> {
    return this.callTool({
      name: 'get_document_symbols',
      arguments: {
        file_path: filePath,
      },
    });
  }

  /**
   * Search workspace symbols
   */
  async searchWorkspaceSymbols(query: string): Promise<unknown> {
    return this.callTool({
      name: 'search_workspace_symbols',
      arguments: {
        query,
      },
    });
  }

  /**
   * Format a document
   */
  async formatDocument(filePath: string, options?: { tabSize?: number; insertSpaces?: boolean }): Promise<unknown> {
    return this.callTool({
      name: 'format_document',
      arguments: {
        file_path: filePath,
        options,
      },
    });
  }

  /**
   * Get diagnostics (errors/warnings) for a file
   */
  async getDiagnostics(filePath: string): Promise<unknown> {
    return this.callTool({
      name: 'get_diagnostics',
      arguments: {
        file_path: filePath,
      },
    });
  }

  /**
   * Close the connection
   */
  disconnect(): void {
    this.ws.close();
  }
}

// Usage example
async function main() {
  const client = new CodeBuddyMcpClient('ws://localhost:3000');

  try {
    // Wait for connection
    await client.waitForConnection();
    console.log('✓ Connected');

    // Initialize MCP session
    const initResult = await client.initialize('example-client', '1.0.0');
    console.log('✓ Initialized:', initResult.serverInfo);

    // List available tools
    const tools = await client.listTools();
    console.log(`✓ Available tools: ${tools.tools.length}`);
    tools.tools.forEach(tool => {
      console.log(`  - ${tool.name}: ${tool.description}`);
    });

    // Example: Find definition of a symbol
    const definition = await client.findDefinition(
      '/workspace/examples/frontend/src/index.ts',
      'User'
    );
    console.log('✓ Found definition:', definition);

    // Example: Find references
    const references = await client.findReferences(
      '/workspace/examples/frontend/src/index.ts',
      'User',
      true
    );
    console.log('✓ Found references:', references);

    // Example: Get hover information
    const hover = await client.getHover(
      '/workspace/examples/frontend/src/index.ts',
      4,  // line number (0-indexed)
      17  // character position (0-indexed)
    );
    console.log('✓ Hover info:', hover);

    // Example: Search workspace symbols
    const symbols = await client.searchWorkspaceSymbols('User');
    console.log('✓ Workspace symbols:', symbols);

    // Example: Dry-run rename (preview changes without applying)
    const renamePreview = await client.renameSymbol(
      '/workspace/examples/frontend/src/index.ts',
      'User',
      'UserProfile',
      true  // dry run
    );
    console.log('✓ Rename preview:', renamePreview);

  } catch (error) {
    console.error('Error:', error);
  } finally {
    client.disconnect();
  }
}

// Run example if executed directly
if (require.main === module) {
  main().catch(console.error);
}

export { CodeBuddyMcpClient, McpRequest, McpResponse, McpError, McpToolCall };
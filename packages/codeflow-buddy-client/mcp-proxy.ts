import { WebSocketClient, type WebSocketClientOptions } from './websocket.js';

export interface ProxyOptions extends Omit<WebSocketClientOptions, 'reconnect'> {
  autoConnect?: boolean;
}

export interface MCPToolCall {
  method: string;
  params?: unknown;
}

export interface MCPToolResponse<T = unknown> {
  result?: T;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

/**
 * High-level MCP proxy client for programmatic use.
 * Provides a simplified API for interacting with the Codeflow Buddy server.
 */
export class MCPProxy {
  private client: WebSocketClient;
  private connectPromise?: Promise<void>;
  private url: string;
  private options: ProxyOptions;

  constructor(url: string, options: ProxyOptions = {}) {
    this.url = url;
    this.options = {
      autoConnect: true,
      ...options,
    };

    // Always enable reconnect for the proxy
    this.client = new WebSocketClient(url, {
      ...options,
      reconnect: true,
    });

    // Forward events from the WebSocket client
    this.client.on('connected', () => this.emit('connected'));
    this.client.on('disconnected', (info) => this.emit('disconnected', info));
    this.client.on('reconnecting', (info) => this.emit('reconnecting', info));
    this.client.on('error', (error) => this.emit('error', error));
    this.client.on('notification', (notification) => this.emit('notification', notification));
    this.client.on('status', (status) => this.emit('status', status));
  }

  // Event emitter methods (simplified delegation)
  private eventHandlers = new Map<string, Set<(...args: any[]) => void>>();

  on(event: string, handler: (...args: any[]) => void): this {
    if (!this.eventHandlers.has(event)) {
      this.eventHandlers.set(event, new Set());
    }
    this.eventHandlers.get(event)?.add(handler);
    return this;
  }

  off(event: string, handler: (...args: any[]) => void): this {
    this.eventHandlers.get(event)?.delete(handler);
    return this;
  }

  private emit(event: string, ...args: any[]): void {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      for (const handler of handlers) {
        try {
          handler(...args);
        } catch (error) {
          console.error(`Error in event handler for '${event}':`, error);
        }
      }
    }
  }

  /**
   * Ensures the client is connected before sending requests.
   */
  private async ensureConnected(): Promise<void> {
    if (this.client.isConnected()) {
      return;
    }

    // If already connecting, wait for that
    if (this.connectPromise) {
      return this.connectPromise;
    }

    // Start new connection
    this.connectPromise = this.client.connect();
    try {
      await this.connectPromise;
    } finally {
      this.connectPromise = undefined;
    }
  }

  /**
   * Send an MCP tool call to the server.
   * Automatically handles connection management.
   */
  async send<T = unknown>(call: MCPToolCall): Promise<T> {
    await this.ensureConnected();
    return this.client.send<T>(call.method, call.params);
  }

  /**
   * Send multiple MCP tool calls in parallel.
   * Returns results in the same order as the calls.
   */
  async sendBatch<T = unknown>(calls: MCPToolCall[]): Promise<MCPToolResponse<T>[]> {
    await this.ensureConnected();

    const promises = calls.map((call) =>
      this.client
        .send(call.method, call.params)
        .then((result) => ({ result }) as MCPToolResponse<T>)
        .catch(
          (error) =>
            ({
              error: {
                code: error.code || -32603,
                message: error.message,
                data: error.data,
              },
            }) as MCPToolResponse<T>
        )
    );

    return Promise.all(promises);
  }

  /**
   * List available tools from the server.
   */
  async listTools(): Promise<any> {
    return this.send({ method: 'tools/list' });
  }

  /**
   * Get the current connection status.
   */
  get status(): string {
    return this.client.status;
  }

  /**
   * Check if the client is currently connected.
   */
  isConnected(): boolean {
    return this.client.isConnected();
  }

  /**
   * Manually connect to the server.
   * Note: Connection is usually automatic on first send().
   */
  async connect(): Promise<void> {
    return this.client.connect();
  }

  /**
   * Disconnect from the server.
   */
  async disconnect(): Promise<void> {
    return this.client.disconnect();
  }

  /**
   * Create an HTTP proxy server that forwards requests to the WebSocket.
   * Useful for integrating with tools that only support HTTP.
   */
  createHttpProxy(port: number = 3001): any {
    // Lazy load to avoid importing express if not needed
    const { createProxyServer } = require('./http-proxy.js');
    return createProxyServer(this, port);
  }
}

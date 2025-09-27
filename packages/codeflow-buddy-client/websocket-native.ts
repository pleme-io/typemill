// WebSocket implementation using Bun's native WebSocket for testing
import { randomUUID } from 'node:crypto';
import { EventEmitter } from 'node:events';

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

export interface WebSocketClientOptions {
  token?: string;
  reconnect?: boolean;
  reconnectInterval?: number;
  reconnectMaxRetries?: number;
  requestTimeout?: number;
  headers?: Record<string, string>;
}

export interface MCPRequest {
  jsonrpc: '2.0';
  method: string;
  params?: unknown;
  id?: string;
}

export interface MCPResponse {
  jsonrpc: '2.0';
  result?: unknown;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
  id: string;
}

interface PendingRequest {
  resolve: (result: unknown) => void;
  reject: (error: Error) => void;
  timeout: NodeJS.Timeout;
}

export class WebSocketClient extends EventEmitter {
  private ws?: WebSocket;
  private url: string;
  private options: WebSocketClientOptions;
  private pendingRequests = new Map<string, PendingRequest>();
  private _status: ConnectionStatus = 'disconnected';

  constructor(url: string, options: WebSocketClientOptions = {}) {
    super();
    this.url = url;
    this.options = options;
  }

  get status(): ConnectionStatus {
    return this._status;
  }

  isConnected(): boolean {
    return this._status === 'connected';
  }

  async connect(): Promise<void> {
    if (this._status === 'connected') {
      return;
    }

    this._status = 'connecting';
    this.emit('status', 'connecting');

    return new Promise((resolve, reject) => {
      // Use Bun's native WebSocket
      this.ws = new WebSocket(this.url);

      this.ws.onopen = () => {
        this._status = 'connected';
        this.emit('status', 'connected');
        this.emit('connected');
        resolve();
      };

      this.ws.onerror = (error) => {
        this._status = 'disconnected';
        this.emit('status', 'disconnected');
        this.emit('error', error);
        reject(new Error('Connection failed'));
      };

      this.ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data as string);
          this.handleMessage(message);
        } catch (error) {
          console.error('Failed to parse message:', error);
        }
      };

      this.ws.onclose = () => {
        this._status = 'disconnected';
        this.emit('status', 'disconnected');
        this.emit('disconnected', { code: 1000, reason: 'Closed' });

        // Clear pending requests
        for (const [_id, pending] of this.pendingRequests.entries()) {
          clearTimeout(pending.timeout);
          pending.reject(new Error('Connection closed'));
        }
        this.pendingRequests.clear();
      };
    });
  }

  async disconnect(): Promise<void> {
    if (this.ws) {
      this.ws.close();
      this.ws = undefined;
    }
    this._status = 'disconnected';
    this.emit('status', 'disconnected');
  }

  async send(method: string, params?: unknown): Promise<unknown> {
    if (!this.isConnected()) {
      await this.connect();
    }

    const id = randomUUID();
    const request: MCPRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id,
    };

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error('Request timeout'));
      }, this.options.requestTimeout || 30000);

      this.pendingRequests.set(id, {
        resolve,
        reject,
        timeout,
      });

      this.ws!.send(JSON.stringify(request));
    });
  }

  private handleMessage(message: MCPResponse): void {
    if (message.id && this.pendingRequests.has(message.id)) {
      const pending = this.pendingRequests.get(message.id)!;
      clearTimeout(pending.timeout);
      this.pendingRequests.delete(message.id);

      if (message.error) {
        pending.reject(new Error(message.error.message));
      } else {
        pending.resolve(message.result);
      }
    } else if (!message.id) {
      // It's a notification
      this.emit('notification', message);
    }
  }

  once(event: string, listener: (...args: any[]) => void): this {
    super.once(event, listener);
    return this;
  }

  on(event: string, listener: (...args: any[]) => void): this {
    super.on(event, listener);
    return this;
  }

  off(event: string, listener: (...args: any[]) => void): this {
    super.off(event, listener);
    return this;
  }
}

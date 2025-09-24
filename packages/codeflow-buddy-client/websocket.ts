import { randomUUID } from 'node:crypto';
import { EventEmitter } from 'node:events';
import WebSocket from 'ws';

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

export interface WebSocketClientOptions {
  token?: string;
  reconnect?: boolean;
  reconnectInterval?: number;
  reconnectMaxRetries?: number;
  requestTimeout?: number;
  headers?: Record<string, string>;
}

interface WebSocketClientInternalOptions {
  token?: string;
  reconnect: boolean;
  reconnectInterval: number;
  reconnectMaxRetries: number;
  requestTimeout: number;
  headers: Record<string, string>;
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

export interface MCPNotification {
  jsonrpc: '2.0';
  method: string;
  params?: unknown;
}

interface PendingRequest {
  resolve: (result: unknown) => void;
  reject: (error: Error) => void;
  timeout: NodeJS.Timeout;
}

export class WebSocketClient extends EventEmitter {
  private ws?: WebSocket;
  private url: string;
  private options: WebSocketClientInternalOptions;
  private pendingRequests = new Map<string, PendingRequest>();
  private _status: ConnectionStatus = 'disconnected';
  private reconnectAttempts = 0;
  private reconnectTimer?: NodeJS.Timeout;
  private isManualDisconnect = false;

  constructor(url: string, options: WebSocketClientOptions = {}) {
    super();
    this.url = url;
    this.options = {
      token: options.token,
      reconnect: options.reconnect ?? true,
      reconnectInterval: options.reconnectInterval ?? 1000,
      reconnectMaxRetries: options.reconnectMaxRetries ?? 10,
      requestTimeout: options.requestTimeout ?? 30000,
      headers: options.headers ?? {},
    };
  }

  get status(): ConnectionStatus {
    return this._status;
  }

  private setStatus(status: ConnectionStatus): void {
    if (this._status !== status) {
      this._status = status;
      this.emit('status', status);
    }
  }

  async connect(): Promise<void> {
    if (this._status === 'connected') {
      return;
    }

    if (this._status === 'connecting') {
      // Wait for existing connection attempt
      return new Promise((resolve, reject) => {
        const onConnect = () => {
          this.off('error', onError);
          resolve();
        };
        const onError = (err: Error) => {
          this.off('connected', onConnect);
          reject(err);
        };
        this.once('connected', onConnect);
        this.once('error', onError);
      });
    }

    this.isManualDisconnect = false;
    this.setStatus('connecting');

    return new Promise((resolve, reject) => {
      const headers = { ...this.options.headers };
      if (this.options.token) {
        headers.Authorization = `Bearer ${this.options.token}`;
      }

      this.ws = new WebSocket(this.url, { headers });

      const onOpen = () => {
        cleanup();
        this.setStatus('connected');
        this.reconnectAttempts = 0;
        this.setupWebSocketHandlers();
        this.emit('connected');
        resolve();
      };

      const onError = (err: Error) => {
        cleanup();
        this.setStatus('disconnected');
        this.emit('error', err);
        reject(err);
        this.handleReconnection();
      };

      const cleanup = () => {
        this.ws?.off('open', onOpen);
        this.ws?.off('error', onError);
      };

      this.ws.once('open', onOpen);
      this.ws.once('error', onError);
    });
  }

  private setupWebSocketHandlers(): void {
    if (!this.ws) return;

    this.ws.on('message', (data: WebSocket.RawData) => {
      try {
        const message = JSON.parse(data.toString());
        this.handleMessage(message);
      } catch (error) {
        this.emit('error', new Error(`Failed to parse message: ${error}`));
      }
    });

    this.ws.on('close', (code: number, reason: Buffer) => {
      this.setStatus('disconnected');
      this.emit('disconnected', { code, reason: reason.toString() });

      // Clear all pending requests
      for (const [_id, pending] of this.pendingRequests.entries()) {
        clearTimeout(pending.timeout);
        pending.reject(new Error('Connection closed'));
      }
      this.pendingRequests.clear();

      if (!this.isManualDisconnect) {
        this.handleReconnection();
      }
    });

    this.ws.on('error', (error: Error) => {
      this.emit('error', error);
    });

    this.ws.on('ping', () => {
      this.ws?.pong();
    });
  }

  private handleMessage(message: MCPResponse | MCPNotification): void {
    // Check if it's a response to a request
    if ('id' in message && message.id) {
      const pending = this.pendingRequests.get(message.id);
      if (pending) {
        clearTimeout(pending.timeout);
        this.pendingRequests.delete(message.id);

        if (message.error) {
          const error = new Error(message.error.message);
          Object.assign(error, { code: message.error.code, data: message.error.data });
          pending.reject(error);
        } else {
          pending.resolve(message.result);
        }
      }
    } else {
      // It's a notification from the server
      this.emit('notification', message);
    }
  }

  private handleReconnection(): void {
    if (!this.options.reconnect || this.isManualDisconnect) {
      return;
    }

    if (this.reconnectAttempts >= this.options.reconnectMaxRetries) {
      this.emit('error', new Error('Max reconnection attempts reached'));
      return;
    }

    const delay = Math.min(
      this.options.reconnectInterval * 2 ** this.reconnectAttempts,
      30000 // Max 30 seconds
    );

    this.reconnectAttempts++;
    this.setStatus('reconnecting');
    this.emit('reconnecting', { attempt: this.reconnectAttempts, delay });

    this.reconnectTimer = setTimeout(() => {
      this.connect().catch(() => {
        // Error already handled in connect()
      });
    }, delay);
  }

  async send<T = unknown>(method: string, params?: unknown): Promise<T> {
    if (this._status !== 'connected') {
      if (this._status === 'disconnected') {
        await this.connect();
      } else {
        // Wait for connection
        await new Promise<void>((resolve, reject) => {
          const onConnect = () => {
            this.off('error', onError);
            resolve();
          };
          const onError = (err: Error) => {
            this.off('connected', onConnect);
            reject(err);
          };
          this.once('connected', onConnect);
          this.once('error', onError);
        });
      }
    }

    return new Promise<T>((resolve, reject) => {
      const id = randomUUID();

      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request timeout for method: ${method}`));
      }, this.options.requestTimeout);

      this.pendingRequests.set(id, {
        resolve: resolve as (result: unknown) => void,
        reject,
        timeout,
      });

      const request: MCPRequest = {
        jsonrpc: '2.0',
        method,
        params,
        id,
      };

      this.ws?.send(JSON.stringify(request), (error) => {
        if (error) {
          clearTimeout(timeout);
          this.pendingRequests.delete(id);
          reject(error);
        }
      });
    });
  }

  async disconnect(): Promise<void> {
    this.isManualDisconnect = true;

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }

    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      return new Promise((resolve) => {
        this.ws?.once('close', () => {
          this.ws = undefined;
          resolve();
        });
        this.ws?.close();
      });
    }

    this.ws = undefined;
    this.setStatus('disconnected');
  }

  isConnected(): boolean {
    return this._status === 'connected' && this.ws?.readyState === WebSocket.OPEN;
  }
}

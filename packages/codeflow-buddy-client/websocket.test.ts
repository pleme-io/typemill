import { afterEach, beforeEach, describe, expect, it } from 'bun:test';
import type WebSocket from 'ws';
import { WebSocketServer } from 'ws';
import type { MCPRequest, MCPResponse } from './websocket.js';
import { WebSocketClient } from './websocket.js';

describe('WebSocketClient', () => {
  let server: WebSocketServer;
  let serverUrl: string;
  const serverClients: Set<WebSocket> = new Set();

  beforeEach((done) => {
    // Create mock WebSocket server
    server = new WebSocketServer({ port: 0 }, () => {
      const address = server.address() as any;
      serverUrl = `ws://localhost:${address.port}`;
      done();
    });

    server.on('connection', (ws, req) => {
      serverClients.add(ws);

      // Check for auth header
      const authHeader = req.headers.authorization;
      if (authHeader && !authHeader.includes('test-token')) {
        ws.close(1008, 'Invalid token');
        return;
      }

      ws.on('message', (data) => {
        try {
          const request: MCPRequest = JSON.parse(data.toString());

          // Mock responses based on method
          if (request.method === 'test') {
            const response: MCPResponse = {
              jsonrpc: '2.0',
              result: { success: true, echo: request.params },
              id: request.id!,
            };
            ws.send(JSON.stringify(response));
          } else if (request.method === 'error') {
            const response: MCPResponse = {
              jsonrpc: '2.0',
              error: {
                code: -32000,
                message: 'Test error',
                data: request.params,
              },
              id: request.id!,
            };
            ws.send(JSON.stringify(response));
          } else if (request.method === 'slow') {
            // Simulate slow response
            setTimeout(() => {
              const response: MCPResponse = {
                jsonrpc: '2.0',
                result: { delayed: true },
                id: request.id!,
              };
              ws.send(JSON.stringify(response));
            }, 100);
          } else if (request.method === 'no-response') {
            // Don't send any response (for timeout testing)
          }
        } catch (_error) {
          // Invalid JSON
        }
      });

      ws.on('close', () => {
        serverClients.delete(ws);
      });
    });
  });

  afterEach(async () => {
    // Close all connections
    for (const client of serverClients) {
      client.close();
    }
    serverClients.clear();

    // Close server
    await new Promise<void>((resolve) => {
      server.close(() => resolve());
    });
  });

  describe('connection management', () => {
    it('should connect successfully', async () => {
      const client = new WebSocketClient(serverUrl);
      await client.connect();
      expect(client.status).toBe('connected');
      expect(client.isConnected()).toBe(true);
      await client.disconnect();
    });

    it('should handle connection with auth token', async () => {
      const client = new WebSocketClient(serverUrl, { token: 'test-token' });
      await client.connect();
      expect(client.status).toBe('connected');
      await client.disconnect();
    });

    it('should reject invalid auth token', async () => {
      const client = new WebSocketClient(serverUrl, { token: 'invalid-token' });
      await expect(client.connect()).rejects.toThrow();
      expect(client.status).toBe('disconnected');
    });

    it('should handle multiple connect calls gracefully', async () => {
      const client = new WebSocketClient(serverUrl);

      // Start multiple connections simultaneously
      const promises = [client.connect(), client.connect(), client.connect()];

      await Promise.all(promises);
      expect(client.status).toBe('connected');
      await client.disconnect();
    });

    it('should disconnect cleanly', async () => {
      const client = new WebSocketClient(serverUrl);
      await client.connect();
      await client.disconnect();
      expect(client.status).toBe('disconnected');
      expect(client.isConnected()).toBe(false);
    });
  });

  describe('request/response', () => {
    it('should send request and receive response', async () => {
      const client = new WebSocketClient(serverUrl);
      await client.connect();

      const result = await client.send('test', { message: 'hello' });
      expect(result).toEqual({ success: true, echo: { message: 'hello' } });

      await client.disconnect();
    });

    it('should handle error responses', async () => {
      const client = new WebSocketClient(serverUrl);
      await client.connect();

      await expect(client.send('error', { test: 'data' })).rejects.toThrow('Test error');

      await client.disconnect();
    });

    it('should handle request timeout', async () => {
      const client = new WebSocketClient(serverUrl, { requestTimeout: 50 });
      await client.connect();

      await expect(client.send('no-response')).rejects.toThrow('Request timeout');

      await client.disconnect();
    });

    it('should handle multiple concurrent requests', async () => {
      const client = new WebSocketClient(serverUrl);
      await client.connect();

      const promises = [
        client.send('test', { id: 1 }),
        client.send('test', { id: 2 }),
        client.send('slow', { id: 3 }),
        client.send('test', { id: 4 }),
      ];

      const results = await Promise.all(promises);
      expect(results).toHaveLength(4);
      expect(results[0]).toEqual({ success: true, echo: { id: 1 } });
      expect(results[2]).toEqual({ delayed: true });

      await client.disconnect();
    });

    it('should auto-connect on first send', async () => {
      const client = new WebSocketClient(serverUrl);
      expect(client.status).toBe('disconnected');

      const result = await client.send('test', { auto: true });
      expect(client.status).toBe('connected');
      expect(result).toEqual({ success: true, echo: { auto: true } });

      await client.disconnect();
    });
  });

  describe('reconnection', () => {
    it('should reconnect after connection loss', async () => {
      const client = new WebSocketClient(serverUrl, {
        reconnect: true,
        reconnectInterval: 10,
        reconnectMaxRetries: 3,
      });

      await client.connect();
      expect(client.status).toBe('connected');

      // Force close the connection from server side
      const serverClient = Array.from(serverClients)[0];
      serverClient.close();

      // Wait for reconnection
      await new Promise((resolve) => {
        client.once('reconnecting', () => {
          expect(client.status).toBe('reconnecting');
        });
        client.once('connected', () => {
          expect(client.status).toBe('connected');
          resolve(undefined);
        });
      });

      await client.disconnect();
    });

    it('should not reconnect after manual disconnect', async () => {
      const client = new WebSocketClient(serverUrl, {
        reconnect: true,
        reconnectInterval: 10,
      });

      await client.connect();
      await client.disconnect();

      // Wait a bit to ensure no reconnection happens
      await new Promise((resolve) => setTimeout(resolve, 50));
      expect(client.status).toBe('disconnected');
    });

    it('should respect max reconnection attempts', async () => {
      // Use a non-existent port to force connection failures
      const badClient = new WebSocketClient('ws://localhost:59999', {
        reconnect: true,
        reconnectInterval: 10,
        reconnectMaxRetries: 2,
      });

      const errors: Error[] = [];
      badClient.on('error', (err) => errors.push(err));

      await expect(badClient.connect()).rejects.toThrow();

      // Wait for max retries
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Should have emitted error for max retries
      const maxRetriesError = errors.find((e) => e.message.includes('Max reconnection attempts'));
      expect(maxRetriesError).toBeDefined();
    });
  });

  describe('notifications', () => {
    it('should emit server notifications', async () => {
      const client = new WebSocketClient(serverUrl);
      await client.connect();

      const notifications: any[] = [];
      client.on('notification', (notif) => notifications.push(notif));

      // Send notification from server
      const serverClient = Array.from(serverClients)[0];
      serverClient.send(
        JSON.stringify({
          jsonrpc: '2.0',
          method: 'server.notification',
          params: { test: 'data' },
        })
      );

      // Wait for notification to be received
      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(notifications).toHaveLength(1);
      expect(notifications[0]).toEqual({
        jsonrpc: '2.0',
        method: 'server.notification',
        params: { test: 'data' },
      });

      await client.disconnect();
    });
  });

  describe('event emitter', () => {
    it('should emit status changes', async () => {
      const client = new WebSocketClient(serverUrl);
      const statuses: string[] = [];

      client.on('status', (status) => statuses.push(status));

      await client.connect();
      await client.disconnect();

      expect(statuses).toContain('connecting');
      expect(statuses).toContain('connected');
      expect(statuses).toContain('disconnected');
    });

    it('should emit connected event', async () => {
      const client = new WebSocketClient(serverUrl);
      let connected = false;

      client.on('connected', () => {
        connected = true;
      });

      await client.connect();
      expect(connected).toBe(true);

      await client.disconnect();
    });

    it('should emit disconnected event with details', async () => {
      const client = new WebSocketClient(serverUrl);
      await client.connect();

      const disconnectPromise = new Promise<any>((resolve) => {
        client.on('disconnected', (info) => resolve(info));
      });

      await client.disconnect();
      const info = await disconnectPromise;

      expect(info).toHaveProperty('code');
      expect(info).toHaveProperty('reason');
    });
  });
});

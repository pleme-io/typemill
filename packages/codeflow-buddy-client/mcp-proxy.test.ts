import { afterEach, beforeEach, describe, expect, it } from 'bun:test';
import { WebSocketServer } from 'ws';
import { MCPProxy } from './mcp-proxy.js';
import type { MCPRequest, MCPResponse } from './websocket.js';

describe('MCPProxy', () => {
  let server: WebSocketServer;
  let serverUrl: string;

  beforeEach((done) => {
    // Create mock WebSocket server
    server = new WebSocketServer({ port: 0 }, () => {
      const address = server.address() as any;
      serverUrl = `ws://localhost:${address.port}`;
      done();
    });

    server.on('connection', (ws) => {
      ws.on('message', (data) => {
        try {
          const request: MCPRequest = JSON.parse(data.toString());

          // Mock responses
          if (request.method === 'tools/list') {
            const response: MCPResponse = {
              jsonrpc: '2.0',
              result: {
                tools: [
                  { name: 'test-tool-1', description: 'Test tool 1' },
                  { name: 'test-tool-2', description: 'Test tool 2' },
                ],
              },
              id: request.id!,
            };
            ws.send(JSON.stringify(response));
          } else if (request.method === 'test-tool') {
            const response: MCPResponse = {
              jsonrpc: '2.0',
              result: { executed: true, params: request.params },
              id: request.id!,
            };
            ws.send(JSON.stringify(response));
          } else if (request.method === 'error-tool') {
            const response: MCPResponse = {
              jsonrpc: '2.0',
              error: {
                code: -32601,
                message: 'Method not found',
              },
              id: request.id!,
            };
            ws.send(JSON.stringify(response));
          }
        } catch (_error) {
          // Invalid JSON
        }
      });
    });
  });

  afterEach(async () => {
    await new Promise<void>((resolve) => {
      server.close(() => resolve());
    });
  });

  describe('basic functionality', () => {
    it('should create proxy instance', () => {
      const proxy = new MCPProxy(serverUrl);
      expect(proxy).toBeDefined();
      expect(proxy.status).toBe('disconnected');
    });

    it('should auto-connect on first send', async () => {
      const proxy = new MCPProxy(serverUrl);
      expect(proxy.status).toBe('disconnected');

      const result = await proxy.send({
        method: 'test-tool',
        params: { test: 'data' },
      });

      expect(proxy.isConnected()).toBe(true);
      expect(result).toEqual({ executed: true, params: { test: 'data' } });

      await proxy.disconnect();
    });

    it('should handle manual connect', async () => {
      const proxy = new MCPProxy(serverUrl, { autoConnect: false });

      await proxy.connect();
      expect(proxy.isConnected()).toBe(true);

      await proxy.disconnect();
    });

    it('should list tools', async () => {
      const proxy = new MCPProxy(serverUrl);

      const tools = await proxy.listTools();
      expect(tools).toHaveProperty('tools');
      expect(tools.tools).toHaveLength(2);
      expect(tools.tools[0].name).toBe('test-tool-1');

      await proxy.disconnect();
    });
  });

  describe('batch operations', () => {
    it('should send multiple requests in batch', async () => {
      const proxy = new MCPProxy(serverUrl);

      const calls = [
        { method: 'test-tool', params: { id: 1 } },
        { method: 'test-tool', params: { id: 2 } },
        { method: 'error-tool' },
        { method: 'test-tool', params: { id: 3 } },
      ];

      const results = await proxy.sendBatch(calls);

      expect(results).toHaveLength(4);
      expect(results[0].result).toEqual({ executed: true, params: { id: 1 } });
      expect(results[1].result).toEqual({ executed: true, params: { id: 2 } });
      expect(results[2].error).toBeDefined();
      expect(results[2].error?.message).toBe('Method not found');
      expect(results[3].result).toEqual({ executed: true, params: { id: 3 } });

      await proxy.disconnect();
    });

    it('should handle all errors in batch gracefully', async () => {
      const proxy = new MCPProxy(serverUrl);

      const calls = [{ method: 'error-tool' }, { method: 'error-tool' }];

      const results = await proxy.sendBatch(calls);

      expect(results).toHaveLength(2);
      expect(results[0].error).toBeDefined();
      expect(results[1].error).toBeDefined();

      await proxy.disconnect();
    });
  });

  describe('event forwarding', () => {
    it('should forward connection events', async () => {
      const proxy = new MCPProxy(serverUrl);
      const events: string[] = [];

      proxy.on('connected', () => events.push('connected'));
      proxy.on('disconnected', () => events.push('disconnected'));
      proxy.on('status', (status) => events.push(`status:${status}`));

      await proxy.connect();
      await proxy.disconnect();

      expect(events).toContain('connected');
      expect(events).toContain('disconnected');
      expect(events).toContain('status:connected');
      expect(events).toContain('status:disconnected');
    });

    it('should handle multiple event listeners', async () => {
      const proxy = new MCPProxy(serverUrl);
      let count = 0;

      const handler1 = () => count++;
      const handler2 = () => count++;

      proxy.on('connected', handler1);
      proxy.on('connected', handler2);

      await proxy.connect();
      expect(count).toBe(2);

      // Remove one handler
      proxy.off('connected', handler1);
      count = 0;

      await proxy.disconnect();
      await proxy.connect();
      expect(count).toBe(1);

      await proxy.disconnect();
    });
  });

  describe('connection management', () => {
    it('should reuse existing connection', async () => {
      const proxy = new MCPProxy(serverUrl);

      // Multiple sends should use same connection
      await proxy.send({ method: 'test-tool' });
      const wasConnected = proxy.isConnected();

      await proxy.send({ method: 'test-tool' });
      expect(proxy.isConnected()).toBe(wasConnected);

      await proxy.disconnect();
    });

    it('should handle concurrent connection attempts', async () => {
      const proxy = new MCPProxy(serverUrl);

      // Start multiple operations simultaneously
      const promises = [
        proxy.send({ method: 'test-tool', params: { id: 1 } }),
        proxy.send({ method: 'test-tool', params: { id: 2 } }),
        proxy.send({ method: 'test-tool', params: { id: 3 } }),
      ];

      const results = await Promise.all(promises);
      expect(results).toHaveLength(3);
      expect(proxy.isConnected()).toBe(true);

      await proxy.disconnect();
    });
  });
});

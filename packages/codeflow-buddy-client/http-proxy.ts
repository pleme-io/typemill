import type { Server } from 'node:http';
import express from 'express';
import type { MCPProxy } from './mcp-proxy.js';

/**
 * Create an HTTP proxy server that forwards requests to the WebSocket MCP server.
 * This allows HTTP-only clients to interact with the WebSocket-based server.
 */
export function createProxyServer(proxy: MCPProxy, _port: number = 3001): Server {
  const app = express();

  // Middleware
  app.use(express.json({ limit: '10mb' }));

  // CORS headers for browser-based clients
  app.use((req, res, next) => {
    res.header('Access-Control-Allow-Origin', '*');
    res.header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.header('Access-Control-Allow-Headers', 'Content-Type, Authorization');
    if (req.method === 'OPTIONS') {
      return res.sendStatus(200);
    }
    next();
  });

  // Health check endpoint
  app.get('/health', (_req, res) => {
    const status = proxy.isConnected() ? 'connected' : 'disconnected';
    res.json({
      status: 'ok',
      connection: status,
      timestamp: new Date().toISOString(),
    });
  });

  // Main RPC endpoint
  app.post('/rpc', async (req, res) => {
    const { method, params } = req.body;

    if (!method) {
      return res.status(400).json({
        error: {
          code: -32600,
          message: 'Invalid Request: missing method',
        },
      });
    }

    try {
      const result = await proxy.send({ method, params });
      res.json({ result });
    } catch (error: unknown) {
      const errorObj = error as { code?: number; message: string; data?: unknown };
      res.status(500).json({
        error: {
          code: errorObj.code || -32603,
          message: errorObj.message || 'Internal error',
          data: errorObj.data,
        },
      });
    }
  });

  // Batch RPC endpoint
  app.post('/rpc/batch', async (req, res) => {
    if (!Array.isArray(req.body)) {
      return res.status(400).json({
        error: {
          code: -32600,
          message: 'Invalid Request: expected array',
        },
      });
    }

    try {
      const results = await proxy.sendBatch(req.body);
      res.json(results);
    } catch (error: unknown) {
      const errorObj = error as { code?: number; message: string; data?: unknown };
      res.status(500).json({
        error: {
          code: errorObj.code || -32603,
          message: errorObj.message || 'Internal error',
          data: errorObj.data,
        },
      });
    }
  });

  // List tools endpoint
  app.get('/tools', async (_req, res) => {
    try {
      const tools = await proxy.listTools();
      res.json(tools);
    } catch (error: unknown) {
      const errorObj = error as { code?: number; message?: string };
      res.status(500).json({
        error: {
          code: errorObj.code || -32603,
          message: errorObj.message || 'Internal server error',
        },
      });
    }
  });

  // 404 handler
  app.use((_req, res) => {
    res.status(404).json({
      error: {
        code: -32601,
        message: 'Method not found',
      },
    });
  });

  return app.listen() as Server;
}

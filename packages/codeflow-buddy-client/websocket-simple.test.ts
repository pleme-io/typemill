import { describe, expect, it } from 'bun:test';
import { WebSocketServer } from 'ws';
import { WebSocketClient } from './websocket';

describe('WebSocketClient Simple', () => {
  it('should connect and disconnect', async () => {
    // Create server
    const port = 50000 + Math.floor(Math.random() * 10000);
    const server = await new Promise<WebSocketServer>((resolve) => {
      const s = new WebSocketServer({ port }, () => resolve(s));
    });

    const url = `ws://localhost:${port}`;

    // Connect client
    const client = new WebSocketClient(url);
    await client.connect();
    expect(client.status).toBe('connected');

    // Disconnect
    await client.disconnect();
    expect(client.status).toBe('disconnected');

    // Close server
    await new Promise<void>((resolve) => {
      server.close(() => resolve());
    });
  });
});

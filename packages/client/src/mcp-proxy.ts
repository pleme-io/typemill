// MCP Proxy Implementation

import type { MCPRequest, MCPResponse } from './types.js';
import { WebSocketClient } from './websocket.js';

export class MCPProxy {
  private wsClient: WebSocketClient;

  constructor(wsUrl: string) {
    this.wsClient = new WebSocketClient(wsUrl);
  }

  async send(request: MCPRequest): Promise<MCPResponse> {
    // TODO: Implement proxy logic
    throw new Error('Not implemented');
  }
}

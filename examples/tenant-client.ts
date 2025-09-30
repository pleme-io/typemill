/**
 * DEPRECATED: This example is outdated and does not match the current WebSocket API.
 *
 * The current CodeBuddy WebSocket transport uses the MCP (Model Context Protocol) format
 * for all communication, not the FUSE-specific operations shown here.
 *
 * For the actual WebSocket implementation, see:
 * - rust/crates/cb-transport/src/ws.rs (Rust server implementation)
 * - MCP protocol documentation
 *
 * This file is kept for historical reference only.
 *
 * @deprecated Use MCP protocol via WebSocket transport instead
 */

import WebSocket from 'ws';

/**
 * @deprecated This client does not match the current API
 */
class TenantFuseClient {
  private ws: WebSocket;
  private tenantId: string;
  private apiKey: string;

  constructor(serviceUrl: string, tenantId: string, apiKey: string) {
    this.tenantId = tenantId;
    this.apiKey = apiKey;
    this.ws = new WebSocket(serviceUrl);

    this.ws.on('open', () => {
      // Authenticate
      this.send({
        type: 'auth',
        tenantId: this.tenantId,
        apiKey: this.apiKey,
      });
    });

    this.ws.on('message', (data) => {
      const msg = JSON.parse(data.toString());
      if (msg.type === 'workspace-ready') {
        console.log(`FUSE mount ready at: ${msg.mountPath}`);
        console.log(`Workspace ID: ${msg.workspaceId}`);
        // Now tenant can access their isolated filesystem
      }
    });
  }

  private send(data: object) {
    this.ws.send(JSON.stringify(data));
  }

  // FUSE operations
  async readFile(path: string): Promise<Buffer> {
    return new Promise((resolve, reject) => {
      const requestId = Math.random().toString(36);

      const handler = (data: WebSocket.Data) => {
        const msg = JSON.parse(data.toString());
        if (msg.requestId === requestId) {
          this.ws.off('message', handler);
          if (msg.error) {
            reject(new Error(msg.error));
          } else {
            resolve(Buffer.from(msg.data, 'base64'));
          }
        }
      };

      this.ws.on('message', handler);
      this.send({
        type: 'fuse-read',
        path,
        requestId,
      });
    });
  }

  async writeFile(path: string, content: Buffer): Promise<void> {
    return new Promise((resolve, reject) => {
      const requestId = Math.random().toString(36);

      const handler = (data: WebSocket.Data) => {
        const msg = JSON.parse(data.toString());
        if (msg.requestId === requestId) {
          this.ws.off('message', handler);
          if (msg.error) {
            reject(new Error(msg.error));
          } else {
            resolve();
          }
        }
      };

      this.ws.on('message', handler);
      this.send({
        type: 'fuse-write',
        path,
        data: content.toString('base64'),
        requestId,
      });
    });
  }

  disconnect() {
    this.ws.close();
  }
}

// Usage example
async function main() {
  const client = new TenantFuseClient('ws://localhost:3000', 'tenant-123', 'secret-api-key');

  // Wait for connection
  await new Promise((resolve) => setTimeout(resolve, 1000));

  // Use the FUSE filesystem
  await client.writeFile('/project/README.md', Buffer.from('# My Project'));
  const content = await client.readFile('/project/README.md');
  console.log('File content:', content.toString());

  client.disconnect();
}

main().catch(console.error);

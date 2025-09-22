import type { LSPClient } from '../lsp/client.js';
import type { ServerState } from '../lsp/types.js';
import type { LSPServerConfig } from '../types.js';

export interface PooledLSPServer extends ServerState {
  projectId: string;
  language: string;
  lastUsed: Date;
  refCount: number;
  isRestarting?: boolean;
  crashCount: number;
}

export interface PendingRequest {
  id: string;
  method: string;
  params: any;
  resolve: (result: any) => void;
  reject: (error: Error) => void;
  timestamp: Date;
  retryCount: number;
}

export class LSPServerPool {
  private pools = new Map<string, PooledLSPServer>();
  private lspClient: LSPClient;
  private idleTimeoutMs = 60000; // 60 seconds idle timeout
  private pendingRequests = new Map<string, PendingRequest[]>(); // serverKey -> pending requests
  private readonly MAX_RETRIES = 3;
  private readonly CRASH_RESTART_DELAY_MS = 2000; // 2 seconds delay before restart

  constructor(lspClient: LSPClient) {
    this.lspClient = lspClient;
    this.startIdleCleanup();
  }

  async getServer(
    projectId: string,
    extension: string,
    workspaceDir?: string
  ): Promise<PooledLSPServer> {
    const language = this.getLanguageFromExtension(extension);
    // Include workspace directory in key for isolation
    const key = workspaceDir
      ? `${projectId}:${language}:${workspaceDir}`
      : `${projectId}:${language}`;

    let server = this.pools.get(key);

    if (!server) {
      // Create new server for this project/language combination
      const lspServer = await this.lspClient.getServer(extension, workspaceDir);

      server = {
        ...lspServer,
        projectId,
        language,
        lastUsed: new Date(),
        refCount: 0,
        crashCount: 0,
      };

      this.pools.set(key, server);

      // Set up crash monitoring
      this.setupCrashMonitoring(key, server);
    }

    // Check if server is currently restarting
    if (server.isRestarting) {
      // Wait for restart to complete
      await this.waitForRestart(key);
      server = this.pools.get(key)!;
    }

    // Update usage tracking
    server.lastUsed = new Date();
    server.refCount++;

    return server;
  }

  releaseServer(projectId: string, extension: string): void {
    const language = this.getLanguageFromExtension(extension);
    const key = `${projectId}:${language}`;

    const server = this.pools.get(key);
    if (server && server.refCount > 0) {
      server.refCount--;
      server.lastUsed = new Date();
    }
  }

  async restartServer(projectId: string, extension: string): Promise<void> {
    const language = this.getLanguageFromExtension(extension);
    const key = `${projectId}:${language}`;

    const server = this.pools.get(key);
    if (server) {
      // Use LSP client's restart method
      await this.lspClient.restartServer([extension]);
      this.pools.delete(key);

      // Force creation of new server on next request
    }
  }

  getActiveServers(): Array<{
    projectId: string;
    language: string;
    refCount: number;
    lastUsed: Date;
  }> {
    return Array.from(this.pools.values()).map((server) => ({
      projectId: server.projectId,
      language: server.language,
      refCount: server.refCount,
      lastUsed: server.lastUsed,
    }));
  }

  async shutdown(): Promise<void> {
    // Reject all pending requests
    for (const [serverKey, requests] of this.pendingRequests.entries()) {
      for (const request of requests) {
        request.reject(new Error('Server shutting down'));
      }
    }
    this.pendingRequests.clear();

    // Dispose the LSP client which will handle shutting down all servers
    await this.lspClient.dispose();
    this.pools.clear();

    console.log('LSP server pool shutdown complete');
  }

  /**
   * Set up crash monitoring for an LSP server
   */
  private setupCrashMonitoring(serverKey: string, server: PooledLSPServer): void {
    server.process.on('exit', (code, signal) => {
      if (code !== 0 && code !== null) {
        console.error(
          `LSP server crashed for ${serverKey} (exit code: ${code}, signal: ${signal})`
        );
        this.handleServerCrash(serverKey, server);
      }
    });

    server.process.on('error', (error) => {
      console.error(`LSP server error for ${serverKey}:`, error);
      this.handleServerCrash(serverKey, server);
    });
  }

  /**
   * Handle LSP server crash with auto-restart and request replay
   */
  private async handleServerCrash(
    serverKey: string,
    crashedServer: PooledLSPServer
  ): Promise<void> {
    crashedServer.crashCount++;

    console.log(`Handling crash for ${serverKey} (crash count: ${crashedServer.crashCount})`);

    // Don't restart if we've crashed too many times
    if (crashedServer.crashCount > this.MAX_RETRIES) {
      console.error(
        `LSP server ${serverKey} has crashed ${crashedServer.crashCount} times. Not restarting.`
      );

      // Reject all pending requests
      const pendingRequests = this.pendingRequests.get(serverKey) || [];
      for (const request of pendingRequests) {
        request.reject(new Error('LSP server crashed too many times'));
      }
      this.pendingRequests.delete(serverKey);

      this.pools.delete(serverKey);
      return;
    }

    // Mark as restarting
    crashedServer.isRestarting = true;

    try {
      // Wait before restarting to avoid rapid restart loops
      await new Promise((resolve) => setTimeout(resolve, this.CRASH_RESTART_DELAY_MS));

      // Get the extension for this language
      const extension = this.getExtensionFromLanguage(crashedServer.language);

      // Restart the server
      console.log(`Restarting LSP server for ${serverKey}...`);
      await this.lspClient.restartServer([extension]);

      // Get the new server instance
      const newLspServer = await this.lspClient.getServer(extension);

      // Update the pooled server with new process
      const newServer: PooledLSPServer = {
        ...newLspServer,
        projectId: crashedServer.projectId,
        language: crashedServer.language,
        lastUsed: new Date(),
        refCount: crashedServer.refCount,
        crashCount: crashedServer.crashCount,
        isRestarting: false,
      };

      this.pools.set(serverKey, newServer);

      // Set up crash monitoring for the new server
      this.setupCrashMonitoring(serverKey, newServer);

      console.log(`LSP server ${serverKey} restarted successfully`);

      // Replay pending requests
      await this.replayPendingRequests(serverKey, newServer);
    } catch (error) {
      console.error(`Failed to restart LSP server ${serverKey}:`, error);

      // Reject all pending requests
      const pendingRequests = this.pendingRequests.get(serverKey) || [];
      for (const request of pendingRequests) {
        request.reject(
          new Error(
            `Failed to restart LSP server: ${error instanceof Error ? error.message : 'Unknown error'}`
          )
        );
      }
      this.pendingRequests.delete(serverKey);

      // Remove from pool
      this.pools.delete(serverKey);
    }
  }

  /**
   * Replay pending requests after server restart
   */
  private async replayPendingRequests(serverKey: string, server: PooledLSPServer): Promise<void> {
    const pendingRequests = this.pendingRequests.get(serverKey) || [];

    if (pendingRequests.length === 0) {
      return;
    }

    console.log(`Replaying ${pendingRequests.length} pending requests for ${serverKey}`);

    // Clear the pending requests list
    this.pendingRequests.delete(serverKey);

    // Replay each request
    for (const request of pendingRequests) {
      try {
        // Increase retry count
        request.retryCount++;

        if (request.retryCount > this.MAX_RETRIES) {
          request.reject(new Error(`Max retries exceeded for request ${request.method}`));
          continue;
        }

        // Send the request to the new server
        const result = await this.lspClient.sendRequest(server, request.method, request.params);
        request.resolve(result);
      } catch (error) {
        // Re-queue for retry if not max retries
        if (request.retryCount < this.MAX_RETRIES) {
          this.addPendingRequest(serverKey, request);
        } else {
          request.reject(
            new Error(
              `Request failed after restart: ${error instanceof Error ? error.message : 'Unknown error'}`
            )
          );
        }
      }
    }
  }

  /**
   * Add a request to the pending queue
   */
  private addPendingRequest(serverKey: string, request: PendingRequest): void {
    if (!this.pendingRequests.has(serverKey)) {
      this.pendingRequests.set(serverKey, []);
    }
    this.pendingRequests.get(serverKey)?.push(request);
  }

  /**
   * Wait for server restart to complete
   */
  private async waitForRestart(serverKey: string): Promise<void> {
    const maxWaitTime = 30000; // 30 seconds max wait
    const checkInterval = 100; // Check every 100ms
    let waitTime = 0;

    while (waitTime < maxWaitTime) {
      const server = this.pools.get(serverKey);
      if (!server || !server.isRestarting) {
        return;
      }

      await new Promise((resolve) => setTimeout(resolve, checkInterval));
      waitTime += checkInterval;
    }

    throw new Error(`Timeout waiting for server restart: ${serverKey}`);
  }

  /**
   * Send request with crash handling
   */
  async sendRequest(serverKey: string, method: string, params: any): Promise<any> {
    const server = this.pools.get(serverKey);
    if (!server) {
      throw new Error(`Server not found: ${serverKey}`);
    }

    if (server.isRestarting) {
      // Queue the request for replay after restart
      return new Promise((resolve, reject) => {
        const request: PendingRequest = {
          id: Math.random().toString(36),
          method,
          params,
          resolve,
          reject,
          timestamp: new Date(),
          retryCount: 0,
        };
        this.addPendingRequest(serverKey, request);
      });
    }
    return await this.lspClient.sendRequest(server, method, params);
  }

  private getLanguageFromExtension(extension: string): string {
    // Map file extensions to language identifiers
    const languageMap: Record<string, string> = {
      ts: 'typescript',
      tsx: 'typescript',
      js: 'javascript',
      jsx: 'javascript',
      py: 'python',
      go: 'go',
      rs: 'rust',
      java: 'java',
      c: 'c',
      cpp: 'cpp',
      cs: 'csharp',
      php: 'php',
    };

    return languageMap[extension] || extension;
  }

  private getExtensionFromLanguage(language: string): string {
    // Reverse mapping - pick primary extension for each language
    const extensionMap: Record<string, string> = {
      typescript: 'ts',
      javascript: 'js',
      python: 'py',
      go: 'go',
      rust: 'rs',
      java: 'java',
      c: 'c',
      cpp: 'cpp',
      csharp: 'cs',
      php: 'php',
    };

    return extensionMap[language] || language;
  }

  private startIdleCleanup(): void {
    setInterval(() => {
      const now = new Date();
      const toRemove: string[] = [];

      for (const [key, server] of this.pools.entries()) {
        const idleTime = now.getTime() - server.lastUsed.getTime();

        // Remove servers that are idle and have no active references
        if (idleTime > this.idleTimeoutMs && server.refCount === 0) {
          toRemove.push(key);
        }
      }

      // Stop and remove idle servers
      for (const key of toRemove) {
        const server = this.pools.get(key);
        if (server) {
          this.lspClient
            .restartServer([this.getExtensionFromLanguage(server.language)])
            .catch((error: any) => console.error(`Error stopping idle server ${key}:`, error));
          this.pools.delete(key);
        }
      }
    }, 30000); // Check every 30 seconds
  }
}

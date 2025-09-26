import type { LSPClient } from '../../../@codeflow/features/src/lsp/lsp-client.js';
import type { ServerManager } from '../lsp/server-manager.js';
import type { ServerState } from '../lsp/types.js';

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
  // True pooling system properties
  private serverPool = new Map<string, PooledLSPServer[]>(); // language -> array of servers
  private leasedServers = new Map<string, PooledLSPServer>(); // serverKey -> server
  private maxServersPerLanguage = 2; // Max 2 servers per language (e.g., 2 for TS, 2 for Python)

  // Core dependencies
  private lspClient: LSPClient;
  private serverManager: ServerManager;
  private idleTimeoutMs = 60000; // 60 seconds idle timeout
  private pendingRequests = new Map<string, PendingRequest[]>(); // serverKey -> pending requests
  private readonly MAX_RETRIES = 3;
  private readonly CRASH_RESTART_DELAY_MS = 2000; // 2 seconds delay before restart

  constructor(lspClient: LSPClient) {
    this.lspClient = lspClient;
    this.serverManager = lspClient.serverManager;
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

    // Check if we already have a leased server for this specific key
    let server = this.leasedServers.get(key);
    if (server) {
      // Check if server is currently restarting
      if (server.isRestarting) {
        await this.waitForRestart(key);
        server = this.leasedServers.get(key)!;
      }
      // Update usage tracking
      server.lastUsed = new Date();
      server.refCount++;
      return server;
    }

    // Try to lease an available server from the pool
    const pool = this.serverPool.get(language) || [];

    // Look for an idle server in the pool
    const idleServer = pool.find((s) => s.refCount === 0 && !s.isRestarting);

    if (idleServer) {
      // Lease the idle server
      server = {
        ...idleServer,
        projectId,
        lastUsed: new Date(),
        refCount: 1,
      };

      // Remove from pool and add to leased servers
      const serverIndex = pool.indexOf(idleServer);
      pool.splice(serverIndex, 1);
      this.leasedServers.set(key, server);

      return server;
    }

    // If no idle server and pool is not at capacity, create a new one
    if (pool.length < this.maxServersPerLanguage) {
      const dummyFilePath = `dummy.${extension}`;
      const lspServer = await this.serverManager.spawnServerProcess(
        dummyFilePath,
        this.getServerManagerConfig(),
        workspaceDir
      );

      server = {
        ...lspServer,
        projectId,
        language,
        lastUsed: new Date(),
        refCount: 1,
        crashCount: 0,
      };

      // Add to leased servers
      this.leasedServers.set(key, server);

      // Set up crash monitoring
      this.setupCrashMonitoring(key, server);

      return server;
    }

    // Pool is full - wait for a server to be released
    // This is a simplified implementation - in production you might want a more sophisticated queuing system
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error(`Timeout waiting for available ${language} server`));
      }, 30000); // 30 second timeout

      const checkForAvailableServer = async () => {
        const updatedPool = this.serverPool.get(language) || [];
        const availableServer = updatedPool.find((s) => s.refCount === 0 && !s.isRestarting);

        if (availableServer) {
          clearTimeout(timeout);
          try {
            const result = await this.getServer(projectId, extension, workspaceDir);
            resolve(result);
          } catch (error) {
            reject(error);
          }
        } else {
          setTimeout(checkForAvailableServer, 1000); // Check every second
        }
      };

      setTimeout(checkForAvailableServer, 1000);
    });
  }

  releaseServer(projectId: string, extension: string, workspaceDir?: string): void {
    const language = this.getLanguageFromExtension(extension);
    const key = workspaceDir
      ? `${projectId}:${language}:${workspaceDir}`
      : `${projectId}:${language}`;

    const server = this.leasedServers.get(key);
    if (!server) return;

    // Decrease reference count
    server.refCount--;
    server.lastUsed = new Date();

    // If no more references, return server to pool
    if (server.refCount === 0) {
      // Remove from leased servers
      this.leasedServers.delete(key);

      // Reset project-specific information
      const pooledServer: PooledLSPServer = {
        ...server,
        projectId: '', // Clear project assignment
        refCount: 0,
        lastUsed: new Date(),
      };

      // Add back to pool
      const pool = this.serverPool.get(language) || [];
      pool.push(pooledServer);
      this.serverPool.set(language, pool);
    }
  }

  async restartServer(projectId: string, extension: string, workspaceDir?: string): Promise<void> {
    const language = this.getLanguageFromExtension(extension);
    const key = workspaceDir
      ? `${projectId}:${language}:${workspaceDir}`
      : `${projectId}:${language}`;

    // Check if server is currently leased
    const leasedServer = this.leasedServers.get(key);
    if (leasedServer) {
      // Use LSP client's restart method
      await this.lspClient.restartServer([extension]);
      this.leasedServers.delete(key);
      return;
    }

    // Check if any servers in the pool need restarting
    const pool = this.serverPool.get(language) || [];
    for (let i = pool.length - 1; i >= 0; i--) {
      const server = pool[i];
      if (server.projectId === projectId || !projectId) {
        // Use LSP client's restart method
        await this.lspClient.restartServer([extension]);
        pool.splice(i, 1);
      }
    }
  }

  getActiveServers(): Array<{
    projectId: string;
    language: string;
    refCount: number;
    lastUsed: Date;
    status: 'leased' | 'pooled';
  }> {
    const active: Array<{
      projectId: string;
      language: string;
      refCount: number;
      lastUsed: Date;
      status: 'leased' | 'pooled';
    }> = [];

    // Add leased servers
    for (const server of this.leasedServers.values()) {
      active.push({
        projectId: server.projectId,
        language: server.language,
        refCount: server.refCount,
        lastUsed: server.lastUsed,
        status: 'leased',
      });
    }

    // Add pooled servers
    for (const [language, pool] of this.serverPool.entries()) {
      for (const server of pool) {
        active.push({
          projectId: server.projectId || '(available)',
          language,
          refCount: server.refCount,
          lastUsed: server.lastUsed,
          status: 'pooled',
        });
      }
    }

    return active;
  }

  async shutdown(): Promise<void> {
    // Reject all pending requests
    for (const [_serverKey, requests] of this.pendingRequests.entries()) {
      for (const request of requests) {
        request.reject(new Error('Server shutting down'));
      }
    }
    this.pendingRequests.clear();

    // Dispose the LSP client which will handle shutting down all servers
    await this.lspClient.dispose();

    // Clear all pool data structures
    this.serverPool.clear();
    this.leasedServers.clear();

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

      // Remove from both leased servers and pool
      this.leasedServers.delete(serverKey);
      this.removeServerFromPool(crashedServer);
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
      const dummyFilePath = `dummy.${extension}`;
      const newLspServer = await this.serverManager.spawnServerProcess(
        dummyFilePath,
        this.getServerManagerConfig()
      );

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

      // Update the server reference in the appropriate location
      const isLeased = this.leasedServers.has(serverKey);
      if (isLeased) {
        this.leasedServers.set(serverKey, newServer);
      } else {
        // Replace in pool
        this.removeServerFromPool(crashedServer);
        const pool = this.serverPool.get(newServer.language) || [];
        pool.push(newServer);
        this.serverPool.set(newServer.language, pool);
      }

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

      // Remove from both leased servers and pool
      this.leasedServers.delete(serverKey);
      this.removeServerFromPool(crashedServer);
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
      // Check leased servers first
      let server = this.leasedServers.get(serverKey);

      // If not leased, check pools
      if (!server) {
        for (const [_language, pool] of this.serverPool.entries()) {
          server = pool.find((s) => s.projectId && serverKey.includes(s.projectId));
          if (server) break;
        }
      }

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
    // Check if server is leased
    let server = this.leasedServers.get(serverKey);

    if (!server) {
      // Check if it's in any pool
      for (const [_language, pool] of this.serverPool.entries()) {
        server = pool.find((s) => s.projectId && serverKey.includes(s.projectId));
        if (server) break;
      }
    }

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

  /**
   * Remove a server from the pool (helper method for crash handling)
   */
  private removeServerFromPool(server: PooledLSPServer): void {
    const pool = this.serverPool.get(server.language) || [];
    const serverIndex = pool.indexOf(server);
    if (serverIndex >= 0) {
      pool.splice(serverIndex, 1);
    }
  }

  /**
   * Get the server manager config (delegated through LSPClient)
   */
  private getServerManagerConfig() {
    // Access the config through the LSPClient's private field
    // This is a temporary solution - in a real refactor, we'd pass the config more cleanly
    return (this.lspClient as any).config;
  }

  private startIdleCleanup(): void {
    setInterval(() => {
      const now = new Date();

      // Clean up idle servers from pools
      for (const [language, pool] of this.serverPool.entries()) {
        const serversToRemove: PooledLSPServer[] = [];

        for (const server of pool) {
          const idleTime = now.getTime() - server.lastUsed.getTime();

          // Remove servers that are idle and have no active references
          if (idleTime > this.idleTimeoutMs && server.refCount === 0) {
            serversToRemove.push(server);
          }
        }

        // Stop and remove idle servers
        for (const server of serversToRemove) {
          const serverIndex = pool.indexOf(server);
          if (serverIndex >= 0) {
            pool.splice(serverIndex, 1);
            this.lspClient
              .restartServer([this.getExtensionFromLanguage(server.language)])
              .catch((error: any) =>
                console.error(`Error stopping idle ${language} server:`, error)
              );
          }
        }
      }
    }, 30000); // Check every 30 seconds
  }
}

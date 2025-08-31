import { type ChildProcess, spawn } from 'node:child_process';
import type { ServerCapabilities } from '../capability-manager.js';
import { capabilityManager } from '../capability-manager.js';
import type { ServerState } from '../lsp-types.js';
import { pathToUri } from '../path-utils.js';
import type { Config, LSPServerConfig } from '../types.js';
import type { LSPProtocol } from './protocol.js';

/**
 * Manages LSP server processes and lifecycle
 * Handles server spawning, initialization, restart timers
 */
export class ServerManager {
  private servers: Map<string, ServerState> = new Map();
  private serversStarting: Map<string, Promise<ServerState>> = new Map();
  private failedServers: Set<string> = new Set();
  private protocol: LSPProtocol;

  constructor(protocol: LSPProtocol) {
    this.protocol = protocol;
  }

  /**
   * Get access to the servers map (for workspace operations)
   */
  get activeServers(): Map<string, ServerState> {
    return this.servers;
  }

  /**
   * Get or start LSP server for a file
   */
  async getServer(filePath: string, config: Config): Promise<ServerState> {
    const serverConfig = this.getServerForFile(filePath, config);
    if (!serverConfig) {
      throw new Error(`No language server configured for file: ${filePath}`);
    }

    const serverKey = JSON.stringify(serverConfig.command);

    // Don't try to start servers that have already failed
    if (this.failedServers.has(serverKey)) {
      throw new Error(
        `Language server for ${serverConfig.extensions.join(', ')} files is not available. ` +
          `Install it with: ${this.getInstallInstructions(serverConfig.command[0] || '')}`
      );
    }

    // Return existing server if available
    const existingServer = this.servers.get(serverKey);
    if (existingServer) {
      if (!existingServer.process.killed) {
        await existingServer.initializationPromise;
        return existingServer;
      }
      // Server was killed, remove it and start a new one
      this.servers.delete(serverKey);
    }

    // Return ongoing startup promise if server is starting
    const startingPromise = this.serversStarting.get(serverKey);
    if (startingPromise) {
      return await startingPromise;
    }

    // Start new server
    const startupPromise = this.startServer(serverConfig);
    this.serversStarting.set(serverKey, startupPromise);

    try {
      const serverState = await startupPromise;
      this.servers.set(serverKey, serverState);
      return serverState;
    } finally {
      this.serversStarting.delete(serverKey);
    }
  }

  /**
   * Clear failed servers to allow retry
   */
  clearFailedServers(): void {
    const count = this.failedServers.size;
    this.failedServers.clear();
    if (count > 0) {
      process.stderr.write(
        `Cleared ${count} failed server(s). They will be retried on next access.\n`
      );
    }
  }

  /**
   * Restart servers for specified extensions
   */
  async restartServer(extensions?: string[], config?: Config): Promise<string[]> {
    const restartedServers: string[] = [];

    if (!extensions || extensions.length === 0) {
      // Restart all servers - fix iterator invalidation by collecting entries first
      const serversToRestart = Array.from(this.servers.entries());
      for (const [serverKey, serverState] of serversToRestart) {
        this.killServer(serverState);
        this.servers.delete(serverKey);
        restartedServers.push(serverState.config?.command?.join(' ') || 'unknown');
      }
    } else {
      // Restart servers for specific extensions - fix iterator invalidation
      const serversToRestart = Array.from(this.servers.entries()).filter(([, serverState]) => {
        const serverConfig = serverState.config;
        return serverConfig && extensions.some((ext) => serverConfig.extensions.includes(ext));
      });

      for (const [serverKey, serverState] of serversToRestart) {
        this.killServer(serverState);
        this.servers.delete(serverKey);
        restartedServers.push(serverState.config?.command.join(' '));
      }
    }

    return restartedServers;
  }

  /**
   * Preload servers for detected file types
   */
  async preloadServers(config: Config, extensions: string[]): Promise<void> {
    const serverConfigs = new Map<string, LSPServerConfig>();

    for (const extension of extensions) {
      const serverConfig = this.getServerForFile(`dummy.${extension}`, config);
      if (serverConfig) {
        const key = JSON.stringify(serverConfig.command);
        serverConfigs.set(key, serverConfig);
      }
    }

    const startPromises = Array.from(serverConfigs.values()).map(async (serverConfig) => {
      try {
        await this.startServer(serverConfig);
        process.stderr.write(`Preloaded server: ${serverConfig.command.join(' ')}\n`);
      } catch (error) {
        process.stderr.write(
          `Failed to preload server ${serverConfig.command.join(' ')}: ${error}\n`
        );
      }
    });

    await Promise.allSettled(startPromises);
  }

  /**
   * Find server config for file extension
   */
  private getServerForFile(filePath: string, config: Config): LSPServerConfig | null {
    const extension = filePath.split('.').pop();
    if (!extension) return null;

    process.stderr.write(`Looking for server for extension: ${extension}\n`);
    const server = config.servers.find((server) => server.extensions.includes(extension));

    if (server) {
      process.stderr.write(`Found server for ${extension}: ${server.command.join(' ')}\n`);
    } else {
      process.stderr.write(`No server found for extension: ${extension}\n`);
    }

    return server || null;
  }

  /**
   * Start a new LSP server process
   */
  private async startServer(serverConfig: LSPServerConfig): Promise<ServerState> {
    const [command, ...args] = serverConfig.command;
    if (!command) {
      throw new Error('No command specified in server config');
    }

    // For npx commands, provide helpful error message if npm is not installed
    if (command === 'npx') {
      try {
        const { execSync } = await import('node:child_process');
        execSync('npm --version', { stdio: 'ignore' });
      } catch {
        throw new Error(
          'npm is required for TypeScript/JavaScript support. Please install Node.js from https://nodejs.org'
        );
      }
    }

    const childProcess = spawn(command, args, {
      stdio: ['pipe', 'pipe', 'pipe'],
      cwd: serverConfig.rootDir || process.cwd(),
    });

    // Immediately attach error handler to catch ENOENT (command not found)
    let startupFailed = false;
    const startupErrorHandler = (error: Error) => {
      startupFailed = true;
      const extensions = serverConfig.extensions.join(', ');

      if (error.message.includes('ENOENT')) {
        process.stderr.write(
          `⚠️  Language server not found for ${extensions} files\n` +
            `   Command: ${serverConfig.command.join(' ')}\n` +
            `   To enable: ${this.getInstallInstructions(command)}\n`
        );
      } else {
        process.stderr.write(
          `⚠️  Failed to start language server for ${extensions} files\n` +
            `   Error: ${error.message}\n`
        );
      }

      // Mark this server as failed to prevent retry storms
      const serverKey = JSON.stringify(serverConfig.command);
      this.failedServers.add(serverKey);
    };

    childProcess.once('error', startupErrorHandler);

    // Give it a moment to fail if command doesn't exist
    await new Promise((resolve) => setTimeout(resolve, 100));

    if (startupFailed) {
      throw new Error(`Language server for ${serverConfig.extensions.join(', ')} is not available`);
    }

    // Remove the startup error handler since we're past the critical period
    childProcess.removeListener('error', startupErrorHandler);

    let initializationResolve: (() => void) | undefined;
    const initializationPromise = new Promise<void>((resolve) => {
      initializationResolve = resolve;
    });

    const serverState: ServerState = {
      process: childProcess,
      initialized: false,
      initializationPromise,
      initializationResolve,
      capabilities: undefined,
      buffer: '',
      openFiles: new Set(),
      diagnostics: new Map(),
      lastDiagnosticUpdate: new Map(),
      diagnosticVersions: new Map(),
      restartTimer: undefined,
      config: serverConfig,
      fileVersions: new Map(),
      startTime: Date.now(),
    };

    // Set up protocol handlers
    this.setupProtocolHandlers(serverState);

    // Initialize the server
    const initResult = await this.initializeServer(serverState, serverConfig);

    // Cache capabilities
    const serverKey = JSON.stringify(serverConfig.command);
    capabilityManager.cacheCapabilities(serverKey, initResult);

    if (initResult && typeof initResult === 'object' && 'capabilities' in initResult) {
      serverState.capabilities = (initResult as { capabilities: ServerCapabilities }).capabilities;
    }

    // Send initialized notification
    this.protocol.sendNotification(childProcess, 'initialized', {});

    // Give server time to process
    await new Promise((resolve) => setTimeout(resolve, 500));

    serverState.initialized = true;
    if (serverState.initializationResolve) {
      serverState.initializationResolve();
      serverState.initializationResolve = undefined;
    }

    process.stderr.write(`Server initialized successfully: ${serverConfig.command.join(' ')}\n`);

    // Set up auto-restart timer
    this.setupRestartTimer(serverState, serverConfig);

    return serverState;
  }

  /**
   * Set up protocol message handlers for server
   */
  private setupProtocolHandlers(serverState: ServerState): void {
    const serverKey = JSON.stringify(serverState.config?.command);

    serverState.process.stdout?.on('data', (data: Buffer) => {
      serverState.buffer += data.toString();
      const { messages, remainingBuffer } = this.protocol.parseMessages(serverState.buffer);
      serverState.buffer = remainingBuffer;

      for (const message of messages) {
        this.protocol.handleMessage(message, serverState);
      }
    });

    serverState.process.stderr?.on('data', (data: Buffer) => {
      process.stderr.write(data);
    });

    // CRITICAL FIX: Handle process errors to prevent crashes
    serverState.process.on('error', (error: Error) => {
      process.stderr.write(
        `LSP server process error (${serverState.config?.command.join(' ')}): ${error.message}\n`
      );
      // Remove from servers map so it can be restarted on next request
      this.servers.delete(serverKey);
    });

    // CRITICAL FIX: Handle unexpected server exits
    serverState.process.on('exit', (code: number | null, signal: string | null) => {
      process.stderr.write(
        `LSP server exited (${serverState.config?.command.join(' ')}): code=${code}, signal=${signal}\n`
      );

      // Clean up timers
      if (serverState.restartTimer) {
        clearTimeout(serverState.restartTimer);
        serverState.restartTimer = undefined;
      }

      // Remove from servers map so it can be restarted on next request
      this.servers.delete(serverKey);
    });
  }

  /**
   * Initialize LSP server with capabilities
   */
  private async initializeServer(
    serverState: ServerState,
    serverConfig: LSPServerConfig
  ): Promise<unknown> {
    const initializeParams = {
      processId: serverState.process.pid || null,
      clientInfo: { name: 'cclsp', version: '0.5.13' },
      capabilities: {
        textDocument: {
          synchronization: {
            didOpen: true,
            didChange: true,
            didClose: true,
          },
          definition: { linkSupport: false },
          references: {
            includeDeclaration: true,
            dynamicRegistration: false,
          },
          rename: { prepareSupport: false },
          documentSymbol: {
            symbolKind: {
              valueSet: [
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26,
              ],
            },
            hierarchicalDocumentSymbolSupport: true,
          },
          completion: {
            completionItem: {
              snippetSupport: true,
            },
          },
          hover: {},
          signatureHelp: {},
          diagnostic: {
            dynamicRegistration: false,
            relatedDocumentSupport: false,
          },
        },
        workspace: {
          workspaceEdit: {
            documentChanges: true,
          },
          workspaceFolders: true,
        },
      },
      rootUri: pathToUri(serverConfig.rootDir || process.cwd()),
      workspaceFolders: [
        {
          uri: pathToUri(serverConfig.rootDir || process.cwd()),
          name: 'workspace',
        },
      ],
      initializationOptions: this.getInitializationOptions(serverConfig),
    };

    return await this.protocol.sendRequest(
      serverState.process,
      'initialize',
      initializeParams,
      10000
    );
  }

  /**
   * Get server-specific initialization options
   */
  private getInitializationOptions(serverConfig: LSPServerConfig): unknown {
    if (serverConfig.initializationOptions !== undefined) {
      return serverConfig.initializationOptions;
    }

    // Server-specific defaults
    if (this.isPylspServer(serverConfig)) {
      return {
        settings: {
          pylsp: {
            plugins: {
              jedi_completion: { enabled: true },
              jedi_definition: { enabled: true },
              jedi_hover: { enabled: true },
              jedi_references: { enabled: true },
              jedi_signature_help: { enabled: true },
              jedi_symbols: { enabled: true },
              pylint: { enabled: false },
              pycodestyle: { enabled: false },
              pyflakes: { enabled: false },
              yapf: { enabled: false },
              rope_completion: { enabled: false },
            },
          },
        },
      };
    }

    if (this.isTypeScriptServer(serverConfig)) {
      return {
        hostInfo: 'cclsp',
        preferences: {
          includeCompletionsForModuleExports: true,
          includeCompletionsWithInsertText: true,
        },
      };
    }

    return undefined;
  }

  /**
   * Set up auto-restart timer for server
   */
  private setupRestartTimer(serverState: ServerState, serverConfig: LSPServerConfig): void {
    if (serverConfig.restartInterval && serverConfig.restartInterval > 0) {
      const intervalMs = serverConfig.restartInterval * 60 * 1000; // Convert minutes to milliseconds
      serverState.restartTimer = setTimeout(() => {
        process.stderr.write(
          `Auto-restarting server ${serverConfig.command.join(' ')} after ${serverConfig.restartInterval} minutes\n`
        );
        this.killServer(serverState);
        const serverKey = JSON.stringify(serverConfig.command);
        this.servers.delete(serverKey);
      }, intervalMs);
    }
  }

  /**
   * Kill a server process and clean up
   */
  private killServer(serverState: ServerState): void {
    if (serverState.restartTimer) {
      clearTimeout(serverState.restartTimer);
    }

    try {
      if (!serverState.process.killed) {
        serverState.process.kill('SIGTERM');
      }
    } catch (error) {
      // Process might already be dead or permissions issue - log but don't throw
      process.stderr.write(
        `Warning: Failed to kill server process (PID: ${serverState.process.pid}): ${error instanceof Error ? error.message : String(error)}\n`
      );
    }
  }

  private isPylspServer(serverConfig: LSPServerConfig): boolean {
    return serverConfig.command.some((cmd) => cmd.includes('pylsp'));
  }

  private getInstallInstructions(command: string): string {
    const instructions: Record<string, string> = {
      'typescript-language-server': 'npm install -g typescript-language-server typescript',
      pylsp: 'pip install python-lsp-server',
      gopls: 'go install golang.org/x/tools/gopls@latest',
      'rust-analyzer': 'rustup component add rust-analyzer',
      clangd: 'apt install clangd OR brew install llvm',
      jdtls: 'Download from Eclipse JDT releases',
      solargraph: 'gem install solargraph',
      intelephense: 'npm install -g intelephense',
    };

    return instructions[command] || `Install ${command} for your system`;
  }

  private isTypeScriptServer(serverConfig: LSPServerConfig): boolean {
    return serverConfig.command.some(
      (cmd) => cmd.includes('typescript-language-server') || cmd.includes('tsserver')
    );
  }

  /**
   * Clean up all servers
   */
  dispose(): void {
    for (const serverState of this.servers.values()) {
      this.killServer(serverState);
    }
    this.servers.clear();
    this.serversStarting.clear();
    this.protocol.dispose();
  }
}

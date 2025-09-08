import { type ChildProcess, spawn } from 'node:child_process';
import type { ServerCapabilities } from '../capability-manager.js';
import { capabilityManager } from '../capability-manager.js';
import type { ServerState } from '../lsp-types.js';
import { pathToUri } from '../path-utils.js';
import type { Config, LSPServerConfig } from '../types.js';
import { ServerNotAvailableError, getErrorMessage, logError } from '../utils/error-utils.js';
import { getPackageVersion } from '../utils/version.js';
import type { LSPProtocol } from './protocol.js';

// Server lifecycle constants
const SERVER_STARTUP_DELAY_MS = 100; // Delay to check for startup failures
const SERVER_PROCESSING_DELAY_MS = 500; // Delay for server to process initialization
const SERVER_INITIALIZE_TIMEOUT_MS = 10000; // Timeout for server initialization
const MINUTES_TO_MS = 60 * 1000; // Conversion factor

// Memory cleanup constants
const DIAGNOSTIC_CLEANUP_AGE_MS = 5 * MINUTES_TO_MS; // Clean diagnostics older than 5 minutes
const MAX_OPEN_FILES = 100; // Maximum number of open files to track (LRU cleanup)
const CLEANUP_INTERVAL_MS = 2 * MINUTES_TO_MS; // Run cleanup every 2 minutes

// Retry logic constants
const MAX_RETRY_ATTEMPTS = 1; // Maximum number of retry attempts per server
const RETRY_BACKOFF_MS = 2000; // 2 second backoff before retry attempt

/**
 * Manages LSP server processes and lifecycle
 * Handles server spawning, initialization, restart timers
 */
export class ServerManager {
  private servers: Map<string, ServerState> = new Map();
  private serversStarting: Map<string, Promise<ServerState>> = new Map();
  private failedServers: Set<string> = new Set();
  private retryAttempts: Map<string, number> = new Map(); // Track retry attempts per server
  private lastRetryTime: Map<string, number> = new Map(); // Track last retry timestamp per server
  private protocol: LSPProtocol;
  private cleanupTimer?: NodeJS.Timeout;

  constructor(protocol: LSPProtocol) {
    this.protocol = protocol;
    this.startCleanupTimer();
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
      const extension = filePath.split('.').pop() || 'unknown';
      throw new ServerNotAvailableError(
        `No language server configured for file: ${filePath}`,
        [extension],
        [],
        undefined
      );
    }

    const serverKey = JSON.stringify(serverConfig.command);

    // Check if server has failed and attempt recovery before giving up
    if (this.failedServers.has(serverKey)) {
      // Try to recover the server if possible
      try {
        const recoveredServer = await this.attemptServerRecovery(
          serverKey,
          serverConfig,
          new Error(`Server previously failed: ${serverConfig.command.join(' ')}`)
        );

        if (recoveredServer) {
          return recoveredServer;
        }
      } catch (recoveryError) {
        // Recovery failed, log and continue to throw original error
        logError('ServerManager', 'Server recovery failed', recoveryError, {
          serverKey,
          command: serverConfig.command,
        });
      }

      // Recovery failed or not attempted, throw original error
      throw new ServerNotAvailableError(
        `Language server for ${serverConfig.extensions.join(', ')} files is not available. ` +
          `Install it with: ${this.getInstallInstructions(serverConfig.command[0] || '')}`,
        serverConfig.extensions,
        serverConfig.command,
        undefined
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

    // Also clear retry tracking data for a fresh start
    this.retryAttempts.clear();
    this.lastRetryTime.clear();

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
        logError(
          'ServerManager',
          `Failed to preload server ${serverConfig.command.join(' ')}`,
          error,
          {
            serverCommand: serverConfig.command,
            extensions: serverConfig.extensions,
          }
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
      throw new ServerNotAvailableError(
        'No command specified in server config',
        serverConfig.extensions,
        serverConfig.command,
        undefined
      );
    }

    // For npx commands, provide helpful error message if npm is not installed
    if (command === 'npx') {
      try {
        const { execSync } = await import('node:child_process');
        execSync('npm --version', { stdio: 'ignore' });
      } catch (npmError) {
        throw new ServerNotAvailableError(
          'npm is required for TypeScript/JavaScript support. Please install Node.js from https://nodejs.org',
          ['ts', 'tsx', 'js', 'jsx'],
          serverConfig.command,
          npmError
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

      logError('ServerManager', 'Language server startup failed', error, {
        extensions,
        command: serverConfig.command,
        isENOENT: error.message.includes('ENOENT'),
      });

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
    await new Promise((resolve) => setTimeout(resolve, SERVER_STARTUP_DELAY_MS));

    if (startupFailed) {
      throw new ServerNotAvailableError(
        `Language server for ${serverConfig.extensions.join(', ')} is not available`,
        serverConfig.extensions,
        serverConfig.command,
        undefined
      );
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
    await new Promise((resolve) => setTimeout(resolve, SERVER_PROCESSING_DELAY_MS));

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
      logError('ServerManager', 'LSP server process error', error, {
        serverCommand: serverState.config?.command,
        serverKey,
        pid: serverState.process.pid,
      });
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
      clientInfo: { name: 'cclsp', version: getPackageVersion() },
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
      SERVER_INITIALIZE_TIMEOUT_MS
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
      const intervalMs = serverConfig.restartInterval * MINUTES_TO_MS; // Convert minutes to milliseconds
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
      const errorMessage = getErrorMessage(error);
      logError('ServerManager', 'Failed to kill server process', error, {
        pid: serverState.process.pid,
        serverCommand: serverState.config?.command,
      });
      process.stderr.write(
        `Warning: Failed to kill server process (PID: ${serverState.process.pid}): ${errorMessage}\n`
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
   * Check if an error is transient and worth retrying
   */
  private isTransientError(error: Error): boolean {
    const message = error.message.toLowerCase();

    // Transient errors that are worth retrying
    const transientPatterns = [
      'enoent', // Command not found (might be temporary PATH issue)
      'eacces', // Permission denied (might be temporary)
      'econnrefused', // Connection refused (network/service issues)
      'timeout', // Timeout errors
      'network', // Network-related errors
      'temporary', // Explicitly temporary errors
      'busy', // Resource busy
      'eagain', // Resource temporarily unavailable
    ];

    // Permanent errors that should not be retried
    const permanentPatterns = [
      'eisdir', // Is a directory error
      'enotdir', // Not a directory error
      'enomem', // Out of memory
      'configuration', // Configuration errors
      'syntax', // Syntax errors
      'parse', // Parse errors
      'invalid', // Invalid configuration/arguments
    ];

    // Check for permanent errors first (they take precedence)
    if (permanentPatterns.some((pattern) => message.includes(pattern))) {
      return false;
    }

    // Check for transient error patterns
    return transientPatterns.some((pattern) => message.includes(pattern));
  }

  /**
   * Attempt to recover a failed server by retrying startup
   */
  private async attemptServerRecovery(
    serverKey: string,
    serverConfig: LSPServerConfig,
    lastError: Error
  ): Promise<ServerState | null> {
    // Check if error is worth retrying
    if (!this.isTransientError(lastError)) {
      logError('ServerManager', 'Server failure not recoverable', lastError, {
        serverKey,
        command: serverConfig.command,
        reason: 'Non-transient error',
      });
      return null;
    }

    // Check retry limits
    const currentAttempts = this.retryAttempts.get(serverKey) || 0;
    if (currentAttempts >= MAX_RETRY_ATTEMPTS) {
      logError('ServerManager', 'Maximum retry attempts reached', lastError, {
        serverKey,
        command: serverConfig.command,
        attempts: currentAttempts,
        maxAttempts: MAX_RETRY_ATTEMPTS,
      });
      return null;
    }

    // Check backoff timing
    const lastRetry = this.lastRetryTime.get(serverKey) || 0;
    const timeSinceLastRetry = Date.now() - lastRetry;
    if (timeSinceLastRetry < RETRY_BACKOFF_MS) {
      logError('ServerManager', 'Retry attempted too soon', lastError, {
        serverKey,
        timeSinceLastRetry,
        requiredBackoff: RETRY_BACKOFF_MS,
      });
      return null;
    }

    // Update retry tracking
    this.retryAttempts.set(serverKey, currentAttempts + 1);
    this.lastRetryTime.set(serverKey, Date.now());

    // Remove from failed servers to allow retry
    this.failedServers.delete(serverKey);

    process.stderr.write(
      `🔄 Attempting server recovery (attempt ${currentAttempts + 1}/${MAX_RETRY_ATTEMPTS}): ${serverConfig.command.join(' ')}\n`
    );

    // Wait for backoff period
    await new Promise((resolve) => setTimeout(resolve, RETRY_BACKOFF_MS));

    try {
      // Attempt to start the server again
      const startupPromise = this.startServer(serverConfig);
      this.serversStarting.set(serverKey, startupPromise);

      try {
        const serverState = await startupPromise;
        this.servers.set(serverKey, serverState);

        // Clear retry tracking on successful recovery
        this.retryAttempts.delete(serverKey);
        this.lastRetryTime.delete(serverKey);

        process.stderr.write(`✅ Server recovery successful: ${serverConfig.command.join(' ')}\n`);
        return serverState;
      } finally {
        this.serversStarting.delete(serverKey);
      }
    } catch (retryError) {
      const retryErrorMessage = getErrorMessage(retryError);
      logError('ServerManager', 'Server recovery attempt failed', retryError, {
        serverKey,
        command: serverConfig.command,
        attempt: currentAttempts + 1,
        originalError: lastError.message,
        retryError: retryErrorMessage,
      });

      process.stderr.write(
        `❌ Server recovery failed (attempt ${currentAttempts + 1}/${MAX_RETRY_ATTEMPTS}): ${retryErrorMessage}\n`
      );

      // Re-add to failed servers
      this.failedServers.add(serverKey);
      return null;
    }
  }

  /**
   * Manually trigger memory cleanup (useful for testing or immediate cleanup)
   */
  cleanupMemory(): void {
    this.cleanupStaleData();
  }

  /**
   * Start background cleanup timer to prevent memory leaks
   */
  private startCleanupTimer(): void {
    this.cleanupTimer = setInterval(() => {
      this.cleanupStaleData();
    }, CLEANUP_INTERVAL_MS);
  }

  /**
   * Clean up stale diagnostic data and limit open files to prevent memory leaks
   */
  private cleanupStaleData(): void {
    const currentTime = Date.now();
    let diagnosticsCleared = 0;
    let filesLimited = 0;

    for (const serverState of this.servers.values()) {
      // Clean up stale diagnostics (older than DIAGNOSTIC_CLEANUP_AGE_MS)
      for (const [fileUri, lastUpdateTime] of serverState.lastDiagnosticUpdate.entries()) {
        if (currentTime - lastUpdateTime > DIAGNOSTIC_CLEANUP_AGE_MS) {
          serverState.diagnostics.delete(fileUri);
          serverState.lastDiagnosticUpdate.delete(fileUri);
          serverState.diagnosticVersions.delete(fileUri);
          diagnosticsCleared++;
        }
      }

      // Implement LRU cleanup for open files if we exceed MAX_OPEN_FILES
      if (serverState.openFiles.size > MAX_OPEN_FILES) {
        // Convert Set to Array to be able to slice it
        const openFilesArray = Array.from(serverState.openFiles);
        // Keep the most recent MAX_OPEN_FILES (assuming newer files are added later)
        const filesToKeep = openFilesArray.slice(-MAX_OPEN_FILES);
        const filesToRemove = openFilesArray.slice(0, -MAX_OPEN_FILES);

        // Clear and re-add the files to keep
        serverState.openFiles.clear();
        for (const file of filesToKeep) {
          serverState.openFiles.add(file);
        }

        filesLimited += filesToRemove.length;
      }
    }

    // Log cleanup activity if any cleanup was performed
    if (diagnosticsCleared > 0 || filesLimited > 0) {
      process.stderr.write(
        `Memory cleanup: cleared ${diagnosticsCleared} stale diagnostics, limited ${filesLimited} open files\n`
      );
    }
  }

  /**
   * Clean up all servers
   */
  dispose(): void {
    // Clean up background cleanup timer
    if (this.cleanupTimer) {
      clearInterval(this.cleanupTimer);
      this.cleanupTimer = undefined;
    }

    for (const serverState of this.servers.values()) {
      this.killServer(serverState);
    }
    this.servers.clear();
    this.serversStarting.clear();
    this.failedServers.clear();
    this.retryAttempts.clear();
    this.lastRetryTime.clear();
    this.protocol.dispose();
  }
}

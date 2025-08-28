import { type ChildProcess, spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { constants, access, readFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { type ServerCapabilities, capabilityManager } from './capability-manager.js';
import { loadGitignore, scanDirectoryForExtensions } from './file-scanner.js';
import * as CoreMethods from './lsp-methods/core-methods.js';
import * as DiagnosticMethods from './lsp-methods/diagnostic-methods.js';
import * as DocumentMethods from './lsp-methods/document-methods.js';
import * as WorkspaceMethods from './lsp-methods/workspace-methods.js';
import type {
  Config,
  Diagnostic,
  DocumentDiagnosticReport,
  DocumentSymbol,
  LSPError,
  LSPLocation,
  LSPServerConfig,
  Location,
  Position,
  SymbolInformation,
  SymbolMatch,
} from './types.js';
import { SymbolKind } from './types.js';
import { pathToUri } from './utils.js';

interface LSPMessage {
  jsonrpc: string;
  id?: number;
  method?: string;
  params?: unknown;
  result?: unknown;
  error?: LSPError;
}

interface ServerState {
  process: ChildProcess;
  initialized: boolean;
  initializationPromise: Promise<void>;
  openFiles: Set<string>;
  fileVersions: Map<string, number>; // Track file versions for didChange notifications
  startTime: number;
  config: LSPServerConfig;
  restartTimer?: NodeJS.Timeout;
  initializationResolve?: () => void;
  diagnostics: Map<string, Diagnostic[]>; // Store diagnostics by file URI
  lastDiagnosticUpdate: Map<string, number>; // Track last update time per file
  diagnosticVersions: Map<string, number>; // Track diagnostic versions per file
  capabilities?: ServerCapabilities; // LSP server capabilities from initialization
}

export class LSPClient {
  private config: Config;
  private servers: Map<string, ServerState> = new Map();
  private serversStarting: Map<string, Promise<ServerState>> = new Map();
  private nextId = 1;
  private pendingRequests: Map<
    number,
    { resolve: (value: unknown) => void; reject: (reason?: unknown) => void }
  > = new Map();

  private isPylspServer(serverConfig: LSPServerConfig): boolean {
    return serverConfig.command.some((cmd) => cmd.includes('pylsp'));
  }

  private isTypeScriptServer(serverConfig: LSPServerConfig): boolean {
    return serverConfig.command.some(
      (cmd) => cmd.includes('typescript-language-server') || cmd.includes('tsserver')
    );
  }

  constructor(configPath?: string) {
    // First try to load from environment variable (MCP config)
    if (process.env.CCLSP_CONFIG_PATH) {
      process.stderr.write(
        `Loading config from CCLSP_CONFIG_PATH: ${process.env.CCLSP_CONFIG_PATH}\n`
      );

      if (!existsSync(process.env.CCLSP_CONFIG_PATH)) {
        process.stderr.write(
          `Config file specified in CCLSP_CONFIG_PATH does not exist: ${process.env.CCLSP_CONFIG_PATH}\n`
        );
        process.exit(1);
      }

      try {
        const configData = readFileSync(process.env.CCLSP_CONFIG_PATH, 'utf-8');
        this.config = JSON.parse(configData);
        process.stderr.write(
          `Loaded ${this.config.servers.length} server configurations from env\n`
        );
        return;
      } catch (error) {
        process.stderr.write(`Failed to load config from CCLSP_CONFIG_PATH: ${error}\n`);
        process.exit(1);
      }
    }

    // configPath must be provided if CCLSP_CONFIG_PATH is not set
    if (!configPath) {
      process.stderr.write(
        'Error: configPath is required when CCLSP_CONFIG_PATH environment variable is not set\n'
      );
      process.exit(1);
    }

    // Try to load from config file
    try {
      process.stderr.write(`Loading config from file: ${configPath}\n`);
      const configData = readFileSync(configPath, 'utf-8');
      this.config = JSON.parse(configData);
      process.stderr.write(`Loaded ${this.config.servers.length} server configurations\n`);
    } catch (error) {
      process.stderr.write(`Failed to load config from ${configPath}: ${error}\n`);
      process.exit(1);
    }
  }

  private getServerForFile(filePath: string): LSPServerConfig | null {
    const extension = filePath.split('.').pop();
    if (!extension) return null;

    process.stderr.write(`Looking for server for extension: ${extension}\n`);
    process.stderr.write(
      `Available servers: ${this.config.servers.map((s) => s.extensions.join(',')).join(' | ')}\n`
    );

    const server = this.config.servers.find((server) => server.extensions.includes(extension));

    if (server) {
      process.stderr.write(`Found server for ${extension}: ${server.command.join(' ')}\n`);
    } else {
      process.stderr.write(`No server found for extension: ${extension}\n`);
    }

    return server || null;
  }

  private async startServer(serverConfig: LSPServerConfig): Promise<ServerState> {
    const [command, ...args] = serverConfig.command;
    if (!command) {
      throw new Error('No command specified in server config');
    }
    const childProcess = spawn(command, args, {
      stdio: ['pipe', 'pipe', 'pipe'],
      cwd: serverConfig.rootDir || process.cwd(),
    });

    let initializationResolve: (() => void) | undefined;
    const initializationPromise = new Promise<void>((resolve) => {
      initializationResolve = resolve;
    });

    const serverState: ServerState = {
      process: childProcess,
      initialized: false,
      initializationPromise,
      openFiles: new Set(),
      fileVersions: new Map(),
      startTime: Date.now(),
      config: serverConfig,
      restartTimer: undefined,
      diagnostics: new Map(),
      lastDiagnosticUpdate: new Map(),
      diagnosticVersions: new Map(),
    };

    // Store the resolve function to call when initialized notification is received
    serverState.initializationResolve = initializationResolve;

    let buffer = '';
    childProcess.stdout?.on('data', (data: Buffer) => {
      buffer += data.toString();

      while (buffer.includes('\r\n\r\n')) {
        const headerEndIndex = buffer.indexOf('\r\n\r\n');
        const headerPart = buffer.substring(0, headerEndIndex);
        const contentLengthMatch = headerPart.match(/Content-Length: (\d+)/);

        if (contentLengthMatch?.[1]) {
          const contentLength = Number.parseInt(contentLengthMatch[1]);
          const messageStart = headerEndIndex + 4;

          if (buffer.length >= messageStart + contentLength) {
            const messageContent = buffer.substring(messageStart, messageStart + contentLength);
            buffer = buffer.substring(messageStart + contentLength);

            try {
              const message: LSPMessage = JSON.parse(messageContent);
              // Debug log all messages
              if (
                message.method === 'textDocument/hover' ||
                (message.id && message.method === undefined)
              ) {
                process.stderr.write(
                  `[DEBUG LSP Message] ${JSON.stringify(message).substring(0, 200)}\n`
                );
              }
              this.handleMessage(message, serverState);
            } catch (error) {
              process.stderr.write(`Failed to parse LSP message: ${error}\n`);
            }
          } else {
            break;
          }
        } else {
          buffer = buffer.substring(headerEndIndex + 4);
        }
      }
    });

    childProcess.stderr?.on('data', (data: Buffer) => {
      // Forward LSP server stderr directly to MCP stderr
      process.stderr.write(data);
    });

    // Initialize the server
    const initializeParams: {
      processId: number | null;
      clientInfo: { name: string; version: string };
      capabilities: unknown;
      rootUri: string;
      workspaceFolders: Array<{ uri: string; name: string }>;
      initializationOptions?: unknown;
    } = {
      processId: childProcess.pid || null,
      clientInfo: { name: 'cclsp', version: '0.1.0' },
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
    };

    // Handle initializationOptions with backwards compatibility
    if (serverConfig.initializationOptions !== undefined) {
      initializeParams.initializationOptions = serverConfig.initializationOptions;
    } else if (this.isPylspServer(serverConfig)) {
      // Backwards compatibility: provide default pylsp settings when none are specified
      initializeParams.initializationOptions = {
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
    } else if (this.isTypeScriptServer(serverConfig)) {
      // Provide TypeScript server initialization options
      initializeParams.initializationOptions = {
        hostInfo: 'cclsp',
        preferences: {
          includeCompletionsForModuleExports: true,
          includeCompletionsWithInsertText: true,
        },
      };
    }

    const initResult = await this.sendRequest(childProcess, 'initialize', initializeParams, 10000); // 10 second timeout for initialization

    // Cache server capabilities for feature detection
    const serverKey = JSON.stringify(serverConfig.command);
    capabilityManager.cacheCapabilities(serverKey, initResult);

    // Store capabilities in server state for easy access
    if (initResult && typeof initResult === 'object' && (initResult as any).capabilities) {
      serverState.capabilities = (initResult as any).capabilities as ServerCapabilities;
    }

    // Send the initialized notification after receiving the initialize response
    await this.sendNotification(childProcess, 'initialized', {});

    // Give the server a moment to process the initialized notification and become ready
    // Most servers are ready immediately after the initialized notification
    await new Promise((resolve) => setTimeout(resolve, 500)); // 500ms should be enough

    serverState.initialized = true;
    if (serverState.initializationResolve) {
      serverState.initializationResolve();
      serverState.initializationResolve = undefined;
    }

    process.stderr.write(
      `[DEBUG startServer] Server initialized successfully: ${serverConfig.command.join(' ')}\n`
    );

    // Set up auto-restart timer if configured
    this.setupRestartTimer(serverState);

    return serverState;
  }

  private handleMessage(message: LSPMessage, serverState?: ServerState) {
    if (message.id && this.pendingRequests.has(message.id)) {
      const request = this.pendingRequests.get(message.id);
      if (!request) return;
      const { resolve, reject } = request;
      this.pendingRequests.delete(message.id);

      if (message.error) {
        reject(new Error(message.error.message || 'LSP Error'));
      } else {
        resolve(message.result);
      }
    }

    // Handle notifications from server
    if (message.method && serverState) {
      if (message.method === 'initialized') {
        process.stderr.write(
          '[DEBUG handleMessage] Received initialized notification from server\n'
        );
        serverState.initialized = true;
        // Resolve the initialization promise
        const resolve = serverState.initializationResolve;
        if (resolve) {
          resolve();
          serverState.initializationResolve = undefined;
        }
      } else if (message.method === 'textDocument/publishDiagnostics') {
        // Handle diagnostic notifications from the server
        const params = message.params as {
          uri: string;
          diagnostics: Diagnostic[];
          version?: number;
        };
        if (params?.uri) {
          process.stderr.write(
            `[DEBUG handleMessage] Received publishDiagnostics for ${params.uri} with ${params.diagnostics?.length || 0} diagnostics${params.version !== undefined ? ` (version: ${params.version})` : ''}\n`
          );
          serverState.diagnostics.set(params.uri, params.diagnostics || []);
          serverState.lastDiagnosticUpdate.set(params.uri, Date.now());
          if (params.version !== undefined) {
            serverState.diagnosticVersions.set(params.uri, params.version);
          }
        }
      }
    }
  }

  private sendMessage(process: ChildProcess, message: LSPMessage): void {
    const content = JSON.stringify(message);
    const header = `Content-Length: ${Buffer.byteLength(content)}\r\n\r\n`;
    process.stdin?.write(header + content);
  }

  private sendRequest(
    process: ChildProcess,
    method: string,
    params: unknown,
    timeout = 30000
  ): Promise<unknown> {
    const id = this.nextId++;
    const message: LSPMessage = {
      jsonrpc: '2.0',
      id,
      method,
      params,
    };

    console.log(
      `[DEBUG sendRequest] Sending ${method} with params:`,
      JSON.stringify(params, null, 2)
    );

    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.pendingRequests.delete(id);
        console.log(`[DEBUG sendRequest] Request ${id} (${method}) timed out after ${timeout}ms`);
        reject(new Error(`LSP request timeout: ${method} (${timeout}ms)`));
      }, timeout);

      this.pendingRequests.set(id, {
        resolve: (value: unknown) => {
          clearTimeout(timeoutId);
          console.log(
            `[DEBUG sendRequest] Request ${id} (${method}) resolved with:`,
            JSON.stringify(value, null, 2)
          );
          resolve(value);
        },
        reject: (reason?: unknown) => {
          clearTimeout(timeoutId);
          reject(reason);
        },
      });

      this.sendMessage(process, message);
    });
  }

  sendNotification(process: ChildProcess, method: string, params: unknown): void {
    const message: LSPMessage = {
      jsonrpc: '2.0',
      method,
      params,
    };

    // Debug log important notifications
    if (method === 'textDocument/didOpen' || method === 'initialized') {
      console.error(`[DEBUG sendNotification] Sending ${method}`);
    }

    this.sendMessage(process, message);
  }

  private setupRestartTimer(serverState: ServerState): void {
    if (serverState.config.restartInterval && serverState.config.restartInterval > 0) {
      // Minimum interval is 0.1 minutes (6 seconds) for testing, practical minimum is 1 minute
      const minInterval = 0.1;
      const actualInterval = Math.max(serverState.config.restartInterval, minInterval);
      const intervalMs = actualInterval * 60 * 1000; // Convert minutes to milliseconds

      process.stderr.write(
        `[DEBUG setupRestartTimer] Setting up restart timer for ${actualInterval} minutes\n`
      );

      serverState.restartTimer = setTimeout(() => {
        this.restartServer(serverState);
      }, intervalMs);
    }
  }

  private async restartServer(serverState: ServerState): Promise<void> {
    const key = JSON.stringify(serverState.config);
    process.stderr.write(
      `[DEBUG restartServer] Restarting LSP server for ${serverState.config.command.join(' ')}\n`
    );

    // Clear existing timer
    if (serverState.restartTimer) {
      clearTimeout(serverState.restartTimer);
      serverState.restartTimer = undefined;
    }

    // Terminate old server
    serverState.process.kill();

    // Remove from servers map
    this.servers.delete(key);

    try {
      // Start new server
      const newServerState = await this.startServer(serverState.config);
      this.servers.set(key, newServerState);

      process.stderr.write(
        `[DEBUG restartServer] Successfully restarted LSP server for ${serverState.config.command.join(' ')}\n`
      );
    } catch (error) {
      process.stderr.write(`[DEBUG restartServer] Failed to restart LSP server: ${error}\n`);
    }
  }

  /**
   * Manually restart LSP servers for specific extensions or all servers
   * @param extensions Array of file extensions, or null to restart all
   * @returns Object with success status and details about restarted servers
   */
  async restartServers(
    extensions?: string[]
  ): Promise<{ success: boolean; restarted: string[]; failed: string[]; message: string }> {
    const restarted: string[] = [];
    const failed: string[] = [];

    process.stderr.write(
      `[DEBUG restartServers] Request to restart servers for extensions: ${extensions ? extensions.join(', ') : 'all'}\n`
    );

    // Collect servers to restart
    const serversToRestart: Array<{ key: string; state: ServerState }> = [];

    for (const [key, serverState] of this.servers.entries()) {
      if (!extensions || extensions.some((ext) => serverState.config.extensions.includes(ext))) {
        serversToRestart.push({ key, state: serverState });
      }
    }

    if (serversToRestart.length === 0) {
      const message = extensions
        ? `No LSP servers found for extensions: ${extensions.join(', ')}`
        : 'No LSP servers are currently running';
      return { success: false, restarted: [], failed: [], message };
    }

    // Restart each server
    for (const { key, state } of serversToRestart) {
      const serverDesc = `${state.config.command.join(' ')} (${state.config.extensions.join(', ')})`;

      try {
        // Clear existing timer
        if (state.restartTimer) {
          clearTimeout(state.restartTimer);
          state.restartTimer = undefined;
        }

        // Terminate old server
        state.process.kill();

        // Remove from servers map
        this.servers.delete(key);

        // Start new server with timeout wrapper
        const startPromise = this.startServer(state.config);
        const timeoutPromise = new Promise((_, reject) =>
          setTimeout(() => reject(new Error('Server restart timed out after 15 seconds')), 15000)
        );

        const newServerState = (await Promise.race([startPromise, timeoutPromise])) as ServerState;
        this.servers.set(key, newServerState);

        restarted.push(serverDesc);
        process.stderr.write(`[DEBUG restartServers] Successfully restarted: ${serverDesc}\n`);
      } catch (error) {
        failed.push(`${serverDesc}: ${error}`);
        process.stderr.write(`[DEBUG restartServers] Failed to restart: ${serverDesc}: ${error}\n`);
      }
    }

    const success = failed.length === 0;
    let message: string;

    if (success) {
      message = `Successfully restarted ${restarted.length} LSP server(s)`;
    } else if (restarted.length > 0) {
      message = `Restarted ${restarted.length} server(s), but ${failed.length} failed`;
    } else {
      message = `Failed to restart all ${failed.length} server(s)`;
    }

    return { success, restarted, failed, message };
  }

  /**
   * Synchronize file content with LSP server after external modifications
   * This should be called after any disk writes to keep the LSP server in sync
   */
  async syncFileContent(filePath: string): Promise<void> {
    try {
      const serverState = await this.getServer(filePath);

      // If file is not already open in the LSP server, open it first
      if (!serverState.openFiles.has(filePath)) {
        process.stderr.write(
          `[DEBUG syncFileContent] File not open, opening it first: ${filePath}\n`
        );
        await this.ensureFileOpen(serverState, filePath);
      }

      process.stderr.write(`[DEBUG syncFileContent] Syncing file: ${filePath}\n`);

      const fileContent = readFileSync(filePath, 'utf-8');
      const uri = pathToUri(filePath);

      // Increment version and send didChange notification
      const version = (serverState.fileVersions.get(filePath) || 1) + 1;
      serverState.fileVersions.set(filePath, version);

      await this.sendNotification(serverState.process, 'textDocument/didChange', {
        textDocument: {
          uri,
          version,
        },
        contentChanges: [
          {
            text: fileContent,
          },
        ],
      });

      process.stderr.write(
        `[DEBUG syncFileContent] File synced with version ${version}: ${filePath}\n`
      );
    } catch (error) {
      process.stderr.write(`[DEBUG syncFileContent] Failed to sync file ${filePath}: ${error}\n`);
      // Don't throw - syncing is best effort
    }
  }

  private async ensureFileOpen(serverState: ServerState, filePath: string): Promise<void> {
    if (serverState.openFiles.has(filePath)) {
      process.stderr.write(`[DEBUG ensureFileOpen] File already open: ${filePath}\n`);
      return;
    }

    process.stderr.write(`[DEBUG ensureFileOpen] Opening file: ${filePath}\n`);

    try {
      const fileContent = readFileSync(filePath, 'utf-8');
      const uri = pathToUri(filePath);
      const languageId = this.getLanguageId(filePath);

      process.stderr.write(
        `[DEBUG ensureFileOpen] File content length: ${fileContent.length}, languageId: ${languageId}\n`
      );

      process.stderr.write('[DEBUG ensureFileOpen] About to send didOpen notification\n');
      process.stderr.write(
        `[DEBUG ensureFileOpen] serverState.process exists: ${!!serverState.process}\n`
      );

      if (!serverState.process) {
        throw new Error('Server process is null!');
      }

      try {
        this.sendNotification(serverState.process, 'textDocument/didOpen', {
          textDocument: {
            uri,
            languageId,
            version: 1,
            text: fileContent,
          },
        });
        process.stderr.write('[DEBUG ensureFileOpen] Sent didOpen notification\n');
      } catch (notifError) {
        process.stderr.write(`[DEBUG ensureFileOpen] Error sending didOpen: ${notifError}\n`);
        throw notifError;
      }

      serverState.openFiles.add(filePath);
      serverState.fileVersions.set(filePath, 1);
      process.stderr.write(`[DEBUG ensureFileOpen] File opened successfully: ${filePath}\n`);
    } catch (error) {
      process.stderr.write(`[DEBUG ensureFileOpen] Failed to open file ${filePath}: ${error}\n`);
      throw error;
    }
  }

  private getLanguageId(filePath: string): string {
    const extension = filePath.split('.').pop()?.toLowerCase();
    const languageMap: Record<string, string> = {
      ts: 'typescript',
      tsx: 'typescriptreact',
      js: 'javascript',
      jsx: 'javascriptreact',
      py: 'python',
      go: 'go',
      rs: 'rust',
      c: 'c',
      cpp: 'cpp',
      h: 'c',
      hpp: 'cpp',
      java: 'java',
      cs: 'csharp',
      php: 'php',
      rb: 'ruby',
      swift: 'swift',
      kt: 'kotlin',
      scala: 'scala',
      dart: 'dart',
      lua: 'lua',
      sh: 'shellscript',
      bash: 'shellscript',
      json: 'json',
      yaml: 'yaml',
      yml: 'yaml',
      xml: 'xml',
      html: 'html',
      css: 'css',
      scss: 'scss',
      vue: 'vue',
      svelte: 'svelte',
      tf: 'terraform',
      sql: 'sql',
      graphql: 'graphql',
      gql: 'graphql',
      md: 'markdown',
      tex: 'latex',
      elm: 'elm',
      hs: 'haskell',
      ml: 'ocaml',
      clj: 'clojure',
      fs: 'fsharp',
      r: 'r',
      toml: 'toml',
      zig: 'zig',
    };

    return languageMap[extension || ''] || 'plaintext';
  }

  private async getServer(filePath: string): Promise<ServerState> {
    process.stderr.write(`[DEBUG getServer] Getting server for file: ${filePath}\n`);

    const serverConfig = this.getServerForFile(filePath);
    if (!serverConfig) {
      throw new Error(`No LSP server configured for file: ${filePath}`);
    }

    process.stderr.write(
      `[DEBUG getServer] Found server config: ${serverConfig.command.join(' ')}\n`
    );

    const key = JSON.stringify(serverConfig);

    // Check if server already exists
    if (this.servers.has(key)) {
      process.stderr.write('[DEBUG getServer] Using existing server instance\n');
      const server = this.servers.get(key);
      if (!server) {
        throw new Error('Server exists in map but is undefined');
      }
      return server;
    }

    // Check if server is currently starting
    if (this.serversStarting.has(key)) {
      process.stderr.write('[DEBUG getServer] Waiting for server startup in progress\n');
      const startPromise = this.serversStarting.get(key);
      if (!startPromise) {
        throw new Error('Server start promise exists in map but is undefined');
      }
      return await startPromise;
    }

    // Start new server with concurrency protection
    process.stderr.write('[DEBUG getServer] Starting new server instance\n');
    const startPromise = this.startServer(serverConfig);
    this.serversStarting.set(key, startPromise);

    try {
      const serverState = await startPromise;
      this.servers.set(key, serverState);
      this.serversStarting.delete(key);
      process.stderr.write('[DEBUG getServer] Server started and cached\n');
      return serverState;
    } catch (error) {
      this.serversStarting.delete(key);
      throw error;
    }
  }

  async findDefinition(filePath: string, position: Position): Promise<Location[]> {
    const context: CoreMethods.CoreMethodsContext = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: this.sendRequest.bind(this),
    };
    return CoreMethods.findDefinition(context, filePath, position);
  }

  async findReferences(
    filePath: string,
    position: Position,
    includeDeclaration = true
  ): Promise<Location[]> {
    const context: CoreMethods.CoreMethodsContext = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: this.sendRequest.bind(this),
    };
    return CoreMethods.findReferences(context, filePath, position, includeDeclaration);
  }

  async renameSymbol(
    filePath: string,
    position: Position,
    newName: string
  ): Promise<{
    changes?: Record<string, Array<{ range: { start: Position; end: Position }; newText: string }>>;
  }> {
    const context: CoreMethods.CoreMethodsContext = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: this.sendRequest.bind(this),
    };
    return CoreMethods.renameSymbol(context, filePath, position, newName);
  }

  async getDocumentSymbols(filePath: string): Promise<DocumentSymbol[] | SymbolInformation[]> {
    const context: DocumentMethods.DocumentMethodsContext = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: this.sendRequest.bind(this),
    };
    return DocumentMethods.getDocumentSymbols(context, filePath);
  }

  flattenDocumentSymbols = DocumentMethods.flattenDocumentSymbols;
  isDocumentSymbolArray = DocumentMethods.isDocumentSymbolArray;
  symbolKindToString = DocumentMethods.symbolKindToString;
  getValidSymbolKinds = DocumentMethods.getValidSymbolKinds;

  private async findSymbolPositionInFile(
    filePath: string,
    symbol: SymbolInformation
  ): Promise<Position> {
    try {
      const fileContent = readFileSync(filePath, 'utf-8');
      const lines = fileContent.split('\n');

      const range = symbol.location.range;
      const startLine = range.start.line;
      const endLine = range.end.line;

      process.stderr.write(
        `[DEBUG findSymbolPositionInFile] Searching for "${symbol.name}" in lines ${startLine}-${endLine}\n`
      );

      // Search within the symbol's range for the symbol name
      for (let lineNum = startLine; lineNum <= endLine && lineNum < lines.length; lineNum++) {
        const line = lines[lineNum];
        if (!line) continue;

        // Find all occurrences of the symbol name in this line
        let searchStart = 0;
        if (lineNum === startLine) {
          searchStart = range.start.character;
        }

        let searchEnd = line.length;
        if (lineNum === endLine) {
          searchEnd = range.end.character;
        }

        const searchText = line.substring(searchStart, searchEnd);
        const symbolIndex = searchText.indexOf(symbol.name);

        if (symbolIndex !== -1) {
          const actualCharacter = searchStart + symbolIndex;
          process.stderr.write(
            `[DEBUG findSymbolPositionInFile] Found "${symbol.name}" at line ${lineNum}, character ${actualCharacter}\n`
          );

          return {
            line: lineNum,
            character: actualCharacter,
          };
        }
      }

      // Fallback to range start if not found
      process.stderr.write(
        `[DEBUG findSymbolPositionInFile] Symbol "${symbol.name}" not found in range, using range start\n`
      );
      return range.start;
    } catch (error) {
      process.stderr.write(
        `[DEBUG findSymbolPositionInFile] Error reading file: ${error}, using range start\n`
      );
      return symbol.location.range.start;
    }
  }

  private stringToSymbolKind(kindStr: string): SymbolKind | null {
    const kindMap: Record<string, SymbolKind> = {
      file: SymbolKind.File,
      module: SymbolKind.Module,
      namespace: SymbolKind.Namespace,
      package: SymbolKind.Package,
      class: SymbolKind.Class,
      method: SymbolKind.Method,
      property: SymbolKind.Property,
      field: SymbolKind.Field,
      constructor: SymbolKind.Constructor,
      enum: SymbolKind.Enum,
      interface: SymbolKind.Interface,
      function: SymbolKind.Function,
      variable: SymbolKind.Variable,
      constant: SymbolKind.Constant,
      string: SymbolKind.String,
      number: SymbolKind.Number,
      boolean: SymbolKind.Boolean,
      array: SymbolKind.Array,
      object: SymbolKind.Object,
      key: SymbolKind.Key,
      null: SymbolKind.Null,
      enum_member: SymbolKind.EnumMember,
      struct: SymbolKind.Struct,
      event: SymbolKind.Event,
      operator: SymbolKind.Operator,
      type_parameter: SymbolKind.TypeParameter,
    };
    return kindMap[kindStr.toLowerCase()] || null;
  }

  async findSymbolsByName(
    filePath: string,
    symbolName: string,
    symbolKind?: string
  ): Promise<{ matches: SymbolMatch[]; warning?: string }> {
    process.stderr.write(
      `[DEBUG findSymbolsByName] Searching for symbol "${symbolName}" with kind "${symbolKind || 'any'}" in ${filePath}\n`
    );

    // Validate symbolKind if provided - return validation info for caller to handle
    let validationWarning: string | undefined;
    let effectiveSymbolKind = symbolKind;
    if (symbolKind && this.stringToSymbolKind(symbolKind) === null) {
      const validKinds = this.getValidSymbolKinds();
      validationWarning = `⚠️ Invalid symbol kind "${symbolKind}". Valid kinds are: ${validKinds.join(', ')}. Searching all symbol types instead.`;
      effectiveSymbolKind = undefined; // Reset to search all kinds
    }

    const symbols = await this.getDocumentSymbols(filePath);
    const matches: SymbolMatch[] = [];

    process.stderr.write(
      `[DEBUG findSymbolsByName] Got ${symbols.length} symbols from documentSymbols\n`
    );

    if (this.isDocumentSymbolArray(symbols)) {
      process.stderr.write(
        '[DEBUG findSymbolsByName] Processing DocumentSymbol[] (hierarchical format)\n'
      );
      // Handle DocumentSymbol[] (hierarchical)
      const flatSymbols = this.flattenDocumentSymbols(symbols);
      process.stderr.write(
        `[DEBUG findSymbolsByName] Flattened to ${flatSymbols.length} symbols\n`
      );

      for (const symbol of flatSymbols) {
        const nameMatches = symbol.name === symbolName || symbol.name.includes(symbolName);
        const kindMatches =
          !effectiveSymbolKind ||
          this.symbolKindToString(symbol.kind) === effectiveSymbolKind.toLowerCase();

        process.stderr.write(
          `[DEBUG findSymbolsByName] Checking DocumentSymbol: ${symbol.name} (${this.symbolKindToString(symbol.kind)}) - nameMatch: ${nameMatches}, kindMatch: ${kindMatches}\n`
        );

        if (nameMatches && kindMatches) {
          process.stderr.write(
            `[DEBUG findSymbolsByName] DocumentSymbol match: ${symbol.name} (kind=${symbol.kind}) using selectionRange ${symbol.selectionRange.start.line}:${symbol.selectionRange.start.character}\n`
          );

          matches.push({
            name: symbol.name,
            kind: symbol.kind,
            position: symbol.selectionRange.start,
            range: symbol.range,
            detail: symbol.detail,
          });
        }
      }
    } else {
      process.stderr.write(
        '[DEBUG findSymbolsByName] Processing SymbolInformation[] (flat format)\n'
      );
      // Handle SymbolInformation[] (flat)
      for (const symbol of symbols) {
        const nameMatches = symbol.name === symbolName || symbol.name.includes(symbolName);
        const kindMatches =
          !effectiveSymbolKind ||
          this.symbolKindToString(symbol.kind) === effectiveSymbolKind.toLowerCase();

        process.stderr.write(
          `[DEBUG findSymbolsByName] Checking SymbolInformation: ${symbol.name} (${this.symbolKindToString(symbol.kind)}) - nameMatch: ${nameMatches}, kindMatch: ${kindMatches}\n`
        );

        if (nameMatches && kindMatches) {
          process.stderr.write(
            `[DEBUG findSymbolsByName] SymbolInformation match: ${symbol.name} (kind=${symbol.kind}) at ${symbol.location.range.start.line}:${symbol.location.range.start.character} to ${symbol.location.range.end.line}:${symbol.location.range.end.character}\n`
          );

          // For SymbolInformation, we need to find the actual symbol name position within the range
          // by reading the file content and searching for the symbol name
          const position = await this.findSymbolPositionInFile(filePath, symbol);

          process.stderr.write(
            `[DEBUG findSymbolsByName] Found symbol position in file: ${position.line}:${position.character}\n`
          );

          matches.push({
            name: symbol.name,
            kind: symbol.kind,
            position: position,
            range: symbol.location.range,
            detail: undefined, // SymbolInformation doesn't have detail
          });
        }
      }
    }

    process.stderr.write(`[DEBUG findSymbolsByName] Found ${matches.length} matching symbols\n`);

    // If a specific symbol kind was requested but no matches found, try searching all kinds as fallback
    let fallbackWarning: string | undefined;
    if (effectiveSymbolKind && matches.length === 0) {
      process.stderr.write(
        `[DEBUG findSymbolsByName] No matches found for kind "${effectiveSymbolKind}", trying fallback search for all kinds\n`
      );

      const fallbackMatches: SymbolMatch[] = [];

      if (this.isDocumentSymbolArray(symbols)) {
        const flatSymbols = this.flattenDocumentSymbols(symbols);
        for (const symbol of flatSymbols) {
          const nameMatches = symbol.name === symbolName || symbol.name.includes(symbolName);
          if (nameMatches) {
            fallbackMatches.push({
              name: symbol.name,
              kind: symbol.kind,
              position: symbol.selectionRange.start,
              range: symbol.range,
              detail: symbol.detail,
            });
          }
        }
      } else {
        for (const symbol of symbols) {
          const nameMatches = symbol.name === symbolName || symbol.name.includes(symbolName);
          if (nameMatches) {
            const position = await this.findSymbolPositionInFile(filePath, symbol);
            fallbackMatches.push({
              name: symbol.name,
              kind: symbol.kind,
              position: position,
              range: symbol.location.range,
              detail: undefined,
            });
          }
        }
      }

      if (fallbackMatches.length > 0) {
        const foundKinds = [
          ...new Set(fallbackMatches.map((m) => this.symbolKindToString(m.kind))),
        ];
        fallbackWarning = `⚠️ No symbols found with kind "${effectiveSymbolKind}". Found ${fallbackMatches.length} symbol(s) with name "${symbolName}" of other kinds: ${foundKinds.join(', ')}.`;
        matches.push(...fallbackMatches);
        process.stderr.write(
          `[DEBUG findSymbolsByName] Fallback search found ${fallbackMatches.length} additional matches\n`
        );
      }
    }

    const combinedWarning = [validationWarning, fallbackWarning].filter(Boolean).join(' ');
    return { matches, warning: combinedWarning || undefined };
  }

  /**
   * Wait for LSP server to become idle after a change.
   * Uses multiple heuristics to determine when diagnostics are likely complete.
   */
  async waitForDiagnosticsIdle(
    serverState: ServerState,
    fileUri: string,
    options: {
      maxWaitTime?: number; // Maximum time to wait in ms (default: 1000)
      idleTime?: number; // Time without updates to consider idle in ms (default: 100)
      checkInterval?: number; // How often to check for updates in ms (default: 50)
    } = {}
  ): Promise<void> {
    const { maxWaitTime = 1000, idleTime = 100, checkInterval = 50 } = options;

    const startTime = Date.now();
    let lastVersion = serverState.diagnosticVersions.get(fileUri) ?? -1;
    let lastUpdateTime = serverState.lastDiagnosticUpdate.get(fileUri) ?? startTime;

    process.stderr.write(
      `[DEBUG waitForDiagnosticsIdle] Waiting for diagnostics to stabilize for ${fileUri}\n`
    );

    while (Date.now() - startTime < maxWaitTime) {
      await new Promise((resolve) => setTimeout(resolve, checkInterval));

      const currentVersion = serverState.diagnosticVersions.get(fileUri) ?? -1;
      const currentUpdateTime = serverState.lastDiagnosticUpdate.get(fileUri) ?? lastUpdateTime;

      // Check if version changed
      if (currentVersion !== lastVersion) {
        process.stderr.write(
          `[DEBUG waitForDiagnosticsIdle] Version changed from ${lastVersion} to ${currentVersion}\n`
        );
        lastVersion = currentVersion;
        lastUpdateTime = currentUpdateTime;
        continue;
      }

      // Check if enough time has passed without updates
      const timeSinceLastUpdate = Date.now() - currentUpdateTime;
      if (timeSinceLastUpdate >= idleTime) {
        process.stderr.write(
          `[DEBUG waitForDiagnosticsIdle] Server appears idle after ${timeSinceLastUpdate}ms without updates\n`
        );
        return;
      }
    }

    process.stderr.write(
      `[DEBUG waitForDiagnosticsIdle] Max wait time reached (${maxWaitTime}ms)\n`
    );
  }

  async getDiagnostics(filePath: string): Promise<Diagnostic[]> {
    const context: DiagnosticMethods.DiagnosticMethodsContext = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: this.sendRequest.bind(this),
      sendNotification: this.sendNotification.bind(this),
      waitForDiagnosticsIdle: this.waitForDiagnosticsIdle.bind(this),
    };
    return DiagnosticMethods.getDiagnostics(context, filePath);
  }

  async getCodeActions(
    filePath: string,
    range?: { start: Position; end: Position },
    context?: { diagnostics?: Diagnostic[] }
  ): Promise<any[]> {
    const methodContext: DiagnosticMethods.DiagnosticMethodsContext = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: this.sendRequest.bind(this),
      sendNotification: this.sendNotification.bind(this),
      waitForDiagnosticsIdle: this.waitForDiagnosticsIdle.bind(this),
    };
    return DiagnosticMethods.getCodeActions(methodContext, filePath, range, context);
  }

  async formatDocument(
    filePath: string,
    options?: {
      tabSize?: number;
      insertSpaces?: boolean;
      trimTrailingWhitespace?: boolean;
      insertFinalNewline?: boolean;
      trimFinalNewlines?: boolean;
    }
  ): Promise<any[]> {
    const context: DocumentMethods.DocumentMethodsContext = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: this.sendRequest.bind(this),
    };
    return DocumentMethods.formatDocument(context, filePath, options);
  }

  async searchWorkspaceSymbols(query: string): Promise<any[]> {
    const context: WorkspaceMethods.WorkspaceMethodsContext = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: this.sendRequest.bind(this),
      preloadServers: this.preloadServers.bind(this),
      servers: this.servers,
    };
    return WorkspaceMethods.searchWorkspaceSymbols(context, query);
  }

  async preloadServers(debug = true): Promise<void> {
    if (debug) {
      process.stderr.write('Scanning configured server directories for supported file types\n');
    }

    const serversToStart = new Set<LSPServerConfig>();

    // Scan each server's rootDir for its configured extensions
    for (const serverConfig of this.config.servers) {
      const serverDir = serverConfig.rootDir || process.cwd();

      if (debug) {
        process.stderr.write(
          `Scanning ${serverDir} for extensions: ${serverConfig.extensions.join(', ')}\n`
        );
      }

      try {
        const ig = await loadGitignore(serverDir);
        const foundExtensions = await scanDirectoryForExtensions(serverDir, 3, ig, false);

        // Check if any of this server's extensions are found in its rootDir
        const hasMatchingExtensions = serverConfig.extensions.some((ext) =>
          foundExtensions.has(ext)
        );

        if (hasMatchingExtensions) {
          serversToStart.add(serverConfig);
          if (debug) {
            const matchingExts = serverConfig.extensions.filter((ext) => foundExtensions.has(ext));
            process.stderr.write(
              `Found matching extensions in ${serverDir}: ${matchingExts.join(', ')}\n`
            );
          }
        }
      } catch (error) {
        if (debug) {
          process.stderr.write(`Failed to scan ${serverDir}: ${error}\n`);
        }
      }
    }

    if (debug) {
      process.stderr.write(`Starting ${serversToStart.size} LSP servers...\n`);
    }

    const startPromises = Array.from(serversToStart).map(async (serverConfig) => {
      try {
        const key = JSON.stringify(serverConfig);
        if (!this.servers.has(key)) {
          if (debug) {
            process.stderr.write(`Preloading LSP server: ${serverConfig.command.join(' ')}\n`);
          }
          const serverState = await this.startServer(serverConfig);
          this.servers.set(key, serverState);
          if (debug) {
            process.stderr.write(
              `Successfully preloaded LSP server for extensions: ${serverConfig.extensions.join(', ')}\n`
            );
          }
        }
      } catch (error) {
        process.stderr.write(
          `Failed to preload LSP server for ${serverConfig.extensions.join(', ')}: ${error}\n`
        );
      }
    });

    await Promise.all(startPromises);
    if (debug) {
      process.stderr.write('LSP server preloading completed\n');
    }
  }

  // New intelligence methods for LLM agents

  async getHover(filePath: string, position: Position): Promise<import('./types.js').Hover | null> {
    const { getHover } = await import('./lsp-methods/intelligence-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return getHover(context, filePath, position);
  }

  async getCompletions(
    filePath: string,
    position: Position,
    triggerCharacter?: string
  ): Promise<import('./types.js').CompletionItem[]> {
    const { getCompletions } = await import('./lsp-methods/intelligence-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return getCompletions(context, filePath, position, triggerCharacter);
  }

  async getInlayHints(
    filePath: string,
    range: { start: Position; end: Position }
  ): Promise<import('./types.js').InlayHint[]> {
    const { getInlayHints } = await import('./lsp-methods/intelligence-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return getInlayHints(context, filePath, range);
  }

  async getSemanticTokens(filePath: string): Promise<import('./types.js').SemanticTokens | null> {
    const { getSemanticTokens } = await import('./lsp-methods/intelligence-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return getSemanticTokens(context, filePath);
  }

  async getSignatureHelp(
    filePath: string,
    position: Position,
    triggerCharacter?: string
  ): Promise<import('./types.js').SignatureHelp | null> {
    const { getSignatureHelp } = await import('./lsp-methods/intelligence-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return getSignatureHelp(context, filePath, position, triggerCharacter);
  }

  // Hierarchy methods

  async prepareCallHierarchy(
    filePath: string,
    position: Position
  ): Promise<import('./types.js').CallHierarchyItem[]> {
    const { prepareCallHierarchy } = await import('./lsp-methods/hierarchy-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return prepareCallHierarchy(context, filePath, position);
  }

  async getCallHierarchyIncomingCalls(
    item: import('./types.js').CallHierarchyItem
  ): Promise<import('./types.js').CallHierarchyIncomingCall[]> {
    const { getCallHierarchyIncomingCalls } = await import('./lsp-methods/hierarchy-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return getCallHierarchyIncomingCalls(context, item);
  }

  async getCallHierarchyOutgoingCalls(
    item: import('./types.js').CallHierarchyItem
  ): Promise<import('./types.js').CallHierarchyOutgoingCall[]> {
    const { getCallHierarchyOutgoingCalls } = await import('./lsp-methods/hierarchy-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return getCallHierarchyOutgoingCalls(context, item);
  }

  async prepareTypeHierarchy(
    filePath: string,
    position: Position
  ): Promise<import('./types.js').TypeHierarchyItem[]> {
    const { prepareTypeHierarchy } = await import('./lsp-methods/hierarchy-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return prepareTypeHierarchy(context, filePath, position);
  }

  async getTypeHierarchySupertypes(
    item: import('./types.js').TypeHierarchyItem
  ): Promise<import('./types.js').TypeHierarchyItem[]> {
    const { getTypeHierarchySupertypes } = await import('./lsp-methods/hierarchy-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return getTypeHierarchySupertypes(context, item);
  }

  async getTypeHierarchySubtypes(
    item: import('./types.js').TypeHierarchyItem
  ): Promise<import('./types.js').TypeHierarchyItem[]> {
    const { getTypeHierarchySubtypes } = await import('./lsp-methods/hierarchy-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return getTypeHierarchySubtypes(context, item);
  }

  async getSelectionRange(
    filePath: string,
    positions: Position[]
  ): Promise<import('./types.js').SelectionRange[]> {
    const { getSelectionRange } = await import('./lsp-methods/hierarchy-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: (serverState: any, method: string, params: any, timeout?: number) => {
        return this.sendRequest(serverState.process, method, params, timeout);
      },
    };
    return getSelectionRange(context, filePath, positions);
  }

  async getFoldingRanges(filePath: string): Promise<import('./types.js').FoldingRange[]> {
    const { getFoldingRanges } = await import('./lsp-methods/document-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: this.sendRequest.bind(this),
    };
    return getFoldingRanges(context, filePath);
  }

  async getDocumentLinks(filePath: string): Promise<import('./types.js').DocumentLink[]> {
    const { getDocumentLinks } = await import('./lsp-methods/document-methods.js');
    const context = {
      getServer: this.getServer.bind(this),
      ensureFileOpen: this.ensureFileOpen.bind(this),
      sendRequest: this.sendRequest.bind(this),
    };
    return getDocumentLinks(context, filePath);
  }

  // Capability checking methods for feature validation

  /**
   * Check if a server supports a specific capability
   */
  hasCapability(filePath: string, capabilityPath: string): Promise<boolean> {
    return this.getServer(filePath)
      .then((serverState) => {
        return capabilityManager.hasCapability(serverState, capabilityPath);
      })
      .catch(() => false);
  }

  /**
   * Get server capabilities info for debugging
   */
  async getCapabilityInfo(filePath: string): Promise<string> {
    try {
      const serverState = await this.getServer(filePath);
      return capabilityManager.getCapabilityInfo(serverState);
    } catch (error) {
      return `Error getting server: ${error instanceof Error ? error.message : String(error)}`;
    }
  }

  /**
   * Validate required capabilities for a feature
   */
  async validateCapabilities(
    filePath: string,
    requiredCapabilities: string[]
  ): Promise<{
    supported: boolean;
    missing: string[];
    serverDescription: string;
  }> {
    try {
      const serverState = await this.getServer(filePath);
      const validation = capabilityManager.validateRequiredCapabilities(
        serverState,
        requiredCapabilities
      );
      return {
        ...validation,
        serverDescription: capabilityManager.getServerDescription(serverState),
      };
    } catch (error) {
      return {
        supported: false,
        missing: requiredCapabilities,
        serverDescription: 'Unknown Server',
      };
    }
  }

  dispose(): void {
    for (const serverState of this.servers.values()) {
      // Clear restart timer if exists
      if (serverState.restartTimer) {
        clearTimeout(serverState.restartTimer);
      }
      serverState.process.kill();
    }
    this.servers.clear();
  }
}

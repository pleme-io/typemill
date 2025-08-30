import { createRequire } from "node:module";
var __create = Object.create;
var __getProtoOf = Object.getPrototypeOf;
var __defProp = Object.defineProperty;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __toESM = (mod, isNodeMode, target) => {
  target = mod != null ? __create(__getProtoOf(mod)) : {};
  const to = isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target;
  for (let key of __getOwnPropNames(mod))
    if (!__hasOwnProp.call(to, key))
      __defProp(to, key, {
        get: () => mod[key],
        enumerable: true
      });
  return to;
};
var __require = /* @__PURE__ */ createRequire(import.meta.url);

// src/lsp/server-manager.ts
import { spawn } from "node:child_process";

// src/capability-manager.ts
class CapabilityManager {
  capabilityCache = new Map;
  cacheCapabilities(serverKey, initResult) {
    if (initResult && typeof initResult === "object" && "capabilities" in initResult) {
      this.capabilityCache.set(serverKey, initResult.capabilities);
      process.stderr.write(`[DEBUG CapabilityManager] Cached capabilities for ${serverKey}
`);
    } else {
      process.stderr.write(`[DEBUG CapabilityManager] No capabilities found in init result for ${serverKey}
`);
    }
  }
  getCapabilities(serverKeyOrState) {
    if (typeof serverKeyOrState === "string") {
      return this.capabilityCache.get(serverKeyOrState) || null;
    }
    const serverKey = this.getServerKey(serverKeyOrState);
    return this.capabilityCache.get(serverKey) || serverKeyOrState.capabilities;
  }
  hasCapability(serverState, capabilityPath) {
    const capabilities = this.getCapabilities(serverState);
    if (!capabilities) {
      process.stderr.write(`[DEBUG CapabilityManager] No capabilities found for server
`);
      return false;
    }
    const pathParts = capabilityPath.split(".");
    let current = capabilities;
    for (const part of pathParts) {
      if (current && typeof current === "object" && part in current) {
        current = current[part];
      } else {
        process.stderr.write(`[DEBUG CapabilityManager] Capability ${capabilityPath} not found
`);
        return false;
      }
    }
    if (typeof current === "boolean") {
      return current;
    }
    if (current && typeof current === "object") {
      return true;
    }
    process.stderr.write(`[DEBUG CapabilityManager] Capability ${capabilityPath} has unexpected type: ${typeof current}
`);
    return false;
  }
  checkCapability(serverKey, capabilityPath, subCapability) {
    const capabilities = this.getCapabilities(serverKey);
    if (!capabilities) {
      process.stderr.write(`[DEBUG CapabilityManager] No capabilities found for server ${serverKey}
`);
      return false;
    }
    let fullPath = capabilityPath;
    if (subCapability) {
      fullPath = `${capabilityPath}.${subCapability}`;
    }
    const pathParts = fullPath.split(".");
    let current = capabilities;
    for (const part of pathParts) {
      if (current && typeof current === "object" && part in current) {
        current = current[part];
      } else {
        process.stderr.write(`[DEBUG CapabilityManager] Capability ${fullPath} not found for server ${serverKey}
`);
        return false;
      }
    }
    if (typeof current === "boolean") {
      return current;
    }
    if (current && typeof current === "object") {
      return true;
    }
    process.stderr.write(`[DEBUG CapabilityManager] Capability ${fullPath} has unexpected type: ${typeof current} for server ${serverKey}
`);
    return false;
  }
  getSignatureHelpTriggers(serverState) {
    const capabilities = this.getCapabilities(serverState);
    if (capabilities?.signatureHelpProvider?.triggerCharacters) {
      return capabilities.signatureHelpProvider.triggerCharacters;
    }
    return ["(", ","];
  }
  supportsAdvancedWorkspaceEdit(serverState) {
    return this.hasCapability(serverState, "workspace.workspaceEdit.documentChanges");
  }
  supportsFileOperations(serverState) {
    return this.hasCapability(serverState, "workspace.fileOperations");
  }
  getCapabilityInfo(serverState) {
    const capabilities = this.getCapabilities(serverState);
    if (!capabilities) {
      return "No capabilities available";
    }
    const supportedFeatures = [
      "hoverProvider",
      "signatureHelpProvider",
      "definitionProvider",
      "referencesProvider",
      "documentSymbolProvider",
      "workspaceSymbolProvider",
      "codeActionProvider",
      "documentLinkProvider",
      "documentFormattingProvider",
      "renameProvider",
      "foldingRangeProvider",
      "selectionRangeProvider",
      "callHierarchyProvider",
      "semanticTokensProvider",
      "typeHierarchyProvider",
      "inlayHintProvider"
    ].filter((feature) => {
      const value = capabilities[feature];
      return Boolean(value);
    });
    const workspaceFeatures = [];
    if (capabilities.workspace) {
      if (capabilities.workspace.workspaceEdit)
        workspaceFeatures.push("workspaceEdit");
      if (capabilities.workspace.fileOperations)
        workspaceFeatures.push("fileOperations");
      if (capabilities.workspace.workspaceFolders)
        workspaceFeatures.push("workspaceFolders");
    }
    return `Supported features: ${supportedFeatures.join(", ")}
Workspace features: ${workspaceFeatures.join(", ")}`;
  }
  getServerKey(serverState) {
    if (serverState.config?.command) {
      return JSON.stringify(serverState.config.command);
    }
    return "unknown-server";
  }
  validateRequiredCapabilities(serverState, requiredCapabilities) {
    const missing = [];
    for (const capability of requiredCapabilities) {
      if (!this.hasCapability(serverState, capability)) {
        missing.push(capability);
      }
    }
    return {
      supported: missing.length === 0,
      missing
    };
  }
  getServerDescription(serverState) {
    if (serverState.config?.command) {
      const command = serverState.config.command;
      if (Array.isArray(command) && command.length > 0) {
        const serverName = command[0];
        if (serverName?.includes("typescript-language-server"))
          return "TypeScript";
        if (serverName?.includes("pylsp"))
          return "Python (pylsp)";
        if (serverName?.includes("gopls"))
          return "Go (gopls)";
        if (serverName?.includes("rust-analyzer"))
          return "Rust (rust-analyzer)";
        return serverName || "Unknown Server";
      }
      return String(command);
    }
    return "Unknown Server";
  }
}
var capabilityManager = new CapabilityManager;

// src/path-utils.ts
import { fileURLToPath, pathToFileURL } from "node:url";
function pathToUri(filePath) {
  return pathToFileURL(filePath).toString();
}

// src/lsp/server-manager.ts
class ServerManager {
  servers = new Map;
  serversStarting = new Map;
  failedServers = new Set;
  protocol;
  constructor(protocol) {
    this.protocol = protocol;
  }
  get activeServers() {
    return this.servers;
  }
  async getServer(filePath, config) {
    const serverConfig = this.getServerForFile(filePath, config);
    if (!serverConfig) {
      throw new Error(`No language server configured for file: ${filePath}`);
    }
    const serverKey = JSON.stringify(serverConfig.command);
    if (this.failedServers.has(serverKey)) {
      throw new Error(`Language server for ${serverConfig.extensions.join(", ")} files is not available. ` + `Install it with: ${this.getInstallInstructions(serverConfig.command[0])}`);
    }
    const existingServer = this.servers.get(serverKey);
    if (existingServer) {
      if (!existingServer.process.killed) {
        await existingServer.initializationPromise;
        return existingServer;
      }
      this.servers.delete(serverKey);
    }
    const startingPromise = this.serversStarting.get(serverKey);
    if (startingPromise) {
      return await startingPromise;
    }
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
  clearFailedServers() {
    const count = this.failedServers.size;
    this.failedServers.clear();
    if (count > 0) {
      process.stderr.write(`Cleared ${count} failed server(s). They will be retried on next access.
`);
    }
  }
  async restartServer(extensions, config) {
    const restartedServers = [];
    if (!extensions || extensions.length === 0) {
      const serversToRestart = Array.from(this.servers.entries());
      for (const [serverKey, serverState] of serversToRestart) {
        this.killServer(serverState);
        this.servers.delete(serverKey);
        restartedServers.push(serverState.config?.command?.join(" ") || "unknown");
      }
    } else {
      const serversToRestart = Array.from(this.servers.entries()).filter(([, serverState]) => {
        const serverConfig = serverState.config;
        return serverConfig && extensions.some((ext) => serverConfig.extensions.includes(ext));
      });
      for (const [serverKey, serverState] of serversToRestart) {
        this.killServer(serverState);
        this.servers.delete(serverKey);
        restartedServers.push(serverState.config?.command.join(" "));
      }
    }
    return restartedServers;
  }
  async preloadServers(config, extensions) {
    const serverConfigs = new Map;
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
        process.stderr.write(`Preloaded server: ${serverConfig.command.join(" ")}
`);
      } catch (error) {
        process.stderr.write(`Failed to preload server ${serverConfig.command.join(" ")}: ${error}
`);
      }
    });
    await Promise.allSettled(startPromises);
  }
  getServerForFile(filePath, config) {
    const extension = filePath.split(".").pop();
    if (!extension)
      return null;
    process.stderr.write(`Looking for server for extension: ${extension}
`);
    const server = config.servers.find((server2) => server2.extensions.includes(extension));
    if (server) {
      process.stderr.write(`Found server for ${extension}: ${server.command.join(" ")}
`);
    } else {
      process.stderr.write(`No server found for extension: ${extension}
`);
    }
    return server || null;
  }
  async startServer(serverConfig) {
    const [command, ...args] = serverConfig.command;
    if (!command) {
      throw new Error("No command specified in server config");
    }
    if (command === "npx") {
      try {
        const { execSync } = await import("node:child_process");
        execSync("npm --version", { stdio: "ignore" });
      } catch {
        throw new Error("npm is required for TypeScript/JavaScript support. Please install Node.js from https://nodejs.org");
      }
    }
    const childProcess = spawn(command, args, {
      stdio: ["pipe", "pipe", "pipe"],
      cwd: serverConfig.rootDir || process.cwd()
    });
    let startupFailed = false;
    const startupErrorHandler = (error) => {
      startupFailed = true;
      const extensions = serverConfig.extensions.join(", ");
      if (error.message.includes("ENOENT")) {
        process.stderr.write(`⚠️  Language server not found for ${extensions} files
   Command: ${serverConfig.command.join(" ")}
   To enable: ${this.getInstallInstructions(command)}
`);
      } else {
        process.stderr.write(`⚠️  Failed to start language server for ${extensions} files
   Error: ${error.message}
`);
      }
      const serverKey2 = JSON.stringify(serverConfig.command);
      this.failedServers.add(serverKey2);
    };
    childProcess.once("error", startupErrorHandler);
    await new Promise((resolve) => setTimeout(resolve, 100));
    if (startupFailed) {
      throw new Error(`Language server for ${serverConfig.extensions.join(", ")} is not available`);
    }
    childProcess.removeListener("error", startupErrorHandler);
    let initializationResolve;
    const initializationPromise = new Promise((resolve) => {
      initializationResolve = resolve;
    });
    const serverState = {
      process: childProcess,
      initialized: false,
      initializationPromise,
      initializationResolve,
      capabilities: undefined,
      buffer: "",
      openFiles: new Set,
      diagnostics: new Map,
      lastDiagnosticUpdate: new Map,
      diagnosticVersions: new Map,
      restartTimer: undefined,
      config: serverConfig,
      fileVersions: new Map,
      startTime: Date.now()
    };
    this.setupProtocolHandlers(serverState);
    const initResult = await this.initializeServer(serverState, serverConfig);
    const serverKey = JSON.stringify(serverConfig.command);
    capabilityManager.cacheCapabilities(serverKey, initResult);
    if (initResult && typeof initResult === "object" && "capabilities" in initResult) {
      serverState.capabilities = initResult.capabilities;
    }
    this.protocol.sendNotification(childProcess, "initialized", {});
    await new Promise((resolve) => setTimeout(resolve, 500));
    serverState.initialized = true;
    if (serverState.initializationResolve) {
      serverState.initializationResolve();
      serverState.initializationResolve = undefined;
    }
    process.stderr.write(`Server initialized successfully: ${serverConfig.command.join(" ")}
`);
    this.setupRestartTimer(serverState, serverConfig);
    return serverState;
  }
  setupProtocolHandlers(serverState) {
    const serverKey = JSON.stringify(serverState.config?.command);
    serverState.process.stdout?.on("data", (data) => {
      serverState.buffer += data.toString();
      const { messages, remainingBuffer } = this.protocol.parseMessages(serverState.buffer);
      serverState.buffer = remainingBuffer;
      for (const message of messages) {
        this.protocol.handleMessage(message, serverState);
      }
    });
    serverState.process.stderr?.on("data", (data) => {
      process.stderr.write(data);
    });
    serverState.process.on("error", (error) => {
      process.stderr.write(`LSP server process error (${serverState.config?.command.join(" ")}): ${error.message}
`);
      this.servers.delete(serverKey);
    });
    serverState.process.on("exit", (code, signal) => {
      process.stderr.write(`LSP server exited (${serverState.config?.command.join(" ")}): code=${code}, signal=${signal}
`);
      if (serverState.restartTimer) {
        clearTimeout(serverState.restartTimer);
        serverState.restartTimer = undefined;
      }
      this.servers.delete(serverKey);
    });
  }
  async initializeServer(serverState, serverConfig) {
    const initializeParams = {
      processId: serverState.process.pid || null,
      clientInfo: { name: "cclsp", version: "0.5.12" },
      capabilities: {
        textDocument: {
          synchronization: {
            didOpen: true,
            didChange: true,
            didClose: true
          },
          definition: { linkSupport: false },
          references: {
            includeDeclaration: true,
            dynamicRegistration: false
          },
          rename: { prepareSupport: false },
          documentSymbol: {
            symbolKind: {
              valueSet: [
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                9,
                10,
                11,
                12,
                13,
                14,
                15,
                16,
                17,
                18,
                19,
                20,
                21,
                22,
                23,
                24,
                25,
                26
              ]
            },
            hierarchicalDocumentSymbolSupport: true
          },
          completion: {
            completionItem: {
              snippetSupport: true
            }
          },
          hover: {},
          signatureHelp: {},
          diagnostic: {
            dynamicRegistration: false,
            relatedDocumentSupport: false
          }
        },
        workspace: {
          workspaceEdit: {
            documentChanges: true
          },
          workspaceFolders: true
        }
      },
      rootUri: pathToUri(serverConfig.rootDir || process.cwd()),
      workspaceFolders: [
        {
          uri: pathToUri(serverConfig.rootDir || process.cwd()),
          name: "workspace"
        }
      ],
      initializationOptions: this.getInitializationOptions(serverConfig)
    };
    return await this.protocol.sendRequest(serverState.process, "initialize", initializeParams, 1e4);
  }
  getInitializationOptions(serverConfig) {
    if (serverConfig.initializationOptions !== undefined) {
      return serverConfig.initializationOptions;
    }
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
              rope_completion: { enabled: false }
            }
          }
        }
      };
    }
    if (this.isTypeScriptServer(serverConfig)) {
      return {
        hostInfo: "cclsp",
        preferences: {
          includeCompletionsForModuleExports: true,
          includeCompletionsWithInsertText: true
        }
      };
    }
    return;
  }
  setupRestartTimer(serverState, serverConfig) {
    if (serverConfig.restartInterval && serverConfig.restartInterval > 0) {
      const intervalMs = serverConfig.restartInterval * 60 * 1000;
      serverState.restartTimer = setTimeout(() => {
        process.stderr.write(`Auto-restarting server ${serverConfig.command.join(" ")} after ${serverConfig.restartInterval} minutes
`);
        this.killServer(serverState);
        const serverKey = JSON.stringify(serverConfig.command);
        this.servers.delete(serverKey);
      }, intervalMs);
    }
  }
  killServer(serverState) {
    if (serverState.restartTimer) {
      clearTimeout(serverState.restartTimer);
    }
    try {
      if (!serverState.process.killed) {
        serverState.process.kill("SIGTERM");
      }
    } catch (error) {
      process.stderr.write(`Warning: Failed to kill server process (PID: ${serverState.process.pid}): ${error instanceof Error ? error.message : String(error)}
`);
    }
  }
  isPylspServer(serverConfig) {
    return serverConfig.command.some((cmd) => cmd.includes("pylsp"));
  }
  getInstallInstructions(command) {
    const instructions = {
      "typescript-language-server": "npm install -g typescript-language-server typescript",
      pylsp: "pip install python-lsp-server",
      gopls: "go install golang.org/x/tools/gopls@latest",
      "rust-analyzer": "rustup component add rust-analyzer",
      clangd: "apt install clangd OR brew install llvm",
      jdtls: "Download from Eclipse JDT releases",
      solargraph: "gem install solargraph",
      intelephense: "npm install -g intelephense"
    };
    return instructions[command] || `Install ${command} for your system`;
  }
  isTypeScriptServer(serverConfig) {
    return serverConfig.command.some((cmd) => cmd.includes("typescript-language-server") || cmd.includes("tsserver"));
  }
  dispose() {
    for (const serverState of this.servers.values()) {
      this.killServer(serverState);
    }
    this.servers.clear();
    this.serversStarting.clear();
    this.protocol.dispose();
  }
}
export {
  ServerManager
};

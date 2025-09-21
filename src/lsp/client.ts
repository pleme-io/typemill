import { existsSync, readFileSync } from 'node:fs';
import {
  createDefaultConfig,
  getAvailableDefaultServers,
  mergeWithDefaults,
} from '../core/configuration/default-config.js';
import { handleConfigurationError, logError } from '../core/diagnostics/error-utils.js';
import { getLogger } from '../core/diagnostics/structured-logger.js';
import { scanDirectoryForExtensions } from '../core/file-operations/scanner.js';
import type { Config } from '../types.js';
import { LSPProtocol } from './protocol.js';
import { ServerManager } from './server-manager.js';
import type { ServerState } from './types.js';

const logger = getLogger('LSPClient');

/**
 * Main LSP Client facade that coordinates protocol and server management
 * Provides the primary interface for LSP operations
 */
export class LSPClient {
  private config: Config;
  private _protocol: LSPProtocol;
  private _serverManager: ServerManager;

  // Public getters for facade access
  public get protocol(): LSPProtocol {
    return this._protocol;
  }
  public get serverManager(): ServerManager {
    return this._serverManager;
  }

  constructor(configPath?: string) {
    this._protocol = new LSPProtocol();
    this._serverManager = new ServerManager(this._protocol);
    this.config = this.loadConfig(configPath);
  }

  /**
   * Load configuration from environment, file, or use defaults
   */
  private loadConfig(configPath?: string): Config {
    // Try environment variable first (MCP config)
    if (process.env.CODEBUDDY_CONFIG_PATH) {
      logger.info('Loading config from environment variable', {
        config_path: process.env.CODEBUDDY_CONFIG_PATH,
      });

      if (!existsSync(process.env.CODEBUDDY_CONFIG_PATH)) {
        logger.warn('Config file from environment does not exist, falling back to defaults', {
          config_path: process.env.CODEBUDDY_CONFIG_PATH,
        });
        return this.loadDefaultConfig();
      }

      try {
        const configData = readFileSync(process.env.CODEBUDDY_CONFIG_PATH, 'utf-8');
        const config = JSON.parse(configData);
        logger.info('Loaded config from environment variable', {
          server_count: config.servers.length,
        });
        return mergeWithDefaults(config);
      } catch (error) {
        logError('LSPClient', 'Failed to load config from CODEBUDDY_CONFIG_PATH', error, {
          configPath: process.env.CODEBUDDY_CONFIG_PATH,
        });
        logger.warn('Failed to load config from environment, falling back to defaults', {
          config_path: process.env.CODEBUDDY_CONFIG_PATH,
          error_type: error instanceof Error ? error.constructor.name : typeof error,
          error_message: error instanceof Error ? error.message : String(error),
        });
        return this.loadDefaultConfig();
      }
    }

    // Try loading from provided path
    if (configPath) {
      try {
        logger.info('Loading config from file', { config_path: configPath });
        const configData = readFileSync(configPath, 'utf-8');
        const config = JSON.parse(configData);
        logger.info('Loaded server configurations', { server_count: config.servers.length });
        return mergeWithDefaults(config);
      } catch (error) {
        logError('LSPClient', 'Failed to load config from provided path', error, {
          configPath,
        });
        logger.warn('Failed to load config from path, falling back to defaults', {
          config_path: configPath,
          error_type: error instanceof Error ? error.constructor.name : typeof error,
          error_message: error instanceof Error ? error.message : String(error),
        });
        return this.loadDefaultConfig();
      }
    }

    // Try minimal test config first if in test mode
    if (process.env.TEST_MINIMAL_CONFIG === 'true') {
      const testConfigPath = '.codebuddy/test-config.json';
      if (existsSync(testConfigPath)) {
        try {
          logger.info('Using minimal test config for faster startup');
          const configData = readFileSync(testConfigPath, 'utf-8');
          const config = JSON.parse(configData);
          logger.info('Loaded test server configurations', { server_count: config.servers.length });
          return mergeWithDefaults(config);
        } catch (error) {
          logger.warn('Failed to load test config, falling back to normal config');
        }
      }
    }

    // Try .codebuddy/config.json first (new location)
    const newConfigPath = '.codebuddy/config.json';
    if (existsSync(newConfigPath)) {
      try {
        logger.info('Found .codebuddy/config.json, loading');
        const configData = readFileSync(newConfigPath, 'utf-8');
        const config = JSON.parse(configData);
        logger.info('Loaded server configurations', { server_count: config.servers.length });
        return mergeWithDefaults(config);
      } catch (error) {
        logError('LSPClient', 'Failed to load .codebuddy/config.json', error, {
          configPath: newConfigPath,
        });
        logger.warn('Failed to load .codebuddy/config.json', {
          error_type: error instanceof Error ? error.constructor.name : typeof error,
          error_message: error instanceof Error ? error.message : String(error),
        });
      }
    }

    // Try to find codebuddy.json in current directory (legacy)
    const oldConfigPath = 'codebuddy.json';
    if (existsSync(oldConfigPath)) {
      try {
        logger.warn('Found legacy codebuddy.json, consider running: codebuddy init');
        const configData = readFileSync(oldConfigPath, 'utf-8');
        const config = JSON.parse(configData);
        logger.info('Loaded server configurations from legacy file', {
          server_count: config.servers.length,
        });
        return mergeWithDefaults(config);
      } catch (error) {
        logError('LSPClient', 'Failed to load codebuddy.json', error, {
          configPath: oldConfigPath,
        });
        logger.warn('Failed to load codebuddy.json', {
          error_type: error instanceof Error ? error.constructor.name : typeof error,
          error_message: error instanceof Error ? error.message : String(error),
        });
      }
    }

    // Use default configuration
    logger.info('No configuration found, using smart defaults');
    logger.info('To customize configuration, run: codebuddy init');
    return this.loadDefaultConfig();
  }

  /**
   * Load default configuration with all potential language servers
   * Actual availability will be checked when servers are started
   */
  private loadDefaultConfig(): Config {
    const defaultConfig = createDefaultConfig();
    logger.info('Using default configuration', {
      supported_languages: defaultConfig.servers.length,
      bundled_support: 'TypeScript/JavaScript',
      note: 'Other languages work if their servers are installed',
    });
    logger.info('To customize, create a codebuddy.json file or run: codebuddy setup');
    return defaultConfig;
  }

  private getLanguageName(extension: string): string | null {
    const languageMap: Record<string, string> = {
      ts: 'TypeScript',
      tsx: 'TypeScript',
      js: 'JavaScript',
      jsx: 'JavaScript',
      py: 'Python',
      go: 'Go',
      rs: 'Rust',
      java: 'Java',
      rb: 'Ruby',
      php: 'PHP',
      c: 'C',
      cpp: 'C++',
      css: 'CSS',
      html: 'HTML',
      json: 'JSON',
      yaml: 'YAML',
      vue: 'Vue',
      svelte: 'Svelte',
    };
    return languageMap[extension] || null;
  }

  /**
   * Get LSP server for a file path
   */
  async getServer(filePath: string): Promise<ServerState> {
    return await this._serverManager.getServer(filePath, this.config);
  }

  /**
   * Send request through LSP protocol
   */
  async sendRequest(
    serverState: ServerState,
    method: string,
    params: unknown,
    timeout?: number
  ): Promise<unknown> {
    return await this._protocol.sendRequest(serverState.process, method, params, timeout);
  }

  /**
   * Send notification through LSP protocol
   */
  sendNotification(serverState: ServerState, method: string, params: unknown): void {
    this._protocol.sendNotification(serverState.process, method, params);
  }

  /**
   * Sync file content with LSP server after external file changes
   * Sends textDocument/didChange notification to keep server in sync
   */
  async syncFileContent(filePath: string): Promise<void> {
    try {
      const serverState = await this.getServer(filePath);
      if (!serverState || !serverState.initialized) {
        return; // Skip if no server or not initialized
      }

      // Read current file content
      const fileContent = readFileSync(filePath, 'utf-8');
      const fileUri = `file://${filePath}`;

      // Increment version for didChange notification
      const version = (serverState.fileVersions.get(filePath) || 1) + 1;
      serverState.fileVersions.set(filePath, version);

      // Send didChange notification with full content
      this._protocol.sendNotification(serverState.process, 'textDocument/didChange', {
        textDocument: {
          uri: fileUri,
          version,
        },
        contentChanges: [
          {
            // Full content replacement
            text: fileContent,
          },
        ],
      });

      logger.info('Synced file content with LSP server', {
        file_path: filePath,
        version,
      });
    } catch (error) {
      logger.error('Failed to sync file content with LSP server', {
        file_path: filePath,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  /**
   * Restart servers for specified extensions
   */
  async restartServer(extensions?: string[]): Promise<string[]> {
    return await this._serverManager.restartServer(extensions, this.config);
  }

  /**
   * Preload servers for detected file types in project
   */
  async preloadServers(): Promise<void> {
    try {
      const extensions = await scanDirectoryForExtensions(process.cwd());
      await this._serverManager.preloadServers(this.config, Array.from(extensions));
    } catch (error) {
      logError('LSPClient', 'Failed to scan directory for extensions', error, {
        workingDirectory: process.cwd(),
      });
      logger.error('Failed to scan directory for extensions', {
        error_type: error instanceof Error ? error.constructor.name : typeof error,
        error_message: error instanceof Error ? error.message : String(error),
      });
    }
  }

  /**
   * Clean up all resources
   */
  async dispose(): Promise<void> {
    await this._serverManager.dispose();
  }
}

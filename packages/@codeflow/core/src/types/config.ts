/**
 * Configuration type definitions
 * Extracted from src/types.ts during Phase 1 refactoring
 */

export interface LSPServerConfig {
  extensions: string[];
  command: string[];
  rootDir?: string;
  restartInterval?: number; // in minutes, optional auto-restart interval
  initializationOptions?: unknown; // LSP initialization options
}

/**
 * Server-wide options that affect the MCP server behavior
 */
export interface ServerOptions {
  /**
   * Enable predictive loading of imports when files are opened.
   * This pre-warms the LSP server with likely-needed files to reduce latency.
   * Default: true
   */
  enablePredictiveLoading?: boolean;

  /**
   * Maximum depth for recursive predictive loading.
   * 0 = only direct imports, 1 = imports of imports, etc.
   * Default: 0 (only direct imports)
   */
  predictiveLoadingDepth?: number;

  /**
   * File extensions to consider for predictive loading.
   * Default: ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs']
   */
  predictiveLoadingExtensions?: string[];
}

export interface Config {
  servers: LSPServerConfig[];
  serverOptions?: ServerOptions; // Server-wide configuration options
}

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

export interface Config {
  servers: LSPServerConfig[];
}
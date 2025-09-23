/**
 * Internal LSP client types shared across the codebase
 */

import type { ChildProcess } from 'node:child_process';
import type { ServerCapabilities } from '../core/capability-manager.js';
import type { Diagnostic, LSPServerConfig } from '../types.js';

/**
 * Represents the state of an LSP server instance
 */
export interface ServerState {
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
  buffer: string; // Buffer for protocol message parsing
}

/**
 * Base context for LSP method implementations
 */
interface BaseLSPMethodContext {
  getServer: (filePath: string) => Promise<ServerState>;
  ensureFileOpen: (serverState: ServerState, filePath: string) => Promise<void>;
  sendRequest: (
    process: ChildProcess,
    method: string,
    params: unknown,
    timeout?: number
  ) => Promise<unknown>;
  sendNotification: (process: ChildProcess, method: string, params: unknown) => void;
}

/**
 * Context for workspace methods
 */
export interface WorkspaceMethodsContext extends BaseLSPMethodContext {
  preloadServers: (debug?: boolean) => Promise<void>;
  servers: Map<string, ServerState>;
}

/**
 * Context for diagnostic methods
 */
export interface DiagnosticMethodsContext extends BaseLSPMethodContext {
  waitForDiagnosticsIdle: (
    serverState: ServerState,
    fileUri: string,
    options: { maxWaitTime?: number; idleTime?: number; checkInterval?: number }
  ) => Promise<void>;
}

/**
 * Context for hierarchy methods (simplified context)
 */
export interface HierarchyMethodsContext {
  getServer: (filePath: string) => Promise<ServerState>;
  ensureFileOpen: (serverState: ServerState, filePath: string) => Promise<void>;
  sendRequest: (
    serverState: ServerState,
    method: string,
    params: unknown,
    timeout?: number
  ) => Promise<unknown>;
}

/**
 * Context for intelligence methods (simplified - doesn't need sendNotification)
 */
export interface IntelligenceMethodsContext {
  getServer: (filePath: string) => Promise<ServerState>;
  ensureFileOpen: (serverState: ServerState, filePath: string) => Promise<void>;
  sendRequest: (
    serverState: ServerState,
    method: string,
    params: unknown,
    timeout?: number
  ) => Promise<unknown>;
}

/**
 * Context for document methods
 */
export interface DocumentMethodsContext extends BaseLSPMethodContext {
  capabilityManager: {
    getCapabilities(serverKey: string): ServerCapabilities | null;
    checkCapability(
      serverKey: string,
      capabilityPath: string,
      subCapability?: string | null
    ): boolean;
  };
}

/**
 * Context for core methods
 */
export interface CoreMethodsContext extends BaseLSPMethodContext {
  // Core methods use the base context
}

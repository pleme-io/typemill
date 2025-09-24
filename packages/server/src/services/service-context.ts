import { readFileSync } from 'node:fs';
import type { TransactionManager } from '../core/transaction/index.js';
import type { LSPProtocol } from '../lsp/protocol.js';
import type { ServerState } from '../lsp/types.js';

/**
 * Service Context Interface
 * Provides shared infrastructure for all LSP service classes
 */
export interface ServiceContext {
  getServer: (filePath: string, workspaceDir?: string) => Promise<ServerState>;
  protocol: LSPProtocol;
  ensureFileOpen: (serverState: ServerState, filePath: string) => Promise<void>;
  getLanguageId: (filePath: string) => string;
  prepareFile: (filePath: string, workspaceDir?: string) => Promise<ServerState>;
  transactionManager: TransactionManager;
}

/**
 * Service Context Utilities
 * Extracted from duplicated service methods to eliminate code duplication
 */
export const ServiceContextUtils = {
  /**
   * Get LSP language ID from file path extension
   * Centralized language mapping to eliminate duplication across services
   */
  getLanguageId(filePath: string): string {
    const ext = filePath.split('.').pop()?.toLowerCase();
    const languageMap: Record<string, string> = {
      ts: 'typescript',
      tsx: 'typescriptreact',
      js: 'javascript',
      jsx: 'javascriptreact',
      py: 'python',
      go: 'go',
      rs: 'rust',
      java: 'java',
      jar: 'java', // JAR files contain Java bytecode
      class: 'java', // Java class files
      cpp: 'cpp',
      c: 'c',
      h: 'c',
      hpp: 'cpp',
    };
    return languageMap[ext || ''] || 'plaintext';
  },

  /**
   * Ensure file is open in LSP server with proper synchronization
   * Centralized file opening logic to eliminate duplication across services
   */
  async ensureFileOpen(
    serverState: ServerState,
    filePath: string,
    protocol: LSPProtocol
  ): Promise<void> {
    if (serverState.openFiles.has(filePath)) {
      return;
    }

    try {
      const fileContent = readFileSync(filePath, 'utf-8');

      protocol.sendNotification(serverState.process, 'textDocument/didOpen', {
        textDocument: {
          uri: `file://${filePath}`,
          languageId: ServiceContextUtils.getLanguageId(filePath),
          version: 1,
          text: fileContent,
        },
      });

      serverState.openFiles.add(filePath);
    } catch (error) {
      throw new Error(`Failed to open file ${filePath}: ${error}`);
    }
  },

  /**
   * Prepare a file for LSP operations
   * Combines server initialization and file opening into a single operation
   * Eliminates the repetitive 3-step pattern across all services
   */
  async prepareFile(
    filePath: string,
    getServer: (filePath: string, workspaceDir?: string) => Promise<ServerState>,
    ensureFileOpen: (serverState: ServerState, filePath: string) => Promise<void>,
    workspaceDir?: string
  ): Promise<ServerState> {
    const serverState = await getServer(filePath, workspaceDir);

    // Wait for the server to be fully initialized
    await serverState.initializationPromise;

    // Ensure the file is opened and synced with the LSP server
    await ensureFileOpen(serverState, filePath);

    return serverState;
  },

  /**
   * Create a service context with shared utilities
   * Factory function for creating consistent service contexts
   */
  createServiceContext(
    getServer: (filePath: string, workspaceDir?: string) => Promise<ServerState>,
    protocol: LSPProtocol,
    transactionManager: TransactionManager
  ): ServiceContext {
    return {
      getServer,
      protocol,
      ensureFileOpen: async (serverState: ServerState, filePath: string) =>
        ServiceContextUtils.ensureFileOpen(serverState, filePath, protocol),
      getLanguageId: ServiceContextUtils.getLanguageId,
      prepareFile: async (filePath: string, workspaceDir?: string) =>
        ServiceContextUtils.prepareFile(
          filePath,
          getServer,
          async (serverState: ServerState, filePath: string) =>
            ServiceContextUtils.ensureFileOpen(serverState, filePath, protocol),
          workspaceDir
        ),
      transactionManager,
    };
  },
};

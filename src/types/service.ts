/**
 * Service layer type definitions
 * Extracted from service-context.ts during Phase 1 refactoring
 */

import type { LSPProtocol } from '../lsp/protocol.js';
import type { ServerState } from '../lsp/types.js';

/**
 * Service Context Interface
 * Provides shared infrastructure for all LSP service classes
 */
export interface LSPServiceContext {
  getServer: (filePath: string, workspaceDir?: string) => Promise<ServerState>;
  protocol: LSPProtocol;
  ensureFileOpen: (serverState: ServerState, filePath: string) => Promise<void>;
  getLanguageId: (filePath: string) => string;
  prepareFile: (filePath: string, workspaceDir?: string) => Promise<ServerState>;
}
import { readFileSync } from 'node:fs';
import type { ServerState } from '../lsp-types.js';
import type { LSPProtocol } from '../lsp/protocol.js';
import type {
  CallHierarchyIncomingCall,
  CallHierarchyItem,
  CallHierarchyOutgoingCall,
  Position,
  SelectionRange,
  TypeHierarchyItem,
} from '../types.js';

/**
 * Service for hierarchy and navigation-related LSP operations
 * Handles call hierarchy, type hierarchy, and selection ranges
 */
export class HierarchyService {
  constructor(
    private getServer: (filePath: string) => Promise<ServerState>,
    private protocol: LSPProtocol
  ) {}

  /**
   * Prepare call hierarchy at position
   */
  async prepareCallHierarchy(filePath: string, position: Position): Promise<CallHierarchyItem[]> {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    await this.ensureFileOpen(serverState, filePath);

    const response = await this.protocol.sendRequest(
      serverState.process,
      'textDocument/prepareCallHierarchy',
      {
        textDocument: { uri: `file://${filePath}` },
        position,
      }
    );

    return Array.isArray(response) ? response : [];
  }

  /**
   * Get incoming calls for call hierarchy item
   */
  async getCallHierarchyIncomingCalls(
    item: CallHierarchyItem
  ): Promise<CallHierarchyIncomingCall[]> {
    // Extract the file path from the item's URI to determine the correct server
    const filePath = item.uri.replace('file://', '');
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    const response = await this.protocol.sendRequest(
      serverState.process,
      'callHierarchy/incomingCalls',
      {
        item,
      }
    );

    return Array.isArray(response) ? response : [];
  }

  /**
   * Get outgoing calls for call hierarchy item
   */
  async getCallHierarchyOutgoingCalls(
    item: CallHierarchyItem
  ): Promise<CallHierarchyOutgoingCall[]> {
    // Extract the file path from the item's URI to determine the correct server
    const filePath = item.uri.replace('file://', '');
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    const response = await this.protocol.sendRequest(
      serverState.process,
      'callHierarchy/outgoingCalls',
      {
        item,
      }
    );

    return Array.isArray(response) ? response : [];
  }

  /**
   * Prepare type hierarchy at position
   */
  async prepareTypeHierarchy(filePath: string, position: Position): Promise<TypeHierarchyItem[]> {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    await this.ensureFileOpen(serverState, filePath);

    const response = await this.protocol.sendRequest(
      serverState.process,
      'textDocument/prepareTypeHierarchy',
      {
        textDocument: { uri: `file://${filePath}` },
        position,
      }
    );

    return Array.isArray(response) ? response : [];
  }

  /**
   * Get supertypes for type hierarchy item
   */
  async getTypeHierarchySupertypes(item: TypeHierarchyItem): Promise<TypeHierarchyItem[]> {
    // Extract the file path from the item's URI to determine the correct server
    const filePath = item.uri.replace('file://', '');
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    const response = await this.protocol.sendRequest(
      serverState.process,
      'typeHierarchy/supertypes',
      {
        item,
      }
    );

    return Array.isArray(response) ? response : [];
  }

  /**
   * Get subtypes for type hierarchy item
   */
  async getTypeHierarchySubtypes(item: TypeHierarchyItem): Promise<TypeHierarchyItem[]> {
    // Extract the file path from the item's URI to determine the correct server
    const filePath = item.uri.replace('file://', '');
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    const response = await this.protocol.sendRequest(
      serverState.process,
      'typeHierarchy/subtypes',
      {
        item,
      }
    );

    return Array.isArray(response) ? response : [];
  }

  /**
   * Get selection ranges for positions
   */
  async getSelectionRange(filePath: string, positions: Position[]): Promise<SelectionRange[]> {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    await this.ensureFileOpen(serverState, filePath);

    try {
      const response = await this.protocol.sendRequest(
        serverState.process,
        'textDocument/selectionRange',
        {
          textDocument: { uri: `file://${filePath}` },
          positions,
        },
        5000
      ); // 5 second timeout

      return Array.isArray(response) ? response : [];
    } catch (error: unknown) {
      if (error instanceof Error && error.message?.includes('timeout')) {
        throw new Error('Selection range request timed out - TypeScript server may be overloaded');
      }
      throw error;
    }
  }

  /**
   * Ensure file is open in LSP server
   */
  private async ensureFileOpen(serverState: ServerState, filePath: string): Promise<void> {
    if (serverState.openFiles.has(filePath)) {
      return;
    }

    try {
      const fileContent = readFileSync(filePath, 'utf-8');

      this.protocol.sendNotification(serverState.process, 'textDocument/didOpen', {
        textDocument: {
          uri: `file://${filePath}`,
          languageId: this.getLanguageId(filePath),
          version: 1,
          text: fileContent,
        },
      });

      serverState.openFiles.add(filePath);
    } catch (error) {
      throw new Error(
        `Failed to open file for LSP server: ${filePath} - ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  private getLanguageId(filePath: string): string {
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
      cpp: 'cpp',
      c: 'c',
      h: 'c',
      hpp: 'cpp',
    };
    return languageMap[ext || ''] || 'plaintext';
  }
}

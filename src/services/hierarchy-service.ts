import type {
  CallHierarchyIncomingCall,
  CallHierarchyItem,
  CallHierarchyOutgoingCall,
  Position,
  SelectionRange,
  TypeHierarchyItem,
} from '../types.js';
import type { ServiceContext } from './service-context.js';

/**
 * Service for hierarchy and navigation-related LSP operations
 * Handles call hierarchy, type hierarchy, and selection ranges
 */
export class HierarchyService {
  constructor(private context: ServiceContext) {}

  /**
   * Prepare call hierarchy at position
   */
  async prepareCallHierarchy(filePath: string, position: Position): Promise<CallHierarchyItem[]> {
    const serverState = await this.context.prepareFile(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    // Use ProjectScanner to open related files for better cross-file support
    try {
      const { projectScanner } = await import('../utils/project-scanner.js');
      await projectScanner.openRelatedFiles(filePath, this.context, 30);
      process.stderr.write(
        '[DEBUG prepareCallHierarchy] Opened related files for better context\n'
      );
    } catch (error) {
      process.stderr.write(`[DEBUG prepareCallHierarchy] Could not open related files: ${error}\n`);
    }

    const response = await this.context.protocol.sendRequest(
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
    const serverState = await this.context.prepareFile(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    const response = await this.context.protocol.sendRequest(
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
    const serverState = await this.context.prepareFile(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    const response = await this.context.protocol.sendRequest(
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
    const serverState = await this.context.prepareFile(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    // Use ProjectScanner to open related files for better cross-file type resolution
    try {
      const { projectScanner } = await import('../utils/project-scanner.js');
      await projectScanner.openRelatedFiles(filePath, this.context, 30);
      process.stderr.write(
        '[DEBUG prepareTypeHierarchy] Opened related files for better context\n'
      );
    } catch (error) {
      process.stderr.write(`[DEBUG prepareTypeHierarchy] Could not open related files: ${error}\n`);
    }

    const response = await this.context.protocol.sendRequest(
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
    const serverState = await this.context.prepareFile(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    const response = await this.context.protocol.sendRequest(
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
    const serverState = await this.context.prepareFile(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    const response = await this.context.protocol.sendRequest(
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
    const serverState = await this.context.prepareFile(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    try {
      const response = await this.context.protocol.sendRequest(
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

  // ensureFileOpen() and getLanguageId() methods removed - provided by ServiceContext
  // This eliminates ~45 lines of duplicated code from this service
}

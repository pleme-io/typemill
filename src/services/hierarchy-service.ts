import { logDebugMessage } from '../core/diagnostics/debug-logger.js';
import type {
  CallHierarchyIncomingCall,
  CallHierarchyItem,
  CallHierarchyOutgoingCall,
  Position,
  SelectionRange,
  TypeHierarchyItem,
} from '../types.js';
import type { ServiceContext } from './service-context.js';

// Hierarchy service constants
const RELATED_FILES_LIMIT = 30; // Maximum related files to open for context
const SELECTION_RANGE_TIMEOUT_MS = process.env.TEST_MODE ? 10000 : 5000; // Longer timeout in tests

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
      const { projectScanner } = await import('./project-analyzer.js');
      await projectScanner.openRelatedFiles(filePath, this.context, RELATED_FILES_LIMIT);
      logDebugMessage('HierarchyService', 'Opened related files for better context');
    } catch (error) {
      logDebugMessage('HierarchyService', `Could not open related files: ${error}`);
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
      const { projectScanner } = await import('./project-analyzer.js');
      await projectScanner.openRelatedFiles(filePath, this.context, RELATED_FILES_LIMIT);
      logDebugMessage('HierarchyService', 'Opened related files for better context');
    } catch (error) {
      logDebugMessage('HierarchyService', `Could not open related files: ${error}`);
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
        SELECTION_RANGE_TIMEOUT_MS
      ); // Selection range timeout

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

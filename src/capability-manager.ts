// Server capability management for robust cross-language LSP support
// Ensures graceful degradation when servers don't support specific features

import type { ServerState } from './lsp-types.js';

// LSP capability types - simplified version without external dependencies
type ProviderOption = boolean | Record<string, unknown>;

export interface ServerCapabilities {
  textDocumentSync?:
    | number
    | {
        openClose?: boolean;
        change?: number;
        willSave?: boolean;
        willSaveWaitUntil?: boolean;
        save?: boolean | { includeText?: boolean };
      };
  completionProvider?: {
    resolveProvider?: boolean;
    triggerCharacters?: string[];
    allCommitCharacters?: string[];
    workDoneProgress?: boolean;
  };
  hoverProvider?: ProviderOption;
  signatureHelpProvider?: {
    triggerCharacters?: string[];
    retriggerCharacters?: string[];
  };
  definitionProvider?: ProviderOption;
  typeDefinitionProvider?: ProviderOption;
  implementationProvider?: ProviderOption;
  referencesProvider?: ProviderOption;
  documentHighlightProvider?: ProviderOption;
  documentSymbolProvider?: ProviderOption;
  workspaceSymbolProvider?: ProviderOption;
  codeActionProvider?:
    | boolean
    | {
        codeActionKinds?: string[];
        resolveProvider?: boolean;
      };
  codeLensProvider?: {
    resolveProvider?: boolean;
  };
  documentLinkProvider?: {
    resolveProvider?: boolean;
  };
  documentFormattingProvider?: ProviderOption;
  documentRangeFormattingProvider?: ProviderOption;
  documentOnTypeFormattingProvider?: {
    firstTriggerCharacter: string;
    moreTriggerCharacter?: string[];
  };
  renameProvider?:
    | boolean
    | {
        prepareProvider?: boolean;
      };
  foldingRangeProvider?: ProviderOption;
  executeCommandProvider?: {
    commands: string[];
    workDoneProgress?: boolean;
  };
  selectionRangeProvider?: ProviderOption;
  linkedEditingRangeProvider?: ProviderOption;
  callHierarchyProvider?: ProviderOption;
  semanticTokensProvider?: {
    legend: {
      tokenTypes: string[];
      tokenModifiers: string[];
    };
    range?: boolean;
    full?: boolean | { delta?: boolean };
  };
  monikerProvider?: ProviderOption;
  typeHierarchyProvider?: ProviderOption;
  inlineValueProvider?: ProviderOption;
  inlayHintProvider?:
    | boolean
    | {
        resolveProvider?: boolean;
      };
  diagnostic?: {
    identifier?: string;
    interFileDependencies: boolean;
    workspaceDiagnostics: boolean;
  };
  workspace?: {
    workspaceFolders?: {
      supported?: boolean;
      changeNotifications?: boolean | string;
    };
    fileOperations?: {
      didCreate?: FileOperationRegistrationOptions;
      willCreate?: FileOperationRegistrationOptions;
      didRename?: FileOperationRegistrationOptions;
      willRename?: FileOperationRegistrationOptions;
      didDelete?: FileOperationRegistrationOptions;
      willDelete?: FileOperationRegistrationOptions;
    };
    workspaceEdit?: {
      documentChanges?: boolean;
      resourceOperations?: string[];
      failureHandling?: string;
      normalizesLineEndings?: boolean;
      changeAnnotationSupport?: {
        groupsOnLabel?: boolean;
      };
    };
  };
}

interface FileOperationRegistrationOptions {
  filters: Array<{
    scheme?: string;
    pattern: {
      glob: string;
      matches?: 'file' | 'folder';
      options?: { ignoreCase?: boolean };
    };
  }>;
}

// ServerState is now imported from lsp-types.ts

class CapabilityManager {
  private capabilityCache = new Map<string, ServerCapabilities>();

  /**
   * Extract and cache server capabilities from initialization result
   */
  cacheCapabilities(serverKey: string, initResult: unknown): void {
    if (initResult && typeof initResult === 'object' && 'capabilities' in initResult) {
      this.capabilityCache.set(
        serverKey,
        (initResult as { capabilities: ServerCapabilities }).capabilities
      );
      process.stderr.write(`[DEBUG CapabilityManager] Cached capabilities for ${serverKey}\n`);
    } else {
      process.stderr.write(
        `[DEBUG CapabilityManager] No capabilities found in init result for ${serverKey}\n`
      );
    }
  }

  /**
   * Get cached capabilities for a server
   */
  getCapabilities(serverState: ServerState): ServerCapabilities | undefined;
  getCapabilities(serverKey: string): ServerCapabilities | null;
  getCapabilities(serverKeyOrState: ServerState | string): ServerCapabilities | null | undefined {
    if (typeof serverKeyOrState === 'string') {
      return this.capabilityCache.get(serverKeyOrState) || null;
    }
    // ServerState case
    const serverKey = this.getServerKey(serverKeyOrState);
    return this.capabilityCache.get(serverKey) || serverKeyOrState.capabilities;
  }

  /**
   * Check if a server supports a specific capability
   */
  hasCapability(serverState: ServerState, capabilityPath: string): boolean {
    const capabilities = this.getCapabilities(serverState);
    if (!capabilities) {
      process.stderr.write('[DEBUG CapabilityManager] No capabilities found for server\n');
      return false;
    }

    // Navigate nested capability path (e.g., "workspace.workspaceEdit.documentChanges")
    const pathParts = capabilityPath.split('.');
    let current: unknown = capabilities;

    for (const part of pathParts) {
      if (current && typeof current === 'object' && part in current) {
        current = (current as Record<string, unknown>)[part];
      } else {
        process.stderr.write(`[DEBUG CapabilityManager] Capability ${capabilityPath} not found\n`);
        return false;
      }
    }

    // Handle different capability value types
    if (typeof current === 'boolean') {
      return current;
    }

    if (current && typeof current === 'object') {
      return true; // Provider object exists
    }

    process.stderr.write(
      `[DEBUG CapabilityManager] Capability ${capabilityPath} has unexpected type: ${typeof current}\n`
    );
    return false;
  }

  /**
   * Check if a server supports a specific capability (alternate method name for interface compatibility)
   */
  checkCapability(
    serverKey: string,
    capabilityPath: string,
    subCapability?: string | null
  ): boolean {
    // Find the server state by key
    // For now, this is a simplified implementation that assumes serverKey is available
    // In practice, you might need to maintain a mapping of serverKey to ServerState
    const capabilities = this.getCapabilities(serverKey);
    if (!capabilities) {
      process.stderr.write(
        `[DEBUG CapabilityManager] No capabilities found for server ${serverKey}\n`
      );
      return false;
    }

    let fullPath = capabilityPath;
    if (subCapability) {
      fullPath = `${capabilityPath}.${subCapability}`;
    }

    // Navigate nested capability path
    const pathParts = fullPath.split('.');
    let current: unknown = capabilities;

    for (const part of pathParts) {
      if (current && typeof current === 'object' && part in current) {
        current = (current as Record<string, unknown>)[part];
      } else {
        process.stderr.write(
          `[DEBUG CapabilityManager] Capability ${fullPath} not found for server ${serverKey}\n`
        );
        return false;
      }
    }

    // Handle different capability value types
    if (typeof current === 'boolean') {
      return current;
    }

    if (current && typeof current === 'object') {
      return true; // Provider object exists
    }

    process.stderr.write(
      `[DEBUG CapabilityManager] Capability ${fullPath} has unexpected type: ${typeof current} for server ${serverKey}\n`
    );
    return false;
  }

  /**
   * Get signature help trigger characters if supported
   */
  getSignatureHelpTriggers(serverState: ServerState): string[] {
    const capabilities = this.getCapabilities(serverState);
    if (capabilities?.signatureHelpProvider?.triggerCharacters) {
      return capabilities.signatureHelpProvider.triggerCharacters;
    }
    return ['(', ',']; // Common defaults
  }

  /**
   * Check if server supports workspace edit with document changes
   */
  supportsAdvancedWorkspaceEdit(serverState: ServerState): boolean {
    return this.hasCapability(serverState, 'workspace.workspaceEdit.documentChanges');
  }

  /**
   * Check if server supports file operations
   */
  supportsFileOperations(serverState: ServerState): boolean {
    return this.hasCapability(serverState, 'workspace.fileOperations');
  }

  /**
   * Get detailed capability information for debugging
   */
  getCapabilityInfo(serverState: ServerState): string {
    const capabilities = this.getCapabilities(serverState);
    if (!capabilities) {
      return 'No capabilities available';
    }

    const supportedFeatures = [
      'hoverProvider',
      'signatureHelpProvider',
      'definitionProvider',
      'referencesProvider',
      'documentSymbolProvider',
      'workspaceSymbolProvider',
      'codeActionProvider',
      'documentLinkProvider',
      'documentFormattingProvider',
      'renameProvider',
      'foldingRangeProvider',
      'selectionRangeProvider',
      'callHierarchyProvider',
      'semanticTokensProvider',
      'typeHierarchyProvider',
      'inlayHintProvider',
    ].filter((feature) => {
      const value = (capabilities as Record<string, unknown>)[feature];
      return Boolean(value);
    });

    const workspaceFeatures: string[] = [];
    if (capabilities.workspace) {
      if (capabilities.workspace.workspaceEdit) workspaceFeatures.push('workspaceEdit');
      if (capabilities.workspace.fileOperations) workspaceFeatures.push('fileOperations');
      if (capabilities.workspace.workspaceFolders) workspaceFeatures.push('workspaceFolders');
    }

    return `Supported features: ${supportedFeatures.join(', ')}\nWorkspace features: ${workspaceFeatures.join(', ')}`;
  }

  /**
   * Generate server key for caching
   */
  private getServerKey(serverState: ServerState): string {
    if (serverState.config?.command) {
      return JSON.stringify(serverState.config.command);
    }
    return 'unknown-server';
  }

  /**
   * Validate that required capabilities exist for a feature
   */
  validateRequiredCapabilities(
    serverState: ServerState,
    requiredCapabilities: string[]
  ): {
    supported: boolean;
    missing: string[];
  } {
    const missing: string[] = [];

    for (const capability of requiredCapabilities) {
      if (!this.hasCapability(serverState, capability)) {
        missing.push(capability);
      }
    }

    return {
      supported: missing.length === 0,
      missing,
    };
  }

  /**
   * Get human-readable server description for error messages
   */
  getServerDescription(serverState: ServerState): string {
    if (serverState.config?.command) {
      const command = serverState.config.command;
      if (Array.isArray(command) && command.length > 0) {
        const serverName = command[0];
        if (serverName?.includes('typescript-language-server')) return 'TypeScript';
        if (serverName?.includes('pylsp')) return 'Python (pylsp)';
        if (serverName?.includes('gopls')) return 'Go (gopls)';
        if (serverName?.includes('rust-analyzer')) return 'Rust (rust-analyzer)';
        return serverName || 'Unknown Server';
      }
      return String(command);
    }
    return 'Unknown Server';
  }
}

// Global instance for use across the application
export const capabilityManager = new CapabilityManager();

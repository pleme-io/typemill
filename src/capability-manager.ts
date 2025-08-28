// Server capability management for robust cross-language LSP support
// Ensures graceful degradation when servers don't support specific features

export interface ServerCapabilities {
  textDocumentSync?: any;
  completionProvider?: any;
  hoverProvider?: boolean | any;
  signatureHelpProvider?: {
    triggerCharacters?: string[];
    retriggerCharacters?: string[];
  };
  definitionProvider?: boolean | any;
  typeDefinitionProvider?: boolean | any;
  implementationProvider?: boolean | any;
  referencesProvider?: boolean | any;
  documentHighlightProvider?: boolean | any;
  documentSymbolProvider?: boolean | any;
  workspaceSymbolProvider?: boolean | any;
  codeActionProvider?: boolean | any;
  codeLensProvider?: any;
  documentLinkProvider?: {
    resolveProvider?: boolean;
  };
  documentFormattingProvider?: boolean | any;
  documentRangeFormattingProvider?: boolean | any;
  documentOnTypeFormattingProvider?: any;
  renameProvider?: boolean | any;
  foldingRangeProvider?: boolean | any;
  executeCommandProvider?: any;
  selectionRangeProvider?: boolean | any;
  linkedEditingRangeProvider?: boolean | any;
  callHierarchyProvider?: boolean | any;
  semanticTokensProvider?: any;
  monikerProvider?: boolean | any;
  typeHierarchyProvider?: boolean | any;
  inlineValueProvider?: boolean | any;
  inlayHintProvider?: boolean | any;
  diagnostic?: any;
  workspace?: {
    workspaceFolders?: any;
    fileOperations?: {
      didCreate?: any;
      willCreate?: any;
      didRename?: any;
      willRename?: any;
      didDelete?: any;
      willDelete?: any;
    };
    workspaceEdit?: {
      documentChanges?: boolean;
      resourceOperations?: string[];
      failureHandling?: string;
      normalizesLineEndings?: boolean;
      changeAnnotationSupport?: any;
    };
  };
}

interface ServerState {
  process: any;
  config: any;
  initialized: boolean;
  capabilities?: ServerCapabilities;
  initializationPromise: Promise<void>;
  [key: string]: any;
}

class CapabilityManager {
  private capabilityCache = new Map<string, ServerCapabilities>();

  /**
   * Extract and cache server capabilities from initialization result
   */
  cacheCapabilities(serverKey: string, initResult: any): void {
    if (initResult && typeof initResult === 'object' && initResult.capabilities) {
      this.capabilityCache.set(serverKey, initResult.capabilities as ServerCapabilities);
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
  getCapabilities(serverState: ServerState): ServerCapabilities | undefined {
    const serverKey = this.getServerKey(serverState);
    return this.capabilityCache.get(serverKey) || serverState.capabilities;
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
    let current: any = capabilities;

    for (const part of pathParts) {
      if (current && typeof current === 'object' && part in current) {
        current = current[part];
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
      const value = (capabilities as any)[feature];
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
      if (Array.isArray(command)) {
        const serverName = command[0];
        if (serverName.includes('typescript-language-server')) return 'TypeScript';
        if (serverName.includes('pylsp')) return 'Python (pylsp)';
        if (serverName.includes('gopls')) return 'Go (gopls)';
        if (serverName.includes('rust-analyzer')) return 'Rust (rust-analyzer)';
        return serverName;
      }
      return String(command);
    }
    return 'Unknown Server';
  }
}

// Global instance for use across the application
export const capabilityManager = new CapabilityManager();

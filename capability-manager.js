// src/capability-manager.ts
class CapabilityManager {
  capabilityCache = new Map;
  cacheCapabilities(serverKey, initResult) {
    if (initResult && typeof initResult === "object" && "capabilities" in initResult) {
      this.capabilityCache.set(serverKey, initResult.capabilities);
      process.stderr.write(`[DEBUG CapabilityManager] Cached capabilities for ${serverKey}
`);
    } else {
      process.stderr.write(`[DEBUG CapabilityManager] No capabilities found in init result for ${serverKey}
`);
    }
  }
  getCapabilities(serverKeyOrState) {
    if (typeof serverKeyOrState === "string") {
      return this.capabilityCache.get(serverKeyOrState) || null;
    }
    const serverKey = this.getServerKey(serverKeyOrState);
    return this.capabilityCache.get(serverKey) || serverKeyOrState.capabilities;
  }
  hasCapability(serverState, capabilityPath) {
    const capabilities = this.getCapabilities(serverState);
    if (!capabilities) {
      process.stderr.write(`[DEBUG CapabilityManager] No capabilities found for server
`);
      return false;
    }
    const pathParts = capabilityPath.split(".");
    let current = capabilities;
    for (const part of pathParts) {
      if (current && typeof current === "object" && part in current) {
        current = current[part];
      } else {
        process.stderr.write(`[DEBUG CapabilityManager] Capability ${capabilityPath} not found
`);
        return false;
      }
    }
    if (typeof current === "boolean") {
      return current;
    }
    if (current && typeof current === "object") {
      return true;
    }
    process.stderr.write(`[DEBUG CapabilityManager] Capability ${capabilityPath} has unexpected type: ${typeof current}
`);
    return false;
  }
  checkCapability(serverKey, capabilityPath, subCapability) {
    const capabilities = this.getCapabilities(serverKey);
    if (!capabilities) {
      process.stderr.write(`[DEBUG CapabilityManager] No capabilities found for server ${serverKey}
`);
      return false;
    }
    let fullPath = capabilityPath;
    if (subCapability) {
      fullPath = `${capabilityPath}.${subCapability}`;
    }
    const pathParts = fullPath.split(".");
    let current = capabilities;
    for (const part of pathParts) {
      if (current && typeof current === "object" && part in current) {
        current = current[part];
      } else {
        process.stderr.write(`[DEBUG CapabilityManager] Capability ${fullPath} not found for server ${serverKey}
`);
        return false;
      }
    }
    if (typeof current === "boolean") {
      return current;
    }
    if (current && typeof current === "object") {
      return true;
    }
    process.stderr.write(`[DEBUG CapabilityManager] Capability ${fullPath} has unexpected type: ${typeof current} for server ${serverKey}
`);
    return false;
  }
  getSignatureHelpTriggers(serverState) {
    const capabilities = this.getCapabilities(serverState);
    if (capabilities?.signatureHelpProvider?.triggerCharacters) {
      return capabilities.signatureHelpProvider.triggerCharacters;
    }
    return ["(", ","];
  }
  supportsAdvancedWorkspaceEdit(serverState) {
    return this.hasCapability(serverState, "workspace.workspaceEdit.documentChanges");
  }
  supportsFileOperations(serverState) {
    return this.hasCapability(serverState, "workspace.fileOperations");
  }
  getCapabilityInfo(serverState) {
    const capabilities = this.getCapabilities(serverState);
    if (!capabilities) {
      return "No capabilities available";
    }
    const supportedFeatures = [
      "hoverProvider",
      "signatureHelpProvider",
      "definitionProvider",
      "referencesProvider",
      "documentSymbolProvider",
      "workspaceSymbolProvider",
      "codeActionProvider",
      "documentLinkProvider",
      "documentFormattingProvider",
      "renameProvider",
      "foldingRangeProvider",
      "selectionRangeProvider",
      "callHierarchyProvider",
      "semanticTokensProvider",
      "typeHierarchyProvider",
      "inlayHintProvider"
    ].filter((feature) => {
      const value = capabilities[feature];
      return Boolean(value);
    });
    const workspaceFeatures = [];
    if (capabilities.workspace) {
      if (capabilities.workspace.workspaceEdit)
        workspaceFeatures.push("workspaceEdit");
      if (capabilities.workspace.fileOperations)
        workspaceFeatures.push("fileOperations");
      if (capabilities.workspace.workspaceFolders)
        workspaceFeatures.push("workspaceFolders");
    }
    return `Supported features: ${supportedFeatures.join(", ")}
Workspace features: ${workspaceFeatures.join(", ")}`;
  }
  getServerKey(serverState) {
    if (serverState.config?.command) {
      return JSON.stringify(serverState.config.command);
    }
    return "unknown-server";
  }
  validateRequiredCapabilities(serverState, requiredCapabilities) {
    const missing = [];
    for (const capability of requiredCapabilities) {
      if (!this.hasCapability(serverState, capability)) {
        missing.push(capability);
      }
    }
    return {
      supported: missing.length === 0,
      missing
    };
  }
  getServerDescription(serverState) {
    if (serverState.config?.command) {
      const command = serverState.config.command;
      if (Array.isArray(command) && command.length > 0) {
        const serverName = command[0];
        if (serverName?.includes("typescript-language-server"))
          return "TypeScript";
        if (serverName?.includes("pylsp"))
          return "Python (pylsp)";
        if (serverName?.includes("gopls"))
          return "Go (gopls)";
        if (serverName?.includes("rust-analyzer"))
          return "Rust (rust-analyzer)";
        return serverName || "Unknown Server";
      }
      return String(command);
    }
    return "Unknown Server";
  }
}
var capabilityManager = new CapabilityManager;
export {
  capabilityManager
};

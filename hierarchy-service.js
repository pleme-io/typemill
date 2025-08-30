// src/services/hierarchy-service.ts
import { readFileSync } from "node:fs";

class HierarchyService {
  getServer;
  protocol;
  constructor(getServer, protocol) {
    this.getServer = getServer;
    this.protocol = protocol;
  }
  async prepareCallHierarchy(filePath, position) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    const response = await this.protocol.sendRequest(serverState.process, "textDocument/prepareCallHierarchy", {
      textDocument: { uri: `file://${filePath}` },
      position
    });
    return Array.isArray(response) ? response : [];
  }
  async getCallHierarchyIncomingCalls(item) {
    const filePath = item.uri.replace("file://", "");
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    const response = await this.protocol.sendRequest(serverState.process, "callHierarchy/incomingCalls", {
      item
    });
    return Array.isArray(response) ? response : [];
  }
  async getCallHierarchyOutgoingCalls(item) {
    const filePath = item.uri.replace("file://", "");
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    const response = await this.protocol.sendRequest(serverState.process, "callHierarchy/outgoingCalls", {
      item
    });
    return Array.isArray(response) ? response : [];
  }
  async prepareTypeHierarchy(filePath, position) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    const response = await this.protocol.sendRequest(serverState.process, "textDocument/prepareTypeHierarchy", {
      textDocument: { uri: `file://${filePath}` },
      position
    });
    return Array.isArray(response) ? response : [];
  }
  async getTypeHierarchySupertypes(item) {
    const filePath = item.uri.replace("file://", "");
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    const response = await this.protocol.sendRequest(serverState.process, "typeHierarchy/supertypes", {
      item
    });
    return Array.isArray(response) ? response : [];
  }
  async getTypeHierarchySubtypes(item) {
    const filePath = item.uri.replace("file://", "");
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    const response = await this.protocol.sendRequest(serverState.process, "typeHierarchy/subtypes", {
      item
    });
    return Array.isArray(response) ? response : [];
  }
  async getSelectionRange(filePath, positions) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    try {
      const response = await this.protocol.sendRequest(serverState.process, "textDocument/selectionRange", {
        textDocument: { uri: `file://${filePath}` },
        positions
      }, 5000);
      return Array.isArray(response) ? response : [];
    } catch (error) {
      if (error instanceof Error && error.message?.includes("timeout")) {
        throw new Error("Selection range request timed out - TypeScript server may be overloaded");
      }
      throw error;
    }
  }
  async ensureFileOpen(serverState, filePath) {
    if (serverState.openFiles.has(filePath)) {
      return;
    }
    try {
      const fileContent = readFileSync(filePath, "utf-8");
      this.protocol.sendNotification(serverState.process, "textDocument/didOpen", {
        textDocument: {
          uri: `file://${filePath}`,
          languageId: this.getLanguageId(filePath),
          version: 1,
          text: fileContent
        }
      });
      serverState.openFiles.add(filePath);
    } catch (error) {
      throw new Error(`Failed to open file for LSP server: ${filePath} - ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  getLanguageId(filePath) {
    const ext = filePath.split(".").pop()?.toLowerCase();
    const languageMap = {
      ts: "typescript",
      tsx: "typescriptreact",
      js: "javascript",
      jsx: "javascriptreact",
      py: "python",
      go: "go",
      rs: "rust",
      java: "java",
      cpp: "cpp",
      c: "c",
      h: "c",
      hpp: "cpp"
    };
    return languageMap[ext || ""] || "plaintext";
  }
}
export {
  HierarchyService
};

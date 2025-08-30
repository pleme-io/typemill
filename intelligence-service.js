// src/services/intelligence-service.ts
import { readFileSync } from "node:fs";

class IntelligenceService {
  getServer;
  protocol;
  constructor(getServer, protocol) {
    this.getServer = getServer;
    this.protocol = protocol;
  }
  async getHover(filePath, position) {
    console.error("[DEBUG getHover] Starting hover request for", filePath);
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    console.error("[DEBUG getHover] Got server state");
    await this.ensureFileOpen(serverState, filePath);
    console.error("[DEBUG getHover] File opened");
    await new Promise((resolve) => setTimeout(resolve, 500));
    console.error("[DEBUG getHover] Waited for TS to process");
    console.error("[DEBUG getHover] Calling sendRequest with 30s timeout");
    try {
      const response = await this.protocol.sendRequest(serverState.process, "textDocument/hover", {
        textDocument: { uri: `file://${filePath}` },
        position
      }, 30000);
      console.error("[DEBUG getHover] Got response:", response);
      return response && typeof response === "object" && "contents" in response ? response : null;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error("[DEBUG getHover] Error:", errorMessage);
      if (error instanceof Error && error.message?.includes("timeout")) {
        return {
          contents: {
            kind: "markdown",
            value: `**Hover information unavailable**

The TypeScript Language Server did not respond to the hover request at line ${position.line + 1}, character ${position.character + 1}. This feature may not be fully supported in the current server configuration.`
          }
        };
      }
      throw error;
    }
  }
  async getCompletions(filePath, position, triggerCharacter) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    await new Promise((resolve) => setTimeout(resolve, 500));
    const completionParams = {
      textDocument: { uri: `file://${filePath}` },
      position,
      context: triggerCharacter ? {
        triggerKind: 2,
        triggerCharacter
      } : {
        triggerKind: 1
      }
    };
    try {
      const response = await this.protocol.sendRequest(serverState.process, "textDocument/completion", completionParams, 5000);
      if (!response || typeof response !== "object")
        return [];
      const result = response;
      return Array.isArray(result.items) ? result.items : result.items || [];
    } catch (error) {
      if (error instanceof Error && error.message?.includes("timeout")) {
        return [
          {
            label: "Completions unavailable",
            detail: "TypeScript Language Server timeout",
            documentation: "The TypeScript Language Server did not respond to the completion request. This feature may not be fully supported in the current server configuration.",
            insertText: "",
            kind: 1
          }
        ];
      }
      throw error;
    }
  }
  async getSignatureHelp(filePath, position, triggerCharacter) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    const signatureHelpParams = {
      textDocument: { uri: `file://${filePath}` },
      position,
      context: triggerCharacter ? {
        triggerKind: 2,
        triggerCharacter,
        isRetrigger: false
      } : {
        triggerKind: 1,
        isRetrigger: false
      }
    };
    const response = await this.protocol.sendRequest(serverState.process, "textDocument/signatureHelp", signatureHelpParams);
    return response && typeof response === "object" && "signatures" in response ? response : null;
  }
  async getInlayHints(filePath, range) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    const inlayHintParams = {
      textDocument: { uri: `file://${filePath}` },
      range
    };
    const response = await this.protocol.sendRequest(serverState.process, "textDocument/inlayHint", inlayHintParams);
    return Array.isArray(response) ? response : [];
  }
  async getSemanticTokens(filePath) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    const semanticTokensParams = {
      textDocument: { uri: `file://${filePath}` }
    };
    const response = await this.protocol.sendRequest(serverState.process, "textDocument/semanticTokens/full", semanticTokensParams);
    return response && typeof response === "object" && "data" in response ? response : null;
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
  IntelligenceService
};

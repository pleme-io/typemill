// src/services/file-service.ts
import { readFileSync } from "node:fs";

// src/path-utils.ts
import { fileURLToPath, pathToFileURL } from "node:url";
function pathToUri(filePath) {
  return pathToFileURL(filePath).toString();
}

// src/services/file-service.ts
class FileService {
  getServer;
  protocol;
  constructor(getServer, protocol) {
    this.getServer = getServer;
    this.protocol = protocol;
  }
  async formatDocument(filePath, options) {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error("Server not initialized");
    }
    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);
    const formattingOptions = {
      tabSize: options?.tabSize || 2,
      insertSpaces: options?.insertSpaces !== false,
      ...options?.trimTrailingWhitespace !== undefined && {
        trimTrailingWhitespace: options.trimTrailingWhitespace
      },
      ...options?.insertFinalNewline !== undefined && {
        insertFinalNewline: options.insertFinalNewline
      },
      ...options?.trimFinalNewlines !== undefined && {
        trimFinalNewlines: options.trimFinalNewlines
      }
    };
    const result = await this.protocol.sendRequest(serverState.process, "textDocument/formatting", {
      textDocument: { uri: fileUri },
      options: formattingOptions
    });
    return Array.isArray(result) ? result : [];
  }
  async getCodeActions(filePath, range, context) {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error("Server not initialized");
    }
    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);
    const diagnostics = serverState.diagnostics.get(fileUri) || [];
    const requestRange = range || {
      start: { line: 0, character: 0 },
      end: { line: Math.min(100, 999999), character: 0 }
    };
    const codeActionContext = {
      diagnostics: context?.diagnostics || diagnostics,
      only: undefined
    };
    process.stderr.write(`[DEBUG getCodeActions] Request params: ${JSON.stringify({
      textDocument: { uri: fileUri },
      range: requestRange,
      context: codeActionContext
    }, null, 2)}
`);
    try {
      const result = await this.protocol.sendRequest(serverState.process, "textDocument/codeAction", {
        textDocument: { uri: fileUri },
        range: requestRange,
        context: codeActionContext
      });
      process.stderr.write(`[DEBUG getCodeActions] Raw result: ${JSON.stringify(result)}
`);
      if (!result)
        return [];
      if (Array.isArray(result))
        return result.filter((action) => action != null);
      return [];
    } catch (error) {
      process.stderr.write(`[DEBUG getCodeActions] Error: ${error}
`);
      return [];
    }
  }
  async getFoldingRanges(filePath) {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error("Server not initialized");
    }
    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);
    process.stderr.write(`[DEBUG getFoldingRanges] Requesting folding ranges for: ${filePath}
`);
    const result = await this.protocol.sendRequest(serverState.process, "textDocument/foldingRange", {
      textDocument: { uri: fileUri }
    });
    process.stderr.write(`[DEBUG getFoldingRanges] Result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : "N/A"}
`);
    if (Array.isArray(result)) {
      return result;
    }
    return [];
  }
  async getDocumentLinks(filePath) {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error("Server not initialized");
    }
    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);
    process.stderr.write(`[DEBUG getDocumentLinks] Requesting document links for: ${filePath}
`);
    const result = await this.protocol.sendRequest(serverState.process, "textDocument/documentLink", {
      textDocument: { uri: fileUri }
    });
    process.stderr.write(`[DEBUG getDocumentLinks] Result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : "N/A"}
`);
    if (Array.isArray(result)) {
      return result;
    }
    return [];
  }
  async applyWorkspaceEdit(edit) {
    try {
      if (edit.changes) {
        for (const [uri, edits] of Object.entries(edit.changes)) {
          const filePath = uri.replace("file://", "");
          await this.applyTextEdits(filePath, edits);
        }
      }
      if (edit.documentChanges) {
        for (const change of edit.documentChanges) {
          const filePath = change.textDocument.uri.replace("file://", "");
          await this.applyTextEdits(filePath, change.edits);
        }
      }
      return { applied: true };
    } catch (error) {
      return {
        applied: false,
        failureReason: error instanceof Error ? error.message : String(error)
      };
    }
  }
  async renameFile(oldPath, newPath) {
    try {
      const serverConfigs = new Map;
      for (const serverState of serverConfigs.values()) {
        this.protocol.sendNotification(serverState.process, "workspace/willRenameFiles", {
          files: [
            {
              oldUri: `file://${oldPath}`,
              newUri: `file://${newPath}`
            }
          ]
        });
      }
    } catch (error) {
      process.stderr.write(`[ERROR renameFile] ${error}
`);
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
  async applyTextEdits(filePath, edits) {
    if (edits.length === 0)
      return;
    try {
      const fileContent = readFileSync(filePath, "utf-8");
      const lines = fileContent.split(`
`);
      const sortedEdits = [...edits].sort((a, b) => {
        if (a.range.start.line !== b.range.start.line) {
          return b.range.start.line - a.range.start.line;
        }
        return b.range.start.character - a.range.start.character;
      });
      for (const edit of sortedEdits) {
        const startLine = edit.range.start.line;
        const startChar = edit.range.start.character;
        const endLine = edit.range.end.line;
        const endChar = edit.range.end.character;
        if (startLine === endLine) {
          const line = lines[startLine];
          if (line !== undefined) {
            lines[startLine] = line.substring(0, startChar) + edit.newText + line.substring(endChar);
          }
        } else {
          const newLines = edit.newText.split(`
`);
          const startLineText = lines[startLine];
          const endLineText = lines[endLine];
          if (startLineText !== undefined && endLineText !== undefined) {
            const firstLine = startLineText.substring(0, startChar) + newLines[0];
            const lastLine = newLines[newLines.length - 1] + endLineText.substring(endChar);
            const replacementLines = [firstLine, ...newLines.slice(1, -1), lastLine];
            lines.splice(startLine, endLine - startLine + 1, ...replacementLines);
          }
        }
      }
      process.stderr.write(`[DEBUG applyTextEdits] Would apply ${edits.length} edits to ${filePath}
`);
    } catch (error) {
      throw new Error(`Failed to apply text edits to ${filePath}: ${error}`);
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
  async syncFileContent(filePath) {
    try {
      const serverState = await this.getServer(filePath);
      if (!serverState.openFiles.has(filePath)) {
        process.stderr.write(`[DEBUG syncFileContent] File not open, opening it first: ${filePath}
`);
        await this.ensureFileOpen(serverState, filePath);
      }
      process.stderr.write(`[DEBUG syncFileContent] Syncing file: ${filePath}
`);
      const fileContent = readFileSync(filePath, "utf-8");
      const uri = pathToUri(filePath);
      const version = (serverState.fileVersions.get(filePath) || 1) + 1;
      serverState.fileVersions.set(filePath, version);
      await this.protocol.sendNotification(serverState.process, "textDocument/didChange", {
        textDocument: {
          uri,
          version
        },
        contentChanges: [
          {
            text: fileContent
          }
        ]
      });
      process.stderr.write(`[DEBUG syncFileContent] File synced with version ${version}: ${filePath}
`);
    } catch (error) {
      process.stderr.write(`[DEBUG syncFileContent] Failed to sync file ${filePath}: ${error}
`);
    }
  }
}
export {
  FileService
};

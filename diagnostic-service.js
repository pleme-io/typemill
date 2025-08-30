// src/services/diagnostic-service.ts
import { readFileSync } from "node:fs";

// src/path-utils.ts
import { fileURLToPath, pathToFileURL } from "node:url";
function pathToUri(filePath) {
  return pathToFileURL(filePath).toString();
}

// src/services/diagnostic-service.ts
class DiagnosticService {
  getServer;
  protocol;
  constructor(getServer, protocol) {
    this.getServer = getServer;
    this.protocol = protocol;
  }
  async getDiagnostics(filePath) {
    process.stderr.write(`[DEBUG getDiagnostics] Requesting diagnostics for ${filePath}
`);
    const serverState = await this.getServer(filePath);
    await serverState.initializationPromise;
    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);
    const cachedDiagnostics = serverState.diagnostics.get(fileUri);
    if (cachedDiagnostics !== undefined) {
      process.stderr.write(`[DEBUG getDiagnostics] Returning ${cachedDiagnostics.length} cached diagnostics from publishDiagnostics
`);
      return cachedDiagnostics;
    }
    process.stderr.write(`[DEBUG getDiagnostics] No cached diagnostics, trying textDocument/diagnostic request
`);
    try {
      const result = await this.protocol.sendRequest(serverState.process, "textDocument/diagnostic", {
        textDocument: { uri: fileUri }
      });
      process.stderr.write(`[DEBUG getDiagnostics] Result type: ${typeof result}, has kind: ${result && typeof result === "object" && "kind" in result}
`);
      process.stderr.write(`[DEBUG getDiagnostics] Full result: ${JSON.stringify(result)}
`);
      if (result && typeof result === "object" && "kind" in result) {
        const report = result;
        if (report.kind === "full" && report.items) {
          process.stderr.write(`[DEBUG getDiagnostics] Full report with ${report.items.length} diagnostics
`);
          return report.items;
        }
        if (report.kind === "unchanged") {
          process.stderr.write(`[DEBUG getDiagnostics] Unchanged report (no new diagnostics)
`);
          return [];
        }
      }
      if (Array.isArray(result)) {
        process.stderr.write(`[DEBUG getDiagnostics] Direct diagnostic array with ${result.length} diagnostics
`);
        return result;
      }
      if (result === null || result === undefined) {
        process.stderr.write(`[DEBUG getDiagnostics] Null/undefined result, falling back to other methods
`);
      } else {
        process.stderr.write(`[DEBUG getDiagnostics] Unexpected response format, falling back to other methods
`);
      }
    } catch (error) {
      process.stderr.write(`[DEBUG getDiagnostics] textDocument/diagnostic not supported or failed: ${error}. Waiting for publishDiagnostics...
`);
      await this.waitForDiagnosticsIdle(serverState, fileUri, {
        maxWaitTime: 5000,
        idleTime: 300
      });
      const diagnosticsAfterWait = serverState.diagnostics.get(fileUri);
      if (diagnosticsAfterWait !== undefined) {
        process.stderr.write(`[DEBUG getDiagnostics] Returning ${diagnosticsAfterWait.length} diagnostics after waiting for idle state
`);
        return diagnosticsAfterWait;
      }
      process.stderr.write(`[DEBUG getDiagnostics] No diagnostics yet, triggering publishDiagnostics with no-op change
`);
      try {
        const fileContent = readFileSync(filePath, "utf-8");
        const version1 = (serverState.fileVersions.get(filePath) || 1) + 1;
        serverState.fileVersions.set(filePath, version1);
        await this.protocol.sendNotification(serverState.process, "textDocument/didChange", {
          textDocument: {
            uri: fileUri,
            version: version1
          },
          contentChanges: [
            {
              text: `${fileContent} `
            }
          ]
        });
        const version2 = version1 + 1;
        serverState.fileVersions.set(filePath, version2);
        await this.protocol.sendNotification(serverState.process, "textDocument/didChange", {
          textDocument: {
            uri: fileUri,
            version: version2
          },
          contentChanges: [
            {
              text: fileContent
            }
          ]
        });
        await this.waitForDiagnosticsIdle(serverState, fileUri, {
          maxWaitTime: 3000,
          idleTime: 300
        });
        const diagnosticsAfterTrigger = serverState.diagnostics.get(fileUri);
        if (diagnosticsAfterTrigger !== undefined) {
          process.stderr.write(`[DEBUG getDiagnostics] Returning ${diagnosticsAfterTrigger.length} diagnostics after triggering publishDiagnostics
`);
          return diagnosticsAfterTrigger;
        }
      } catch (triggerError) {
        process.stderr.write(`[DEBUG getDiagnostics] Failed to trigger publishDiagnostics: ${triggerError}
`);
      }
      return [];
    }
  }
  filterDiagnosticsByLevel(diagnostics, minSeverity) {
    return diagnostics.filter((diagnostic) => diagnostic.severity === undefined || diagnostic.severity <= minSeverity);
  }
  getRelatedDiagnostics(diagnostics, position) {
    return diagnostics.filter((diagnostic) => {
      const range = diagnostic.range;
      return position.line >= range.start.line && position.line <= range.end.line && (position.line !== range.start.line || position.character >= range.start.character) && (position.line !== range.end.line || position.character <= range.end.character);
    });
  }
  categorizeDiagnostics(diagnostics) {
    const errors = [];
    const warnings = [];
    const infos = [];
    const hints = [];
    for (const diagnostic of diagnostics) {
      switch (diagnostic.severity) {
        case 1:
          errors.push(diagnostic);
          break;
        case 2:
          warnings.push(diagnostic);
          break;
        case 3:
          infos.push(diagnostic);
          break;
        case 4:
          hints.push(diagnostic);
          break;
        default:
          errors.push(diagnostic);
      }
    }
    return { errors, warnings, infos, hints };
  }
  async waitForDiagnosticsIdle(serverState, fileUri, options = {}) {
    const {
      maxWaitTime = 1e4,
      idleTime = 1000,
      checkInterval = 100
    } = options;
    const startTime = Date.now();
    let lastUpdateTime = serverState.lastDiagnosticUpdate.get(fileUri) || 0;
    return new Promise((resolve) => {
      const checkIdle = () => {
        const now = Date.now();
        const currentUpdateTime = serverState.lastDiagnosticUpdate.get(fileUri) || 0;
        if (now - startTime >= maxWaitTime) {
          process.stderr.write(`[DEBUG waitForDiagnosticsIdle] Max wait time reached for ${fileUri}
`);
          resolve();
          return;
        }
        if (currentUpdateTime > lastUpdateTime) {
          lastUpdateTime = currentUpdateTime;
          setTimeout(checkIdle, checkInterval);
          return;
        }
        if (now - lastUpdateTime >= idleTime) {
          process.stderr.write(`[DEBUG waitForDiagnosticsIdle] Diagnostics idle for ${fileUri}
`);
          resolve();
          return;
        }
        setTimeout(checkIdle, checkInterval);
      };
      setTimeout(checkIdle, checkInterval);
    });
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
  DiagnosticService
};

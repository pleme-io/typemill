// src/lsp/protocol.ts
class LSPProtocol {
  nextId = 1;
  pendingRequests = new Map;
  async sendRequest(process2, method, params, timeout = 30000) {
    return new Promise((resolve, reject) => {
      const id = this.nextId++;
      const message = {
        jsonrpc: "2.0",
        id,
        method,
        params
      };
      this.pendingRequests.set(id, { resolve, reject });
      const timeoutId = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request timed out after ${timeout}ms: ${method}`));
      }, timeout);
      const originalResolve = resolve;
      const originalReject = reject;
      this.pendingRequests.set(id, {
        resolve: (value) => {
          clearTimeout(timeoutId);
          originalResolve(value);
        },
        reject: (reason) => {
          clearTimeout(timeoutId);
          originalReject(reason);
        }
      });
      this.sendMessage(process2, message);
    });
  }
  sendNotification(process2, method, params) {
    const message = {
      jsonrpc: "2.0",
      method,
      params
    };
    this.sendMessage(process2, message);
  }
  handleMessage(message, serverState) {
    if (message.id && this.pendingRequests.has(message.id)) {
      const request = this.pendingRequests.get(message.id);
      if (!request)
        return;
      const { resolve, reject } = request;
      this.pendingRequests.delete(message.id);
      if (message.error) {
        if (message.error.code === -32601 || message.error.message?.toLowerCase().includes("unhandled method") || message.error.message?.toLowerCase().includes("method not found")) {
          resolve(null);
        } else {
          reject(new Error(message.error.message || "LSP Error"));
        }
      } else {
        resolve(message.result);
      }
    }
    if (message.method && serverState) {
      this.handleServerNotification(message, serverState);
    }
  }
  parseMessages(buffer) {
    const messages = [];
    let remaining = buffer;
    while (true) {
      const headerEndIndex = remaining.indexOf(`\r
\r
`);
      if (headerEndIndex === -1)
        break;
      const headers = remaining.substring(0, headerEndIndex);
      const contentLengthMatch = headers.match(/Content-Length: (\d+)/);
      if (!contentLengthMatch || !contentLengthMatch[1]) {
        remaining = remaining.substring(headerEndIndex + 4);
        continue;
      }
      const contentLength = Number.parseInt(contentLengthMatch[1], 10);
      const messageStart = headerEndIndex + 4;
      if (remaining.length < messageStart + contentLength)
        break;
      const messageContent = remaining.substring(messageStart, messageStart + contentLength);
      try {
        const message = JSON.parse(messageContent);
        messages.push(message);
      } catch (error) {
        process.stderr.write(`[ERROR] Failed to parse LSP message: ${error}
`);
      }
      remaining = remaining.substring(messageStart + contentLength);
    }
    return { messages, remainingBuffer: remaining };
  }
  sendMessage(process2, message) {
    const content = JSON.stringify(message);
    const header = `Content-Length: ${Buffer.byteLength(content)}\r
\r
`;
    process2.stdin?.write(header + content);
  }
  handleServerNotification(message, serverState) {
    if (message.method === "initialized") {
      process.stderr.write(`[DEBUG] Received initialized notification from server
`);
      serverState.initialized = true;
      if (serverState.initializationResolve) {
        serverState.initializationResolve();
        serverState.initializationResolve = undefined;
      }
    } else if (message.method === "textDocument/publishDiagnostics") {
      const params = message.params;
      if (params?.uri) {
        process.stderr.write(`[DEBUG] Received publishDiagnostics for ${params.uri} with ${params.diagnostics?.length || 0} diagnostics${params.version !== undefined ? ` (version: ${params.version})` : ""}
`);
        serverState.diagnostics.set(params.uri, params.diagnostics || []);
        serverState.lastDiagnosticUpdate.set(params.uri, Date.now());
        if (params.version !== undefined) {
          serverState.diagnosticVersions.set(params.uri, params.version);
        }
      }
    }
  }
  dispose() {
    for (const [id, request] of this.pendingRequests) {
      request.reject(new Error("LSP client disposed"));
    }
    this.pendingRequests.clear();
  }
}
export {
  LSPProtocol
};

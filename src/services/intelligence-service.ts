import { readFileSync } from 'node:fs';
import type { ServerState } from '../lsp-types.js';
import type { LSPProtocol } from '../lsp/protocol.js';
import type {
  CompletionItem,
  Hover,
  InlayHint,
  InlayHintParams,
  Position,
  SemanticTokens,
  SemanticTokensParams,
  SignatureHelp,
} from '../types.js';

/**
 * Service for intelligence-related LSP operations
 * Handles hover, completions, signature help, inlay hints, and semantic tokens
 */
export class IntelligenceService {
  constructor(
    private getServer: (filePath: string) => Promise<ServerState>,
    private protocol: LSPProtocol
  ) {}

  /**
   * Get hover information at position
   */
  async getHover(filePath: string, position: Position): Promise<Hover | null> {
    console.error('[DEBUG getHover] Starting hover request for', filePath);
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }
    console.error('[DEBUG getHover] Got server state');

    await this.ensureFileOpen(serverState, filePath);
    console.error('[DEBUG getHover] File opened');

    // Give TypeScript Language Server time to process the file
    await new Promise((resolve) => setTimeout(resolve, 500));
    console.error('[DEBUG getHover] Waited for TS to process');

    console.error('[DEBUG getHover] Calling sendRequest with 30s timeout');

    try {
      const response = await this.protocol.sendRequest(
        serverState.process,
        'textDocument/hover',
        {
          textDocument: { uri: `file://${filePath}` },
          position,
        },
        30000 // 30 second timeout - give it plenty of time
      );
      console.error('[DEBUG getHover] Got response:', response);
      return response && typeof response === 'object' && 'contents' in response
        ? (response as Hover)
        : null;
    } catch (error: unknown) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error('[DEBUG getHover] Error:', errorMessage);
      if (error instanceof Error && error.message?.includes('timeout')) {
        // Return a fallback hover response
        return {
          contents: {
            kind: 'markdown',
            value: `**Hover information unavailable**\n\nThe TypeScript Language Server did not respond to the hover request at line ${position.line + 1}, character ${position.character + 1}. This feature may not be fully supported in the current server configuration.`,
          },
        };
      }
      throw error;
    }
  }

  /**
   * Get completions at position
   */
  async getCompletions(
    filePath: string,
    position: Position,
    triggerCharacter?: string
  ): Promise<CompletionItem[]> {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    await this.ensureFileOpen(serverState, filePath);

    // Give TypeScript Language Server time to process the file
    await new Promise((resolve) => setTimeout(resolve, 500));

    const completionParams = {
      textDocument: { uri: `file://${filePath}` },
      position,
      context: triggerCharacter
        ? {
            triggerKind: 2, // TriggerCharacter
            triggerCharacter,
          }
        : {
            triggerKind: 1, // Invoked
          },
    };

    try {
      const response = await this.protocol.sendRequest(
        serverState.process,
        'textDocument/completion',
        completionParams,
        5000 // 5 second timeout
      );

      if (!response || typeof response !== 'object') return [];
      const result = response as { items?: CompletionItem[] };
      return Array.isArray(result.items) ? result.items : result.items || [];
    } catch (error: unknown) {
      if (error instanceof Error && error.message?.includes('timeout')) {
        // Return empty completion list with explanation
        return [
          {
            label: 'Completions unavailable',
            detail: 'TypeScript Language Server timeout',
            documentation:
              'The TypeScript Language Server did not respond to the completion request. This feature may not be fully supported in the current server configuration.',
            insertText: '',
            kind: 1, // Text
          },
        ];
      }
      throw error;
    }
  }

  /**
   * Get signature help at position
   */
  async getSignatureHelp(
    filePath: string,
    position: Position,
    triggerCharacter?: string
  ): Promise<SignatureHelp | null> {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    await this.ensureFileOpen(serverState, filePath);

    const signatureHelpParams = {
      textDocument: { uri: `file://${filePath}` },
      position,
      context: triggerCharacter
        ? {
            triggerKind: 2, // TriggerCharacter
            triggerCharacter,
            isRetrigger: false,
          }
        : {
            triggerKind: 1, // Invoked
            isRetrigger: false,
          },
    };

    const response = await this.protocol.sendRequest(
      serverState.process,
      'textDocument/signatureHelp',
      signatureHelpParams
    );

    return response && typeof response === 'object' && 'signatures' in response
      ? (response as SignatureHelp)
      : null;
  }

  /**
   * Get inlay hints for range
   */
  async getInlayHints(
    filePath: string,
    range: { start: Position; end: Position }
  ): Promise<InlayHint[]> {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    await this.ensureFileOpen(serverState, filePath);

    const inlayHintParams: InlayHintParams = {
      textDocument: { uri: `file://${filePath}` },
      range,
    };

    const response = await this.protocol.sendRequest(
      serverState.process,
      'textDocument/inlayHint',
      inlayHintParams
    );

    return Array.isArray(response) ? response : [];
  }

  /**
   * Get semantic tokens for file
   */
  async getSemanticTokens(filePath: string): Promise<SemanticTokens | null> {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error('No LSP server available for this file type');
    }

    await this.ensureFileOpen(serverState, filePath);

    const semanticTokensParams: SemanticTokensParams = {
      textDocument: { uri: `file://${filePath}` },
    };

    const response = await this.protocol.sendRequest(
      serverState.process,
      'textDocument/semanticTokens/full',
      semanticTokensParams
    );

    return response && typeof response === 'object' && 'data' in response
      ? (response as SemanticTokens)
      : null;
  }

  /**
   * Ensure file is open in LSP server
   */
  private async ensureFileOpen(serverState: ServerState, filePath: string): Promise<void> {
    if (serverState.openFiles.has(filePath)) {
      return;
    }

    const fileContent = readFileSync(filePath, 'utf-8');

    this.protocol.sendNotification(serverState.process, 'textDocument/didOpen', {
      textDocument: {
        uri: `file://${filePath}`,
        languageId: this.getLanguageId(filePath),
        version: 1,
        text: fileContent,
      },
    });

    serverState.openFiles.add(filePath);
  }

  private getLanguageId(filePath: string): string {
    const ext = filePath.split('.').pop()?.toLowerCase();
    const languageMap: Record<string, string> = {
      ts: 'typescript',
      tsx: 'typescriptreact',
      js: 'javascript',
      jsx: 'javascriptreact',
      py: 'python',
      go: 'go',
      rs: 'rust',
      java: 'java',
      cpp: 'cpp',
      c: 'c',
      h: 'c',
      hpp: 'cpp',
    };
    return languageMap[ext || ''] || 'plaintext';
  }
}

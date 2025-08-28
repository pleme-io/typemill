import type { IntelligenceMethodsContext } from '../lsp-types.js';
// LLM Agent Intelligence LSP Methods
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

export async function getHover(
  context: IntelligenceMethodsContext,
  filePath: string,
  position: Position
): Promise<Hover | null> {
  console.error('[DEBUG getHover] Starting hover request for', filePath);
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }
  console.error('[DEBUG getHover] Got server state');

  await context.ensureFileOpen(serverState, filePath);
  console.error('[DEBUG getHover] File opened');

  // Give TypeScript Language Server time to process the file
  await new Promise((resolve) => setTimeout(resolve, 500));
  console.error('[DEBUG getHover] Waited for TS to process');

  console.error('[DEBUG getHover] Calling sendRequest with 30s timeout');

  try {
    const response = await context.sendRequest(
      serverState,
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

export async function getCompletions(
  context: IntelligenceMethodsContext,
  filePath: string,
  position: Position,
  triggerCharacter?: string
): Promise<CompletionItem[]> {
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  await context.ensureFileOpen(serverState, filePath);

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
    const response = await context.sendRequest(
      serverState,
      'textDocument/completion',
      completionParams,
      5000 // 5 second timeout
    );

    if (!response || typeof response !== 'object') return [];
    const result = response as any;
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

export async function getInlayHints(
  context: IntelligenceMethodsContext,
  filePath: string,
  range: { start: Position; end: Position }
): Promise<InlayHint[]> {
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  await context.ensureFileOpen(serverState, filePath);

  const inlayHintParams: InlayHintParams = {
    textDocument: { uri: `file://${filePath}` },
    range,
  };

  const response = await context.sendRequest(
    serverState,
    'textDocument/inlayHint',
    inlayHintParams
  );

  return Array.isArray(response) ? response : [];
}

export async function getSemanticTokens(
  context: IntelligenceMethodsContext,
  filePath: string
): Promise<SemanticTokens | null> {
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  await context.ensureFileOpen(serverState, filePath);

  const semanticTokensParams: SemanticTokensParams = {
    textDocument: { uri: `file://${filePath}` },
  };

  const response = await context.sendRequest(
    serverState,
    'textDocument/semanticTokens/full',
    semanticTokensParams
  );

  return response && typeof response === 'object' && 'data' in response
    ? (response as SemanticTokens)
    : null;
}

export async function getSignatureHelp(
  context: IntelligenceMethodsContext,
  filePath: string,
  position: Position,
  triggerCharacter?: string
): Promise<SignatureHelp | null> {
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  await context.ensureFileOpen(serverState, filePath);

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

  const response = await context.sendRequest(
    serverState,
    'textDocument/signatureHelp',
    signatureHelpParams
  );

  return response && typeof response === 'object' && 'signatures' in response
    ? (response as SignatureHelp)
    : null;
}

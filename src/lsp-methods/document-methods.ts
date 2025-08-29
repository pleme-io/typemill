import type { LSPClient } from '../lsp-client.js';
import { pathToUri } from '../path-utils.js';
import type {
  DocumentLink,
  DocumentSymbol,
  FoldingRange,
  SymbolInformation,
  TextEdit,
} from '../types.js';
import { SymbolKind } from '../types.js';

// Type definitions for the methods in this module
export interface DocumentMethodsContext {
  getServer: LSPClient['getServer'];
  ensureFileOpen: LSPClient['ensureFileOpen'];
  sendRequest: LSPClient['sendRequest'];
}

export async function getDocumentSymbols(
  context: DocumentMethodsContext,
  filePath: string
): Promise<DocumentSymbol[] | SymbolInformation[]> {
  const serverState = await context.getServer(filePath);

  // Wait for the server to be fully initialized
  await serverState.initializationPromise;

  // Ensure the file is opened and synced with the LSP server
  await context.ensureFileOpen(serverState, filePath);

  process.stderr.write(`[DEBUG] Requesting documentSymbol for: ${filePath}\n`);

  const result = await context.sendRequest(serverState.process, 'textDocument/documentSymbol', {
    textDocument: { uri: pathToUri(filePath) },
  });

  process.stderr.write(
    `[DEBUG] documentSymbol result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : 'N/A'}\n`
  );

  if (result && Array.isArray(result) && result.length > 0) {
    process.stderr.write(`[DEBUG] First symbol: ${JSON.stringify(result[0], null, 2)}\n`);
  } else if (result === null || result === undefined) {
    process.stderr.write('[DEBUG] documentSymbol returned null/undefined\n');
  } else {
    process.stderr.write(
      `[DEBUG] documentSymbol returned unexpected result: ${JSON.stringify(result)}\n`
    );
  }

  if (Array.isArray(result)) {
    return result as DocumentSymbol[] | SymbolInformation[];
  }

  return [];
}

export function flattenDocumentSymbols(symbols: DocumentSymbol[]): DocumentSymbol[] {
  const flattened: DocumentSymbol[] = [];

  for (const symbol of symbols) {
    flattened.push(symbol);
    if (symbol.children) {
      flattened.push(...flattenDocumentSymbols(symbol.children));
    }
  }

  return flattened;
}

export function isDocumentSymbolArray(
  symbols: DocumentSymbol[] | SymbolInformation[]
): symbols is DocumentSymbol[] {
  if (symbols.length === 0) return true;
  const firstSymbol = symbols[0];
  if (!firstSymbol) return true;
  // DocumentSymbol has 'range' and 'selectionRange', SymbolInformation has 'location'
  return 'range' in firstSymbol && 'selectionRange' in firstSymbol;
}

export function symbolKindToString(kind: SymbolKind): string {
  const kindMap: Record<SymbolKind, string> = {
    [SymbolKind.File]: 'file',
    [SymbolKind.Module]: 'module',
    [SymbolKind.Namespace]: 'namespace',
    [SymbolKind.Package]: 'package',
    [SymbolKind.Class]: 'class',
    [SymbolKind.Method]: 'method',
    [SymbolKind.Property]: 'property',
    [SymbolKind.Field]: 'field',
    [SymbolKind.Constructor]: 'constructor',
    [SymbolKind.Enum]: 'enum',
    [SymbolKind.Interface]: 'interface',
    [SymbolKind.Function]: 'function',
    [SymbolKind.Variable]: 'variable',
    [SymbolKind.Constant]: 'constant',
    [SymbolKind.String]: 'string',
    [SymbolKind.Number]: 'number',
    [SymbolKind.Boolean]: 'boolean',
    [SymbolKind.Array]: 'array',
    [SymbolKind.Object]: 'object',
    [SymbolKind.Key]: 'key',
    [SymbolKind.Null]: 'null',
    [SymbolKind.EnumMember]: 'enum_member',
    [SymbolKind.Struct]: 'struct',
    [SymbolKind.Event]: 'event',
    [SymbolKind.Operator]: 'operator',
    [SymbolKind.TypeParameter]: 'type_parameter',
  };
  return kindMap[kind] || 'unknown';
}

export function getValidSymbolKinds(): string[] {
  return [
    'file',
    'module',
    'namespace',
    'package',
    'class',
    'method',
    'property',
    'field',
    'constructor',
    'enum',
    'interface',
    'function',
    'variable',
    'constant',
    'string',
    'number',
    'boolean',
    'array',
    'object',
    'key',
    'null',
    'enum_member',
    'struct',
    'event',
    'operator',
    'type_parameter',
  ];
}

export async function formatDocument(
  context: DocumentMethodsContext,
  filePath: string,
  options?: {
    tabSize?: number;
    insertSpaces?: boolean;
    trimTrailingWhitespace?: boolean;
    insertFinalNewline?: boolean;
    trimFinalNewlines?: boolean;
  }
): Promise<TextEdit[]> {
  const serverState = await context.getServer(filePath);
  if (!serverState.initialized) {
    throw new Error('Server not initialized');
  }

  await context.ensureFileOpen(serverState, filePath);
  const fileUri = pathToUri(filePath);

  const formattingOptions = {
    tabSize: options?.tabSize || 2,
    insertSpaces: options?.insertSpaces !== false,
    ...(options?.trimTrailingWhitespace !== undefined && {
      trimTrailingWhitespace: options.trimTrailingWhitespace,
    }),
    ...(options?.insertFinalNewline !== undefined && {
      insertFinalNewline: options.insertFinalNewline,
    }),
    ...(options?.trimFinalNewlines !== undefined && {
      trimFinalNewlines: options.trimFinalNewlines,
    }),
  };

  const result = await context.sendRequest(serverState.process, 'textDocument/formatting', {
    textDocument: { uri: fileUri },
    options: formattingOptions,
  });

  return Array.isArray(result) ? result : [];
}

export async function getFoldingRanges(
  context: DocumentMethodsContext,
  filePath: string
): Promise<FoldingRange[]> {
  const serverState = await context.getServer(filePath);
  if (!serverState.initialized) {
    throw new Error('Server not initialized');
  }

  await context.ensureFileOpen(serverState, filePath);
  const fileUri = pathToUri(filePath);

  process.stderr.write(`[DEBUG getFoldingRanges] Requesting folding ranges for: ${filePath}\n`);

  const result = await context.sendRequest(serverState.process, 'textDocument/foldingRange', {
    textDocument: { uri: fileUri },
  });

  process.stderr.write(
    `[DEBUG getFoldingRanges] Result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : 'N/A'}\n`
  );

  if (Array.isArray(result)) {
    return result as FoldingRange[];
  }

  return [];
}

export async function getDocumentLinks(
  context: DocumentMethodsContext,
  filePath: string
): Promise<DocumentLink[]> {
  const serverState = await context.getServer(filePath);
  if (!serverState.initialized) {
    throw new Error('Server not initialized');
  }

  await context.ensureFileOpen(serverState, filePath);
  const fileUri = pathToUri(filePath);

  process.stderr.write(`[DEBUG getDocumentLinks] Requesting document links for: ${filePath}\n`);

  const result = await context.sendRequest(serverState.process, 'textDocument/documentLink', {
    textDocument: { uri: fileUri },
  });

  process.stderr.write(
    `[DEBUG getDocumentLinks] Result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : 'N/A'}\n`
  );

  if (Array.isArray(result)) {
    return result as DocumentLink[];
  }

  return [];
}

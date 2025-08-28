// Hierarchy and Navigation LSP Methods for LLM agents
import type {
  CallHierarchyIncomingCall,
  CallHierarchyItem,
  CallHierarchyOutgoingCall,
  Position,
  SelectionRange,
  TypeHierarchyItem,
} from '../types.js';

import type { HierarchyMethodsContext } from '../lsp-types.js';

export async function prepareCallHierarchy(
  context: HierarchyMethodsContext,
  filePath: string,
  position: Position
): Promise<CallHierarchyItem[]> {
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  await context.ensureFileOpen(serverState, filePath);

  const response = await context.sendRequest(serverState, 'textDocument/prepareCallHierarchy', {
    textDocument: { uri: `file://${filePath}` },
    position,
  });

  return Array.isArray(response) ? response : [];
}

export async function getCallHierarchyIncomingCalls(
  context: HierarchyMethodsContext,
  item: CallHierarchyItem
): Promise<CallHierarchyIncomingCall[]> {
  // Extract the file path from the item's URI to determine the correct server
  const filePath = item.uri.replace('file://', '');
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  const response = await context.sendRequest(serverState, 'callHierarchy/incomingCalls', {
    item,
  });

  return Array.isArray(response) ? response : [];
}

export async function getCallHierarchyOutgoingCalls(
  context: HierarchyMethodsContext,
  item: CallHierarchyItem
): Promise<CallHierarchyOutgoingCall[]> {
  // Extract the file path from the item's URI to determine the correct server
  const filePath = item.uri.replace('file://', '');
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  const response = await context.sendRequest(serverState, 'callHierarchy/outgoingCalls', {
    item,
  });

  return Array.isArray(response) ? response : [];
}

export async function prepareTypeHierarchy(
  context: HierarchyMethodsContext,
  filePath: string,
  position: Position
): Promise<TypeHierarchyItem[]> {
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  await context.ensureFileOpen(serverState, filePath);

  const response = await context.sendRequest(serverState, 'textDocument/prepareTypeHierarchy', {
    textDocument: { uri: `file://${filePath}` },
    position,
  });

  return Array.isArray(response) ? response : [];
}

export async function getTypeHierarchySupertypes(
  context: HierarchyMethodsContext,
  item: TypeHierarchyItem
): Promise<TypeHierarchyItem[]> {
  // Extract the file path from the item's URI to determine the correct server
  const filePath = item.uri.replace('file://', '');
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  const response = await context.sendRequest(serverState, 'typeHierarchy/supertypes', {
    item,
  });

  return Array.isArray(response) ? response : [];
}

export async function getTypeHierarchySubtypes(
  context: HierarchyMethodsContext,
  item: TypeHierarchyItem
): Promise<TypeHierarchyItem[]> {
  // Extract the file path from the item's URI to determine the correct server
  const filePath = item.uri.replace('file://', '');
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  const response = await context.sendRequest(serverState, 'typeHierarchy/subtypes', {
    item,
  });

  return Array.isArray(response) ? response : [];
}

export async function getSelectionRange(
  context: HierarchyMethodsContext,
  filePath: string,
  positions: Position[]
): Promise<SelectionRange[]> {
  const serverState = await context.getServer(filePath);
  if (!serverState) {
    throw new Error('No LSP server available for this file type');
  }

  await context.ensureFileOpen(serverState, filePath);

  try {
    const response = await context.sendRequest(
      serverState,
      'textDocument/selectionRange',
      {
        textDocument: { uri: `file://${filePath}` },
        positions,
      },
      5000
    ); // 5 second timeout

    return Array.isArray(response) ? response : [];
  } catch (error: unknown) {
    if (error instanceof Error && error.message?.includes('timeout')) {
      throw new Error('Selection range request timed out - TypeScript server may be overloaded');
    }
    throw error;
  }
}

#!/usr/bin/env node

import { resolve } from 'node:path';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { LSPClient } from './src/lsp-client.js';
import { allToolDefinitions } from './src/mcp/definitions/index.js';
import type {
  ApplyWorkspaceEditArgs,
  CreateFileArgs,
  DeleteFileArgs,
  FindDefinitionArgs,
  FindReferencesArgs,
  FormatDocumentArgs,
  GetCallHierarchyIncomingCallsArgs,
  GetCallHierarchyOutgoingCallsArgs,
  GetCodeActionsArgs,
  GetCompletionsArgs,
  GetDiagnosticsArgs,
  GetDocumentLinksArgs,
  GetDocumentSymbolsArgs,
  GetFoldingRangesArgs,
  GetHoverArgs,
  GetInlayHintsArgs,
  GetSelectionRangeArgs,
  GetSemanticTokensArgs,
  GetSignatureHelpArgs,
  GetTypeHierarchySubtypesArgs,
  GetTypeHierarchySupertypesArgs,
  PrepareCallHierarchyArgs,
  PrepareTypeHierarchyArgs,
  RenameFileArgs,
  RenameSymbolArgs,
  RenameSymbolStrictArgs,
  RestartServerArgs,
  SearchWorkspaceSymbolsArgs,
} from './src/mcp/handler-types.js';
import {
  handleApplyWorkspaceEdit,
  handleCreateFile,
  handleDeleteFile,
  handleFindDefinition,
  handleFindReferences,
  handleFormatDocument,
  handleGetCallHierarchyIncomingCalls,
  handleGetCallHierarchyOutgoingCalls,
  handleGetCodeActions,
  handleGetCompletions,
  handleGetDiagnostics,
  handleGetDocumentLinks,
  handleGetDocumentSymbols,
  handleGetFoldingRanges,
  handleGetHover,
  handleGetInlayHints,
  handleGetSelectionRange,
  handleGetSemanticTokens,
  handleGetSignatureHelp,
  handleGetTypeHierarchySubtypes,
  handleGetTypeHierarchySupertypes,
  handlePrepareCallHierarchy,
  handlePrepareTypeHierarchy,
  handleRenameFile,
  handleRenameSymbol,
  handleRenameSymbolStrict,
  handleRestartServer,
  handleSearchWorkspaceSymbols,
} from './src/mcp/handlers/index.js';
import { createMCPError } from './src/mcp/utils.js';

// Handle subcommands
const args = process.argv.slice(2);
if (args.length > 0) {
  const subcommand = args[0];

  if (subcommand === 'setup') {
    const { main } = await import('./src/setup.js');
    await main();
    process.exit(0);
  } else {
    console.error(`Unknown subcommand: ${subcommand}`);
    console.error('Available subcommands:');
    console.error('  setup    Configure cclsp for your project');
    console.error('');
    console.error('Run without arguments to start the MCP server.');
    process.exit(1);
  }
}

const lspClient = new LSPClient();

const server = new Server(
  {
    name: 'cclsp',
    version: '0.1.0',
  },
  {
    capabilities: {
      tools: {},
    },
  }
);

server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: allToolDefinitions,
  };
});

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  console.error('[DEBUG] Tool request received:', name, args);

  try {
    switch (name) {
      case 'find_definition':
        return await handleFindDefinition(lspClient, args as unknown as FindDefinitionArgs);
      case 'find_references':
        return await handleFindReferences(lspClient, args as unknown as FindReferencesArgs);
      case 'rename_symbol':
        return await handleRenameSymbol(lspClient, args as unknown as RenameSymbolArgs);
      case 'rename_symbol_strict':
        return await handleRenameSymbolStrict(lspClient, args as unknown as RenameSymbolStrictArgs);
      case 'get_code_actions':
        return await handleGetCodeActions(lspClient, args as unknown as GetCodeActionsArgs);
      case 'format_document':
        return await handleFormatDocument(lspClient, args as unknown as FormatDocumentArgs);
      case 'search_workspace_symbols':
        return await handleSearchWorkspaceSymbols(
          lspClient,
          args as unknown as SearchWorkspaceSymbolsArgs
        );
      case 'get_document_symbols':
        return await handleGetDocumentSymbols(lspClient, args as unknown as GetDocumentSymbolsArgs);
      case 'get_folding_ranges':
        return await handleGetFoldingRanges(lspClient, args as unknown as GetFoldingRangesArgs);
      case 'get_document_links':
        return await handleGetDocumentLinks(lspClient, args as unknown as GetDocumentLinksArgs);
      case 'get_diagnostics':
        return await handleGetDiagnostics(lspClient, args as unknown as GetDiagnosticsArgs);
      case 'restart_server':
        return await handleRestartServer(lspClient, args as unknown as RestartServerArgs);
      case 'rename_file':
        return await handleRenameFile(lspClient, args as unknown as RenameFileArgs);
      // Intelligence tools
      case 'get_hover':
        return await handleGetHover(lspClient, args as unknown as GetHoverArgs);
      case 'get_completions':
        return await handleGetCompletions(lspClient, args as unknown as GetCompletionsArgs);
      case 'get_inlay_hints':
        return await handleGetInlayHints(lspClient, args as unknown as GetInlayHintsArgs);
      case 'get_semantic_tokens':
        return await handleGetSemanticTokens(lspClient, args as unknown as GetSemanticTokensArgs);
      case 'get_signature_help':
        return await handleGetSignatureHelp(lspClient, args as unknown as GetSignatureHelpArgs);
      // Hierarchy tools
      case 'prepare_call_hierarchy':
        return await handlePrepareCallHierarchy(
          lspClient,
          args as unknown as PrepareCallHierarchyArgs
        );
      case 'get_call_hierarchy_incoming_calls':
        return await handleGetCallHierarchyIncomingCalls(
          lspClient,
          args as unknown as GetCallHierarchyIncomingCallsArgs
        );
      case 'get_call_hierarchy_outgoing_calls':
        return await handleGetCallHierarchyOutgoingCalls(
          lspClient,
          args as unknown as GetCallHierarchyOutgoingCallsArgs
        );
      case 'prepare_type_hierarchy':
        return await handlePrepareTypeHierarchy(
          lspClient,
          args as unknown as PrepareTypeHierarchyArgs
        );
      case 'get_type_hierarchy_supertypes':
        return await handleGetTypeHierarchySupertypes(
          lspClient,
          args as unknown as GetTypeHierarchySupertypesArgs
        );
      case 'get_type_hierarchy_subtypes':
        return await handleGetTypeHierarchySubtypes(
          lspClient,
          args as unknown as GetTypeHierarchySubtypesArgs
        );
      case 'get_selection_range':
        return await handleGetSelectionRange(lspClient, args as unknown as GetSelectionRangeArgs);
      case 'apply_workspace_edit':
        return await handleApplyWorkspaceEdit(lspClient, args as unknown as ApplyWorkspaceEditArgs);
      case 'create_file':
        return await handleCreateFile(lspClient, args as unknown as CreateFileArgs);
      case 'delete_file':
        return await handleDeleteFile(lspClient, args as unknown as DeleteFileArgs);
      default:
        throw new Error(`Unknown tool: ${name}`);
    }
  } catch (error) {
    return createMCPError(error);
  }
});

process.on('SIGINT', () => {
  lspClient.dispose();
  process.exit(0);
});

process.on('SIGTERM', () => {
  lspClient.dispose();
  process.exit(0);
});

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  process.stderr.write('CCLSP Server running on stdio\n');

  // Preload LSP servers for file types found in the project
  try {
    process.stderr.write('Starting LSP server preload...\n');
    await lspClient.preloadServers();
    process.stderr.write('LSP servers preloaded successfully\n');
  } catch (error) {
    process.stderr.write(`Failed to preload LSP servers: ${error}\n`);
  }
}

main().catch((error) => {
  process.stderr.write(`Server error: ${error}\n`);
  lspClient.dispose();
  process.exit(1);
});

#!/usr/bin/env node

import { resolve } from 'node:path';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { LSPClient as OldLSPClient } from './src/lsp-client.js';
import { LSPClient as NewLSPClient } from './src/lsp/client.js';
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
import { DiagnosticService } from './src/services/diagnostic-service.js';
import { FileService } from './src/services/file-service.js';
import { HierarchyService } from './src/services/hierarchy-service.js';
import { IntelligenceService } from './src/services/intelligence-service.js';
import { SymbolService } from './src/services/symbol-service.js';

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

// Create new LSP client and initialize services
const newLspClient = new NewLSPClient();
const getServer = (filePath: string) => newLspClient.getServer(filePath);
const protocol = newLspClient.protocol;

// Initialize services with LSP components
const symbolService = new SymbolService(getServer, protocol);
const fileService = new FileService(getServer, protocol);
const diagnosticService = new DiagnosticService(getServer, protocol);
const intelligenceService = new IntelligenceService(getServer, protocol);
const hierarchyService = new HierarchyService(getServer, protocol);

// Temporary: Keep old LSP client for handlers that haven't been refactored yet
const oldLspClient = new OldLSPClient();

const server = new Server(
  {
    name: 'cclsp',
    version: '0.5.12',
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
        return await handleFindDefinition(symbolService, args as unknown as FindDefinitionArgs);
      case 'find_references':
        return await handleFindReferences(symbolService, args as unknown as FindReferencesArgs);
      case 'rename_symbol':
        return await handleRenameSymbol(symbolService, args as unknown as RenameSymbolArgs);
      case 'rename_symbol_strict':
        return await handleRenameSymbolStrict(
          symbolService,
          args as unknown as RenameSymbolStrictArgs
        );
      case 'get_code_actions':
        return await handleGetCodeActions(fileService, args as unknown as GetCodeActionsArgs);
      case 'format_document':
        return await handleFormatDocument(fileService, args as unknown as FormatDocumentArgs);
      case 'search_workspace_symbols':
        return await handleSearchWorkspaceSymbols(
          oldLspClient,
          args as unknown as SearchWorkspaceSymbolsArgs
        );
      case 'get_document_symbols':
        return await handleGetDocumentSymbols(
          symbolService,
          args as unknown as GetDocumentSymbolsArgs
        );
      case 'get_folding_ranges':
        return await handleGetFoldingRanges(oldLspClient, args as unknown as GetFoldingRangesArgs);
      case 'get_document_links':
        return await handleGetDocumentLinks(oldLspClient, args as unknown as GetDocumentLinksArgs);
      case 'get_diagnostics':
        return await handleGetDiagnostics(diagnosticService, args as unknown as GetDiagnosticsArgs);
      case 'restart_server':
        return await handleRestartServer(oldLspClient, args as unknown as RestartServerArgs);
      case 'rename_file':
        return await handleRenameFile(oldLspClient, args as unknown as RenameFileArgs);
      // Intelligence tools
      case 'get_hover':
        return await handleGetHover(intelligenceService, args as unknown as GetHoverArgs);
      case 'get_completions':
        return await handleGetCompletions(
          intelligenceService,
          args as unknown as GetCompletionsArgs
        );
      case 'get_inlay_hints':
        return await handleGetInlayHints(intelligenceService, args as unknown as GetInlayHintsArgs);
      case 'get_semantic_tokens':
        return await handleGetSemanticTokens(
          intelligenceService,
          args as unknown as GetSemanticTokensArgs
        );
      case 'get_signature_help':
        return await handleGetSignatureHelp(
          intelligenceService,
          args as unknown as GetSignatureHelpArgs
        );
      // Hierarchy tools
      case 'prepare_call_hierarchy':
        return await handlePrepareCallHierarchy(
          hierarchyService,
          args as unknown as PrepareCallHierarchyArgs
        );
      case 'get_call_hierarchy_incoming_calls':
        return await handleGetCallHierarchyIncomingCalls(
          hierarchyService,
          args as unknown as GetCallHierarchyIncomingCallsArgs
        );
      case 'get_call_hierarchy_outgoing_calls':
        return await handleGetCallHierarchyOutgoingCalls(
          hierarchyService,
          args as unknown as GetCallHierarchyOutgoingCallsArgs
        );
      case 'prepare_type_hierarchy':
        return await handlePrepareTypeHierarchy(
          hierarchyService,
          args as unknown as PrepareTypeHierarchyArgs
        );
      case 'get_type_hierarchy_supertypes':
        return await handleGetTypeHierarchySupertypes(
          hierarchyService,
          args as unknown as GetTypeHierarchySupertypesArgs
        );
      case 'get_type_hierarchy_subtypes':
        return await handleGetTypeHierarchySubtypes(
          hierarchyService,
          args as unknown as GetTypeHierarchySubtypesArgs
        );
      case 'get_selection_range':
        return await handleGetSelectionRange(
          hierarchyService,
          args as unknown as GetSelectionRangeArgs
        );
      case 'apply_workspace_edit':
        return await handleApplyWorkspaceEdit(
          fileService,
          args as unknown as ApplyWorkspaceEditArgs
        );
      case 'create_file':
        return await handleCreateFile(oldLspClient, args as unknown as CreateFileArgs);
      case 'delete_file':
        return await handleDeleteFile(oldLspClient, args as unknown as DeleteFileArgs);
      default:
        throw new Error(`Unknown tool: ${name}`);
    }
  } catch (error) {
    return createMCPError(error);
  }
});

process.on('SIGINT', () => {
  newLspClient.dispose();
  oldLspClient.dispose();
  process.exit(0);
});

process.on('SIGTERM', () => {
  newLspClient.dispose();
  oldLspClient.dispose();
  process.exit(0);
});

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  process.stderr.write('CCLSP Server running on stdio\n');

  // Preload LSP servers for file types found in the project
  try {
    process.stderr.write('Starting LSP server preload...\n');
    // Use old client for preloading since it has the full preload functionality
    await oldLspClient.preloadServers();
    process.stderr.write('LSP servers preloaded successfully\n');
  } catch (error) {
    process.stderr.write(`Failed to preload LSP servers: ${error}\n`);
  }
}

main().catch((error) => {
  process.stderr.write(`Server error: ${error}\n`);
  newLspClient.dispose();
  oldLspClient.dispose();
  process.exit(1);
});

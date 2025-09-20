#!/usr/bin/env node

import { resolve } from 'node:path';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { LSPClient as NewLSPClient } from './src/lsp/client.js';
import * as Validation from './src/mcp/comprehensive-validation.js';
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
  HealthCheckArgs,
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
  handleHealthCheck,
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
import {
  StructuredLogger,
  createRequestContext,
  getLogger,
} from './src/utils/structured-logger.js';
import { getPackageVersion } from './src/utils/version.js';

// Initialize module logger
const logger = getLogger('MCP-Server');

// Handle subcommands and help flags
const args = process.argv.slice(2);
if (args.length > 0) {
  const subcommand = args[0];

  if (subcommand === 'init') {
    const { initCommand } = await import('./src/cli/commands/init.js');
    await initCommand();
    process.exit(0);
  } else if (subcommand === 'status') {
    const { statusCommand } = await import('./src/cli/commands/status.js');
    await statusCommand();
    process.exit(0);
  } else if (subcommand === 'fix') {
    const { fixCommand } = await import('./src/cli/commands/fix.js');
    const options = {
      auto: args.includes('--auto'),
      manual: args.includes('--manual'),
    };
    await fixCommand(options);
    process.exit(0);
  } else if (subcommand === '--help' || subcommand === '-h' || subcommand === 'help') {
    console.log('codebuddy - MCP server for accessing LSP functionality');
    console.log('');
    console.log('Usage: codebuddy [command] [options]');
    console.log('');
    console.log('Commands:');
    console.log('  init          Smart setup with auto-detection');
    console.log("  status        Show what's working right now");
    console.log('  fix           Actually fix problems (auto-install when possible)');
    console.log('  help          Show this help message');
    console.log('');
    console.log('Fix options:');
    console.log('  --auto        Auto-install without prompting');
    console.log('  --manual      Show manual installation commands');
    console.log('');
    console.log('Configuration & Logs:');
    console.log('  Config file: .codebuddy/config.json');
    console.log('  Log file: .codebuddy/logs/debug.log (use tail -f to follow)');
    console.log('');
    console.log('Run without arguments to start the MCP server.');
    process.exit(0);
  } else {
    console.error(`Unknown command: ${subcommand}`);
    console.error('Available commands:');
    console.error('  init     Smart setup with auto-detection');
    console.error("  status   Show what's working right now");
    console.error('  fix      Actually fix problems');
    console.error('  help     Show help message');
    console.error('');
    console.error('Run without arguments to start the MCP server.');
    process.exit(1);
  }
}

// Create LSP clients and services with proper error handling
let newLspClient: NewLSPClient;
let symbolService: SymbolService;
let fileService: FileService;
let diagnosticService: DiagnosticService;
let intelligenceService: IntelligenceService;
let hierarchyService: HierarchyService;

try {
  // Create new LSP client
  newLspClient = new NewLSPClient();

  // Create ServiceContext for all services
  const { ServiceContextUtils } = await import('./src/services/service-context.js');
  const serviceContext = ServiceContextUtils.createServiceContext(
    newLspClient.getServer.bind(newLspClient),
    newLspClient.protocol
  );

  // Initialize services with ServiceContext
  symbolService = new SymbolService(serviceContext);
  fileService = new FileService(serviceContext);
  diagnosticService = new DiagnosticService(serviceContext);
  intelligenceService = new IntelligenceService(serviceContext);
  hierarchyService = new HierarchyService(serviceContext);
} catch (error) {
  logger.error('Failed to initialize LSP clients', error);
  process.exit(1);
}

const server = new Server(
  {
    name: 'codebuddy',
    version: getPackageVersion(),
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
  const requestId = `mcp_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`;

  return await StructuredLogger.withContextAsync(
    createRequestContext(requestId, 'MCP_CALL', `tool:${name}`),
    async () => {
      logger.info('MCP tool request', { tool: name, request_id: requestId });

      try {
        switch (name) {
          case 'find_definition':
            if (!Validation.validateFindDefinitionArgs(args)) {
              throw Validation.createValidationError(
                'find_definition',
                'object with file_path and symbol_name strings'
              );
            }
            return await handleFindDefinition(symbolService, args);
          case 'find_references':
            if (!Validation.validateFindReferencesArgs(args)) {
              throw Validation.createValidationError(
                'find_references',
                'object with file_path and symbol_name strings'
              );
            }
            return await handleFindReferences(symbolService, args);
          case 'rename_symbol':
            if (!Validation.validateRenameSymbolArgs(args)) {
              throw Validation.createValidationError(
                'rename_symbol',
                'object with file_path, symbol_name, and new_name'
              );
            }
            return await handleRenameSymbol(symbolService, args);
          case 'rename_symbol_strict':
            if (!Validation.validateRenameSymbolStrictArgs(args)) {
              throw Validation.createValidationError(
                'rename_symbol_strict',
                'object with file_path, line, character, and new_name'
              );
            }
            return await handleRenameSymbolStrict(symbolService, args);
          case 'get_code_actions':
            if (!Validation.validateGetCodeActionsArgs(args)) {
              throw Validation.createValidationError(
                'get_code_actions',
                'object with file_path string'
              );
            }
            return await handleGetCodeActions(fileService, args);
          case 'format_document':
            if (!Validation.validateFormatDocumentArgs(args)) {
              throw Validation.createValidationError(
                'format_document',
                'object with file_path string and optional formatting options'
              );
            }
            return await handleFormatDocument(fileService, args);
          case 'search_workspace_symbols':
            if (!Validation.validateSearchWorkspaceSymbolsArgs(args)) {
              throw Validation.createValidationError(
                'search_workspace_symbols',
                'object with query string'
              );
            }
            return await handleSearchWorkspaceSymbols(symbolService, args, newLspClient);
          case 'get_document_symbols':
            if (!Validation.validateGetDocumentSymbolsArgs(args)) {
              throw Validation.createValidationError(
                'get_document_symbols',
                'object with file_path string'
              );
            }
            return await handleGetDocumentSymbols(symbolService, args);
          case 'get_folding_ranges':
            if (!Validation.validateGetFoldingRangesArgs(args)) {
              throw Validation.createValidationError(
                'get_folding_ranges',
                'object with file_path string'
              );
            }
            return await handleGetFoldingRanges(fileService, args, newLspClient);
          case 'get_document_links':
            if (!Validation.validateGetDocumentLinksArgs(args)) {
              throw Validation.createValidationError(
                'get_document_links',
                'object with file_path string'
              );
            }
            return await handleGetDocumentLinks(fileService, args, newLspClient);
          case 'get_diagnostics':
            if (!Validation.validateGetDiagnosticsArgs(args)) {
              throw Validation.createValidationError(
                'get_diagnostics',
                'object with file_path string'
              );
            }
            return await handleGetDiagnostics(diagnosticService, args);
          case 'restart_server':
            if (!Validation.validateRestartServerArgs(args)) {
              throw Validation.createValidationError(
                'restart_server',
                'object with optional extensions array'
              );
            }
            return await handleRestartServer(newLspClient, args);
          case 'rename_file':
            if (!Validation.validateRenameFileArgs(args)) {
              throw Validation.createValidationError(
                'rename_file',
                'object with old_path and new_path strings'
              );
            }
            return await handleRenameFile(args);
          // Intelligence tools
          case 'get_hover':
            if (!Validation.validateGetHoverArgs(args)) {
              throw Validation.createValidationError(
                'get_hover',
                'object with file_path, line, and character'
              );
            }
            return await handleGetHover(intelligenceService, args);
          case 'get_completions':
            if (!Validation.validateGetCompletionsArgs(args)) {
              throw Validation.createValidationError(
                'get_completions',
                'object with file_path, line, and character'
              );
            }
            return await handleGetCompletions(intelligenceService, args);
          case 'get_inlay_hints':
            if (!Validation.validateGetInlayHintsArgs(args)) {
              throw Validation.createValidationError(
                'get_inlay_hints',
                'object with file_path and range coordinates'
              );
            }
            return await handleGetInlayHints(intelligenceService, args);
          case 'get_semantic_tokens':
            if (!Validation.validateGetSemanticTokensArgs(args)) {
              throw Validation.createValidationError(
                'get_semantic_tokens',
                'object with file_path string'
              );
            }
            return await handleGetSemanticTokens(intelligenceService, args);
          case 'get_signature_help':
            if (!Validation.validateGetSignatureHelpArgs(args)) {
              throw Validation.createValidationError(
                'get_signature_help',
                'object with file_path, line, and character'
              );
            }
            return await handleGetSignatureHelp(intelligenceService, args);
          // Hierarchy tools
          case 'prepare_call_hierarchy':
            if (!Validation.validatePrepareCallHierarchyArgs(args)) {
              throw Validation.createValidationError(
                'prepare_call_hierarchy',
                'object with file_path, line, and character'
              );
            }
            return await handlePrepareCallHierarchy(hierarchyService, args);
          case 'get_call_hierarchy_incoming_calls':
            if (!Validation.validateGetCallHierarchyIncomingCallsArgs(args)) {
              throw Validation.createValidationError(
                'get_call_hierarchy_incoming_calls',
                'object with either "item" (CallHierarchyItem) or "file_path", "line", and "character"'
              );
            }
            return await handleGetCallHierarchyIncomingCalls(hierarchyService, args);
          case 'get_call_hierarchy_outgoing_calls':
            if (!Validation.validateGetCallHierarchyOutgoingCallsArgs(args)) {
              throw Validation.createValidationError(
                'get_call_hierarchy_outgoing_calls',
                'object with either "item" (CallHierarchyItem) or "file_path", "line", and "character"'
              );
            }
            return await handleGetCallHierarchyOutgoingCalls(hierarchyService, args);
          case 'prepare_type_hierarchy':
            if (!Validation.validatePrepareTypeHierarchyArgs(args)) {
              throw Validation.createValidationError(
                'prepare_type_hierarchy',
                'object with file_path, line, and character'
              );
            }
            return await handlePrepareTypeHierarchy(hierarchyService, args);
          case 'get_type_hierarchy_supertypes':
            if (!Validation.validateGetTypeHierarchySupertypesArgs(args)) {
              throw Validation.createValidationError(
                'get_type_hierarchy_supertypes',
                'object with TypeHierarchyItem'
              );
            }
            return await handleGetTypeHierarchySupertypes(hierarchyService, args);
          case 'get_type_hierarchy_subtypes':
            if (!Validation.validateGetTypeHierarchySubtypesArgs(args)) {
              throw Validation.createValidationError(
                'get_type_hierarchy_subtypes',
                'object with TypeHierarchyItem'
              );
            }
            return await handleGetTypeHierarchySubtypes(hierarchyService, args);
          case 'get_selection_range':
            if (!Validation.validateGetSelectionRangeArgs(args)) {
              throw Validation.createValidationError(
                'get_selection_range',
                'object with file_path and positions array'
              );
            }
            return await handleGetSelectionRange(hierarchyService, args);
          case 'apply_workspace_edit':
            if (!Validation.validateApplyWorkspaceEditArgs(args)) {
              throw Validation.createValidationError(
                'apply_workspace_edit',
                'object with either "changes" mapping file paths to text edits or "edit" containing the workspace edit'
              );
            }
            return await handleApplyWorkspaceEdit(fileService, args);
          case 'create_file':
            if (!Validation.validateCreateFileArgs(args)) {
              throw Validation.createValidationError('create_file', 'object with file_path string');
            }
            return await handleCreateFile(args);
          case 'delete_file':
            if (!Validation.validateDeleteFileArgs(args)) {
              throw Validation.createValidationError('delete_file', 'object with file_path string');
            }
            return await handleDeleteFile(args);
          case 'health_check': {
            if (!Validation.validateHealthCheckArgs(args)) {
              throw Validation.createValidationError(
                'health_check',
                'object with optional include_details boolean'
              );
            }
            const { ServiceContextUtils } = await import('./src/services/service-context.js');
            const serviceContext = ServiceContextUtils.createServiceContext(
              newLspClient.getServer.bind(newLspClient),
              newLspClient.protocol
            );
            return await handleHealthCheck(args, serviceContext);
          }
          default: {
            const { createUnknownToolMessage } = await import(
              './src/utils/enhanced-error-messages.js'
            );
            const enhancedMessage = createUnknownToolMessage(name);
            throw new Error(enhancedMessage);
          }
        }
      } catch (error) {
        logger.error('MCP tool request failed', error, { tool: name, request_id: requestId });
        return createMCPError(error);
      }
    }
  );
});

process.on('SIGINT', () => {
  logger.info('Received SIGINT, shutting down gracefully');
  newLspClient.dispose();
  process.exit(0);
});

process.on('SIGTERM', () => {
  logger.info('Received SIGTERM, shutting down gracefully');
  newLspClient.dispose();
  process.exit(0);
});

async function main() {
  await StructuredLogger.withContextAsync(
    { operation: 'server_startup', component: 'MCP-Server' },
    async () => {
      logger.info('Codebuddy MCP server starting');

      const transport = new StdioServerTransport();
      await server.connect(transport);

      logger.info('MCP server connected and ready');

      // Output message that tests expect to detect server readiness
      console.error('Codebuddy Server running on stdio');

      // Preload LSP servers for file types found in the project (unless disabled)
      if (process.env.SKIP_LSP_PRELOAD !== 'true') {
        try {
          logger.info('Starting LSP server preload');
          await newLspClient.preloadServers();
          logger.info('LSP servers preloaded successfully');
        } catch (error) {
          logger.error('Failed to preload LSP servers', error);
        }
      } else {
        logger.info('LSP server preload skipped', { reason: 'SKIP_LSP_PRELOAD=true' });
      }
    }
  );
}

main().catch((error) => {
  logger.error('Server startup error', error);
  newLspClient.dispose();
  process.exit(1);
});

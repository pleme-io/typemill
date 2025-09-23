#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import {
  StructuredLogger,
  createRequestContext,
  getLogger,
} from './src/core/diagnostics/structured-logger.js';
import { LSPClient as NewLSPClient } from './src/lsp/client.js';
import * as Validation from './src/mcp/comprehensive-validation.js';
import { allToolDefinitions } from './src/mcp/definitions/index.js';
import type {
  ApplyWorkspaceEditArgs,
  BatchExecuteArgs,
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
  handleBatchExecute,
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
import { getPackageVersion } from './src/utils/version.js';

// Initialize module logger
const logger = getLogger('MCP-Server');

// Handle subcommands and help flags
const args = process.argv.slice(2);

// Show help if no arguments provided
if (args.length === 0 || args[0] === '--help' || args[0] === '-h' || args[0] === 'help') {
  const showAdvanced = args.includes('--advanced');

  console.log('codeflow-buddy - MCP server for accessing LSP functionality');
  console.log('');
  console.log('Usage: codeflow-buddy <command> [options]');
  console.log('');
  console.log('Commands:');
  console.log('  setup         Configure LSP servers');
  console.log('  link          Link to AI assistants');
  console.log('  unlink        Remove AI from config');
  console.log('  start         Start the server');
  console.log('  stop          Stop the server');
  console.log('  status        Show everything');
  console.log('');
  console.log('Setup options:');
  console.log('  --all                Install all available servers');
  console.log('  --force              Skip confirmation prompts');
  console.log('  --servers ts,python  Install specific servers');
  console.log('');
  console.log('Quick start:');
  console.log('  npm install -g @goobits/codeflow-buddy');
  console.log('  codeflow-buddy setup         # Configure LSP servers');
  console.log('  codeflow-buddy link          # Link to AI assistants');
  console.log('  codeflow-buddy status        # Verify everything');

  if (showAdvanced) {
    console.log('');
    console.log('Advanced Commands:');
    console.log('  serve         Start WebSocket server');
    console.log('');
    console.log('Hidden Flags:');
    console.log('  link --all              Link all available assistants');
    console.log('  unlink --all            Unlink all assistants');
    console.log('  start --foreground      Run in foreground with logs');
    console.log('  status --json           Machine-readable output');
    console.log('');
    console.log('Serve options:');
    console.log('  --port N                Port number (default: 3000)');
    console.log('  --max-clients N         Maximum concurrent clients (default: 10)');
    console.log('  --enable-fuse           Enable FUSE filesystem isolation');
    console.log('  --require-auth          Require JWT authentication');
    console.log('  --jwt-secret SECRET     JWT signing secret');
  }

  console.log('');
  console.log('For more options: codeflow-buddy help --advanced');
  process.exit(0);
}

const subcommand = args[0];

if (subcommand === 'setup') {
  const { setupCommand } = await import('./src/cli/commands/setup.js');

  // Parse server list from --servers flag
  const serversIndex = args.findIndex(arg => arg === '--servers');
  const servers = serversIndex !== -1 && args[serversIndex + 1]
    ? args[serversIndex + 1]?.split(',').map(s => s.trim()).filter(Boolean)
    : undefined;

  const options = {
    all: args.includes('--all'),
    force: args.includes('--force'),
    servers,
  };
  await setupCommand(options);
  process.exit(0);
} else if (subcommand === 'status') {
  const { statusCommand } = await import('./src/cli/commands/status.js');
  await statusCommand();
  process.exit(0);
} else if (subcommand === 'start') {
  // Continue to start MCP server below
  console.log('Starting MCP server for Claude Code...');
} else if (subcommand === 'serve') {
  const { serveCommand } = await import('./src/cli/commands/serve.js');

  // Parse serve options
  const portIndex = args.indexOf('--port');
  const maxClientsIndex = args.indexOf('--max-clients');
  const enableFuseIndex = args.indexOf('--enable-fuse');
  const requireAuthIndex = args.indexOf('--require-auth');
  const jwtSecretIndex = args.indexOf('--jwt-secret');
  const allowedOriginsIndex = args.indexOf('--allowed-origins');
  const allowedCorsOriginsIndex = args.indexOf('--allowed-cors-origins');

  const options = {
    port:
      portIndex !== -1 && args[portIndex + 1] ? Number.parseInt(args[portIndex + 1]!, 10) : 3000,
    maxClients:
      maxClientsIndex !== -1 && args[maxClientsIndex + 1]
        ? Number.parseInt(args[maxClientsIndex + 1]!, 10)
        : 10,
    enableFuse: enableFuseIndex !== -1,
    requireAuth: requireAuthIndex !== -1,
    jwtSecret:
      jwtSecretIndex !== -1 && args[jwtSecretIndex + 1] ? args[jwtSecretIndex + 1] : undefined,
    allowedOrigins:
      allowedOriginsIndex !== -1 && args[allowedOriginsIndex + 1]
        ? args[allowedOriginsIndex + 1]?.split(',')
        : undefined,
    allowedCorsOrigins:
      allowedCorsOriginsIndex !== -1 && args[allowedCorsOriginsIndex + 1]
        ? args[allowedCorsOriginsIndex + 1]?.split(',')
        : undefined,
  };

  await serveCommand(options);
  // The serve command handles its own process lifecycle
  process.exit(0);
} else if (subcommand === 'link') {
  const { linkCommand } = await import('./src/cli/commands/link.js');
  const assistants = args.slice(1).filter((arg) => !arg.startsWith('-'));
  const options = {
    assistants: assistants.length > 0 ? assistants : undefined,
    all: args.includes('--all'),
  };
  await linkCommand(options);
  process.exit(0);
} else if (subcommand === 'unlink') {
  const { unlinkCommand } = await import('./src/cli/commands/unlink.js');
  const assistants = args.slice(1).filter((arg) => !arg.startsWith('-'));
  const options = {
    assistants: assistants.length > 0 ? assistants : undefined,
    all: args.includes('--all'),
  };
  await unlinkCommand(options);
  process.exit(0);
} else if (subcommand === 'stop') {
  const { stopCommand } = await import('./src/cli/commands/stop.js');
  await stopCommand();
  process.exit(0);
} else if (subcommand === '--help' || subcommand === '-h' || subcommand === 'help') {
  // Help is handled at the top
  process.exit(0);
} else if (subcommand === '--version' || subcommand === '-v') {
  const packageJson = await import('./package.json', { assert: { type: 'json' } });
  console.log(packageJson.default.version);
  process.exit(0);
  console.log('');
  console.log('Usage: codeflow-buddy <command> [options]');
  console.log('');
  console.log('Commands:');
  console.log('  setup         Interactive setup with language server selection');
  console.log("  status        Show what's working right now");
  console.log('  start         Start the MCP server for Claude Code');
  console.log('  serve         Start WebSocket server for multi-client support');
  console.log('  stop          Stop the running MCP server');
  console.log('  help          Show this help message');
  console.log('');
  console.log('Setup options:');
  console.log('  --all         Auto-install all language servers for detected file types');
  console.log('');
  console.log('Quick start:');
  console.log('  codeflow-buddy setup        # Interactive setup');
  console.log('  codeflow-buddy setup --all  # Auto-install all servers');
  console.log('  codeflow-buddy status       # Check server status');
  console.log('  codeflow-buddy start        # Start MCP server for Claude Code');
  console.log('');
  console.log('Configuration:');
  console.log('  Config file: .codebuddy/config.json');
  console.log('  Log file: .codebuddy/logs/debug.log');
  process.exit(0);
} else {
  console.error(`Unknown command: ${subcommand}`);
  console.error('');
  console.error('Available commands:');
  console.error('  setup    Interactive setup with language server selection');
  console.error("  status   Show what's working right now");
  console.error('  start    Start the MCP server for Claude Code');
  console.error('  stop     Stop the running MCP server');
  console.error('  help     Show help message');
  console.error('');
  console.error('Run "codeflow-buddy help" for more information.');
  process.exit(1);
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
            return await handleRenameSymbol(symbolService, args, newLspClient);
          case 'rename_symbol_strict':
            if (!Validation.validateRenameSymbolStrictArgs(args)) {
              throw Validation.createValidationError(
                'rename_symbol_strict',
                'object with file_path, line, character, and new_name'
              );
            }
            return await handleRenameSymbolStrict(symbolService, args, newLspClient);
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
            return await handleFormatDocument(fileService, args, newLspClient);
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
            return await handleApplyWorkspaceEdit(fileService, args, newLspClient);
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
          case 'batch_execute':
            if (!Validation.validateBatchExecuteArgs(args)) {
              throw Validation.createValidationError(
                'batch_execute',
                'object with operations array and options object'
              );
            }
            return await handleBatchExecute(
              symbolService,
              fileService,
              diagnosticService,
              intelligenceService,
              hierarchyService,
              newLspClient,
              args
            );
          default: {
            const { createUnknownToolMessage } = await import(
              './src/core/diagnostics/error-utils.js'
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

// Cleanup function for PID file
const cleanupPidFile = () => {
  try {
    const PID_FILE = join('.codebuddy', 'server.pid');
    if (existsSync(PID_FILE)) {
      unlinkSync(PID_FILE);
      logger.info('PID file removed');
    }
  } catch (error) {
    logger.warn('Could not remove PID file', { error: String(error) });
  }
};

process.on('SIGINT', async () => {
  logger.info('Received SIGINT, shutting down gracefully');
  cleanupPidFile();
  await newLspClient.dispose();
  process.exit(0);
});

process.on('SIGTERM', async () => {
  logger.info('Received SIGTERM, shutting down gracefully');
  cleanupPidFile();
  await newLspClient.dispose();
  process.exit(0);
});

process.on('exit', () => {
  cleanupPidFile();
});

async function main() {
  await StructuredLogger.withContextAsync(
    { operation: 'server_startup', component: 'MCP-Server' },
    async () => {
      logger.info('Codebuddy MCP server starting');

      // Save PID file for stop command
      const PID_DIR = '.codebuddy';
      const PID_FILE = join(PID_DIR, 'server.pid');

      try {
        if (!existsSync(PID_DIR)) {
          mkdirSync(PID_DIR, { recursive: true });
        }

        // Check if another server is already running
        if (existsSync(PID_FILE)) {
          const existingPid = Number.parseInt(readFileSync(PID_FILE, 'utf-8').trim(), 10);
          if (!Number.isNaN(existingPid)) {
            // Check if the process is actually running
            const { isProcessRunning } = await import('./src/utils/platform-utils.js');
            if (isProcessRunning(existingPid)) {
              console.error(`Error: MCP server is already running (PID: ${existingPid})`);
              console.error('Use "codeflow-buddy stop" to stop it first.');
              process.exit(1);
            } else {
              // Clean up stale PID file
              logger.info('Removing stale PID file', { pid: existingPid });
            }
          }
        }

        writeFileSync(PID_FILE, process.pid.toString());
        logger.info('PID file created', { pid: process.pid, file: PID_FILE });
      } catch (error) {
        logger.warn('Could not create PID file', { error: String(error) });
      }

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

main().catch(async (error) => {
  logger.error('Server startup error', error);
  await newLspClient.dispose();
  process.exit(1);
});

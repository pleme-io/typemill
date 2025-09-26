#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import {
  createRequestContext,
  getLogger,
  StructuredLogger,
} from './src/core/diagnostics/structured-logger.js';
import { LSPClient as NewLSPClient } from '../@codeflow/features/src/lsp/lsp-client.js';
import * as Validation from './src/mcp/comprehensive-validation.js';
import { allToolDefinitions } from './src/mcp/definitions/index.js';
import { allWorkflowDefinitions } from './src/mcp/definitions/workflow-definitions.js';
import { getTool } from './src/mcp/tool-registry.js';
import {
  handleAnalyzeImports,
  handleApplyWorkspaceEdit,
  handleBatchExecute,
  handleCreateFile,
  handleDeleteFile,
  handleExecuteWorkflow,
  handleFindDeadCode,
  handleFindDefinition,
  handleFindReferences,
  handleFixImports,
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
  handleRenameDirectory,
  handleRenameFile,
  handleRenameSymbol,
  handleRenameSymbolStrict,
  handleRestartServer,
  handleSearchWorkspaceSymbols,
  handleUpdatePackageJson,
} from './src/mcp/handlers/index.js';
import { getWorkflow, isWorkflowTool, registerWorkflows } from './src/mcp/tool-registry.js';
import { createMCPError } from './src/mcp/utils.js';
import { createWorkflowResponse, executeWorkflow } from './src/mcp/workflow-executor.js';
import { FileService } from './src/services/file-service.js';
import { HierarchyService } from './src/services/intelligence/hierarchy-service.js';
import { IntelligenceService } from './src/services/intelligence/intelligence-service.js';
import { DiagnosticService } from '../@codeflow/features/src/services/lsp/diagnostic-service.js';
import { SymbolService } from '../@codeflow/features/src/services/lsp/symbol-service.js';
import { PredictiveLoaderService } from './src/services/predictive-loader.js';
import { ServiceContainer } from './src/services/service-container.js';
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
  console.log('  check-fuse    Check FUSE availability');
  console.log('  mcp-debug     Debug MCP tools directly');
  console.log('');
  console.log('Setup options:');
  console.log('  --all                Install all available servers');
  console.log('  --force              Skip confirmation prompts');
  console.log('  --install-prereqs    Auto-install prerequisites (brew/apt)');
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
    console.log('  --auto-restart          Enable auto-restart on file changes');
    console.log('  --no-auto-restart       Disable auto-restart');
  }

  console.log('');
  console.log('For more options: codeflow-buddy help --advanced');
  process.exit(0);
}

const subcommand = args[0];

if (subcommand === 'setup') {
  const { setupCommand } = await import('./src/cli/commands/setup.js');

  // Parse server list from --servers flag
  const serversIndex = args.indexOf('--servers');
  const servers =
    serversIndex !== -1 && args[serversIndex + 1]
      ? args[serversIndex + 1]
          ?.split(',')
          .map((s) => s.trim())
          .filter(Boolean)
      : undefined;

  const options = {
    all: args.includes('--all'),
    force: args.includes('--force'),
    installPrereqs: args.includes('--install-prereqs'),
    servers,
  };
  await setupCommand(options);
  process.exit(0);
} else if (subcommand === 'status') {
  const { statusCommand } = await import('./src/cli/commands/status.js');
  await statusCommand();
  process.exit(0);
} else if (subcommand === 'check-fuse') {
  const { checkFuseAvailability, printFuseStatus } = await import('./src/fs/fuse-detector.js');
  const status = checkFuseAvailability();
  printFuseStatus(status);

  if (!status.available && args.includes('--setup')) {
    console.log('\nRunning FUSE setup...');
    const { setupFuse } = await import('./src/cli/fuse-setup.js');
    await setupFuse();
  }

  process.exit(status.available ? 0 : 1);
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
  const autoRestartIndex = args.indexOf('--auto-restart');
  const noAutoRestartIndex = args.indexOf('--no-auto-restart');

  const options = {
    port:
      portIndex !== -1 && args[portIndex + 1]
        ? Number.parseInt(args[portIndex + 1] || '3000', 10)
        : 3000,
    maxClients:
      maxClientsIndex !== -1 && args[maxClientsIndex + 1]
        ? Number.parseInt(args[maxClientsIndex + 1] || '10', 10)
        : 10,
    enableFuse: enableFuseIndex !== -1,
    requireAuth: requireAuthIndex !== -1,
    autoRestart: autoRestartIndex !== -1 ? true : noAutoRestartIndex !== -1 ? false : undefined,
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
} else if (subcommand === 'mcp-debug') {
  const { mcpDebugCommand } = await import('./src/cli/commands/mcp-debug.js');
  const toolName = args[1] || '';
  const toolArgs = args[2] || '';
  await mcpDebugCommand(toolName, toolArgs);
  process.exit(0);
} else if (subcommand === '--help' || subcommand === '-h' || subcommand === 'help') {
  // Help is handled at the top
  process.exit(0);
} else if (subcommand === '--version' || subcommand === '-v') {
  const packageJson = await import('./package.json', { assert: { type: 'json' } });
  console.log(packageJson.default.version);
  process.exit(0);
} else {
  console.error(`Unknown command: ${subcommand}`);
  console.error('');
  console.error('Available commands:');
  console.error('  setup      Interactive setup with language server selection');
  console.error("  status     Show what's working right now");
  console.error('  start      Start the MCP server for Claude Code');
  console.error('  stop       Stop the running MCP server');
  console.error('  mcp-debug  Debug MCP tools directly');
  console.error('  help       Show help message');
  console.error('');
  console.error('Run "codeflow-buddy help" for more information.');
  process.exit(1);
}

// Load configuration with server options
// The actual config is loaded by LSPClient, but we can set defaults here
const defaultServerOptions = {
  serverOptions: {
    enablePredictiveLoading: true, // Enable predictive loading by default
  },
};

// Create LSP clients and services with proper error handling
let newLspClient: NewLSPClient;
let serviceContainer: ServiceContainer;
let predictiveLoaderService: PredictiveLoaderService;

try {
  // Create new LSP client
  newLspClient = new NewLSPClient();

  // Create ServiceContext for all services
  const { ServiceContextUtils } = await import('./src/services/service-context.js');
  const { TransactionManager } = await import('./src/core/transaction/index.js');
  const transactionManager = new TransactionManager();

  // Get the loaded config from the LSP client (which loads it from file)
  const loadedConfig = (newLspClient as { config?: Record<string, unknown> }).config || {};
  const configWithDefaults = {
    ...loadedConfig,
    ...defaultServerOptions, // Merge in our defaults
  };

  const serviceContext = ServiceContextUtils.createServiceContext(
    newLspClient.getServer.bind(newLspClient),
    newLspClient.protocol,
    transactionManager,
    logger,
    configWithDefaults
  );

  // Initialize services with ServiceContext
  const symbolService = new SymbolService(serviceContext);
  const fileService = new FileService(serviceContext);
  const diagnosticService = new DiagnosticService(serviceContext);
  const intelligenceService = new IntelligenceService(serviceContext);
  const hierarchyService = new HierarchyService(serviceContext);

  // Set the FileService in TransactionManager to resolve circular dependency
  transactionManager.setFileService(fileService);

  // Create PredictiveLoaderService with context that includes fileService
  predictiveLoaderService = new PredictiveLoaderService({
    logger,
    openFile: (filePath: string) => fileService.openFileInternal(filePath),
    config: configWithDefaults,
  });

  // Add predictive loader and fileService references to the context
  serviceContext.predictiveLoader = predictiveLoaderService;
  serviceContext.fileService = fileService;

  // Create ServiceContainer with all services
  serviceContainer = ServiceContainer.create({
    symbolService,
    fileService,
    diagnosticService,
    intelligenceService,
    hierarchyService,
    lspClient: newLspClient,
    serviceContext,
    predictiveLoader: predictiveLoaderService,
    transactionManager,
    logger,
  });
} catch (error) {
  logger.error('Failed to initialize LSP clients', error);
  process.exit(1);
}

// Register workflow definitions
try {
  registerWorkflows(allWorkflowDefinitions);
  logger.info('Workflow definitions registered', {
    count: allWorkflowDefinitions.length,
    workflows: allWorkflowDefinitions.map((w) => w.name).join(', '),
  });
} catch (error) {
  logger.error('Failed to register workflow definitions', error);
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

  // Check for trace flag in arguments for enhanced debugging
  const isTraceEnabled = args && typeof args === 'object' && 'trace' in args && args.trace === true;

  return await StructuredLogger.withContextAsync(
    createRequestContext(requestId, 'MCP_CALL', `tool:${name}`),
    async () => {
      if (isTraceEnabled) {
        logger.info('🔧 MCP TRACE: Full request received', {
          tool: name,
          request_id: requestId,
          full_request: JSON.stringify(request, null, 2),
          args: JSON.stringify(args, null, 2),
        });
      } else {
        logger.info('MCP tool request', { tool: name, request_id: requestId });
      }

      try {
        // Check if this is a workflow tool
        if (isWorkflowTool(name)) {
          const workflowDef = getWorkflow(name);
          if (!workflowDef) {
            throw new Error(`Workflow definition not found: ${name}`);
          }

          if (isTraceEnabled) {
            logger.info('🔧 MCP TRACE: Executing workflow', {
              workflow: name,
              request_id: requestId,
              steps: workflowDef.steps.length,
            });
          }

          // Create a tool executor function for the workflow
          const toolExecutor = async (toolName: string, toolArgs: Record<string, unknown>) => {
            // Recursively call the main tool dispatcher for each step
            const _stepRequest = {
              method: 'tools/call' as const,
              params: {
                name: toolName,
                arguments: toolArgs,
              },
            };

            // Execute the tool (this will go through the normal tool execution path)
            return await (async () => {
              switch (toolName) {
                case 'find_definition':
                  if (!Validation.validateFindDefinitionArgs(toolArgs)) {
                    throw Validation.createValidationError(
                      'find_definition',
                      'object with file_path and symbol_name strings'
                    );
                  }
                  return await handleFindDefinition(serviceContainer.symbolService, toolArgs);
                case 'find_references':
                  if (!Validation.validateFindReferencesArgs(toolArgs)) {
                    throw Validation.createValidationError(
                      'find_references',
                      'object with file_path and symbol_name strings'
                    );
                  }
                  return await handleFindReferences(serviceContainer.symbolService, toolArgs);
                case 'get_document_symbols':
                  if (!Validation.validateGetDocumentSymbolsArgs(toolArgs)) {
                    throw Validation.createValidationError(
                      'get_document_symbols',
                      'object with file_path string'
                    );
                  }
                  return await handleGetDocumentSymbols(serviceContainer.symbolService, toolArgs);
                case 'get_diagnostics':
                  if (!Validation.validateGetDiagnosticsArgs(toolArgs)) {
                    throw Validation.createValidationError(
                      'get_diagnostics',
                      'object with file_path string'
                    );
                  }
                  return await handleGetDiagnostics(serviceContainer.diagnosticService, toolArgs);
                case 'get_code_actions':
                  if (!Validation.validateGetCodeActionsArgs(toolArgs)) {
                    throw Validation.createValidationError(
                      'get_code_actions',
                      'object with file_path string'
                    );
                  }
                  return await handleGetCodeActions(serviceContainer.fileService, toolArgs);
                case 'get_folding_ranges':
                  if (!Validation.validateGetFoldingRangesArgs(toolArgs)) {
                    throw Validation.createValidationError(
                      'get_folding_ranges',
                      'object with file_path string'
                    );
                  }
                  return await handleGetFoldingRanges(serviceContainer.fileService, toolArgs, serviceContainer.lspClient);
                case 'get_hover':
                  if (!Validation.validateGetHoverArgs(toolArgs)) {
                    throw Validation.createValidationError(
                      'get_hover',
                      'object with file_path, line, and character'
                    );
                  }
                  return await handleGetHover(serviceContainer.intelligenceService, toolArgs);
                case 'get_signature_help':
                  if (!Validation.validateGetSignatureHelpArgs(toolArgs)) {
                    throw Validation.createValidationError(
                      'get_signature_help',
                      'object with file_path, line, and character'
                    );
                  }
                  return await handleGetSignatureHelp(serviceContainer.intelligenceService, toolArgs);
                case 'prepare_call_hierarchy':
                  if (!Validation.validatePrepareCallHierarchyArgs(toolArgs)) {
                    throw Validation.createValidationError(
                      'prepare_call_hierarchy',
                      'object with file_path, line, and character'
                    );
                  }
                  return await handlePrepareCallHierarchy(serviceContainer.hierarchyService, toolArgs);
                default:
                  throw new Error(`Unsupported tool in workflow: ${toolName}`);
              }
            })();
          };

          // Execute the workflow
          const workflowResult = await executeWorkflow(
            workflowDef,
            args as Record<string, unknown>,
            toolExecutor
          );
          return createWorkflowResponse(workflowResult);
        }

        const result = await (async () => {
          // Dynamic dispatch using tool registry - MUCH better than giant switch!
          // Load handlers to trigger registration when running from source
          await import('./src/mcp/handlers/index.js');
          const toolEntry = getTool(name);

          if (!toolEntry) {
            const { createUnknownToolMessage } = await import(
              './src/core/diagnostics/error-utils.js'
            );
            const enhancedMessage = createUnknownToolMessage(name);
            throw new Error(enhancedMessage);
          }

          // Prepare arguments based on tool requirements
          const toolArgs = [];
          if (toolEntry.requiresService !== 'none') {
            const service = serviceContainer.getService(toolEntry.requiresService);
            if (toolEntry.requiresService === 'serviceContext') {
              // Special handling for serviceContext: args first, service second
              // This matches the batch executor's behavior and the handler signatures
              toolArgs.push(args, service);
            } else {
              // Standard handling: service first, args second
              toolArgs.push(service, args);
            }
          } else {
            toolArgs.push(args);
          }

          // Execute the tool via registry
          return await toolEntry.handler(...toolArgs);
        })();

        // Log the response if trace is enabled
        if (isTraceEnabled) {
          logger.info('🔧 MCP TRACE: Tool execution completed', {
            tool: name,
            request_id: requestId,
            response_type: typeof result,
            response: JSON.stringify(result, null, 2),
          });
        }

        return result;
      } catch (error) {
        logger.error('MCP tool request failed', error, { tool: name, request_id: requestId });
        return createMCPError(error);
      }
    }
  );
});

// Cleanup function for PID file
const cleanupPidFile = () => {
  if (process.env.SKIP_PID_FILE === 'true') {
    return; // Skip PID file cleanup in test mode
  }
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

      // Save PID file for stop command (skip in test mode)
      const skipPidFile = process.env.SKIP_PID_FILE === 'true';

      if (!skipPidFile) {
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
              const { isProcessRunning } = await import('./src/utils/platform/process.js');
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
      } else {
        logger.info('Skipping PID file management (test mode)');
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

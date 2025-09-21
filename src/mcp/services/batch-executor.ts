import type { LSPClient } from '../../lsp/client.js';
import type { DiagnosticService } from '../../services/diagnostic-service.js';
import type { FileService } from '../../services/file-service.js';
import type { HierarchyService } from '../../services/hierarchy-service.js';
import type { IntelligenceService } from '../../services/intelligence-service.js';
import type { ServiceContext } from '../../services/service-context.js';
import type { SymbolService } from '../../services/symbol-service.js';
import { createMCPResponse } from '../utils.js';

// Import all existing handlers for dynamic calling
import * as AdvancedHandlers from '../handlers/advanced-handlers.js';
import * as CoreHandlers from '../handlers/core-handlers.js';
import * as HierarchyHandlers from '../handlers/hierarchy-handlers.js';
import * as IntelligenceHandlers from '../handlers/intelligence-handlers.js';
import * as UtilityHandlers from '../handlers/utility-handlers.js';

export interface BatchOperation {
  tool: string;
  args: unknown;
  id?: string;
}

export interface BatchOptions {
  atomic?: boolean;
  parallel?: boolean;
  dry_run?: boolean;
  stop_on_error?: boolean;
}

export interface BatchExecuteArgs {
  operations: BatchOperation[];
  options: BatchOptions;
}

export interface BatchResult {
  success: boolean;
  results: Array<{
    operation: BatchOperation;
    success: boolean;
    result?: unknown;
    error?: string;
  }>;
  summary: {
    total: number;
    successful: number;
    failed: number;
    skipped: number;
  };
  execution_mode: string;
  dry_run: boolean;
}

// Registry of all available MCP tools and their handlers
const TOOL_REGISTRY: Record<
  string,
  {
    handler: Function;
    requiresService:
      | 'symbol'
      | 'file'
      | 'diagnostic'
      | 'intelligence'
      | 'hierarchy'
      | 'lsp'
      | 'serviceContext'
      | 'none';
  }
> = {
  // Core tools
  find_definition: { handler: CoreHandlers.handleFindDefinition, requiresService: 'symbol' },
  find_references: { handler: CoreHandlers.handleFindReferences, requiresService: 'symbol' },
  rename_symbol: { handler: CoreHandlers.handleRenameSymbol, requiresService: 'symbol' },
  rename_symbol_strict: {
    handler: CoreHandlers.handleRenameSymbolStrict,
    requiresService: 'symbol',
  },

  // Advanced tools
  get_code_actions: { handler: AdvancedHandlers.handleGetCodeActions, requiresService: 'file' },
  format_document: { handler: AdvancedHandlers.handleFormatDocument, requiresService: 'file' },
  search_workspace_symbols: {
    handler: AdvancedHandlers.handleSearchWorkspaceSymbols,
    requiresService: 'symbol',
  },
  get_document_symbols: {
    handler: AdvancedHandlers.handleGetDocumentSymbols,
    requiresService: 'symbol',
  },
  get_folding_ranges: { handler: AdvancedHandlers.handleGetFoldingRanges, requiresService: 'file' },
  get_document_links: { handler: AdvancedHandlers.handleGetDocumentLinks, requiresService: 'file' },
  apply_workspace_edit: {
    handler: AdvancedHandlers.handleApplyWorkspaceEdit,
    requiresService: 'file',
  },

  // Intelligence tools
  get_hover: { handler: IntelligenceHandlers.handleGetHover, requiresService: 'intelligence' },
  get_completions: {
    handler: IntelligenceHandlers.handleGetCompletions,
    requiresService: 'intelligence',
  },
  get_inlay_hints: {
    handler: IntelligenceHandlers.handleGetInlayHints,
    requiresService: 'intelligence',
  },
  get_semantic_tokens: {
    handler: IntelligenceHandlers.handleGetSemanticTokens,
    requiresService: 'intelligence',
  },
  get_signature_help: {
    handler: IntelligenceHandlers.handleGetSignatureHelp,
    requiresService: 'intelligence',
  },

  // Hierarchy tools
  prepare_call_hierarchy: {
    handler: HierarchyHandlers.handlePrepareCallHierarchy,
    requiresService: 'hierarchy',
  },
  get_call_hierarchy_incoming_calls: {
    handler: HierarchyHandlers.handleGetCallHierarchyIncomingCalls,
    requiresService: 'hierarchy',
  },
  get_call_hierarchy_outgoing_calls: {
    handler: HierarchyHandlers.handleGetCallHierarchyOutgoingCalls,
    requiresService: 'hierarchy',
  },
  prepare_type_hierarchy: {
    handler: HierarchyHandlers.handlePrepareTypeHierarchy,
    requiresService: 'hierarchy',
  },
  get_type_hierarchy_supertypes: {
    handler: HierarchyHandlers.handleGetTypeHierarchySupertypes,
    requiresService: 'hierarchy',
  },
  get_type_hierarchy_subtypes: {
    handler: HierarchyHandlers.handleGetTypeHierarchySubtypes,
    requiresService: 'hierarchy',
  },
  get_selection_range: {
    handler: HierarchyHandlers.handleGetSelectionRange,
    requiresService: 'hierarchy',
  },

  // Utility tools
  get_diagnostics: { handler: UtilityHandlers.handleGetDiagnostics, requiresService: 'diagnostic' },
  restart_server: { handler: UtilityHandlers.handleRestartServer, requiresService: 'lsp' },
  rename_file: { handler: UtilityHandlers.handleRenameFile, requiresService: 'none' },
  create_file: { handler: UtilityHandlers.handleCreateFile, requiresService: 'none' },
  delete_file: { handler: UtilityHandlers.handleDeleteFile, requiresService: 'none' },
  health_check: { handler: UtilityHandlers.handleHealthCheck, requiresService: 'serviceContext' },
};

export class BatchExecutor {
  constructor(
    private symbolService: SymbolService,
    private fileService: FileService,
    private diagnosticService: DiagnosticService,
    private intelligenceService: IntelligenceService,
    private hierarchyService: HierarchyService,
    private lspClient: LSPClient,
    private serviceContext: ServiceContext
  ) {}

  async execute(args: BatchExecuteArgs): Promise<BatchResult> {
    const { operations, options } = args;
    const { atomic = false, parallel = false, dry_run = false, stop_on_error = true } = options;

    const result: BatchResult = {
      success: true,
      results: [],
      summary: {
        total: operations.length,
        successful: 0,
        failed: 0,
        skipped: 0,
      },
      execution_mode: parallel ? 'parallel' : 'sequential',
      dry_run,
    };

    // Validate all operations first
    const validationErrors = this.validateOperations(operations);
    if (validationErrors.length > 0) {
      result.success = false;
      for (let i = 0; i < operations.length; i++) {
        const operation = operations[i];
        if (!operation) continue;

        const error = validationErrors.find((e) => e.operationIndex === i);
        result.results.push({
          operation,
          success: false,
          error: error?.error || 'Unknown validation error',
        });
        result.summary.failed++;
      }
      return result;
    }

    if (dry_run) {
      return this.previewOperations(operations, result);
    }

    if (parallel) {
      return this.executeParallel(operations, result, stop_on_error);
    }
    return this.executeSequential(operations, result, atomic, stop_on_error);
  }

  private validateOperations(
    operations: BatchOperation[]
  ): Array<{ operationIndex: number; error: string }> {
    const errors: Array<{ operationIndex: number; error: string }> = [];

    for (let i = 0; i < operations.length; i++) {
      const op = operations[i];
      if (!op) {
        errors.push({ operationIndex: i, error: 'Null operation' });
        continue;
      }

      if (!op.tool || typeof op.tool !== 'string') {
        errors.push({ operationIndex: i, error: 'Tool name is required and must be a string' });
        continue;
      }

      if (!TOOL_REGISTRY[op.tool]) {
        errors.push({
          operationIndex: i,
          error: `Unknown tool: ${op.tool}. Available tools: ${Object.keys(TOOL_REGISTRY).join(', ')}`,
        });
        continue;
      }

      if (op.tool === 'batch_execute') {
        errors.push({ operationIndex: i, error: 'Recursive batch execution is not allowed' });
      }
    }

    return errors;
  }

  private async previewOperations(
    operations: BatchOperation[],
    result: BatchResult
  ): Promise<BatchResult> {
    for (const operation of operations) {
      const toolInfo = TOOL_REGISTRY[operation.tool];
      if (!toolInfo) continue;

      try {
        // For preview mode, we describe what would happen rather than executing
        const previewText = this.generatePreviewText(operation);

        result.results.push({
          operation,
          success: true,
          result: createMCPResponse(`[PREVIEW] ${previewText}`),
        });
        result.summary.successful++;
      } catch (error) {
        result.results.push({
          operation,
          success: false,
          error: error instanceof Error ? error.message : String(error),
        });
        result.summary.failed++;
      }
    }

    return result;
  }

  private generatePreviewText(operation: BatchOperation): string {
    const { tool, args } = operation;

    switch (tool) {
      case 'find_definition':
        return `Would find definition for symbol "${(args as any)?.symbol_name}" in ${(args as any)?.file_path}`;
      case 'find_references':
        return `Would find references for symbol "${(args as any)?.symbol_name}" in ${(args as any)?.file_path}`;
      case 'rename_symbol':
        return `Would rename symbol "${(args as any)?.symbol_name}" to "${(args as any)?.new_name}" in ${(args as any)?.file_path}`;
      case 'rename_file':
        return `Would rename file from "${(args as any)?.old_path}" to "${(args as any)?.new_path}"`;
      case 'format_document':
        return `Would format document ${(args as any)?.file_path}`;
      case 'get_diagnostics':
        return `Would get diagnostics for ${(args as any)?.file_path}`;
      case 'apply_workspace_edit': {
        const changes = (args as any)?.changes;
        const fileCount = changes ? Object.keys(changes).length : 0;
        return `Would apply workspace edits to ${fileCount} file(s)`;
      }
      default:
        return `Would execute ${tool} with provided arguments`;
    }
  }

  private async executeSequential(
    operations: BatchOperation[],
    result: BatchResult,
    atomic: boolean,
    stopOnError: boolean
  ): Promise<BatchResult> {
    const executedOperations: Array<{ operation: BatchOperation; result: unknown }> = [];

    for (const operation of operations) {
      try {
        const operationResult = await this.executeOperation(operation);

        result.results.push({
          operation,
          success: true,
          result: operationResult,
        });
        result.summary.successful++;

        executedOperations.push({ operation, result: operationResult });
      } catch (error) {
        result.results.push({
          operation,
          success: false,
          error: error instanceof Error ? error.message : String(error),
        });
        result.summary.failed++;
        result.success = false;

        if (atomic) {
          // Rollback previous operations if atomic mode
          await this.rollbackOperations(executedOperations);
          result.summary.successful = 0; // Reset successful count after rollback
          break;
        }

        if (stopOnError) {
          // Mark remaining operations as skipped
          const currentIndex = operations.indexOf(operation);
          for (let i = currentIndex + 1; i < operations.length; i++) {
            const skippedOp = operations[i];
            if (skippedOp) {
              result.results.push({
                operation: skippedOp,
                success: false,
                error: 'Skipped due to previous error',
              });
              result.summary.skipped++;
            }
          }
          break;
        }
      }
    }

    return result;
  }

  private async executeParallel(
    operations: BatchOperation[],
    result: BatchResult,
    stopOnError: boolean
  ): Promise<BatchResult> {
    const promises = operations.map(async (operation) => {
      try {
        const operationResult = await this.executeOperation(operation);
        return {
          operation,
          success: true,
          result: operationResult,
        };
      } catch (error) {
        return {
          operation,
          success: false,
          error: error instanceof Error ? error.message : String(error),
        };
      }
    });

    const results = await Promise.allSettled(promises);

    for (const settledResult of results) {
      if (settledResult.status === 'fulfilled') {
        const opResult = settledResult.value;
        result.results.push(opResult);

        if (opResult.success) {
          result.summary.successful++;
        } else {
          result.summary.failed++;
          result.success = false;
        }
      } else {
        // This shouldn't happen since we catch errors in the promise
        result.summary.failed++;
        result.success = false;
      }
    }

    return result;
  }

  private async executeOperation(operation: BatchOperation): Promise<unknown> {
    const toolInfo = TOOL_REGISTRY[operation.tool];
    if (!toolInfo) {
      throw new Error(`Unknown tool: ${operation.tool}`);
    }

    const serviceArg = this.getServiceArgument(toolInfo.requiresService);

    // Call the handler with appropriate service
    if (toolInfo.requiresService === 'none') {
      return await toolInfo.handler(operation.args);
    }
    if (toolInfo.requiresService === 'lsp') {
      return await toolInfo.handler(this.lspClient, operation.args);
    }
    if (toolInfo.requiresService === 'serviceContext') {
      return await toolInfo.handler(operation.args, this.serviceContext);
    }
    return await toolInfo.handler(serviceArg, operation.args, this.lspClient);
  }

  private getServiceArgument(serviceType: string): unknown {
    switch (serviceType) {
      case 'symbol':
        return this.symbolService;
      case 'file':
        return this.fileService;
      case 'diagnostic':
        return this.diagnosticService;
      case 'intelligence':
        return this.intelligenceService;
      case 'hierarchy':
        return this.hierarchyService;
      case 'lsp':
        return this.lspClient;
      case 'serviceContext':
        return this.serviceContext;
      default:
        return undefined;
    }
  }

  private async rollbackOperations(
    executedOperations: Array<{ operation: BatchOperation; result: unknown }>
  ): Promise<void> {
    // Rollback operations in reverse order
    for (let i = executedOperations.length - 1; i >= 0; i--) {
      const { operation } = executedOperations[i] || {};
      if (!operation) continue;

      try {
        await this.rollbackOperation(operation);
      } catch (error) {
        // Log rollback errors but don't fail the entire rollback process
        console.error(`Failed to rollback operation ${operation.tool}:`, error);
      }
    }
  }

  private async rollbackOperation(operation: BatchOperation): Promise<void> {
    // Only certain operations can be rolled back
    switch (operation.tool) {
      case 'rename_file': {
        // Reverse the file rename
        const args = operation.args as any;
        if (args?.old_path && args?.new_path) {
          await UtilityHandlers.handleRenameFile({
            old_path: args.new_path,
            new_path: args.old_path,
          });
        }
        break;
      }
      case 'create_file': {
        // Delete the created file
        const createArgs = operation.args as any;
        if (createArgs?.file_path) {
          await UtilityHandlers.handleDeleteFile({
            file_path: createArgs.file_path,
            force: true,
          });
        }
        break;
      }
      // Other operations like find_definition, get_diagnostics don't need rollback
      // as they are read-only operations
      default:
        // No rollback needed for read-only operations
        break;
    }
  }

  // Static method to get available tools for validation
  static getAvailableTools(): string[] {
    return Object.keys(TOOL_REGISTRY);
  }

  // Static method to check if a tool exists
  static isValidTool(toolName: string): boolean {
    return toolName in TOOL_REGISTRY;
  }
}

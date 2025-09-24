import type { LSPClient } from '../lsp/lsp-client.js';
import { getTool, getToolNames } from '../mcp/tool-registry.js';
import { createMCPResponse } from '../mcp/utils.js';
import type { FileService } from './file-service.js';
import type { HierarchyService } from './intelligence/hierarchy-service.js';
import type { IntelligenceService } from './intelligence/intelligence-service.js';
import type { DiagnosticService } from './lsp/diagnostic-service.js';
import type { SymbolService } from './lsp/symbol-service.js';
import type { ServiceContext } from './service-context.js';

// Import handlers to trigger their registration
// These imports only have side effects (registering tools)
import '../mcp/handlers/core-handlers.js';
import '../mcp/handlers/advanced-handlers.js';
import '../mcp/handlers/hierarchy-handlers.js';
import '../mcp/handlers/intelligence-handlers.js';
import '../mcp/handlers/utility-handlers.js';

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

      if (!getTool(op.tool)) {
        errors.push({
          operationIndex: i,
          error: `Unknown tool: ${op.tool}. Available tools: ${getToolNames().join(', ')}`,
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
      const toolInfo = getTool(operation.tool);
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

    // Type guard helper for safer property access
    const getArg = (key: string): string | undefined => {
      if (args && typeof args === 'object' && key in args) {
        return String((args as Record<string, unknown>)[key]);
      }
      return undefined;
    };

    switch (tool) {
      case 'find_definition':
        return `Would find definition for symbol "${getArg('symbol_name') || 'unknown'}" in ${getArg('file_path') || 'unknown'}`;
      case 'find_references':
        return `Would find references for symbol "${getArg('symbol_name') || 'unknown'}" in ${getArg('file_path') || 'unknown'}`;
      case 'rename_symbol':
        return `Would rename symbol "${getArg('symbol_name') || 'unknown'}" to "${getArg('new_name') || 'unknown'}" in ${getArg('file_path') || 'unknown'}`;
      case 'rename_file':
        return `Would rename file from "${getArg('old_path') || 'unknown'}" to "${getArg('new_path') || 'unknown'}"`;
      case 'format_document':
        return `Would format document ${getArg('file_path') || 'unknown'}`;
      case 'get_diagnostics':
        return `Would get diagnostics for ${getArg('file_path') || 'unknown'}`;
      case 'apply_workspace_edit': {
        const changes =
          args && typeof args === 'object' && 'changes' in args
            ? (args as Record<string, unknown>).changes
            : undefined;
        const fileCount = changes && typeof changes === 'object' ? Object.keys(changes).length : 0;
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
    if (atomic) {
      // Start a new transaction
      this.serviceContext.transactionManager.beginTransaction();
      await this.serviceContext.transactionManager.saveCheckpoint('before-batch');
    }

    for (const operation of operations) {
      try {
        const operationResult = await this.executeOperation(operation);

        result.results.push({
          operation,
          success: true,
          result: operationResult,
        });
        result.summary.successful++;
      } catch (error) {
        result.results.push({
          operation,
          success: false,
          error: error instanceof Error ? error.message : String(error),
        });
        result.summary.failed++;
        result.success = false;

        if (atomic) {
          // Rollback to checkpoint
          await this.serviceContext.transactionManager.rollbackToCheckpoint('before-batch');
          this.serviceContext.transactionManager.commit(); // End the transaction
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

    // Commit transaction if atomic and successful
    if (atomic && result.success) {
      this.serviceContext.transactionManager.commit();
    }

    return result;
  }

  private async executeParallel(
    operations: BatchOperation[],
    result: BatchResult,
    _stopOnError: boolean
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
    const toolInfo = getTool(operation.tool);
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

  // Static method to get available tools for validation
  static getAvailableTools(): string[] {
    return getToolNames();
  }

  // Static method to check if a tool exists
  static isValidTool(toolName: string): boolean {
    return getTool(toolName) !== undefined;
  }
}

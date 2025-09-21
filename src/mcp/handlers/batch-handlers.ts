import type { LSPClient } from '../../lsp/client.js';
import type { DiagnosticService } from '../../services/diagnostic-service.js';
import type { FileService } from '../../services/file-service.js';
import type { HierarchyService } from '../../services/hierarchy-service.js';
import type { IntelligenceService } from '../../services/intelligence-service.js';
import { ServiceContextUtils } from '../../services/service-context.js';
import type { SymbolService } from '../../services/symbol-service.js';
import { type BatchExecuteArgs, BatchExecutor } from '../services/batch-executor.js';
import { createMCPResponse } from '../utils.js';

/**
 * Universal batch execution handler
 * Replaces specific batch tools (analyze_refactor_impact, batch_move_files, preview_batch_operation)
 * with a generalized system that can batch any MCP operations
 */
export async function handleBatchExecute(
  symbolService: SymbolService,
  fileService: FileService,
  diagnosticService: DiagnosticService,
  intelligenceService: IntelligenceService,
  hierarchyService: HierarchyService,
  lspClient: LSPClient,
  args: BatchExecuteArgs
) {
  try {
    // Create service context for operations that need it
    const serviceContext = ServiceContextUtils.createServiceContext(
      lspClient.getServer.bind(lspClient),
      lspClient.protocol
    );

    // Create batch executor with all required services
    const batchExecutor = new BatchExecutor(
      symbolService,
      fileService,
      diagnosticService,
      intelligenceService,
      hierarchyService,
      lspClient,
      serviceContext
    );

    // Execute the batch operation
    const result = await batchExecutor.execute(args);

    // Format the response
    let summary = '## Batch Execution Results\\n\\n';
    summary += `**Total Operations**: ${result.summary.total}\\n`;
    summary += `**Successful**: ${result.summary.successful}\\n`;
    summary += `**Failed**: ${result.summary.failed}\\n`;
    summary += `**Skipped**: ${result.summary.skipped}\\n`;
    summary += `**Execution Mode**: ${result.execution_mode}\\n`;
    summary += `**Dry Run**: ${result.dry_run ? 'Yes' : 'No'}\\n\\n`;

    if (result.dry_run) {
      summary += '### 🔍 Preview Results\\n\\n';
    } else if (result.success) {
      summary += '### ✅ Execution Results\\n\\n';
    } else {
      summary += '### ❌ Execution Results (with errors)\\n\\n';
    }

    // Add individual operation results
    for (let i = 0; i < result.results.length; i++) {
      const opResult = result.results[i];
      if (!opResult) continue;

      const { operation, success, result: opResultData, error } = opResult;
      const operationId = operation.id || `op-${i + 1}`;
      const statusIcon = success ? '✅' : '❌';

      summary += `**${statusIcon} ${operationId}**: ${operation.tool}\\n`;

      if (success && opResultData) {
        // Extract meaningful content from the MCP response
        const content =
          typeof opResultData === 'object' && opResultData !== null && 'content' in opResultData
            ? String((opResultData as any).content)
            : String(opResultData);

        // Truncate long responses for summary
        const truncatedContent = content.length > 200 ? `${content.substring(0, 200)}...` : content;

        summary += `   Result: ${truncatedContent}\\n\\n`;
      } else if (error) {
        summary += `   Error: ${error}\\n\\n`;
      }
    }

    // Add summary statistics
    if (result.summary.failed > 0) {
      summary += '### ⚠️ Summary\\n\\n';
      if (result.summary.successful > 0) {
        summary += `Partial success: ${result.summary.successful}/${result.summary.total} operations completed successfully.\\n`;
      } else {
        summary += 'All operations failed. Please check the error messages above.\\n';
      }

      if (result.summary.skipped > 0) {
        summary += `${result.summary.skipped} operations were skipped due to errors.\\n`;
      }
    } else {
      summary += '### 🎉 All operations completed successfully!\\n';
    }

    // Add helpful tips
    if (result.dry_run) {
      summary += '\\n💡 **Tip**: Run again with `"dry_run": false` to execute the operations.\\n';
    }

    if (!result.dry_run && args.options.atomic === false && result.summary.failed > 0) {
      summary +=
        '\\n💡 **Tip**: Use `"atomic": true` for all-or-nothing execution with automatic rollback.\\n';
    }

    return createMCPResponse(summary);
  } catch (error) {
    return createMCPResponse(
      `Error in batch execution: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

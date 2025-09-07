/**
 * Centralized debug logging utility for consistent logging across the application
 */
export const DebugLogger = {
  /**
   * Log a request operation with file path and optional parameters
   */
  logRequest(operation: string, filePath: string, params?: unknown): void {
    const paramsStr = params ? ` ${JSON.stringify(params)}` : '';
    process.stderr.write(`[DEBUG ${operation}] ${filePath}${paramsStr}\n`);
  },

  /**
   * Log operation results with type and length information
   */
  logResult(operation: string, result: unknown): void {
    const resultType = typeof result;
    const length = Array.isArray(result) ? result.length : 'N/A';
    process.stderr.write(`[DEBUG ${operation}] Result: ${resultType}, length: ${length}\n`);
  },

  /**
   * Log symbol matches found during operations
   */
  logSymbolMatches(operation: string, symbolName: string, count: number): void {
    process.stderr.write(
      `[DEBUG ${operation}] Found ${count} symbol matches for "${symbolName}"\n`
    );
  },

  /**
   * Log errors during operations
   */
  logError(operation: string, error: unknown, context?: string): void {
    const message = error instanceof Error ? error.message : String(error);
    const contextStr = context ? ` (${context})` : '';
    process.stderr.write(`[ERROR ${operation}]${contextStr} ${message}\n`);
  },

  /**
   * Log server operations and status
   */
  logServerOperation(operation: string, serverCommand: string, status?: string): void {
    const statusStr = status ? ` - ${status}` : '';
    process.stderr.write(`[DEBUG ServerManager] ${operation}: ${serverCommand}${statusStr}\n`);
  },
} as const;

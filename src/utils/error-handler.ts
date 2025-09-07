import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';

/**
 * Centralized error handling utility for consistent error responses
 */
export const ErrorHandler = {
  /**
   * Handle LSP-related errors and return formatted MCP response
   */
  handleLSPError(error: unknown, operation: string, filePath?: string): CallToolResult {
    const message = error instanceof Error ? error.message : String(error);
    const context = filePath ? ` for ${filePath}` : '';

    return {
      content: [
        {
          type: 'text',
          text: `Error in ${operation}${context}: ${message}`,
        },
      ],
      isError: true,
    };
  },

  /**
   * Handle file operation errors
   */
  handleFileError(error: unknown, filePath: string, operation: string): never {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${operation} failed for ${filePath}: ${message}`);
  },

  /**
   * Handle server initialization errors
   */
  handleServerError(error: unknown, serverCommand: string): CallToolResult {
    const message = error instanceof Error ? error.message : String(error);

    return {
      content: [
        {
          type: 'text',
          text: `Server initialization failed for ${serverCommand}: ${message}`,
        },
      ],
      isError: true,
    };
  },

  /**
   * Handle timeout errors specifically
   */
  handleTimeoutError(operation: string, timeout: number, filePath?: string): CallToolResult {
    const context = filePath ? ` for ${filePath}` : '';

    return {
      content: [
        {
          type: 'text',
          text: `${operation} timed out after ${timeout}ms${context}. The operation may still complete in the background.`,
        },
      ],
      isError: true,
    };
  },

  /**
   * Create a generic error response
   */
  createErrorResponse(message: string): CallToolResult {
    return {
      content: [{ type: 'text', text: message }],
      isError: true,
    };
  },
} as const;

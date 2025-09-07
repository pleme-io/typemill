import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';

/**
 * Utility for building consistent MCP responses
 */
export const MCPResponseBuilder = {
  /**
   * Create a successful response with a message
   */
  success(message: string): CallToolResult {
    return {
      content: [{ type: 'text', text: message }],
    };
  },

  /**
   * Create a "not found" response for a specific resource
   */
  notFound(resource: string, context?: string): CallToolResult {
    const contextStr = context ? ` in ${context}` : '';
    return {
      content: [
        {
          type: 'text',
          text: `No ${resource} found${contextStr}. Try different search criteria.`,
        },
      ],
    };
  },

  /**
   * Create a response with count information for collections
   */
  withCount(items: unknown[], resource: string, limit?: number): CallToolResult {
    const total = items.length;
    const showing = limit && total > limit ? limit : total;

    let message: string;
    if (limit && total > limit) {
      message = `Found ${total} ${resource} (showing first ${showing})`;
    } else {
      message = `Found ${total} ${resource}`;
    }

    return this.success(message);
  },

  /**
   * Create a response for dry run operations
   */
  dryRun(operation: string, changes: string[]): CallToolResult {
    const message = `[DRY RUN] ${operation} would make the following changes:\n\n${changes.join('\n')}`;
    return this.success(message);
  },

  /**
   * Create a response for operations with warnings
   */
  withWarning(message: string, warning: string): CallToolResult {
    const fullMessage = `${message}\n\n⚠️  Warning: ${warning}`;
    return this.success(fullMessage);
  },

  /**
   * Create a response for partial results
   */
  partialResult(message: string, reason: string): CallToolResult {
    const fullMessage = `${message}\n\nNote: ${reason}`;
    return this.success(fullMessage);
  },

  /**
   * Create a response with formatted list
   */
  list(items: string[], title?: string): CallToolResult {
    const titleStr = title ? `${title}:\n\n` : '';
    const listStr = items.map((item, i) => `${i + 1}. ${item}`).join('\n');
    return this.success(`${titleStr}${listStr}`);
  },

  /**
   * Create an empty result response
   */
  empty(resource: string): CallToolResult {
    return this.success(`No ${resource} to display.`);
  },
} as const;

// Shared MCP utilities
import {
  ConfigurationError,
  FileSystemError,
  LSPError,
  ServerNotAvailableError,
  createUserFriendlyErrorMessage,
  logError,
} from '../utils/error-utils.js';

export interface MCPResponse {
  content: Array<{
    type: 'text';
    text: string;
  }>;
}

export function createMCPResponse(text: string): MCPResponse {
  return {
    content: [
      {
        type: 'text',
        text,
      },
    ],
  };
}

/**
 * Create MCP error response with enhanced error handling
 */
export function createMCPError(
  error: unknown,
  operation = 'operation',
  context?: { filePath?: string }
): MCPResponse {
  // Log the error for debugging
  logError('MCP', `Error during ${operation}`, error, context);

  // Create user-friendly error message with context
  const userMessage = createUserFriendlyErrorMessage(error, operation, undefined, context);

  return createMCPResponse(userMessage);
}

/**
 * Create MCP error response with suggestions
 */
function createMCPErrorWithSuggestions(
  error: unknown,
  operation: string,
  suggestions: string[]
): MCPResponse {
  logError('MCP', `Error during ${operation}`, error, { suggestions });
  const userMessage = createUserFriendlyErrorMessage(error, operation, suggestions);
  return createMCPResponse(userMessage);
}

/**
 * Create an MCP response for unsupported features with helpful guidance
 */
export function createUnsupportedFeatureResponse(
  featureName: string,
  serverDescription: string,
  missingCapabilities: string[],
  alternativeSuggestions?: string[]
): MCPResponse {
  let text = `❌ **${featureName} not supported** by ${serverDescription}\n\n`;

  text += `**Missing capabilities:** ${missingCapabilities.join(', ')}\n\n`;

  text += `**What this means:** The language server for this file type doesn't provide ${featureName.toLowerCase()} functionality. This is a limitation of the server, not Codebuddy.\n\n`;

  if (alternativeSuggestions && alternativeSuggestions.length > 0) {
    text += `**Alternatives:**\n${alternativeSuggestions.map((suggestion) => `• ${suggestion}`).join('\n')}\n\n`;
  }

  text +=
    '**Note:** Different language servers support different features. TypeScript and Rust servers typically have the most comprehensive support.';

  return createMCPResponse(text);
}

/**
 * Create a warning response for features with limited server support
 */
export function createLimitedSupportResponse(
  featureName: string,
  serverDescription: string,
  warningMessage: string,
  result?: string
): MCPResponse {
  let text = `⚠️ **${featureName}** - Limited support on ${serverDescription}\n\n`;
  text += `**Warning:** ${warningMessage}\n\n`;

  if (result) {
    text += `**Result:**\n${result}`;
  } else {
    text += '**Result:** Feature attempted but may not work as expected on this server.';
  }

  return createMCPResponse(text);
}

/**
 * Safely handle MCP operation with consistent error handling
 */
async function withMCPErrorHandling<T>(
  operation: () => Promise<MCPResponse>,
  operationName: string
): Promise<MCPResponse> {
  try {
    return await operation();
  } catch (error) {
    return createMCPError(error, operationName);
  }
}

/**
 * Create context-aware error message for different error types
 */
export function createContextualErrorResponse(
  error: unknown,
  context: {
    operation: string;
    filePath?: string;
    suggestions?: string[];
  }
): MCPResponse {
  if (error instanceof ServerNotAvailableError) {
    const suggestions = context.suggestions || [
      'Run `codebuddy setup` to configure language servers',
      `Install the required language server: ${error.command.join(' ')}`,
      'Check that the language server is in your PATH',
    ];
    return createMCPErrorWithSuggestions(error, context.operation, suggestions);
  }

  if (error instanceof FileSystemError) {
    const suggestions = context.suggestions || [
      'Check that the file path is correct',
      'Verify file permissions',
      'Ensure the file exists',
    ];
    return createMCPErrorWithSuggestions(error, context.operation, suggestions);
  }

  if (error instanceof ConfigurationError) {
    const suggestions = context.suggestions || [
      'Run `codebuddy setup` to reconfigure',
      'Check your codebuddy.json file syntax',
      'Verify configuration paths are correct',
    ];
    return createMCPErrorWithSuggestions(error, context.operation, suggestions);
  }

  if (error instanceof LSPError) {
    const suggestions = context.suggestions || [
      'Try restarting the language server',
      'Check server logs for more details',
      'Verify the file is supported by the language server',
    ];
    return createMCPErrorWithSuggestions(error, context.operation, suggestions);
  }

  return createMCPError(error, context.operation);
}

// Response builders for common MCP response patterns

/**
 * Create a success response with simple text
 */
export function createSuccessResponse(message: string): MCPResponse {
  return createMCPResponse(`✅ ${message}`);
}

/**
 * Create a no-results response for searches/queries
 */
export function createNoResultsResponse(
  operation: string,
  target: string,
  suggestions?: string[]
): MCPResponse {
  let message = `No ${operation} found for ${target}.`;

  if (suggestions && suggestions.length > 0) {
    message += ` ${suggestions.join(' ')}`;
  }

  return createMCPResponse(message);
}

/**
 * Create a response for operations with a dry-run mode
 */
function createDryRunResponse(operation: string, details: string): MCPResponse {
  return createMCPResponse(`[DRY RUN] Would ${operation}. ${details}`);
}

/**
 * Create a response with a count of results and formatted list
 */
export function createListResponse(
  title: string,
  items: string[],
  options: {
    singular?: string;
    plural?: string;
    maxItems?: number;
    showTotal?: boolean;
  } = {}
): MCPResponse {
  const { singular = 'item', plural = 'items', maxItems = 50, showTotal = true } = options;

  if (items.length === 0) {
    return createMCPResponse(`No ${plural} found${title ? ` for ${title}` : ''}.`);
  }

  const displayItems = items.slice(0, maxItems);
  const itemWord = items.length === 1 ? singular : plural;

  let response = '';

  if (showTotal) {
    response += `Found ${items.length} ${itemWord}`;
    if (title) response += ` ${title}`;
    if (items.length > maxItems) {
      response += ` (showing first ${maxItems})`;
    }
    response += ':\n\n';
  }

  response += displayItems.join('\n');

  return createMCPResponse(response);
}

/**
 * Create a response for operations that modify files
 */
export function createFileModificationResponse(
  operation: string,
  filePath: string,
  details?: {
    changeCount?: number;
    fileCount?: number;
    additionalInfo?: string;
  }
): MCPResponse {
  let message = `✅ Successfully ${operation}`;

  if (details?.fileCount && details.fileCount > 1) {
    message += ` across ${details.fileCount} file${details.fileCount === 1 ? '' : 's'}`;
  } else {
    message += ` ${filePath}`;
  }

  if (details?.changeCount) {
    message += ` with ${details.changeCount} change${details.changeCount === 1 ? '' : 's'}`;
  }

  if (details?.additionalInfo) {
    message += `\n\n${details.additionalInfo}`;
  }

  return createMCPResponse(message);
}

/**
 * Create a response for location/position-based operations
 */
function createLocationResponse(
  operation: string,
  filePath: string,
  position: { line: number; character: number },
  results: string[]
): MCPResponse {
  const positionStr = `${filePath}:${position.line}:${position.character}`;

  if (results.length === 0) {
    return createMCPResponse(`No ${operation} found at ${positionStr}`);
  }

  const title = `${operation.charAt(0).toUpperCase()}${operation.slice(1)} at ${positionStr}`;
  return createListResponse(title, results, { showTotal: false });
}

/**
 * Create a response for operations that failed with no changes
 */
export function createNoChangesResponse(operation: string, reason?: string): MCPResponse {
  let message = 'No changes needed';
  if (operation) message += ` for ${operation}`;
  if (reason) message += ` - ${reason}`;
  message += '.';
  return createMCPResponse(message);
}

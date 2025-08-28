// Shared MCP utilities

interface MCPResponse {
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

export function createMCPError(error: unknown): MCPResponse {
  const message = error instanceof Error ? error.message : String(error);
  return createMCPResponse(`Error: ${message}`);
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

  text += `**What this means:** The language server for this file type doesn't provide ${featureName.toLowerCase()} functionality. This is a limitation of the server, not CCLSP.\n\n`;

  if (alternativeSuggestions && alternativeSuggestions.length > 0) {
    text += `**Alternatives:**\n${alternativeSuggestions.map((suggestion) => `• ${suggestion}`).join('\n')}\n\n`;
  }

  text +=
    '**Note:** Different language servers support different features. TypeScript and Rust servers typically have the most comprehensive support.';

  return createMCPResponse(text);
}

/**
 * Create a capability validation response with helpful debugging info
 */
function createCapabilityInfoResponse(
  filePath: string,
  serverDescription: string,
  capabilityInfo: string
): MCPResponse {
  const text = `## Server Capabilities for ${filePath}\n\n**Server:** ${serverDescription}\n\n**Available Features:**\n${capabilityInfo}\n\n*This information helps debug which LSP features are available for this file type.*`;

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

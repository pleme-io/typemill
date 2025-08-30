// src/mcp/utils.ts
function createMCPResponse(text) {
  return {
    content: [
      {
        type: "text",
        text
      }
    ]
  };
}
function createMCPError(error) {
  const message = error instanceof Error ? error.message : String(error);
  return createMCPResponse(`Error: ${message}`);
}
function createUnsupportedFeatureResponse(featureName, serverDescription, missingCapabilities, alternativeSuggestions) {
  let text = `❌ **${featureName} not supported** by ${serverDescription}

`;
  text += `**Missing capabilities:** ${missingCapabilities.join(", ")}

`;
  text += `**What this means:** The language server for this file type doesn't provide ${featureName.toLowerCase()} functionality. This is a limitation of the server, not CCLSP.

`;
  if (alternativeSuggestions && alternativeSuggestions.length > 0) {
    text += `**Alternatives:**
${alternativeSuggestions.map((suggestion) => `• ${suggestion}`).join(`
`)}

`;
  }
  text += "**Note:** Different language servers support different features. TypeScript and Rust servers typically have the most comprehensive support.";
  return createMCPResponse(text);
}
function createLimitedSupportResponse(featureName, serverDescription, warningMessage, result) {
  let text = `⚠️ **${featureName}** - Limited support on ${serverDescription}

`;
  text += `**Warning:** ${warningMessage}

`;
  if (result) {
    text += `**Result:**
${result}`;
  } else {
    text += "**Result:** Feature attempted but may not work as expected on this server.";
  }
  return createMCPResponse(text);
}
export {
  createUnsupportedFeatureResponse,
  createMCPResponse,
  createMCPError,
  createLimitedSupportResponse
};

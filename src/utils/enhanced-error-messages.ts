/**
 * Enhanced, user-friendly error messages with actionable guidance
 */

import { allToolDefinitions } from '../mcp/definitions/index.js';
import { getLogger } from './structured-logger.js';

const logger = getLogger('EnhancedErrors');

/**
 * Create contextual error message for LSP server not available
 */
export function createLSPServerUnavailableMessage(filePath: string, operation: string): string {
  const extension = filePath.split('.').pop()?.toLowerCase() || 'unknown';

  // Common language mappings
  const languageInfo = getLanguageInfo(extension);

  let message = `❌ **${operation} not available** for ${languageInfo.name} files\n\n`;
  message += `**What happened:** No language server is configured for .${extension} files\n\n`;
  message += '**To fix this:**\n';

  if (languageInfo.servers.length > 0) {
    message += `1. **Install a ${languageInfo.name} language server:**\n`;
    for (let i = 0; i < languageInfo.servers.length; i++) {
      const server = languageInfo.servers[i];
      if (server) {
        message += `   ${i + 1}. ${server.install} (${server.description})\n`;
      }
    }
    message += '\n2. **Configure Codebuddy:**\n';
    message += `   Run: \`codebuddy init\` and select ${languageInfo.name} support\n\n`;
  } else {
    message += '1. Run: `codebuddy init` to set up language servers\n';
    message += `2. Check if there's a language server available for .${extension} files\n\n`;
  }

  message += '**Alternative:** Try the operation on a supported file type:\n';
  message += '• TypeScript/JavaScript (.ts, .js) - Full support\n';
  message += '• Python (.py) - Good support\n';
  message += '• Go (.go) - Good support\n';

  return message;
}

/**
 * Create contextual error message for server initialization failures
 */
export function createServerInitializationMessage(
  filePath: string,
  operation: string,
  serverCommand?: string
): string {
  const extension = filePath.split('.').pop()?.toLowerCase() || 'unknown';
  const languageInfo = getLanguageInfo(extension);

  let message = `⚠️ **${operation} temporarily unavailable**\n\n`;
  message += `**What happened:** The ${languageInfo.name} language server is starting up or encountered an error\n\n`;
  message += '**Quick fixes to try:**\n';
  message +=
    '1. **Wait a moment** - Language servers need time to initialize (especially TypeScript)\n';
  message += `2. **Restart the server:** \`codebuddy restart ${extension}\` or restart Codebuddy\n`;
  message += '3. **Check installation:** Verify the language server is installed correctly\n\n';

  if (serverCommand) {
    message += `**Server command:** \`${serverCommand}\`\n`;
    message += '**Manual test:** Run the above command to check if it works\n\n';
  }

  message += '**If this persists:**\n';
  message += '• Check `codebuddy logs` for error details\n';
  message += '• Run `codebuddy status` to see server health\n';
  message += '• Try `codebuddy fix` to auto-repair configuration\n';

  return message;
}

/**
 * Create helpful message for unknown tool errors with suggestions
 */
export function createUnknownToolMessage(toolName: string): string {
  const availableTools = allToolDefinitions.map((t) => t.name);
  const suggestions = findSimilarTools(toolName, availableTools);

  let message = `❌ **Unknown tool:** \`${toolName}\`\n\n`;

  if (suggestions.length > 0) {
    message += '**Did you mean:**\n';
    for (const suggestion of suggestions.slice(0, 3)) {
      message += `• \`${suggestion.name}\` - ${suggestion.description}\n`;
    }
    message += '\n';
  }

  message += '**Available tools:**\n';
  message += '• **Navigation:** find_definition, find_references, search_workspace_symbols\n';
  message += '• **Intelligence:** get_hover, get_completions, get_diagnostics\n';
  message += '• **Refactoring:** rename_symbol, format_document, get_code_actions\n';
  message += '• **Hierarchy:** prepare_call_hierarchy, prepare_type_hierarchy\n';
  message += '• **System:** health_check, restart_server\n\n';

  message += '**Full list:** Use the MCP client to list all available tools\n';

  return message;
}

/**
 * Create helpful message for file not found errors
 */
export function createFileNotFoundMessage(filePath: string, operation: string): string {
  let message = `❌ **File not found:** \`${filePath}\`\n\n`;
  message += `**What happened:** Cannot perform ${operation} - file doesn't exist\n\n`;
  message += '**Please check:**\n';
  message += '• **Path spelling:** Verify the file path is correct\n';
  message += '• **Current directory:** Are you in the right folder?\n';
  message += '• **File existence:** Does the file actually exist?\n';
  message += '• **Permissions:** Do you have read access to the file?\n\n';

  // Try to suggest similar files in the directory
  try {
    const dir = filePath.substring(0, filePath.lastIndexOf('/')) || '.';
    const fileName = filePath.substring(filePath.lastIndexOf('/') + 1);
    message += `**Tip:** Check if similar files exist in \`${dir}/\`\n`;
  } catch {
    // Ignore path parsing errors
  }

  return message;
}

/**
 * Get language-specific information for better error messages
 */
function getLanguageInfo(extension: string): {
  name: string;
  servers: Array<{ install: string; description: string }>;
} {
  const languageMap: Record<
    string,
    { name: string; servers: Array<{ install: string; description: string }> }
  > = {
    ts: {
      name: 'TypeScript',
      servers: [
        {
          install: 'npm install -g typescript-language-server typescript',
          description: 'Official TypeScript server',
        },
      ],
    },
    tsx: {
      name: 'TypeScript React',
      servers: [
        {
          install: 'npm install -g typescript-language-server typescript',
          description: 'Official TypeScript server',
        },
      ],
    },
    js: {
      name: 'JavaScript',
      servers: [
        {
          install: 'npm install -g typescript-language-server typescript',
          description: 'TypeScript server (works for JS)',
        },
      ],
    },
    jsx: {
      name: 'JavaScript React',
      servers: [
        {
          install: 'npm install -g typescript-language-server typescript',
          description: 'TypeScript server (works for JSX)',
        },
      ],
    },
    py: {
      name: 'Python',
      servers: [
        { install: 'pip install python-lsp-server', description: 'Python Language Server' },
        { install: 'pip install pylsp', description: 'Alternative Python server' },
      ],
    },
    go: {
      name: 'Go',
      servers: [
        {
          install: 'go install golang.org/x/tools/gopls@latest',
          description: 'Official Go language server',
        },
      ],
    },
    rs: {
      name: 'Rust',
      servers: [
        { install: 'rustup component add rust-analyzer', description: 'Official Rust analyzer' },
      ],
    },
    java: {
      name: 'Java',
      servers: [{ install: 'Download Eclipse JDT Language Server', description: 'Eclipse JDT LS' }],
    },
    cpp: {
      name: 'C++',
      servers: [
        {
          install: 'Install clangd via your package manager',
          description: 'Clang language server',
        },
      ],
    },
    c: {
      name: 'C',
      servers: [
        {
          install: 'Install clangd via your package manager',
          description: 'Clang language server',
        },
      ],
    },
  };

  return (
    languageMap[extension] || {
      name: extension.toUpperCase(),
      servers: [],
    }
  );
}

/**
 * Find tools with similar names using simple string similarity
 */
function findSimilarTools(
  input: string,
  availableTools: string[]
): Array<{ name: string; description: string }> {
  const similarities = availableTools
    .map((tool) => ({
      name: tool,
      description: getToolDescription(tool),
      similarity: calculateSimilarity(input.toLowerCase(), tool.toLowerCase()),
    }))
    .filter((item) => item.similarity > 0.4) // Only include reasonably similar tools
    .sort((a, b) => b.similarity - a.similarity);

  return similarities;
}

/**
 * Get description for a tool name
 */
function getToolDescription(toolName: string): string {
  const tool = allToolDefinitions.find((t) => t.name === toolName);
  return tool?.description || 'No description available';
}

/**
 * Calculate simple string similarity using Levenshtein-like algorithm
 */
function calculateSimilarity(str1: string, str2: string): number {
  const longer = str1.length > str2.length ? str1 : str2;
  const shorter = str1.length > str2.length ? str2 : str1;

  if (longer.length === 0) return 1.0;

  // Check for substring matches (higher weight)
  if (longer.includes(shorter) || shorter.includes(longer)) {
    return 0.8;
  }

  // Simple character overlap
  const overlap = [...shorter].filter((char) => longer.includes(char)).length;
  return overlap / longer.length;
}

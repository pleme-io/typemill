import { advancedToolDefinitions } from './advanced-tools.js';
// All MCP tool definitions
import { coreToolDefinitions } from './core-tools.js';
import { hierarchyToolDefinitions } from './hierarchy-tools.js';
import { intelligenceToolDefinitions } from './intelligence-tools.js';
import { utilityToolDefinitions } from './utility-tools.js';

// Combine all tool definitions
export const allToolDefinitions = [
  ...coreToolDefinitions,
  ...advancedToolDefinitions,
  ...utilityToolDefinitions,
  ...intelligenceToolDefinitions,
  ...hierarchyToolDefinitions,
] as const;
// Re-export individual categories for convenience

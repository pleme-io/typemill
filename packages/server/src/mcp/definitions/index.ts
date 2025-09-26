import { advancedToolDefinitions } from './advanced-tools.js';
import { ANALYSIS_TOOLS } from './analysis-tools.js';
import { batchToolDefinitions } from './batch-tools.js';
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
  ...batchToolDefinitions,
  ...ANALYSIS_TOOLS,
] as const;
// Re-export individual categories for convenience

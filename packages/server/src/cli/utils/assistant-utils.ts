import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import type { AssistantInfo } from '../types.js';

/**
 * AI Assistant configuration management utilities
 */

interface AssistantDefinition {
  name: string;
  displayName: string;
  configPaths: {
    linux?: string;
    darwin?: string;
    win32?: string;
  };
  configFormat: 'mcpServers' | 'custom';
}

const SUPPORTED_ASSISTANTS: AssistantDefinition[] = [
  {
    name: 'claude',
    displayName: 'Claude Desktop',
    configPaths: {
      linux: '~/.config/claude_desktop/claude_desktop_config.json',
      darwin: '~/Library/Application Support/Claude/claude_desktop_config.json',
      win32: '%APPDATA%\\Claude\\claude_desktop_config.json',
    },
    configFormat: 'mcpServers',
  },
  {
    name: 'cursor',
    displayName: 'Cursor',
    configPaths: {
      linux: '.cursor/mcp.json',
      darwin: '.cursor/mcp.json',
      win32: '.cursor/mcp.json',
    },
    configFormat: 'mcpServers',
  },
  {
    name: 'gemini',
    displayName: 'Gemini Code Assist',
    configPaths: {
      linux: '~/.gemini/settings.json',
      darwin: '~/.gemini/settings.json',
      win32: '~/.gemini/settings.json',
    },
    configFormat: 'mcpServers',
  },
];

/**
 * Expand path with environment variables, home directory, and project-relative paths
 */
function expandPath(filepath: string): string {
  // Handle project-relative paths (like .cursor/mcp.json)
  if (filepath.startsWith('./') || (!filepath.includes('/') && !filepath.includes('\\'))) {
    if (filepath.startsWith('./')) {
      filepath = path.join(process.cwd(), filepath.slice(2));
    } else if (filepath.startsWith('.')) {
      filepath = path.join(process.cwd(), filepath);
    }
  }
  // Expand home directory
  else if (filepath.startsWith('~/')) {
    filepath = path.join(os.homedir(), filepath.slice(2));
  }

  // Expand environment variables on Windows
  if (process.platform === 'win32') {
    filepath = filepath.replace(/%([^%]+)%/g, (_, name) => {
      return process.env[name] || '';
    });
  }

  return path.resolve(filepath);
}

/**
 * Get the configuration path for an assistant on the current platform
 */
function getAssistantConfigPath(assistant: AssistantDefinition): string | null {
  const platform = process.platform as 'linux' | 'darwin' | 'win32';
  const configPath = assistant.configPaths[platform];

  if (!configPath) {
    return null;
  }

  return expandPath(configPath);
}

/**
 * Check if an assistant is installed/available
 */
function isAssistantInstalled(assistant: AssistantDefinition): boolean {
  const configPath = getAssistantConfigPath(assistant);
  if (!configPath) {
    return false;
  }

  try {
    // Special handling for different assistant types
    if (assistant.name === 'cursor') {
      // For Cursor, we consider it available if we're in a valid project directory
      // The .cursor directory will be created when linking if it doesn't exist
      return process.cwd() !== '/'; // Basic check that we're not in root
    }
    // For Claude Desktop and Gemini, check if parent directory exists
    const configDir = path.dirname(configPath);
    return fs.existsSync(configDir);
  } catch {
    return false;
  }
}

/**
 * Read assistant configuration file
 */
export function readAssistantConfig(configPath: string): Record<string, unknown> | null {
  try {
    const expandedPath = expandPath(configPath);
    if (!fs.existsSync(expandedPath)) {
      // Config file doesn't exist yet, return empty structure
      return { mcpServers: {} };
    }

    const content = fs.readFileSync(expandedPath, 'utf-8');
    return JSON.parse(content);
  } catch (error) {
    console.error(`Error reading config from ${configPath}:`, error);
    return null;
  }
}

/**
 * Write assistant configuration file with backup
 */
export function writeAssistantConfig(configPath: string, config: Record<string, unknown>): void {
  const expandedPath = expandPath(configPath);

  try {
    // Ensure directory exists
    const configDir = path.dirname(expandedPath);
    if (!fs.existsSync(configDir)) {
      fs.mkdirSync(configDir, { recursive: true });
    }

    // Create backup if file exists
    if (fs.existsSync(expandedPath)) {
      const backupPath = `${expandedPath}.backup`;
      fs.copyFileSync(expandedPath, backupPath);
    }

    // Write config with pretty formatting
    fs.writeFileSync(expandedPath, `${JSON.stringify(config, null, 2)}\n`, 'utf-8');
  } catch (error) {
    throw new Error(`Failed to write config to ${configPath}: ${error}`);
  }
}

/**
 * Check if codeflow-buddy is linked to an assistant
 */
export function isLinked(assistant: AssistantInfo): boolean {
  const configPath = assistant.configPath;
  if (!configPath) {
    return false;
  }

  const config = readAssistantConfig(configPath);
  if (!config || !config.mcpServers) {
    return false;
  }

  return 'codeflow-buddy' in config.mcpServers;
}

/**
 * Find all installed AI assistants
 */
export function findInstalledAssistants(): AssistantInfo[] {
  return SUPPORTED_ASSISTANTS.map((assistant) => {
    const configPath = getAssistantConfigPath(assistant);
    const installed = isAssistantInstalled(assistant);

    const info: AssistantInfo = {
      name: assistant.name,
      displayName: assistant.displayName,
      configPath: configPath || '',
      installed,
      linked: false,
    };

    // Check if linked (only if installed)
    if (installed && configPath) {
      info.linked = isLinked(info);
    }

    return info;
  });
}

/**
 * Add MCP server to assistant configuration
 */
export function addMCPServer(
  configPath: string,
  serverName: string,
  serverConfig: Record<string, unknown>
): void {
  const config = readAssistantConfig(configPath) || { mcpServers: {} };

  if (!config.mcpServers) {
    config.mcpServers = {};
  }

  config.mcpServers[serverName] = serverConfig;

  writeAssistantConfig(configPath, config);
}

/**
 * Remove MCP server from assistant configuration
 */
export function removeMCPServer(configPath: string, serverName: string): boolean {
  const config = readAssistantConfig(configPath);

  if (!config || !config.mcpServers || !(serverName in config.mcpServers)) {
    return false;
  }

  delete config.mcpServers[serverName];

  writeAssistantConfig(configPath, config);
  return true;
}

/**
 * Get MCP server configuration for codeflow-buddy
 */
export function getMCPServerConfig(): Record<string, unknown> {
  // Determine the command to run
  const isGlobalInstall = __filename.includes('npm/global') || __filename.includes('.npm-global');

  if (isGlobalInstall) {
    // Global install - use the codeflow-buddy command
    return {
      command: 'codeflow-buddy',
      args: ['start'],
    };
  }
  // Local install - use node with the full path
  const scriptPath = path.resolve(__dirname, '../../../index.js');
  return {
    command: 'node',
    args: [scriptPath, 'start'],
  };
}

/**
 * Get assistant by name
 */
export function getAssistantByName(name: string): AssistantInfo | null {
  const assistants = findInstalledAssistants();

  // Exact match
  const exact = assistants.find((a) => a.name.toLowerCase() === name.toLowerCase());
  if (exact) {
    return exact;
  }

  // Fuzzy match for typos
  const fuzzy = assistants.find(
    (a) =>
      a.name.toLowerCase().includes(name.toLowerCase()) ||
      a.displayName.toLowerCase().includes(name.toLowerCase())
  );

  return fuzzy || null;
}

/**
 * Get all assistant names for help text
 */
export function getAllAssistantNames(): string[] {
  return SUPPORTED_ASSISTANTS.map((a) => a.name);
}

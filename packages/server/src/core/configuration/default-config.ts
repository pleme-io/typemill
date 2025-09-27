import type { Config, LSPServerConfig } from '../../types.js';

// Configuration constants
const SERVER_AVAILABILITY_CHECK_TIMEOUT_MS = 2000; // Timeout for checking server availability

/**
 * Default configurations for common language servers
 * These are carefully chosen to work on most systems
 * @internal - Internal use only, not part of public API
 */
export const DEFAULT_SERVERS: LSPServerConfig[] = [
  {
    // TypeScript/JavaScript - Already bundled as dependency!
    extensions: ['ts', 'tsx', 'js', 'jsx', 'mjs', 'cjs'],
    command: ['npx', '--', 'typescript-language-server', '--stdio'],
    // restartInterval removed - opt-in only
  },
  {
    // Python - Via pipx
    extensions: ['py', 'pyi'],
    command: ['pylsp'],
    // restartInterval removed - opt-in only
  },
  {
    // Rust - Via rustup
    extensions: ['rs'],
    command: ['rust-analyzer'],
    // restartInterval removed - opt-in only
  },
  {
    // JSON/YAML - Via vscode-json-languageserver
    extensions: ['json', 'jsonc'],
    command: [
      'npx',
      '--',
      'vscode-langservers-extracted',
      '--',
      'vscode-json-language-server',
      '--stdio',
    ],
    // restartInterval removed - opt-in only
  },
  {
    // HTML/CSS - Via vscode-css-languageserver
    extensions: ['html', 'htm'],
    command: [
      'npx',
      '--',
      'vscode-langservers-extracted',
      '--',
      'vscode-html-language-server',
      '--stdio',
    ],
    // restartInterval removed - opt-in only
  },
  {
    // CSS/SCSS/LESS
    extensions: ['css', 'scss', 'sass', 'less'],
    command: [
      'npx',
      '--',
      'vscode-langservers-extracted',
      '--',
      'vscode-css-language-server',
      '--stdio',
    ],
    // restartInterval removed - opt-in only
  },
  {
    // Vue
    extensions: ['vue'],
    command: ['npx', '--', 'vue-language-server', '--stdio'],
    // restartInterval removed - opt-in only
  },
  {
    // Svelte
    extensions: ['svelte'],
    command: ['npx', '--', 'svelteserver', '--stdio'],
    // restartInterval removed - opt-in only
  },
  {
    // C/C++ via clangd
    extensions: ['c', 'cpp', 'cc', 'cxx', 'h', 'hpp'],
    command: ['clangd'],
    // restartInterval removed - opt-in only
  },
  {
    // Java
    extensions: ['java'],
    command: ['jdtls'],
    // restartInterval removed - opt-in only
  },
  {
    // Ruby
    extensions: ['rb', 'ruby'],
    command: ['solargraph', 'stdio'],
    // restartInterval removed - opt-in only
  },
  {
    // PHP
    extensions: ['php'],
    command: ['intelephense', '--stdio'],
    // restartInterval removed - opt-in only
  },
  {
    // Shell scripts
    extensions: ['sh', 'bash', 'zsh'],
    command: ['npx', '--', 'bash-language-server', 'start'],
    // restartInterval removed - opt-in only
  },
  {
    // Dockerfile
    extensions: ['dockerfile', 'Dockerfile'],
    command: ['docker-langserver', '--stdio'],
    // restartInterval removed - opt-in only
  },
  {
    // YAML
    extensions: ['yaml', 'yml'],
    command: ['npx', '--', 'yaml-language-server', '--stdio'],
    // restartInterval removed - opt-in only
  },
];

/**
 * Create default configuration
 */
export function createDefaultConfig(): Config {
  return {
    servers: DEFAULT_SERVERS,
  };
}

/**
 * Check if a command is available on the system
 */
async function isCommandAvailable(command: string[]): Promise<boolean> {
  try {
    const { isCommandAvailable: checkCommand } = await import(
      '../../utils/platform/command-utils.js'
    );
    return await checkCommand(command, SERVER_AVAILABILITY_CHECK_TIMEOUT_MS);
  } catch {
    return false;
  }
}

/**
 * Filter default servers to only those with available commands
 */
export async function getAvailableDefaultServers(): Promise<LSPServerConfig[]> {
  const available: LSPServerConfig[] = [];

  for (const server of DEFAULT_SERVERS) {
    // Always include npx-based servers since typescript-language-server is bundled
    if (server.command[0] === 'npx') {
      available.push(server);
      continue;
    }

    // Check if the command is available
    if (await isCommandAvailable(server.command)) {
      available.push(server);
    }
  }

  return available;
}

/**
 * Merge user config with defaults
 * User config takes precedence
 */
export function mergeWithDefaults(userConfig?: Partial<Config>): Config {
  if (!userConfig?.servers) {
    return createDefaultConfig();
  }

  // Create a map of user-configured extensions
  const userExtensions = new Set<string>();
  for (const server of userConfig.servers) {
    for (const ext of server.extensions) {
      userExtensions.add(ext);
    }
  }

  // Add default servers for unconfigured extensions
  const mergedServers = [...userConfig.servers];
  for (const defaultServer of DEFAULT_SERVERS) {
    const hasUnconfiguredExtension = defaultServer.extensions.some(
      (ext) => !userExtensions.has(ext)
    );

    if (hasUnconfiguredExtension) {
      // Only add extensions that aren't already configured
      const unconfiguredExtensions = defaultServer.extensions.filter(
        (ext) => !userExtensions.has(ext)
      );

      if (unconfiguredExtensions.length > 0) {
        mergedServers.push({
          ...defaultServer,
          extensions: unconfiguredExtensions,
        });
      }
    }
  }

  return {
    servers: mergedServers,
  };
}

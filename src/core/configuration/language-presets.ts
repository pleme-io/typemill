import { getCommandPath } from '../../cli/utils/server-utils.js';
import type { Config as CLIConfig } from '../../types.js';

export interface LanguageServerConfig {
  name: string;
  displayName: string;
  extensions: string[];
  command: string[];
  installInstructions: string;
  installCommand?: string[]; // Command to auto-install the server
  rootDir?: string;
  description?: string;
  installRequired?: boolean;
  restartInterval?: number; // Default restart interval in minutes
  initializationOptions?: unknown; // Default LSP initialization options
}

export const LANGUAGE_SERVERS: LanguageServerConfig[] = [
  {
    name: 'typescript',
    displayName: 'TypeScript/JavaScript',
    extensions: ['js', 'ts', 'jsx', 'tsx'],
    command: ['npx', '--', 'typescript-language-server', '--stdio'],
    installInstructions: 'npm install -g typescript-language-server',
    installCommand: ['npm', 'install', '-g', 'typescript-language-server', 'typescript'],
    description: 'TypeScript and JavaScript language server',
    installRequired: false,
  },
  {
    name: 'python',
    displayName: 'Python (pylsp)',
    extensions: ['py', 'pyi'],
    command: ['pylsp'],
    installInstructions: 'pip install python-lsp-server',
    installCommand: ['pip', 'install', 'python-lsp-server'],
    description: 'Python Language Server Protocol implementation (comprehensive features)',
    installRequired: false,
    restartInterval: 5, // Auto-restart every 5 minutes to prevent performance degradation
    initializationOptions: {
      settings: {
        pylsp: {
          plugins: {
            jedi_completion: { enabled: true },
            jedi_definition: { enabled: true },
            jedi_hover: { enabled: true },
            jedi_references: { enabled: true },
            jedi_signature_help: { enabled: true },
            jedi_symbols: { enabled: true },
            pylint: { enabled: false },
            pycodestyle: { enabled: false },
            pyflakes: { enabled: false },
            yapf: { enabled: false },
            rope_completion: { enabled: false },
          },
        },
      },
    },
  },
  {
    name: 'pyright',
    displayName: 'Python (Pyright)',
    extensions: ['py', 'pyi'],
    command: ['npx', '--', 'pyright-langserver', '--stdio'],
    installInstructions: 'npm install -g pyright',
    installCommand: ['npm', 'install', '-g', 'pyright'],
    description: 'Microsoft Pyright language server (fast type checking)',
    installRequired: false,
    restartInterval: 10, // Less frequent restarts as it's more stable
  },
  {
    name: 'go',
    displayName: 'Go',
    extensions: ['go'],
    command: ['gopls'],
    installInstructions: 'go install golang.org/x/tools/gopls@latest',
    installCommand: ['go', 'install', 'golang.org/x/tools/gopls@latest'],
    description: 'Official language server for the Go language',
    installRequired: true,
  },
  {
    name: 'rust',
    displayName: 'Rust',
    extensions: ['rs'],
    command: ['rust-analyzer'],
    installInstructions: 'rustup component add rust-analyzer',
    installCommand: ['rustup', 'component', 'add', 'rust-analyzer'],
    description: 'Rust language server providing IDE-like features',
    installRequired: true,
  },
  {
    name: 'json',
    displayName: 'JSON',
    extensions: ['json', 'jsonc'],
    command: ['npx', '--', 'vscode-json-language-server', '--stdio'],
    installInstructions: 'npm install -g vscode-langservers-extracted',
    installCommand: ['npm', 'install', '-g', 'vscode-langservers-extracted'],
    description: 'JSON language server with schema validation',
    installRequired: false,
  },
  {
    name: 'html',
    displayName: 'HTML',
    extensions: ['html', 'htm'],
    command: ['npx', '--', 'vscode-html-language-server', '--stdio'],
    installInstructions: 'npm install -g vscode-langservers-extracted',
    installCommand: ['npm', 'install', '-g', 'vscode-langservers-extracted'],
    description: 'HTML language server with IntelliSense',
    installRequired: false,
  },
  {
    name: 'css',
    displayName: 'CSS',
    extensions: ['css', 'scss', 'sass', 'less'],
    command: ['npx', '--', 'vscode-css-language-server', '--stdio'],
    installInstructions: 'npm install -g vscode-langservers-extracted',
    installCommand: ['npm', 'install', '-g', 'vscode-langservers-extracted'],
    description: 'CSS/SCSS/LESS language server',
    installRequired: false,
  },
  {
    name: 'yaml',
    displayName: 'YAML',
    extensions: ['yaml', 'yml'],
    command: ['npx', '--', 'yaml-language-server', '--stdio'],
    installInstructions: 'npm install -g yaml-language-server',
    installCommand: ['npm', 'install', '-g', 'yaml-language-server'],
    description: 'YAML language server with schema support',
    installRequired: false,
  },
  {
    name: 'bash',
    displayName: 'Bash/Shell',
    extensions: ['sh', 'bash', 'zsh'],
    command: ['npx', '--', 'bash-language-server', 'start'],
    installInstructions: 'npm install -g bash-language-server',
    installCommand: ['npm', 'install', '-g', 'bash-language-server'],
    description: 'Bash language server for shell scripts',
    installRequired: false,
  },
  {
    name: 'c-cpp',
    displayName: 'C/C++',
    extensions: ['c', 'cpp', 'cc', 'h', 'hpp'],
    command: ['clangd'],
    installInstructions: 'Install clangd via your system package manager',
    description: 'LLVM-based language server for C and C++',
    installRequired: true,
  },
  {
    name: 'java',
    displayName: 'Java',
    extensions: ['java'],
    command: ['jdtls'],
    installInstructions: 'Download Eclipse JDT Language Server',
    description: 'Eclipse JDT Language Server for Java',
    installRequired: true,
  },
  {
    name: 'ruby',
    displayName: 'Ruby',
    extensions: ['rb'],
    command: ['solargraph', 'stdio'],
    installInstructions: 'gem install solargraph',
    installCommand: ['gem', 'install', 'solargraph'],
    description: 'Ruby language server providing IntelliSense',
    installRequired: true,
  },
  {
    name: 'php',
    displayName: 'PHP',
    extensions: ['php'],
    command: ['intelephense', '--stdio'],
    installInstructions: 'npm install -g intelephense',
    installCommand: ['npm', 'install', '-g', 'intelephense'],
    description: 'PHP language server with advanced features',
    installRequired: true,
  },
  {
    name: 'csharp',
    displayName: 'C#',
    extensions: ['cs'],
    command: ['omnisharp', '-lsp'],
    installInstructions: 'Install OmniSharp language server',
    description: 'Language server for C# and .NET',
    installRequired: true,
  },
  {
    name: 'swift',
    displayName: 'Swift',
    extensions: ['swift'],
    command: ['sourcekit-lsp'],
    installInstructions: 'Comes with Swift toolchain',
    description: 'Language server for Swift programming language',
    installRequired: true,
  },
  {
    name: 'kotlin',
    displayName: 'Kotlin',
    extensions: ['kt', 'kts'],
    command: ['kotlin-language-server'],
    installInstructions: 'Download from kotlin-language-server releases',
    description: 'Language server for Kotlin programming language',
    installRequired: true,
  },
  {
    name: 'dart',
    displayName: 'Dart/Flutter',
    extensions: ['dart'],
    command: ['dart', 'language-server'],
    installInstructions: 'Install with Dart SDK',
    description: 'Dart language server for Dart and Flutter development',
    installRequired: true,
  },
  {
    name: 'elixir',
    displayName: 'Elixir',
    extensions: ['ex', 'exs'],
    command: ['elixir-ls'],
    installInstructions: 'Install ElixirLS language server',
    description: 'Language server for Elixir programming language',
    installRequired: true,
  },
  {
    name: 'haskell',
    displayName: 'Haskell',
    extensions: ['hs', 'lhs'],
    command: ['haskell-language-server-wrapper', '--lsp'],
    installInstructions: 'Install via ghcup or stack',
    description: 'Haskell Language Server for Haskell development',
    installRequired: true,
  },
  {
    name: 'lua',
    displayName: 'Lua',
    extensions: ['lua'],
    command: ['lua-language-server'],
    installInstructions: 'Install lua-language-server',
    description: 'Language server for Lua programming language',
    installRequired: true,
  },
  {
    name: 'vue',
    displayName: 'Vue.js',
    extensions: ['vue'],
    command: ['npx', '--', '@vue/language-server', '--stdio'],
    installInstructions: 'npm install -g @vue/language-server',
    installCommand: ['npm', 'install', '-g', '@vue/language-server'],
    description: 'Official Vue.js language server (formerly Volar)',
    installRequired: false,
  },
  {
    name: 'svelte',
    displayName: 'Svelte',
    extensions: ['svelte'],
    command: ['npx', '--', 'svelte-language-server', '--stdio'],
    installInstructions: 'npm install -g svelte-language-server',
    installCommand: ['npm', 'install', '-g', 'svelte-language-server'],
    description: 'Language server for Svelte framework',
    installRequired: false,
  },
];

export function generateConfig(selectedLanguages: string[]): CLIConfig {
  const selectedServers = LANGUAGE_SERVERS.filter((server) =>
    selectedLanguages.includes(server.name)
  );

  return {
    servers: selectedServers.map((server) => {
      // For commands that might be in non-standard locations, update the command path
      const command = [...server.command];
      if (command[0] === 'gopls' || command[0] === 'rust-analyzer') {
        command[0] = getCommandPath(command[0]);
      }

      const config: {
        extensions: string[];
        command: string[];
        rootDir: string;
        restartInterval?: number;
        initializationOptions?: unknown;
      } = {
        extensions: server.extensions,
        command,
        rootDir: server.rootDir || '.',
      };

      // Add restartInterval if specified for the server
      if (server.restartInterval) {
        config.restartInterval = server.restartInterval;
      }

      // Add initializationOptions if specified for the server
      if (server.initializationOptions) {
        config.initializationOptions = server.initializationOptions;
      }

      return config;
    }),
  };
}

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/ktnyt-cclsp-badge.png)](https://mseep.ai/app/ktnyt-cclsp)

# cclsp - not your average LSP adapter

[![npm version](https://badge.fury.io/js/cclsp.svg)](https://www.npmjs.com/package/cclsp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Node.js Version](https://img.shields.io/node/v/cclsp.svg)](https://nodejs.org)
[![CI](https://github.com/ktnyt/cclsp/actions/workflows/ci.yml/badge.svg)](https://github.com/ktnyt/cclsp/actions/workflows/ci.yml)
[![npm downloads](https://img.shields.io/npm/dm/cclsp.svg)](https://www.npmjs.com/package/cclsp)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

**cclsp** is a Model Context Protocol (MCP) server that seamlessly integrates LLM-based coding agents with Language Server Protocol (LSP) servers. LLM-based coding agents often struggle with providing accurate line/column numbers, which makes naive attempts to integrate with LSP servers fragile and frustrating. cclsp solves this by intelligently trying multiple position combinations and providing robust symbol resolution that just works, no matter how your AI assistant counts lines.

## Setup & Usage Demo

https://github.com/user-attachments/assets/52980f32-64d6-4b78-9cbf-18d6ae120cdd

## Table of Contents

- [Why cclsp?](#why-cclsp)
- [Features](#features)
- [📋 Prerequisites](#-prerequisites)
- [⚡ Setup](#-setup)
  - [Automated Setup (Recommended)](#automated-setup-recommended)
  - [Claude Code Quick Setup](#claude-code-quick-setup)
  - [Manual Setup](#manual-setup)
  - [Language Server Installation](#language-server-installation)
  - [Verification](#verification)
- [🚀 Usage](#-usage)
  - [As MCP Server](#as-mcp-server)
  - [Configuration](#configuration)
- [🛠️ Development](#️-development)
- [🔧 MCP Tools](#-mcp-tools)
  - [`find_definition`](#find_definition)
  - [`find_references`](#find_references)
  - [`rename_symbol`](#rename_symbol)
  - [`rename_symbol_strict`](#rename_symbol_strict)
  - [`get_diagnostics`](#get_diagnostics)
  - [`restart_server`](#restart_server)
- [💡 Real-world Examples](#-real-world-examples)
  - [Finding Function Definitions](#finding-function-definitions)
  - [Finding All References](#finding-all-references)
  - [Renaming Symbols](#renaming-symbols)
- [🔍 Troubleshooting](#-troubleshooting)
- [🤝 Contributing](#-contributing)
- [📄 License](#-license)

## Why cclsp?

When using AI-powered coding assistants like Claude, you often need to navigate codebases to understand symbol relationships. **cclsp** bridges the gap between Language Server Protocol capabilities and Model Context Protocol, enabling:

- 🔍 **Instant symbol navigation** - Jump to definitions without manually searching
- 📚 **Complete reference finding** - Find all usages of functions, variables, and types
- ✏️ **Safe symbol renaming** - Rename across entire codebases with confidence
- 🌍 **Universal language support** - Works with any LSP-compatible language server
- 🤖 **AI-friendly interface** - Designed for LLMs to understand and use effectively

## Features

### Core Navigation & Code Understanding
- **Go to Definition**: Find where symbols are defined
- **Find References**: Locate all references to a symbol  
- **Hover Information**: Get documentation, types, and signatures
- **Code Completions**: Context-aware code suggestions
- **Semantic Analysis**: Enhanced syntax and meaning understanding
- **Symbol Search**: Find symbols across the entire workspace

### Code Modification & Refactoring
- **Safe Symbol Renaming**: Rename across entire codebases with confidence
- **File Renaming**: Rename files and update all import references
- **Code Actions**: Get and apply suggested fixes and refactors
- **Document Formatting**: Standardize code style automatically

### Code Structure Analysis
- **Call Hierarchy**: Understand function call relationships (incoming/outgoing)
- **Type Hierarchy**: Navigate class/interface inheritance (supertypes/subtypes)
- **Document Symbols**: Get complete file symbol outline
- **Selection Ranges**: Smart hierarchical code block selection
- **Inlay Hints**: Parameter names and type annotations in code

### Code Quality & Diagnostics
- **Real-time Diagnostics**: Get errors, warnings, and hints
- **Language Server Management**: Restart and manage LSP servers

### Multi-language Support
- **TypeScript/JavaScript**: Full support via typescript-language-server
- **Python**: Complete support via python-lsp-server (pylsp)
- **Go**: Full support via gopls
- **Rust**: Support via rust-analyzer
- **And many more**: 15+ pre-configured language server setups

## 📋 Prerequisites

- Node.js 18+ or Bun runtime
- Language servers for your target languages (installed separately)

## ⚡ Setup

cclsp provides an interactive setup wizard that automates the entire configuration process. Choose your preferred method:

### Automated Setup (Recommended)

Run the interactive setup wizard:

```bash
# One-time setup (no installation required)
npx cclsp@latest setup

# For user-wide configuration
npx cclsp@latest setup --user
```

The setup wizard will:

1. **🔍 Auto-detect languages** in your project by scanning files
2. **📋 Show pre-selected LSP servers** based on detected languages
3. **📦 Display installation requirements** with detailed guides
4. **⚡ Install LSPs automatically** (optional, with user confirmation)
5. **🔗 Add to Claude MCP** (optional, with user confirmation)
6. **✅ Verify setup** and show available tools

#### Setup Options

- **Project Configuration** (default): Creates `.claude/cclsp.json` in current directory
- **User Configuration** (`--user`): Creates global config in `~/.config/claude/cclsp.json`

### Manual Setup

If you prefer manual configuration:

1. **Install cclsp**:

   ```bash
   npm install -g cclsp
   ```

2. **Install language servers** (see [Language Server Installation](#language-server-installation))

3. **Create configuration file**:

   ```bash
   # Use the interactive generator
   cclsp setup

   # Or create manually (see Configuration section)
   ```

4. **Add to Claude MCP**:
   ```bash
   claude mcp add cclsp npx cclsp@latest --env CCLSP_CONFIG_PATH=/path/to/cclsp.json
   ```

### Language Server Installation

The setup wizard shows installation commands for each LSP, but you can also install them manually:

<details>
<summary>📦 Common Language Servers</summary>

#### TypeScript/JavaScript

```bash
npm install -g typescript-language-server typescript
```

#### Python

```bash
pip install "python-lsp-server[all]"
# Or basic installation: pip install python-lsp-server
```

#### Go

```bash
go install golang.org/x/tools/gopls@latest
```

#### Rust

```bash
rustup component add rust-analyzer
rustup component add rust-src
```

#### C/C++

```bash
# Ubuntu/Debian
sudo apt install clangd

# macOS
brew install llvm

# Windows: Download from LLVM releases
```

#### Ruby

```bash
gem install solargraph
```

#### PHP

```bash
npm install -g intelephense
```

For more languages and detailed instructions, run `npx cclsp@latest setup` and select "Show detailed installation guides".

</details>

## 🚀 Usage

### As MCP Server

Configure in your MCP client (e.g., Claude Code):

#### Using npm package (after global install)

```json
{
  "mcpServers": {
    "cclsp": {
      "command": "cclsp",
      "env": {
        "CCLSP_CONFIG_PATH": "/path/to/your/cclsp.json"
      }
    }
  }
}
```

#### Using local installation

```json
{
  "mcpServers": {
    "cclsp": {
      "command": "node",
      "args": ["/path/to/cclsp/dist/index.js"],
      "env": {
        "CCLSP_CONFIG_PATH": "/path/to/your/cclsp.json"
      }
    }
  }
}
```

### Configuration

#### Interactive Configuration Generator

For easy setup, use the interactive configuration generator:

```bash
# Using npx (recommended for one-time setup)
npx cclsp@latest setup

# If installed globally
cclsp setup

# Or run directly with the development version
bun run setup
```

The interactive tool will:

- Show you all available language servers
- Let you select which ones to configure with intuitive controls:
  - **Navigation**: ↑/↓ arrow keys or Ctrl+P/Ctrl+N (Emacs-style)
  - **Selection**: Space to toggle, A to toggle all, I to invert selection
  - **Confirm**: Enter to proceed
- Display installation instructions for your selected languages
- Generate the configuration file automatically
- Show you the final configuration

#### Manual Configuration

Alternatively, create an `cclsp.json` configuration file manually:

```json
{
  "servers": [
    {
      "extensions": ["py", "pyi"],
      "command": ["uvx", "--from", "python-lsp-server", "pylsp"],
      "rootDir": ".",
      "initializationOptions": {
        "settings": {
          "pylsp": {
            "plugins": {
              "jedi_completion": { "enabled": true },
              "jedi_definition": { "enabled": true },
              "jedi_hover": { "enabled": true },
              "jedi_references": { "enabled": true },
              "jedi_signature_help": { "enabled": true },
              "jedi_symbols": { "enabled": true },
              "pylint": { "enabled": false },
              "pycodestyle": { "enabled": false },
              "pyflakes": { "enabled": false }
            }
          }
        }
      }
    },
    {
      "extensions": ["js", "ts", "jsx", "tsx"],
      "command": ["npx", "--", "typescript-language-server", "--stdio"],
      "rootDir": "."
    }
  ]
}
```

**Configuration Options:**

- `extensions`: Array of file extensions this server handles
- `command`: Command array to spawn the LSP server
- `rootDir`: Working directory for the LSP server (optional, defaults to ".")
- `restartInterval`: Auto-restart interval in minutes (optional)
- `initializationOptions`: LSP server initialization options (optional)

The `initializationOptions` field allows you to customize how each LSP server initializes. This is particularly useful for servers like `pylsp` (Python) that have extensive plugin configurations, or servers like `devsense-php-ls` that require specific settings.

<details>
<summary>📋 More Language Server Examples</summary>

```json
{
  "servers": [
    {
      "extensions": ["go"],
      "command": ["gopls"],
      "rootDir": "."
    },
    {
      "extensions": ["rs"],
      "command": ["rust-analyzer"],
      "rootDir": "."
    },
    {
      "extensions": ["c", "cpp", "cc", "h", "hpp"],
      "command": ["clangd"],
      "rootDir": "."
    },
    {
      "extensions": ["java"],
      "command": ["jdtls"],
      "rootDir": "."
    },
    {
      "extensions": ["rb"],
      "command": ["solargraph", "stdio"],
      "rootDir": "."
    },
    {
      "extensions": ["php"],
      "command": ["intelephense", "--stdio"],
      "rootDir": "."
    },
    {
      "extensions": ["cs"],
      "command": ["omnisharp", "-lsp"],
      "rootDir": "."
    },
    {
      "extensions": ["swift"],
      "command": ["sourcekit-lsp"],
      "rootDir": "."
    }
  ]
}
```

</details>

## 🛠️ Development

```bash
# Run in development mode
bun run dev

# Run tests
bun test

# Run manual integration test
bun run test:manual

# Lint code
bun run lint

# Format code
bun run format

# Type check
bun run typecheck
```

## 🔧 MCP Tools

The server exposes these MCP tools:

### Core Navigation & Analysis

#### `find_definition`

Find the definition of a symbol by name and kind in a file. Returns definitions for all matching symbols.

**Parameters:**

- `file_path`: The path to the file
- `symbol_name`: The name of the symbol
- `symbol_kind`: The kind of symbol (function, class, variable, method, etc.) (optional)

#### `find_references`

Find all references to a symbol by name and kind in a file. Returns references for all matching symbols.

**Parameters:**

- `file_path`: The path to the file
- `symbol_name`: The name of the symbol
- `symbol_kind`: The kind of symbol (function, class, variable, method, etc.) (optional)
- `include_declaration`: Whether to include the declaration (optional, default: true)

### Code Modification

#### `rename_symbol`

Rename a symbol by name and kind in a file. **This tool now applies the rename to all affected files by default.** If multiple symbols match, returns candidate positions and suggests using rename_symbol_strict.

**Parameters:**

- `file_path`: The path to the file
- `symbol_name`: The name of the symbol
- `symbol_kind`: The kind of symbol (function, class, variable, method, etc.) (optional)
- `new_name`: The new name for the symbol
- `dry_run`: If true, only preview the changes without applying them (optional, default: false)

**Note:** When `dry_run` is false (default), the tool will:
- Apply the rename to all affected files
- Create backup files with `.bak` extension
- Return the list of modified files

#### `rename_symbol_strict`

Rename a symbol at a specific position in a file. Use this when rename_symbol returns multiple candidates. **This tool now applies the rename to all affected files by default.**

**Parameters:**

- `file_path`: The path to the file
- `line`: The line number (1-indexed)
- `character`: The character position in the line (1-indexed)
- `new_name`: The new name for the symbol
- `dry_run`: If true, only preview the changes without applying them (optional, default: false)

#### `format_document`

Format a document according to the language server's formatting rules.

**Parameters:**
- `file_path`: The path to the file to format

#### `get_code_actions`

Get available code actions (fixes, refactors) for a specific range in a file.

**Parameters:**
- `file_path`: The path to the file
- `start_line`: Start line number (1-indexed)
- `start_character`: Start character position (0-indexed)  
- `end_line`: End line number (1-indexed)
- `end_character`: End character position (0-indexed)

#### `apply_workspace_edit`

Apply workspace edits (file changes) across multiple files.

**Parameters:**
- `changes`: Record mapping file paths to arrays of text edits
- `validate_before_apply`: Whether to validate changes before applying (optional, default: false)

#### `create_file`

Create a new file with specified content.

**Parameters:**
- `file_path`: Path where the new file should be created
- `content`: Content for the new file

#### `delete_file`

Delete a file from the filesystem.

**Parameters:**
- `file_path`: Path to the file to delete
- `force`: Whether to force deletion (optional, default: false)

### Code Intelligence

#### `get_hover`

Get hover information (documentation, types, signatures) for a symbol at a specific position.

**Parameters:**
- `file_path`: The path to the file
- `line`: The line number (1-indexed)
- `character`: The character position in the line (0-indexed)

#### `get_completions`

Get code completion suggestions at a specific position in a file.

**Parameters:**
- `file_path`: The path to the file
- `line`: The line number (1-indexed) 
- `character`: The character position in the line (0-indexed)

#### `get_inlay_hints`

Get inlay hints (parameter names, type annotations) for a range in a file.

**Parameters:**
- `file_path`: The path to the file
- `start_line`: Start line number (1-indexed)
- `start_character`: Start character position (0-indexed)
- `end_line`: End line number (1-indexed)  
- `end_character`: End character position (0-indexed)

#### `get_semantic_tokens`

Get semantic tokens (detailed syntax analysis) for enhanced code understanding.

**Parameters:**
- `file_path`: The path to the file

#### `get_signature_help`

Get function signature help at a specific position in the code.

**Parameters:**
- `file_path`: The path to the file
- `line`: The line number (1-indexed)
- `character`: The character position in the line (0-indexed)

### Code Structure Analysis

#### `prepare_call_hierarchy`

Prepare call hierarchy items for a symbol at a specific position.

**Parameters:**
- `file_path`: The path to the file
- `line`: The line number (1-indexed)
- `character`: The character position in the line (0-indexed)

#### `get_call_hierarchy_incoming_calls`

Get incoming calls for a call hierarchy item.

**Parameters:**
- `item`: Call hierarchy item from prepare_call_hierarchy

#### `get_call_hierarchy_outgoing_calls`

Get outgoing calls for a call hierarchy item.

**Parameters:**
- `item`: Call hierarchy item from prepare_call_hierarchy

#### `prepare_type_hierarchy`

Prepare type hierarchy items for a symbol at a specific position.

**Parameters:**
- `file_path`: The path to the file
- `line`: The line number (1-indexed)
- `character`: The character position in the line (0-indexed)

#### `get_type_hierarchy_supertypes`

Get supertypes (parent classes/interfaces) for a type hierarchy item.

**Parameters:**
- `item`: Type hierarchy item from prepare_type_hierarchy

#### `get_type_hierarchy_subtypes`

Get subtypes (child implementations) for a type hierarchy item.

**Parameters:**
- `item`: Type hierarchy item from prepare_type_hierarchy

#### `get_selection_range`

Get hierarchical selection ranges for smart code block selection.

**Parameters:**
- `file_path`: The path to the file
- `positions`: Array of positions with line (1-indexed) and character (0-indexed) properties

### Workspace Operations

#### `get_document_symbols`

Get all symbols in a document for code outline and navigation.

**Parameters:**
- `file_path`: The path to the file

#### `get_folding_ranges`

Get folding ranges for code collapse/expand functionality.

**Parameters:**
- `file_path`: The path to the file

#### `get_document_links`

Get document links (URLs, imports, references) found in the file.

**Parameters:**
- `file_path`: The path to the file

#### `search_workspace_symbols`

Search for symbols across the entire workspace.

**Parameters:**
- `query`: Search query string

#### `rename_file`

Rename a file and update all import statements that reference it.

**Parameters:**
- `old_path`: Current file path
- `new_path`: New file path
- `dry_run`: If true, only preview changes without applying them (optional, default: false)

### System Operations

#### `get_diagnostics`

Get language diagnostics (errors, warnings, hints) for a file. Uses LSP textDocument/diagnostic to pull current diagnostics.

**Parameters:**
- `file_path`: The path to the file to get diagnostics for

#### `restart_server`

Manually restart LSP servers. Can restart servers for specific file extensions or all running servers.

**Parameters:**
- `extensions`: Array of file extensions to restart servers for (e.g., ["ts", "tsx"]). If not provided, all servers will be restarted (optional)

## 💡 Real-world Examples

### Finding Function Definitions

When Claude needs to understand how a function works:

```
Claude: Let me find the definition of the `processRequest` function
> Using cclsp.find_definition with symbol_name="processRequest", symbol_kind="function"

Result: Found definition at src/handlers/request.ts:127:1
```

### Finding All References

When refactoring or understanding code impact:

```
Claude: I'll find all places where `CONFIG_PATH` is used
> Using cclsp.find_references with symbol_name="CONFIG_PATH"

Results: Found 5 references:
- src/config.ts:10:1 (declaration)
- src/index.ts:45:15
- src/utils/loader.ts:23:8
- tests/config.test.ts:15:10
- tests/config.test.ts:89:12
```

### Renaming Symbols

Safe refactoring across the entire codebase (now with actual file modification!):

```
Claude: I'll rename `getUserData` to `fetchUserProfile`
> Using cclsp.rename_symbol with symbol_name="getUserData", new_name="fetchUserProfile"

Result: Successfully renamed getUserData (function) to "fetchUserProfile".

Modified files:
- src/api/user.ts
- src/services/auth.ts
- src/components/UserProfile.tsx
... (12 files total)
```

Preview changes before applying (using dry_run):

```
Claude: Let me first preview what will be renamed
> Using cclsp.rename_symbol with symbol_name="getUserData", new_name="fetchUserProfile", dry_run=true

Result: [DRY RUN] Would rename getUserData (function) to "fetchUserProfile":
File: src/api/user.ts
  - Line 55, Column 10 to Line 55, Column 21: "fetchUserProfile"
File: src/services/auth.ts
  - Line 123, Column 15 to Line 123, Column 26: "fetchUserProfile"
... (12 files total)
```

When multiple symbols match:

```
Claude: I'll rename the `data` variable to `userData`
> Using cclsp.rename_symbol with symbol_name="data", new_name="userData"

Result: Multiple symbols found matching "data". Please use rename_symbol_strict with one of these positions:
- data (variable) at line 45, character 10
- data (parameter) at line 89, character 25
- data (property) at line 112, character 5

> Using cclsp.rename_symbol_strict with line=45, character=10, new_name="userData"

Result: Successfully renamed symbol at line 45, character 10 to "userData".

Modified files:
- src/utils/parser.ts
```

### Checking File Diagnostics

When analyzing code quality:

```
Claude: Let me check for any errors or warnings in this file
> Using cclsp.get_diagnostics

Results: Found 3 diagnostics:
- Error [TS2304]: Cannot find name 'undefinedVar' (Line 10, Column 5)
- Warning [no-unused-vars]: 'config' is defined but never used (Line 25, Column 10)
- Hint: Consider using const instead of let (Line 30, Column 1)
```

### Restarting LSP Servers

When LSP servers become unresponsive or configuration changes:

```
Claude: The TypeScript server seems unresponsive, let me restart it
> Using cclsp.restart_server with extensions ["ts", "tsx"]

Result: Successfully restarted 1 LSP server(s)
Restarted servers:
• typescript-language-server --stdio (ts, tsx)
```

Or restart all servers:

```
Claude: I'll restart all LSP servers to ensure they're working properly
> Using cclsp.restart_server

Result: Successfully restarted 2 LSP server(s)
Restarted servers:
• typescript-language-server --stdio (ts, tsx)
• pylsp (py)
```

## 🔍 Troubleshooting

### Known Issues

<details>
<summary>🐍 Python LSP Server (pylsp) Performance Degradation</summary>

**Problem**: The Python Language Server (pylsp) may become slow or unresponsive after extended use (several hours), affecting symbol resolution and code navigation.

**Symptoms**:
- Slow or missing "go to definition" results for Python files
- Delayed or incomplete symbol references
- General responsiveness issues with Python code analysis

**Solution**: Use the auto-restart feature to periodically restart the pylsp server:

Add `restartInterval` to your Python server configuration:

```json
{
  "servers": [
    {
      "extensions": ["py", "pyi"],
      "command": ["pylsp"],
      "restartInterval": 5
    }
  ]
}
```

This will automatically restart the Python LSP server every 5 minutes, maintaining optimal performance for long coding sessions.

**Alternative**: You can also manually restart servers using the `restart_server` tool when needed:
- Restart specific server: `restart_server` with `extensions: ["py"]`
- Restart all servers: `restart_server` without parameters

**Note**: The setup wizard automatically configures this for Python servers when detected.

</details>

### Common Issues

<details>
<summary>🔧 LSP server not starting</summary>

**Problem**: Error message about LSP server not found

**Solution**: Ensure the language server is installed:

```bash
# For TypeScript
npm install -g typescript-language-server

# For Python
pip install python-lsp-server

# For Go
go install golang.org/x/tools/gopls@latest
```

</details>

<details>
<summary>🔧 Configuration not loading</summary>

**Problem**: cclsp uses default TypeScript configuration only

**Solution**: Check that:

1. Your config file is named `cclsp.json` (not `cclsp.config.json`)
2. The `CCLSP_CONFIG_PATH` environment variable points to the correct file
3. The JSON syntax is valid
</details>

<details>
<summary>🔧 Symbol not found errors</summary>

**Problem**: "Go to definition" returns no results

**Solution**:

1. Ensure the file is saved and part of the project
2. Check that the language server supports the file type
3. Some language servers need a few seconds to index the project
</details>

## 🤝 Contributing

We welcome contributions! Here's how you can help:

### Reporting Issues

Found a bug or have a feature request? [Open an issue](https://github.com/ktnyt/cclsp/issues) with:

- Clear description of the problem
- Steps to reproduce
- Expected vs actual behavior
- Your environment (OS, Node version, etc.)

### Adding Language Support

Want to add support for a new language?

1. Find the LSP server for your language
2. Test the configuration locally
3. Submit a PR with:
   - Updated README examples
   - Test files if possible
   - Configuration documentation

### Code Contributions

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Run tests: `bun test`
5. Commit: `git commit -m '✨ feat: add amazing feature'`
6. Push: `git push origin feature/amazing-feature`
7. Open a Pull Request

## 📄 License

MIT

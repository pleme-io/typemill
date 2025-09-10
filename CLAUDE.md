# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

codebuddy is an MCP (Model Context Protocol) server that bridges Language Server Protocol (LSP) functionality to MCP tools. It allows MCP clients to access LSP features like "go to definition" and "find references" through a standardized interface.

## Development Commands

```bash
# Install dependencies
bun install

# Development with hot reload
bun run dev

# Build for production
bun run build

# Run the built server
bun run start
# or directly
node dist/index.js

# CLI commands for configuration and management
codebuddy init     # Smart setup with auto-detection  
codebuddy status   # Show what's working right now
codebuddy fix      # Actually fix problems (auto-install when possible)
codebuddy config   # Show/edit configuration
codebuddy logs     # Debug output when things go wrong

# Quality assurance
bun run lint         # Check code style and issues
bun run lint:fix     # Auto-fix safe issues
bun run format       # Format code with Biome
bun run typecheck    # Run TypeScript type checking
bun run test         # Run unit tests
bun run test:mcp     # Run MCP integration tests

# Test Performance Optimizations (for slow systems)
bun run test:fast     # Optimized test runner with system detection
bun run test:minimal  # Ultra-minimal runner for very slow systems
# Fast runner: 5min timeout, parallel on fast systems, LSP preload optional
# Minimal runner: 10min timeout, sequential only, no LSP preload, minimal config

# Full pre-publish check
bun run prepublishOnly  # build + test + typecheck
```

## Architecture

### Core Components

**MCP Server Layer** (`index.ts`)

- Entry point that implements MCP protocol
- Exposes 38 MCP tools covering navigation, refactoring, intelligence, and diagnostics
- Handles MCP client requests and delegates to LSP layer
- Includes CLI subcommand handling for `init`, `status`, `fix`, `config`, `logs`

**LSP Client Layer** (`src/lsp/client.ts`)

- Manages multiple LSP server processes concurrently
- Handles LSP protocol communication (JSON-RPC over stdio)
- Maps file extensions to appropriate language servers
- Maintains process lifecycle and request/response correlation

**Configuration System** (`.codebuddy/config.json`)

- Defines which LSP servers to use for different file extensions  
- Smart setup with auto-detection via `codebuddy init` command
- File scanning with gitignore support for project structure detection
- Automatic migration from old `codebuddy.json` format

### Data Flow

1. MCP client sends tool request (e.g., `find_definition`)
2. Main server resolves file path and extracts position
3. LSP client determines appropriate language server for file extension
4. If server not running, spawns new LSP server process
5. Sends LSP request to server and correlates response
6. Transforms LSP response back to MCP format

### LSP Server Management

The system spawns separate LSP server processes per configuration. Each server:

- Runs as child process with stdio communication
- Maintains its own initialization state
- Handles multiple concurrent requests
- Gets terminated on process exit

Supported language servers (configurable):

- TypeScript: `typescript-language-server`
- Python: `pylsp`
- Go: `gopls`

## Configuration

The server loads configuration from `.codebuddy/config.json` in the current working directory. If no configuration exists, run `codebuddy init` to create one.

### Smart Setup  

Use `codebuddy init` to configure LSP servers with auto-detection:

- Scans project for file extensions (respects .gitignore)
- Presents pre-configured language server options for detected languages
- Generates `.codebuddy/config.json` configuration file  
- Tests server availability during setup

Each server config requires:

- `extensions`: File extensions to handle (array)
- `command`: Command array to spawn LSP server
- `rootDir`: Working directory for LSP server (optional)
- `restartInterval`: Auto-restart interval in minutes (optional, helps with long-running server stability, minimum 1 minute)

### Example Configuration

```json
{
  "servers": [
    {
      "extensions": ["py"],
      "command": ["pylsp"],
      "restartInterval": 5
    },
    {
      "extensions": ["ts", "tsx", "js", "jsx"],
      "command": ["typescript-language-server", "--stdio"],
      "restartInterval": 10
    }
  ]
}
```

## Code Quality & Testing

The project uses Biome for linting and formatting:

- **Linting**: Enabled with recommended rules + custom strictness
- **Formatting**: 2-space indents, single quotes, semicolons always, LF endings
- **TypeScript**: Strict type checking with `--noEmit`
- **Testing**: Bun test framework with unit tests in `src/*.test.ts`

Run quality checks before committing:

```bash
bun run lint:fix && bun run format && bun run typecheck && bun run test
```

## LSP Protocol Details

The implementation handles LSP protocol specifics:

- Content-Length headers for message framing
- JSON-RPC 2.0 message format
- Request/response correlation via ID tracking
- Server initialization handshake
- Proper process cleanup on shutdown
- Preloading of servers for detected file types
- Automatic server restart based on configured intervals
- Manual server restart via MCP tool

## Dead Code Detection

Run dead code detection with:
- `bun run dead-code` - Check for dead code
- `bun run dead-code:fix` - Auto-fix where possible
- `bun run dead-code:ci` - CI-friendly output

Tool: Knip (detects unused files, dependencies, exports)
Config: knip.json

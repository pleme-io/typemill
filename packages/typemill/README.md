# TypeMill

Pure Rust MCP server bridging Language Server Protocol (LSP) functionality to AI coding assistants.

## Quick Start

```bash
# Run without installing
npx @goobits/typemill start

# Or install globally
npm install -g @goobits/typemill
typemill start
```

## What is TypeMill?

TypeMill provides AI coding assistants (like Claude) with powerful code intelligence tools:

- **Code Navigation**: Go to definition, find references, hover info
- **Refactoring**: Rename symbols, move files, extract functions
- **Workspace Operations**: Find/replace, dependency analysis

## Commands

```bash
# Start the MCP server
typemill start

# Auto-detect languages and install LSP servers
typemill setup

# Check server status
typemill status

# List available tools
typemill tools
```

## MCP Configuration

Add TypeMill to your Claude Desktop config (`~/.config/claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "typemill": {
      "command": "npx",
      "args": ["@goobits/typemill", "start"]
    }
  }
}
```

## Supported Languages

- **TypeScript/JavaScript** - via typescript-language-server
- **Rust** - via rust-analyzer
- **Python** - via pylsp

## Tools Available

| Tool | Description |
|------|-------------|
| `inspect_code` | Get definition, references, type info at a position |
| `search_code` | Search workspace symbols |
| `rename_all` | Rename symbols, files, or directories |
| `relocate` | Move symbols, files, or directories |
| `prune` | Delete with cleanup |
| `refactor` | Extract, inline, reorder, transform code |
| `workspace` | Find/replace, dependency extraction |

## Documentation

Full documentation at [github.com/goobits/typemill](https://github.com/goobits/typemill)

## License

MIT

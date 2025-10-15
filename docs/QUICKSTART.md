# Quickstart

Get Codebuddy running in 2 minutes.

## Prerequisites

- Language server for your project (e.g., `typescript-language-server`, `rust-analyzer`)
- AI assistant with MCP support (Claude Desktop, etc.)

## Setup Steps

### 1. Install Codebuddy

**Option A: Install script (recommended)**
```bash
curl -fsSL https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh | bash
```

**Option B: Build from source**
```bash
cargo install codebuddy --locked
```

### 2. Configure Your Project

Auto-detects languages and creates `.codebuddy/config.json`:
```bash
codebuddy setup
```

Manual config: see [examples/setup/mcp-config.json](../examples/setup/mcp-config.json)

### 3. Start the Server

```bash
codebuddy start
```

### 4. Connect Your AI Assistant

Add to your MCP configuration:
```json
{
  "mcpServers": {
    "codebuddy": {
      "command": "codebuddy",
      "args": ["start"]
    }
  }
}
```

Full example: [examples/setup/mcp-config.json](../examples/setup/mcp-config.json)

### 5. Verify

```bash
codebuddy status
```

## First Tool Call

Ask your AI assistant:
- "Find the definition of `main` in src/main.rs"
- "Show me all references to the `Config` type"
- "Rename the function `oldName` to `newName`"

## Next Steps

- **[TOOLS_CATALOG.md](TOOLS_CATALOG.md)** - Complete list of 23 MCP tools
- **[API_REFERENCE.md](API_REFERENCE.md)** - Detailed API with parameters and returns
- **[OPERATIONS.md](OPERATIONS.md)** - Advanced configuration and analysis options

## Troubleshooting

**Server won't start:**
- Check `codebuddy status` for LSP server availability
- Verify language servers are installed and in PATH
- Check `.codebuddy/config.json` for correct command paths

**Tools not working:**
- Ensure file extensions match config (e.g., `.rs` → `rust-analyzer`)
- Check MCP connection with AI assistant
- Review server logs for errors

**Performance issues:**
- Enable cache (disabled by default for development)
- Adjust `restartInterval` in config (recommended: 10-30 minutes)
- Check system resources (LSP servers can be memory-intensive)

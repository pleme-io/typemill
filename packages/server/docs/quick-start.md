# Quick Start Guide

Get CodeFlow Buddy running with Claude Code in 2 minutes.

## Installation (30 seconds)

```bash
npm install -g @goobits/codeflow-buddy
```

## Setup (30 seconds)

Run the interactive setup to auto-detect your languages:

```bash
codeflow-buddy setup
```

This scans your project and configures the right language servers automatically.

## Add to Claude Code (1 minute)

Add to your Claude Code MCP settings:

```json
{
  "mcpServers": {
    "codeflow-buddy": {
      "command": "codeflow-buddy",
      "args": ["start"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

## Verify It's Working

```bash
codeflow-buddy status
```

You should see:
```
✅ TypeScript Language Server - Running
✅ Python Language Server - Running
✅ MCP Server - Ready for connections
```

## Try These Commands

Ask Claude to:
- "Find the definition of handleLogin function"
- "Find all references to UserService class"
- "Rename validateEmail to checkEmail across the codebase"
- "Show me the call hierarchy for processPayment"

## Common Issues

**Language server not found?**
```bash
# TypeScript
npm install -g typescript-language-server typescript

# Python
pip install python-lsp-server

# Go
go install golang.org/x/tools/gopls@latest
```

**Permission errors?**
```bash
# Fix npm permissions
sudo npm install -g @goobits/codeflow-buddy
```

## Next Steps

- [View all 25 MCP tools](api.md)
- [Configure additional languages](languages.md)
- [WebSocket deployment](../README.md#websocket-server-optional)
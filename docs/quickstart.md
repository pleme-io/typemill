# Quick Start Guide

**Get Mill running in 5 minutes with your TypeScript project**

---

## Prerequisites

- Git repository with TypeScript files (`.ts`, `.tsx`, `.js`, `.jsx`)
- Rust installed (for building from source) OR download pre-built binary
- Terminal access

---

## Installation

### Option 1: Install Script (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/goobits/mill/main/install.sh | bash
```

This installs Mill to `~/.local/bin/mill` and adds it to your PATH.

### Option 2: Build from Source

```bash
git clone https://github.com/goobits/typemill
cd typemill
cargo build --release
sudo cp target/release/mill /usr/local/bin/
```

### Option 3: Cargo Install

```bash
cargo install mill --locked
```

---

## Setup (2 minutes)

Navigate to your TypeScript project and run setup:

```bash
cd /path/to/your/typescript/project
mill setup
```

**What happens:**
1. 🔍 Mill scans your project and detects TypeScript files
2. 📋 Creates `.typemill/config.json` with LSP server configuration
3. 📥 Prompts to auto-download `typescript-language-server` (if not found)
4. ✅ Verifies the LSP server works
5. 💾 Caches LSP binary in `~/.mill/lsp/` for reuse

**Example output:**
```
🔍 Scanning project...
✅ Detected: TypeScript project (45 .ts files, 12 .tsx files)
📦 Required LSP: typescript-language-server

❓ Install typescript-language-server? [Y/n] y
⬇️  Downloading typescript-language-server v4.3.0...
✅ Installed to ~/.mill/lsp/typescript-language-server
📝 Created config at .typemill/config.json

✨ Setup complete!
```

---

## Start the Server (30 seconds)

```bash
mill start
```

This starts Mill in MCP (Model Context Protocol) mode, ready to connect to Claude Desktop or other AI assistants.

**Check status:**
```bash
mill status
```

You should see:
```
✅ Mill is running (PID: 12345)
✅ LSP servers configured: typescript-language-server
📁 Config: .typemill/config.json
```

---

## Connect to Claude Desktop (1 minute)

Add Mill to your Claude Desktop MCP configuration:

**Location:**
- **macOS/Linux:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

**Configuration:**
```json
{
  "mcpServers": {
    "mill": {
      "command": "mill",
      "args": ["start"]
    }
  }
}
```

**Restart Claude Desktop** to load the configuration.

---

## First Commands (1 minute)

Ask Claude to use Mill:

### 1. **Find a Definition**
```
"Find the definition of the App component in src/App.tsx"
```

Mill will use `find_definition` tool and show you the exact location.

### 2. **Show All References**
```
"Show me everywhere the Config type is used"
```

Mill will use `find_references` tool and list all usages.

### 3. **Rename Safely**
```
"Rename the function fetchData to loadData and update all references"
```

Mill will:
1. Show a preview plan (dry run mode - default)
2. Ask for confirmation
3. Execute the rename and update all imports/references

---

## Common Next Steps

### Explore Available Tools
```bash
mill tools
```

Shows all 28 MCP tools organized by category.

### View Documentation
```bash
mill docs                    # List all topics
mill docs tools              # Tool reference
mill docs cheatsheet         # Command quick reference
```

### Use CLI Directly

You can also call tools directly without Claude:

```bash
# Find definition
mill tool find_definition '{"file_path": "src/App.tsx", "line": 10, "character": 5}'

# Rename file (dry run)
mill tool rename --target file:src/old.ts --new-name src/new.ts

# Rename file (execute)
mill tool rename --target file:src/old.ts --new-name src/new.ts --dry-run false
```

---

## Troubleshooting

### LSP Server Not Found

If setup couldn't find `typescript-language-server`, install it manually:

```bash
npm install -g typescript-language-server typescript
```

Then re-run setup:
```bash
mill setup
```

### Server Won't Start

Check for errors:
```bash
mill doctor
```

This runs diagnostics and shows what's wrong.

### Configuration Issues

View your configuration:
```bash
cat .typemill/config.json
```

Reset configuration:
```bash
rm -rf .typemill
mill setup
```

---

## What's Next?

- **[Cheat Sheet](cheatsheet.md)** - Quick command reference
- **[Tool Reference](tools/README.md)** - Complete API for all 28 tools
- **[Refactoring Guide](tools/refactoring.md)** - Deep dive into refactoring tools
- **[LSP Troubleshooting](guides/lsp-troubleshooting.md)** - Fix common issues

---

**🎉 You're ready to use Mill!** Ask Claude to analyze, refactor, or navigate your codebase.

**Need help?**
- Run `mill docs` for embedded documentation
- Check [GitHub Issues](https://github.com/goobits/typemill/issues)
- Search docs: `mill docs --search <keyword>`

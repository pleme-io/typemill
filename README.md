# 🤖 Codebuddy

**Give your AI coding assistant superpowers.**

Codebuddy bridges the gap between AI assistants and your codebase by exposing Language Server Protocol (LSP) functionality through the Model Context Protocol (MCP). Think of it as a universal translator that lets AI tools like Claude understand your code the same way your IDE does—with full awareness of definitions, references, types, and refactoring capabilities.

## 📋 Table of Contents
- [Key Features](#-key-features)
- [Quick Start](#-quick-start)
- [Language Server Setup](#️-language-server-setup)
- [MCP Integration](#-mcp-integration)
- [Configuration](#️-configuration)
- [CLI Commands](#-cli-commands)
- [Docker Deployment](#-docker-deployment)
- [Documentation](#-documentation)
- [Troubleshooting](#-troubleshooting)
- [Development](#-development)
- [License](#-license)
- [Support](#-support)

## ✨ What Can It Do?

**Navigation & Understanding**
- 🔍 Jump to definitions, find all references, search symbols across your entire workspace
- 💡 Get hover documentation, completions, and signature help—just like in your IDE
- 📊 Visualize call hierarchies and trace type definitions

**Intelligent Refactoring**
- 🔧 Rename symbols safely across files with automatic import updates
- ⚡ Extract functions and variables, inline code, organize imports
- 🎯 Apply code actions and fixes suggested by your language server

**Powerful Operations**
- 📦 Atomic multi-file edits that either succeed completely or roll back
- 🚀 Batch operations with parallel execution for maximum speed
- 🔄 Smart directory moves with automatic import path updates (including Rust crate consolidation)

**Multi-Language Support**
- 🌐 Works with TypeScript, Python, Go, Rust—any language with an LSP server
- 🔌 Flexible transport: stdio for direct MCP integration or WebSocket with JWT auth
- 🛡️ Built in Rust for memory safety and blazing performance

## 🚀 Quick Start

Ready to get started? The setup is straightforward—just install, configure your language servers, and you're ready to go.

```bash
# Install from source
git clone https://github.com/goobits/codebuddy.git
cd codebuddy
cargo build --release
sudo cp target/release/codebuddy /usr/local/bin/

# Run the interactive setup wizard
# It'll detect your project languages and help configure the right servers
codebuddy setup

# Start the MCP server (for Claude Code, Cursor, Aider, etc.)
codebuddy start

# Or start the WebSocket server (for custom integrations)
codebuddy serve
```

That's it! Your AI assistant now has deep code intelligence for your entire project.

## 🛠️ Language Server Setup

Codebuddy works with any LSP-compatible language server. Here's how to install the most common ones:

```bash
# TypeScript/JavaScript
npm install -g typescript-language-server typescript

# Python
pip install "python-lsp-server[all]"

# Go
go install golang.org/x/tools/gopls@latest

# Rust
rustup component add rust-analyzer

# Verify everything is working
codebuddy status
```

The `codebuddy setup` wizard will detect which languages you're using and guide you through configuration. No manual JSON editing required (unless you want to).

## 📚 MCP Integration

Connecting Codebuddy to your AI assistant is simple. Works with Claude Code, Cursor, Aider, and any MCP-compatible client. Add this to your MCP configuration:

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

Once connected, your AI assistant can navigate your code, suggest refactorings, and understand your project structure—all powered by the same language servers your IDE uses.

## ⚙️ Configuration

**Prefer the easy way?** Just run `codebuddy setup` and follow the prompts. It'll scan your project, detect languages, and create the config automatically.

**Like to tinker?** Configuration lives in `.codebuddy/config.json`. Here's an example:

```json
{
  "servers": [
    {
      "extensions": ["py", "pyi"],
      "command": ["pylsp"],
      "restartInterval": 30
    },
    {
      "extensions": ["js", "ts", "jsx", "tsx"],
      "command": ["npx", "--", "typescript-language-server", "--stdio"]
    }
  ]
}
```

Each server maps file extensions to an LSP command. The optional `restartInterval` helps with long-running server stability.

```bash
# View your current configuration
codebuddy status
```

## 🎯 CLI Commands
```bash
# Server lifecycle
codebuddy start          # Start stdio MCP server
codebuddy serve          # Start WebSocket server
codebuddy stop           # Stop server
codebuddy status         # Show status

# Configuration
codebuddy setup          # Setup wizard
codebuddy doctor         # Diagnose issues
codebuddy link           # Link AI assistants
codebuddy unlink         # Remove links

# Tools
codebuddy tool <name>    # Execute MCP tool
codebuddy tools          # List tools
```

## 🐳 Docker Deployment
```bash
# Development
cd deployment/docker
docker-compose up -d

# Production (with JWT auth)
export JWT_SECRET="your-secret-key"
docker-compose -f docker-compose.production.yml up -d

# Health check
curl http://localhost/health
```

See [`deployment/docker/README.md`](deployment/docker/README.md) for details.

## 📖 Documentation
- **[API.md](API.md)** - MCP tools reference
- **[CLAUDE.md](CLAUDE.md)** - AI assistant guide
- **[docs/architecture/ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md)** - System design
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development guide

## 🔧 Troubleshooting

Running into issues? Here's how to diagnose them:

```bash
# Check if language servers are installed and accessible
codebuddy status

# See detailed logs to understand what's happening
RUST_LOG=debug codebuddy start

# Run the full diagnostic suite
codebuddy doctor

# Reconfigure if something changed
codebuddy setup
```

**Common issues and fixes:**
- **LSP server not starting** → Make sure it's installed and in your PATH
- **Import updates failing** → The language server needs to support workspace edits (most do)
- **Can't find definitions** → File might be outside workspace root, or server needs initialization time

## 🔗 Related Projects
- **[Model Context Protocol](https://github.com/modelcontextprotocol/servers)** - MCP specification
- **[Language Server Protocol](https://langserver.org/)** - LSP specification

## 🧪 Development

Want to contribute or modify Codebuddy? We'd love to have you! The codebase is pure Rust with a focus on clarity and performance.

```bash
# Build and test
cargo build --release
cargo test
cargo clippy
cargo fmt

# Or use the Makefile for convenience
make setup       # Install dev tools (one-time)
make             # Build debug version
make test        # Run tests
make check       # Run fmt + clippy + test

# Run locally for testing
cargo run -- start
```

Check out [CONTRIBUTING.md](CONTRIBUTING.md) for architecture details, code standards, and how to add new MCP tools. We welcome PRs!

## 📝 License
MIT - see [LICENSE](LICENSE)

## 💡 Support

Have questions? Found a bug? Want to request a feature?

- **[Report issues](https://github.com/goobits/codebuddy/issues)** - Bug reports and feature requests
- **[Join discussions](https://github.com/goobits/codebuddy/discussions)** - Ask questions, share ideas, show what you've built

We're here to help make your AI coding experience better.

---

**Credits**: Inspired by [ktnyt/cclsp](https://github.com/ktnyt/cclsp). Codebuddy is a ground-up Rust rewrite with production architecture, batch operations, plugin system, and enterprise features.

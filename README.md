# 🤖 Codebuddy

**Give your AI coding assistant superpowers.**

Codebuddy bridges the gap between AI assistants and your codebase by exposing Language Server Protocol (LSP) functionality through the Model Context Protocol (MCP). Think of it as a universal translator that lets AI tools like Claude understand your code the same way your IDE does—with full awareness of definitions, references, types, and refactoring capabilities.

## 📋 Table of Contents
- [What Can It Do?](#-what-can-it-do)
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

Your AI assistant can finally understand your codebase the way your IDE does:

- **Navigate intelligently** - Jump to definitions, find references, search symbols across your workspace
- **Refactor safely** - Rename across files, extract functions, organize imports—with automatic updates
- **Scale confidently** - Atomic multi-file edits, batch operations, smart directory moves

Supports TypeScript, Python, Go, Rust, Java—any language with an LSP server. Built in Rust for memory safety and blazing performance.

## 🚀 Quick Start

### For End Users (Use the tool)

Install the pre-built binary:

```bash
curl -fsSL https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh | bash
```

Then configure and start:

```bash
codebuddy setup    # Configure LSP servers for your project
codebuddy start    # Start the MCP server
```

**That's it!** Your AI assistant now has deep code intelligence.

---

### For Developers (Build from source)

**One command does everything:**

```bash
git clone https://github.com/goobits/codebuddy.git
cd codebuddy
make first-time-setup  # Installs all tools, builds parsers, runs tests (~3-5 min)
```

**What gets installed:**
- cargo-nextest, sccache, cargo-watch, cargo-audit (via cargo-binstall)
- mold linker (if sudo available)
- LSP servers: typescript-language-server, pylsp, gopls, rust-analyzer
- External parsers: Java, TypeScript, C# (if Maven/.NET/Node.js available)

**Or use Dev Container for zero-setup:**
- Open in VS Code → Automatically installs everything
- Perfect for quick experimentation

See **[CONTRIBUTING.md](CONTRIBUTING.md)** for development workflow and architecture.

## 🛠️ Language Server Setup

**Quick setup:** Run `codebuddy setup` and it'll auto-detect your project languages and guide you through configuration.

**Manual installation?** Expand for LSP server install commands:

<details>
<summary>Click to show language server installation</summary>

```bash
# TypeScript/JavaScript
npm install -g typescript-language-server typescript

# Python
pip install "python-lsp-server[all]"

# Go
go install golang.org/x/tools/gopls@latest

# Rust
rustup component add rust-analyzer

# Java
# Download from https://download.eclipse.org/jdtls/snapshots/
# Or use your IDE's bundled language server
```

Verify with: `codebuddy status`
</details>

## 📚 MCP Integration

Already using Claude Code or Cursor? Add this one snippet to your MCP configuration and you're done:

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

**Most used:**
```bash
codebuddy start    # Start the server
codebuddy status   # Check what's running
codebuddy setup    # Configure languages
```

<details>
<summary>See all commands</summary>

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
</details>

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

See [`docs/deployment/DOCKER_DEPLOYMENT.md`](docs/deployment/DOCKER_DEPLOYMENT.md) for details.

## 📖 Documentation
- **[API_REFERENCE.md](API_REFERENCE.md)** - Complete MCP tools API reference
- **[TOOLS_QUICK_REFERENCE.md](TOOLS_QUICK_REFERENCE.md)** - Quick tool lookup table
- **[docs/architecture/ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md)** - System architecture
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
cargo nextest run
cargo clippy
cargo fmt

# Or use the Makefile for convenience
make build       # Build debug version
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

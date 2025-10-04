# 🤖 Codebuddy
MCP server that exposes Language Server Protocol functionality to AI coding assistants

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

## ✨ Key Features
- **🔍 Navigation** - Find definitions, references, symbols via LSP
- **🔧 Refactoring** - Rename symbols, extract functions, organize imports
- **💡 Intelligence** - Completions, hover docs, diagnostics, call graphs
- **⚡ Batch Operations** - Atomic multi-file edits and parallel execution
- **🌐 Multi-Language** - TypeScript, Python, Go, Rust via configured LSP servers
- **🛠️ Transport Options** - Stdio for MCP clients, WebSocket with JWT auth

## 🚀 Quick Start
```bash
# Install from source
git clone https://github.com/goobits/codebuddy.git
cd codebuddy
cargo build --release
sudo cp target/release/codebuddy /usr/local/bin/

# Setup language servers (interactive wizard)
codebuddy setup

# Start MCP server (stdio transport)
codebuddy start

# Or start WebSocket server
codebuddy serve
```

## 🛠️ Language Server Setup
```bash
# TypeScript/JavaScript
npm install -g typescript-language-server typescript

# Python
pip install "python-lsp-server[all]"

# Go
go install golang.org/x/tools/gopls@latest

# Rust
rustup component add rust-analyzer

# Verify installation
codebuddy status
```

## 📚 MCP Integration
Configure your MCP client to connect to Codebuddy:

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

## ⚙️ Configuration
```bash
# Interactive setup
codebuddy setup

# View configuration
codebuddy status

# Manual configuration (.codebuddy/config.json)
cat > .codebuddy/config.json << 'EOF'
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
EOF
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
```bash
# LSP server not starting
codebuddy status                # Check installation
RUST_LOG=debug codebuddy start  # View logs

# Import updates failing
codebuddy setup                 # Reconfigure servers
```

Common issues:
- LSP server not in PATH
- File outside workspace root
- LSP server doesn't support workspace edits

## 🔗 Related Projects
- **[Model Context Protocol](https://github.com/modelcontextprotocol/servers)** - MCP specification
- **[Language Server Protocol](https://langserver.org/)** - LSP specification

## 🧪 Development
```bash
# Build and test
cargo build --release
cargo test
cargo clippy
cargo fmt

# Or use Makefile
make setup       # Install dev tools (one-time)
make             # Build debug
make test        # Run tests
make check       # fmt + clippy + test

# Run locally
cargo run -- start
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development guide.

## 📝 License
MIT - see [LICENSE](LICENSE)

## 💡 Support
- [Bug reports and features](https://github.com/goobits/codebuddy/issues)
- [Discussions](https://github.com/goobits/codebuddy/discussions)

---

**Credits**: Based on [ktnyt/cclsp](https://github.com/ktnyt/cclsp)
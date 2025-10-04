# 🤖 Codebuddy
Pure Rust MCP server bridging Language Server Protocol to AI coding assistants

## ✨ Key Features
- **🔍 Code Navigation** - Jump to definitions, find references, search symbols across projects
- **🔧 Safe Refactoring** - Rename symbols with compile-time safety guarantees
- **💡 Code Intelligence** - Hover documentation, completions, diagnostics, call hierarchies
- **⚡ Batch Operations** - Execute multiple LSP operations atomically with parallel processing
- **🌐 Multi-Language** - TypeScript, Python, Go, Rust + 15 more languages via LSP
- **🚀 Production Ready** - WebSocket server with JWT authentication and health monitoring

## 🚀 Quick Start
```bash
# Install via automated script (recommended)
curl -fsSL https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh | bash

# Or install from source manually
git clone https://github.com/goobits/codebuddy.git
cd codebuddy
cargo build --release
sudo cp target/release/codebuddy /usr/local/bin/

# Or install via Cargo (when published)
# cargo install codebuddy

# Setup with auto-detection
codebuddy setup

# Start MCP server for Claude Code
codebuddy start

# Or start WebSocket server
codebuddy serve
```

## 🛠️ Language Server Setup
```bash
# TypeScript/JavaScript (works via npx, or install explicitly)
npm install -g typescript-language-server typescript

# Python
pip install "python-lsp-server[all]"

# Go
go install golang.org/x/tools/gopls@latest

# Rust
rustup component add rust-analyzer

# Check configured servers
codebuddy status
```

## 📚 MCP Integration
The installer configures Claude Code automatically. For manual setup:

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
# Interactive setup wizard
codebuddy setup

# View current configuration
codebuddy status

# Link to AI assistants
codebuddy link

# Manual configuration
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
# Server management
codebuddy start          # Start MCP server (stdio mode)
codebuddy serve          # Start WebSocket server
codebuddy stop           # Stop running server
codebuddy status         # Check server status

# Configuration
codebuddy setup          # Interactive setup wizard
codebuddy doctor         # Diagnose configuration issues
codebuddy link           # Link to AI assistants
codebuddy unlink         # Remove AI integration

# Direct tool execution
codebuddy tool <name>    # Call MCP tool directly
codebuddy tools          # List available tools
```

## 🐳 Docker Deployment

### Development
```bash
cd deployment/docker

# Start all services
docker-compose up -d

# View logs
docker-compose logs -f codebuddy
```

### Production
```bash
cd deployment/docker

# Configure authentication
export JWT_SECRET="your-secure-secret-key"

# Start with nginx reverse proxy
docker-compose -f docker-compose.production.yml up -d

# Verify health
curl http://localhost/health
```

**Features**: Multi-stage Rust build, FUSE support, pre-installed LSP servers, nginx reverse proxy, multi-container workspaces

See [`deployment/docker/README.md`](deployment/docker/README.md) for detailed documentation.

## 📖 Documentation
- **[API Reference](API.md)** - Complete MCP tools documentation
- **[CLAUDE.md](CLAUDE.md)** - AI assistant integration guide
- **[Architecture](docs/architecture/ARCHITECTURE.md)** - System design
- **[Support Matrix](SUPPORT_MATRIX.md)** - Language support

## 🔧 Troubleshooting

**LSP server not starting?**
```bash
# Check server is installed
codebuddy status

# View detailed logs
RUST_LOG=debug codebuddy start
```

**Import updates not working?**
- Ensure LSP server supports workspace edits
- Check file is within workspace root
- Try `codebuddy setup` to reconfigure servers

## 🔗 Related Projects
- **[Model Context Protocol](https://github.com/modelcontextprotocol/servers)** - MCP specification and ecosystem
- **[Language Server Protocol](https://langserver.org/)** - LSP specification and implementations

## 🧪 Development
```bash
# Quick start (using Makefile)
make setup       # One-time: install build optimization tools
make             # Build debug version
make test        # Run tests
make install     # Install to ~/.local/bin

# Or use cargo directly
cargo build --release

# Run development version
cargo run -- start

# Testing
cargo test                    # Run all tests
cargo test -- --nocapture     # With output

# Code quality
cargo clippy                  # Linting
cargo fmt                     # Formatting
cargo check                   # Type checking
make clippy                   # Run clippy linter
make fmt                      # Check code formatting
make check                    # Run fmt + clippy + test
make check-duplicates         # Detect duplicate code & complexity
```

**Directory Structure:**
- `apps/` - Binary applications (codebuddy CLI)
- `crates/` - Library crates (cb-core, cb-server, cb-api, etc.)
- `integration-tests/` - Integration and E2E tests
- `benchmarks/` - Performance benchmarks
- `examples/` - User-facing examples
- `playground/` - Developer scratch space (gitignored)

**Contributing:** See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development setup and build optimization tips.

## 📝 License
MIT - see [LICENSE](LICENSE)

## 💡 Support
- [Bug reports and features](https://github.com/goobits/codebuddy/issues)
- [Discussions](https://github.com/goobits/codebuddy/discussions)

---

**Credits**: Based on [ktnyt/cclsp](https://github.com/ktnyt/cclsp)
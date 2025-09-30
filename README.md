# 🤖 Codebuddy

**Pure Rust MCP server** bridging Language Server Protocol functionality to AI coding assistants with comprehensive tools for navigation, refactoring, code intelligence, and batch operations.

## ✨ What It Does

**Comprehensive MCP tools** that give AI assistants LSP superpowers:
- **Find & Navigate** - Jump to definitions, find all references, search symbols
- **Refactor Safely** - Rename across entire codebase, with compile-time safety
- **Code Intelligence** - Hover docs, completions, diagnostics, call hierarchies
- **Batch Operations** - Execute multiple tools atomically with parallel processing
- **Advanced Analysis** - Directory renaming, import fixing, package.json management
- **Multi-Language** - TypeScript, Python, Go, Rust + 15 more languages
- **WebSocket Mode** - Production-ready server with authentication

## 🚀 Quick Install

### Option 1: One-Liner Install (Recommended)
```bash
curl -fsSL https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh | bash
```

### Option 2: Cargo Install
```bash
cargo install codebuddy
```

### Option 3: Download Pre-built Binary
1. Download from [GitHub Releases](https://github.com/goobits/codebuddy/releases/latest)
2. Extract and place in your PATH
3. Run `codebuddy setup`

### Option 4: Package Managers

#### Homebrew (macOS/Linux)
```bash
brew install goobits/tap/codebuddy
```

#### Chocolatey (Windows)
```bash
choco install codebuddy
```

## ⚡ Usage

```bash
# Smart setup with auto-detection
codebuddy setup

# Start MCP server for Claude Code
codebuddy start

# Check status
codebuddy status

# WebSocket server
codebuddy serve

# Stop the running server
codebuddy stop

# Link to AI assistants
codebuddy link

# Remove AI from config
codebuddy unlink
```

## 📚 MCP Integration

The installer automatically configures Claude Code. Manual setup:

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


## 🛠️ Language Server Setup
```bash
# TypeScript/JavaScript (works out of the box via npx)
# Optional explicit install:
npm install -g typescript-language-server typescript

# Python
pip install "python-lsp-server[all]"

# Go
go install golang.org/x/tools/gopls@latest

# Rust
rustup component add rust-analyzer

# View configuration and status
codebuddy status
```

## ⚙️ Configuration
```bash
# Smart setup with auto-detection
codebuddy setup

# Check status of language servers
codebuddy status

# Link to AI assistants
codebuddy link

# Manual configuration (creates .codebuddy/config.json)
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


## 📖 Documentation
- **[CLAUDE.md](CLAUDE.md)** - Project instructions and architecture
- **[Architecture](rust/docs/ARCHITECTURE.md)** - System design and implementation details
- **[Operations Guide](rust/docs/OPERATIONS.md)** - Deployment and operational procedures
- **[Usage Guide](rust/docs/USAGE.md)** - Detailed usage instructions

## 🔗 Related Projects
- **[Model Context Protocol](https://github.com/modelcontextprotocol/servers)** - Protocol specification and ecosystem
- **[Language Server Protocol](https://langserver.org/)** - LSP specification and implementations

## 🧪 Development
```bash
# Build from source
cd rust
cargo build --release

# Run development version
cargo run -- start

# WebSocket server
./target/release/codebuddy serve

# Testing
cargo test
cargo test -- --nocapture  # With output

# Code quality
cargo clippy              # Linting
cargo fmt                 # Format code
cargo check               # Type checking

# Build for production
cargo build --release
```

## 🐳 Docker Deployment

### Development Environment
```bash
cd docker

# Start all services (codebuddy + example workspaces)
docker-compose up -d

# View logs
docker-compose logs -f codebuddy
```

### Production Deployment
```bash
cd docker

# Set JWT secret for authentication
export JWT_SECRET="your-secure-secret-key"

# Start production stack with nginx proxy
docker-compose -f docker-compose.production.yml up -d

# Check health
curl http://localhost/health
```

### Features
- **Multi-stage Rust build**: Optimized 400MB runtime image
- **FUSE support**: Pre-configured with proper capabilities and security
- **Pre-installed LSP servers**: TypeScript and Python ready out-of-the-box
- **Nginx reverse proxy**: Production-grade WebSocket handling with SSL ready
- **Multi-container development**: Isolated workspaces for frontend/backend

See [`docker/README.md`](docker/README.md) for detailed Docker documentation and troubleshooting.

## 📝 License
MIT - see [LICENSE](LICENSE) for details

## 💡 Support
- [Bug reports and feature requests](https://github.com/goobits/codebuddy/issues)
- [Discussions and community support](https://github.com/goobits/codebuddy/discussions)

---

## 🙏 Special Thanks

This project is based on [ktnyt/cclsp](https://github.com/ktnyt/cclsp)

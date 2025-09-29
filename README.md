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

# WebSocket server (advanced)
codebuddy serve --port 3000

# With authentication
codebuddy serve --require-auth --jwt-secret "your-secret"
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

## 📊 Production Monitoring
```bash
# Health check
curl http://localhost:3000/healthz

# Prometheus metrics
curl http://localhost:3000/metrics

# Authentication endpoint (if enabled)
curl -X POST http://localhost:3000/auth \
  -H "Content-Type: application/json" \
  -d '{"projectId": "my-project", "secretKey": "my-secret"}'
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

## 🗄️ FUSE Support (Experimental)

FUSE (Filesystem in Userspace) enables mounting remote filesystems locally, allowing LSP servers to access files as if they were on the local filesystem.

### Platform Support

| Platform | Support | Requirements |
|----------|---------|-------------|
| Linux | ✅ Full | FUSE kernel module |
| macOS | ✅ Full | macFUSE installation |
| Windows | ❌ None | Use WSL2 for FUSE support |

### Setup FUSE

```bash
# FUSE setup must be done manually
# See platform-specific instructions below
```

#### Linux Setup
```bash
# Debian/Ubuntu
sudo apt-get install fuse fuse-dev
sudo usermod -aG fuse $USER  # Add user to fuse group

# RedHat/Fedora
sudo dnf install fuse fuse-devel

# Arch
sudo pacman -S fuse2 fuse3
```

#### macOS Setup
```bash
# Install macFUSE via Homebrew
brew install --cask macfuse

# Note: You'll need to allow the kernel extension in:
# System Preferences > Security & Privacy
```

### Verify Installation
```bash
# Check FUSE availability
codebuddy status  # Will show FUSE support status

# The system will automatically detect FUSE on startup
# and provide platform-specific instructions if needed
```

### Using FUSE with WebSocket Server
```bash
# Start server with FUSE enabled
codebuddy serve --port 3000 --enable-fuse

# Mount path configuration
codebuddy serve --fuse-mount-path /tmp/codeflow-mount
```

### Troubleshooting

- **Linux**: If you get permission errors, ensure you're in the `fuse` group and have logged out/in
- **macOS**: If macFUSE isn't detected, restart after installation and check Security & Privacy settings
- **Both**: Rebuild if needed after installing FUSE

## 📖 Documentation
- **[CLAUDE.md](CLAUDE.md)** - Project instructions and architecture
- **[Plugin Architecture](PLUGIN_ARCHITECTURE.md)** - System design and implementation

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

# WebSocket server development
./target/release/codebuddy serve --port 3000             # Basic server
./target/release/codebuddy serve --require-auth --jwt-secret KEY # With auth
docker-compose up -d                                      # Full stack

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

## 📝 License
MIT - see [LICENSE](LICENSE) for details

## 💡 Support
- [Bug reports and feature requests](https://github.com/goobits/codebuddy/issues)
- [Discussions and community support](https://github.com/goobits/codebuddy/discussions)

---

## 🙏 Special Thanks

This project is based on [ktnyt/cclsp](https://github.com/ktnyt/cclsp)

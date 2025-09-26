# 🤖 codeflow-buddy
**Enterprise-grade MCP server** bridging Language Server Protocol functionality to AI coding assistants with **31 MCP tools** and **WebSocket deployment**

## ✨ What It Does

**31 MCP tools** that give AI assistants LSP superpowers:
- **Find & Navigate** - Jump to definitions, find all references, search symbols
- **Refactor Safely** - Rename across entire codebase, with undo safety
- **Code Intelligence** - Hover docs, completions, diagnostics, call hierarchies
- **Multi-Language** - TypeScript, Python, Go, Rust + 15 more languages
- **WebSocket Mode** - Multi-client support for team deployments

## 🚀 Quick Start

### Traditional MCP Server
```bash
# Install globally (provides `codeflow-buddy` command)
npm install -g @goobits/codeflow-buddy

# Smart setup with auto-detection
codeflow-buddy setup

# Check status of language servers
codeflow-buddy status

# Start the MCP server for Claude Code
codeflow-buddy start
```

### WebSocket Server (Optional)
```bash
# Clone and build
git clone https://github.com/goobits/codeflow-buddy
cd codeflow-buddy && bun install && bun run build

# Start basic WebSocket server
node dist/index.js serve --port 3000

# With authentication (requires 32+ character secret)
node dist/index.js serve --require-auth --jwt-secret "your-32-character-secret-key-here"
```

## 📚 MCP Integration
```json
# Add to your MCP client configuration (e.g., Claude Code)
{
  "mcpServers": {
    "codeflow-buddy": {
      "command": "codeflow-buddy",
      "cwd": "/path/to/your/project"
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
codeflow-buddy status
```

## ⚙️ Configuration
```bash
# Smart setup with auto-detection
codeflow-buddy setup

# Check status of language servers
codeflow-buddy status

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
# Automatic setup script
./packages/server/scripts/setup-fuse.sh

# Or use the interactive CLI
node dist/cli/fuse-setup.js
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
codeflow-buddy check-fuse

# The system will automatically detect FUSE on startup
# and provide platform-specific instructions if needed
```

### Using FUSE with WebSocket Server
```bash
# Start server with FUSE enabled
node dist/index.js serve --port 3000 --enable-fuse

# Mount path configuration
node dist/index.js serve --fuse-mount-path /tmp/codeflow-mount
```

### Troubleshooting

- **Linux**: If you get permission errors, ensure you're in the `fuse` group and have logged out/in
- **macOS**: If macFUSE isn't detected, restart after installation and check Security & Privacy settings
- **Both**: Run `npm rebuild @cocalc/fuse-native` after installing FUSE

## 📖 Documentation
- **[Quick Start Guide](docs/quick-start.md)** - Get running in 2 minutes
- **[MCP Tools Reference](docs/api.md)** - All 31 tools with examples
- **[Language Setup](docs/languages.md)** - TypeScript, Python, Go, and more
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions

## 🔗 Related Projects
- **[Model Context Protocol](https://github.com/modelcontextprotocol/servers)** - Protocol specification and ecosystem
- **[Language Server Protocol](https://langserver.org/)** - LSP specification and implementations

## 🧪 Development
```bash
# Install dependencies
bun install

# Development with hot reload
bun run dev

# WebSocket server development
node dist/index.js serve --port 3000                    # Basic server
node dist/index.js serve --require-auth --jwt-secret KEY # With auth
docker-compose up -d                                     # Full stack

# Testing
bun run test:fast     # Fast mode with optimizations
bun run test          # Full test suite
bun run test:comprehensive # All MCP tools test
bun run test:minimal  # Minimal runner for slow systems

# Code quality
bun run lint         # Check code style and issues
bun run format       # Format code with Biome
bun run typecheck    # TypeScript type checking

# Build for production
bun run build
```

## 📝 License
MIT - see [LICENSE](LICENSE) for details

## 💡 Support
- **[GitHub Issues](https://github.com/goobits/codeflow-buddy/issues)** - Bug reports and feature requests
- **[Discussions](https://github.com/goobits/codeflow-buddy/discussions)** - Questions and community support

---

## 🙏 Special Thanks

This project is based on [ktnyt/cclsp](https://github.com/ktnyt/cclsp)

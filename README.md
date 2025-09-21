# 🤖 codeflow-buddy
MCP server bridging Language Server Protocol functionality to AI coding assistants with 38 MCP tools

## ✨ Key Features
- **🎯 Symbol Navigation** - Go to definition and find references with intelligent position resolution
- **🔧 Safe Refactoring** - Rename symbols and files across entire codebases with LSP validation
- **🧠 Code Intelligence** - Hover info, completions, diagnostics, and semantic analysis via LSP
- **🌐 Multi-Language Support** - TypeScript, Python, Go, Rust, and 15+ languages via configurable LSP servers
- **🤖 AI-Optimized Protocol** - Robust symbol resolution handling imprecise positions from LLMs
- **⚡ Smart Configuration** - Auto-detection and setup with `codeflow-buddy init` command

## 🚀 Quick Start
```bash
# Install globally (provides `codeflow-buddy` command)
npm install -g @goobits/codeflow-buddy

# Smart setup with auto-detection
codeflow-buddy init

# Check status of language servers
codeflow-buddy status

# Fix any missing language servers
codeflow-buddy fix
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
codeflow-buddy init

# Show current configuration
codeflow-buddy config

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
- **[API Reference](docs/api.md)** - Complete tool documentation with examples
- **[Language Setup](docs/languages.md)** - Installation for 15+ languages
- **[Configuration](docs/configuration.md)** - Advanced settings and options
- **[Testing Guide](docs/testing_guide.md)** - Development and testing instructions
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

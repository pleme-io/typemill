# 🔗 cclsp
MCP server that bridges Language Server Protocol functionality to AI coding assistants

## ✨ Key Features
- **🎯 Symbol Navigation** - Go to definition and find references with intelligent position resolution
- **🔧 Safe Refactoring** - Rename symbols and files across entire codebases with confidence
- **🧠 Code Intelligence** - Hover info, completions, diagnostics, and semantic analysis
- **🌐 Universal Language Support** - Works with any LSP-compatible server (TypeScript, Python, Go, Rust, etc.)
- **🤖 AI-Optimized** - Robust symbol resolution that handles imprecise line/column numbers from LLMs
- **⚡ Zero Configuration** - Works out of the box with TypeScript, configurable for other languages

## 🚀 Quick Start
```bash
# Install globally (provides `cclsp` in PATH)
npm install -g @goobits/cclsp

# Generate project configuration
cclsp init

# Interactive setup wizard for multi-language projects
npx @goobits/cclsp@latest setup

# Start MCP server (for manual testing)
cclsp --env CCLSP_CONFIG_PATH=/path/to/cclsp.json
```

## 📚 MCP Integration
```json
# Add to your MCP client configuration (e.g., Claude Code)
{
  "mcpServers": {
    "cclsp": {
      "command": "cclsp",
      "env": {
        "CCLSP_CONFIG_PATH": "/path/to/your/cclsp.json"
      }
    }
  }
}
```

## 🛠️ Language Server Setup
```bash
# TypeScript/JavaScript
# Works out of the box via npx. Optional explicit install:
# npm install -g typescript-language-server typescript

# Python
pip install "python-lsp-server[all]"

# Go
go install golang.org/x/tools/gopls@latest

# Rust  
rustup component add rust-analyzer

# View all supported languages
cclsp setup
```

## ⚙️ Configuration
```bash
# Generate commented config file
cclsp init

# Interactive setup with language detection
cclsp setup

# Manual configuration
cat > cclsp.json << 'EOF'
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
- **[Full API Reference](docs/api.md)** - Complete tool documentation with examples
- **[Language Setup Guide](docs/languages.md)** - Installation for 15+ languages
- **[Configuration Reference](docs/configuration.md)** - Advanced settings and options
- **[Troubleshooting Guide](docs/troubleshooting.md)** - Common issues and solutions

## 🔗 Related Projects
- **[Model Context Protocol](https://github.com/modelcontextprotocol/servers)** - Protocol specification and ecosystem
- **[Language Server Protocol](https://langserver.org/)** - LSP specification and implementations

## 🧪 Development
```bash
# Install dependencies  
bun install

# Run in development mode
bun run dev

# Run tests
bun run test:fast    # Quick unit tests (~8s)
bun run test:slow    # Full integration tests
bun run test:ci      # All tests for CI

# Code quality
bun run lint         # Check issues
bun run format       # Format code  
bun run typecheck    # Type checking
```

## 📝 License
MIT - see [LICENSE](LICENSE) for details

## 💡 Support
- **[GitHub Issues](https://github.com/ktnyt/cclsp/issues)** - Bug reports and feature requests
- **[Discussions](https://github.com/ktnyt/cclsp/discussions)** - Questions and community support

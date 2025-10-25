# 🤖 TypeMill

![CI Status](https://github.com/goobits/typemill/actions/workflows/ci.yml/badge.svg)
![Version](https://img.shields.io/crates/v/typemill)
![License](https://img.shields.io/crates/l/typemill)

Pure Rust MCP server bridging Language Server Protocol (LSP) to AI coding assistants

Provides 36 MCP tools for code navigation, refactoring, analysis, and batch operations across TypeScript and Rust projects.

## ✨ Key Features
- **🎯 Safe Refactoring** - Two-step plan → apply pattern with automatic rollback on failure
- **🔍 LSP Integration** - Native language server support for precise code intelligence
- **⚡ Rust Performance** - Zero-cost abstractions, memory safety, async I/O
- **🔄 Comprehensive Updates** - Automatic import updates, cross-file reference tracking
- **🐳 Production Ready** - WebSocket server, JWT auth, multi-tenant isolation, Docker support
- **🛠️ 36 Tools** - Navigation, refactoring, analysis, workspace operations, batch processing

## 🚀 Quick Start
```bash
# Install (recommended method)
curl -fsSL https://raw.githubusercontent.com/goobits/mill/main/install.sh | bash

# Alternative: Build from source
cargo install mill --locked

# Auto-detect languages and configure
mill setup

# Start the server
mill start

# Verify it's running
mill status
```

### Connect Your AI Assistant
Add to your MCP configuration (e.g., Claude Desktop):
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

### First Commands
Ask your AI assistant:
```
"Find the definition of main in src/main.rs"
"Show me all references to the Config type"
"Rename the function oldName to newName"
```

## 🛠️ CLI Usage
```bash
# File operations (no position needed)
mill tool rename --target file:src/old.rs --new-name src/new.rs
mill tool rename --target directory:old-dir --new-name new-dir

# Code operations (requires line:char position)
mill tool move --source src/app.rs:10:5 --destination src/utils.rs
mill tool extract --kind function --source src/app.rs:10:5 --name handleLogin

# Analysis
mill tool analyze.quality --kind complexity --scope workspace
mill tool analyze.dead_code --kind unused_imports --scope file:src/app.rs

# Workspace operations
mill tool workspace.find_replace --pattern "oldName" --replacement "newName"
```

**Key Distinction:**
- Use `rename` for file/directory operations
- Use `move` for code symbol operations (requires source position)

## 📚 Available Tools (36 total)

**🔍 Navigation & Intelligence (8 tools)**
- `find_definition`, `find_references`, `search_symbols`
- `find_implementations`, `find_type_definition`, `get_symbol_info`
- `get_diagnostics`, `get_call_hierarchy`

**✂️ Editing & Refactoring (15 tools)**
- **Plan Operations**: `rename.plan`, `extract.plan`, `inline.plan`, `move.plan`, `reorder.plan`, `transform.plan`, `delete.plan`
- **Quick Operations**: `rename`, `extract`, `inline`, `move`, `reorder`, `transform`, `delete`
- **Apply**: `workspace.apply_edit`

**📊 Analysis (8 tools)**
- `analyze.quality`, `analyze.dead_code`, `analyze.dependencies`
- `analyze.structure`, `analyze.documentation`, `analyze.tests`
- `analyze.batch`, `analyze.module_dependencies`

**📦 Workspace (4 tools)**
- `workspace.create_package`, `workspace.extract_dependencies`
- `workspace.update_members`, `workspace.find_replace`

**💚 System (1 tool)**
- `health_check`

## 🌐 Language Support

| Language | Extensions | LSP Server | Refactoring |
|----------|-----------|------------|-------------|
| TypeScript/JavaScript | ts, tsx, js, jsx | typescript-language-server | Full ✅ |
| Rust | rs | rust-analyzer | Full ✅ |

*Additional languages (Python, Go, Java, Swift, C#) available in git tag `pre-language-reduction`*

## ⚙️ Configuration
```bash
# View current configuration
cat .typemill/config.json

# Restart LSP servers (if experiencing issues)
mill stop && mill start

# Enable caching (disabled by default for development)
export TYPEMILL_DISABLE_CACHE=0
```

### Example Configuration
```json
{
  "servers": [
    {
      "extensions": ["ts", "tsx", "js", "jsx"],
      "command": ["typescript-language-server", "--stdio"],
      "restartInterval": 10
    },
    {
      "extensions": ["rs"],
      "command": ["rust-analyzer"],
      "restartInterval": 30
    }
  ]
}
```

### Environment Variable Overrides

Override any configuration value using `TYPEMILL__` prefix (double underscores):

```bash
# Server configuration
export TYPEMILL__SERVER__PORT=3000
export TYPEMILL__SERVER__HOST="127.0.0.1"

# Authentication (use env vars for secrets!)
export TYPEMILL__SERVER__AUTH__JWT_SECRET="your-secret-key"

# Cache settings
export TYPEMILL__CACHE__ENABLED=true
export TYPEMILL__CACHE__TTL_SECONDS=3600

# Or use a .env file (gitignored)
echo 'TYPEMILL__SERVER__AUTH__JWT_SECRET=dev-secret' > .env
```

**Security**: Never commit secrets to config files. Always use environment variables for sensitive data.

See [CLAUDE.md](CLAUDE.md#environment-variables) for complete environment variable reference.

## 🔧 Troubleshooting

**Server won't start:**
```bash
# Check LSP server availability
mill status

# Verify language servers are installed
which typescript-language-server
which rust-analyzer

# Review config file
cat .typemill/config.json
```

**Tools not working:**
- Ensure file extensions match config (`.rs` → `rust-analyzer`)
- Check MCP connection with AI assistant
- Review server logs for errors

**Performance issues:**
- Enable cache: `unset TYPEMILL_DISABLE_CACHE`
- Adjust `restartInterval` in config (recommended: 10-30 minutes)
- Check system resources (LSP servers can be memory-intensive)

## 📖 Documentation
- **[Tool Reference](docs/tools/)** - Complete API for all 36 tools
- **[Architecture Overview](docs/architecture/overview.md)** - System design and components
- **[Contributing Guide](contributing.md)** - Development setup and workflow
- **[Docker Deployment](docs/operations/docker_deployment.md)** - Production deployment
- **[CLAUDE.md](CLAUDE.md)** - AI agent instructions and comprehensive guide

## 🧪 Development
```bash
# Clone repository
git clone https://github.com/goobits/typemill.git
cd mill

# First-time setup (installs dev tools, builds parsers, validates)
make first-time-setup

# Run tests
cargo nextest run --workspace

# Run with LSP server tests (~60s, requires LSP servers)
cargo nextest run --workspace --features lsp-tests

# Code quality checks
cargo fmt && cargo clippy && cargo nextest run
```

See [contributing.md](contributing.md) for detailed development guide.

## 📝 License
See [LICENSE](LICENSE) for details.

## 💡 Support
- **Issues:** [GitHub Issues](https://github.com/goobits/typemill/issues)
- **Discussions:** [GitHub Discussions](https://github.com/goobits/typemill/discussions)
- **Security:** security@goobits.com (private disclosure)

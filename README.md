# 🤖 TypeMill

![CI Status](https://github.com/goobits/typemill/actions/workflows/ci.yml/badge.svg)
![Version](https://img.shields.io/crates/v/typemill)
![License](https://img.shields.io/crates/l/typemill)

Pure Rust MCP server bridging Language Server Protocol (LSP) to AI coding assistants

Provides 29 MCP tools for code navigation, refactoring, analysis, and batch operations across TypeScript and Rust projects.

## ✨ Key Features
- **🎯 Safe Refactoring** - Unified dryRun API (default: preview, explicit opt-in to execute) with automatic rollback on failure
- **🔍 LSP Integration** - Native language server support for precise code intelligence
- **⚡ Rust Performance** - Zero-cost abstractions, memory safety, async I/O
- **🔄 Comprehensive Updates** - Automatic import updates, cross-file reference tracking
- **🐳 Production Ready** - WebSocket server, JWT auth, multi-tenant isolation, Docker support
- **🛠️ 49 Tools** - 29 public MCP tools + 20 internal tools for navigation, refactoring, analysis, workspace operations

## 🚀 Quick Start
```bash
# Install (recommended method)
curl -fsSL https://raw.githubusercontent.com/goobits/mill/main/install.sh | bash

# Alternative: Build from source
cargo install mill --locked

# Auto-detect languages, configure, and install LSP servers
mill setup

# Start the server
mill start

# Verify it's running
mill status
```
**What `mill setup` does:**
- 🔍 Scans your project to detect languages (TypeScript, Rust, Python)
- 📋 Creates `.typemill/config.json` with LSP server configurations
- 📥 **Auto-downloads missing LSP servers** (with your permission)
- ✅ Verifies LSP servers are working
- 💾 Caches LSPs in `~/.mill/lsp/` for reuse across projects

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
mill tool workspace.find_replace '{"pattern": "oldName", "replacement": "newName", "scope": "workspace"}'
```
**Key Distinction:**
- Use `rename` for file/directory operations
- Use `move` for code symbol operations (requires source position)

## 📚 Available Tools (29 total)

**🔍 Navigation & Intelligence (8 tools)**
- `find_definition`, `find_references`, `search_symbols`
- `find_implementations`, `find_type_definition`, `get_symbol_info`
- `get_diagnostics`, `get_call_hierarchy`

**✂️ Editing & Refactoring (7 tools with dryRun API)**
- `rename`, `extract`, `inline`, `move`, `reorder`, `transform`, `delete`
- Each tool supports `options.dryRun` (default: true for safety, false to execute)

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

| Language | Extensions | LSP Server | Refactoring | Runtime Required |
|----------|-----------|------------|-------------|------------------|
| TypeScript/JavaScript | ts, tsx, js, jsx | typescript-language-server | Full ✅ | Node.js |
| Rust | rs | rust-analyzer | Full ✅ | - |
| Python | py | python-lsp-server (pylsp) | Full ✅ | Python 3 |
| Java | java | jdtls (Eclipse JDT LS) | Full ✅ | **Java 11+** |
| Go | go | gopls | Full ✅ | - |
| Swift | swift | sourcekit-lsp | Full ✅ | - |
| C# | cs | csharp-ls | Full ✅ | .NET SDK (optional) |
| C/C++ | c, cpp, h, hpp | clangd | Basic | - |
| Markdown | md | - | N/A | - |

**Note**: All languages except TypeScript and Rust restored from `pre-language-reduction` tag with 100% feature parity.

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
**Security Best Practices**:
- ✅ Never commit secrets to config files - use environment variables
- ✅ Keep server on `127.0.0.1` for local development (not `0.0.0.0`)
- ✅ Enable TLS when binding to non-loopback addresses for production
- ✅ Use secret management services (Vault, AWS Secrets Manager) in production

See [docs/configuration.md](docs/configuration.md) for complete configuration reference including environment variables, and [Docker Deployment](docs/operations/docker_deployment.md) for production setup.

## 📥 LSP Server Management

TypeMill automatically downloads and installs LSP servers during `mill setup`, but you can also manage them manually:

```bash
# Install LSP for a specific language
mill install-lsp rust
mill install-lsp typescript
mill install-lsp python

# Check what's installed
mill status  # Shows LSP server status

# LSPs are cached in ~/.mill/lsp/ for reuse across projects
ls ~/.mill/lsp/
```
**How it works:**
- **TypeScript**: Installs `typescript-language-server` via npm (requires Node.js)
- **Rust**: Downloads `rust-analyzer` binary from GitHub releases
- **Python**: Installs `python-lsp-server` via pip/pipx (requires Python)
- **Java**: Provides installation instructions for jdtls (requires **Java 11+ runtime**)

**Java Requirements:**
TypeMill's Java parser requires a Java runtime (JRE/JDK 11+) to be installed:
```bash
# Ubuntu/Debian
sudo apt-get install openjdk-17-jre-headless

# macOS (Homebrew)
brew install openjdk@17

# Verify installation
java --version
```

**Manual installation:**
If you prefer to install LSP servers yourself:
```bash
# TypeScript
npm install -g typescript-language-server typescript

# Rust
cargo install rust-analyzer

# Java (jdtls) - see mill install-lsp java for instructions
# Requires Java runtime (see above)

# Python
pipx install python-lsp-server  # Recommended (PEP 668 compliant)
# OR
pip install --user python-lsp-server
```
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
**LSP installation fails:**
```bash
# TypeScript: Ensure Node.js/npm is installed
node --version && npm --version

# Python: Ensure pip or pipx is available
python3 --version && pip3 --version
# Or use pipx (recommended for PEP 668 environments)
pipx --version

# Rust: Downloads from GitHub - check network/firewall
curl -I https://github.com/rust-lang/rust-analyzer/releases
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
- **[Getting Started](docs/user-guide/getting-started.md)** - Complete setup guide
- **[Configuration Reference](docs/user-guide/configuration.md)** - Configuration options
- **[Tool Reference](docs/tools/)** - Complete API for all 29 tools
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
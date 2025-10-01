# Codebuddy

> **📖 Looking for user documentation?** See the main [README.md](../README.md) at the repository root.
>
> **This document is for contributors** working on the Rust codebase.

---

A high-performance MCP (Model Context Protocol) server built in pure Rust that bridges Language Server Protocol functionality to AI coding assistants, providing comprehensive code intelligence, automated refactoring, and project-wide analysis capabilities.

## 🌟 Key Features

- **🤖 MCP Protocol Support** - Native implementation of Model Context Protocol for AI assistant integration
- **🔧 Language Server Integration** - Direct LSP communication for TypeScript, Python, Go, Rust, and more
- **♻️ Intelligent Refactoring** - AST-powered, project-wide refactoring with automatic import management
- **⚡ High Performance** - 300x+ performance improvements through intelligent caching and native Rust
- **🔌 Plugin Architecture** - Extensible system for language-specific implementations
- **🛠️ Complete CLI** - Full-featured command-line interface for server management
- **🔒 Production Ready** - JWT authentication, TLS support, and enterprise-grade security

## 📦 Installation

### Prerequisites

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs)
- **Language Servers** (optional, but recommended):
  - TypeScript/JavaScript: `npm install -g typescript-language-server`
  - Python: `pip install python-lsp-server`
  - Go: `go install golang.org/x/tools/gopls@latest`
  - Rust: `rustup component add rust-analyzer`

### Building from Source

```bash
# Clone the repository
git clone https://github.com/goobits/codebuddy.git
cd codebuddy/rust

# Build in release mode (recommended for performance)
cargo build --release

# The binary will be at: ./target/release/codebuddy
```

### Quick Install (Unix/Linux/macOS)

```bash
# Option 1: Install from published crate (recommended)
cargo install codebuddy

# Option 2: Build and install from source
cargo install --path apps/server

# Verify installation
codebuddy --help
```

## 🚀 Quick Start

### 1. Initial Setup

```bash
# Create default configuration with LSP servers
codebuddy setup

# This creates .codebuddy/config.json with sensible defaults
```

### 2. Start the Server

```bash
# Start in stdio mode (for AI assistants like Claude)
codebuddy start

# Or start as a background daemon
codebuddy start --daemon

# Or start WebSocket server for remote connections
codebuddy serve --port 3000
```

### 3. Check Server Status

```bash
# See if the server is running
codebuddy status
```

### 4. Stop the Server

```bash
# Gracefully stop the server
codebuddy stop
```

## 📖 CLI Commands

### `codebuddy setup`
Initialize configuration with default LSP servers. Creates `.codebuddy/config.json` with configurations for TypeScript, Python, Go, and Rust.

### `codebuddy start [--daemon]`
Start the MCP server in stdio mode for AI assistant integration.
- `--daemon`: Run as a background process with PID file management

### `codebuddy serve [--daemon] [--port <PORT>]`
Start WebSocket server for remote connections.
- `--daemon`: Run as a background process
- `--port`: Specify port (default: 3000)

### `codebuddy status`
Check if the server is running and display process information.

### `codebuddy stop`
Gracefully stop a running server instance.

## ⚙️ Configuration

The configuration file (`.codebuddy/config.json`) controls server behavior and LSP settings:

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 3040,
    "maxClients": 10,
    "timeoutMs": 30000
  },
  "lsp": {
    "servers": [
      {
        "extensions": ["ts", "tsx", "js", "jsx"],
        "command": ["typescript-language-server", "--stdio"],
        "restartInterval": 10
      },
      {
        "extensions": ["py"],
        "command": ["pylsp"],
        "restartInterval": 5
      }
    ]
  }
}
```

### Adding Language Servers

Edit `.codebuddy/config.json` to add support for additional languages:

```json
{
  "extensions": ["java"],
  "command": ["jdtls"],
  "restartInterval": 15
}
```

## 🔧 MCP Tools Available

Codebuddy exposes comprehensive code intelligence tools through MCP:

### Navigation
- `find_definition` - Jump to symbol definition
- `find_references` - Find all references to a symbol
- `search_workspace_symbols` - Search for symbols across the project
- `get_document_symbols` - Get all symbols in a file

### Code Intelligence
- `get_hover` - Get documentation and type information
- `get_completions` - Get intelligent code completions
- `get_signature_help` - Get function signature information
- `get_diagnostics` - Get errors and warnings

### Refactoring
- `rename_symbol` - Rename symbols project-wide
- `rename_symbol_with_imports` - Rename with import updates
- `organize_imports` - Clean up and sort imports
- `format_document` - Format code according to language rules

### File Operations
- `create_file` - Create new files with LSP awareness
- `delete_file` - Delete files with LSP cleanup
- `rename_file` - Rename/move files with import updates

### Advanced
- `apply_workspace_edit` - Apply multi-file atomic edits
- `extract_function` - Extract code into new functions
- `extract_variable` - Extract expressions into variables

## 🤝 Integration with AI Assistants

### Claude Desktop Integration

1. Build and start the server:
```bash
codebuddy start --daemon
```

2. Claude will automatically discover and use the MCP server for code intelligence.

### Custom Integration

Connect to the WebSocket server:

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.send(JSON.stringify({
  jsonrpc: "2.0",
  method: "tools/call",
  params: {
    name: "find_definition",
    arguments: {
      file_path: "src/main.ts",
      line: 10,
      character: 15
    }
  },
  id: 1
}));
```

## 🧪 Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test suite
cargo test --package cb-server
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check for issues
cargo check
```

### Performance Testing

```bash
# Run benchmarks
cargo bench

# Profile with flamegraph
cargo flamegraph --bin codebuddy
```

## 📊 Performance

- **AST Caching**: 300-400x speedup for repeated operations
- **Concurrent Processing**: Lock-free reads, minimal contention
- **Native Performance**: Zero-cost abstractions in Rust
- **Memory Efficient**: Typical usage under 100MB RAM
- **Fast Startup**: < 100ms to operational state

## 🏗️ Architecture

Codebuddy uses a modern, service-oriented architecture:

- **`cb-api`** - Trait definitions and contracts
- **`cb-core`** - Core types and configuration
- **`cb-ast`** - AST parsing and analysis engine
- **`cb-server`** - MCP server and LSP management
- **`cb-plugins`** - Language-specific plugins
- **`apps/server`** - CLI and executable entry point

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed architecture documentation.

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Quick Contribution Checklist

- [ ] Run `cargo fmt` before committing
- [ ] Ensure `cargo clippy` passes
- [ ] Add tests for new functionality
- [ ] Update documentation as needed

## 📝 License

[MIT License](LICENSE) - See LICENSE file for details

## 🙏 Acknowledgments

Built with excellent tools and libraries:
- [Tower LSP](https://github.com/tower-lsp/tower-lsp) for LSP client implementation
- [Tokio](https://tokio.rs/) for async runtime
- [Tree-sitter](https://tree-sitter.github.io/) for syntax parsing
- [Clap](https://github.com/clap-rs/clap) for CLI argument parsing

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/goobits/codebuddy/issues)
- **Discussions**: [GitHub Discussions](https://github.com/goobits/codebuddy/discussions)
- **Documentation**: [docs/](docs/)

---

**Ready to supercharge your AI coding assistant?** Get started with `codebuddy setup`! 🚀
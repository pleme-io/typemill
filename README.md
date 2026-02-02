# 🤖 TypeMill

![CI Status](https://github.com/goobits/typemill/actions/workflows/ci.yml/badge.svg)
![Version](https://img.shields.io/crates/v/typemill)
![License](https://img.shields.io/crates/l/typemill)

**Pure Rust MCP server bridging Language Server Protocol (LSP) to AI coding assistants.**

![TypeMill Demo](packages/typemill/demo/demo.svg)

TypeMill gives your AI assistant (Claude, Cursor, etc.) direct access to language server intelligence. It enables safe refactoring, precise code navigation, and workspace-aware operations across TypeScript, Rust, and Python projects.

## ✨ Key Features

- **🛡️ Safe Refactoring** - Unified dry-run API with automatic rollback protection. Preview changes before execution.
- **🧠 Native Intelligence** - Leverages industry-standard LSP servers (`rust-analyzer`, `tsserver`) for 100% accurate symbol resolution.
- **⚡ Rust Performance** - Built for speed with zero-cost abstractions and async I/O.
- **🔄 Auto-Updates** - Automatically handles imports, cross-file references, and self-references during moves and renames.
- **🔌 Tooling Depth** - Comprehensive suite for navigation (`inspect_code`), search (`search_code`), and refactoring (`rename_all`, `relocate`).

## 🚀 Quick Start

### 1. Install
```bash
curl -fsSL https://raw.githubusercontent.com/goobits/mill/main/install.sh | bash
```
*Alternatively: `cargo install mill --locked`*

### 2. Setup
Run in your project root to auto-detect languages and install necessary LSP servers:
```bash
mill setup
```

### 3. Connect
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

## 📚 Documentation

Detailed guides and references are available in the [docs/](docs/) directory.

- **[Getting Started](docs/user-guide/getting-started.md)** - comprehensive setup & configuration.
- **[Tool Reference](docs/tools/README.md)** - complete catalog of available tools.
- **[Configuration](docs/user-guide/configuration.md)** - customize servers and behavior.
- **[Troubleshooting](docs/user-guide/troubleshooting.md)** - common issues and solutions.
- **[Contributing](contributing.md)** - development workflow.

## 🌐 Language Support

| Language | Support Level | LSP Server |
|----------|---------------|------------|
| **TypeScript/JS** | Full ✅ | `typescript-language-server` |
| **Rust** | Full ✅ | `rust-analyzer` |
| **Python** | Full ✅ | `python-lsp-server` |
| **Markdown** | Basic | - |

## License

[MIT](LICENSE) © [Goobits](https://github.com/goobits)

# 🤖 Codebuddy

**Give your AI coding assistant superpowers.**

Codebuddy bridges the gap between AI assistants and your codebase by exposing Language Server Protocol (LSP) functionality through the Model Context Protocol (MCP). It lets AI tools understand your code the same way your IDE does.

---

## 🚀 Get Started

**New users:** See **[QUICKSTART.md](docs/QUICKSTART.md)** for complete 2-minute setup.

**Quick summary:**
1. Install: `curl -fsSL https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh | bash`
2. Configure: `codebuddy setup`
3. Start: `codebuddy start`

**References:** [QUICKSTART](docs/QUICKSTART.md) · [Tools Catalog](docs/TOOLS_CATALOG.md) · [API Reference](docs/API_REFERENCE.md)

---

## 📋 Table of Contents
- [What is Codebuddy?](#-what-is-codebuddy)
- [Documentation](#-documentation)
- [Development](#-development)
- [License](#-license)

## 🤔 What is Codebuddy?

Your AI assistant can finally understand your codebase the way your IDE does:

- **Navigate intelligently** - Jump to definitions, find references, search symbols across your workspace.
- **Refactor safely** - Rename across files, extract functions, organize imports—with automatic updates.
- **Scale confidently** - Atomic multi-file edits, batch operations, smart directory moves.

Currently supports **TypeScript and Rust** with full AST analysis and refactoring capabilities. Built in Rust for memory safety and blazing performance.

## 📖 Documentation

- **[QUICKSTART.md](docs/QUICKSTART.md)** - Get running in 2 minutes
- **[TOOLS_CATALOG.md](docs/TOOLS_CATALOG.md)** - Complete list of 23 MCP tools
- **[API_REFERENCE.md](docs/API_REFERENCE.md)** - Detailed API with parameters and returns
- **[OPERATIONS.md](docs/OPERATIONS.md)** - Advanced configuration and analysis
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Developer guide for contributors
- **[docs/architecture/overview.md](docs/architecture/overview.md)** - System architecture deep-dive

## 💻 Development

Want to contribute? We'd love to have you!

```bash
# Build, lint, and test
make check

# Run tests
make test
```
See **[CONTRIBUTING.md](CONTRIBUTING.md)** for the full development guide, including how to add new tools.

## 📝 License
MIT - see [LICENSE](LICENSE)

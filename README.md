# 🤖 Codebuddy

**Give your AI coding assistant superpowers.**

Codebuddy bridges the gap between AI assistants and your codebase by exposing Language Server Protocol (LSP) functionality through the Model Context Protocol (MCP). It lets AI tools understand your code the same way your IDE does.

---

## 🚀 Get Started

**New users:** See **[quickstart.md](docs/quickstart.md)** for complete 2-minute setup.

**Quick summary:**
1. Install: `curl -fsSL https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh | bash`
2. Configure: `codebuddy setup`
3. Start: `codebuddy start`

**References:** [QUICKSTART](docs/quickstart.md) · [Tools Catalog](docs/tools_catalog.md) · [API Reference](docs/api_reference.md)

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

- **[quickstart.md](docs/quickstart.md)** - Get running in 2 minutes
- **[tools_catalog.md](docs/tools_catalog.md)** - Complete list of 23 MCP tools
- **[api_reference.md](docs/api_reference.md)** - Detailed API with parameters and returns
- **[operations.md](docs/operations.md)** - Advanced configuration and analysis
- **[contributing.md](contributing.md)** - Developer guide for contributors
- **[docs/architecture/overview.md](docs/architecture/overview.md)** - System architecture deep-dive

## 💻 Development

Want to contribute? We'd love to have you!

### Quick Start

```bash
# Build, lint, and test
make check

# Run tests
make test

# Build automation tasks (xtask pattern)
cargo xtask install           # Install codebuddy
cargo xtask check-all         # Run all checks
cargo xtask new-lang python   # Scaffold new language plugin
```

See **[contributing.md](contributing.md)** for the full development guide, including how to add new tools.

### Build Automation

This project uses the **xtask pattern** for cross-platform build automation. Instead of shell scripts, we write tasks in Rust for better type safety and Windows compatibility.

Available commands:
- `cargo xtask install` - Install codebuddy to ~/.local/bin
- `cargo xtask check-all` - Run fmt + clippy + test + deny
- `cargo xtask check-duplicates` - Detect duplicate code
- `cargo xtask check-features` - Validate cargo features
- `cargo xtask new-lang <language>` - Create language plugin scaffold
- `cargo xtask --help` - Show all available tasks

## 🔒 Security

This project uses [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) for comprehensive dependency management:

- **Security vulnerability scanning** via RustSec Advisory Database
- **License compliance checking** (MIT/Apache-2.0/BSD)
- **Duplicate dependency detection** to minimize bloat
- **Dependency source validation**

Run security checks:
```bash
make deny           # Check all (advisories, licenses, duplicates, sources)
cargo deny check    # Direct cargo-deny invocation
```

For more details on dependency management, see **[contributing.md](contributing.md#dependency-management)**.

## 📝 License
MIT - see [LICENSE](LICENSE)

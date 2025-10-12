# 🤖 Codebuddy

**Give your AI coding assistant superpowers.**

Codebuddy bridges the gap between AI assistants and your codebase by exposing Language Server Protocol (LSP) functionality through the Model Context Protocol (MCP). It lets AI tools understand your code the same way your IDE does.

---

### **For Experts: The 2-Minute Setup**

Already know your way around LSP and AI assistants? Get productive in minutes.

1.  **Install & Configure:**
    ```bash
    # Install the binary
    curl -fsSL https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh | bash

    # Configure for your project (auto-detects languages)
    codebuddy setup
    ```

2.  **Start & Integrate:**
    ```bash
    # Start the server
    codebuddy start

    # Add to your MCP config
    # { "mcpServers": { "codebuddy": { "command": "codebuddy", "args": ["start"] } } }
    ```

**Done.** For a quick reference of commands and tools, see the **[🚀 Quick Reference Guide](QUICK_REFERENCE.md)**.

---

## 📋 Table of Contents
- [What is Codebuddy?](#-what-is-codebuddy)
- [Installation](#-installation)
- [Usage](#-usage)
- [Configuration](#️-configuration)
- [Documentation](#-documentation)
- [Development](#-development)
- [License](#-license)

## 🤔 What is Codebuddy?

Your AI assistant can finally understand your codebase the way your IDE does:

- **Navigate intelligently** - Jump to definitions, find references, search symbols across your workspace.
- **Refactor safely** - Rename across files, extract functions, organize imports—with automatic updates.
- **Scale confidently** - Atomic multi-file edits, batch operations, smart directory moves.

Currently supports **TypeScript and Rust** with full AST analysis and refactoring capabilities. Built in Rust for memory safety and blazing performance.

## 📥 Installation

```bash
# Recommended: Use the install script
curl -fsSL https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh | bash

# Or, build from source if you have Rust installed
cargo install codebuddy --locked
```

## 🕹️ Usage

1.  **Configure for your project:**
    The `setup` command scans your project, detects languages, and creates a `.codebuddy/config.json` file for you.
    ```bash
    codebuddy setup
    ```

2.  **Start the server:**
    This command starts the MCP server that your AI assistant will connect to.
    ```bash
    codebuddy start
    ```

3.  **Check the status:**
    Verify that Codebuddy and your language servers are running correctly.
    ```bash
    codebuddy status
    ```

## ⚙️ Configuration

Configuration is handled by `codebuddy setup`, but you can edit `.codebuddy/config.json` directly.

<details>
<summary>Click to see a configuration example</summary>

```json
{
  "servers": [
    {
      "extensions": ["js", "ts", "jsx", "tsx"],
      "command": ["npx", "--", "typescript-language-server", "--stdio"],
      "restartInterval": 30
    },
    {
      "extensions": ["rs"],
      "command": ["rust-analyzer"]
    }
  ]
}
```
The optional `restartInterval` helps with long-running server stability.
</details>

## 📖 Documentation

- **[🚀 QUICK_REFERENCE.md](QUICK_REFERENCE.md)** - A one-page guide for power users.
- **[API_REFERENCE.md](API_REFERENCE.md)** - The complete MCP tools API reference.
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - The guide for developers building from source.
- **[docs/architecture/ARCHITECTURE.md](overview.md)** - System architecture deep-dive.

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

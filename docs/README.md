# TypeMill Documentation

Complete documentation for TypeMill - Pure Rust MCP server for AI-powered code intelligence.

> **💡 Viewing from CLI?** All links work with `mill docs <topic>`. For example: `mill docs quickstart` or `mill docs tools/refactor`

---

## 📚 User Guide

- **[Getting Started](user-guide/getting-started.md)** - Complete setup guide (installation → team setup)
- **[Configuration](user-guide/configuration.md)** - Configuration reference and environment variables
- **[Cheatsheet](user-guide/cheatsheet.md)** - Quick command reference
- **[Troubleshooting](user-guide/troubleshooting.md)** - Common issues and solutions

## 🛠️ Tool Reference

Complete catalog of available tools. See **[tools/README.md](tools/README.md)** for the full index.

- **[inspect_code](tools/inspect_code.md)** - Aggregate code intelligence (definition, references, types, diagnostics)
- **[search_code](tools/search_code.md)** - Search workspace symbols
- **[refactor](tools/refactor.md)** - Extract/inline/transform operations
- **[Workspace Tools](tools/workspace.md)** - Package management, find-replace
- **[System Tools](tools/system.md)** - Health checks, server status

**Language-Specific Setup:**
- [TypeScript Setup](tools/workspace-typescript.md)
- [Rust Setup](tools/workspace-rust.md)
- [Python Setup](tools/workspace-python.md)

## 🏗️ Architecture & Development

For contributors and architects.

- **[Core Concepts](architecture/core-concepts.md)** - System architecture and design philosophy
- **[Specifications](architecture/specifications.md)** - API contracts and tool visibility
- **[Internal Tools](architecture/internal_tools.md)** - Tool visibility policy
- **[Language API](architecture/lang_common_api.md)** - Language abstraction

**Contributing:**
- [Development Overview](development/overview.md)
- [Dev Containers](development/dev-container.md)
- [Plugin Development](development/plugin-development.md)
- [Testing Architecture](development/testing.md)
- [Logging Guidelines](development/logging_guidelines.md)

## ⚙️ Operations

- **[Docker Deployment](operations/docker_deployment.md)** - Production deployment guide
- **[Cache Configuration](operations/cache_configuration.md)** - Performance tuning
- **[CI/CD Integration](operations/cicd.md)** - Automating workflows

---

## 🔍 Quick Reference

| Document | Purpose | CLI Command |
|----------|---------|-------------|
| **[user-guide/getting-started](user-guide/getting-started.md)** | Complete setup | `mill docs user-guide/getting-started` |
| **[user-guide/cheatsheet](user-guide/cheatsheet.md)** | Command reference | `mill docs user-guide/cheatsheet` |
| **[tools/](tools/)** | Tool API reference | `mill docs tools` |
| **[development/overview](development/overview.md)** | Plugin development | `mill docs development/overview` |
| **[architecture/core-concepts](architecture/core-concepts.md)** | System design | `mill docs architecture/core-concepts` |

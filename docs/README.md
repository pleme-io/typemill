# TypeMill Documentation

> **📍 You are here:** Documentation hub (organized by role: users, contributors, operators)
> - 👤 **New to TypeMill?** See [../README.md](../README.md) for project overview & quick start
> - 🤖 **AI agents**: See [../CLAUDE.md](../CLAUDE.md) for tool quick reference
> - 📖 **Practical workflows**: See [cookbook.md](cookbook.md) for step-by-step recipes

**Complete documentation for TypeMill - Pure Rust MCP server for AI-powered code intelligence**

> **💡 Viewing from CLI?** All links work with `mill docs <topic>`. For example: `mill docs quickstart` or `mill docs tools/refactor`

---

## 🚀 Start Here

**Pick your path based on your goal:**

| I want to... | Read this | CLI Command |
|--------------|-----------|-------------|
| Get started from scratch | [user-guide/getting-started](user-guide/getting-started.md) | `mill docs user-guide/getting-started` |
| See common commands | [user-guide/cheatsheet](user-guide/cheatsheet.md) | `mill docs user-guide/cheatsheet` |
| Browse the tool catalog | [tools/](tools/) | `mill docs tools` |
| Search documentation | - | `mill docs --search "keyword"` |

**For GitHub users:** See [../README.md](https://github.com/goobits/typemill#readme) for project overview

---

## 📚 Documentation by Role

### End Users

**Getting started:**
1. **[user-guide/getting-started.md](user-guide/getting-started.md)** - Complete setup guide (installation → team setup)
2. **[user-guide/cheatsheet.md](user-guide/cheatsheet.md)** - Quick command reference
3. **[user-guide/configuration.md](user-guide/configuration.md)** - Configuration reference
4. **[user-guide/troubleshooting.md](user-guide/troubleshooting.md)** - Common issues and solutions
5. **[tools/README.md](tools/README.md)** - Complete tool catalog (see **[architecture/specifications.md](architecture/specifications.md#tools-visibility-specification)** for full specification)

**Tool categories:**
- **[inspect_code](tools/inspect_code.md)** - Aggregate code intelligence (definition, references, types, diagnostics)
- **[search_code](tools/search_code.md)** - Search workspace symbols
- **[refactor](tools/refactor.md)** - Extract/inline/transform operations
- **[Workspace Tools](tools/workspace.md)** - Package management, find-replace (4 tools)
- **[System Tools](tools/system.md)** - Health checks, server status (1 tool)

**Language-specific guides:**
- **[TypeScript Setup](tools/workspace-typescript.md)** - Configure for TypeScript projects
- **[Rust Setup](tools/workspace-rust.md)** - Configure for Rust projects
- **[Python Setup](tools/workspace-python.md)** - Configure for Python projects

---

### Contributors

**Getting started with development:**
1. **[development/overview.md](development/overview.md)** - Complete development guide
2. **[development/testing.md](development/testing.md)** - Testing architecture
3. **[development/logging_guidelines.md](development/logging_guidelines.md)** - Structured logging

**Architecture deep-dive:**
- **[architecture/core-concepts.md](architecture/core-concepts.md)** - System architecture
- **[architecture/specifications.md](architecture/specifications.md)** - API contracts and tool visibility
- **[architecture/lang_common_api.md](architecture/lang_common_api.md)** - Language abstraction

**For contributing workflow:** See [contributing.md](https://github.com/goobits/typemill/blob/main/contributing.md) on GitHub

---

### Operators

**Deployment & operations:**
1. **[operations/docker_deployment.md](operations/docker_deployment.md)** - Docker deployment guide
2. **[operations/cache_configuration.md](operations/cache_configuration.md)** - Performance tuning
3. **[operations/cicd.md](operations/cicd.md)** - CI/CD integration

**Monitoring:**
- **[development/logging_guidelines.md](development/logging_guidelines.md)** - Production logging

---

### 🏗️ Architects

**Understanding internals:**
1. **[architecture/core-concepts.md](architecture/core-concepts.md)** - Complete system design
2. **[architecture/specifications.md](architecture/specifications.md)** - API contracts and tool visibility
3. **[architecture/internal_tools.md](architecture/internal_tools.md)** - Tool visibility policy
4. **[architecture/lang_common_api.md](architecture/lang_common_api.md)** - Language abstraction

---

## 📖 Complete Documentation Index

### User Guides
- **[user-guide/getting-started.md](user-guide/getting-started.md)** - Complete setup guide
- **[user-guide/configuration.md](user-guide/configuration.md)** - Configuration reference
- **[user-guide/cheatsheet.md](user-guide/cheatsheet.md)** - Command quick reference
- **[user-guide/troubleshooting.md](user-guide/troubleshooting.md)** - Troubleshooting guide

### Tool Reference
- **[tools/README.md](tools/README.md)** - Complete tool catalog
  _For authoritative specification including internal tools, see **[architecture/specifications.md](architecture/specifications.md#tools-visibility-specification)**_
- **[tools/inspect_code.md](tools/inspect_code.md)** - Code intelligence
- **[tools/search_code.md](tools/search_code.md)** - Symbol search
- **[tools/refactor.md](tools/refactor.md)** - Refactoring operations
- **[tools/workspace.md](tools/workspace.md)** - Workspace operations (4 tools)
- **[tools/workspace-rust.md](tools/workspace-rust.md)** - Rust-specific workspace tools
- **[tools/workspace-typescript.md](tools/workspace-typescript.md)** - TypeScript-specific workspace tools
- **[tools/workspace-python.md](tools/workspace-python.md)** - Python-specific workspace tools
- **[tools/system.md](tools/system.md)** - System tools (1 tool)

### Development
- **[development/overview.md](development/overview.md)** - Complete contributor quickstart
- **[development/plugin-development.md](development/plugin-development.md)** - Plugin development guide
- **[development/testing.md](development/testing.md)** - Testing architecture & workflow
- **[development/logging_guidelines.md](development/logging_guidelines.md)** - Structured logging standards

### Operations
- **[operations/docker_deployment.md](operations/docker_deployment.md)** - Docker deployment (dev & production)
- **[operations/cache_configuration.md](operations/cache_configuration.md)** - Cache configuration & performance
- **[operations/cicd.md](operations/cicd.md)** - CI/CD integration guide

### Architecture
- **[architecture/core-concepts.md](architecture/core-concepts.md)** - System architecture & design philosophy
- **[architecture/specifications.md](architecture/specifications.md)** - API contracts & tool visibility
- **[architecture/internal_tools.md](architecture/internal_tools.md)** - Public vs internal tools policy
- **[architecture/lang_common_api.md](architecture/lang_common_api.md)** - Language plugin common API

### Guides
- **[guides/plugin-migration.md](guides/plugin-migration.md)** - Plugin refactoring migration guide

---

## 💡 Tips for CLI Users

**Viewing documentation:**
```bash
mill docs                              # List all topics
mill docs quickstart                   # View quickstart guide
mill docs tools/refactor               # View refactor tool
mill docs README                       # View this file
mill docs --search "setup"             # Search all documentation
mill docs <topic> --raw                # View raw markdown
```
**All documentation paths work as-is in the CLI!**
- `mill docs tools/navigation` ✅
- `mill docs architecture/overview` ✅
- `mill docs development/testing` ✅

---

## 🔍 Quick Reference Table

| Document | Purpose | Audience | CLI Command |
|----------|---------|----------|-------------|
| **[user-guide/getting-started](user-guide/getting-started.md)** | Complete setup | New users | `mill docs user-guide/getting-started` |
| **[user-guide/cheatsheet](user-guide/cheatsheet.md)** | Command reference | All users | `mill docs user-guide/cheatsheet` |
| **[tools/](tools/)** | Tool API reference | Integrators | `mill docs tools` |
| **[development/overview](development/overview.md)** | Plugin development | Contributors | `mill docs development/overview` |
| **[architecture/core-concepts](architecture/core-concepts.md)** | System design | Architects | `mill docs architecture/core-concepts` |
| **[operations/docker_deployment](operations/docker_deployment.md)** | Deployment | Operators | `mill docs operations/docker_deployment` |

---

## 📝 Documentation Standards

All TypeMill documentation follows these principles:

- **Accuracy First** - Every statement reflects current code reality
- **Concise & Dense** - Maximum information density, minimum word count
- **Single Source of Truth** - One canonical location per topic
- **Up-to-date** - Synchronized with codebase changes
- **CLI-Friendly** - All links work in embedded viewer

---

## 🆘 Need Help?

**Within Mill:**
- `mill docs --search "<keyword>"` - Search all documentation
- `mill doctor` - Diagnose configuration issues
- `mill status` - Check server status

**Online:**
- **GitHub Issues**: https://github.com/goobits/typemill/issues
- **GitHub Discussions**: https://github.com/goobits/typemill/discussions
- **Security**: security@goobits.com (private disclosure)

---

*Last Updated: 2025-10-28 (Documentation restructuring - Phase 1)*

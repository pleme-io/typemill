# TypeMill Documentation

**Complete documentation for TypeMill - Pure Rust MCP server for AI-powered code intelligence**

> **💡 Viewing from CLI?** All links work with `mill docs <topic>`. For example: `mill docs quickstart` or `mill docs tools/refactoring`

---

## 🚀 Start Here

**Pick your path based on your goal:**

| I want to... | Read this | CLI Command |
|--------------|-----------|-------------|
| 🎯 Get running in 5 minutes | [quickstart](quickstart.md) | `mill docs quickstart` |
| 📋 See common commands | [cheatsheet](cheatsheet.md) | `mill docs cheatsheet` |
| 🛠️ Browse all 28 tools | [tools/](tools/) | `mill docs tools` |
| 🔍 Search documentation | - | `mill docs --search "keyword"` |

**For GitHub users:** See [../README.md](https://github.com/goobits/typemill#readme) for project overview

---

## 📚 Documentation by Role

### 👤 End Users

**Getting started:**
1. **[quickstart.md](quickstart.md)** - 5-minute setup tutorial
2. **[cheatsheet.md](cheatsheet.md)** - Quick command reference
3. **[tools/README.md](tools/README.md)** - Complete tool catalog (28 tools)

**Tool categories:**
- **[Navigation Tools](tools/navigation.md)** - Find definitions, references, symbols (8 tools)
- **[Refactoring Tools](tools/refactoring.md)** - Rename, extract, move, inline (7 tools)
- **[Analysis Tools](tools/analysis.md)** - Quality, dead code, dependencies (8 tools)
- **[Workspace Tools](tools/workspace.md)** - Package management, find-replace (4 tools)
- **[System Tools](tools/system.md)** - Health checks, server status (1 tool)

**Language-specific guides:**
- **[TypeScript Setup](tools/workspace-typescript.md)** - Configure for TypeScript projects
- **[Rust Setup](tools/workspace-rust.md)** - Configure for Rust projects
- **[Python Setup](tools/workspace-python.md)** - Configure for Python projects

---

### 💻 Contributors

**Getting started with development:**
1. **[DEVELOPMENT.md](DEVELOPMENT.md)** - Complete development guide
2. **[development/testing.md](development/testing.md)** - Testing architecture
3. **[development/logging_guidelines.md](development/logging_guidelines.md)** - Structured logging

**Architecture deep-dive:**
- **[architecture/overview.md](architecture/overview.md)** - System architecture
- **[architecture/api_contracts.md](architecture/api_contracts.md)** - Handler contracts
- **[architecture/lang_common_api.md](architecture/lang_common_api.md)** - Language abstraction

**For contributing workflow:** See [contributing.md](https://github.com/goobits/typemill/blob/main/contributing.md) on GitHub

---

### 🔧 Operators

**Deployment & operations:**
1. **[operations/docker_deployment.md](operations/docker_deployment.md)** - Docker deployment guide
2. **[operations/cache_configuration.md](operations/cache_configuration.md)** - Performance tuning
3. **[operations/cicd.md](operations/cicd.md)** - CI/CD integration

**Monitoring:**
- **[development/logging_guidelines.md](development/logging_guidelines.md)** - Production logging

---

### 🏗️ Architects

**Understanding internals:**
1. **[architecture/overview.md](architecture/overview.md)** - Complete system design
2. **[architecture/layers.md](architecture/layers.md)** - Layer architecture
3. **[architecture/internal_tools.md](architecture/internal_tools.md)** - Tool visibility policy
4. **[architecture/primitives.md](architecture/primitives.md)** - Core data structures
5. **[architecture/api_contracts.md](architecture/api_contracts.md)** - Handler patterns

---

## 📖 Complete Documentation Index

### User Guides
- **[quickstart.md](quickstart.md)** - 5-minute setup guide
- **[cheatsheet.md](cheatsheet.md)** - Command quick reference

### Tool Reference (28 tools)
- **[tools/README.md](tools/README.md)** - Complete tool catalog
- **[tools/navigation.md](tools/navigation.md)** - Navigation & intelligence (8 tools)
- **[tools/refactoring.md](tools/refactoring.md)** - Editing & refactoring (7 tools)
- **[tools/analysis.md](tools/analysis.md)** - Code analysis (8 tools)
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
- **[architecture/overview.md](architecture/overview.md)** - System architecture & data flow
- **[architecture/api_contracts.md](architecture/api_contracts.md)** - Unified Analysis & Refactoring API
- **[architecture/internal_tools.md](architecture/internal_tools.md)** - Public vs internal tools policy
- **[architecture/lang_common_api.md](architecture/lang_common_api.md)** - Language plugin common API
- **[architecture/layers.md](architecture/layers.md)** - Architectural layers
- **[architecture/primitives.md](architecture/primitives.md)** - Code primitives foundation
- **[architecture/tools_visibility_spec.md](architecture/tools_visibility_spec.md)** - Tools visibility specification

### Features
- **[features/actionable_suggestions.md](features/actionable_suggestions.md)** - Actionable suggestions system

### Guides
- **[guides/plugin-migration.md](guides/plugin-migration.md)** - Plugin refactoring migration guide

---

## 💡 Tips for CLI Users

**Viewing documentation:**
```bash
mill docs                              # List all topics
mill docs quickstart                   # View quickstart guide
mill docs tools/refactoring            # View refactoring tools
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
| **[quickstart](quickstart.md)** | 5-min setup | New users | `mill docs quickstart` |
| **[cheatsheet](cheatsheet.md)** | Command reference | All users | `mill docs cheatsheet` |
| **[tools/](tools/)** | Tool API reference | Integrators | `mill docs tools` |
| **[DEVELOPMENT](DEVELOPMENT.md)** | Plugin development | Contributors | `mill docs DEVELOPMENT` |
| **[architecture/overview](architecture/overview.md)** | System design | Architects | `mill docs architecture/overview` |
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

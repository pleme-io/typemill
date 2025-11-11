# TypeMill Architecture

> **Quick navigation for understanding TypeMill's system design**

TypeMill is a Pure Rust MCP server that bridges Language Server Protocol (LSP) functionality to AI coding assistants. The architecture is organized around **29 public tools** built on a layered, service-oriented design.

---

## Start Here

### For Understanding the System

**1. [core-concepts.md](core-concepts.md)** - System architecture fundamentals
- High-level crate structure and dependencies
- 7-layer architectural model with enforcement
- Code primitives framework (refactoring + analysis pillars)
- Request lifecycle and data flow

**2. [specifications.md](specifications.md)** - API contracts and tool visibility
- Public vs internal tools (29 public, 20 internal)
- Unified Analysis API contracts (request/response formats)
- Unified Refactoring API with dryRun pattern
- Error codes and validation rules

### For Implementing Features

**3. [lang_common_api.md](lang_common_api.md)** - Language plugin development
- LanguagePlugin trait implementation
- Import analysis, symbol extraction
- Language-specific capabilities
- Plugin registration and discovery

**4. [internal_tools.md](internal_tools.md)** - Internal tool reference
- Backend-only tools (lifecycle, editing, workspace)
- LSP plumbing (completions, signature help)
- Legacy operations and migration paths

---

## Architecture at a Glance

### Layer Model (7 Layers)
```
Application    → Entry points (CLI, servers)
Handlers       → MCP tool implementations (29 public)
Services       → Business logic, LSP integration
Language       → Language-specific plugins
Plugin API     → Plugin trait contracts
Foundation     → Core types, protocol, config
Support        → Testing, tooling
```

**Rule**: Each layer can only depend on layers below it (enforced by `cargo-deny`).

### Tool Organization

**29 Public MCP Tools:**
- **Navigation** (8) - find_definition, find_references, search_symbols, etc.
- **Refactoring** (7) - rename, extract, inline, move, reorder, transform, delete
- **Analysis** (9) - analyze.quality, analyze.dead_code, analyze.dependencies, etc.
- **Workspace** (4) - create_package, extract_dependencies, update_members, find_replace
- **System** (1) - health_check

**20 Internal Tools** - Backend plumbing, LSP integration, legacy operations

See [specifications.md#tools-visibility](specifications.md#tools-visibility-specification) for complete lists.

### Code Primitives Framework

TypeMill is built on **two pillars**:

1. **Refactoring Primitives** - Atomic code transformations (rename, extract, inline, move, etc.)
2. **Analysis Primitives** - Code understanding (quality, dead code, dependencies, structure, etc.)

**Analysis informs refactoring, refactoring builds on analysis.**

See [core-concepts.md#code-primitives](core-concepts.md#code-primitives-framework) for details.

---

## Request Flow

```
Transport (stdio/WebSocket)
  ↓
JSON parsing → McpMessage
  ↓
PluginDispatcher → Tool lookup
  ↓
Handler execution → Services (LSP, AST, etc.)
  ↓
Response → JSON → Transport
```

**Key components:**
- **mill-server** - Transport, routing, state management
- **mill-handlers** - MCP tool implementations
- **mill-services** - LSP integration, AST caching, refactoring engine
- **mill-lsp** - Language server client abstraction
- **mill-ast** - Language plugin coordination

---

## Key Design Principles

### Safety First (dryRun Pattern)
All refactoring tools default to preview mode (`dryRun: true`). Execution requires explicit opt-in (`dryRun: false`).

### Unified APIs
- Refactoring tools share common structure: target → operation → dryRun → plan/apply
- Analysis tools share common envelope: category → kind → scope → findings

### Plugin Architecture
Language support is modular. Each language gets its own plugin (mill-lang-rust, mill-lang-typescript, etc.).

### Layer Enforcement
Dependencies between crates are validated by `cargo-deny` to prevent circular dependencies and maintain clean architecture.

---

## Where to Learn More

**Understanding the system:**
- [core-concepts.md](core-concepts.md) - Architecture deep-dive
- [specifications.md](specifications.md) - API contracts

**Implementing features:**
- [../tools/](../tools/) - User-facing tool documentation
- [lang_common_api.md](lang_common_api.md) - Plugin development
- [../../contributing.md](../../contributing.md) - Development guide

**Operational details:**
- [../operations/](../operations/) - Deployment, caching, CI/CD
- [../development/](../development/) - Testing, logging guidelines

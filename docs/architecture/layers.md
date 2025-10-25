# Architectural Layers

This document defines the layered dependency model for the TypeMill/TypeMill workspace. These layers are programmatically enforced using `cargo-deny` to prevent "spider web" dependencies and maintain clean architecture.

## Layer Hierarchy

Layers are organized from foundational (bottom) to application (top). Each layer can only depend on layers below it, never above.

```
┌─────────────────────────────────────────┐
│  Layer 7: Application                   │  ← Entry points, CLI, servers
├─────────────────────────────────────────┤
│  Layer 6: Handlers                      │  ← MCP tool handlers
├─────────────────────────────────────────┤
│  Layer 5: Services                      │  ← Business logic, LSP integration
├─────────────────────────────────────────┤
│  Layer 4: Language Plugins              │  ← Language-specific implementations
├─────────────────────────────────────────┤
│  Layer 3: Plugin API                    │  ← Plugin trait definitions
├─────────────────────────────────────────┤
│  Layer 2: Foundation                    │  ← Core types, protocol, config
├─────────────────────────────────────────┤
│  Layer 1: Support (special)             │  ← Testing, tooling (can access any layer)
└─────────────────────────────────────────┘
```

## Layer Definitions

### Layer 1: Support (Special Status)

**Purpose:** Testing infrastructure, build tooling, and analysis tools

**Crates:**
- `mill-test-support` / `mill-test-support`
- `xtask`
- `analysis/*` (mill-analysis-*)

**Dependencies:** Can access any layer (testing needs)

**Rationale:** Test and tooling crates need broad access to verify system behavior. They are never depended upon by production code.

---

### Layer 2: Foundation

**Purpose:** Core data structures, protocol definitions, and configuration

**Crates:**
- `cb-types` / `mill-types`
- `cb-protocol` / `mill-protocol`
- `mill-config`
- `mill-core` (configuration, logging, errors)

**Dependencies:**
- External crates only (serde, tokio, etc.)
- No workspace crate dependencies

**Constraints:**
- No upward dependencies
- Minimal external dependencies
- Stable APIs (changes ripple through entire codebase)

**Planned Consolidation:**
- **Target:** Merge `cb-types`, `cb-protocol`, `mill-core` → `mill-foundation`
- **Rationale:** These crates are tightly coupled and rarely modified independently

---

### Layer 3: Plugin API

**Purpose:** Define language plugin trait and capabilities

**Crates:**
- `mill-plugin-api` / `mill-plugin-api`

**Dependencies:**
- Layer 2: Foundation (types, protocol)

**Constraints:**
- Must remain stable (external plugins depend on this)
- No dependencies on language implementations
- No dependencies on services or handlers

---

### Layer 4: Language Plugins

**Purpose:** Language-specific implementations of code intelligence

**Crates:**
- `mill-lang-common` / `mill-lang-common` (shared utilities)
- `cb-lang-rust` / `mill-lang-rust`
- `mill-lang-typescript` / `mill-lang-typescript`
- `cb-lang-markdown` / `mill-lang-markdown`
- `mill-lang-toml` / `mill-lang-toml`
- `cb-lang-yaml` / `mill-lang-yaml`

**Dependencies:**
- Layer 3: Plugin API
- Layer 2: Foundation

**Constraints:**
- Plugins are independent (no cross-plugin dependencies)
- Each plugin only depends on plugin-api and foundation
- `mill-lang-common` can be used by any plugin for shared utilities

---

### Layer 5: Services

**Purpose:** Core business logic, file operations, AST processing, LSP integration

**Crates:**
- `cb-ast` / `mill-ast` (AST parsing, code analysis)
- `mill-services` / `mill-services` (file service, lock manager, planner)
- `mill-lsp` / `mill-lsp` (LSP client management)
- `mill-plugin-bundle` (plugin registration)
- `mill-plugin-system` (plugin loading, dispatch)

**Dependencies:**
- Layer 4: Language Plugins
- Layer 3: Plugin API
- Layer 2: Foundation

**Constraints:**
- Services coordinate plugins but don't implement language logic
- No dependencies on handlers or application layer
- Services can depend on each other within this layer (mill-services may use cb-ast)

**Planned Consolidation:**
- **Target:** Merge plugin-related crates → `mill-plugin-system`
- **Rationale:** Plugin loading and dispatch are tightly coupled

---

### Layer 6: Handlers

**Purpose:** MCP tool implementations that delegate to services

**Crates:**
- `mill-handlers` / `mill-handlers`

**Dependencies:**
- Layer 5: Services
- Layer 4: Language Plugins (for thin delegating handlers)
- Layer 3: Plugin API
- Layer 2: Foundation

**Constraints:**
- Handlers are thin adapters (business logic belongs in services)
- No upward dependencies
- May directly access language plugins for simple delegation

---

### Layer 7: Application

**Purpose:** Server, client, transport, and CLI entry points

**Crates:**
- `mill-server` / `mill-server` (MCP server orchestration)
- `mill-client` / `mill-client` (CLI client, WebSocket client)
- `mill-transport` / `mill-transport` (stdio, WebSocket protocols)
- `mill-auth` (authentication)
- `mill-workspaces` (workspace management)

**Dependencies:**
- Layer 6: Handlers
- Layer 5: Services
- Layer 3: Plugin API (for registration)
- Layer 2: Foundation

**Constraints:**
- Top-level wiring and initialization only
- No business logic (delegate to services/handlers)
- Entry points for binaries

---

## Dependency Rules

### ✅ Allowed Dependencies

1. **Downward only:** Higher layers depend on lower layers
2. **Same layer:** Within Layer 5 (services can depend on each other)
3. **Support access:** Layer 1 (test/tooling) can access any layer

### ❌ Forbidden Dependencies

1. **Upward:** Lower layers depending on higher layers
2. **Cross-plugin:** Language plugins depending on each other
3. **Handler bypass:** Application layer bypassing handlers to call services directly
4. **Production depends on test:** Any production crate depending on `mill-test-support`

## Enforcement

The `deny.toml` configuration enforces architectural boundaries programmatically using cargo-deny's bans feature.

### Validation Commands

```bash
# Check architectural violations
cargo deny check bans

# Check all (advisories, licenses, bans, sources)
cargo deny check

# Visualize dependency graph
cargo depgraph --workspace-only | dot -Tpng > deps.png
```

### What's Enforced

**✅ Active Enforcement:**
- Plugins (Layer 4) cannot depend on Services (Layer 5) or higher
- Cross-plugin isolation - plugins cannot depend on each other (except via mill-lang-common)
- Services (Layer 5) cannot depend on Handlers (Layer 6) or higher
- Handlers (Layer 6) cannot depend on Application (Layer 7)
- Production crates cannot depend on mill-test-support
- Analysis crates remain isolated (only mill-handlers can optionally use them via features)

**⚠️ Not Enforced by cargo-deny:**
- Foundation (Layer 2) isolation - Manually verify mill-foundation only depends on external crates
- Review Cargo.toml changes to foundation carefully

### Status

- ✅ **Phase 06a COMPLETE** (2025-10-19)
- Architectural enforcement rules enabled and passing
- Updated for post-consolidation crate structure

## Migration Plan (Proposal 06)

This layered architecture will be fully realized through Proposal 06's consolidation and standardization phases.

## Benefits

### For Developers
- **Clear mental model:** Know where to add code
- **Reduced coupling:** Changes are more localized
- **Easier debugging:** Dependency direction is predictable

### For Architecture
- **Prevents rot:** Automatic detection of violations
- **Scalable:** New crates fit into clear layers
- **Modular:** Layers can be versioned independently

### For Testing
- **Isolated testing:** Lower layers test without mocking upper layers
- **Clear boundaries:** Test one layer at a time

## References

- **Proposal 06:** Workspace Consolidation & Architectural Hardening
- **deny.toml:** Programmatic enforcement configuration
- **Architecture Overview:** [overview.md](overview.md)

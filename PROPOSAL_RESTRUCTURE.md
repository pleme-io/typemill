# Proposal: A Unified Crate & Repository Restructure

This document presents a consolidated, phased approach to refactoring the workspace. It integrates the previous plan (focused on splitting `cb-server` and file organization) with our recent discussion on refining foundational crates like `cb-core` and `cb-api`.

## Problem Statement

The current structure has several areas for improvement:
1.  **Monolithic Crates:** `cb-core` and `cb-server` have grown too large, mixing multiple distinct responsibilities.
2.  **Unclear API Boundaries:** The purpose of `cb-api` is ambiguous, and the plugin API is not formally defined.
3.  **Repository Clutter:** The root directory contains numerous documentation and script files that could be better organized.
4.  **Inconsistent Crate Size:** Some crates are extremely small (`cb-api`, `cb-mcp-proxy`), suggesting they could be merged or their purpose re-evaluated.

## The "Flat & Focused" Guiding Principles

This refactor will follow a clear philosophy:
1.  **Maintain a Flat Crate Directory:** Keep the `crates/` directory structure flat for easy scanning.
2.  **Increase Crate Focus:** Decompose large crates into smaller, single-responsibility components.
3.  **Clarify Naming:** Use descriptive names to make the purpose of each crate obvious.
4.  **Formalize Contracts:** Use dedicated API crates to define clear contracts between components.

---

## The Four-Phase Refactoring Plan

### Phase 1: Project Organization & Cleanup (Low Risk) ✅ COMPLETED

This phase, adapted from the original proposal, focuses on improving the repository layout without changing core logic.

*   ~~**Action:** Move all root-level documentation (`.md` files) into a `docs/proposals/` or similar subdirectory.~~ **SKIPPED** (Deferred for now)
*   ✅ **Action:** Consolidate all shell, Python, and Go scripts into the top-level `scripts/` directory.
*   ✅ **Action:** Move test fixtures from `tests/fixtures` to `integration-tests/test-fixtures` to centralize test data.
*   ✅ **Action:** Remove old empty directories (`tests`, `deployment/scripts`, `crates/cb-ast/resources`)
*   **Benefit:** A clean, uncluttered root directory.

### Phase 2: Foundational Crate Refactoring (High Value)

This phase implements the "Flat & Focused" structure for our foundational crates. This is a prerequisite for later phases.

*   **Action:** **Split `cb-core`** into three focused crates:
    1.  **`cb-types` (New):** A new, logic-free crate for shared data structures (`models`) and `error` types. It will be a lightweight, stable foundation for the entire workspace.
    2.  **`cb-plugin-api` (New):** A new crate to formally define the plugin contract (traits and types).
    3.  **`cb-core` (Edited):** A shrunken, logic-only crate that orchestrates core functionality.
*   **Action:** **Rename `cb-api` to `cb-protocol`**.
    *   **Rationale:** This clarifies its specific purpose as the client-server network communication contract, distinguishing it from the new `cb-plugin-api`.
*   **Benefit:** Establishes clear, logical boundaries at the core of the application, making all subsequent development easier.

### Phase 3: Deconstruct `cb-server`

This phase, adapted from the original proposal, breaks down the monolithic `cb-server`.

*   **Action:** Create three new crates from `cb-server`'s responsibilities:
    1.  **`cb-lsp` (New):** For all Language Server Protocol-related logic.
    2.  **`cb-services` (New):** For high-level business logic and orchestration services (e.g., `AstService`, `FileService`).
    3.  **`cb-handlers` (New):** For implementing the MCP tool handlers.
*   **Action:** Shrink `cb-server` to be a thin entry point that assembles the server from the above components.
*   **Benefit:** Transforms `cb-server` from a monolith into a well-structured, maintainable application.

### Phase 4: Consolidate `cb-mcp-proxy`

This phase addresses the "tiny crate" problem by merging `cb-mcp-proxy` where it logically belongs.

*   **Action:** Merge the `cb-mcp-proxy` crate into `cb-plugins`.
    *   **Rationale:** The MCP proxy can be viewed as a built-in, specialized plugin. Its logic fits naturally within the `cb-plugins` crate.
*   **Contradiction with Original Plan:** We will **not** merge `cb-protocol` (formerly `cb-api`) into `cb-core`. Keeping it separate is crucial for maintaining a clean boundary between the core application logic and the network communication contract.
*   **Benefit:** Reduces crate overhead while improving the logical cohesion of the `cb-plugins` crate.

---

## Final Proposed Structure

After all four phases, the `crates/` directory will look like this:

```
/
└── crates/
    ├── cb-core/          # EDITED: Lean business logic orchestrator
    ├── cb-types/         # NEW: Foundational structs, models, and errors
    ├── cb-protocol/      # RENAMED from cb-api
    ├── cb-plugin-api/    # NEW: Formal plugin interface
    │
    ├── cb-ast/           # UNCHANGED
    ├── cb-client/        # UNCHANGED
    ├── cb-transport/     # UNCHANGED
    │
    ├── cb-server/        # EDITED: Slimmed down entry point
    ├── cb-lsp/           # NEW: Split from cb-server
    ├── cb-services/      # NEW: Split from cb-server
    ├── cb-handlers/      # NEW: Split from cb-server
    │
    ├── cb-plugins/       # EDITED: Absorbed cb-mcp-proxy
    └── cb-mcp-proxy/     # REMOVED
```

This unified proposal creates a highly modular, maintainable, and logically sound architecture that addresses all identified issues.

---
---

## Implementation with `codebuddy` Tool Calls

Here are the actionable commands to execute the four-phase refactoring plan.

### Phase 1: Project Organization & Cleanup

```bash
# 1a: Move root documentation into docs/proposals
codebuddy tool batch_execute '{ 
  "operations": [
    {"type": "create_directory", "path": "docs/proposals"},
    {"type": "rename_file", "old_path": "BUG_REPORT.md", "new_path": "docs/proposals/BUG_REPORT.md"},
    {"type": "rename_file", "old_path": "CHANGELOG.md", "new_path": "docs/proposals/CHANGELOG.md"},
    {"type": "rename_file", "old_path": "CLAUDE.md", "new_path": "docs/proposals/CLAUDE.md"},
    {"type": "rename_file", "old_path": "MCP_API.md", "new_path": "docs/proposals/MCP_API.md"},
    {"type": "rename_file", "old_path": "ROADMAP.md", "new_path": "docs/proposals/ROADMAP.md"},
    {"type": "rename_file", "old_path": "SUPPORT_MATRIX.md", "new_path": "docs/proposals/SUPPORT_MATRIX.md"},
    {"type": "rename_file", "old_path": "PROPOSAL_ADVANCED_ANALYSIS.md", "new_path": "docs/proposals/PROPOSAL_ADVANCED_ANALYSIS.md"},
    {"type": "rename_file", "old_path": "PROPOSAL_BACKEND_ARCHITECTURE.md", "new_path": "docs/proposals/PROPOSAL_BACKEND_ARCHITECTURE.md"},
    {"type": "rename_file", "old_path": "PROPOSAL_HANDLER_ARCHITECTURE.md", "new_path": "docs/proposals/PROPOSAL_HANDLER_ARCHITECTURE.md"},
    {"type": "rename_file", "old_path": "PROPOSAL_RESTRUCTURE.md", "new_path": "docs/proposals/PROPOSAL_RESTRUCTURE.md"}
  ]
}
'

# 1b: Consolidate scripts
codebuddy tool batch_execute '{ 
  "operations": [
    {"type": "create_directory", "path": "scripts"},
    {"type": "rename_file", "old_path": "install.sh", "new_path": "scripts/install.sh"},
    {"type": "rename_file", "old_path": "deployment/scripts/setup-dev-tools.sh", "new_path": "scripts/setup-dev-tools.sh"},
    {"type": "rename_file", "old_path": "crates/cb-ast/resources/ast_tool.py", "new_path": "scripts/ast_tool.py"},
    {"type": "rename_file", "old_path": "crates/cb-ast/resources/ast_tool.go", "new_path": "scripts/ast_tool.go"}
  ]
}
'

# 1c: Consolidate test fixtures
codebuddy tool rename_directory '{"old_path":"tests/fixtures", "new_path":"integration-tests/test-fixtures"}'

# 1d: Manual cleanup
echo "Run 'git rm -r tests deployment/scripts crates/cb-ast/resources' to remove old directories"
```

### Phase 2: Foundational Crate Refactoring

```bash
# 2a: Create new crates cb-types and cb-plugin-api
codebuddy tool batch_execute '{ 
  "operations": [
    {
      "type": "create_file",
      "path": "crates/cb-types/Cargo.toml",
      "content": "[package]\nname = \"cb-types\"\nversion = \"1.0.0-beta\"\nedition = \"2021\"\n\n[dependencies]\nserde = { workspace = true }\nthiserror = { workspace = true }"
    },
    {
      "type": "create_file",
      "path": "crates/cb-types/src/lib.rs",
      "content": "pub mod error;\npub mod model;\n"
    },
    {
      "type": "create_file",
      "path": "crates/cb-plugin-api/Cargo.toml",
      "content": "[package]\nname = \"cb-plugin-api\"\nversion = \"1.0.0-beta\"\nedition = \"2021\"\n\n[dependencies]\ncb-types = { path = \"../cb-types\" }\nasync-trait = { workspace = true }\nserde_json = { workspace = true }"
    },
    {
      "type": "create_file",
      "path": "crates/cb-plugin-api/src/lib.rs",
      "content": "// Plugin traits and types will be defined here.\n"
    }
  ]
}
'

# 2b: Move model and error from cb-core to cb-types
codebuddy tool batch_execute '{ 
  "operations": [
    {"type": "rename_file", "old_path": "crates/cb-core/src/model", "new_path": "crates/cb-types/src/model"},
    {"type": "rename_file", "old_path": "crates/cb-core/src/error.rs", "new_path": "crates/cb-types/src/error.rs"}
  ]
}
'

# 2c: Rename cb-api to cb-protocol
codebuddy tool rename_directory '{"old_path":"crates/cb-api", "new_path":"crates/cb-protocol"}'

# 2d: Manual updates
echo "1. Add '\"crates/cb-types\"', '\"crates/cb-plugin-api\"' to workspace members in root Cargo.toml"
echo "2. Remove '\"crates/cb-api\"' from workspace members and add '\"crates/cb-protocol\"'"
echo "3. Update dependencies in all Cargo.toml files (replace cb-api with cb-protocol, add cb-types where needed)"
echo "4. Fix 'use' statements across the workspace"
```

### Phase 3: Deconstruct `cb-server`

```bash
# 3a: Create new crates cb-lsp, cb-services, cb-handlers (taken from original proposal)
codebuddy tool batch_execute '{"operations": [
    {"type": "create_file", "path": "crates/cb-lsp/Cargo.toml", "content": "[package]\nname = \"cb-lsp\"\nversion = \"1.0.0-beta\"\nedition = \"2021\"\n[dependencies]\ncb-core = { path = \"../cb-core\" }\ntokio = { workspace = true }\nserde_json = { workspace = true }\nlsp-types = \"0.97\""},
    {"type": "create_file", "path": "crates/cb-lsp/src/lib.rs", "content": "pub mod client;"},
    {"type": "create_file", "path": "crates/cb-services/Cargo.toml", "content": "[package]\nname = \"cb-services\"\nversion = \"1.0.0-beta\"\nedition = \"2021\"\n[dependencies]\ncb-types = { path = \"../cb-types\" }\ncb-core = { path = \"../cb-core\" }\ncb-ast = { path = \"../cb-ast\" }\ncb-lsp = { path = \"../cb-lsp\" }\nanyhow = { workspace = true }"},
    {"type": "create_file", "path": "crates/cb-services/src/lib.rs", "content": "pub mod ast_service;\npub mod file_service;"},
    {"type": "create_file", "path": "crates/cb-handlers/Cargo.toml", "content": "[package]\nname = \"cb-handlers\"\nversion = \"1.0.0-beta\"\nedition = \"2021\"\n[dependencies]\ncb-protocol = { path = \"../cb-protocol\" }\ncb-core = { path = \"../cb-core\" }\ncb-services = { path = \"../cb-services\" }\ncb-plugin-api = { path = \"../cb-plugin-api\" }\nanyhow = { workspace = true }"},
    {"type": "create_file", "path": "crates/cb-handlers/src/lib.rs", "content": "pub mod tools;"}
]}'

# 3b: Move code from cb-server to new crates (example)
codebuddy tool batch_execute '{ 
  "operations": [
    {"type": "rename_file", "old_path": "crates/cb-server/src/systems/lsp", "new_path": "crates/cb-lsp/src/lsp_system"},
    {"type": "rename_file", "old_path": "crates/cb-server/src/services", "new_path": "crates/cb-services/src/services"},
    {"type": "rename_file", "old_path": "crates/cb-server/src/handlers", "new_path": "crates/cb-handlers/src/handlers"}
  ]
}
'

# 3c: Manual updates
echo "1. Add '\"crates/cb-lsp\"', '\"crates/cb-services\"', '\"crates/cb-handlers\"' to workspace members in root Cargo.toml"
echo "2. Add new dependencies to crates/cb-server/Cargo.toml"
echo "3. Fix 'use' statements across the workspace"
```

### Phase 4: Consolidate `cb-mcp-proxy`

```bash
# 4a: Move files from cb-mcp-proxy to cb-plugins/src/mcp
codebuddy tool batch_execute '{ 
  "operations": [
    {"type": "create_directory", "path": "crates/cb-plugins/src/mcp"},
    {"type": "rename_file", "old_path": "crates/cb-mcp-proxy/src/lib.rs", "new_path": "crates/cb-plugins/src/mcp/mod.rs"},
    {"type": "rename_file", "old_path": "crates/cb-mcp-proxy/src/client.rs", "new_path": "crates/cb-plugins/src/mcp/client.rs"},
    {"type": "rename_file", "old_path": "crates/cb-mcp-proxy/src/error.rs", "new_path": "crates/cb-plugins/src/mcp/error.rs"},
    {"type": "rename_file", "old_path": "crates/cb-mcp-proxy/src/manager.rs", "new_path": "crates/cb-plugins/src/mcp/manager.rs"},
    {"type": "rename_file", "old_path": "crates/cb-mcp-proxy/src/plugin.rs", "new_path": "crates/cb-plugins/src/mcp/plugin.rs"},
    {"type": "rename_file", "old_path": "crates/cb-mcp-proxy/src/presets.rs", "new_path": "crates/cb-plugins/src/mcp/presets.rs"},
    {"type": "rename_file", "old_path": "crates/cb-mcp-proxy/src/protocol.rs", "new_path": "crates/cb-plugins/src/mcp/protocol.rs"}
  ]
}
'

# 4b: Manual updates
echo "1. Remove '\"crates/cb-mcp-proxy\"' from workspace members in root Cargo.toml"
echo "2. Update crates/cb-plugins/src/lib.rs to include 'pub mod mcp;'"
echo "3. Remove dependency on cb-mcp-proxy from other Cargo.toml files"
echo "4. Run 'git rm -r crates/cb-mcp-proxy'"
```
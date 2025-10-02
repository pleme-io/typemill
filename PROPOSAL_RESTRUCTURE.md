# Repository Restructure Proposal

## Problem Statement

1. **cb-server is too large** (9,679 lines, 33 files) with 3 distinct concerns mixed together
2. **Tiny crates add overhead** (cb-api: 2 files, 709 lines)
3. **Root directory cluttered** with docs/proposals that should be organized
4. **Handler refactor incomplete** (tools/ modules half-done, old handlers remain)

## Solution: Hybrid Approach with batch_execute

Use codebuddy's `batch_execute` MCP tool to perform atomic batch operations.

---

## Phase 1: Organization (Low Risk)

### Phase 1a: Move Root Documentation

```bash
codebuddy call batch_execute --arguments '{
  "operations": [
    {
      "type": "create_file",
      "path": "docs/project/.gitkeep",
      "content": ""
    },
    {
      "type": "create_file",
      "path": "scripts/.gitkeep",
      "content": ""
    },
    {
      "type": "rename_file",
      "old_path": "BUG_REPORT.md",
      "new_path": "docs/project/BUG_REPORT.md"
    },
    {
      "type": "rename_file",
      "old_path": "CHANGELOG.md",
      "new_path": "docs/project/CHANGELOG.md"
    },
    {
      "type": "rename_file",
      "old_path": "CLAUDE.md",
      "new_path": "docs/project/CLAUDE.md"
    },
    {
      "type": "rename_file",
      "old_path": "MCP_API.md",
      "new_path": "docs/project/MCP_API.md"
    },
    {
      "type": "rename_file",
      "old_path": "ROADMAP.md",
      "new_path": "docs/project/ROADMAP.md"
    },
    {
      "type": "rename_file",
      "old_path": "SUPPORT_MATRIX.md",
      "new_path": "docs/project/SUPPORT_MATRIX.md"
    },
    {
      "type": "rename_file",
      "old_path": "PROPOSAL_ADVANCED_ANALYSIS.md",
      "new_path": "docs/project/PROPOSAL_ADVANCED_ANALYSIS.md"
    },
    {
      "type": "rename_file",
      "old_path": "PROPOSAL_BACKEND_ARCHITECTURE.md",
      "new_path": "docs/project/PROPOSAL_BACKEND_ARCHITECTURE.md"
    },
    {
      "type": "rename_file",
      "old_path": "PROPOSAL_HANDLER_ARCHITECTURE.md",
      "new_path": "docs/project/PROPOSAL_HANDLER_ARCHITECTURE.md"
    },
    {
      "type": "rename_file",
      "old_path": "PROPOSAL_RESTRUCTURE.md",
      "new_path": "docs/project/PROPOSAL_RESTRUCTURE.md"
    }
  ]
}'
```

### Phase 1b: Flatten Documentation Structure

```bash
codebuddy call batch_execute --arguments '{
  "operations": [
    {
      "type": "rename_file",
      "old_path": "docs/architecture/ARCHITECTURE.md",
      "new_path": "docs/ARCHITECTURE.md"
    },
    {
      "type": "rename_file",
      "old_path": "docs/architecture/contracts.md",
      "new_path": "docs/CONTRACTS.md"
    },
    {
      "type": "rename_file",
      "old_path": "docs/deployment/OPERATIONS.md",
      "new_path": "docs/DEPLOYMENT.md"
    },
    {
      "type": "rename_file",
      "old_path": "docs/deployment/USAGE.md",
      "new_path": "docs/USAGE.md"
    },
    {
      "type": "rename_file",
      "old_path": "docs/development/LOGGING_GUIDELINES.md",
      "new_path": "docs/LOGGING.md"
    },
    {
      "type": "rename_file",
      "old_path": "docs/features/WORKFLOWS.md",
      "new_path": "docs/WORKFLOWS.md"
    }
  ]
}'
```

### Phase 1c: Consolidate Scripts

```bash
codebuddy call batch_execute --arguments '{
  "operations": [
    {
      "type": "rename_file",
      "old_path": "install.sh",
      "new_path": "scripts/install.sh"
    },
    {
      "type": "rename_file",
      "old_path": "deployment/scripts/setup-dev-tools.sh",
      "new_path": "scripts/setup-dev-tools.sh"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-ast/resources/ast_tool.py",
      "new_path": "scripts/ast_tool.py"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-ast/resources/ast_tool.go",
      "new_path": "scripts/ast_tool.go"
    }
  ]
}'
```

### Phase 1d: Move Test Fixtures (Individual calls - batch_execute doesn't support rename_directory)

```bash
codebuddy call rename_directory --arguments '{"old_path":"tests/fixtures/atomic-refactoring-test","new_path":"integration-tests/fixtures/atomic-refactoring-test"}'
codebuddy call rename_directory --arguments '{"old_path":"tests/fixtures/python","new_path":"integration-tests/fixtures/python"}'
codebuddy call rename_directory --arguments '{"old_path":"tests/fixtures/rust","new_path":"integration-tests/fixtures/rust"}'
codebuddy call rename_directory --arguments '{"old_path":"tests/fixtures/src","new_path":"integration-tests/fixtures/src"}'
codebuddy call rename_directory --arguments '{"old_path":"tests/fixtures/test-workspace-symbols","new_path":"integration-tests/fixtures/test-workspace-symbols"}'
```

### Phase 1e: Cleanup Empty Directories

```bash
# Manual cleanup (git commands)
git rm -r docs/architecture docs/deployment docs/development docs/features
git rm -r deployment/scripts crates/cb-ast/resources tests

# Verify
cargo check --workspace
git status
git add . && git commit -m "Phase 1: Reorganize documentation and scripts"
```

**Phase 1 Summary**: 3 batch operations (22 files) + 5 directory moves + cleanup

---

## Phase 2: Split cb-server (High Value)

### Phase 2a: Create New Crate Structures

```bash
# Create cb-lsp crate
codebuddy call batch_execute --arguments '{
  "operations": [
    {
      "type": "create_file",
      "path": "crates/cb-lsp/Cargo.toml",
      "content": "[package]\nname = \"cb-lsp\"\nversion = \"1.0.0-beta\"\nedition = \"2021\"\ndescription = \"LSP protocol adapter for Codeflow Buddy\"\nlicense = \"MIT\"\n\n[dependencies]\ncb-core = { path = \"../cb-core\" }\ntokio = { workspace = true }\nasync-trait = { workspace = true }\nserde = { workspace = true }\nserde_json = { workspace = true }\ntracing = { workspace = true }\nlsp-types = \"0.97\"\ndashmap = { workspace = true }\n"
    },
    {
      "type": "create_file",
      "path": "crates/cb-lsp/src/lib.rs",
      "content": "//! LSP protocol adapter for Codeflow Buddy\n\npub mod client;\n\npub use client::LspClient;\n"
    },
    {
      "type": "create_file",
      "path": "crates/cb-services/Cargo.toml",
      "content": "[package]\nname = \"cb-services\"\nversion = \"1.0.0-beta\"\nedition = \"2021\"\ndescription = \"Business logic and orchestration services for Codeflow Buddy\"\nlicense = \"MIT\"\n\n[dependencies]\ncb-api = { path = \"../cb-api\" }\ncb-core = { path = \"../cb-core\" }\ncb-ast = { path = \"../cb-ast\" }\ncb-lsp = { path = \"../cb-lsp\" }\ntokio = { workspace = true }\nasync-trait = { workspace = true }\nserde = { workspace = true }\nserde_json = { workspace = true }\ntracing = { workspace = true }\nanyhow = { workspace = true }\nthiserror = { workspace = true }\ndashmap = { workspace = true }\nignore = \"0.4\"\n\n[dev-dependencies]\ntempfile = \"3.0\"\n"
    },
    {
      "type": "create_file",
      "path": "crates/cb-services/src/lib.rs",
      "content": "//! Services for coordinating complex operations\n\npub mod ast_service;\npub mod file_service;\npub mod import_service;\npub mod lock_manager;\npub mod operation_queue;\npub mod planner;\npub mod workflow_executor;\n\n#[cfg(test)]\npub mod tests;\n\npub use ast_service::DefaultAstService;\npub use file_service::FileService;\npub use import_service::ImportService;\npub use lock_manager::{LockManager, LockType};\npub use operation_queue::{FileOperation, OperationQueue, OperationType, QueueStats};\n"
    },
    {
      "type": "create_file",
      "path": "crates/cb-handlers/Cargo.toml",
      "content": "[package]\nname = \"cb-handlers\"\nversion = \"1.0.0-beta\"\nedition = \"2021\"\ndescription = \"MCP tool handler implementations for Codeflow Buddy\"\nlicense = \"MIT\"\n\n[dependencies]\ncb-api = { path = \"../cb-api\" }\ncb-core = { path = \"../cb-core\" }\ncb-ast = { path = \"../cb-ast\" }\ncb-lsp = { path = \"../cb-lsp\" }\ncb-services = { path = \"../cb-services\" }\ncb-plugins = { path = \"../cb-plugins\" }\ntokio = { workspace = true }\nasync-trait = { workspace = true }\nserde = { workspace = true }\nserde_json = { workspace = true }\ntracing = { workspace = true }\nanyhow = { workspace = true }\nthiserror = { workspace = true }\nlsp-types = \"0.97\"\nuuid = { version = \"1.0\", features = [\"v4\"] }\n"
    },
    {
      "type": "create_file",
      "path": "crates/cb-handlers/src/lib.rs",
      "content": "//! MCP tool handler implementations\n\npub mod plugin_dispatcher;\npub mod tool_registry;\npub mod tools;\n\npub use plugin_dispatcher::PluginDispatcher;\npub use tool_registry::ToolRegistry;\n"
    },
    {
      "type": "create_file",
      "path": "crates/cb-handlers/src/tools/mod.rs",
      "content": "//! MCP tool implementations organized by domain\n\npub mod advanced;\npub mod editing;\npub mod file_ops;\npub mod lifecycle;\npub mod navigation;\npub mod system;\npub mod workspace;\n"
    }
  ]
}'
```

### Phase 2b: Move LSP & Service Code

```bash
codebuddy call batch_execute --arguments '{
  "operations": [
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/systems/lsp/client.rs",
      "new_path": "crates/cb-lsp/src/client.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/services/ast_service.rs",
      "new_path": "crates/cb-services/src/ast_service.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/services/file_service.rs",
      "new_path": "crates/cb-services/src/file_service.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/services/import_service.rs",
      "new_path": "crates/cb-services/src/import_service.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/services/lock_manager.rs",
      "new_path": "crates/cb-services/src/lock_manager.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/services/operation_queue.rs",
      "new_path": "crates/cb-services/src/operation_queue.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/services/planner.rs",
      "new_path": "crates/cb-services/src/planner.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/services/workflow_executor.rs",
      "new_path": "crates/cb-services/src/workflow_executor.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/services/tests.rs",
      "new_path": "crates/cb-services/src/tests.rs"
    }
  ]
}'
```

### Phase 2c: Move Handler Code

```bash
codebuddy call batch_execute --arguments '{
  "operations": [
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/handlers/plugin_dispatcher.rs",
      "new_path": "crates/cb-handlers/src/plugin_dispatcher.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/handlers/tool_registry.rs",
      "new_path": "crates/cb-handlers/src/tool_registry.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/handlers/tools/advanced.rs",
      "new_path": "crates/cb-handlers/src/tools/advanced.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/handlers/tools/editing.rs",
      "new_path": "crates/cb-handlers/src/tools/editing.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/handlers/tools/file_ops.rs",
      "new_path": "crates/cb-handlers/src/tools/file_ops.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/handlers/tools/lifecycle.rs",
      "new_path": "crates/cb-handlers/src/tools/lifecycle.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/handlers/tools/navigation.rs",
      "new_path": "crates/cb-handlers/src/tools/navigation.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/handlers/tools/system.rs",
      "new_path": "crates/cb-handlers/src/tools/system.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-server/src/handlers/tools/workspace.rs",
      "new_path": "crates/cb-handlers/src/tools/workspace.rs"
    }
  ]
}'
```

### Phase 2d: Manual Updates

```bash
# Update workspace Cargo.toml - add to [workspace] members:
#   "crates/cb-handlers",
#   "crates/cb-lsp",
#   "crates/cb-services",

# Update crates/cb-server/Cargo.toml - add to [dependencies]:
#   cb-handlers = { path = "../cb-handlers" }
#   cb-lsp = { path = "../cb-lsp" }
#   cb-services = { path = "../cb-services" }

# Fix imports (bulk find-replace)
find crates/cb-services/src -name "*.rs" -exec sed -i 's/crate::services::/cb_services::/g' {} +
find crates/cb-lsp/src -name "*.rs" -exec sed -i 's/crate::systems::lsp::/cb_lsp::/g' {} +
find crates/cb-handlers/src -name "*.rs" -exec sed -i 's/crate::handlers::/cb_handlers::/g' {} +
find crates/cb-handlers/src -name "*.rs" -exec sed -i 's/crate::services::/cb_services::/g' {} +
find crates/cb-handlers/src -name "*.rs" -exec sed -i 's/crate::systems::lsp::/cb_lsp::/g' {} +

# Cleanup old directories
git rm -r crates/cb-server/src/handlers/tools
git rm -r crates/cb-server/src/services
git rm -r crates/cb-server/src/systems
git rm crates/cb-server/src/handlers/file_operation_handler.rs
git rm crates/cb-server/src/handlers/refactoring_handler.rs
git rm crates/cb-server/src/handlers/system_handler.rs
git rm crates/cb-server/src/handlers/tool_handler.rs
git rm crates/cb-server/src/handlers/workflow_handler.rs
git rm crates/cb-server/src/handlers/lsp_adapter.rs

# Verify
cargo check --workspace && cargo test --workspace
git add . && git commit -m "Phase 2: Split cb-server into focused crates"
```

**Phase 2 Summary**: 3 batch operations (25 files) + manual edits + cleanup

---

## Phase 3: Merge Tiny Crates

### Phase 3a: Merge cb-api into cb-core

```bash
codebuddy call batch_execute --arguments '{
  "operations": [
    {
      "type": "create_file",
      "path": "crates/cb-core/src/api/.gitkeep",
      "content": ""
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-api/src/error.rs",
      "new_path": "crates/cb-core/src/api/error.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-api/src/lib.rs",
      "new_path": "crates/cb-core/src/api/mod.rs"
    }
  ]
}'
```

### Phase 3b: Merge cb-mcp-proxy into cb-plugins

```bash
codebuddy call batch_execute --arguments '{
  "operations": [
    {
      "type": "create_file",
      "path": "crates/cb-plugins/src/mcp/.gitkeep",
      "content": ""
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-mcp-proxy/src/client.rs",
      "new_path": "crates/cb-plugins/src/mcp/client.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-mcp-proxy/src/error.rs",
      "new_path": "crates/cb-plugins/src/mcp/error.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-mcp-proxy/src/manager.rs",
      "new_path": "crates/cb-plugins/src/mcp/manager.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-mcp-proxy/src/plugin.rs",
      "new_path": "crates/cb-plugins/src/mcp/plugin.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-mcp-proxy/src/presets.rs",
      "new_path": "crates/cb-plugins/src/mcp/presets.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-mcp-proxy/src/protocol.rs",
      "new_path": "crates/cb-plugins/src/mcp/protocol.rs"
    },
    {
      "type": "rename_file",
      "old_path": "crates/cb-mcp-proxy/src/lib.rs",
      "new_path": "crates/cb-plugins/src/mcp/mod.rs"
    }
  ]
}'
```

### Phase 3c: Manual Updates

```bash
# Update cb-core/src/lib.rs - add at top:
#   pub mod api;
#   pub use api::*;

# Update cb-plugins/src/lib.rs - add at top:
#   pub mod mcp;
#   pub use mcp::*;

# Update workspace Cargo.toml - remove from members:
#   "crates/cb-api",
#   "crates/cb-mcp-proxy",

# Fix all Cargo.toml dependencies
find crates -name "Cargo.toml" -exec sed -i '/cb-api = { path = "..\/cb-api" }/d' {} +
find crates -name "Cargo.toml" -exec sed -i '/cb-mcp-proxy = { path = "..\/cb-mcp-proxy".*}/d' {} +

# Fix all imports
find crates -name "*.rs" -exec sed -i 's/use cb_api::/use cb_core::api::/g' {} +
find crates -name "*.rs" -exec sed -i 's/cb_api::/cb_core::api::/g' {} +
find crates -name "*.rs" -exec sed -i 's/use cb_mcp_proxy::/use cb_plugins::mcp::/g' {} +
find crates -name "*.rs" -exec sed -i 's/cb_mcp_proxy::/cb_plugins::mcp::/g' {} +

# Remove old crate directories
git rm -r crates/cb-api
git rm -r crates/cb-mcp-proxy

# Verify
cargo check --workspace
cargo test --workspace
cargo clippy --workspace
cargo fmt --workspace
git add . && git commit -m "Phase 3: Merge tiny crates into parent crates"
```

**Phase 3 Summary**: 2 batch operations (11 files) + manual edits + cleanup

---

## Execution Summary

**Total batch_execute operations**: 8 batches, 58 file operations
**Total rename_directory operations**: 5 directories
**Manual steps**: Cargo.toml edits, import fixes, cleanups

### Complete Execution Script

```bash
#!/bin/bash
set -e

echo "=== Phase 1: Organization ==="
# Run Phase 1a, 1b, 1c batch_execute commands
# Run Phase 1d rename_directory commands
# Phase 1e cleanup
cargo check --workspace
git commit -m "Phase 1: Reorganize documentation and scripts"

echo "=== Phase 2: Split cb-server ==="
# Run Phase 2a, 2b, 2c batch_execute commands
# Phase 2d manual updates
cargo check --workspace && cargo test --workspace
git commit -m "Phase 2: Split cb-server into focused crates"

echo "=== Phase 3: Merge Tiny Crates ==="
# Run Phase 3a, 3b batch_execute commands
# Phase 3c manual updates
cargo check --workspace && cargo test --workspace && cargo clippy --workspace
git commit -m "Phase 3: Merge tiny crates"

echo "=== Complete ==="
cargo fmt --workspace
```

---

## Final Structure

```
crates/
├── cb-core/              (EXPANDED: +api/)
├── cb-ast/               (UNCHANGED)
├── cb-client/            (UNCHANGED)
├── cb-plugins/           (EXPANDED: +mcp/)
├── cb-transport/         (UNCHANGED)
├── cb-lsp/               (NEW: LSP adapter)
├── cb-services/          (NEW: Business logic)
├── cb-handlers/          (NEW: Tool implementations)
└── cb-server/            (SLIMMED: Entry point)

docs/
├── ARCHITECTURE.md
├── CONTRACTS.md
├── DEPLOYMENT.md
├── LOGGING.md
├── USAGE.md
├── WORKFLOWS.md
└── project/
    └── [project docs]

scripts/
└── [helper scripts]
```

---

## Benefits

- ✅ **Atomic operations**: batch_execute ensures all-or-nothing file moves
- ✅ **Automatic import updates**: rename_file in batch handles imports
- ✅ **Rollback on failure**: Failed operations automatically rollback
- ✅ **Clear separation**: cb-server split into focused crates
- ✅ **Reduced overhead**: Tiny crates merged into logical parents
- ✅ **Clean root**: Documentation and scripts organized

---

## Total Effort

**2-3 hours** with batch operations and verification steps.

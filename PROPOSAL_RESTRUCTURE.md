# Repository Restructure Proposal

## Problem Statement

1. **cb-server is too large** (9,679 lines, 33 files) with 3 distinct concerns mixed together
2. **Tiny crates add overhead** (cb-api: 2 files, 709 lines)
3. **Root directory cluttered** with docs/proposals that should be organized
4. **Handler refactor incomplete** (tools/ modules half-done, old handlers remain)

## Solution: Hybrid Approach

Combine organizational cleanup (low-risk) with strategic crate restructuring (high-value).

---

## Phase 1: Organization (Low Risk)

### Step 1.1: Move Root Documentation to docs/project/

```bash
# Create directory
mkdir -p docs/project

# Use individual rename_file commands
codebuddy call rename_file '{"old_path": "BUG_REPORT.md", "new_path": "docs/project/BUG_REPORT.md"}'
codebuddy call rename_file '{"old_path": "CHANGELOG.md", "new_path": "docs/project/CHANGELOG.md"}'
codebuddy call rename_file '{"old_path": "CLAUDE.md", "new_path": "docs/project/CLAUDE.md"}'
codebuddy call rename_file '{"old_path": "MCP_API.md", "new_path": "docs/project/MCP_API.md"}'
codebuddy call rename_file '{"old_path": "ROADMAP.md", "new_path": "docs/project/ROADMAP.md"}'
codebuddy call rename_file '{"old_path": "SUPPORT_MATRIX.md", "new_path": "docs/project/SUPPORT_MATRIX.md"}'
codebuddy call rename_file '{"old_path": "PROPOSAL_ADVANCED_ANALYSIS.md", "new_path": "docs/project/PROPOSAL_ADVANCED_ANALYSIS.md"}'
codebuddy call rename_file '{"old_path": "PROPOSAL_BACKEND_ARCHITECTURE.md", "new_path": "docs/project/PROPOSAL_BACKEND_ARCHITECTURE.md"}'
codebuddy call rename_file '{"old_path": "PROPOSAL_HANDLER_ARCHITECTURE.md", "new_path": "docs/project/PROPOSAL_HANDLER_ARCHITECTURE.md"}'
codebuddy call rename_file '{"old_path": "PROPOSAL_RESTRUCTURE.md", "new_path": "docs/project/PROPOSAL_RESTRUCTURE.md"}'
```

### Step 1.2: Flatten Documentation Subdirectories

```bash
codebuddy call rename_file '{"old_path": "docs/architecture/ARCHITECTURE.md", "new_path": "docs/ARCHITECTURE.md"}'
codebuddy call rename_file '{"old_path": "docs/architecture/contracts.md", "new_path": "docs/CONTRACTS.md"}'
codebuddy call rename_file '{"old_path": "docs/deployment/OPERATIONS.md", "new_path": "docs/DEPLOYMENT.md"}'
codebuddy call rename_file '{"old_path": "docs/deployment/USAGE.md", "new_path": "docs/USAGE.md"}'
codebuddy call rename_file '{"old_path": "docs/development/LOGGING_GUIDELINES.md", "new_path": "docs/LOGGING.md"}'
codebuddy call rename_file '{"old_path": "docs/features/WORKFLOWS.md", "new_path": "docs/WORKFLOWS.md"}'
```

### Step 1.3: Consolidate Scripts

```bash
# Create scripts directory
mkdir -p scripts

codebuddy call rename_file '{"old_path": "install.sh", "new_path": "scripts/install.sh"}'
codebuddy call rename_file '{"old_path": "deployment/scripts/setup-dev-tools.sh", "new_path": "scripts/setup-dev-tools.sh"}'
codebuddy call rename_file '{"old_path": "crates/cb-ast/resources/ast_tool.py", "new_path": "scripts/ast_tool.py"}'
codebuddy call rename_file '{"old_path": "crates/cb-ast/resources/ast_tool.go", "new_path": "scripts/ast_tool.go"}'
```

### Step 1.4: Consolidate Test Fixtures

```bash
codebuddy call rename_directory '{"old_path": "tests/fixtures/atomic-refactoring-test", "new_path": "integration-tests/fixtures/atomic-refactoring-test"}'
codebuddy call rename_directory '{"old_path": "tests/fixtures/python", "new_path": "integration-tests/fixtures/python"}'
codebuddy call rename_directory '{"old_path": "tests/fixtures/rust", "new_path": "integration-tests/fixtures/rust"}'
codebuddy call rename_directory '{"old_path": "tests/fixtures/src", "new_path": "integration-tests/fixtures/src"}'
codebuddy call rename_directory '{"old_path": "tests/fixtures/test-workspace-symbols", "new_path": "integration-tests/fixtures/test-workspace-symbols"}'
```

### Step 1.5: Cleanup Empty Directories

```bash
# Manual cleanup with git
git rm -r docs/architecture docs/deployment docs/development docs/features
git rm -r deployment/scripts crates/cb-ast/resources tests
```

**Verification:**

```bash
cargo check --workspace
```

**Result After Phase 1:**
```
docs/
├── ARCHITECTURE.md
├── CONTRACTS.md
├── DEPLOYMENT.md
├── LOGGING.md
├── USAGE.md
├── WORKFLOWS.md
└── project/
    ├── BUG_REPORT.md
    ├── CHANGELOG.md
    ├── CLAUDE.md
    ├── MCP_API.md
    ├── PROPOSAL_*.md
    ├── ROADMAP.md
    └── SUPPORT_MATRIX.md

scripts/
├── install.sh
├── setup-dev-tools.sh
├── ast_tool.py
└── ast_tool.go
```

**Phase 1 Command Count**: ~20 codebuddy calls + manual git cleanup

---

## Phase 2: Split cb-server (High Value)

### Step 2.1: Create New Crate Structures

**Use shell script to create directories and Cargo.toml files:**

```bash
# Create cb-lsp crate
mkdir -p crates/cb-lsp/src

cat > crates/cb-lsp/Cargo.toml << 'EOF'
[package]
name = "cb-lsp"
version = "1.0.0-beta"
edition = "2021"
description = "LSP protocol adapter for Codeflow Buddy"
license = "MIT"

[dependencies]
cb-core = { path = "../cb-core" }
tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
lsp-types = "0.97"
dashmap = { workspace = true }
EOF

cat > crates/cb-lsp/src/lib.rs << 'EOF'
//! LSP protocol adapter for Codeflow Buddy

pub mod client;

pub use client::LspClient;
EOF

# Create cb-services crate
mkdir -p crates/cb-services/src

cat > crates/cb-services/Cargo.toml << 'EOF'
[package]
name = "cb-services"
version = "1.0.0-beta"
edition = "2021"
description = "Business logic and orchestration services for Codeflow Buddy"
license = "MIT"

[dependencies]
cb-api = { path = "../cb-api" }
cb-core = { path = "../cb-core" }
cb-ast = { path = "../cb-ast" }
cb-lsp = { path = "../cb-lsp" }
tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
dashmap = { workspace = true }
ignore = "0.4"

[dev-dependencies]
tempfile = "3.0"
EOF

cat > crates/cb-services/src/lib.rs << 'EOF'
//! Services for coordinating complex operations

pub mod ast_service;
pub mod file_service;
pub mod import_service;
pub mod lock_manager;
pub mod operation_queue;
pub mod planner;
pub mod workflow_executor;

#[cfg(test)]
pub mod tests;

pub use ast_service::DefaultAstService;
pub use file_service::FileService;
pub use import_service::ImportService;
pub use lock_manager::{LockManager, LockType};
pub use operation_queue::{FileOperation, OperationQueue, OperationType, QueueStats};
EOF

# Create cb-handlers crate
mkdir -p crates/cb-handlers/src/tools

cat > crates/cb-handlers/Cargo.toml << 'EOF'
[package]
name = "cb-handlers"
version = "1.0.0-beta"
edition = "2021"
description = "MCP tool handler implementations for Codeflow Buddy"
license = "MIT"

[dependencies]
cb-api = { path = "../cb-api" }
cb-core = { path = "../cb-core" }
cb-ast = { path = "../cb-ast" }
cb-lsp = { path = "../cb-lsp" }
cb-services = { path = "../cb-services" }
cb-plugins = { path = "../cb-plugins" }
tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
lsp-types = "0.97"
uuid = { version = "1.0", features = ["v4"] }
EOF

cat > crates/cb-handlers/src/lib.rs << 'EOF'
//! MCP tool handler implementations

pub mod plugin_dispatcher;
pub mod tool_registry;
pub mod tools;

pub use plugin_dispatcher::PluginDispatcher;
pub use tool_registry::ToolRegistry;
EOF

cat > crates/cb-handlers/src/tools/mod.rs << 'EOF'
//! MCP tool implementations organized by domain

pub mod advanced;
pub mod editing;
pub mod file_ops;
pub mod lifecycle;
pub mod navigation;
pub mod system;
pub mod workspace;
EOF
```

### Step 2.2: Move LSP Code

```bash
codebuddy call rename_file '{"old_path": "crates/cb-server/src/systems/lsp/client.rs", "new_path": "crates/cb-lsp/src/client.rs"}'
```

### Step 2.3: Move Service Files

```bash
codebuddy call rename_file '{"old_path": "crates/cb-server/src/services/ast_service.rs", "new_path": "crates/cb-services/src/ast_service.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/services/file_service.rs", "new_path": "crates/cb-services/src/file_service.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/services/import_service.rs", "new_path": "crates/cb-services/src/import_service.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/services/lock_manager.rs", "new_path": "crates/cb-services/src/lock_manager.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/services/operation_queue.rs", "new_path": "crates/cb-services/src/operation_queue.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/services/planner.rs", "new_path": "crates/cb-services/src/planner.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/services/workflow_executor.rs", "new_path": "crates/cb-services/src/workflow_executor.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/services/tests.rs", "new_path": "crates/cb-services/src/tests.rs"}'
```

### Step 2.4: Move Handler Files

```bash
codebuddy call rename_file '{"old_path": "crates/cb-server/src/handlers/plugin_dispatcher.rs", "new_path": "crates/cb-handlers/src/plugin_dispatcher.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/handlers/tool_registry.rs", "new_path": "crates/cb-handlers/src/tool_registry.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/handlers/tools/advanced.rs", "new_path": "crates/cb-handlers/src/tools/advanced.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/handlers/tools/editing.rs", "new_path": "crates/cb-handlers/src/tools/editing.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/handlers/tools/file_ops.rs", "new_path": "crates/cb-handlers/src/tools/file_ops.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/handlers/tools/lifecycle.rs", "new_path": "crates/cb-handlers/src/tools/lifecycle.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/handlers/tools/navigation.rs", "new_path": "crates/cb-handlers/src/tools/navigation.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/handlers/tools/system.rs", "new_path": "crates/cb-handlers/src/tools/system.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-server/src/handlers/tools/workspace.rs", "new_path": "crates/cb-handlers/src/tools/workspace.rs"}'
```

### Step 2.5: Update Workspace Configuration

**Manually edit `/workspace/Cargo.toml`:**

```toml
[workspace]
members = [
    "apps/codebuddy",
    "benchmarks",
    "crates/cb-api",
    "crates/cb-ast",
    "crates/cb-client",
    "crates/cb-core",
    "crates/cb-handlers",      # NEW
    "crates/cb-lsp",           # NEW
    "crates/cb-mcp-proxy",
    "crates/cb-plugins",
    "crates/cb-server",
    "crates/cb-services",      # NEW
    "crates/cb-transport",
    "integration-tests",
]
```

### Step 2.6: Update cb-server Dependencies

**Manually edit `crates/cb-server/Cargo.toml` dependencies section:**

```toml
[dependencies]
cb-api = { path = "../cb-api" }
cb-core = { path = "../cb-core" }
cb-ast = { path = "../cb-ast" }
cb-handlers = { path = "../cb-handlers" }     # NEW
cb-lsp = { path = "../cb-lsp" }               # NEW
cb-services = { path = "../cb-services" }     # NEW
cb-plugins = { path = "../cb-plugins" }
cb-transport = { path = "../cb-transport" }
cb-mcp-proxy = { path = "../cb-mcp-proxy", optional = true }
# ... rest of dependencies unchanged
```

### Step 2.7: Fix Imports (Batch Find-Replace)

**Use sed for bulk import updates:**

```bash
# Update imports in moved files
find crates/cb-lsp crates/cb-services crates/cb-handlers -name "*.rs" -type f -exec sed -i 's/crate::services::/cb_services::/g' {} +
find crates/cb-lsp crates/cb-services crates/cb-handlers -name "*.rs" -type f -exec sed -i 's/crate::systems::lsp::/cb_lsp::/g' {} +
find crates/cb-lsp crates/cb-services crates/cb-handlers -name "*.rs" -type f -exec sed -i 's/crate::handlers::/cb_handlers::/g' {} +

# Update imports in cb-server
find crates/cb-server -name "*.rs" -type f -exec sed -i 's/crate::services::/cb_services::/g' {} +
find crates/cb-server -name "*.rs" -type f -exec sed -i 's/crate::systems::lsp::/cb_lsp::/g' {} +
find crates/cb-server -name "*.rs" -type f -exec sed -i 's/crate::handlers::/cb_handlers::/g' {} +

# Update ServerError references in services
find crates/cb-services -name "*.rs" -type f -exec sed -i 's/use crate::ServerError/use cb_server::ServerError/g' {} +
find crates/cb-services -name "*.rs" -type f -exec sed -i 's/use crate::ServerResult/use cb_server::ServerResult/g' {} +
```

### Step 2.8: Cleanup Old Files

```bash
# Remove old handler files
git rm crates/cb-server/src/handlers/file_operation_handler.rs
git rm crates/cb-server/src/handlers/refactoring_handler.rs
git rm crates/cb-server/src/handlers/system_handler.rs
git rm crates/cb-server/src/handlers/tool_handler.rs
git rm crates/cb-server/src/handlers/workflow_handler.rs
git rm crates/cb-server/src/handlers/lsp_adapter.rs

# Remove empty directories
git rm -r crates/cb-server/src/handlers/tools
git rm -r crates/cb-server/src/services
git rm -r crates/cb-server/src/systems
```

**Verification:**

```bash
cargo check --workspace
cargo test --workspace
```

**Result After Phase 2:**
```
crates/
├── cb-lsp/           (NEW: LSP protocol adapter)
├── cb-services/      (NEW: Business logic)
├── cb-handlers/      (NEW: MCP tool implementations)
└── cb-server/        (SLIMMED: Entry point only)
```

**Phase 2 Command Count**: 1 shell script + ~10 codebuddy calls + 2 manual edits + batch sed

---

## Phase 3: Merge Tiny Crates

### Step 3.1: Move cb-api Files

```bash
# Create directory
mkdir -p crates/cb-core/src/api

codebuddy call rename_file '{"old_path": "crates/cb-api/src/error.rs", "new_path": "crates/cb-core/src/api/error.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-api/src/lib.rs", "new_path": "crates/cb-core/src/api/mod.rs"}'
```

### Step 3.2: Move cb-mcp-proxy Files

```bash
# Create directory
mkdir -p crates/cb-plugins/src/mcp

codebuddy call rename_file '{"old_path": "crates/cb-mcp-proxy/src/client.rs", "new_path": "crates/cb-plugins/src/mcp/client.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-mcp-proxy/src/error.rs", "new_path": "crates/cb-plugins/src/mcp/error.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-mcp-proxy/src/manager.rs", "new_path": "crates/cb-plugins/src/mcp/manager.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-mcp-proxy/src/plugin.rs", "new_path": "crates/cb-plugins/src/mcp/plugin.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-mcp-proxy/src/presets.rs", "new_path": "crates/cb-plugins/src/mcp/presets.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-mcp-proxy/src/protocol.rs", "new_path": "crates/cb-plugins/src/mcp/protocol.rs"}'
codebuddy call rename_file '{"old_path": "crates/cb-mcp-proxy/src/lib.rs", "new_path": "crates/cb-plugins/src/mcp/mod.rs"}'
```

### Step 3.3: Update Module Exports

**Edit `crates/cb-core/src/lib.rs`:**

Add near the top:
```rust
pub mod api;
pub use api::*;
```

**Edit `crates/cb-plugins/src/lib.rs`:**

Add near the top:
```rust
pub mod mcp;
pub use mcp::*;
```

### Step 3.4: Update Workspace Configuration

**Edit `/workspace/Cargo.toml`:**

```toml
[workspace]
members = [
    "apps/codebuddy",
    "benchmarks",
    # "crates/cb-api",         # REMOVED - merged into cb-core
    "crates/cb-ast",
    "crates/cb-client",
    "crates/cb-core",
    "crates/cb-handlers",
    "crates/cb-lsp",
    # "crates/cb-mcp-proxy",   # REMOVED - merged into cb-plugins
    "crates/cb-plugins",
    "crates/cb-server",
    "crates/cb-services",
    "crates/cb-transport",
    "integration-tests",
]
```

### Step 3.5: Update Dependencies in Cargo.toml Files

**Use sed for batch updates:**

```bash
# Comment out cb-api dependencies (will manually add cb-core if needed)
find crates -name "Cargo.toml" -type f -exec sed -i 's/cb-api = { path = "..\/cb-api" }/# cb-api merged into cb-core/g' {} +

# Comment out cb-mcp-proxy dependencies
find crates -name "Cargo.toml" -type f -exec sed -i 's/cb-mcp-proxy = { path = "..\/cb-mcp-proxy" }/# cb-mcp-proxy merged into cb-plugins/g' {} +
```

**Manually verify these Cargo.toml files have cb-core listed:**
- `crates/cb-handlers/Cargo.toml`
- `crates/cb-services/Cargo.toml`
- `crates/cb-server/Cargo.toml`

### Step 3.6: Fix All Imports

```bash
# Update cb-api imports to cb-core::api
find crates -name "*.rs" -type f -exec sed -i 's/use cb_api::/use cb_core::api::/g' {} +
find crates -name "*.rs" -type f -exec sed -i 's/cb_api::/cb_core::api::/g' {} +

# Update cb-mcp-proxy imports to cb-plugins::mcp
find crates -name "*.rs" -type f -exec sed -i 's/use cb_mcp_proxy::/use cb_plugins::mcp::/g' {} +
find crates -name "*.rs" -type f -exec sed -i 's/cb_mcp_proxy::/cb_plugins::mcp::/g' {} +
```

### Step 3.7: Remove Old Crate Directories

```bash
git rm -r crates/cb-api
git rm -r crates/cb-mcp-proxy
```

**Verification:**

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace
cargo fmt --workspace
```

**Result After Phase 3:**
```
crates/
├── cb-core/        (EXPANDED: +api/)
├── cb-ast/         (UNCHANGED)
├── cb-client/      (UNCHANGED)
├── cb-plugins/     (EXPANDED: +mcp/)
├── cb-transport/   (UNCHANGED)
├── cb-lsp/         (NEW)
├── cb-services/    (NEW)
├── cb-handlers/    (NEW)
└── cb-server/      (SLIMMED)

Result: 9 crates (was 8)
```

**Phase 3 Command Count**: ~9 codebuddy calls + 2 manual edits + batch sed

---

## Final Structure

```
/workspace/
├── apps/
│   └── codebuddy/
├── benchmarks/
├── crates/
│   ├── cb-core/              (EXPANDED: +api/, 16 files, ~3,500 lines)
│   ├── cb-ast/               (UNCHANGED: 13 files, 9,544 lines)
│   ├── cb-client/            (UNCHANGED: 14 files, 5,747 lines)
│   ├── cb-plugins/           (EXPANDED: +mcp/, 17 files, ~4,900 lines)
│   ├── cb-transport/         (UNCHANGED: 4 files, 966 lines)
│   ├── cb-lsp/               (NEW: 3 files, ~715 lines)
│   ├── cb-services/          (NEW: 9 files, ~4,000 lines)
│   ├── cb-handlers/          (NEW: 17 files, ~4,500 lines)
│   └── cb-server/            (SLIMMED: 2 files, ~400 lines)
├── deployment/
│   └── docker/
├── docs/
│   ├── ARCHITECTURE.md
│   ├── CONTRACTS.md
│   ├── DEPLOYMENT.md
│   ├── LOGGING.md
│   ├── USAGE.md
│   ├── WORKFLOWS.md
│   └── project/
│       ├── BUG_REPORT.md
│       ├── CHANGELOG.md
│       ├── CLAUDE.md
│       ├── MCP_API.md
│       ├── PROPOSAL_*.md
│       ├── ROADMAP.md
│       └── SUPPORT_MATRIX.md
├── examples/
├── integration-tests/
│   └── fixtures/
├── playground/
├── scripts/
│   ├── install.sh
│   ├── setup-dev-tools.sh
│   ├── ast_tool.py
│   └── ast_tool.go
└── [root files]
```

---

## Execution Summary

### Phase 1
- ~20 `codebuddy call rename_file` commands
- ~5 `codebuddy call rename_directory` commands
- Manual git cleanup

### Phase 2
- 1 shell script (create crates)
- ~10 `codebuddy call rename_file` commands
- 2 manual Cargo.toml edits
- Batch sed for imports
- Manual git cleanup

### Phase 3
- ~9 `codebuddy call rename_file` commands
- 2 manual lib.rs edits
- 1 manual Cargo.toml edit
- Batch sed for imports and dependencies
- Manual git cleanup

**Total: ~44 codebuddy calls + 1 shell script + ~5 manual edits + batch sed operations**

---

## Benefits

### Maintainability
- ✅ Clear separation of concerns (handlers vs services vs LSP)
- ✅ Easier navigation (find operations in cb-services, not buried in cb-server)
- ✅ Reduced cognitive load per crate

### Development Experience
- ✅ Faster iteration (change handlers without recompiling services)
- ✅ Clearer APIs (explicit crate boundaries)
- ✅ Better testability (isolated concerns)

### Documentation
- ✅ Organized root directory
- ✅ Centralized project documentation
- ✅ Clear script location

### Code Quality
- ✅ Completes handler refactor (finish tools/ migration)
- ✅ Addresses cb-server bloat
- ✅ Reduces crate fragmentation (merge tiny crates)

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking imports | Git branch, incremental testing after each phase |
| Circular dependencies | Clear dependency graph (handlers → services → lsp → core) |
| Compilation issues | Fix phase by phase with cargo check after each step |
| Incomplete migration | Use git to track progress, revert if needed |
| sed errors on macOS | Use `sed -i ''` on macOS instead of `sed -i` |

---

## Execution Checklist

- [ ] **Phase 1.1**: Move root docs (10 codebuddy calls)
- [ ] **Phase 1.2**: Flatten docs (6 codebuddy calls)
- [ ] **Phase 1.3**: Consolidate scripts (4 codebuddy calls)
- [ ] **Phase 1.4**: Consolidate test fixtures (5 codebuddy calls)
- [ ] **Phase 1.5**: Git cleanup
- [ ] **Verify Phase 1**: `cargo check --workspace`

- [ ] **Phase 2.1**: Create new crate structures (shell script)
- [ ] **Phase 2.2**: Move LSP code (1 codebuddy call)
- [ ] **Phase 2.3**: Move service files (8 codebuddy calls)
- [ ] **Phase 2.4**: Move handler files (9 codebuddy calls)
- [ ] **Phase 2.5**: Update workspace Cargo.toml (manual)
- [ ] **Phase 2.6**: Update cb-server Cargo.toml (manual)
- [ ] **Phase 2.7**: Fix imports (batch sed)
- [ ] **Phase 2.8**: Git cleanup
- [ ] **Verify Phase 2**: `cargo check --workspace && cargo test --workspace`

- [ ] **Phase 3.1**: Move cb-api files (2 codebuddy calls)
- [ ] **Phase 3.2**: Move cb-mcp-proxy files (7 codebuddy calls)
- [ ] **Phase 3.3**: Update cb-core/cb-plugins lib.rs (manual)
- [ ] **Phase 3.4**: Update workspace Cargo.toml (manual)
- [ ] **Phase 3.5**: Update dependencies (batch sed + manual verification)
- [ ] **Phase 3.6**: Fix imports (batch sed)
- [ ] **Phase 3.7**: Git cleanup
- [ ] **Verify Phase 3**: `cargo check --workspace && cargo test --workspace && cargo clippy --workspace`

- [ ] **Final**: Run full test suite, update CI/CD, commit changes

---

## Decision

**Recommendation**: Execute all 3 phases in order, with verification after each phase.

**Total Effort**: 6-8 hours

**Rationale**:
- Using codebuddy CLI for all file operations ensures LSP awareness and import tracking
- Shell scripts only for creating new files (Cargo.toml, lib.rs stubs)
- Batch sed operations for consistency in import updates
- Incremental verification catches issues early
- Git cleanup keeps repository clean

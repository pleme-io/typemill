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

### Step 1.1: Create Directory Structure & Move Files

**Single batch operation using shell:**

```bash
# Create new directories
mkdir -p docs/project scripts

# Move root documentation to docs/project/
mv BUG_REPORT.md CHANGELOG.md CLAUDE.md MCP_API.md ROADMAP.md SUPPORT_MATRIX.md \
   PROPOSAL_ADVANCED_ANALYSIS.md PROPOSAL_BACKEND_ARCHITECTURE.md \
   PROPOSAL_HANDLER_ARCHITECTURE.md PROPOSAL_RESTRUCTURE.md \
   docs/project/

# Flatten docs subdirectories
mv docs/architecture/ARCHITECTURE.md docs/architecture/contracts.md docs/
mv docs/deployment/OPERATIONS.md docs/DEPLOYMENT.md
mv docs/deployment/USAGE.md docs/USAGE.md
mv docs/development/LOGGING_GUIDELINES.md docs/LOGGING.md
mv docs/features/WORKFLOWS.md docs/WORKFLOWS.md

# Consolidate scripts
mv install.sh deployment/scripts/setup-dev-tools.sh scripts/
mv crates/cb-ast/resources/ast_tool.py crates/cb-ast/resources/ast_tool.go scripts/

# Merge test fixtures
mv tests/fixtures/* integration-tests/fixtures/

# Cleanup empty directories
rm -rf docs/architecture docs/deployment docs/development docs/features
rm -rf deployment/scripts crates/cb-ast/resources tests

# Fix contracts.md naming
mv docs/contracts.md docs/CONTRACTS.md
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

---

## Phase 2: Split cb-server (High Value)

### Step 2.1: Create New Crate Structures

**Batch create using shell script:**

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

### Step 2.2: Move Code in Bulk

```bash
# Move LSP code
mv crates/cb-server/src/systems/lsp/client.rs crates/cb-lsp/src/

# Move service files
mv crates/cb-server/src/services/*.rs crates/cb-services/src/

# Move handler files
mv crates/cb-server/src/handlers/plugin_dispatcher.rs crates/cb-handlers/src/
mv crates/cb-server/src/handlers/tool_registry.rs crates/cb-handlers/src/
mv crates/cb-server/src/handlers/tools/*.rs crates/cb-handlers/src/tools/

# Cleanup old directories
rm -rf crates/cb-server/src/handlers/tools
rm -rf crates/cb-server/src/services
rm -rf crates/cb-server/src/systems
rm -f crates/cb-server/src/handlers/file_operation_handler.rs
rm -f crates/cb-server/src/handlers/refactoring_handler.rs
rm -f crates/cb-server/src/handlers/system_handler.rs
rm -f crates/cb-server/src/handlers/tool_handler.rs
rm -f crates/cb-server/src/handlers/workflow_handler.rs
rm -f crates/cb-server/src/handlers/lsp_adapter.rs
```

### Step 2.3: Update Workspace Configuration

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

### Step 2.4: Update cb-server Dependencies

**Manually edit `crates/cb-server/Cargo.toml`:**

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
# ... rest of dependencies
```

### Step 2.5: Fix Imports (Batch Find-Replace)

**Use sed or your editor's find-replace:**

```bash
# Find all moved files and update imports
find crates/cb-lsp crates/cb-services crates/cb-handlers -name "*.rs" -type f -exec sed -i 's/crate::services::/cb_services::/g' {} +
find crates/cb-lsp crates/cb-services crates/cb-handlers -name "*.rs" -type f -exec sed -i 's/crate::systems::lsp::/cb_lsp::/g' {} +
find crates/cb-lsp crates/cb-services crates/cb-handlers -name "*.rs" -type f -exec sed -i 's/crate::handlers::/cb_handlers::/g' {} +
find crates/cb-server -name "*.rs" -type f -exec sed -i 's/crate::services::/cb_services::/g' {} +
find crates/cb-server -name "*.rs" -type f -exec sed -i 's/crate::systems::lsp::/cb_lsp::/g' {} +
find crates/cb-server -name "*.rs" -type f -exec sed -i 's/crate::handlers::/cb_handlers::/g' {} +

# Update ServerError references if needed
find crates/cb-services -name "*.rs" -type f -exec sed -i 's/use crate::ServerError/use cb_server::ServerError/g' {} +
find crates/cb-services -name "*.rs" -type f -exec sed -i 's/use crate::ServerResult/use cb_server::ServerResult/g' {} +
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

---

## Phase 3: Merge Tiny Crates

### Step 3.1: Merge cb-api and cb-mcp-proxy

**Batch operation:**

```bash
# Merge cb-api into cb-core
mkdir -p crates/cb-core/src/api
mv crates/cb-api/src/error.rs crates/cb-core/src/api/
mv crates/cb-api/src/lib.rs crates/cb-core/src/api/mod.rs

# Merge cb-mcp-proxy into cb-plugins
mkdir -p crates/cb-plugins/src/mcp
mv crates/cb-mcp-proxy/src/*.rs crates/cb-plugins/src/mcp/
mv crates/cb-plugins/src/mcp/lib.rs crates/cb-plugins/src/mcp/mod.rs

# Remove old crate directories
rm -rf crates/cb-api crates/cb-mcp-proxy
```

### Step 3.2: Update Module Exports

**Edit `crates/cb-core/src/lib.rs`:**

Add at the top:
```rust
pub mod api;
pub use api::*;
```

**Edit `crates/cb-plugins/src/lib.rs`:**

Add at the top:
```rust
pub mod mcp;
pub use mcp::*;
```

### Step 3.3: Update Workspace Configuration

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

### Step 3.4: Update All Dependencies (Batch)

**Use sed to update all Cargo.toml files:**

```bash
# Replace cb-api with cb-core in all Cargo.toml files
find crates -name "Cargo.toml" -type f -exec sed -i 's/cb-api = { path = "..\/cb-api" }/# cb-api merged into cb-core/g' {} +

# Replace cb-mcp-proxy with cb-plugins
find crates -name "Cargo.toml" -type f -exec sed -i 's/cb-mcp-proxy = { path = "..\/cb-mcp-proxy" }/# cb-mcp-proxy merged into cb-plugins/g' {} +
```

**Manually review and ensure cb-core and cb-plugins dependencies are present where needed.**

### Step 3.5: Fix All Imports (Batch)

```bash
# Update cb-api imports to cb-core::api
find crates -name "*.rs" -type f -exec sed -i 's/use cb_api::/use cb_core::api::/g' {} +
find crates -name "*.rs" -type f -exec sed -i 's/cb_api::/cb_core::api::/g' {} +

# Update cb-mcp-proxy imports to cb-plugins::mcp
find crates -name "*.rs" -type f -exec sed -i 's/use cb_mcp_proxy::/use cb_plugins::mcp::/g' {} +
find crates -name "*.rs" -type f -exec sed -i 's/cb_mcp_proxy::/cb_plugins::mcp::/g' {} +
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

### Phase 1: 1 batch shell script
- Create dirs, move files, cleanup (1 script execution)

### Phase 2: 5 steps
1. Create new crates (1 shell script)
2. Move code files (1 batch mv)
3. Update workspace Cargo.toml (manual edit)
4. Update cb-server Cargo.toml (manual edit)
5. Fix imports (batch sed)

### Phase 3: 5 steps
1. Merge crates (1 batch shell script)
2. Update module exports (2 manual edits)
3. Update workspace Cargo.toml (manual edit)
4. Update dependencies (batch sed)
5. Fix imports (batch sed)

**Total: ~3 shell scripts + ~5 manual edits + verification steps**

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

- [ ] **Phase 1**: Run shell script (create dirs, move files, cleanup)
- [ ] **Verify Phase 1**: `cargo check --workspace`

- [ ] **Phase 2.1**: Run shell script (create new crates)
- [ ] **Phase 2.2**: Run shell script (move code files)
- [ ] **Phase 2.3**: Edit `/workspace/Cargo.toml` (add new crates)
- [ ] **Phase 2.4**: Edit `crates/cb-server/Cargo.toml` (add dependencies)
- [ ] **Phase 2.5**: Run batch sed (fix imports)
- [ ] **Verify Phase 2**: `cargo check --workspace && cargo test --workspace`

- [ ] **Phase 3.1**: Run shell script (merge crates)
- [ ] **Phase 3.2**: Edit `crates/cb-core/src/lib.rs` and `crates/cb-plugins/src/lib.rs`
- [ ] **Phase 3.3**: Edit `/workspace/Cargo.toml` (remove old crates)
- [ ] **Phase 3.4**: Run batch sed (update dependencies)
- [ ] **Phase 3.5**: Run batch sed (fix imports)
- [ ] **Verify Phase 3**: `cargo check --workspace && cargo test --workspace && cargo clippy --workspace`

- [ ] **Final**: Run full test suite, update CI/CD, commit changes

---

## Decision

**Recommendation**: Execute all 3 phases in order, with verification after each phase.

**Total Effort**: 4-6 hours (much faster with batch operations)

**Rationale**:
- Batch operations reduce manual effort and errors
- Shell scripts provide atomic, reversible operations
- sed batch replacements ensure consistency
- Verification steps catch issues early

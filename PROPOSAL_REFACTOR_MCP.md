# Codebuddy Refactor Plan - MCP-Driven Execution

**Status**: Phases 1-2 Complete ✅ | Phases 3-10 Pending
**Approach**: Incremental, safe, automated where possible using codebuddy MCP tools
**Validation**: `cargo build --release` checkpoint after each phase

---

## ✅ Completed Phases

### Phase 1: Flatten Rust Directory ✅
- Moved `rust/*` → `/workspace/*` (Cargo workspace now at root)
- Updated all documentation references
- Updated Makefile paths
- **Result**: Standard Rust project structure

### Phase 2: Consolidate Binary Architecture ✅
- **Phase 2B**: Renamed `apps/server` → `apps/codebuddy`
- **Phase 2A**: Extracted duplicated AppState creation to `cb-server/lib.rs`
- Eliminated ~100 lines of duplication
- **Result**: Single source of truth for dispatcher initialization

---

## 📋 Remaining Phases (Organized by Dependencies)

### **STAGE 1: File Structure Cleanup** (Phases 3-5)
*No code changes, just file moves - safe to batch together*

### **STAGE 2: Organization & Documentation** (Phases 6-7)
*Organizational improvements - depends on clean file structure*

### **STAGE 3: Code Architecture** (Phases 8-10)
*Requires stable file structure from Stages 1-2*

---

## STAGE 1: File Structure Cleanup

### Phase 3: Reorganize Test Structure ⭐ **DO FIRST**

**Goal**: Separate integration tests from test utilities, move fixtures to proper location

**Why Now**: Prevents conflicts with Phase 4 (benchmarks) and Phase 5 (playground moves)

#### Step 1: Dry-run rename integration-tests → integration-tests

```bash
./target/release/codebuddy tool rename_directory '{
  "old_path": "integration-tests",
  "new_path": "integration-tests",
  "dry_run": true
}'
```

**Expected Output:**
- Files to move count
- Workspace Cargo.toml updates preview
- Documentation reference updates preview

#### Step 2: Execute the rename

```bash
./target/release/codebuddy tool rename_directory '{
  "old_path": "integration-tests",
  "new_path": "integration-tests"
}'
```

#### Step 3: Move playground to test fixtures (dry-run first)

```bash
./target/release/codebuddy tool rename_directory '{
  "old_path": "tests/fixtures",
  "new_path": "tests/fixtures",
  "dry_run": true
}'
```

#### Step 4: Execute playground move

```bash
./target/release/codebuddy tool rename_directory '{
  "old_path": "tests/fixtures",
  "new_path": "tests/fixtures"
}'
```

#### Step 5: CHECKPOINT

```bash
cargo build --release
cargo test --workspace
```

**Success Criteria:**
- ✅ Build succeeds
- ✅ All tests pass
- ✅ Workspace members updated in Cargo.toml

#### Step 6: Commit Phase 3

```bash
git add -A
git commit -m "refactor: reorganize test structure (Phase 3)

- Renamed integration-tests/ → integration-tests/
- Moved tests/fixtures/ → tests/fixtures/
- Updated workspace Cargo.toml members
- Auto-updated documentation references

Separates integration tests from test utilities, improves discoverability.
Phase 3 of restructure complete."
```

---

### Phase 4: Move Benchmarks

**Goal**: Move benchmarks to standard location at repository root

**Why Now**: Can now safely move without conflicting with test reorganization

**Dependencies**: Phase 3 complete (test structure finalized)

#### Step 1: Dry-run benchmark move

```bash
./target/release/codebuddy tool rename_directory '{
  "old_path": "benchmarks",
  "new_path": "benchmarks",
  "dry_run": true
}'
```

#### Step 2: Execute benchmark move

```bash
./target/release/codebuddy tool rename_directory '{
  "old_path": "benchmarks",
  "new_path": "benchmarks"
}'
```

#### Step 3: Check if testing/ directory is now empty

```bash
ls -la testing/
```

**If empty:**
```bash
rmdir testing
```

#### Step 4: CHECKPOINT

```bash
cargo build --release
cargo bench --no-run  # Verify benchmarks compile
```

#### Step 5: Commit Phase 4

```bash
git add -A
git commit -m "refactor: move benchmarks to repository root (Phase 4)

- Moved benchmarks/ → benchmarks/
- Removed empty testing/ directory
- Updated workspace Cargo.toml

Follows Rust standard project layout.
Phase 4 of restructure complete."
```

---

### Phase 5: Split Examples and Playground

**Goal**: Create user-facing examples/, gitignored playground/, proper test fixtures

**Why Now**: File structure is clean, can now organize user-facing content

**Dependencies**: Phase 3 complete (fixtures already moved to tests/fixtures)

#### Step 1: Create playground directory (manual)

```bash
mkdir -p playground
echo "# Developer Playground

Scratch space for testing and experimentation. Not committed to git.
" > playground/README.md

echo '/playground/*
!/playground/.gitkeep
!/playground/README.md' >> .gitignore

touch playground/.gitkeep
```

#### Step 2: Reorganize examples (if needed - check current state first)

```bash
ls -la examples/
```

**If examples need reorganization:**

```bash
# Example: Consolidate TypeScript examples
./target/release/codebuddy tool rename_directory '{
  "old_path": "examples/backend",
  "new_path": "examples/typescript-integration/backend",
  "dry_run": true
}'

# Execute if preview looks good
./target/release/codebuddy tool rename_directory '{
  "old_path": "examples/backend",
  "new_path": "examples/typescript-integration/backend"
}'
```

#### Step 3: Clean up example build artifacts (manual)

```bash
# Remove any target/ directories from examples
find examples -type d -name "target" -exec rm -rf {} + 2>/dev/null || true
```

#### Step 4: CHECKPOINT

```bash
cargo build --release
# Verify examples still work
cargo check --manifest-path examples/*/Cargo.toml 2>/dev/null || true
```

#### Step 5: Commit Phase 5

```bash
git add -A
git commit -m "refactor: split examples and playground (Phase 5)

- Created /playground/ for developer experimentation (gitignored)
- Cleaned up /examples/ for user-facing integration examples
- Removed build artifacts from examples
- Updated .gitignore

Separates internal testing from public examples.
Phase 5 of restructure complete."
```

---

## STAGE 2: Organization & Documentation

### Phase 6: Consolidate Documentation

**Goal**: Organize docs/ into logical categories, move scattered docs to proper locations

**Why Now**: File structure is stable, can now organize documentation properly

**Dependencies**: Phases 3-5 complete (stable file structure)

#### Step 1: Audit current documentation structure

```bash
# See what docs exist
tree docs/ -L 2
ls -la *.md | grep -v README
```

#### Step 2: Create doc category structure (if needed)

```bash
# Check if categories already exist
ls -la docs/
```

**Expected categories:**
- `docs/architecture/` - System design docs
- `docs/deployment/` - Deployment guides
- `docs/api/` - API references
- `docs/development/` - Contributing, dev guides
- `docs/user-guide/` - End-user documentation

#### Step 3: Move scattered root-level docs (manual review needed)

**Identify candidates:**
```bash
ls -la *.md | grep -v -E "README|CLAUDE|PROPOSAL|REFACTOR|CHANGELOG|LICENSE"
```

**For each misplaced doc, use MCP:**
```bash
# Example: Move ARCHITECTURE.md if it's at root
./target/release/codebuddy tool rename_file '{
  "old_path": "ARCHITECTURE.md",
  "new_path": "docs/architecture/ARCHITECTURE.md",
  "dry_run": true
}'
```

#### Step 4: Update internal doc references

This is **mostly manual** - requires editorial decisions:
- Read each doc
- Update cross-references
- Ensure consistency

**Use grep to find references:**
```bash
# Find references to moved docs
grep -r "ARCHITECTURE.md" --include="*.md" .
```

#### Step 5: CHECKPOINT

```bash
# Verify links still work (if you have a link checker)
# Or manually spot-check key docs
cat docs/architecture/ARCHITECTURE.md | head -20
```

#### Step 6: Commit Phase 6

```bash
git add -A
git commit -m "docs: consolidate documentation structure (Phase 6)

- Organized docs/ into logical categories
- Moved scattered root-level docs to proper locations
- Updated cross-references between documents
- Improved discoverability

Phase 6 of restructure complete."
```

---

### Phase 7: Organize Infrastructure Files

**Goal**: Move deployment/infrastructure files to dedicated directory

**Why Now**: Documentation organized, can now organize infrastructure

**Dependencies**: Phase 6 complete (docs organized, no conflicts)

#### Step 1: Create deployment directory structure

```bash
mkdir -p deployment/{deployment/docker,deployment/scripts,vm}
```

#### Step 2: Move deployment/docker files (dry-run first)

```bash
# Check what deployment/docker files exist
ls -la deployment/docker/ 2>/dev/null || ls -la *.deployment/dockerfile 2>/dev/null || ls -la Dockerfile* 2>/dev/null

# If deployment/docker/ directory exists:
./target/release/codebuddy tool rename_directory '{
  "old_path": "deployment/docker",
  "new_path": "deployment/deployment/docker",
  "dry_run": true
}'

# Execute if good:
./target/release/codebuddy tool rename_directory '{
  "old_path": "deployment/docker",
  "new_path": "deployment/deployment/docker"
}'
```

#### Step 3: Move VM configuration

```bash
# Check for VM files
ls -la vm.yaml 2>/dev/null

# If exists:
./target/release/codebuddy tool rename_file '{
  "old_path": "vm.yaml",
  "new_path": "deployment/vm/vm.yaml",
  "dry_run": true
}'

./target/release/codebuddy tool rename_file '{
  "old_path": "vm.yaml",
  "new_path": "deployment/vm/vm.yaml"
}'
```

#### Step 4: Move deployment deployment/scripts

```bash
# Check deployment/scripts directory
ls -la deployment/scripts/

# Move deployment-related deployment/scripts
./target/release/codebuddy tool rename_directory '{
  "old_path": "deployment/scripts",
  "new_path": "deployment/deployment/scripts",
  "dry_run": true
}'

# Or move selectively if deployment/scripts contains dev tools
# (Review and move only deployment deployment/scripts manually)
```

#### Step 5: Update CI/CD references (manual)

```bash
# Find CI/CD files that reference moved paths
grep -r "deployment/docker/" .github/ 2>/dev/null
grep -r "deployment/scripts/" .github/ 2>/dev/null
grep -r "vm.yaml" .github/ 2>/dev/null

# Update paths in CI/CD configs manually
```

#### Step 6: CHECKPOINT

```bash
cargo build --release

# Verify deployment/docker builds still work
deployment/docker-compose -f deployment/deployment/docker/deployment/docker-compose.yml config 2>/dev/null || echo "No deployment/docker-compose file"
```

#### Step 7: Commit Phase 7

```bash
git add -A
git commit -m "refactor: organize infrastructure files (Phase 7)

- Created deployment/ directory structure
- Moved deployment/docker/ → deployment/deployment/docker/
- Moved vm.yaml → deployment/vm/vm.yaml
- Moved deployment deployment/scripts → deployment/deployment/scripts/
- Updated CI/CD path references

Centralizes deployment and infrastructure configuration.
Phase 7 of restructure complete."
```

---

## STAGE 3: Code Architecture Refactoring

### Phase 8: Extract Dispatcher Factory ✅ **ALREADY DONE**

**Status**: Complete in Phase 2A

**What was done:**
- ✅ Created `create_dispatcher_with_workspace()` in `cb-server/lib.rs`
- ✅ Updated `cb-server/main.rs` to use shared function
- ✅ Updated `apps/codebuddy/dispatcher_factory.rs` to use shared function
- ✅ Removed ~100 lines of duplicated code

**Result**: Single source of truth for dispatcher initialization

---

### Phase 9: Tool Handler Architecture

**Goal**: Extract tool handlers from monolithic plugin_dispatcher.rs into separate handlers

**Why Now**: File structure is clean and stable, can focus on code architecture

**Dependencies**: Phases 3-7 complete (stable structure), Phase 8 complete (dispatcher extracted)

**⚠️ WARNING**: This phase is **complex manual refactoring** - cannot be automated with MCP tools

#### Overview of Changes

**What we're doing:**
- Creating `ToolHandler` trait for extensibility
- Extracting 4 handler types from `plugin_dispatcher.rs`:
  1. **FileOperationsHandler** - rename_file, create_file, delete_file, etc.
  2. **WorkflowHandler** - achieve_intent, apply_edits
  3. **SystemHandler** - health_check, notify_file_*, find_dead_code
  4. **RefactoringHandler** - extract_function, inline_variable, etc.
- Creating `ToolRegistry` for dynamic routing
- Removing hardcoded if/else chains

**Why this helps:**
- ✅ Eliminates 600+ lines of routing logic
- ✅ Single Responsibility Principle - each handler = one concern
- ✅ Easy to add new tools - just create handler + register
- ✅ No more editing plugin_dispatcher.rs for every tool

#### Step 1: Create ToolHandler trait

**File**: `crates/cb-server/src/tool_handler.rs`

```rust
//! Tool handler trait for non-LSP operations

use crate::{ServerError, ServerResult};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::Value;
use std::sync::Arc;

/// Handler for MCP tool operations
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Tool names this handler supports
    fn supported_tools(&self) -> Vec<&'static str>;

    /// Handle a tool call
    async fn handle_tool(
        &self,
        tool_call: ToolCall,
        context: Arc<ToolContext>,
    ) -> ServerResult<Value>;

    /// Tool definitions for MCP tools/list (optional)
    fn tool_definitions(&self) -> Vec<Value> {
        vec![]
    }
}

/// Context passed to tool handlers
pub struct ToolContext {
    pub app_state: Arc<crate::handlers::AppState>,
    pub plugin_manager: Arc<cb_plugins::PluginManager>,
}
```

#### Step 2: Create ToolRegistry

**File**: `crates/cb-server/src/tool_registry.rs`

```rust
//! Tool handler registry with automatic routing

use crate::tool_handler::{ToolContext, ToolHandler};
use crate::{ServerError, ServerResult};
use cb_core::model::mcp::ToolCall;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

pub struct ToolRegistry {
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, handler: Arc<dyn ToolHandler>) {
        for tool_name in handler.supported_tools() {
            debug!(tool_name = %tool_name, "Registering tool handler");
            if self.handlers.insert(tool_name.to_string(), handler.clone()).is_some() {
                warn!(tool_name = %tool_name, "Tool handler replaced");
            }
        }
    }

    pub async fn handle_tool(
        &self,
        tool_call: ToolCall,
        context: Arc<ToolContext>,
    ) -> ServerResult<Value> {
        if let Some(handler) = self.handlers.get(&tool_call.name) {
            handler.handle_tool(tool_call, context).await
        } else {
            Err(ServerError::Unsupported(format!(
                "No handler for tool: {}",
                tool_call.name
            )))
        }
    }

    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.handlers.contains_key(tool_name)
    }

    pub fn list_tools(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}
```

#### Step 3: Create handlers (extract from plugin_dispatcher.rs)

**This is the manual work** - for each handler:

1. Create file in `crates/cb-server/src/handlers/`
2. Implement `ToolHandler` trait
3. Copy implementation from `plugin_dispatcher.rs`
4. Update to use `ToolContext`

**Files to create:**
- `handlers/file_operations_handler.rs` (~200 lines from plugin_dispatcher.rs:1433-1670)
- `handlers/workflow_handler.rs` (~150 lines from plugin_dispatcher.rs:1279-1432)
- `handlers/system_handler.rs` (~180 lines from plugin_dispatcher.rs:702-881)
- `handlers/refactoring_handler.rs` (~500 lines from plugin_dispatcher.rs:1678-2189)

**See `REFACTOR_PLAN.md` for detailed implementation examples**

#### Step 4: Update plugin_dispatcher.rs to use registry

**Changes to `crates/cb-server/src/handlers/plugin_dispatcher.rs`:**

1. Add `tool_registry` field to `PluginDispatcher` struct
2. Initialize registry in `initialize()` method:
   ```rust
   let mut registry = ToolRegistry::new();
   registry.register(Arc::new(FileOperationsHandler::new()));
   registry.register(Arc::new(WorkflowHandler::new()));
   registry.register(Arc::new(SystemHandler::new()));
   registry.register(Arc::new(RefactoringHandler::new()));
   ```
3. Replace hardcoded routing in `handle_tool_call()`:
   ```rust
   // Before: 600+ lines of if/else
   if tool_name == "rename_file" { ... }
   else if tool_name == "create_file" { ... }
   // ... 50+ more conditions

   // After: Dynamic routing
   let context = Arc::new(ToolContext {
       app_state: self.app_state.clone(),
       plugin_manager: self.plugin_manager.clone(),
   });

   if self.tool_registry.has_tool(&tool_name) {
       self.tool_registry.handle_tool(tool_call, context).await
   } else {
       // Fallback to plugin system for LSP operations
       self.plugin_manager.handle_request(plugin_request).await
   }
   ```

#### Step 5: Update module exports

**File**: `crates/cb-server/src/lib.rs`
```rust
pub mod tool_handler;
pub mod tool_registry;
```

**File**: `crates/cb-server/src/handlers/mod.rs`
```rust
pub mod file_operations_handler;
pub mod workflow_handler;
pub mod system_handler;
pub mod refactoring_handler;

pub use file_operations_handler::FileOperationsHandler;
pub use workflow_handler::WorkflowHandler;
pub use system_handler::SystemHandler;
pub use refactoring_handler::RefactoringHandler;
```

#### Step 6: CHECKPOINT (Critical!)

```bash
# Build after each handler extraction
cargo build --release

# Run all tests
cargo test --workspace

# Verify all tools still work
./target/release/codebuddy tools | head -20
```

#### Step 7: Commit Phase 9

```bash
git add -A
git commit -m "refactor: implement tool handler architecture (Phase 9)

- Created ToolHandler trait for extensibility
- Extracted FileOperationsHandler (file ops)
- Extracted WorkflowHandler (workflows)
- Extracted SystemHandler (health, notifications)
- Extracted RefactoringHandler (refactoring ops)
- Created ToolRegistry for dynamic routing
- Replaced 600+ lines of hardcoded if/else with registry pattern

Benefits:
- Single Responsibility Principle per handler
- Easy to add new tools (create handler + register)
- No more editing plugin_dispatcher for every tool
- Cleaner separation of concerns

Phase 9 of restructure complete."
```

---

### Phase 10: Add CLI Parity

**Goal**: Add `tools` command to CLI for better tool discovery

**Why Now**: Tool handlers are in place, can now expose them via CLI

**Dependencies**: Phase 9 complete (tool handlers implemented)

#### Step 1: Add `tools` subcommand to CLI

**File**: `apps/codebuddy/src/cli.rs`

Add to `Commands` enum (after existing commands):
```rust
/// List all available MCP tools
Tools {
    /// Output format (table, json, or names-only)
    #[arg(long, default_value = "table", value_parser = ["table", "json", "names"])]
    format: String,
},
```

#### Step 2: Add handler for tools command

In the `match cli.command` block:
```rust
Commands::Tools { format } => {
    handle_tools_command(&format).await;
}
```

#### Step 3: Implement handler function

**Add to `apps/codebuddy/src/cli.rs`:**

```rust
/// Handle tools list command
async fn handle_tools_command(format: &str) {
    let dispatcher = match crate::dispatcher_factory::create_initialized_dispatcher().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error initializing: {}", e);
            process::exit(1);
        }
    };

    // Create MCP tools/list request
    use cb_core::model::mcp::{McpMessage, McpRequest};
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        method: "tools/list".to_string(),
        params: None,
    };

    match dispatcher.dispatch(McpMessage::Request(request)).await {
        Ok(McpMessage::Response(response)) => {
            if let Some(result) = response.result {
                match format {
                    "json" => {
                        println!("{}", serde_json::to_string_pretty(&result).unwrap())
                    }
                    "names" => {
                        if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                            for tool in tools {
                                if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                                    println!("{}", name);
                                }
                            }
                        }
                    }
                    _ => {
                        // Table format
                        println!("{:<30} {}", "TOOL NAME", "DESCRIPTION");
                        println!("{}", "=".repeat(80));
                        if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                            for tool in tools {
                                let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                                let desc = tool.get("description").and_then(|d| d.as_str()).unwrap_or("");
                                let desc_short = if desc.len() > 48 {
                                    format!("{}...", &desc[..45])
                                } else {
                                    desc.to_string()
                                };
                                println!("{:<30} {}", name, desc_short);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error listing tools: {}", e);
            process::exit(1);
        }
        _ => {
            eprintln!("Unexpected response type");
            process::exit(1);
        }
    }
}
```

#### Step 4: Test the new command

```bash
cargo build --release

# Test table format (default)
./target/release/codebuddy tools

# Test JSON output
./target/release/codebuddy tools --format json

# Test names only (useful for scripting)
./target/release/codebuddy tools --format names
```

#### Step 5: Update help documentation

**File**: `apps/codebuddy/src/cli.rs`

Update the `#[command(about = "...")]` if needed to mention the new `tools` command.

#### Step 6: CHECKPOINT

```bash
cargo build --release
cargo test --workspace

# Verify tools command works
./target/release/codebuddy tools | head -10
```

#### Step 7: Commit Phase 10

```bash
git add -A
git commit -m "feat: add tools command to CLI (Phase 10)

- Added 'codebuddy tools' subcommand for tool discovery
- Supports table, JSON, and names-only output formats
- Improves developer experience and scriptability

Usage:
  codebuddy tools              # Table format
  codebuddy tools --format json    # JSON output
  codebuddy tools --format names   # Names only

Phase 10 of restructure complete."
```

---

## 📊 Progress Tracking

### Completed ✅
- [x] Phase 1: Flatten Rust directory
- [x] Phase 2: Consolidate binary architecture
  - [x] Phase 2B: Rename apps/server → apps/codebuddy
  - [x] Phase 2A: Extract AppState duplication

### Pending ⏳
- [ ] **Phase 3: Reorganize Test Structure** ⭐ NEXT
- [ ] Phase 4: Move Benchmarks
- [ ] Phase 5: Split Examples and Playground
- [ ] Phase 6: Consolidate Documentation
- [ ] Phase 7: Organize Infrastructure Files
- [ ] Phase 8: Extract Dispatcher Factory ✅ (already done in Phase 2A)
- [ ] Phase 9: Tool Handler Architecture
- [ ] Phase 10: Add CLI Parity

### Estimated Time Remaining
- **Structural cleanup (3-5)**: ~4-6 hours
- **Documentation (6)**: ~2-3 hours
- **Infrastructure (7)**: ~1 hour
- **Code architecture (9-10)**: ~6-10 hours

**Total**: ~13-20 hours

---

## ✅ Success Criteria

After all phases complete:

1. **Clean Structure**
   - ✅ Rust workspace at repository root
   - ✅ Tests organized (unit vs integration)
   - ✅ Examples user-facing, playground gitignored
   - ✅ Documentation categorized
   - ✅ Infrastructure centralized in deployment/

2. **Clean Code**
   - ✅ No duplication (single source of truth)
   - ✅ Tool handlers = single responsibility
   - ✅ Dynamic routing (no hardcoded if/else chains)
   - ✅ Extensible (add tools without touching core)

3. **All Checkpoints Pass**
   - ✅ `cargo build --release` succeeds after each phase
   - ✅ `cargo test --workspace` passes after each phase
   - ✅ All tools still work: `codebuddy tools`

4. **Documentation Updated**
   - ✅ All cross-references updated
   - ✅ Architecture docs reflect new structure
   - ✅ Developer guide includes tool handler pattern

---

## 🚀 Next Steps

**To begin Phase 3:**

```bash
# Ensure latest codebuddy is built
cargo build --release

# Start Phase 3: Reorganize Test Structure
./target/release/codebuddy tool rename_directory '{
  "old_path": "integration-tests",
  "new_path": "integration-tests",
  "dry_run": true
}'
```

**Review the dry-run output, then proceed with execution when ready!**

# Project Structure Reorganization Proposal

## 🔄 Debug & Test Strategy

**CRITICAL WORKFLOW**: When debugging CodeBuddy functionality, follow this cycle strictly:

### The Debug Cycle

1. **Run Test** - Execute CodeBuddy command (e.g., `rename_directory`)
2. **Check Result** - Did it work correctly?
   - ✅ **YES** → Commit the changes surgically (only that specific fix)
   - ❌ **NO** → Proceed to debug cycle below

3. **Stash Changes** (if test failed)
   ```bash
   git add . && git stash
   ```
   - Always stash before debugging to avoid accumulating broken changes
   - Keeps working tree clean for next iteration

4. **Debug in `.debug/` Directory**
   - Create isolated reproduction in `.debug/[feature-name]/`
   - Test fixes WITHOUT rebuilding entire project
   - Iterate quickly on solutions
   - Example: `.debug/import-bug-investigation/test_import_rewrite.rs`

5. **Apply Fix to Main Project**
   - Once satisfied with `.debug/` solution, apply to actual codebase
   - Build and verify: `cargo build --release`

6. **Test Again** - Return to step 1

### Key Principles

- **Never commit broken changes** - If test fails, stash immediately
- **Surgical commits** - If you find ANY working fix (even partial), commit it separately
- **Use `.debug/` liberally** - Faster iteration than full rebuilds
- **Document findings** - Keep analysis docs in `.debug/` for reference
- **One fix at a time** - Don't batch fixes, commit incrementally

### Example Debug Session

```bash
# Test fails
./target/release/codebuddy tool rename_directory '{"old_path": "foo", "new_path": "bar"}'
# Observe duplicate imports

# Stash the broken result
git add . && git stash

# Debug in isolation
mkdir -p .debug/import-fix
# Create standalone test, iterate on solution

# Apply fix to codebase
# Edit crates/cb-services/src/services/file_service.rs
cargo build --release

# Test again
./target/release/codebuddy tool rename_directory '{"old_path": "foo", "new_path": "bar"}'

# Success! Commit the fix
git add crates/cb-services/src/services/file_service.rs
git commit -m "fix: prevent duplicate imports in rename_directory"
```

---

## 🎯 Primary Goal: Dogfood ALL CodeBuddy Tools

**The entire point of this proposal is to ensure ALL of CodeBuddy's MCP tools work correctly by using them on CodeBuddy's own codebase.**

This is a comprehensive tool validation exercise that will test:

### Core File Operations
- ✅ `rename_file` - Moving individual files with import updates
- 🔄 `rename_directory` - Moving entire directories with automatic refactoring
- ⏳ `batch_execute` - Coordinated multi-file operations
- ⏳ `create_file` - Creating new crate structures
- ⏳ `delete_file` - Cleaning up duplicate files
- ⏳ `write_file` - Updating configuration files

### Refactoring & Analysis Tools
- ⏳ `find_references` - Validating all references are updated
- ⏳ `get_diagnostics` - Ensuring no errors after refactoring
- ⏳ `organize_imports` - Cleaning up import statements
- ⏳ `format_document` - Maintaining code style

### Workspace Operations
- ⏳ `analyze_imports` - Verifying import graph integrity
- ⏳ `find_dead_code` - Identifying orphaned code after moves
- ⏳ `update_dependencies` - Cargo.toml path updates

### Validation Tools
- ⏳ `get_document_symbols` - Ensuring symbols still resolve
- ⏳ `find_definition` - Verifying cross-crate navigation
- ⏳ `health_check` - System integrity after changes

**Secondary Benefit**: Achieve a cleaner, Rust-standard workspace layout with flat crate structure.

**Success Criteria**: All tools work correctly without manual intervention, discovering and fixing any bugs encountered.

---

## 📋 Progress Tracker

### ✅ Phase 1: Move Language Metadata Files (COMPLETE)
- ✅ Move `crates/languages/languages.toml` → `config/languages/languages.toml`
- ✅ Move `crates/languages/README.md` → `docs/development/languages/README.md`
- ✅ Move `crates/languages/PLUGIN_DEVELOPMENT_GUIDE.md` → `docs/development/languages/PLUGIN_DEVELOPMENT_GUIDE.md`
- ✅ Commit: `7632bec` - "refactor: Phase 1 - relocate language metadata and configuration files"

### 🔄 Phase 2: Promote Language Crates to Flat Structure (IN PROGRESS)
- ✅ Move `crates/languages/cb-lang-common` → `crates/cb-lang-common`
  - ✅ Commit: `cb1024e` - "refactor: move cb-lang-common to flat crates layout"
- ❌ Move `crates/cb-lang-java` → `crates/cb-lang-java`
- ❌ Move `crates/languages/cb-lang-python` → `crates/cb-lang-python`
- ❌ Move `crates/languages/cb-lang-rust` → `crates/cb-lang-rust`
- ❌ Move `crates/languages/cb-lang-typescript` → `crates/cb-lang-typescript`
- ⚠️ **BLOCKER**: `rename_directory` bug creating duplicate imports - fixing in parallel

### ❌ Phase 3: Reorganize Workspace Crates (NOT STARTED)
- ❌ Move `benchmarks` → `crates/codebuddy-bench`
- ❌ Update `crates/codebuddy-bench/Cargo.toml` (package name)
- ❌ Update root `Cargo.toml` (workspace members)

### ❌ Phase 4: Split Integration Tests (NOT STARTED)
- ❌ Create `crates/test-support/` crate structure
- ❌ Move `integration-tests/src/harness` → `crates/test-support/src/harness`
- ❌ Move `integration-tests/fixtures` → `crates/test-support/fixtures`
- ❌ Move `integration-tests/tests` → `apps/codebuddy/tests`
- ❌ Move helper files to test-support
- ❌ Update `apps/codebuddy/Cargo.toml` (add test-support dev-dependency)

### ❌ Phase 5: Organize Documentation (NOT STARTED)
- ❌ Move proposal files to `docs/proposals/`
- ❌ Move `FAILING_TESTS.md` → `docs/development/FAILING_TESTS.md`
- ❌ Delete duplicate files (`install.sh`, `test_scanner`)
- ❌ Update CLAUDE.md, Makefile, justfile with new paths

---

## 🐛 Current Blocker: rename_directory Bug

**Status**: Investigating and fixing duplicate import bug

**Issue**: `rename_directory` creates malformed duplicate imports:
```rust
use cb_plugin_api :: import_support :: ImportSupport ;  // MALFORMED DUPLICATE
use cb_plugin_api::import_support::ImportSupport;     // ORIGINAL
```

**Root Causes Found**:
1. **Bug #1 (Orchestration)**: Calling `update_imports_for_rename()` once per moved file instead of once for entire directory
2. **Bug #2 (Application)**: Full-file replacements treated incorrectly, multi-line content pushed as single element

**Fixes Applied** (in stash):
- Agent 1's fix: Batch import updates (single call for directory)
- Agent 2's fix: Special case for full-file replacements

**Next Steps**:
1. Test fixes with actual import-triggering scenario
2. Commit if successful
3. Resume Phase 2 reorganization

---

## Proposed Tree Structure

```
/
├── Cargo.toml                     # EDITED: Virtual manifest, members = ["crates/*", "apps/codebuddy"]
├── apps/
│   └── codebuddy/                 # STAYS: main binary (Rust convention)
│       ├── src/
│       ├── tests/                 # MOVED: from integration-tests/tests/
│       └── Cargo.toml             # EDITED: Add test-support dev-dependency
├── crates/
│   ├── cb-ast/                    # STAYS
│   ├── cb-client/                 # STAYS
│   ├── cb-core/                   # STAYS (build.rs updated for config path)
│   ├── cb-handlers/               # STAYS
│   ├── cb-lsp/                    # STAYS
│   ├── cb-plugin-api/             # STAYS (build.rs updated for config path)
│   ├── cb-plugins/                # STAYS
│   ├── cb-protocol/               # STAYS
│   ├── cb-server/                 # STAYS
│   ├── cb-services/               # STAYS (build.rs updated for config path)
│   ├── cb-transport/              # STAYS
│   ├── cb-types/                  # STAYS
│   ├── cb-lang-common/            # ✅ MOVED from crates/languages/
│   ├── cb-lang-go/                # MOVED from crates/languages/ (currently testing)
│   ├── cb-lang-java/              # TO MOVE from crates/languages/
│   ├── cb-lang-python/            # TO MOVE from crates/languages/
│   ├── cb-lang-rust/              # TO MOVE from crates/languages/
│   ├── cb-lang-typescript/        # TO MOVE from crates/languages/
│   ├── codebuddy-bench/           # TO MOVE from benchmarks/
│   └── test-support/              # TO CREATE (extracted from integration-tests/)
│       ├── src/
│       │   ├── harness/
│       │   ├── helpers.rs
│       │   ├── mocks.rs
│       │   └── lib.rs
│       ├── fixtures/
│       └── Cargo.toml
├── config/                        # ✅ CREATED
│   └── languages/
│       └── languages.toml         # ✅ MOVED from crates/languages/
├── docs/
│   ├── architecture/              # STAYS
│   ├── development/
│   │   ├── languages/             # ✅ CREATED
│   │   │   ├── README.md          # ✅ MOVED from crates/languages/
│   │   │   └── PLUGIN_DEVELOPMENT_GUIDE.md  # ✅ MOVED
│   │   ├── FAILING_TESTS.md       # TO MOVE from root
│   │   └── LOGGING_GUIDELINES.md  # STAYS
│   ├── features/                  # STAYS
│   ├── proposals/                 # TO CREATE
│   │   ├── 00_PROPOSAL_LANGUAGE_PLUGIN_REFACTOR.md  # TO MOVE
│   │   ├── 04_PROPOSAL_SUPPORT_MATRIX.md            # TO MOVE
│   │   ├── 05_PROPOSAL_LANG_SUPPORT.md              # TO MOVE
│   │   ├── 06_PROPOSAL_BATCH_EXECUTE.md             # TO MOVE
│   │   ├── 07_PROPOSAL_BACKEND_ARCHITECTURE.md      # TO MOVE
│   │   └── 08_PROPOSAL_ADVANCED_ANALYSIS.md         # TO MOVE
│   ├── security/                  # STAYS
│   └── testing/                   # STAYS
├── deployment/                    # STAYS
├── scripts/                       # STAYS
│   ├── check-features.sh          # Already moved in earlier work
│   └── new-lang.sh                # Already moved in earlier work
└── [root files]                   # STAYS (README, CLAUDE.md, etc.)

# REMOVED after completion:
# - crates/languages/              # After all language crates moved
# - benchmarks/                    # After moving to crates/codebuddy-bench/
# - integration-tests/             # After splitting into test-support + apps/codebuddy/tests/
```

---

## Key Benefits

1. **Dog-fooding**: Stress-test CodeBuddy's own refactoring tools on a real-world workspace
2. **Bug discovery**: Already found and fixing critical `rename_directory` bugs
3. **Rust-standard flat workspace**: All crates at `crates/*` level
4. **Integration tests in binary**: `apps/codebuddy/tests/` per Cargo convention
5. **Centralized config**: `config/languages/languages.toml` for build scripts
6. **Organized documentation**: Proposals, testing guides, language docs properly filed
7. **Clean root directory**: Only essential files

---

## Execution Plan Using CodeBuddy MCP Tools

**Primary Tool Being Tested**: `rename_directory` with automatic import updates

### Phase 1: Move Language Metadata Files ✅ COMPLETE

**MCP Tool**: `rename_file` (individual calls)

```bash
# ✅ DONE - Commit 7632bec
rename_file: crates/languages/languages.toml → config/languages/languages.toml
rename_file: crates/languages/README.md → docs/development/languages/README.md
rename_file: crates/languages/PLUGIN_DEVELOPMENT_GUIDE.md → docs/development/languages/PLUGIN_DEVELOPMENT_GUIDE.md
```

**Validation**: ✅ `cargo check --workspace` passed

---

### Phase 2: Promote Language Crates to Flat Structure 🔄 IN PROGRESS

**MCP Tool**: `rename_directory` (6 separate calls - testing tool capabilities)

```bash
# ✅ DONE - Commit cb1024e
rename_directory: crates/languages/cb-lang-common → crates/cb-lang-common

# ⚠️ BLOCKED - Fixing rename_directory bugs first
rename_directory: crates/cb-lang-java → crates/cb-lang-java
rename_directory: crates/languages/cb-lang-python → crates/cb-lang-python
rename_directory: crates/languages/cb-lang-rust → crates/cb-lang-rust
rename_directory: crates/languages/cb-lang-typescript → crates/cb-lang-typescript
```

**Expected Behavior**:
- CodeBuddy automatically updates all imports across workspace
- No duplicate imports created
- Properly formatted output (no extra spaces)

**Actual Behavior (bug being fixed)**:
- Duplicate imports created
- Malformed formatting with spaces around `::`

**Validation**: `cargo check --workspace` (after fixes committed)

---

### Phase 3: Reorganize Workspace Crates ❌ NOT STARTED

**MCP Tool**: `rename_directory`

```bash
rename_directory: benchmarks → crates/codebuddy-bench
```

**Manual Edits Required**:
```bash
# Update root Cargo.toml workspace members
# Update crates/codebuddy-bench/Cargo.toml package name
```

**Validation**: `cargo check --workspace`

---

### Phase 4: Split Integration Tests ❌ NOT STARTED

**MCP Tool**: `batch_execute` (create test-support structure)

```json
{
  "operations": [
    {"type": "create_file", "path": "crates/test-support/Cargo.toml", "content": "[package]\nname = \"test-support\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\n# Add as needed\n"},
    {"type": "create_file", "path": "crates/test-support/src/lib.rs", "content": "pub mod harness;\npub mod helpers;\npub mod mocks;\n\npub fn fixtures_dir() -> std::path::PathBuf {\n    std::path::PathBuf::from(env!(\"CARGO_MANIFEST_DIR\")).join(\"fixtures\")\n}\n"}
  ]
}
```

**MCP Tool**: `rename_directory` (move test utilities)

```bash
rename_directory: integration-tests/src/harness → crates/test-support/src/harness
rename_directory: integration-tests/fixtures → crates/test-support/fixtures
rename_directory: integration-tests/tests → apps/codebuddy/tests
```

**MCP Tool**: `batch_execute` (move individual files)

```json
{
  "operations": [
    {"type": "rename_file", "old_path": "integration-tests/src/helpers.rs", "new_path": "crates/test-support/src/helpers.rs"},
    {"type": "rename_file", "old_path": "integration-tests/src/mocks.rs", "new_path": "crates/test-support/src/mocks.rs"},
    {"type": "rename_file", "old_path": "integration-tests/TESTING_GUIDE.md", "new_path": "docs/testing/INTEGRATION_GUIDE.md"}
  ]
}
```

**Manual Edits Required**:
```bash
# Update apps/codebuddy/Cargo.toml (add test-support dev-dependency)
# Update test files to use `use test_support::*;`
```

**Validation**: `cargo test --workspace`

---

### Phase 5: Organize Documentation ❌ NOT STARTED

**MCP Tool**: `batch_execute`

```json
{
  "operations": [
    {"type": "rename_file", "old_path": "00_PROPOSAL_LANGUAGE_PLUGIN_REFACTOR.md", "new_path": "docs/proposals/00_PROPOSAL_LANGUAGE_PLUGIN_REFACTOR.md"},
    {"type": "rename_file", "old_path": "04_PROPOSAL_SUPPORT_MATRIX.md", "new_path": "docs/proposals/04_PROPOSAL_SUPPORT_MATRIX.md"},
    {"type": "rename_file", "old_path": "05_PROPOSAL_LANG_SUPPORT.md", "new_path": "docs/proposals/05_PROPOSAL_LANG_SUPPORT.md"},
    {"type": "rename_file", "old_path": "06_PROPOSAL_BATCH_EXECUTE.md", "new_path": "docs/proposals/06_PROPOSAL_BATCH_EXECUTE.md"},
    {"type": "rename_file", "old_path": "07_PROPOSAL_BACKEND_ARCHITECTURE.md", "new_path": "docs/proposals/07_PROPOSAL_BACKEND_ARCHITECTURE.md"},
    {"type": "rename_file", "old_path": "08_PROPOSAL_ADVANCED_ANALYSIS.md", "new_path": "docs/proposals/08_PROPOSAL_ADVANCED_ANALYSIS.md"},
    {"type": "rename_file", "old_path": "FAILING_TESTS.md", "new_path": "docs/development/FAILING_TESTS.md"},
    {"type": "delete_file", "path": "install.sh"},
    {"type": "delete_file", "path": "test_scanner"}
  ]
}
```

**Manual Edits Required**:
```bash
# Update CLAUDE.md with new paths
# Update Makefile/justfile with new benchmark paths
# Update .github/workflows/*.yml with new paths
```

**Validation**:
```bash
cargo fmt
cargo clippy --workspace
cargo test --workspace
cargo bench -p codebuddy-bench --no-run
```

---

## Validation Checklist

**After each phase**:
- ✅ `cargo check --workspace` - Verify compilation
- ✅ `cargo test --workspace` - Verify tests pass
- ✅ `git status` - Review changed files
- ✅ Commit working changes

**Final validation**:
- ✅ `cargo fmt`
- ✅ `cargo clippy --workspace -- -D warnings`
- ✅ `cargo test --workspace --verbose`
- ✅ `cargo bench -p codebuddy-bench --no-run`
- ✅ `cargo build --release -p codebuddy`
- ✅ `./target/release/codebuddy status`

---

## Migration Notes

- All file operations use CodeBuddy MCP tools with automatic import updates
- `rename_directory` and `rename_file` automatically update all imports - no manual edits needed
- Build scripts need manual path updates: `../languages/languages.toml` → `../config/languages/languages.toml`
- CI/Make/just files need manual benchmark path updates
- Test-support crate marked `publish = false`
- Git history preserved through all moves

---

## 🧪 Comprehensive Tool Test Plan

This proposal serves as a **complete validation suite** for CodeBuddy's capabilities. Each phase exercises different tools:

### Already Tested (Phase 1)
- ✅ `rename_file` - Moved 3 files successfully
- ✅ `create_file` - Created `config/languages/` directory
- ✅ Import graph analysis - Verified no broken references
- ✅ `health_check` - System remained healthy

### Currently Testing (Phase 2)
- 🔄 `rename_directory` - **Found critical bugs!**
  - Bug #1: Duplicate imports from redundant scans
  - Bug #2: Malformed full-file replacements
- 🔄 Cross-crate refactoring validation
- 🔄 Cargo.toml dependency path updates

### Upcoming Tests (Phases 3-5)
- ⏳ `batch_execute` with multiple operations
- ⏳ Complex directory moves (benchmarks → crates)
- ⏳ Test fixture reorganization
- ⏳ Documentation reference updates
- ⏳ `delete_file` for cleanup
- ⏳ Full workspace integrity validation

### Validation Tools To Use Throughout
- `find_references` - After each move, verify all references
- `get_diagnostics` - Ensure no compilation errors
- `analyze_imports` - Verify import graph integrity
- `find_dead_code` - Identify orphaned code
- `organize_imports` - Clean up after refactoring
- `format_document` - Maintain code style
- `get_document_symbols` - Ensure symbols resolve
- `find_definition` - Test cross-crate navigation

**Success Metric**: Complete the entire reorganization using ONLY CodeBuddy's MCP tools, with zero manual file edits (except Cargo.toml workspace configuration).

**Current Learning Outcomes**:
- ✅ Discovered critical `rename_directory` bugs
- ✅ Identified need for better full-file replacement logic
- ✅ Validated `rename_file` works correctly
- 🔄 Testing real-world workspace reorganization at scale

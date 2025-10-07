# Project Structure Reorganization Proposal

## Proposed Tree Structure

```
/
├── Cargo.toml                     # EDITED: Virtual manifest, members = ["crates/*"], resolver = "2"
├── Cargo.lock
├── crates/
│   ├── codebuddy/                 # MOVED: from apps/codebuddy/
│   │   ├── src/
│   │   ├── tests/                 # MOVED: integration tests from /integration-tests/tests/
│   │   │   ├── cli_tool_command.rs
│   │   │   ├── contract_validation.rs
│   │   │   ├── debug_hover.rs
│   │   │   ├── e2e_analysis_features.rs
│   │   │   ├── e2e_consolidation.rs
│   │   │   ├── e2e_error_scenarios.rs
│   │   │   ├── e2e_git_operations.rs
│   │   │   ├── e2e_manifest_cross_language.rs
│   │   │   ├── e2e_performance.rs
│   │   │   ├── e2e_python_language_specific.rs
│   │   │   ├── e2e_refactoring_cross_language.rs
│   │   │   ├── e2e_server_lifecycle.rs
│   │   │   ├── e2e_system_tools.rs
│   │   │   ├── e2e_workspace_operations.rs
│   │   │   ├── integration_services.rs
│   │   │   ├── lsp_feature_runners.rs
│   │   │   ├── lsp_features.rs
│   │   │   ├── mcp_file_operations.rs
│   │   │   └── mcp_handler_runners.rs
│   │   └── Cargo.toml             # EDITED: Add test-support as dev-dependency
│   ├── cb-ast/                    # STAYS
│   │   ├── src/
│   │   ├── Cargo.toml
│   │   └── README.md
│   ├── cb-client/                 # STAYS
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-core/                   # STAYS
│   │   ├── src/
│   │   ├── tests/
│   │   ├── build.rs               # EDITED: ../languages/languages.toml → ../config/languages/languages.toml
│   │   └── Cargo.toml
│   ├── cb-handlers/               # STAYS
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-lsp/                    # STAYS
│   │   ├── examples/
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-plugin-api/             # STAYS
│   │   ├── src/
│   │   ├── build.rs               # EDITED: ../languages/languages.toml → ../config/languages/languages.toml
│   │   └── Cargo.toml
│   ├── cb-plugins/                # STAYS
│   │   ├── src/
│   │   ├── tests/
│   │   └── Cargo.toml
│   ├── cb-protocol/               # STAYS
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-server/                 # STAYS
│   │   ├── src/
│   │   ├── tests/
│   │   └── Cargo.toml
│   ├── cb-services/               # STAYS
│   │   ├── src/
│   │   ├── build.rs               # EDITED: ../languages/languages.toml → ../config/languages/languages.toml
│   │   └── Cargo.toml
│   ├── cb-transport/              # STAYS
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-types/                  # STAYS
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-lang-common/            # MOVED: from crates/languages/cb-lang-common/
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-lang-go/                # MOVED: from crates/languages/cb-lang-go/
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-lang-java/              # MOVED: from crates/languages/cb-lang-java/
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-lang-python/            # MOVED: from crates/languages/cb-lang-python/
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-lang-rust/              # MOVED: from crates/languages/cb-lang-rust/
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── cb-lang-typescript/        # MOVED: from crates/languages/cb-lang-typescript/
│   │   ├── src/
│   │   └── Cargo.toml
│   ├── codebuddy-bench/           # MOVED+RENAMED: from benchmarks/
│   │   ├── benches/
│   │   │   ├── dispatch_benchmark.rs
│   │   │   ├── forwarding_benchmark.rs
│   │   │   ├── optimization_benchmark.rs
│   │   │   └── serialization_benchmark.rs
│   │   └── Cargo.toml             # EDITED: name = "codebuddy-bench"
│   └── test-support/              # NEW: extracted from integration-tests/
│       ├── src/
│       │   ├── harness/           # MOVED: from integration-tests/src/harness/
│       │   ├── helpers.rs         # MOVED: from integration-tests/src/helpers.rs
│       │   ├── mocks.rs           # MOVED: from integration-tests/src/mocks.rs
│       │   └── lib.rs             # NEW: exports helpers + fixtures_dir()
│       ├── fixtures/              # MOVED+MERGED: from integration-tests/{fixtures,test-fixtures}/
│       │   ├── consolidation-test/
│       │   ├── atomic-refactoring-test/
│       │   ├── java/
│       │   ├── python/
│       │   ├── rust/
│       │   ├── src/
│       │   ├── test-workspace-symbols/
│       │   ├── app_config.json
│       │   ├── import_graph.json
│       │   ├── intent_spec.json
│       │   ├── mcp_request.json
│       │   ├── mcp_response.json
│       │   └── README.md
│       └── Cargo.toml             # NEW: publish = false
├── config/                        # NEW
│   └── languages/
│       └── languages.toml         # MOVED: from crates/languages/languages.toml
├── examples/                      # STAYS
│   ├── backend/
│   │   ├── main.py
│   │   └── requirements.txt
│   ├── database/
│   │   └── init.sql
│   ├── frontend/
│   │   ├── src/
│   │   └── package.json
│   ├── README.md
│   └── tenant-client.ts
├── docs/
│   ├── architecture/              # STAYS
│   │   ├── ARCHITECTURE.md
│   │   └── INTERNAL_TOOLS.md
│   ├── development/
│   │   ├── languages/             # NEW
│   │   │   ├── README.md          # MOVED: from crates/languages/README.md
│   │   │   ├── PLUGIN_DEVELOPMENT_GUIDE.md  # MOVED: from root (if exists)
│   │   │   └── SCAFFOLDING.md     # MOVED: from crates/languages/SCAFFOLDING.md
│   │   ├── FAILING_TESTS.md       # MOVED: from root
│   │   └── LOGGING_GUIDELINES.md  # STAYS
│   ├── features/                  # STAYS
│   │   └── WORKFLOWS.md
│   ├── proposals/                 # NEW
│   │   ├── 00_PROPOSAL_LANGUAGE_PLUGIN_REFACTOR.md  # MOVED: from root
│   │   ├── 04_PROPOSAL_SUPPORT_MATRIX.md            # MOVED: from root
│   │   ├── 05_PROPOSAL_LANG_SUPPORT.md              # MOVED: from root
│   │   ├── 06_PROPOSAL_BATCH_EXECUTE.md             # MOVED: from root
│   │   ├── 07_PROPOSAL_BACKEND_ARCHITECTURE.md      # MOVED: from root
│   │   └── 08_PROPOSAL_ADVANCED_ANALYSIS.md         # MOVED: from root
│   ├── security/                  # STAYS
│   │   └── AUDIT.md
│   └── testing/                   # STAYS
│       ├── CROSS_LANGUAGE_TESTING.md
│       └── INTEGRATION_GUIDE.md   # MOVED: from integration-tests/TESTING_GUIDE.md
├── deployment/                    # STAYS
│   └── docker/
│       ├── agent.py
│       ├── docker-compose.production.yml
│       ├── docker-compose.yml
│       ├── Dockerfile
│       ├── nginx.conf
│       └── README.md
├── scripts/                       # STAYS
│   ├── check-duplicates.sh        # STAYS
│   ├── check-features.sh          # MOVED: from crates/languages/ + EDITED: path refs
│   ├── install.sh                 # STAYS
│   ├── new-lang.sh                # MOVED: from crates/languages/ + EDITED: path refs
│   └── setup-dev-tools.sh         # STAYS
├── API.md                         # STAYS
├── CHANGELOG.md                   # STAYS
├── CLAUDE.md                      # EDITED: update all path references
├── codebuddy.example.toml         # STAYS
├── codebuddy.toml                 # STAYS
├── CONTRIBUTING.md                # STAYS
├── GEMINI.md -> CLAUDE.md         # STAYS
├── justfile                       # EDITED: update benchmark paths, test paths
├── LICENSE                        # STAYS
├── Makefile                       # EDITED: update benchmark paths, test paths
├── README.md                      # STAYS
├── rust-toolchain.toml            # STAYS
├── SECURITY.md                    # STAYS
└── vm.yaml                        # STAYS

# REMOVED directories:
# - apps/                          # Removed after moving codebuddy to crates/
# - benchmarks/                    # Removed after moving to crates/codebuddy-bench/
# - crates/languages/              # Removed after flattening language crates
# - integration-tests/             # Removed after splitting into test-support + crates/codebuddy/tests/
# - tests/                         # Was empty, not recreated

# REMOVED files:
# - install.sh (root)              # Duplicate of scripts/install.sh
# - test_scanner (root)            # Moved to scripts/ or removed
# - 00_PROPOSAL_*.md (6 files)     # Moved to docs/proposals/
# - FAILING_TESTS.md (root)        # Moved to docs/development/
```

## Key Benefits

1. **Rust-standard flat workspace** - All crates at `crates/*` level
2. **Integration tests in binary crate** - `crates/codebuddy/tests/` per Cargo convention
3. **Language config centralized** - `config/languages/languages.toml` for build scripts
4. **Documentation organized** - Proposals, testing guides, language docs properly filed
5. **Clean root** - Only essential files, no proposal docs or duplicate scripts

## Migration Notes

- All `git mv` operations preserve history
- Build scripts need path updates: `../languages/languages.toml` → `../config/languages/languages.toml`
- CI/Make/just files need benchmark path updates: `benchmarks` → `crates/codebuddy-bench`
- Integration tests import: `use test_support::*;`
- Test-support crate marked `publish = false`

---

## Execution Plan Using CodeBuddy MCP Tools

**Note:** All file operations use CodeBuddy MCP tools with automatic import updates. The `rename_directory` and `rename_file` tools automatically update all imports across the workspace - **no manual file edits needed**.

### Phase 1: Flatten Language Crates
**Goal:** Promote language crates to top level

**MCP Tool:** `batch_execute`

```json
{
  "operations": [
    {"type": "rename_file", "old_path": "crates/languages/languages.toml", "new_path": "config/languages/languages.toml"},
    {"type": "rename_file", "old_path": "crates/languages/check-features.sh", "new_path": "scripts/check-features.sh"},
    {"type": "rename_file", "old_path": "crates/languages/new-lang.sh", "new_path": "scripts/new-lang.sh"},
    {"type": "rename_file", "old_path": "crates/languages/README.md", "new_path": "docs/development/languages/README.md"},
    {"type": "rename_file", "old_path": "crates/languages/SCAFFOLDING.md", "new_path": "docs/development/languages/SCAFFOLDING.md"}
  ]
}
```

**MCP Tool:** `rename_directory` (6 separate calls - cannot batch directories)

```bash
rename_directory: crates/languages/cb-lang-common → crates/cb-lang-common
rename_directory: crates/languages/cb-lang-go → crates/cb-lang-go
rename_directory: crates/languages/cb-lang-java → crates/cb-lang-java
rename_directory: crates/languages/cb-lang-python → crates/cb-lang-python
rename_directory: crates/languages/cb-lang-rust → crates/cb-lang-rust
rename_directory: crates/languages/cb-lang-typescript → crates/cb-lang-typescript
```

**Validation:** `cargo check --workspace`

---

### Phase 2: Reorganize Workspace Crates
**Goal:** Move main binary, rename benchmarks

**MCP Tool:** `rename_directory` (2 separate calls)

```bash
rename_directory: apps/codebuddy → crates/codebuddy
rename_directory: benchmarks → crates/codebuddy-bench
```

**Manual Edits Required:**
```bash
# Update root Cargo.toml (virtual manifest)
# Update crates/codebuddy-bench/Cargo.toml (package name)
```

**Validation:** `cargo check --workspace`

---

### Phase 3: Split Integration Tests
**Goal:** Create test-support crate, reorganize tests

**MCP Tool:** `batch_execute` (create test-support structure)

```json
{
  "operations": [
    {"type": "create_file", "path": "crates/test-support/Cargo.toml", "content": "[package]\nname = \"test-support\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\n# Add as needed\n"},
    {"type": "create_file", "path": "crates/test-support/src/lib.rs", "content": "pub mod harness;\npub mod helpers;\npub mod mocks;\n\npub fn fixtures_dir() -> std::path::PathBuf {\n    std::path::PathBuf::from(env!(\"CARGO_MANIFEST_DIR\")).join(\"fixtures\")\n}\n"}
  ]
}
```

**MCP Tool:** `rename_directory` (move test utilities)

```bash
rename_directory: integration-tests/src/harness → crates/test-support/src/harness
rename_directory: integration-tests/fixtures → crates/test-support/fixtures
rename_directory: integration-tests/tests → crates/codebuddy/tests
```

**MCP Tool:** `batch_execute` (move individual files)

```json
{
  "operations": [
    {"type": "rename_file", "old_path": "integration-tests/src/helpers.rs", "new_path": "crates/test-support/src/helpers.rs"},
    {"type": "rename_file", "old_path": "integration-tests/src/mocks.rs", "new_path": "crates/test-support/src/mocks.rs"},
    {"type": "rename_file", "old_path": "integration-tests/TESTING_GUIDE.md", "new_path": "docs/testing/INTEGRATION_GUIDE.md"}
  ]
}
```

**Manual Edits Required:**
```bash
# Update crates/codebuddy/Cargo.toml (add test-support dev-dependency)
# Update test files to use `use test_support::*;`
```

**Validation:** `cargo test --workspace`

---

### Phase 4: Organize Documentation
**Goal:** Clean up root, organize proposals

**MCP Tool:** `batch_execute`

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

**Manual Edits Required:**
```bash
# Update CLAUDE.md with new paths
# Update Makefile/justfile with new benchmark paths
# Update .github/workflows/*.yml with new paths
```

**Validation:**
```bash
cargo fmt
cargo clippy --workspace
cargo test --workspace
cargo bench -p codebuddy-bench --no-run
```

---

## Validation Checklist

After each phase, run:
- ✅ `cargo check --workspace` - Verify compilation
- ✅ `cargo test --workspace` - Verify tests pass
- ✅ `git status` - Review changed files

Final validation:
- ✅ `cargo fmt`
- ✅ `cargo clippy --workspace -- -D warnings`
- ✅ `cargo test --workspace --verbose`
- ✅ `cargo bench -p codebuddy-bench --no-run`
- ✅ `cargo build --release -p codebuddy`
- ✅ `./target/release/codebuddy status`

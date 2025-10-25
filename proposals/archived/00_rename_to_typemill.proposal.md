# Proposal 00: Rename Project to TypeMill

> **🐶 DOGFOODING NOTE**: This proposal demonstrates using TypeMill's own CLI commands to perform the rename operation. All file movements, symbol renames, and refactoring operations will be executed using the `mill` CLI (via `rename.plan`, `move.plan`, and `workspace.apply_edit` commands) rather than manual text replacement. This serves as both a practical implementation guide and a validation of TypeMill's LSP-backed refactoring capabilities on a real-world, complex codebase.

**Status**: In Progress (Phase 3 Complete ✅ - Moving to Phase 4)
**Author**: Project Team
**Date**: 2025-10-20
**Updated**: 2025-10-23
**Current Name**: `mill` / `mill` CLI
**Proposed Name**: `typemill` / `mill` CLI

## Progress Update (2025-10-23)

**Completed:**
- ✅ **Phase 3: All Crate Renames Complete** (9/9 crates)
  - **Language Plugins (3/3):**
    - `cb-lang-rust` → `mill-lang-rust` (commit 4f12e96b, fab598ae)
    - `cb-lang-typescript` → `mill-lang-typescript` (commit 65aca23e)
    - `cb-lang-yaml` → `mill-lang-yaml` (commit 4f12e96b)
  - **Analysis Crates (5/5):**
    - `cb-analysis-common` → `mill-analysis-common` ✅
    - `cb-analysis-dead-code` → `mill-analysis-dead-code` ✅
    - `cb-analysis-deep-dead-code` → `mill-analysis-deep-dead-code` ✅
    - `cb-analysis-graph` → `mill-analysis-graph` ✅
    - `cb-analysis-circular-deps` → `mill-analysis-circular-deps` ✅
  - All workspace members, dependencies, and config files updated
  - Build passes without manual intervention

- ✅ **CLI API Cleanup** (commit f3e90d7e)
  - Removed 5 legacy/redundant flags (pre-release cleanup)
  - Clean minimal API: `--scope`, `--update-comments`, `--update-markdown-prose`, `--update-all`
  - Default scope now comprehensive (code + docs + configs + exact matches)

- ✅ **Rename Tool Bug Fixes** (commit 5a901441)
  - Fixed workspace members formatting mismatch
  - Added cargo flag recognition for .cargo/config.toml
  - Fixed batch mode edit conflicts (deduplication)
  - Enabled `update_exact_matches` in default scope

**Next Steps:**
- **Phase 4**: Application binary rename (../apps/mill → apps/mill)
- **Phase 5**: Configuration directory path updates
- **Phase 6**: Plugin macro rename
- **Phase 7**: Test fixture updates
- **Phase 8**: Environment variable updates
- **Phase 9**: Documentation and string literals
- **Phase 10**: Infrastructure files
- **Phase 11**: Validation
- **Phase 12**: Release

---

## Executive Summary

This proposal outlines the complete strategy for renaming the project from **TypeMill** to **TypeMill**, with the CLI command changing from `mill` to `mill`. The rename encompasses **31+ Rust crates**, comprehensive documentation, infrastructure configuration, user-facing interfaces, macros, and test fixtures, executed as a major version bump to **v2.0.0**.

---

## Motivation

### Why "TypeMill"?

1. **Better Reflects Core Functionality**
   - The project is fundamentally a "mill" that processes and refines code through LSP servers
   - "Type" emphasizes the strong type-safety focus (LSP intelligence, static analysis, refactoring)
   - Metaphor: A mill processes raw materials into refined products; TypeMill processes code into better code

2. **Stronger Brand Identity**
   - "mill" is generic and conflicts with existing tools/services
   - "typemill" is distinctive and memorable
   - Conveys professionalism and precision

3. **CLI Ergonomics**
   - `mill` is short, fast to type (4 characters vs 9)
   - Follows Unix tradition of concise commands (`git`, `grep`, `sed`, `make`)
   - Natural verb-like quality: "mill the code", "run the mill"

4. **Technical Alignment**
   - Emphasizes the "grinding/processing" nature of the tool
   - "Type" connects to type systems, TypeScript support, and static analysis
   - Better SEO and searchability in developer tools space

5. **Domain Assets**
   - Project owns both `typemill.org` and `typemill.com`
   - `.org` hosts the open-source mill CLI and documentation
   - `.com` reserved for future commercial offerings
   - Complete brand protection and clear product positioning

---

## Current State Inventory

### Workspace Structure (31+ Crates)

**Core Infrastructure Crates (15 crates - `mill-*` prefix):**
- `mill-client` - CLI client implementation
- `mill-handlers` - MCP tool handler implementations
- `mill-lsp` - LSP client and server management
- `mill-server` - MCP server core
- `mill-services` - Core services (file, AST, planner, workflow)
- `mill-transport` - WebSocket and stdio transport
- `mill-plugin-api` - Plugin API definitions
- `mill-test-support` - Testing utilities and harness
- `mill-foundation` - Core foundation layer
- `mill-config` - Configuration system
- `mill-ast` - AST processing and manipulation
- `mill-auth` - Authentication and authorization
- `mill-workspaces` - Workspace management
- `mill-plugin-system` - Plugin system orchestration
- `mill-plugin-bundle` - Plugin bundle packaging

**Language Plugins (6 crates - `mill-lang-*` prefix):**
- `mill-lang-common` - Common language plugin infrastructure ✅
- `mill-lang-rust` - Rust language plugin ✅
- `mill-lang-typescript` - TypeScript/JavaScript plugin ✅
- `mill-lang-markdown` - Markdown documentation plugin ✅
- `mill-lang-toml` - TOML configuration plugin ✅
- `mill-lang-yaml` - YAML configuration plugin ✅

**Analysis Crates (5 crates - `cb-analysis-*` prefix):**
- `cb-analysis-common` - Common analysis utilities *(needs rename)*
- `cb-analysis-dead-code` - Dead code detection *(needs rename)*
- `cb-analysis-deep-dead-code` - Deep dead code analysis *(needs rename)*
- `cb-analysis-graph` - Dependency graph analysis *(needs rename)*
- `cb-analysis-circular-deps` - Circular dependency detection *(needs rename)*

**Applications:**
- `../apps/mill` - Main binary application (produces `mill` executable) *(needs rename)*

**Development Tools:**
- `crates/xtask` - Build automation tasks

**TOTAL: 31+ crates** (21 mill-* renamed ✅, 5 cb-analysis-* needing rename, 1 app, 1 dev tool)

### Additional Rename Targets

**Plugin Registration Macro:**
- `mill_plugin!` → `typemill_plugin!` (or `mill_plugin!`)
  - Location: `crates/mill-plugin-api/src/plugin_registry.rs`
  - Used in: All language plugins for self-registration
  - Impact: Requires updates in 6+ plugin files

**Test Fixtures:**
- `tests/e2e/test-fixtures/rust/Cargo.toml` - Package: `mill-playground` → `mill-playground`
- `tests/e2e/test-fixtures/python/pyproject.toml` - Package: `mill-playground-python` → `mill-playground-python`
- `crates/mill-test-support/src/harness/fixtures.rs` - Java package: `com.mill.example` → `com.mill.example`

**Configuration Files:**
- `mill.toml` → `typemill.toml` (main configuration file)
- `mill.example.toml` → `typemill.example.toml` (example configuration)

**Scripts and Shell Files (10+ files):**
- `install.sh` - Main installation script
- `scripts/install.sh` - Script directory installation
- `scripts/new-lang.sh` - New language plugin scaffolding
- `.typemill/start-with-lsp.sh` - LSP startup script
- `examples/setup/install.sh` - Example setup
- Various debug scripts in `.debug/` directory

**Repository Metadata:**
- Repository URL: `https://github.com/goobits/typemill` → `https://github.com/goobits/typemill`
- Homepage: Same as repository
- Appears in: Root `Cargo.toml` + 31+ crate `Cargo.toml` files

---

## Scope of Changes

### 1. Crate and Package Names

**Status: 15/31 crates already use `mill-*` prefix**

**Crates Already Using `mill-*` Prefix (No rename needed - 15 crates):**
- `mill-client` ✓
- `mill-handlers` ✓
- `mill-lsp` ✓
- `mill-server` ✓
- `mill-services` ✓
- `mill-transport` ✓
- `mill-plugin-api` ✓
- `mill-test-support` ✓
- `mill-lang-common` ✓
- `mill-lang-markdown` ✓
- `mill-lang-toml` ✓
- `mill-foundation` ✓
- `mill-config` ✓
- `mill-ast` ✓
- `mill-auth` ✓
- `mill-workspaces` ✓
- `mill-plugin-system` ✓
- `mill-plugin-bundle` ✓

**Language Plugins Needing Rename (3 crates):**
- `../crates/mill-lang-rust` → `crates/mill-lang-rust`
- `../crates/mill-lang-typescript` → `crates/mill-lang-typescript`
- `../crates/mill-lang-yaml` → `crates/mill-lang-yaml`

**Analysis Crates Needing Rename (5 crates):**
- `../analysis/mill-analysis-common` → `analysis/mill-analysis-common`
- `../analysis/mill-analysis-dead-code` → `analysis/mill-analysis-dead-code`
- `../analysis/mill-analysis-deep-dead-code` → `analysis/mill-analysis-deep-dead-code`
- `../analysis/mill-analysis-graph` → `analysis/mill-analysis-graph`
- `../analysis/mill-analysis-circular-deps` → `analysis/mill-analysis-circular-deps`

**Application Needing Rename (1 crate):**
- `../apps/mill` → `apps/mill`
  - Binary name: `mill` → `mill`

**Development Tools (No rename - 1 crate):**
- `crates/xtask` (unchanged - internal development tool)

**Total Crates Needing Rename: 9 crates** (3 language plugins + 5 analysis + 1 app)

---

### 2. CLI Commands

**Primary Command:**
```bash
# Old
mill setup
mill status
mill start
mill serve

# New
mill setup
mill status
mill start
mill serve
```

**All Subcommands:**
- `mill setup` - Smart setup with auto-detection
- `mill status` - Show current status
- `mill start` - Start MCP server
- `mill stop` - Stop MCP server
- `mill serve` - Start WebSocket server
- `mill link` - Link to AI assistants
- `mill unlink` - Remove AI from config
- `mill --version` - Show version

---

### 3. Plugin System Macro

**Macro Rename:**
- `mill_plugin!` → `mill_plugin!` (or `typemill_plugin!`)

**Definition Location:**
- `crates/mill-plugin-api/src/plugin_registry.rs`

**Usage Sites (6+ files):**
- `crates/mill-lang-rust/src/lib.rs`
- `../crates/mill-lang-rust/src/lib.rs`
- `crates/mill-lang-typescript/src/lib.rs`
- `../crates/mill-lang-typescript/src/lib.rs`
- `crates/mill-lang-markdown/src/lib.rs`
- `crates/mill-lang-toml/src/lib.rs`
- `../crates/mill-lang-yaml/src/lib.rs`

**Example Change:**
```rust
// Old
mill_plugin!(
    name: "rust",
    extensions: ["rs"],
    // ...
)

// New
mill_plugin!(
    name: "rust",
    extensions: ["rs"],
    // ...
)
```

---

### 4. Configuration and Paths

**Configuration Directory:**
- `.typemill/` → `.typemill/`
- `.typemill/config.json` → `.typemill/config.json`
- `.typemill/analysis.toml` → `.typemill/analysis.toml`
- `.typemill/workflows.json` → `.typemill/workflows.json`

**Configuration Files:**
- `mill.toml` → `typemill.toml`
- `mill.example.toml` → `typemill.example.toml`

**Binary Path:**
- `target/release/mill` → `target/release/mill`
- `/usr/local/bin/mill` → `/usr/local/bin/mill`
- `~/.local/bin/mill` → `~/.local/bin/mill`

---

### 5. Environment Variables

**Prefix Migration:**
- `TYPEMILL__*` (multilevel config) → `TYPEMILL__*`
- `TYPEMILL_*` (CLI/runtime helpers) → `TYPEMILL_*`

**Examples:**
- `TYPEMILL_DISABLE_CACHE` → `TYPEMILL_DISABLE_CACHE`
- `TYPEMILL_DISABLE_AST_CACHE` → `TYPEMILL_DISABLE_AST_CACHE`
- `TYPEMILL_DISABLE_IMPORT_CACHE` → `TYPEMILL_DISABLE_IMPORT_CACHE`

**Migration Strategy:**
- Maintain dual-read support for legacy variables for **two release cycles**
- Emit structured `warn!` logs when legacy variables are detected
- Provide migration helper: `mill env migrate` to rewrite `.env`/shell exports
- Update docs and examples to prefer new prefix while noting backward compatibility

---

### 6. Test Fixtures and Examples

**Test Playground Packages:**
- `tests/e2e/test-fixtures/rust/Cargo.toml`:
  - Package name: `mill-playground` → `mill-playground`
- `tests/e2e/test-fixtures/python/pyproject.toml`:
  - Package name: `mill-playground-python` → `mill-playground-python`

**Test Support Fixtures:**
- `crates/mill-test-support/src/harness/fixtures.rs`:
  - Java package: `com.mill.example` → `com.mill.example`

**Impact:** These fixtures are used in integration tests and need updating to prevent test failures.

---

### 7. Documentation Updates

**Critical Files:**
- `README.md` - Project name, CLI examples, installation
- `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` - All references to project name and CLI
- `docs/api_reference.md` - Package names and examples
- `docs/tools_catalog.md` - Tool examples
- `CONTRIBUTING.md` - Development workflow references
- `CHANGELOG.md` - Historical context and version history
- All `docs/**/*.md` files
- `Cargo.toml` - Package metadata
- `mill.toml` / `mill.example.toml` → `typemill.toml` / `typemill.example.toml`

**Example Updates:**
```bash
# Old examples
cargo run --bin mill
./target/release/mill setup

# New examples
cargo run --bin mill
./target/release/mill setup
```

---

### 8. Code References

**Rust Code:**
- Module imports: `use mill::*` → `use mill::*`
- Binary targets in `Cargo.toml`: `[[bin]] name = "mill"` → `[[bin]] name = "mill"`
- Error messages and help text
- Log messages mentioning project name
- String literals with `.typemill` paths → `.typemill`

**Configuration Examples:**
- JSON schema references
- Sample configurations
- Docker compose files
- Test fixtures

---

### 9. Infrastructure

**Docker:**
- Image names: `mill:latest` → `mill:latest`
- Container names: `mill-dev` → `mill-dev`
- Volume mount paths
- Docker compose service names

**GitHub/CI:**
- Workflow files: `.github/workflows/mill-ci.yml` → `.github/workflows/mill-ci.yml`
- Release artifact names
- Repository name consideration (with automatic redirect)

**Scripts:**
- `scripts/install.sh` - Update binary name and paths
- Build automation scripts

---

## Implementation Strategy: CLI-Based Dogfooding

This rename operation will dogfood TypeMill's own refactoring capabilities using the CLI interface.

### Phase 2: Discovery & Analysis

Use TypeMill's own CLI to discover all references:

```bash
# Find all symbol references to "mill"
mill search_symbols --query "mill"

# Analyze dependency graph
mill analyze.dependencies --kind graph --scope workspace

# Find all .typemill path references
rg "\.typemill" --files-with-matches

# Find all TYPEMILL_ environment variables
rg "TYPEMILL_" --files-with-matches

# Find all string literals in code
rg '"mill"' --type rust
rg "'mill'" --type rust
```

### Phase 3: Crate Renames (9 Crates) - ✅ COMPLETE

**All 9 crates successfully renamed to `mill-*` prefix!**

- ✅ Language Plugins (3/3):
  - `cb-lang-rust` → `mill-lang-rust`
  - `cb-lang-typescript` → `mill-lang-typescript`
  - `cb-lang-yaml` → `mill-lang-yaml`

- ✅ Analysis Crates (5/5):
  - `cb-analysis-common` → `mill-analysis-common`
  - `cb-analysis-dead-code` → `mill-analysis-dead-code`
  - `cb-analysis-deep-dead-code` → `mill-analysis-deep-dead-code`
  - `cb-analysis-graph` → `mill-analysis-graph`
  - `cb-analysis-circular-deps` → `mill-analysis-circular-deps`

**NOTE:** 18 crates already used `mill-*` prefix and didn't need renaming.

### Phase 4: Binary and App Rename

```bash
# Rename ../apps/mill → apps/mill
mill rename.plan \
  --target directory:../apps/mill \
  --new-name apps/mill

mill workspace.apply_edit --plan <plan>

# Manual edit: Update binary name in apps/mill/Cargo.toml
# [[bin]]
# name = "mill"
```

### Phase 5: Configuration Directory Path Updates

```bash
# Find all .typemill references
rg "\.typemill" --files-with-matches

# Manual code edits:
# - Update path constants
# - Add dual-path support (.typemill/ with .typemill/ fallback)
# - Add migration warnings

# Files to edit:
# - crates/mill-config/src/config.rs
# - crates/mill-client/src/client_config.rs
# - crates/mill-foundation/src/core/tests/acceptance_config.rs
```

### Phase 6: Plugin Macro Rename

```bash
# Find all mill_plugin! usages
rg "mill_plugin!" --files-with-matches

# Manual code edits required:
# 1. Update macro definition in crates/mill-plugin-api/src/plugin_registry.rs
#    - Rename `mill_plugin!` → `mill_plugin!`
#    - Keep macro_export attribute
#    - Update any internal references

# 2. Update all plugin invocations (6+ files):
#    - crates/mill-lang-rust/src/lib.rs
#    - ../crates/mill-lang-rust/src/lib.rs
#    - crates/mill-lang-typescript/src/lib.rs
#    - ../crates/mill-lang-typescript/src/lib.rs
#    - crates/mill-lang-markdown/src/lib.rs
#    - crates/mill-lang-toml/src/lib.rs
#    - ../crates/mill-lang-yaml/src/lib.rs

# Search and replace pattern:
# mill_plugin!( → mill_plugin!(
```

### Phase 7: Test Fixture Updates

```bash
# Update test playground packages
# tests/e2e/test-fixtures/rust/Cargo.toml
sed -i 's/mill-playground/mill-playground/g' tests/e2e/test-fixtures/rust/Cargo.toml

# tests/e2e/test-fixtures/python/pyproject.toml
sed -i 's/mill-playground-python/mill-playground-python/g' tests/e2e/test-fixtures/python/pyproject.toml

# crates/mill-test-support/src/harness/fixtures.rs
sed -i 's/com.mill.example/com.mill.example/g' crates/mill-test-support/src/harness/fixtures.rs
```

### Phase 8: Environment Variable Updates

```bash
# Find all TYPEMILL_ references
rg "TYPEMILL_" --files-with-matches

# Manual code edits:
# - Extend config loaders to check TYPEMILL_* first, fallback to TYPEMILL_*
# - Add structured warnings for legacy prefixes
# - Update documentation

# Create migration helper:
# - Implement `mill env migrate` CLI command
```

### Phase 9: Documentation and String Literals

```bash
# Update all markdown files
fd -e md -x sed -i 's/mill/mill/g' {} \;
fd -e md -x sed -i 's/TypeMill/TypeMill/g' {} \;

# Update TOML files
fd -e toml -x sed -i 's/mill/mill/g' {} \;

# Update YAML files
fd -e yaml -e yml -x sed -i 's/mill/mill/g' {} \;

# Update shell scripts
fd -e sh -x sed -i 's/mill/mill/g' {} \;

# Manual review required for:
# - README.md
# - CLAUDE.md / AGENTS.md / GEMINI.md
# - CONTRIBUTING.md
# - All docs/**/*.md
# - install.sh and scripts/
```

### Phase 10: Infrastructure Files

```bash
# Docker files
sed -i 's/mill/mill/g' deployment/docker/Dockerfile
sed -i 's/mill/mill/g' deployment/docker/docker-compose*.yml

# GitHub workflows
sed -i 's/mill/mill/g' .github/workflows/*.yml

# Scripts
sed -i 's/mill/mill/g' scripts/install.sh
```

### Phase 11: Validation

```bash
# Full rebuild
cargo clean
cargo build --release

# Verify new binary
./target/release/mill --version

# Run full test suite
cargo nextest run --workspace --all-features

# Check for diagnostics
./target/release/mill get_diagnostics --scope workspace

# Analyze dead code
./target/release/mill analyze.dead_code --kind unused_imports --scope workspace

# Analyze dependencies
./target/release/mill analyze.dependencies --kind circular --scope workspace
```

### Phase 12: Documentation and Release

```bash
# Update CHANGELOG.md
# - Add [2.0.0] section
# - Document BREAKING CHANGES
# - Reference MIGRATION.md

# Create MIGRATION.md guide

# Update version in Cargo.toml
# version = "2.0.0"

# Commit and tag
git add .
git commit -m "feat: Rename project to TypeMill (mill CLI)

BREAKING CHANGES:
- Project renamed from TypeMill to TypeMill
- CLI command changed from 'mill' to 'mill'
- All crates renamed from cb-*/mill-* to mill-*
- Config directory changed from .typemill/ to .typemill/
- Environment variables changed from TYPEMILL_* to TYPEMILL_*

See MIGRATION.md for detailed migration guide.

🤖 Generated with TypeMill dogfooding

Co-Authored-By: TypeMill <noreply@typemill.org>"

git tag v2.0.0
```

---

## Detailed Checklists

### Crate Rename Checklist - ✅ COMPLETE

**NOTE:** 18 crates already use `mill-*` prefix ✓

**Language Plugins (3 crates):**
- [x] `cb-lang-rust` → `mill-lang-rust` ✅
- [x] `cb-lang-typescript` → `mill-lang-typescript` ✅
- [x] `cb-lang-yaml` → `mill-lang-yaml` ✅

**Analysis (5 crates):**
- [x] `cb-analysis-common` → `mill-analysis-common` ✅
- [x] `cb-analysis-dead-code` → `mill-analysis-dead-code` ✅
- [x] `cb-analysis-deep-dead-code` → `mill-analysis-deep-dead-code` ✅
- [x] `cb-analysis-graph` → `mill-analysis-graph` ✅
- [x] `cb-analysis-circular-deps` → `mill-analysis-circular-deps` ✅

**Applications:**
- [ ] `../apps/mill` → `apps/mill` (including binary name)

**After each rename:**
- [ ] Validate with `get_diagnostics`
- [ ] Check imports updated correctly
- [ ] Verify Cargo.toml workspace members

### Plugin Macro Updates

- [ ] Update macro definition: `mill_plugin!` → `mill_plugin!`
- [ ] Update macro invocations in 6+ plugin files
- [ ] Verify all plugins still register correctly after rename
- [ ] Test plugin system works with new macro name

### Test Fixture Updates

- [ ] Update `tests/e2e/test-fixtures/rust/Cargo.toml` package name
- [ ] Update `tests/e2e/test-fixtures/python/pyproject.toml` package name
- [ ] Update Java package references in test support fixtures
- [ ] Run integration tests to verify fixtures work

### Configuration and Path Updates

- [ ] Update `.typemill/` path references to `.typemill/`
- [ ] Implement dual-path support (.typemill/ primary, .typemill/ fallback)
- [ ] Add migration warnings for legacy paths
- [ ] Update path constants in code
- [ ] Update configuration file names (mill.toml → typemill.toml)
- [ ] Update `.typemill/workflows.json` → `.typemill/workflows.json`

### Environment Variable Updates

- [ ] Find all TYPEMILL_* references
- [ ] Update config loaders for dual-prefix support (TYPEMILL_* + TYPEMILL_* fallback)
- [ ] Add structured warnings for legacy prefixes
- [ ] Implement `mill env migrate` CLI command
- [ ] Update documentation with new environment variables

### Binary and CLI Updates

- [ ] Update binary name in Cargo.toml: `name = "mill"`
- [ ] Update CLI help text and error messages
- [ ] Update version display
- [ ] Test all subcommands work with new name

### Documentation Updates

- [ ] `README.md` - Update project name, examples, installation
- [ ] `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` - Update all references
- [ ] `docs/api_reference.md` - Update package names
- [ ] `docs/tools_catalog.md` - Update examples
- [ ] `docs/quickstart.md` - Update CLI commands
- [ ] `CONTRIBUTING.md` - Update development workflow
- [ ] All `docs/**/*.md` files
- [ ] `CHANGELOG.md` - Add v2.0.0 entry

### Infrastructure Updates

- [ ] Docker: Update Dockerfiles
- [ ] Docker: Update docker-compose.yml files
- [ ] Docker: Update image names and tags
- [ ] CI/CD: Update GitHub Actions workflows
- [ ] CI/CD: Update workflow file names
- [ ] Scripts: Update install.sh
- [ ] Scripts: Update any automation scripts

### Validation and Testing

- [ ] Full clean rebuild: `cargo clean && cargo build --release`
- [ ] Verify binary works: `./target/release/mill --version`
- [ ] Run full test suite: `cargo nextest run --workspace --all-features`
- [ ] Check diagnostics: No errors in workspace
- [ ] Analyze dead code: No unused imports introduced
- [ ] Analyze dependencies: No circular dependencies introduced
- [ ] Test migration path: Verify .typemill/ → .typemill/ auto-migration works
- [ ] Test environment variable fallback: Legacy TYPEMILL_* vars still work

### Release Preparation

- [ ] Create MIGRATION.md guide
- [ ] Update CHANGELOG.md with v2.0.0 entry
- [ ] Update version to 2.0.0 in Cargo.toml
- [ ] Commit with detailed message
- [ ] Tag release: `git tag v2.0.0`
- [ ] Test installation from clean environment

---

## Migration Path for Users

### Automatic Migration

The tool will automatically detect and migrate on first run:

```bash
# User runs new version
mill setup

# Output:
# ℹ️  Detected legacy configuration directory: .typemill/
# 🔄 Migrating to .typemill/...
# ✅ Configuration migrated successfully
# 💡 Legacy .typemill/ directory preserved as backup
```

### Manual Migration

Users can manually migrate:

```bash
# Backup old config
cp -r .typemill .typemill.backup

# Rename directory
mv .typemill .typemill

# Update scripts
sed -i 's/mill/mill/g' scripts/*.sh

# Update environment variables
mill env migrate  # Helper command to rewrite .env files
```

### Backward Compatibility

**Environment Variables:**
- Legacy `TYPEMILL_*` variables supported for **2 release cycles** (v2.0.0 - v2.2.0)
- Deprecation warning shown when legacy variables detected
- Removed in v3.0.0

**Configuration Directory:**
- `.typemill/` fallback supported indefinitely (read-only)
- New configurations written to `.typemill/` only
- Migration prompt shown on first run

**CLI Command:**
- Optional: Create `mill` symlink to `mill` for 2-3 releases
- Show deprecation warning when symlink used
- Remove in v3.0.0

---

## Breaking Changes

### For End Users

1. **CLI Command Change**
   - All scripts using `mill` must change to `mill`
   - Shell aliases and shortcuts need updating
   - CI/CD pipelines need updating

2. **Configuration Directory**
   - `.typemill/` → `.typemill/`
   - Automatic migration provided on first run
   - Legacy directory still read as fallback

3. **Binary Name**
   - Installation paths change
   - System PATH may need adjustment
   - Docker images renamed

4. **Environment Variables**
   - `TYPEMILL_*` → `TYPEMILL_*`
   - Legacy variables work with deprecation warning (2 release cycles)
   - Use `mill env migrate` for automatic updates

### For Developers/Contributors

1. **Import Paths**
   - All `use mill::*` → `use mill::*`
   - All `use cb_*::*` → `use mill_*::*`
   - Crate dependencies updated in Cargo.toml

2. **Crate Names**
   - All `cb-*` → `mill-*`
   - All `mill-*` → `mill-*`
   - Affects plugin development and extensions

3. **Repository Structure**
   - Directory names changed under `crates/`
   - Update local development setups
   - Update git submodule references (if any)

---

## Risks and Mitigations

### Risk 1: User Confusion During Transition
**Impact**: Medium
**Mitigation**:
- Clear migration guide (MIGRATION.md)
- Deprecation warnings in CLI output
- Comprehensive changelog
- Keep environment variable backward compatibility
- Optional: `mill` → `mill` symlink for transition period

### Risk 2: Broken CI/CD Pipelines
**Impact**: High
**Mitigation**:
- Document breaking changes prominently in CHANGELOG
- Provide migration examples for common CI setups
- Test migration in example repositories
- Announce breaking change ahead of release

### Risk 3: SEO and Discoverability Loss
**Impact**: Low
**Mitigation**:
- Redirect old documentation URLs to new ones
- Update all external references (GitHub, crates.io)
- Maintain repository redirects
- Update social media and community channels

### Risk 4: Build System Disruption
**Impact**: Medium
**Mitigation**:
- Comprehensive testing before merge
- Git tag before rename for easy rollback
- Staged validation (build → test → lint → integration tests)
- Clear rollback plan documented

### Risk 5: Incomplete Reference Updates
**Impact**: High
**Mitigation**:
- Use automated discovery (search_symbols, grep)
- Multi-pass validation (diagnostics, dead code analysis)
- Manual review of critical files
- Dogfooding validates tooling works correctly

---

## Success Criteria

- [ ] All 27 crates successfully renamed to `mill-*`
- [ ] Binary builds successfully as `mill`
- [ ] All tests pass: `cargo nextest run --workspace --all-features`
- [ ] No diagnostic errors in workspace
- [ ] No unused imports or dead code introduced
- [ ] No circular dependencies introduced
- [ ] CLI commands work with `mill` prefix
- [ ] Migration path tested: `.typemill/` → `.typemill/` works
- [ ] Environment variable fallback works: `TYPEMILL_*` → `TYPEMILL_*`
- [ ] Documentation 100% updated and accurate
- [ ] Docker builds succeed with new names
- [ ] Installation script works for fresh installs
- [ ] Users can successfully migrate from v1.x to v2.0

---

## Timeline Estimate

| Phase | Duration | Status | Description |
|-------|----------|--------|-------------|
| **Phase 2**: Discovery | 2-3 hours | ✅ Complete | Run discovery tools, inventory all references |
| **Phase 3**: Crate Renames | 6-8 hours | ✅ Complete | Rename all 9 crates using CLI |
| **Phase 4**: Binary Rename | 1 hour | 🔄 Next | Rename ../apps/mill → apps/mill |
| **Phase 5**: Config Paths | 2-3 hours | Pending | Update .typemill → .typemill references |
| **Phase 6**: Plugin Macro | 1-2 hours | Pending | Rename mill_plugin! → mill_plugin! |
| **Phase 7**: Test Fixtures | 1 hour | Pending | Update test playground packages |
| **Phase 8**: Environment Variables | 2-3 hours | Pending | Add dual-prefix support, migration helper |
| **Phase 9**: Documentation | 4-6 hours | Pending | Update all markdown, TOML, YAML files |
| **Phase 10**: Infrastructure | 2-3 hours | Pending | Docker, CI/CD, scripts |
| **Phase 11**: Validation | 3-4 hours | Pending | Build, test, diagnostics, analysis |
| **Phase 12**: Release Prep | 2-3 hours | Pending | MIGRATION.md, CHANGELOG.md, versioning |

**Total Estimate**: 22-33 hours (3-4 days of focused work)
**Completed**: ~8-11 hours (Phase 2-3)
**Remaining**: ~14-22 hours

---

## Open Questions

1. **Symlink Transition Period**: Should we create a `mill` → `mill` symlink for 2-3 releases?
   - Recommendation: **Optional** - Only if significant user feedback requests it

2. **Repository Name**: Should GitHub repository also be renamed?
   - Recommendation: **Yes**, with automatic redirect from old name

3. **Version Bump**: Confirm v2.0.0 for major breaking change?
   - Recommendation: **Yes** - CLI command change is major breaking change

4. **crates.io Publication**: Should we publish to crates.io under new names?
   - Recommendation: **Yes** - Claim `mill-*` crate names early
   - Consider: Deprecate old `cb-*` crates with migration notice

5. **Domain Launch Strategy**: How to launch typemill.org and typemill.com?
   - Recommendation: Launch **typemill.org immediately** with CLI docs
   - Point typemill.com to "coming soon" page

---

## Next Steps

1. ✅ **Team Review** - Get feedback on this proposal
2. ✅ **Finalize Open Questions** - Make decisions on symlink, repository name, etc.
3. ✅ **Schedule Implementation** - Block time for focused rename work
4. ✅ **Execute Phase 1** - Create branch and backup
5. ✅ **Begin Dogfooding** - Start with Phase 2 discovery
6. ✅ **Track Progress** - Update checklists as work progresses

---

## Appendix A: Critical Files Reference

**Cargo Manifests (29 files):**
- `/workspace/Cargo.toml` (workspace root)
- All crate Cargo.toml files (27 crates)
- Test fixture Cargo.toml files (optional)

**Documentation (15+ files):**
- `README.md`
- `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`
- `CONTRIBUTING.md`
- `CHANGELOG.md`
- `docs/api_reference.md`
- `docs/tools_catalog.md`
- `docs/quickstart.md`
- `docs/architecture/overview.md`
- `docs/operations/*.md`
- All other `docs/**/*.md`

**Configuration:**
- `mill.toml` → `typemill.toml`
- `mill.example.toml` → `typemill.example.toml`
- `.typemill/config.json` → `.typemill/config.json`
- `.typemill/analysis.toml` → `.typemill/analysis.toml`

**Infrastructure:**
- `Dockerfile`
- `docker-compose*.yml`
- `.github/workflows/*.yml`
- `scripts/install.sh`
- `vm.yaml`

**Source Code:**
- All `src/**/*.rs` files with imports
- All string literals referencing paths
- CLI help text and error messages

---

## Appendix B: CLI Commands Quick Reference

### Discovery Commands

```bash
# Find symbol references
mill search_symbols --query "mill"

# Analyze dependencies
mill analyze.dependencies --kind graph --scope workspace
mill analyze.dependencies --kind circular --scope workspace

# Find dead code
mill analyze.dead_code --kind unused_imports --scope workspace

# Get diagnostics
mill get_diagnostics --scope workspace
```

### Rename Commands

```bash
# Rename a crate directory (with dry-run preview)
mill rename.plan \
  --target directory:../crates/mill-client \
  --new-name crates/mill-client \
  --dry-run

# Apply the rename plan
mill workspace.apply_edit --plan <plan-json>

# Validate after rename
mill get_diagnostics --scope workspace
```

### Build and Test Commands

```bash
# Clean rebuild
cargo clean
cargo build --release

# Run tests
cargo nextest run --workspace --all-features

# Install locally
cargo xtask install
```

---

## Appendix C: Migration Guide Template

**MIGRATION.md** (to be created):

```markdown
# Migrating from TypeMill to TypeMill

Version 2.0.0 introduces a new name: **TypeMill** (CLI: `mill`)

## Quick Migration

### 1. Update CLI Command
```bash
# Old
mill setup

# New
mill setup
```

### 2. Configuration Auto-Migration
On first run, TypeMill will automatically migrate your configuration:
```bash
mill setup
# → Detects .typemill/ and migrates to .typemill/
```

### 3. Update Scripts
```bash
# Find and replace in your scripts
sed -i 's/mill/mill/g' scripts/*.sh
```

### 4. Update Environment Variables
```bash
# Use migration helper
mill env migrate

# Or manually rename:
# TYPEMILL_DISABLE_CACHE → TYPEMILL_DISABLE_CACHE
```

## Backward Compatibility

- Legacy `TYPEMILL_*` environment variables work until v3.0.0
- `.typemill/` directory read as fallback
- Deprecation warnings guide you to update

## Need Help?

See full documentation at https://typemill.org/docs/migration
```

---

## Appendix D: Changelog Entry Template

**CHANGELOG.md entry**:

```markdown
## [2.0.0] - 2025-XX-XX

### 🚀 BREAKING CHANGES

**Project Renamed to TypeMill**

- **CLI command**: `mill` → `mill`
- **Project name**: TypeMill → TypeMill
- **All crates renamed**: `cb-*` and `mill-*` → `mill-*`
- **Config directory**: `.typemill/` → `.typemill/`
- **Environment variables**: `TYPEMILL_*` → `TYPEMILL_*`

### Migration

- Run `mill setup` to automatically migrate configuration
- Update scripts to use `mill` command
- Update environment variables (or use `mill env migrate`)
- See [MIGRATION.md](MIGRATION.md) for detailed guide

### Backward Compatibility

- Legacy `TYPEMILL_*` environment variables supported until v3.0.0
- `.typemill/` configuration directory read as fallback
- Automatic migration on first run

### Internal Changes

- 27 crates renamed from `cb-*` / `mill-*` → `mill-*`
- Binary renamed from `mill` → `mill`
- All documentation updated
- Docker images renamed
- CI/CD workflows updated

🤖 Dogfooded using TypeMill's own refactoring tools
```

---

**End of Proposal**

---

## Approval Section

- [ ] **Approved by**: _____________
- [ ] **Date**: _____________
- [ ] **Ready for Implementation**: Yes / No
- [ ] **Concerns or Modifications**: _____________

---

**Status**: Ready for Review
**Next Review Date**: 2025-10-22
**Implementation Start**: After approval


--------

## APPENDIX E: Complete TypeMill Rename Summary

### Crate Directory Renames (9 crates needing rename)

**Language Plugins (3):**
- `../crates/mill-lang-rust` → `crates/mill-lang-rust`
- `../crates/mill-lang-typescript` → `crates/mill-lang-typescript`
- `../crates/mill-lang-yaml` → `crates/mill-lang-yaml`

**Analysis Crates (5):**
- `../analysis/mill-analysis-common` → `analysis/mill-analysis-common`
- `../analysis/mill-analysis-dead-code` → `analysis/mill-analysis-dead-code`
- `../analysis/mill-analysis-deep-dead-code` → `analysis/mill-analysis-deep-dead-code`
- `../analysis/mill-analysis-graph` → `analysis/mill-analysis-graph`
- `../analysis/mill-analysis-circular-deps` → `analysis/mill-analysis-circular-deps`

**Applications (1):**
- `../apps/mill` → `apps/mill`

**Already Renamed (18 crates using mill-* prefix):** ✓
- mill-client, mill-handlers, mill-lsp, mill-server, mill-services, mill-transport
- mill-plugin-api, mill-test-support, mill-lang-common, mill-lang-markdown, mill-lang-toml
- mill-foundation, mill-config, mill-ast, mill-auth, mill-workspaces, mill-plugin-system, mill-plugin-bundle

---

### Macro Renames (1 definition + 6+ usage sites)

**Macro Definition:**
- `mill_plugin!` → `mill_plugin!` (in crates/mill-plugin-api/src/plugin_registry.rs)

**Macro Usage Sites (6+):**
- All language plugin lib.rs files

---

### Test Fixture Renames (3 files)

- `tests/e2e/test-fixtures/rust/Cargo.toml` - Package: `mill-playground` → `mill-playground`
- `tests/e2e/test-fixtures/python/pyproject.toml` - Package: `mill-playground-python` → `mill-playground-python`
- `crates/mill-test-support/src/harness/fixtures.rs` - Java package: `com.mill.example` → `com.mill.example`

---

### Configuration & Path Renames

**Configuration Directory:**
- `.typemill/` → `.typemill/`
- `.typemill/config.json` → `.typemill/config.json`
- `.typemill/analysis.toml` → `.typemill/analysis.toml`
- `.typemill/workflows.json` → `.typemill/workflows.json`

**Configuration Files:**
- `mill.toml` → `typemill.toml`
- `mill.example.toml` → `typemill.example.toml`

**Binary Paths:**
- `target/release/mill` → `target/release/mill`
- `/usr/local/bin/mill` → `/usr/local/bin/mill`
- `~/.local/bin/mill` → `~/.local/bin/mill`

---

### Environment Variables (10+ variables)

**Cache Control:**
- `TYPEMILL_DISABLE_CACHE` → `TYPEMILL_DISABLE_CACHE`
- `TYPEMILL_DISABLE_AST_CACHE` → `TYPEMILL_DISABLE_AST_CACHE`
- `TYPEMILL_DISABLE_IMPORT_CACHE` → `TYPEMILL_DISABLE_IMPORT_CACHE`
- `TYPEMILL_DISABLE_LSP_METHOD_CACHE` → `TYPEMILL_DISABLE_LSP_METHOD_CACHE`

**Client/Server Config:**
- `TYPEMILL_URL` → `TYPEMILL_URL`
- `TYPEMILL_TOKEN` → `TYPEMILL_TOKEN`
- `TYPEMILL_TIMEOUT` → `TYPEMILL_TIMEOUT`
- `TYPEMILL__SERVER__PORT` → `TYPEMILL__SERVER__PORT`
- `TYPEMILL__LOGGING__LEVEL` → `TYPEMILL__LOGGING__LEVEL`
- `TYPEMILL__CACHE__ENABLED` → `TYPEMILL__CACHE__ENABLED`

---

### Documentation Files (Content updates, no renames)

**Core Documentation:**
- README.md, CLAUDE.md, AGENTS.md, GEMINI.md, CONTRIBUTING.md, CHANGELOG.md

**API & Tools Documentation:**
- docs/api_reference.md, docs/tools_catalog.md, docs/tools/*.md

**Architecture & Operations:**
- docs/architecture/*.md, docs/operations/*.md, docs/development/*.md

---

### Infrastructure Files

**CI/CD:**
- `.github/workflows/*.yml` - Update mill references to mill

**Docker:**
- Image names: `mill:latest` → `mill:latest`
- Container names: `mill-dev` → `mill-dev`
- Dockerfile and docker-compose files

**Scripts (10+ files):**
- `install.sh`, `scripts/install.sh`, `scripts/new-lang.sh`
- `.typemill/start-with-lsp.sh` → `.typemill/start-with-lsp.sh`
- `examples/setup/install.sh`
- Debug scripts in `.debug/` directory

---

### Repository Metadata (32+ files)

**GitHub URLs (in all Cargo.toml files):**
- `repository = "https://github.com/goobits/typemill"` → `"https://github.com/goobits/typemill"`
- `homepage = "https://github.com/goobits/typemill"` → `"https://github.com/goobits/typemill"`

**Appears in:**
- Root Cargo.toml + 31 crate Cargo.toml files

---

### Total Rename Operations Summary

| Category | Count |
|----------|-------|
| Crate directory renames | 9 |
| Macro renames (definition + usage) | 7+ |
| Test fixture updates | 3 |
| Configuration files/directories | 6 |
| Binary paths | 3 |
| Environment variables | 10+ |
| Documentation files (content) | 15+ |
| Infrastructure files | 5+ |
| Repository URLs (Cargo.toml) | 32 |
| **TOTAL OPERATIONS** | **90+** |

**Breakdown:**
- **9 directory renames** (automated via TypeMill's batch rename)
- **7+ macro updates** (manual search-replace)
- **3 test fixtures** (manual edits)
- **67+ configuration, path, and metadata updates** (mix of automated + manual)

**Automation Potential:**
- ~60% can be automated with TypeMill's own tools
- ~40% requires manual edits (macros, env vars, prose docs)

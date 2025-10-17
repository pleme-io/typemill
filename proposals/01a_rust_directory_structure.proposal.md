# Proposal 06: Rust-Optimized Directory Structure

**Status:** Draft
**Created:** 2025-10-13
**Author:** AI Assistant
**Tracking Issue:** TBD

## Summary

Reorganize the workspace to follow Rust ecosystem conventions and best practices, reducing root clutter and improving developer experience.

## Motivation

**Current Issues:**
- 20+ files in root directory (hard to navigate)
- Non-standard directory names (`tests/` vs Rust convention `tests/`)
- Documentation scattered across root and `docs/`
- Configuration files in multiple locations (`.config/`, `config/`, root)
- Unclear where to add new files

**Goals:**
1. Follow Rust ecosystem conventions
2. Reduce root directory clutter (20+ → ~10 files)
3. Improve discoverability for new contributors
4. Maintain compatibility with AI agents (AGENTS.md at root)

## Detailed Design

### Phase 1: Structural Changes (High Priority)

#### 1.1: Rename `tests/` → `tests/`

**Rust Convention:** Integration tests belong in `tests/` at workspace level.

```bash
mv tests tests
```

**Update references:**
```toml
# Cargo.toml
[workspace]
members = [
    # ... other members
-   "tests",
+   "tests",
]
```

**Files to update:**
- `Cargo.toml` - workspace members
- All `Cargo.toml` files that reference `tests` in dev-dependencies
- CI/CD workflows (`.github/workflows/*.yml`)
- Documentation references

**Impact:** Low risk, standard refactoring

---

#### 1.2: Consolidate Documentation

**Move root docs to `docs/` subdirectories:**

```bash
# User-facing docs
mkdir -p docs/getting-started docs/api docs/project
mv README.md docs/getting-started/
mv QUICK_REFERENCE.md docs/getting-started/
mv API_REFERENCE.md docs/api/reference.md
mv CHANGELOG.md docs/project/
mv SECURITY.md docs/project/
mv CONTRIBUTING.md docs/development/

# Keep at root (AI agents expect these)
# - AGENTS.md
# - CLAUDE.md (symlink)
# - GEMINI.md (symlink)
# - LICENSE
```

**Create documentation hub:**
```bash
touch docs/README.md
```

**Content for `docs/README.md`:**
```markdown
# Codebuddy Documentation

## Getting Started
- [README](getting-started/README.md) - Project overview
- [Quick Reference](getting-started/QUICK_REFERENCE.md) - One-page cheatsheet

## API Documentation
- [API Reference](api/reference.md) - Complete MCP tools API

## Development
- [Contributing Guide](development/contributing.md) - How to contribute
- [Testing Guide](development/testing.md) - Testing architecture
- [Logging Guidelines](development/logging.md) - Structured logging

## Architecture
- [System Architecture](architecture/overview.md) - Component design
- [Crate Structure](architecture/crate-deps.png) - Dependency graph

## Deployment
- [Docker Deployment](deployment/DOCKER_DEPLOYMENT.md)

## Project
- [Changelog](project/changelog.md) - Version history
- [Security Policy](project/security.md) - Security disclosure
```

**Update GitHub links:**
- Update README badge/link URLs to point to new locations
- Update `repository` URLs in `Cargo.toml` if they reference docs

**Impact:** Medium - requires updating many documentation links

---

#### 1.3: Organize Configuration Files

**Current scattered config:**
```
/.config/nextest.toml          # cargo-nextest config
/config/                       # Empty except .DS_Store
/codebuddy.toml               # Application config
/codebuddy.example.toml       # Example config
/.cargo/config.toml           # Cargo config
/.jscpdrc.json               # jscpd config
```

**Proposed structure:**
```bash
# Rust ecosystem tools configs
.config/
├── nextest.toml              # cargo-nextest
├── cargo-deny.toml           # NEW: cargo-deny (see Proposal 08)
├── rustfmt.toml              # OPTIONAL: Custom formatting rules
└── clippy.toml               # OPTIONAL: Custom linter rules

# Application configs
config/
└── codebuddy.example.toml    # MOVE from root

# Keep at root (standard locations)
codebuddy.toml               # Keep (runtime config)
.cargo/config.toml           # Keep (Cargo convention)
```

**Actions:**
```bash
mv codebuddy.example.toml config/
rm -rf config/.DS_Store  # Already done
```

**Update references:**
- Documentation that mentions `codebuddy.example.toml`
- Installation scripts that reference example config

**Impact:** Low risk

---

#### 1.4: Move Scripts and Deployment Files

```bash
# Move installation script
mv install.sh scripts/

# Move deployment config
mv vm.yaml deployment/
```

**Update references:**
- README installation instructions
- CI/CD workflows that reference `install.sh`
- Deployment documentation

**Impact:** Low risk

---

### Phase 2: Proposals Organization

```bash
mkdir -p proposals/{active,completed,archived}

# Create index
cat > proposals/README.md <<'EOF'
# Proposals

## Active Proposals
Proposals currently under review or implementation.

## Completed Proposals
Implemented proposals kept for reference.

## Archived Proposals
Rejected or obsolete proposals.

## How to Propose
1. Copy `00_template.proposal.md` to `active/`
2. Number sequentially (e.g., `07_feature_name.proposal.md`)
3. Fill out all sections
4. Submit for review
EOF

# Review and categorize existing proposals
# (Manual step - review each proposal's status)
```

**Manual review needed for:**
- `00_refactor.proposal.md` - Check if completed
- `01c2_actionable_suggestions_integration.proposal.md` - Check status
- `03_language_expansion.proposal.md` - Check status
- `04_rename_to_typemill.proposal.md` - Check status
- `05_fix_search_symbols.proposal.md` - Check status

**Impact:** Low risk, organizational only

---

### Phase 3: Examples Directory

```bash
mkdir -p examples/configs examples/workflows
```

**Create examples:**
```bash
# examples/configs/minimal.toml
cat > examples/configs/minimal.toml <<'EOF'
# Minimal Codebuddy configuration for Rust-only projects
[[servers]]
extensions = ["rs"]
command = ["rust-analyzer"]
EOF

# examples/configs/full-stack.toml
cat > examples/configs/full-stack.toml <<'EOF'
# Full-stack web development configuration
[[servers]]
extensions = ["rs"]
command = ["rust-analyzer"]

[[servers]]
extensions = ["ts", "tsx", "js", "jsx"]
command = ["typescript-language-server", "--stdio"]
EOF
```

**Create README:**
```bash
cat > examples/README.md <<'EOF'
# Examples

## Configuration Examples
- `configs/minimal.toml` - Rust-only setup
- `configs/full-stack.toml` - Rust + TypeScript

## Workflow Examples
- `workflows/refactoring.json` - Common refactoring workflows

## Usage
Copy an example to `.codebuddy/config.json` and customize.
EOF
```

**Impact:** Low risk, adds value for new users

---

## Before/After Directory Tree

### Before (Root: 24 files/dirs)
```
/workspace
├── AGENTS.md
├── API_REFERENCE.md ❌
├── CHANGELOG.md ❌
├── CONTRIBUTING.md ❌
├── QUICK_REFERENCE.md ❌
├── README.md ❌
├── SECURITY.md ❌
├── Cargo.toml
├── LICENSE
├── Makefile
├── codebuddy.example.toml ❌
├── codebuddy.toml
├── install.sh ❌
├── rust-toolchain.toml
├── vm.yaml ❌
├── .gitignore
├── analysis/
├── apps/
├── config/ (empty)
├── crates/
├── docs/
├── tests/ ❌
├── proposals/
└── scripts/
```

### After (Root: 12 files/dirs)
```
/workspace
├── AGENTS.md ✅ (required by AI agents)
├── CLAUDE.md -> AGENTS.md ✅
├── GEMINI.md -> AGENTS.md ✅
├── Cargo.toml
├── LICENSE
├── Makefile
├── codebuddy.toml
├── rust-toolchain.toml
├── .gitignore
├── analysis/
├── apps/
├── config/ (organized) ✅
├── crates/
├── deployment/ (contains vm.yaml) ✅
├── docs/ (consolidated) ✅
├── examples/ (NEW) ✅
├── proposals/ (organized) ✅
├── scripts/ (contains install.sh) ✅
└── tests/ (renamed from tests) ✅
```

**Root files reduced: 24 → 12 (50% reduction)**

---

## Migration Checklist

### Pre-migration
- [ ] Commit all pending changes
- [ ] Create git branch: `git checkout -b refactor/rust-directory-structure`
- [ ] Run full test suite: `cargo nextest run --workspace`
- [ ] Verify CI passes

### Phase 1: Structure (can be done in parallel)
- [ ] Rename `tests/` → `tests/`
- [ ] Update `Cargo.toml` workspace members
- [ ] Update all crate references in `Cargo.toml` files
- [ ] Move root docs to `docs/` subdirectories
- [ ] Create `docs/README.md` hub
- [ ] Move `codebuddy.example.toml` to `config/`
- [ ] Move `install.sh` to `scripts/`
- [ ] Move `vm.yaml` to `deployment/`

### Phase 2: Update References
- [ ] Update documentation links (grep for old paths)
- [ ] Update GitHub Actions workflows
- [ ] Update README installation instructions
- [ ] Update `install.sh` if it references old paths
- [ ] Update Docker/deployment scripts

### Phase 3: Validation
- [ ] Run tests: `cargo nextest run --workspace`
- [ ] Check documentation builds (if applicable)
- [ ] Verify `cargo build --release` works
- [ ] Test installation script
- [ ] Run CI locally (if using act or similar)

### Phase 4: Proposals & Examples
- [ ] Organize proposals into subdirectories
- [ ] Create `proposals/README.md`
- [ ] Create example configs in `examples/`
- [ ] Create `examples/README.md`

### Post-migration
- [ ] Update CHANGELOG.md with migration notes
- [ ] Create PR with migration changes
- [ ] Update any external documentation/wikis
- [ ] Notify team of structural changes

---

## Risks and Mitigations

### Risk: Broken links in documentation
**Likelihood:** High
**Impact:** Medium
**Mitigation:**
- Use grep to find all references before moving
- Set up redirects if documentation is published
- Test all documentation links after migration

### Risk: CI/CD pipeline failures
**Likelihood:** Medium
**Impact:** High
**Mitigation:**
- Review all GitHub Actions workflows before merge
- Test CI locally if possible
- Have rollback plan ready

### Risk: Contributors reference old paths
**Likelihood:** Medium
**Impact:** Low
**Mitigation:**
- Update CONTRIBUTING.md with new structure
- Add migration notes to CHANGELOG
- Consider temporary symlinks for common paths (short-term)

---

## Alternatives Considered

### Alternative 1: Keep current structure
**Pros:** No migration work
**Cons:** Continues to violate Rust conventions, harder for new contributors

### Alternative 2: Minimal changes only (just rename tests)
**Pros:** Less work, lower risk
**Cons:** Doesn't address documentation clutter

### Alternative 3: More aggressive consolidation (merge language crates)
**Pros:** Fewer crates
**Cons:** Slower builds, less modularity (see analysis in proposal)

**Chosen:** Full reorganization (proposed design) balances convention compliance with practical benefits.

---

## Success Criteria

- [ ] Root directory has ≤12 files/directories
- [ ] `tests/` directory follows Rust convention
- [ ] All documentation accessible from `docs/README.md`
- [ ] All tests pass after migration
- [ ] CI/CD pipeline works without modification (or updated successfully)
- [ ] No broken documentation links

---

## References

- [Rust API Guidelines - Project Structure](https://rust-lang.github.io/api-guidelines/naming.html)
- [cargo book - Workspace Structure](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [rust-analyzer project structure](https://github.com/rust-lang/rust-analyzer) (reference implementation)

---

## Timeline

**Estimated effort:** 4-6 hours

- Phase 1 (Structure): 2 hours
- Phase 2 (References): 1-2 hours
- Phase 3 (Validation): 1 hour
- Phase 4 (Proposals/Examples): 1 hour

**Recommended:** Execute over 2 sessions to allow for testing between phases.

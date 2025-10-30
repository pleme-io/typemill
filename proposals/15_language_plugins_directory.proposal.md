# Proposal 15: Reorganize Language Plugins into `languages/` Directory

## Problem

Language plugin crates are currently mixed with infrastructure crates in `crates/`, making the workspace structure harder to navigate:

**Current structure:**
```
crates/
├── mill-lang-common        # Language plugin infrastructure
├── mill-lang-rust          # Language plugin
├── mill-lang-typescript    # Language plugin
├── mill-lang-python        # Language plugin
├── mill-lang-markdown      # Language plugin
├── mill-lang-toml          # Config language plugin
├── mill-lang-yaml          # Config language plugin
├── mill-lang-gitignore     # Config language plugin
├── mill-lang-cpp           # Language plugin (inactive)
├── mill-core               # Infrastructure crate
├── mill-handlers           # Infrastructure crate
├── mill-ast                # Infrastructure crate
└── ... (20+ other infrastructure crates)
```
**Issues:**
- Language plugins are scattered among 20+ infrastructure crates
- Hard to quickly identify which crates are language plugins vs core infrastructure
- Unclear project organization for new contributors
- Makes it harder to document "how to add a new language"

## Solution

Create a dedicated `languages/` directory at the workspace root to house all language plugin crates:

**Proposed structure:**
```
languages/                  # NEW: All language plugins
├── mill-lang-common        # Language plugin infrastructure
├── mill-lang-rust
├── mill-lang-typescript
├── mill-lang-python
├── mill-lang-markdown
├── mill-lang-toml
├── mill-lang-yaml
├── mill-lang-gitignore
└── mill-lang-cpp           # (inactive)

crates/                     # Infrastructure only
├── mill-core
├── mill-handlers
├── mill-ast
├── mill-server
└── ... (infrastructure crates)
```
## Implementation Strategy

### Use `mill rename` Batch Mode (Atomic Operation)

TypeMill's batch rename feature can handle this entire refactor in **one atomic operation**:

```bash
# Step 1: Preview the refactor (dry run - default behavior)
mill tool rename '{
  "targets": [
    {"kind": "directory", "path": "crates/mill-lang-common", "newName": "languages/mill-lang-common"},
    {"kind": "directory", "path": "crates/mill-lang-rust", "newName": "languages/mill-lang-rust"},
    {"kind": "directory", "path": "crates/mill-lang-typescript", "newName": "languages/mill-lang-typescript"},
    {"kind": "directory", "path": "crates/mill-lang-python", "newName": "languages/mill-lang-python"},
    {"kind": "directory", "path": "../languages/mill-lang-markdown", "newName": "languages/mill-lang-markdown"},
    {"kind": "directory", "path": "../languages/mill-lang-toml", "newName": "languages/mill-lang-toml"},
    {"kind": "directory", "path": "../languages/mill-lang-yaml", "newName": "languages/mill-lang-yaml"},
    {"kind": "directory", "path": "crates/mill-lang-gitignore", "newName": "languages/mill-lang-gitignore"},
    {"kind": "directory", "path": "../languages/mill-lang-cpp", "newName": "languages/mill-lang-cpp"}
  ]
}'

# Step 2: Review the output - shows all changes that would be made

# Step 3: Execute the refactor (explicit opt-in)
mill tool rename '{
  "targets": [
    {"kind": "directory", "path": "crates/mill-lang-common", "newName": "languages/mill-lang-common"},
    {"kind": "directory", "path": "crates/mill-lang-rust", "newName": "languages/mill-lang-rust"},
    {"kind": "directory", "path": "crates/mill-lang-typescript", "newName": "languages/mill-lang-typescript"},
    {"kind": "directory", "path": "crates/mill-lang-python", "newName": "languages/mill-lang-python"},
    {"kind": "directory", "path": "../languages/mill-lang-markdown", "newName": "languages/mill-lang-markdown"},
    {"kind": "directory", "path": "../languages/mill-lang-toml", "newName": "languages/mill-lang-toml"},
    {"kind": "directory", "path": "../languages/mill-lang-yaml", "newName": "languages/mill-lang-yaml"},
    {"kind": "directory", "path": "crates/mill-lang-gitignore", "newName": "languages/mill-lang-gitignore"},
    {"kind": "directory", "path": "../languages/mill-lang-cpp", "newName": "languages/mill-lang-cpp"}
  ],
  "options": {
    "dryRun": false
  }
}'

# Step 4: Verify everything compiles
cargo check --workspace
```
### What Gets Updated Automatically

The batch rename will handle:

1. ✅ Creates `languages/` directory
2. ✅ Moves all 9 language plugin crates
3. ✅ Updates `Cargo.toml` workspace members array
4. ✅ Updates `workspace.dependencies` paths in root `Cargo.toml`
5. ✅ Updates path dependencies in dependent crates
6. ✅ Updates any file references (imports, string literals)
7. ✅ Updates documentation links in `.md` files
8. ✅ Updates config file paths in `.toml`/`.yaml` files
9. ✅ Preserves git history (uses filesystem moves)

### Scope Configuration

Using `scope: "standard"` (default) which updates:
- Code files (imports, module declarations, string literal paths)
- Documentation files (`.md` links)
- Configuration files (`.toml`, `.yaml`)
- Cargo.toml (workspace members, dependencies)

**Not using** `scope: "everything"` to avoid:
- Updating "crates" in prose text (comments and markdown body text)
- Unnecessary churn in explanatory comments

## Checklists

### Phase 1: Preview and Validate
- [ ] Run batch rename with dry run (default)
- [ ] Review all proposed changes in output
- [ ] Verify workspace members updates look correct
- [ ] Verify dependent crate paths will be updated
- [ ] Check for any unexpected file modifications

### Phase 2: Execute Refactor
- [ ] Execute batch rename with `dryRun: false`
- [ ] Verify `languages/` directory was created
- [ ] Verify all 9 crates moved successfully
- [ ] Run `cargo check --workspace` to verify compilation
- [ ] Run `cargo nextest run --workspace` to verify tests pass

### Phase 3: Documentation Updates
- [ ] Update `CLAUDE.md` references to language plugin locations
- [ ] Update `docs/DEVELOPMENT.md` with new directory structure
- [ ] Update `contributing.md` plugin scaffolding examples
- [ ] Update any `xtask` commands that reference language plugin paths
- [ ] Update `.debug/language-plugin-migration/` docs if needed

### Phase 4: Validation
- [ ] Verify all tests pass: `cargo nextest run --workspace`
- [ ] Verify clippy passes: `cargo clippy --workspace`
- [ ] Verify formatting: `cargo fmt --check`
- [ ] Test `cargo xtask new-lang` scaffolding command
- [ ] Test language detection in `mill setup`
- [ ] Verify CI/CD pipeline still works

## Success Criteria

- All language plugin crates moved to `languages/` directory
- Workspace compiles successfully with `cargo check --workspace`
- All tests pass with `cargo nextest run --workspace`
- Zero clippy warnings
- Documentation updated to reflect new structure
- Git history preserved (directory moves, not delete+create)
- Total execution time under 2 minutes

## Benefits

### Organization
- **Clear Separation**: Language plugins separated from infrastructure
- **Easy Discovery**: New contributors can find all language plugins in one place
- **Logical Grouping**: Related functionality grouped together

### Maintainability
- **Easier Navigation**: Reduced cognitive load when browsing workspace
- **Better Documentation**: Can reference "see `languages/` directory" instead of listing crates
- **Plugin Development**: Clear location for new language plugins

### Scalability
- **Room to Grow**: Future language additions go in obvious location
- **Consistent Pattern**: Follows common Rust workspace organization patterns
- **Clean Namespace**: Infrastructure vs language concerns cleanly separated

## Risks and Mitigations

### Risk: Batch Rename Fails Mid-Operation
**Mitigation**: Automatic rollback on any failure (atomic operation)

### Risk: Unexpected File References
**Mitigation**: Preview mode shows all changes before execution

### Risk: IDE/Editor Confusion
**Mitigation**: Workspace members updated automatically; IDEs will reload

### Risk: CI/CD Pipeline Breaks
**Mitigation**: Cargo.toml updates handle path changes; test thoroughly before merge

### Risk: Git History Loss
**Mitigation**: Batch rename uses filesystem moves, preserves git history

## Estimated Time

- Preview: ~5 seconds
- Execution: ~10-15 seconds
- Verification (cargo check): ~20 seconds
- Tests: ~40 seconds
- Documentation updates: ~10 minutes
- **Total: ~15 minutes**

## Future Work

After this refactor, consider:
- Moving language plugin docs to `languages/README.md`
- Adding per-language READMEs in each plugin directory
- Creating `languages/template/` for new plugin scaffolding
- Updating CI/CD to test language plugins as a group

## Related Work

- Proposal 01: Plugin refactoring (data structure consolidation)
- Language Registry System: `languages.toml` for centralized feature management
- Python/Swift restoration: Used migration guides to restore full plugins

## Notes

- **Config languages included**: TOML, YAML, GitIgnore plugins also moved (they're still language plugins, just simpler)
- **mill-lang-common included**: Shared infrastructure for language plugins belongs with them
- **Inactive C++ plugin**: Moved for consistency, even though not fully active yet
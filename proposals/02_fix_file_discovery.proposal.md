# Fix Markdown and YAML File Discovery in Rename Operations

## Problem

File discovery mechanism fails to find markdown (`.md`) and YAML (`.yaml`, `.yml`) files during rename operations, causing incomplete refactoring plans with broken documentation links and stale configuration references.

**Evidence:**
- Rename plan shows 22 files, but 29 files contain references (76% coverage)
- Log shows: `Found files for extension, extension: "markdown", files_found: 0`
- Regression test `test_file_discovery_in_non_standard_locations` fails
- Missing files: docs/, proposals/, config files in root

**Impact:** Every rename operation requires manual search-and-replace for documentation and config files.

## Solution

Fix file discovery logic in reference updater to include markdown and YAML files when scanning workspace for references.

### Root Cause

File scanner in `mill-services/src/services/reference_updater/mod.rs` either:
1. Filters by plugin-handled extensions only (excludes markdown/YAML plugins)
2. Has glob patterns excluding docs/proposals directories
3. Missing plugin registration for markdown/YAML file types

## Checklists

### Investigation Phase
- [ ] Check `find_project_files()` implementation in `crates/mill-services/src/services/reference_updater/mod.rs`
- [ ] Verify MarkdownPlugin registered in plugin system
- [ ] Verify YamlPlugin registered in plugin system
- [ ] Check for hardcoded extension allowlist
- [ ] Check for directory exclusion patterns (docs/, proposals/)
- [ ] Verify glob patterns don't filter non-code files

### Fix Implementation
- [ ] Add markdown extensions to file scanner (`.md`, `.markdown`)
- [ ] Add YAML extensions to file scanner (`.yaml`, `.yml`)
- [ ] Ensure plugins registered for file discovery operations
- [ ] Update `find_project_files()` to include all scope-relevant extensions
- [ ] Remove any filters excluding documentation directories

### Testing
- [ ] Run `cargo nextest run test_file_discovery_in_non_standard_locations` (should pass)
- [ ] Verify logs show `Found files for extension, extension: "markdown", files_found: >0`
- [ ] Test real rename: `mill tool rename` on test-support crate shows all 29 files
- [ ] Verify markdown references updated correctly
- [ ] Verify YAML config files updated correctly
- [ ] Run full test suite: `cargo nextest run --workspace` (zero failures)

### Validation
- [ ] Coverage check: `rg "pattern" --files-with-matches | wc -l` matches plan file count
- [ ] Verify scope="standard" includes docs and configs
- [ ] Verify scope="code" excludes docs and configs
- [ ] Test with files in non-standard locations (docs/, proposals/, root)

## Success Criteria

- [ ] `test_file_discovery_in_non_standard_locations` passes
- [ ] Real rename operations include all files with references
- [ ] Log output shows non-zero counts for markdown/YAML extensions
- [ ] Manual verification: all docs/ and proposals/ files appear in rename plans
- [ ] Zero regressions in existing rename functionality

## Benefits

- **Complete refactoring plans** - No manual cleanup required after renames
- **Accurate documentation** - Links and references stay up-to-date
- **Correct configurations** - YAML/TOML files reflect actual crate names
- **Developer confidence** - Trust rename operations to find all references
- **Automated testing** - Regression test prevents future breaks

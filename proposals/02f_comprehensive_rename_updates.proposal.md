# Proposal 13: Comprehensive Rename/Move Updates

**Status:** Draft
**Created:** 2025-10-16
**Dependencies:** None (standalone)

## Problem

CodeBuddy's `rename.plan` tool currently updates only ~9% of references when renaming directories/files:
- ✅ Updates Rust imports (`use` statements)
- ✅ Updates `Cargo.toml` workspace members and dependencies
- ❌ Misses string literals in code (`"tests/path"`)
- ❌ Misses documentation files (.md files with 98+ references)
- ❌ Misses config files (`.cargo/config.toml`, CI configs)
- ❌ Misses examples directory
- ❌ Misses `.gitignore` patterns

Example: Renaming `tests/` → `tests/` affects 113 references across 15 files, but only updates 5 files.

This forces manual find/replace for critical infrastructure files and extensive documentation, reducing confidence in the refactoring tool.

## Solution

Extend rename/move detection to cover all functional references while avoiding false positives in prose.

### 1. Code String Literals (P0 - Critical)

Detect and update path strings in Rust code:
```rust
// Should be updated:
let path = "tests/fixtures";
std::fs::read("tests/test.rs");
Command::new("cargo").arg("--manifest-path=tests/Cargo.toml");
```

**Detection logic:**
- Parse AST for string literals
- Match literals containing the old path
- Update if contains `/`, file extensions, or workspace-relative paths

### 2. Documentation Files (P1 - High Value)

Update markdown files with smart path detection:
```markdown
✅ Update these (clear path references):
- Code blocks: `tests/src/main.rs`
- Directory trees: `├── tests/`
- File paths: `/workspace/tests/`
- Links: `[guide](tests/TESTING_GUIDE.md)`

❌ Skip these (prose):
- "We use tests as a pattern"
- "Other projects call them tests"
```

**Detection logic:**
- Parse markdown structure (code blocks, links, trees)
- Update path-like strings (contains `/` or file extensions)
- Skip plain text paragraphs unless clearly a path

### 3. Config Files (P1 - Infrastructure)

Support common config file formats:
- `.cargo/config.toml` - Rust build config
- `.github/workflows/*.yml` - CI/CD pipelines
- `Makefile` - Build automation
- `.gitignore` - VCS patterns
- `rust-toolchain.toml`, `clippy.toml`, etc.

### 4. Examples Directory (P0 - Critical)

Treat `examples/` as first-class code - same rules as `src/`.

### 5. Code Comments (P2 - Optional)

Add opt-in support for updating comments:
```rust
// TODO: Move tests to new location <- Update
// tests pattern is standard <- Skip (prose)
```

## Checklists

- [x] Add string literal detection to Rust AST parser
- [x] Update string literals in code files during rename operations
- [x] Include `examples/` directory in code scanning
- [x] Add markdown parser for structured path detection
- [x] Implement path vs prose heuristics (contains `/`, extensions)
- [x] Add config file parsers (TOML, YAML, Makefile)
- [ ] Update `.gitignore` pattern matching
- [x] Categorize changes by type (imports, strings, docs, configs)
- [x] Show summary with counts per category
- [x] Add human-readable change descriptions
- [ ] Highlight potential false positives for review
- [x] Add `update_code` option (imports + string literals)
- [x] Add `update_examples` option
- [x] Add `update_docs` option (markdown files)
- [x] Add `update_configs` option (TOML, YAML, Makefile)
- [x] Add `update_comments` option (opt-in)
- [ ] Add `update_gitignore` option
- [x] Add `exclude` patterns for custom filtering
- [x] Add `scope` presets (code-only, all, custom)
- [x] Test string literal updates in Rust code
- [x] Test markdown path detection accuracy
- [x] Test false positive avoidance (prose vs paths)
- [x] Test config file updates (TOML, YAML)
- [ ] Test `.gitignore` pattern updates
- [x] Test examples directory updates
- [x] Verify comprehensive coverage (integration-tests → tests scenario)
- [ ] Document new configuration options in API reference
- [ ] Add examples for different scope presets
- [x] Document path detection heuristics
- [ ] Add troubleshooting guide for false positives/negatives

## Success Criteria

**Measured by test case: Rename `tests/` → `tests/`**

Before: 5/15 files updated (33%)
After: 14+/15 files updated (93%+)

**Breakdown:**
- ✅ Rust imports: 2 files
- ✅ Cargo.toml: 3 files
- ✅ String literals: 3 files (NEW)
- ✅ Markdown docs: 8 files (NEW)
- ✅ Config files: 2 files (NEW)
- ⚠️ Comments: Opt-in only

**Quality metrics:**
- Zero false positives in default mode (no prose corruption)
- Dry-run shows categorized preview before applying
- All functional references updated (builds succeed)
- Documentation stays synchronized with code

## Benefits

**For developers:**
- Rename operations are truly comprehensive (9% → 93%+ coverage)
- Confidence to refactor without manual cleanup
- Documentation stays in sync automatically
- CI configs update with code changes

**For CodeBuddy:**
- Positions tool as production-ready for large refactorings
- Differentiates from basic find/replace tools
- Builds trust through transparency (categorized dry-run)
- Handles complexity developers expect from serious tooling

**Impact:**
- Save hours on large renames (113 manual updates → 5 manual updates)
- Prevent subtle bugs from missed string literals
- Reduce stale documentation confusion
- Enable fearless refactoring at scale

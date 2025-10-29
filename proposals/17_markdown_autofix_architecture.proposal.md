# Markdown Auto-Fix Architecture Refactor

## Problem

Current markdown auto-fix implementation (cafb6c7b) has hardcoded fix methods in a monolithic `MarkdownFixer` struct with limited extensibility:
- No preview/diff support (only counts)
- Global configuration instead of per-fixer options
- Fixes embedded in `quality.rs` handler instead of modular plugins
- No metadata linking findings to available fixes
- Limited to workspace scope only

## Solution

Trait-based fixer architecture with plugin registry:
- `MarkdownFixer` trait for extensible fix implementations
- `FixerRegistry` with lazy-static lookup by fix ID
- Per-fixer configuration via `fix_options` map
- Preview mode with unified diff output
- Metadata in findings to indicate fixability
- Support file/directory/workspace scopes

**Architecture:**
```rust
trait MarkdownFixer {
    fn id(&self) -> &'static str;
    fn apply(&self, ctx: &MarkdownContext, config: Value) -> FixOutcome;
}

struct FixOutcome {
    edits: Vec<TextEdit>,
    preview: Option<String>,  // Unified diff format (manual generation)
    warnings: Vec<String>,
}

struct MarkdownContext {
    content: String,
    file_path: PathBuf,
    content_hash: String,  // Store on read, compare before write
}
```

**Phased delivery:**
- **Core set (Phase 1)**: Registry + 4 migrated fixers + auto_toc = 5 fixers
- **Additional fixers (Follow-up)**: bare_url, table_columns, list_markers, blank_lines, emphasis = 6 more fixers

## Checklists

### Core Architecture
- [ ] Create `markdown_fixers/mod.rs` with `MarkdownFixer` trait
- [ ] Implement `FixOutcome` struct with edits, preview (unified diff), warnings
- [ ] Implement `FixerRegistry` with lazy-static Vec lookup
- [ ] Add `TextEdit` struct (range, old_text, new_text)
- [ ] Add `MarkdownContext` struct (content, file_path, content_hash via SHA-256)
- [ ] Implement manual unified diff generation (no external library)

### API Changes
- [ ] Extend `QualityOptions` with `fix_options: HashMap<String, Value>`
- [ ] Add preview generation (unified diff format via manual generation)
- [ ] Add optimistic file locking (store SHA-256 hash on read, compare before write, abort if mismatch)
- [ ] Update response schema with `fixes[]` array for previews
- [ ] Add `fix_id` metadata to ALL findings in detection functions (bare_url, table_columns, trailing_whitespace, etc.)

### Migrate Existing Fixes
- [ ] Implement `TrailingWhitespaceFixer`
- [ ] Implement `MissingCodeLangFixer`
- [ ] Implement `MalformedHeadingFixer`
- [ ] Implement `ReversedLinkFixer`
- [ ] Remove old hardcoded methods from quality.rs

### New Fixers (Phase 1: auto_toc only; others optional/follow-up)
- [ ] Implement `AutoTocFixer` with options (marker, max_depth, include_h1, exclude_patterns)
- [ ] Add GitHub anchor slug algorithm with duplicate handling
- [ ] Detect `toc_out_of_sync` finding in `detect_markdown_structure` with fix_id metadata
- [ ] **Optional:** Implement `BareUrlFixer` with context-aware skipping (code blocks, inline code), add fix_id to bare_url findings
- [ ] **Optional:** Implement `TableColumnFixer` with safe padding mode, add fix_id to table_column_inconsistency findings

### Additional Safe Fixes (Optional - Follow-up milestone)
- [ ] **Optional:** Implement `ListMarkerFixer` (normalize -, *, + to consistent style), add fix_id to inconsistent_list_markers findings
- [ ] **Optional:** Implement `BlankLineFixer` (add spacing around blocks, compress multiple blanks), add fix_id to missing_blank_lines findings
- [ ] **Optional:** Implement `EmphasisFixer` (normalize * vs _ for italic/bold), add fix_id to inconsistent_emphasis findings

### Handler Integration
- [ ] Update QualityHandler markdown branches to enumerate files (for directory/workspace scopes), run fixers from registry, call preview/write
- [ ] Add file/directory scope support for fixes (currently only workspace)
- [ ] Add apply flag handling (false=preview with diffs, true=write with conflict check)
- [ ] Add fix metadata to AnalysisResult.summary (fix_actions.applied, fix_actions.preview_only)
- [ ] Integrate with `file_service.write_file`, compare content_hash before write, abort on mismatch

### Testing
- [ ] Unit tests for each fixer in `markdown_fixers_test.rs` (test BOTH preview and apply modes)
- [ ] E2E tests for preview mode (apply=false returns diffs, no file writes)
- [ ] E2E tests for execution mode (apply=true writes files, verify content)
- [ ] Test conflict detection (file changed during analysis, write aborts)
- [ ] Test auto-TOC generation with various heading patterns (H1-H6, special chars, duplicates)
- [ ] Test bare URL detection skips code blocks and inline code
- [ ] Test table column padding preserves data (no truncation in safe mode)

### Documentation
- [ ] Update `docs/tools/analysis.md` with new options schema (fix_options map, apply flag)
- [ ] Add examples to `docs/user-guide/cheatsheet.md` showing both dry-run and apply workflows
- [ ] Create `docs/features/markdown-auto-fixes.md` guide with all fixer options
- [ ] Document per-fixer configuration options (auto_toc: marker/max_depth, bare_url: skip_code_blocks, etc.)
- [ ] Add CLI examples showing dry-run (`--apply false`) and apply (`--apply true`) with various fixers

## Success Criteria

**Core delivery (Phase 1 - 5 fixers):**
- Trait-based fixer registry implemented and working
- 4 migrated fixers (trailing_whitespace, missing_code_lang, malformed_heading, reversed_link) + auto_toc fixer operational
- Preview mode returns unified diffs without writing files
- Apply mode writes files with SHA-256 conflict detection
- Findings include `fix_id` metadata for all fixable issues
- 100% test coverage for each fixer (preview + apply modes)
- Can run single fix on single file: `mill tool analyze.quality --kind markdown_structure --scope file:README.md --fix auto_toc --apply true`
- Documentation covers core 5 fixers with CLI examples (dry-run and apply)

**Optional follow-up (6 additional fixers):**
- bare_url, table_columns, list_markers, blank_lines, emphasis fixers implemented
- All 11 fixers registered and tested

## Benefits

- **Extensibility**: Add new fixers without modifying core code
- **Safety**: Preview mode validates changes before applying
- **Granularity**: Per-fixer configuration instead of global flags
- **Testability**: Each fixer is independently testable
- **Metadata**: Findings indicate which fixes are available
- **Conflict detection**: Prevents overwriting concurrent edits
- **Scope flexibility**: Works with file/directory/workspace
- **Auto-TOC**: Sync 330+ markdown files with correct table of contents
- **Bulk fixes**: Apply consistent formatting across entire codebase

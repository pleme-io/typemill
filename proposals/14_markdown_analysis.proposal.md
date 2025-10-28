# Proposal 14: Markdown Structure & Formatting Analysis

## Problem

Markdown plugin supports link tracking but lacks quality analysis within current architecture constraints:
- No structure validation (heading hierarchy, duplicates, empty sections)
- No formatting checks (missing alt text, code language tags, table consistency)
- Current analysis engine only supports sync detection functions (cannot read other files)
- Markdown import parser drops anchor fragments (cannot validate `#heading` links)
- `analyze.documentation` has no "completeness" kind yet

Link validation requires architectural changes (async detection or prefetch) - deferred to separate proposal.

## Solution

Add markdown-specific sync detection functions for structure and formatting analysis within current architecture.

### Architecture Constraints
- Detection functions are sync callbacks receiving pre-parsed data (content, symbols, file_path)
- Cannot read other files (async file_service.exists())
- Cannot parse other files (async plugin.parse())
- Registry only exposes `get_plugin(extension)`, not `get_plugin_for_file(path)`

### What Can Be Analyzed (Sync)
- Current file heading structure (using pre-parsed symbols)
- Current file formatting (regex on content string)
- Code blocks without language tags
- Images without alt text
- Table column consistency
- Duplicate headings
- Multiple top-level headings (H1)
- Malformed headings (no space after `#`)
- Bare URLs (unformatted links)
- Empty or malformed links
- Reversed link syntax (`(text)[url]`)
- Unclosed code fences

### What Cannot Be Analyzed (Needs Async)
- Broken file links (requires file_service.exists())
- Anchor validation in other files (requires read + parse target)
- Remote URL validation (requires network)

### New Analysis Kinds

**`analyze.quality`:**
- `kind: "markdown_structure"` - Heading hierarchy, duplicate headings, empty sections, multiple H1s, malformed headings
- `kind: "markdown_formatting"` - Code block language tags, image alt text, table consistency, bare URLs, malformed links, unclosed code fences

## Checklists

### Implementation
- [ ] Add `detect_markdown_structure()` function in `crates/mill-handlers/src/handlers/tools/analysis/quality.rs`
- [ ] Implement heading hierarchy validation (no skipped levels: # → ### without ##)
- [ ] Use pre-parsed symbols (headers already parsed by engine)
- [ ] Implement duplicate heading detection (same title at same level)
- [ ] Implement empty section detection (headers with no content before next header)
- [ ] Add `detect_markdown_formatting()` function in quality.rs
- [ ] Implement code block language tag checking (```rust vs bare ```)
- [ ] Implement image alt text checking (![](path) vs ![alt](path))
- [ ] Implement table column consistency checking (same number of | per row)
- [ ] Implement trailing whitespace detection
- [ ] Add `markdown_structure` and `markdown_formatting` to quality handler kind validation
- [ ] Add both kinds to quality handler dispatcher

### Testing & Documentation
- [ ] Test structure validation with heading hierarchy violations, duplicates, empty sections
- [ ] Test formatting validation with missing language tags, missing alt text, malformed tables
- [ ] Add integration tests in `tests/e2e/src/test_analysis.rs`
- [ ] Document `markdown_structure` kind in `docs/tools/analysis.md`
- [ ] Document `markdown_formatting` kind in `docs/tools/analysis.md`
- [ ] Add markdown analysis examples to `docs/user-guide/cheatsheet.md`
- [ ] Update AGENTS.md with markdown analysis capabilities

## Success Criteria

- `analyze.quality` supports `markdown_structure` and `markdown_formatting` kinds for markdown files
- Structure validation catches heading hierarchy violations (# → ### without ##)
- Structure validation detects duplicate headings at same level
- Formatting validation identifies missing code block language tags
- Formatting validation detects missing image alt text
- Formatting validation catches table column count inconsistencies
- Detection functions work within sync architecture (no async calls)
- Documentation updated with usage examples
- Integration tests pass for both new kinds

## Benefits

- **Structure Quality**: Maintain proper document hierarchy and organization
- **Formatting Consistency**: Enforce markdown standards across projects
- **Accessibility**: Detect missing alt text for images
- **Code Quality**: Ensure code blocks have language tags for proper highlighting
- **Maintainability**: Sync implementation works within existing architecture
- **Foundation**: Establishes pattern for language-specific analysis extensions

## Future Work (Requires Architectural Changes)

Link validation deferred to separate proposal requiring:
- Async detection function support OR prefetch mechanism in engine
- Enhanced import parser that preserves anchor fragments
- Heading ID generation (GitHub-style slugs) in markdown plugin
- Network access strategy for remote URL validation

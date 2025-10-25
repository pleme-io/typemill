# Proposal 01: Workspace Find & Replace Tool

## Problem

Manual text replacement across the workspace requires external tools (sed, ripgrep) with limitations:

- No atomic multi-file operations (each sed command is separate)
- No preview of changes before applying
- Platform-dependent regex syntax (GNU sed vs BSD sed)
- Manual case preservation (requires multiple sed passes)
- No rollback on partial failures
- Limited integration with existing workspace tools

Current workarounds require shell scripting and careful manual verification.

## Solution

Implement `workspace.find_replace` as a public MCP tool with:

**Core Capabilities:**
- Literal and regex pattern matching
- Workspace-wide scope with include/exclude patterns
- Smart case preservation
- Atomic multi-file edits via WorkspaceEdit
- Dry-run preview with exact positions
- Integration with `workspace.apply_edit`

**API Design:**
```json
{
  "name": "workspace.find_replace",
  "arguments": {
    "pattern": "TYPEMILL_([A-Z_]+)",
    "replacement": "TYPEMILL_$1",
    "mode": "literal|regex",
    "preserveCase": false,
    "scope": {
      "includePatterns": ["**/*.rs", "**/*.toml", "**/*.md"],
      "excludePatterns": ["**/target/**"]
    },
    "dryRun": true
  }
}
```

## Checklists

### Implementation

- [ ] Create `FindReplaceHandler` in `crates/mill-handlers/src/handlers/workspace/`
- [ ] Implement scope parser with glob pattern support
- [ ] Implement literal string replacement mode
- [ ] Implement regex replacement mode with capture groups
- [ ] Implement case preservation logic (detect and apply case styles)
- [ ] Generate WorkspaceEdit with all text edits
- [ ] Register tool as `workspace.find_replace` in handler registry
- [ ] Add tool schema with parameter validation
- [ ] Default `dry_run: true` to prevent accidental mass-replacements
- [ ] Require explicit `dry_run: false` for execution (safety-first design)

### Testing

- [ ] Test literal replacement across multiple files
- [ ] Test regex replacement with capture groups ($1, $2)
- [ ] Test case preservation (lower, UPPER, Title, PascalCase)
- [ ] Test scope filtering (include/exclude patterns)
- [ ] Test dry-run mode (no file modifications)
- [ ] Test dry-run defaults to true when not specified
- [ ] Test workspace.apply_edit integration
- [ ] Test UTF-8 handling with non-ASCII characters
- [ ] Test error handling (invalid regex, file access errors)

### Documentation

- [ ] Add to `docs/api_reference.md` with examples
- [ ] Update `docs/tools_catalog.md` with tool entry
- [ ] Add usage examples to `CLAUDE.md` / `AGENTS.md`
- [ ] Document regex syntax and capture group support
- [ ] Document case preservation behavior

## Success Criteria

- Tool listed in `tools/list` MCP response
- Literal mode successfully replaces text across workspace
- Regex mode supports capture groups ($1, $2, etc.)
- Case preservation correctly handles: `lower`, `UPPER`, `Title`, `PascalCase`
- Scope filtering excludes `target/` and `node_modules/` by default
- Dry-run defaults to `true` when not specified (safety-first)
- Dry-run returns WorkspaceEdit without modifying files
- Integration with `workspace.apply_edit` applies changes atomically
- All tests pass with coverage >90%
- Documentation complete with working examples

## Benefits

**User Experience:**
- No shell scripting required for text replacement
- Preview all changes before applying (dry-run)
- Atomic operations with rollback on failure
- Cross-platform consistency (no sed dialect issues)

**Developer Experience:**
- Eliminates sed scripting for refactoring tasks
- Safer operations (preview + rollback)
- Better error messages than sed silent failures
- Integrates with existing TypeMill workflow

**Use Cases:**
- Configuration path updates (`.typemill` → `.typemill`)
- Environment variable prefix changes (`TYPEMILL_*` → `TYPEMILL_*`)
- CLI command updates in documentation
- Project-wide naming conventions
- Dependency version updates in manifests

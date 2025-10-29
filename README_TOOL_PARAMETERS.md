# Mill Tool Parameters - Complete Documentation

This collection contains comprehensive parameter documentation for Mill tools extracted from source code analysis.

## Quick Start

Choose your document based on your needs:

| Document | Best For | Size |
|----------|----------|------|
| **TOOL_PARAMETERS_SUMMARY.txt** | Quick lookups, checklists, parameter validation | 8 KB |
| **TOOL_PARAMETERS_ANALYSIS.md** | Deep dives, implementation details, code locations | 16 KB |
| **TOOL_PARAMETERS_INDEX.md** | Navigation, overview, documentation improvements | 7 KB |

## What's Documented

### Core Tools

1. **analyze.structure** - Code structure analysis
   - Analyzes symbols, hierarchy, interfaces, inheritance, and modules
   - Required parameter: `kind` (one of 5 values)
   - Handler: `structure.rs` (lines 1170-1251)

2. **analyze.dependencies** - Dependency analysis
   - Analyzes imports, graphs, circular dependencies, coupling, cohesion, depth
   - Required parameter: `kind` (one of 6 values)
   - Handler: `dependencies.rs` (lines 1167-1356)

3. **rename** - Symbol/file/directory renaming
   - Single target mode (existing API) or batch mode (new API)
   - Required: `target`/`targets` and `new_name`
   - Optional: `dryRun` (safe default: true), `scope`, `consolidate`
   - Handler: `rename_handler/mod.rs` (lines 28-277)

4. **Circular Dependencies** - Cycle detection
   - Access via `analyze.dependencies` with `kind="circular"`
   - Two modes: File-level (MVP) and workspace-level (with feature flag)
   - Handler: `dependencies.rs` (lines 316-393, 1213-1311)

## Key Information

### Parameter Validation

All tools follow a consistent pattern:
1. Check for required parameter
2. Validate value against allowed set
3. Return clear error message if invalid
4. Dispatch to appropriate handler

### Error Messages

15+ documented error messages covering:
- Missing parameters
- Invalid parameter values
- Mode conflicts (for tools with multiple APIs)
- Naming conflicts (for batch operations)

### Documentation Status

| Tool | Status | Location |
|------|--------|----------|
| analyze.structure | Excellent | `/workspace/docs/tools/analysis.md` |
| analyze.dependencies | Good* | `/workspace/docs/tools/analysis.md` |
| rename | Excellent | `/workspace/docs/tools/refactoring.md` |
| Circular deps | Partial* | `/workspace/docs/tools/analysis.md` |

*Needs improvements documented in TOOL_PARAMETERS_ANALYSIS.md

## File Organization

```text
/workspace/
├── TOOL_PARAMETERS_INDEX.md       (Navigation guide)
├── TOOL_PARAMETERS_ANALYSIS.md    (Detailed implementation)
├── TOOL_PARAMETERS_SUMMARY.txt    (Quick reference)
├── README_TOOL_PARAMETERS.md      (This file)
│
├── crates/mill-handlers/src/handlers/
│   ├── tools/analysis/
│   │   ├── structure.rs           (analyze.structure handler)
│   │   ├── dependencies.rs        (analyze.dependencies handler)
│   │   ├── circular_dependencies.rs
│   │   └── engine.rs              (Shared analysis engine)
│   └── rename_handler/
│       └── mod.rs                 (rename handler)
│
└── docs/tools/
    ├── analysis.md                (User documentation)
    └── refactoring.md             (User documentation)
```text
## Common Tasks

### I need to know required parameters for a tool
Start with **TOOL_PARAMETERS_SUMMARY.txt** - go to the tool section

### I'm implementing a handler for a new tool
Read **TOOL_PARAMETERS_ANALYSIS.md** section "Analysis Summary" → "Parameter Validation Patterns"

### I need to understand error handling
Check **TOOL_PARAMETERS_ANALYSIS.md** - each tool has "Current Error Messages" section

### I found a documentation gap
See **TOOL_PARAMETERS_ANALYSIS.md** → "Recommendations" or **TOOL_PARAMETERS_INDEX.md** → "Documentation Improvements Needed"

### I need source code locations
All documents include absolute file paths and line numbers. Use **TOOL_PARAMETERS_SUMMARY.txt** under "ABSOLUTE FILE PATHS"

## Parameter Patterns

### Analysis Tools Pattern
```json
{
  "kind": "specific_analysis_type",
  "scope": {
    "type": "file|directory|workspace|symbol",
    "path": "...",
    "include": [],
    "exclude": []
  },
  "options": {}
}
```text
### Refactoring Tools Pattern
```json
{
  "target": {"kind": "...", "path": "..."},
  "newName": "...",
  "options": {
    "dryRun": true,
    ...
  }
}
```text
## Validation Checklist

Before calling a tool, verify:

- [ ] Required parameters present
- [ ] Parameter values valid (check kind/scope options)
- [ ] For rename: Either `target` or `targets`, not both
- [ ] For analyze: Provide either scope.path or file_path
- [ ] For batch operations: All required per-item parameters provided
- [ ] For refactoring: Consider starting with dryRun: true

## Documentation Improvements

The analysis identified these documentation gaps (prioritized):

1. **HIGH:** Circular dependencies feature flag requirement
2. **MEDIUM:** Scope parameter structure documentation
3. **MEDIUM:** Batch rename deduplication behavior
4. **LOW:** Tool author reference guide

Details: See TOOL_PARAMETERS_ANALYSIS.md → "Recommendations"

## Statistics

- Total documentation: 942 lines across 3 files
- Handler line references: 50+
- Error messages catalogued: 15+
- File paths documented: 10+
- Code examples: 15+ JSON snippets
- Coverage: 4 tools (3 direct + circular deps)

## Related Documentation

User-facing documentation:
- `/workspace/docs/tools/analysis.md` - analyze.structure, analyze.dependencies
- `/workspace/docs/tools/refactoring.md` - rename tool
- `/workspace/docs/tools/` - Full tools reference

Implementation documentation:
- `/workspace/crates/mill-handlers/src/` - Handler implementations
- `/workspace/contributing.md` - Contribution guidelines

## Questions?

1. For quick answers: Check TOOL_PARAMETERS_SUMMARY.txt
2. For implementation details: See TOOL_PARAMETERS_ANALYSIS.md with line numbers
3. For code: Follow file paths to source implementation
4. For user-facing details: Check `/workspace/docs/tools/`

## Generated

- **Date:** 2025-10-28
- **Source:** Complete codebase analysis
- **Coverage:** All public analysis and refactoring tools
- **Quality:** Includes line numbers, error messages, and recommendations

---

Start with **TOOL_PARAMETERS_SUMMARY.txt** for quick reference or **TOOL_PARAMETERS_ANALYSIS.md** for detailed implementation information.
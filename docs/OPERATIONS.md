# Operations Guide

Advanced configuration, analysis options, and operational patterns.

## Configuration

### LSP Server Configuration

Location: `.codebuddy/config.json`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `servers` | Array | Yes | List of language server configurations |
| `servers[].extensions` | String[] | Yes | File extensions (e.g., `["ts", "tsx"]`) |
| `servers[].command` | String[] | Yes | Command to spawn LSP server |
| `servers[].rootDir` | String | No | Working directory for LSP server |
| `servers[].restartInterval` | Number | No | Auto-restart interval (minutes, min: 1) |

**Auto-detect:** Run `codebuddy setup` to generate config from project files.

**Manual config:** See [examples/setup/codebuddy-config.json](../examples/setup/codebuddy-config.json)

### Analysis Configuration

Location: `.codebuddy/analysis.toml`

#### Suggestions Settings

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_confidence` | Float | 0.7 | Minimum confidence threshold (0.0-1.0) |
| `include_safety_levels` | String[] | `["safe", "requires_review"]` | Safety levels to include |
| `max_per_finding` | Integer | 3 | Max suggestions per finding |
| `generate_refactor_calls` | Boolean | true | Generate executable refactoring commands |

**Safety Levels:**
- `safe` - Can be applied automatically
- `requires_review` - Manual review recommended
- `experimental` - May have edge cases

**Presets:**
- `strict` - Only safe suggestions, high confidence (0.8+)
- `default` - Safe + requires_review, medium confidence (0.7+)
- `relaxed` - All levels, low confidence (0.5+)

## Unified Analysis API

All `analyze.*` tools follow the same pattern:

### Common Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `kind` | String | Yes | Analysis type (varies by category) |
| `scope` | Object | Yes | Target scope (file/directory/workspace) |
| `options` | Object | No | Analysis options |

### Scope Types

| Type | Required Fields | Example |
|------|----------------|---------|
| `file` | `path` | `{"type": "file", "path": "src/app.ts"}` |
| `directory` | `path`, `recursive` | `{"type": "directory", "path": "src", "recursive": true}` |
| `workspace` | - | `{"type": "workspace"}` |

### Analysis Categories

| Category | Tool | Kinds |
|----------|------|-------|
| **Quality** | `analyze.quality` | complexity, smells, maintainability, readability |
| **Dead Code** | `analyze.dead_code` | unused_imports, unused_symbols, unused_parameters, unused_variables, unused_types, unreachable |
| **Dependencies** | `analyze.dependencies` | imports, graph, circular, coupling, cohesion, depth |
| **Structure** | `analyze.structure` | symbols, hierarchy, interfaces, inheritance, modules |
| **Documentation** | `analyze.documentation` | coverage, quality, style, examples, todos |
| **Tests** | `analyze.tests` | coverage, quality, assertions, organization |

### Common Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `threshold` | Float | - | Threshold for numeric metrics |
| `include_tests` | Boolean | false | Include test files in analysis |
| `follow_imports` | Boolean | true | Follow import chains |
| `max_depth` | Integer | - | Maximum traversal depth |

## Unified Refactoring API

All refactoring follows the **plan → apply** pattern:

### Two-Step Pattern

1. **Generate plan** - Use `*.plan` tool (dry-run, never writes)
2. **Apply changes** - Use `workspace.apply_edit` (writes files)

### Refactoring Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| `rename.plan` | Rename symbol/file/directory | `target`, `new_name` |
| `extract.plan` | Extract function/variable | `kind`, `source`, `name` |
| `inline.plan` | Inline variable/function | `kind`, `target` |
| `move.plan` | Move code between files | `kind`, `source`, `destination` |
| `reorder.plan` | Reorder parameters/imports | `kind`, `target`, `options` |
| `transform.plan` | Transform code (e.g., to async) | `kind`, `target` |
| `delete.plan` | Delete unused code | `kind`, `target` |

### Apply Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `dry_run` | Boolean | false | Preview without applying |
| `backup` | Boolean | true | Create backup before changes |
| `fail_fast` | Boolean | true | Stop on first error |

## Rust-Specific Operations

### File Rename Auto-Updates

When renaming Rust files, automatic updates occur:

**Updates made:**
- Module declarations in parent files (`pub mod old;` → `pub mod new;`)
- Use statements (`use old::*` → `use new::*`)
- Qualified paths (`old::func()` → `new::func()`)
- Cross-crate imports (when moving between crates)

**Coverage:** Handles 80% of common scenarios. Complex nested module paths may require manual verification.

### Crate Consolidation

**Internal tool:** `rename_directory` with `consolidate: true` (backend use only)

**What it does:**
1. Moves `source-crate/src/*` → `target-crate/src/module/*`
2. Merges dependencies from source `Cargo.toml`
3. Removes source from workspace members
4. Updates all imports (`source_crate::*` → `target_crate::module::*`)
5. Deletes source crate directory

**Manual step required:** Add to `target-crate/src/lib.rs`:
```rust
pub mod module;  // Exposes consolidated code
```

## Cache Control

### Environment Variables

| Variable | Effect | Use Case |
|----------|--------|----------|
| `CODEBUDDY_DISABLE_CACHE=1` | Disable all caches | Debugging, testing |
| `CODEBUDDY_DISABLE_AST_CACHE=1` | Disable AST cache only | AST parser debugging |
| `CODEBUDDY_DISABLE_IMPORT_CACHE=1` | Disable import cache only | Import resolution issues |
| `CODEBUDDY_DISABLE_LSP_METHOD_CACHE=1` | Disable LSP method cache only | LSP protocol debugging |

See [CACHE_CONFIGURATION.md](configuration/CACHE_CONFIGURATION.md) for complete cache guide.

## Server Health Monitoring

### Health Check Tool

`health_check` returns:
- Server status and uptime
- LSP servers loaded
- Memory usage
- Cache statistics
- Active connections

### Status Command

```bash
codebuddy status
```

Shows:
- Server running state
- Language servers detected
- Configuration issues
- Connection health

## Language Support Matrix

| Language | Extensions | LSP Server | AST Parser | Refactoring Support |
|----------|-----------|------------|------------|---------------------|
| TypeScript/JavaScript | ts, tsx, js, jsx | typescript-language-server | SWC | Full ✅ |
| Rust | rs | rust-analyzer | syn | Full ✅ |
| Python* | py | pylsp | Native AST | Full ✅ |
| Go* | go | gopls | tree-sitter | Full ✅ |
| Java* | java | jdtls | tree-sitter | Full ✅ |
| Swift* | swift | sourcekit-lsp | tree-sitter | Full ✅ |
| C#* | cs | omnisharp | tree-sitter | Partial ⚠️ |

*Available in git tag `pre-language-reduction`

## Key Links

- [QUICKSTART.md](QUICKSTART.md) - Get running in 2 minutes
- [TOOLS_CATALOG.md](TOOLS_CATALOG.md) - Complete tool list
- [API_REFERENCE.md](API_REFERENCE.md) - Detailed API reference
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Developer guide
- [docs/architecture/overview.md](architecture/overview.md) - System architecture

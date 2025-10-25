# Development Guide

Complete guide for TypeMill development covering plugin creation, refactoring workflows, and utility libraries.

---

## Table of Contents

1. [Plugin Development](#plugin-development)
2. [Language Utilities (mill-lang-common)](#language-utilities-mill-lang-common)
3. [Workflows & Refactoring](#workflows--refactoring)
4. [Additional Resources](#additional-resources)

---

# Plugin Development

Fast reference for implementing new language plugins.

## Quick Start

Creating a new language plugin involves creating a standard Cargo crate and registering it with the `mill-plugin-bundle`.

| Step | Action | Notes |
|------|--------|-------|
| 1. Create Crate | `cargo new --lib crates/mill-lang-mynewlang` | Creates the standard plugin structure |
| 2. Implement Logic | Edit `src/lib.rs` to implement `LanguagePlugin` trait | See reference implementations below |
| 3. Register Plugin | Add to `mill-plugin-bundle/Cargo.toml` and `lib.rs` | Makes plugin discoverable via auto-discovery |
| 4. Build Workspace | `cargo build -p mill-lang-mynewlang` | Verify compilation |
| 5. Run Tests | `cargo nextest run -p mill-lang-mynewlang` | Add tests and verify functionality |

## Plugin Structure

All language plugins are independent crates in the `crates/` directory:

```
crates/
└── mill-lang-{language}/
    ├── Cargo.toml              # Dependencies and metadata
    └── src/
        ├── lib.rs              # Main plugin with LanguagePlugin trait
        ├── parser.rs           # Optional: Symbol extraction logic
        ├── manifest.rs         # Optional: Manifest parsing logic
        ├── import_support.rs   # Optional: Import capability traits
        └── workspace_support.rs # Optional: Workspace capability trait
```

## Core Trait: LanguagePlugin

| Method | Purpose | Required |
|--------|---------|----------|
| `metadata()` | Return language metadata | ✅ |
| `parse()` | Extract symbols from source code | ✅ |
| `analyze_manifest()` | Parse manifest files (package.json, Cargo.toml, etc.) | ✅ |
| `capabilities()` | Declare plugin capabilities (imports, workspace) | ✅ |
| `as_any()` | Enable downcasting | ✅ |

## Optional Capability Traits

**NEW: Segregated Import Traits (Interface Segregation Principle)**

Plugins now implement 5 focused traits instead of one monolithic `ImportSupport`:

### ImportParser (parsing only)
| Method | Purpose |
|--------|---------|
| `parse_imports()` | Extract import paths from content |
| `contains_import()` | Check if content imports a specific module |

### ImportRenameSupport (for file/symbol renames)
| Method | Purpose |
|--------|---------|
| `rewrite_imports_for_rename()` | Update imports when symbols/modules rename |

### ImportMoveSupport (for file moves)
| Method | Purpose |
|--------|---------|
| `rewrite_imports_for_move()` | Update imports when files move |

### ImportMutationSupport (add/remove imports)
| Method | Purpose |
|--------|---------|
| `add_import()` | Add a new import statement |
| `remove_import()` | Remove an existing import |
| `remove_named_import()` | Remove a specific symbol from an import |

### ImportAdvancedSupport (advanced operations)
| Method | Purpose |
|--------|---------|
| `update_import_reference()` | Advanced import reference updates |

**Benefits:**
- Simple plugins (like Markdown) only implement `ImportParser` (2 methods)
- Complex plugins (like Rust) implement all traits as needed
- 60% reduction in required code for simple plugins
- Clear separation of concerns

### WorkspaceSupport (if capabilities().workspace = true)

| Method | Purpose |
|--------|---------|
| `is_workspace_manifest()` | Detect workspace manifest files |
| `add_workspace_member()` | Add member to workspace |
| `remove_workspace_member()` | Remove member from workspace |
| `merge_dependencies()` | Merge dependencies between manifests |

## Plugin Registration (Auto-Discovery)

TypeMill uses compile-time auto-discovery via the `mill_plugin!` macro and `inventory` crate:

### Step 1: Self-Register Your Plugin

In your plugin's `src/lib.rs`:

```rust
use mill_plugin_api::mill_plugin;

mill_plugin! {
    name: "python",
    extensions: ["py", "pyi"],
    manifest: "pyproject.toml",
    capabilities: PythonPlugin::CAPABILITIES,
    factory: PythonPlugin::new,
    lsp: Some(LspConfig::new("pylsp", &[]))
}
```

### Step 2: Add to Plugin Bundle

Edit `crates/mill-plugin-bundle/Cargo.toml`:

```toml
[dependencies]
mill-lang-python = { path = "../mill-lang-python" }
```

Edit `crates/mill-plugin-bundle/src/lib.rs`:

```rust
use mill_lang_python::PythonPlugin;

fn _force_plugin_linkage() {
    let _: Option<PythonPlugin> = None;
    // ... existing plugins
}
```

**That's it!** The plugin is now automatically discovered and loaded.

## Parser Patterns

### Pattern A: Subprocess AST (Python, Node, Go, Java)

| Component | Purpose |
|-----------|---------|
| `SubprocessAstTool` | Spawn language-native parser |
| `run_ast_tool()` | Execute and deserialize JSON output |
| `parse_with_fallback()` | Try AST, fall back to regex |
| `ImportGraphBuilder` | Build structured import graph |

**When to use:** High accuracy needed, language runtime available

### Pattern B: Native Rust (Rust, simple languages)

| Component | Purpose |
|-----------|---------|
| Parser crate (`syn`, `tree-sitter`) | Parse directly in Rust |
| Manual AST traversal | Extract symbols |
| No subprocess | Zero overhead |

**When to use:** Rust parser available, performance critical

## Test Coverage Requirements

| Category | Minimum Tests | What to Test |
|----------|---------------|--------------|
| **Symbol extraction** | 10+ | Functions, classes, structs, enums, edge cases, unicode, syntax errors |
| **Import parsing** | 10+ | All import styles, aliases, multiple imports, empty files |
| **Manifest parsing** | 5+ | Valid manifests, invalid manifests, missing fields |
| **Refactoring** | 5+ | Import rewriting, workspace operations |
| **Total** | **30+** | Comprehensive coverage |

## Reference Implementations

| Use Case | Best Reference | Location |
|----------|---------------|----------|
| Native Rust parsing | Rust plugin | `crates/mill-lang-rust/` |
| Simple document plugin | Markdown plugin | `crates/mill-lang-markdown/` |
| Config file plugin | TOML/YAML plugins | `crates/mill-lang-toml/`, `crates/mill-lang-yaml/` |
| **Auto-Discovery Pattern** | **All current plugins** | **`src/lib.rs` - `mill_plugin!` macro** |
| Simple ImportParser only | Markdown plugin | `mill-lang-markdown/src/import_support.rs` |
| Full import trait suite | Rust plugin | `mill-lang-rust/src/import_support.rs` |
| WorkspaceSupport | Rust plugin | `mill-lang-rust/src/workspace_support.rs` |

## Implementation Checklist

### Core Trait
- [ ] `metadata()` returns correct language info
- [ ] `parse()` extracts all major symbol types
- [ ] `analyze_manifest()` parses manifest correctly
- [ ] `capabilities()` returns accurate flags
- [ ] `as_any()` implemented

### Import Support (if capabilities().imports = true)
**Choose traits based on plugin needs:**
- [ ] **ImportParser** (required): `parse_imports()` handles all import styles, `contains_import()` checks for imports
- [ ] **ImportRenameSupport** (if needed): `rewrite_imports_for_rename()` updates imports for renames
- [ ] **ImportMoveSupport** (if needed): `rewrite_imports_for_move()` updates relative paths for file moves
- [ ] **ImportMutationSupport** (if needed): `add_import()`, `remove_import()`, `remove_named_import()` for import mutations
- [ ] **ImportAdvancedSupport** (if needed): `update_import_reference()` for advanced operations

### Workspace Support (if capabilities().workspace = true)
- [ ] `is_workspace_manifest()` detects workspace files
- [ ] `add_workspace_member()` adds members correctly
- [ ] `remove_workspace_member()` removes members
- [ ] `merge_dependencies()` combines deps

### Additional Capability Traits (Optional)
**Choose based on plugin needs:**

**ManifestUpdater** (for dependency management):
- [ ] `manifest_updater()` discovery method implemented
- [ ] `update_dependency()` updates manifest files (Cargo.toml, package.json)

**ModuleLocator** (for module file discovery):
- [ ] `module_locator()` discovery method implemented
- [ ] `locate_module_files()` finds files for module paths

**RefactoringProvider** (for AST refactoring):
- [ ] `refactoring_provider()` discovery method implemented
- [ ] `supports_inline_variable()`, `supports_extract_function()`, `supports_extract_variable()` flags
- [ ] `plan_inline_variable()` generates inline variable refactoring plans
- [ ] `plan_extract_function()` generates extract function refactoring plans
- [ ] `plan_extract_variable()` generates extract variable refactoring plans

### Quality
- [ ] Uses `ImportGraphBuilder` for ImportGraph
- [ ] Uses `SubprocessAstTool` for external parsers (if applicable)
- [ ] Uses `read_manifest()` for file I/O
- [ ] Uses `parse_with_fallback()` for resilient parsing
- [ ] Uses `ErrorBuilder` for rich error context
- [ ] Minimum 30 tests total
- [ ] All tests pass: `cargo nextest run -p mill-lang-{language}`
- [ ] Uses structured logging (key-value format)
- [ ] Plugin-specific README.md with examples

### Integration
- [ ] `mill_plugin!` macro implemented in `src/lib.rs`
- [ ] Added to `mill-plugin-bundle/Cargo.toml` dependencies
- [ ] Added to `mill-plugin-bundle/src/lib.rs` `_force_plugin_linkage()`
- [ ] Workspace builds: `cargo build --workspace`

## Running Tests

| Command | Purpose |
|---------|---------|
| `cargo nextest run -p mill-lang-mylanguage` | Single plugin tests |
| `cargo nextest run -p mill-lang-mylanguage --no-capture` | With output |
| `cargo nextest run -p mill-lang-mylanguage test_parse_function` | Specific test |
| `cargo nextest run --workspace --lib` | All language plugin tests |
| `RUST_LOG=debug cargo nextest run -p mill-lang-mylanguage` | With verbose logging |

## Plugin Dispatch Patterns

> **📌 IMPORTANT: Capability-Based Dispatch is the ONLY Correct Pattern**
>
> All plugin dispatch in shared code MUST use capability traits. Downcasting and
> cfg guards are strictly forbidden. See warning below for details.

### ✅ Capability-Based Dispatch (REQUIRED - Proposals 05 & 07 Complete)

**Status:** Fully implemented and operational. **This is the only approved dispatch pattern.**

Language-specific operations now use trait-based capability queries with zero cfg guards:

```rust
// Example: Manifest updates using ManifestUpdater capability
let manifest_updater = plugin
    .manifest_updater()
    .ok_or_else(|| ApiError::Unsupported(
        format!("Plugin '{}' does not support manifest updates", plugin.metadata().name)
    ))?;

let updated_content = manifest_updater
    .update_dependency(path, old_dep, new_dep, new_path)
    .await?;
```

**Benefits:**
- ✅ Zero downcasting or cfg guards needed
- ✅ Plugins self-advertise capabilities
- ✅ Language-agnostic: shared code doesn't know about specific languages
- ✅ Easy to add new capabilities
- ✅ Clear error messages for unsupported operations

### New Capability Traits

See [docs/architecture/overview.md](architecture/overview.md) for complete details on ManifestUpdater, ModuleLocator, and RefactoringProvider traits.

### Migration Pattern: Old → New

> ⚠️ **WARNING: Downcasting is Strictly Forbidden**
>
> **NEVER use `as_any().downcast_ref::<ConcretePlugin>()` in shared code.**
> This pattern:
> - Breaks language-agnostic architecture
> - Reintroduces compile-time coupling
> - Defeats the purpose of the capability system
> - Will be rejected in code review
>
> **Always use capability traits** for language-specific operations. If a capability
> doesn't exist for your use case, create a new one instead of downcasting.

**Old pattern (DEPRECATED and FORBIDDEN - cfg guards + downcasting):**
```rust
match plugin.metadata().name {
    #[cfg(feature = "lang-rust")]
    "rust" => {
        plugin.as_any().downcast_ref::<RustPlugin>()?.method()
    }
    #[cfg(feature = "lang-typescript")]
    "typescript" => {
        plugin.as_any().downcast_ref::<TypeScriptPlugin>()?.method()
    }
    _ => Err(...)
}
```

**New pattern (capability-based):**
```rust
let capability = plugin.capability_trait()
    .ok_or_else(|| ApiError::Unsupported(...))?;
capability.method().await?
```

**Results:** 12 cfg guards removed, 2 downcasts eliminated, -48 net lines while adding MORE functionality.

---

# Language Utilities (mill-lang-common)

**Version**: 1.0.0-rc2
**Location**: `/workspace/crates/mill-lang-common`

Shared functionality for all language plugins to reduce boilerplate and maintain consistency.

## Quick Reference

| Module | Use When | Key Functions |
|--------|----------|---------------|
| **subprocess** | External AST parsers needed | `SubprocessAstTool`, `run_ast_tool()` |
| **import_graph** | Building ImportGraph | `ImportGraphBuilder` |
| **refactoring** | Code extraction, indentation | `LineExtractor`, `extract_lines()` |
| **parsing** | AST + fallback parsing | `parse_with_fallback()` |
| **error_helpers** | Rich errors with context | `rich_error()`, `error_with_context()` |
| **io** | File operations | `read_manifest()`, `read_source_file()` |
| **import_parsing** | Import statement parsing | `parse_import_alias()`, `parse_module_path()` |

## Core Utilities

### subprocess

Spawn external AST parsers (Python, Node, Go, Java) with automatic temp file management.

```rust
use mill_lang_common::{SubprocessAstTool, run_ast_tool};

let tool = SubprocessAstTool::new("node")
    .with_embedded_str(AST_TOOL_JS)
    .with_temp_filename("ast_tool.js")
    .with_arg("analyze-imports");

let result: Vec<MyImport> = run_ast_tool(tool, source)?;
```

**Builder methods:**
- `new(runtime: &str)` - Runtime command (e.g., "python3", "node", "go")
- `with_embedded_str(source: &str)` - Embed tool source code
- `with_temp_filename(name: &str)` - Temp file name
- `with_arg(arg: &str)` - Add argument
- `with_env(key, val)` - Set environment variable

### import_graph

Build `ImportGraph` with consistent structure.

```rust
use mill_lang_common::ImportGraphBuilder;

let graph = ImportGraphBuilder::new("python")
    .with_source_file(Some(path))
    .with_imports(imports)
    .extract_external_dependencies(is_external_dependency)
    .with_parser_version("1.0.0")
    .build();
```

### refactoring

Extract code ranges and detect indentation.

```rust
use mill_lang_common::{extract_lines, LineExtractor};

// Extract lines 10-15
let code = extract_lines(source, 10, 15)?;

// Get indentation
let indent = LineExtractor::get_indentation_str(source, line_num);
```

### parsing

Resilient parsing with fallback strategy.

```rust
use mill_lang_common::parse_with_fallback;

let imports = parse_with_fallback(
    source,
    |src| parse_with_ast(src),           // Try AST first
    |src| parse_with_regex(src),         // Fall back to regex
    "import parsing"
)?;
```

## File Operations

### io

Standardized file I/O with error handling.

```rust
use mill_lang_common::{read_manifest, read_source_file};

// Read manifest (package.json, Cargo.toml, etc.)
let content = read_manifest(path).await?;

// Read source file
let source = read_source_file(path).await?;
```

## Error Handling

### error_helpers

Rich error construction with context.

```rust
use mill_lang_common::{rich_error, error_with_context};

// Rich error with multiple context fields
return Err(rich_error(
    "Failed to parse AST",
    &[("file", path), ("parser", "babel"), ("line", &line.to_string())]
));
```

## Import Utilities

### import_parsing

Parse import statement syntax.

```rust
use mill_lang_common::{parse_import_alias, parse_module_path};

// Parse "foo as bar" -> ("foo", Some("bar"))
let (name, alias) = parse_import_alias("numpy as np");

// Parse "a.b.c" -> vec!["a", "b", "c"]
let path = parse_module_path("package.module.Class");
```

## Usage Statistics

Current adoption across 4 plugins (Go, Python, Rust, TypeScript):

| Utility | Go | Python | Rust | TypeScript | Total Uses |
|---------|---:|-------:|-----:|-----------:|-----------:|
| SubprocessAstTool | ✅ | ✅ | - | ✅ | 3 |
| ImportGraphBuilder | ✅ | ✅ | ✅ | ✅ | 4 |
| read_manifest | ✅ | ✅ | ✅ | ✅ | 4 |
| LineExtractor | ✅ | ✅ | ✅ | ✅ | 4 |
| parse_with_fallback | ✅ | ✅ | - | ✅ | 3 |

**Code reduction**: ~460 lines saved per plugin (average)

## Migration Example

### Before (TypeScript plugin, 40 lines)
```rust
let tmp_dir = Builder::new()
    .prefix("mill-ts-ast")
    .tempdir()
    .map_err(|e| PluginError::internal(format!("Failed to create temp dir: {}", e)))?;

let ast_tool_path = tmp_dir.path().join("ast_tool.js");
std::fs::write(&ast_tool_path, AST_TOOL_JS)
    .map_err(|e| PluginError::internal(format!("Failed to write AST tool: {}", e)))?;

let output = Command::new("node")
    .arg(&ast_tool_path)
    .arg("analyze-imports")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| PluginError::external_tool(format!("Failed to spawn node: {}", e)))?;
// ... 25 more lines of error handling ...
```

### After (10 lines)
```rust
use mill_lang_common::{SubprocessAstTool, run_ast_tool};

let tool = SubprocessAstTool::new("node")
    .with_embedded_str(AST_TOOL_JS)
    .with_temp_filename("ast_tool.js")
    .with_arg("analyze-imports");

let ts_imports: Vec<TsImportInfo> = run_ast_tool(tool, source)?;
```

**Savings**: 30 lines removed, cleaner code, better error handling

---

# Workflows & Refactoring

Fast reference for workflow automation and refactoring operations.

## Unified Refactoring API (Recommended)

All refactoring operations follow a safe two-step pattern:

| Step | Tool | Purpose | Modifies Files? |
|------|------|---------|-----------------|
| 1. Plan | `*.plan` | Preview changes | ❌ No |
| 2. Apply | `workspace.apply_edit` | Execute changes | ✅ Yes |

### Available Refactorings

| Tool | Purpose | Required Parameters |
|------|---------|---------------------|
| `rename.plan` | Rename symbol/file/directory | `file_path`, `line`, `character`, `new_name` |
| `extract.plan` | Extract function/variable | `file_path`, `range`, `new_name` |
| `inline.plan` | Inline variable | `file_path`, `line`, `character` |
| `move.plan` | Move code between files | `kind`, `source`, `destination` |
| `reorder.plan` | Reorder parameters/imports | `kind`, `target`, `options` |
| `transform.plan` | Transform code (e.g., to async) | `kind`, `target` |
| `delete.plan` | Delete unused code | `kind`, `target` |
| `workspace.apply_edit` | Apply any plan | `edit_id` (from plan), `options` (optional) |

### Safety Features

| Feature | How It Works | Benefit |
|---------|--------------|---------|
| **Mandatory Preview** | `*.plan` always returns preview | Can't accidentally apply changes |
| **Detailed Change Preview** | Shows all files, exact changes, counts | Full visibility before commit |
| **Double Preview** | `workspace.apply_edit` supports `dryRun: true` | Final check before execution |
| **Atomic Operations** | All files updated or none | Transaction-like behavior |
| **Edit Caching** | Plans cached 5 minutes | Time for review |

### Example Usage

```json
// Step 1: Plan (preview only)
{
  "name": "rename.plan",
  "arguments": {
    "file_path": "src/api.ts",
    "line": 10,
    "character": 5,
    "newName": "getData"
  }
}

// Response
{
  "edit_id": "550e8400-e29b-41d4-a716-446655440000",
  "changes": { "src/api.ts": [...], "src/client.ts": [...] },
  "summary": "Rename 'fetchData' to 'getData' (3 files, 12 occurrences)"
}

// Step 2: Apply
{
  "name": "workspace.apply_edit",
  "arguments": {
    "edit_id": "550e8400-e29b-41d4-a716-446655440000",
    "options": { "dryRun": false }
  }
}
```

## Best Practices

### For Refactoring

| Practice | Rationale |
|----------|-----------|
| Use Unified API | Simpler, safer than legacy workflows |
| Always preview first | Call `*.plan` before applying |
| Review changes carefully | Examine detailed preview |
| Use dry run for final check | Extra safety layer |
| Leverage atomic operations | All succeed or none applied |

## Advanced Features

| Feature | Purpose | Details |
|---------|---------|---------|
| **State Management** | Access previous results | `HashMap<usize, Value>`, `$steps.{index}` |
| **Dry-Run Mode** | Preview changes | No file modifications |
| **Workflow Metadata** | Estimate scope | Complexity score = number of steps |
| **Interactive Workflows** | User approval | Pause/resume with UUID |

---

# Additional Resources

## Documentation

- [Testing Guide](development/testing.md) - Comprehensive testing patterns
- [Logging Guidelines](development/logging_guidelines.md) - Structured logging standards
- [Architecture Overview](architecture/overview.md) - System architecture
- [Tools API Reference](tools/README.md) - Complete tool documentation

## Reference Implementations

- **Rust Plugin**: `crates/mill-lang-rust/` - Full-featured reference
- **TypeScript Plugin**: `crates/mill-lang-typescript/` - Subprocess parser pattern
- **Markdown Plugin**: `crates/mill-lang-markdown/` - Simple document plugin
- **TOML/YAML Plugins**: `crates/mill-lang-toml/`, `crates/mill-lang-yaml/` - Config files

## Key Principles

| Principle | Rationale |
|-----------|-----------|
| Use mill-lang-common utilities | Reduces boilerplate by ~460 lines |
| Follow existing patterns | Proven architecture, easier review |
| Write comprehensive tests | 30+ tests minimum for robust code |
| Use structured logging | Machine-readable, production-ready |
| Implement fallback parsers | Works in environments without runtime |
| Use capability traits | Language-agnostic architecture |

## Time Estimates

| Phase | Time |
|-------|------|
| Initial setup | 5 minutes (automated) |
| Core implementation | 1-2 days |
| Testing and polish | 1 day |
| **Total** | **2-3 days** |

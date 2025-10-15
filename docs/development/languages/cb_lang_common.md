# cb-lang-common Utility Reference

**Version**: 1.0.0-rc2
**Location**: `/workspace/crates/cb-lang-common`
**Last Updated**: 2025-10-07

Quick reference for the `cb-lang-common` utility crate that provides shared functionality for all language plugins. For detailed implementation examples, see [PLUGIN_DEVELOPMENT.md](../../PLUGIN_DEVELOPMENT.md).

---

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
| **location** | Source locations | `SourceLocationBuilder` |
| **versioning** | Version/dependency parsing | `parse_version()`, `parse_git_url()` |
| **ast_deserialization** | Subprocess JSON output | `deserialize_ast_output()` |
| **manifest_templates** | Manifest generation | `TomlManifestTemplate`, `JsonManifestTemplate` |
| **testing** | Plugin test utilities | `setup_test_env()`, `assert_symbols_eq()` |

---

## Core Utilities

### subprocess

Spawn external AST parsers (Python, Node, Go, Java) with automatic temp file management.

**SubprocessAstTool Builder**:
```rust
use cb_lang_common::{SubprocessAstTool, run_ast_tool};

let tool = SubprocessAstTool::new("node")
    .with_embedded_str(AST_TOOL_JS)
    .with_temp_filename("ast_tool.js")
    .with_arg("analyze-imports");

let result: Vec<MyImport> = run_ast_tool(tool, source)?;
```

**Builder methods**:
- `new(runtime: &str)` - Runtime command (e.g., "python3", "node", "go")
- `with_embedded_str(source: &str)` - Embed tool source code
- `with_temp_filename(name: &str)` - Temp file name
- `with_arg(arg: &str)` - Add argument
- `with_env(key, val)` - Set environment variable

**Source**: [`cb-lang-common/src/subprocess.rs`](../cb-lang-common/src/subprocess.rs)

---

### import_graph

Build `ImportGraph` with consistent structure.

```rust
use cb_lang_common::ImportGraphBuilder;

let graph = ImportGraphBuilder::new("python")
    .with_source_file(Some(path))
    .with_imports(imports)
    .extract_external_dependencies(is_external_dependency)
    .with_parser_version("1.0.0")
    .build();
```

**Methods**:
- `new(language: &str)` - Create builder
- `with_source_file(path: Option<&Path>)` - Source file path
- `with_imports(imports: Vec<Import>)` - Add imports
- `extract_external_dependencies(predicate: fn(&str) -> bool)` - Filter external deps
- `with_parser_version(version: &str)` - Parser version
- `build()` - Build ImportGraph

**Source**: [`cb-lang-common/src/import_graph.rs`](../cb-lang-common/src/import_graph.rs)

---

### refactoring

Extract code ranges and detect indentation.

```rust
use cb_lang_common::{extract_lines, LineExtractor};

// Extract lines 10-15
let code = extract_lines(source, 10, 15)?;

// Get indentation
let indent = LineExtractor::get_indentation_str(source, line_num);
```

**Functions**:
- `extract_lines(source: &str, start: usize, end: usize) -> Result<String>`
- `LineExtractor::get_indentation_str(source: &str, line: usize) -> &str`
- `LineExtractor::detect_indentation(source: &str) -> String`

**Source**: [`cb-lang-common/src/refactoring.rs`](../cb-lang-common/src/refactoring.rs)

---

### parsing

Resilient parsing with fallback strategy.

```rust
use cb_lang_common::parse_with_fallback;

let imports = parse_with_fallback(
    source,
    |src| parse_with_ast(src),           // Try AST first
    |src| parse_with_regex(src),         // Fall back to regex
    "import parsing"
)?;
```

**Source**: [`cb-lang-common/src/parsing.rs`](../cb-lang-common/src/parsing.rs)

---

## File Operations

### io

Standardized file I/O with error handling.

```rust
use cb_lang_common::{read_manifest, read_source_file};

// Read manifest (package.json, Cargo.toml, etc.)
let content = read_manifest(path).await?;

// Read source file
let source = read_source_file(path).await?;
```

**Functions**:
- `read_manifest(path: &Path) -> Result<String>` - Read manifest files
- `read_source_file(path: &Path) -> Result<String>` - Read source files
- `write_file(path: &Path, content: &str) -> Result<()>` - Write files

**Source**: [`cb-lang-common/src/io.rs`](../cb-lang-common/src/io.rs)

---

## Error Handling

### error_helpers

Rich error construction with context.

```rust
use cb_lang_common::{rich_error, error_with_context};

// Rich error with multiple context fields
return Err(rich_error(
    "Failed to parse AST",
    &[("file", path), ("parser", "babel"), ("line", &line.to_string())]
));

// Simple error with context
return Err(error_with_context("Parse failed", &format!("file: {}", path)));
```

**Source**: [`cb-lang-common/src/error_helpers.rs`](../cb-lang-common/src/error_helpers.rs)

---

## Import Utilities

### import_parsing

Parse import statement syntax.

```rust
use cb_lang_common::{parse_import_alias, parse_module_path};

// Parse "foo as bar" -> ("foo", Some("bar"))
let (name, alias) = parse_import_alias("numpy as np");

// Parse "a.b.c" -> vec!["a", "b", "c"]
let path = parse_module_path("package.module.Class");
```

**Source**: [`cb-lang-common/src/import_parsing.rs`](../cb-lang-common/src/import_parsing.rs)

---

## Manifest Operations

### manifest_templates

Generate manifest files (package.json, Cargo.toml, etc.).

```rust
use cb_lang_common::manifest_templates::{ManifestTemplate, TomlManifestTemplate};

let template = TomlManifestTemplate::new("package");
let manifest = template.generate("my-lib", "1.0.0", &["dep1", "dep2"]);
```

**Templates available**:
- `TomlManifestTemplate` - Cargo.toml, pyproject.toml
- `JsonManifestTemplate` - package.json

**Source**: [`cb-lang-common/src/manifest_templates.rs`](../cb-lang-common/src/manifest_templates.rs)

---

## Versioning

### versioning

Parse dependency versions and Git URLs.

```rust
use cb_lang_common::{parse_version, parse_git_url, GitUrl};

// Parse semantic version
let version = parse_version("^1.2.3")?;

// Parse Git URL
let git = parse_git_url("git+https://github.com/user/repo.git")?;
assert_eq!(git.host, "github.com");
assert_eq!(git.owner, "user");
assert_eq!(git.repo, "repo");
```

**Source**: [`cb-lang-common/src/versioning.rs`](../cb-lang-common/src/versioning.rs)

---

## Testing

### testing

Test utilities for plugin development.

```rust
use cb_lang_common::testing::{setup_test_env, assert_symbols_eq};

#[tokio::test]
async fn test_parser() {
    let env = setup_test_env();
    let symbols = parse(source)?;
    assert_symbols_eq(&symbols, &expected);
}
```

**Source**: [`cb-lang-common/src/testing.rs`](../cb-lang-common/src/testing.rs)

---

## Usage Statistics

Current adoption across 4 plugins (Go, Python, Rust, TypeScript):

| Utility | Go | Python | Rust | TypeScript | Total Uses |
|---------|---:|-------:|-----:|-----------:|-----------:|
| SubprocessAstTool | ✅ | ✅ | - | ✅ | 3 |
| ImportGraphBuilder | ✅ | ✅ | ✅ | ✅ | 4 |
| read_manifest | ✅ | ✅ | ✅ | ✅ | 4 |
| LineExtractor | ✅ | ✅ | ✅ | ✅ | 4 |
| parse_with_fallback | ✅ | ✅ | - | ✅ | 3 |
| manifest_templates | - | - | ✅ | - | 1 |

**Code reduction**: ~460 lines saved per plugin (average)

---

## Migration Example

### Before (TypeScript plugin, 40 lines)
```rust
let tmp_dir = Builder::new()
    .prefix("codebuddy-ts-ast")
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
use cb_lang_common::{SubprocessAstTool, run_ast_tool};

let tool = SubprocessAstTool::new("node")
    .with_embedded_str(AST_TOOL_JS)
    .with_temp_filename("ast_tool.js")
    .with_arg("analyze-imports");

let ts_imports: Vec<TsImportInfo> = run_ast_tool(tool, source)?;
```

**Savings**: 30 lines removed, cleaner code, better error handling

---

For detailed implementation examples, see:
- [PLUGIN_DEVELOPMENT.md](../../PLUGIN_DEVELOPMENT.md) - Step-by-step guide
- Plugin implementations: [`cb-lang-go`](../cb-lang-go/), [`cb-lang-python`](../cb-lang-python/), [`cb-lang-typescript`](../cb-lang-typescript/), [`cb-lang-rust`](../cb-lang-rust/)

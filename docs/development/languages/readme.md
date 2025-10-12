# Language Plugins

Language-specific plugins for Codebuddy, implementing the `LanguagePlugin` trait to provide AST parsing, symbol extraction, import analysis, and refactoring support.

> **⚠️ IMPORTANT**: Language support temporarily reduced to **TypeScript + Rust** during unified API refactoring.
>
> Python/Go/Java/Swift plugins are available in git tag `pre-language-reduction` and will be re-enabled
> after the API unification work is complete. This documentation reflects the reduced language set.

---

## 📚 Documentation

**For new plugin developers**, read these in order:

1. **[PLUGIN_DEVELOPMENT_GUIDE.md](plugin_development_guide.md)** - Complete step-by-step implementation guide
   - Quick start with automated scaffolding
   - Step-by-step implementation
   - Testing and troubleshooting
   - Common patterns and best practices

2. **[CB_LANG_COMMON.md](cb_lang_common.md)** - Quick reference for shared utilities
   - Subprocess utilities (`SubprocessAstTool`, `run_ast_tool`)
   - Import graph builder (`ImportGraphBuilder`)
   - File I/O helpers (`read_manifest`, `read_source_file`)
   - Error handling, parsing, testing utilities

---

## ⏱️ Feature Implementation Complexity

Understanding the scope of work before you start is crucial. Use this matrix to estimate the effort and coordination required for different features.

| Feature | Time Estimate | System Changes? | Prerequisites | Status |
|---------|---------------|-----------------|---------------|--------|
| **Basic Parsing** | 1-2 hours | ❌ No | Language runtime (for parser) | Required |
| **Manifest Parsing** | 1-2 hours | ❌ No | - | Required |
| **ImportSupport** | 2-4 hours | ❌ No | Trait exists | Optional |
| **WorkspaceSupport** | 2-4 hours | ❌ No | Trait exists | Optional |
| **RefactoringSupport** | 8-16 hours | ⚠️ **Maybe** | May need trait creation | Optional |

### What "System Changes" Means

- **No**: You can implement the feature entirely within your plugin's crate.
- **Maybe**: The feature may require creating a new trait or modifying core components in `cb-plugin-api` or `cb-ast`. This requires coordination with the core team.

### Before Implementing Advanced Features

If you plan to implement `RefactoringSupport` or a similar new, cross-cutting feature:
1. **Check if a trait already exists.**
2. If not, **create a GitHub issue** to discuss the design of the new trait.
3. **Do not proceed without design approval**, as this affects all language plugins.

---

## ⚠️ Common Pitfalls

When developing a new language plugin, you may encounter the following common issues.

### 1. Workspace Dependency Errors
- **Problem**: Your plugin's `Cargo.toml` uses `tempfile = { workspace = true }`, but `tempfile` is not defined as a workspace dependency in the root `Cargo.toml`. The build will fail with an unhelpful message.
- **Solution**: If you add a dependency manually, ensure it either exists in the root `Cargo.toml`'s `[workspace.dependencies]` table or specify a version directly in your plugin's `Cargo.toml` (e.g., `tempfile = "3.10.0"`).

### 2. External Parser Build Failures
- **Problem**: Your local build fails because a parser for another language is missing.
- **Solution**: Run `make build-parsers` from the root directory to build all required external parser artifacts. Currently only TypeScript and Rust are supported.

---

## 🏗️ Architecture Overview

### Plugin System

Each language plugin is a separate Rust crate implementing the `LanguagePlugin` trait from `cb-plugin-api`. Plugins use the `codebuddy_plugin!` macro to self-register with the system, making them automatically discoverable at runtime.

```
crates/
├── cb-lang-common/       # Shared utilities
├── cb-lang-rust/         # Rust language plugin (active)
├── cb-lang-typescript/   # TypeScript/JavaScript plugin (active)
├── cb-plugin-api/        # Core traits and types
└── cb-plugin-registry/   # Self-registration mechanism
```

**Note**: Python, Go, Java, and Swift plugins temporarily disabled. See git tag `pre-language-reduction`.

### Core Trait

```rust
#[async_trait]
pub trait LanguagePlugin: Send + Sync {
    fn metadata(&self) -> &LanguageMetadata;
    fn capabilities(&self) -> LanguageCapabilities;
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource>;
    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData>;
    fn as_any(&self) -> &dyn std::any::Any;

    // Optional capabilities
    fn import_support(&self) -> Option<&dyn ImportSupport> { None }
    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport> { None }
}
```

---

## 🚀 Quick Start

### New Plugin Development

Use the automated scaffolding script (when available):

```bash
# Script location: scripts/new-lang.sh (to be created)
# Creates plugin structure in crates/cb-lang-<name>/
```

Then follow the **[PLUGIN_DEVELOPMENT_GUIDE.md](plugin_development_guide.md)** for step-by-step implementation.

### Using cb-lang-common Utilities

**Before writing custom code**, check if cb-lang-common has what you need:

```rust
// Instead of manual subprocess handling (40 lines)
use cb_lang_common::{SubprocessAstTool, run_ast_tool};

let tool = SubprocessAstTool::new("node")
    .with_embedded_str(AST_TOOL_JS)
    .with_temp_filename("ast_tool.js")
    .with_arg("analyze-imports");

let imports = run_ast_tool(tool, source)?;  // 10 lines total
```

See **[CB_LANG_COMMON.md](cb_lang_common.md)** for complete utility reference.

---

## 📦 Active Plugins

### Currently Supported

| Language | Crate | Manifest | AST Parser | Import Support | Workspace |
|----------|-------|----------|------------|----------------|-----------|
| **TypeScript/JavaScript** | `cb-lang-typescript` | package.json | Node.js + Babel | ✅ | ✅ npm/yarn/pnpm |
| **Rust** | `cb-lang-rust` | Cargo.toml | Native syn | ✅ | ✅ Cargo workspaces |

### Temporarily Disabled

Python, Go, Java, and Swift plugins are temporarily disabled during unified API refactoring. They are available in git tag `pre-language-reduction` and will be re-enabled after the API unification is complete.

### Example: TypeScript Plugin

```rust
use cb_lang_typescript::TypeScriptPlugin;
use cb_plugin_api::LanguagePlugin;

let plugin = TypeScriptPlugin::new();

// Parse TypeScript source
let source = r#"
import React from 'react';

interface User {
    name: string;
}

function greet(user: User) {
    console.log(`Hello, ${user.name}!`);
}
"#;

let parsed = plugin.parse(source).await?;
assert_eq!(parsed.symbols.len(), 2);  // User interface + greet function

// Analyze package.json
let manifest = plugin.analyze_manifest(Path::new("package.json")).await?;
println!("Package: {}", manifest.name);
```

---

## 🔧 Development Workflow

### 1. Scaffolding
```bash
./new-lang.sh <language-name>
```

### 2. Implementation
Follow **[PLUGIN_DEVELOPMENT_GUIDE.md](plugin_development_guide.md)**:
- Implement `parse()` for symbol extraction
- Implement `analyze_manifest()` for dependency parsing
- Add import support (optional)
- Add workspace support (optional)

### 3. Testing
```bash
cd crates/languages/cb-lang-<your-language>
cargo test
```

### 4. Integration
```bash
cd crates/languages
./check-features.sh  # Verify all plugins compile
```

---

## 📖 Key Concepts

### ParsedSource

Result of parsing source code:

```rust
pub struct ParsedSource {
    pub data: serde_json::Value,    // Plugin-specific metadata
    pub symbols: Vec<Symbol>,        // Extracted symbols
}

pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,  // Function, Class, Interface, etc.
    pub location: SourceLocation,
    pub children: Vec<Symbol>,
    pub documentation: Option<String>,
}
```

### ManifestData

Result of parsing manifest files:

```rust
pub struct ManifestData {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<Dependency>,
    pub metadata: serde_json::Value,
}

pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub source: DependencySource,  // Registry, Git, Path, Workspace
    pub kind: DependencyKind,      // Runtime, Dev, Build, Optional
}
```

### ImportGraph

Result of import analysis:

```rust
pub struct ImportGraph {
    pub language: String,
    pub source_file: Option<PathBuf>,
    pub imports: Vec<Import>,
    pub external_dependencies: Vec<String>,
    pub parser_version: String,
}
```

---

## 🧪 Testing Strategy

Each plugin should have comprehensive tests:

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_parse_simple_code() {
        let plugin = MyLanguagePlugin::new();
        let source = "/* test code */";
        let result = plugin.parse(source).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_analyze_manifest() {
        let plugin = MyLanguagePlugin::new();
        let manifest = plugin.analyze_manifest(Path::new("test.manifest")).await;
        assert!(manifest.is_ok());
    }
}
```

See reference implementations for complete test coverage examples.

---

## 📝 Contributing

When adding a new language plugin:

1. Run `./new-lang.sh <language>` to scaffold
2. Implement following **[PLUGIN_DEVELOPMENT_GUIDE.md](plugin_development_guide.md)**
3. Use utilities from **[CB_LANG_COMMON.md](cb_lang_common.md)**
4. Write comprehensive tests (target: 30+ tests)
5. Call the `codebuddy_plugin!` macro in your plugin's `lib.rs` to enable self-registration.
6. Run `./check-features.sh` to verify integration
7. Submit PR with example code snippets

---

## 🎯 Design Principles

1. **Leverage cb-lang-common** - Don't reinvent subprocess handling, file I/O, or error formatting
2. **Graceful degradation** - Provide fallback implementations when external tools unavailable
3. **Comprehensive testing** - Test happy path, error cases, edge cases
4. **Clear error messages** - Use structured logging with context
5. **Consistent patterns** - Follow existing plugin implementations

---

For detailed implementation guidance, see **[PLUGIN_DEVELOPMENT_GUIDE.md](plugin_development_guide.md)**.

For utility reference, see **[CB_LANG_COMMON.md](cb_lang_common.md)**.
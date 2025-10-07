# Language Plugins

Language-specific plugins for Codebuddy, implementing the `LanguagePlugin` trait to provide AST parsing, symbol extraction, import analysis, and refactoring support.

---

## 📚 Documentation

**For new plugin developers**, read these in order:

1. **[PLUGIN_DEVELOPMENT_GUIDE.md](PLUGIN_DEVELOPMENT_GUIDE.md)** - Complete step-by-step implementation guide
   - Quick start with automated scaffolding
   - Step-by-step implementation
   - Testing and troubleshooting
   - Common patterns and best practices

2. **[CB_LANG_COMMON.md](CB_LANG_COMMON.md)** - Quick reference for shared utilities
   - Subprocess utilities (`SubprocessAstTool`, `run_ast_tool`)
   - Import graph builder (`ImportGraphBuilder`)
   - File I/O helpers (`read_manifest`, `read_source_file`)
   - Error handling, parsing, testing utilities

---

## 🏗️ Architecture Overview

### Plugin System

Each language plugin is a separate Rust crate implementing the `LanguagePlugin` trait from `cb-plugin-api`:

```
crates/languages/
├── cb-lang-common/       # Shared utilities (~460 LOC saved per plugin)
├── cb-lang-go/           # Go language plugin
├── cb-lang-python/       # Python language plugin
├── cb-lang-rust/         # Rust language plugin
├── cb-lang-typescript/   # TypeScript/JavaScript plugin
└── cb-plugin-api/        # Core traits and types
```

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

Use the automated scaffolding script:

```bash
cd crates/languages
./new-lang.sh java

# This creates:
# - crates/languages/cb-lang-java/
# - src/lib.rs (skeleton implementation)
# - Cargo.toml
# - tests/
```

Then follow the **[PLUGIN_DEVELOPMENT_GUIDE.md](PLUGIN_DEVELOPMENT_GUIDE.md)** for step-by-step implementation.

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

See **[CB_LANG_COMMON.md](CB_LANG_COMMON.md)** for complete utility reference.

---

## 📦 Existing Plugins

### Production-Ready

| Language | Crate | Manifest | AST Parser | Import Support | Workspace |
|----------|-------|----------|------------|----------------|-----------|
| **TypeScript/JavaScript** | `cb-lang-typescript` | package.json | Node.js + Babel | ✅ | ✅ npm/yarn/pnpm |
| **Python** | `cb-lang-python` | requirements.txt, pyproject.toml | Python + ast | ✅ | ✅ Poetry/pip |
| **Go** | `cb-lang-go` | go.mod | Go + go/parser | ✅ | ✅ Go modules |
| **Rust** | `cb-lang-rust` | Cargo.toml | Native syn | ✅ | ✅ Cargo workspaces |

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
Follow **[PLUGIN_DEVELOPMENT_GUIDE.md](PLUGIN_DEVELOPMENT_GUIDE.md)**:
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

## 🔗 Registry Integration

Plugins are registered in the main codebase at plugin initialization:

```rust
// In crates/cb-plugins/src/lib.rs
pub fn initialize_plugins() -> PluginRegistry {
    let mut registry = PluginRegistry::new();

    registry.register_language(Box::new(TypeScriptPlugin::new()));
    registry.register_language(Box::new(PythonPlugin::new()));
    registry.register_language(Box::new(GoPlugin::new()));
    registry.register_language(Box::new(RustPlugin::new()));

    registry
}
```

---

## 📝 Contributing

When adding a new language plugin:

1. Run `./new-lang.sh <language>` to scaffold
2. Implement following **[PLUGIN_DEVELOPMENT_GUIDE.md](PLUGIN_DEVELOPMENT_GUIDE.md)**
3. Use utilities from **[CB_LANG_COMMON.md](CB_LANG_COMMON.md)**
4. Write comprehensive tests (target: 30+ tests)
5. Update `crates/cb-plugins/src/lib.rs` to register plugin
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

For detailed implementation guidance, see **[PLUGIN_DEVELOPMENT_GUIDE.md](PLUGIN_DEVELOPMENT_GUIDE.md)**.

For utility reference, see **[CB_LANG_COMMON.md](CB_LANG_COMMON.md)**.

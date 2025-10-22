# TypeScript/JavaScript Language Plugin

Implements `LanguagePlugin` for TypeScript and JavaScript language support.

## Features

### Import Analysis
- Full AST-based import parsing using Node.js with Babel parser
- Fallback regex-based parsing when Node.js is unavailable
- Support for ES6 imports (`import ... from '...'`)
- Support for CommonJS (`require('...')`)
- Support for dynamic imports (`import('...')`)
- Support for type-only imports (`import type`)
- External dependency detection

### Symbol Extraction
- AST-based symbol extraction (functions, classes, interfaces, types, enums)
- Regular and async functions
- Arrow functions
- TypeScript interfaces and type aliases
- Enums
- Documentation comment extraction
- Graceful fallback when Node.js is unavailable

### Manifest Support
- package.json parsing and analysis
- Dependency extraction (dependencies, devDependencies, peerDependencies, optionalDependencies)
- Git, path, workspace, and registry dependencies
- Version range support (^, ~, >=, etc.)
- Dependency version updates
- Manifest generation for new packages

### Refactoring Support
- Module file location for TypeScript/JavaScript layout
- Import rewriting for file renames (ES6 + CommonJS + dynamic)
- Module reference finding with configurable scope
- Relative path calculation for imports

## Supported File Extensions

- `.ts` - TypeScript
- `.tsx` - TypeScript with JSX
- `.js` - JavaScript
- `.jsx` - JavaScript with JSX
- `.mjs` - ES Module JavaScript
- `.cjs` - CommonJS JavaScript

## Architecture

The plugin uses a **dual-mode approach** for parsing:

### 1. AST Mode (Primary)

Embeds `resources/ast_tool.js` and spawns it as a subprocess to leverage Node.js with Babel parser (`@babel/parser`) for accurate parsing of both TypeScript and JavaScript. Supports JSX/TSX through Babel plugins.

**Subprocess Communication:**
```
Rust Plugin → Node.js subprocess (ast_tool.js) → @babel/parser → JSON AST → Rust
```

### 2. Regex Mode (Fallback)

When Node.js is unavailable, falls back to regex-based parsing for basic import detection. Symbol extraction returns empty list in fallback mode.

**Fallback Detection:**
- Checks for `node` in PATH
- Validates `resources/ast_tool.js` existence
- Falls back gracefully on errors

This ensures the plugin works in environments without Node.js installed, while providing full features when Node.js is available.

## Installation

The plugin is automatically included when the `lang-typescript` feature is enabled (default).

```toml
[features]
default = ["lang-typescript"]
lang-typescript = ["dep:cb-lang-typescript"]
```

## Usage

```rust
use cb_lang_typescript::TypeScriptPlugin;
use cb_plugin_api::LanguagePlugin;
use std::path::Path;

let plugin = TypeScriptPlugin::new();

// Parse TypeScript source for symbols
let source = r#"
import React from 'react';

interface User {
    name: string;
    age: number;
}

function greet(user: User) {
    console.log(`Hello, ${user.name}!`);
}
"#;

let parsed = plugin.parse(source).await?;
assert!(!parsed.symbols.is_empty());

// Analyze package.json manifest
let manifest = plugin.analyze_manifest(Path::new("package.json")).await?;
println!("Package: {}", manifest.name);
```

## Testing

```bash
# Run all tests
cargo test -p cb-lang-typescript

# Run with output
cargo test -p cb-lang-typescript -- --nocapture

# Test with real TypeScript project
cd /path/to/typescript/project
cargo test -p cb-lang-typescript test_typescript_integration -- --nocapture
```

## Implementation Details

### Parser Module (`src/parser.rs`)

- `extract_symbols()` - Extract functions, classes, interfaces, types, enums
- `analyze_imports()` - Parse all import styles (ES6, CommonJS, dynamic)
- `parse_with_subprocess()` - Spawn Node.js subprocess for AST parsing
- `parse_with_regex()` - Fallback regex-based import extraction

### Manifest Module (`src/manifest.rs`)

- `load_package_json()` - Parse package.json into ManifestData
- `generate_manifest()` - Create package.json for new packages
- `update_dependency()` - Update dependency versions

### Refactoring Support

- `locate_module_files()` - Find .ts/.js files for a module path
- `parse_imports()` - Extract import statements from a file
- `rewrite_imports_for_rename()` - Update imports after file rename
- `find_module_references()` - Find all references to a module
- `calculate_relative_import()` - Compute relative import paths

## Limitations

- **Multiline imports**: May not fully parse complex multiline import statements in regex mode
- **Dynamic require()**: Regex mode cannot detect computed require paths
- **Type imports**: Regex mode may miss `import type` distinctions
- **JSX/TSX**: Full support requires Node.js AST mode

## Dependencies

- **@babel/parser** (embedded in ast_tool.js) - TypeScript/JavaScript parsing
- **Node.js** - Required for AST mode (optional for fallback mode)
- **regex** - Fallback import detection
- **tempfile** - Subprocess communication

## Contributing

When adding new features:

1. Update both AST mode (ast_tool.js) and regex fallback
2. Add unit tests for both modes
3. Use structured logging (key-value format)
4. Document any Node.js version requirements
5. Test with real TypeScript/JavaScript projects

## See Also

- [Language Plugins Guide](../README.md)
- [Plugin API Reference](../../mill-plugin-api/README.md)
- [Babel Parser Documentation](https://babeljs.io/docs/en/babel-parser)

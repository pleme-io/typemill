# Swift Language Plugin

Swift language support for Codebuddy via the `LanguagePlugin` trait.

## Configuration

- **Extensions**: swift
- **Manifest**: Package.swift
- **Source Directory**: Sources
- **Entry Point**: main.swift
- **Module Separator**: .

## Features

- [ ] AST parsing and symbol extraction
- [ ] Import/dependency analysis (ImportSupport trait)
- [ ] Workspace operations (WorkspaceSupport trait)
- [ ] Manifest file parsing

## Implementation Status

🚧 **Under Development**

This plugin has been scaffolded but requires implementation of its core features.

### Next Steps

1. **Implement parser.rs**: Add actual AST parsing logic
   - Use `SubprocessAstTool` from cb-lang-common for external parsers
   - Use `parse_with_fallback` for AST + regex pattern
   - Use `ErrorBuilder` for rich error context
   - Extract symbols (functions, classes, etc.)

2. **Implement manifest.rs**: Parse Package.swift files
   - Use `read_manifest` from cb-lang-common
   - Use `TomlWorkspace` or `JsonWorkspace` for workspace operations
   - Use `ErrorBuilder` for manifest errors
   - Extract project metadata and dependencies

3. **Add Import Support** (optional): Implement `ImportSupport` trait
   - Use `ImportGraphBuilder` from cb-lang-common
   - Use `parse_import_alias` and `split_import_list` helpers
   - Use `ExternalDependencyDetector` for dependency analysis

4. **Add Workspace Support** (optional): Implement `WorkspaceSupport` trait
   - Use workspace utilities from cb-lang-common
   - Use trait helper macros to reduce boilerplate

## Testing

```bash
# Run plugin tests
cargo test -p cb-lang-swift

# Run with output
cargo test -p cb-lang-swift -- --nocapture

# Test specific module
cargo test -p cb-lang-swift parser::tests
```

## Integration

This plugin has been automatically registered in:
- Root `Cargo.toml` workspace dependencies
- `crates/cb-handlers/Cargo.toml` with feature gate `lang-swift`
- `crates/cb-services/src/services/registry_builder.rs`
- `crates/cb-core/src/language.rs` (ProjectLanguage enum)
- `crates/cb-plugin-api/src/metadata.rs` (LanguageMetadata constant)

## Common Utilities (cb-lang-common)

This plugin has access to **cb-lang-common**, a utility crate with:

- **Subprocess utilities**: `SubprocessAstTool`, `run_ast_tool`
- **Parsing patterns**: `parse_with_fallback`, `try_parsers`
- **Error handling**: `ErrorBuilder` with context
- **Import utilities**: `ImportGraphBuilder`, `parse_import_alias`, `ExternalDependencyDetector`
- **File I/O**: `read_manifest`, `read_source`, `find_source_files`
- **Location tracking**: `LocationBuilder`, `offset_to_position`
- **Versioning**: `detect_dependency_source`, `parse_git_url`
- **Workspace ops**: `TomlWorkspace`, `JsonWorkspace`
- **Testing**: Test fixture generators and utilities

See [cb-lang-common documentation](../cb-lang-common/src/lib.rs) for complete API.

## References

- [Language Plugin Guide](../README.md)
- [Common Utilities Guide](../cb-lang-common/src/lib.rs)
- [API Documentation](../../cb-plugin-api/src/lib.rs)
- Reference implementations:
  - `cb-lang-rust` - Full implementation with import and workspace support
  - `cb-lang-go` - Dual-mode parser (subprocess + regex fallback)
  - `cb-lang-typescript` - Subprocess-based parser with ImportGraph
  - `cb-lang-python` - Python-specific patterns with subprocess
  - `cb-lang-java` - Java integration example

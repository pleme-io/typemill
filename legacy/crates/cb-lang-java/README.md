# Java Language Plugin

Java language support for Codebuddy via the `LanguagePlugin` trait.

## Features

- [x] AST parsing and symbol extraction
- [ ] Import/dependency analysis
- [x] Manifest file parsing (`pom.xml`, with placeholder support for Gradle)
- [ ] Refactoring support (rename, extract, etc.)

## Implementation Status

✅ **Active**

### Completed
- ✅ Plugin scaffolding and integration.
- ✅ Dual-mode AST parser using a native Java subprocess for symbol extraction.
- ✅ Manifest analysis for `pom.xml` files.
- ✅ Placeholder support for Gradle build files.

### TODO
- [ ] Implement full parsing for `build.gradle` and `build.gradle.kts` files.
- [ ] Implement import/dependency analysis.
- [ ] Add support for refactoring operations.

## Parser Strategy

This plugin uses a **Dual-Mode (Subprocess)** approach for parsing Java source code.

- **AST Mode**: A pre-compiled, embedded Java JAR (`java-parser-1.0.0.jar`) is executed as a subprocess. This tool uses the `com.github.javaparser` library to generate an accurate AST and extract symbols.
- **Fallback Mode**: If a Java runtime is not available in the environment, the parser will gracefully fall back to providing no symbols.

## Manifest Format

-   **Maven**: Full support for `pom.xml` files, including parsing of project coordinates and dependencies.
-   **Gradle**: Placeholder support for `build.gradle` and `build.gradle.kts`. Full implementation is pending.

## Testing

```bash
# Run plugin tests
cargo test -p cb-lang-java

# Run with output
cargo test -p cb-lang-java -- --nocapture
```text
## Registration

The plugin is registered in `crates/cb-services/src/services/registry_builder.rs` under the `lang-java` feature flag.

```rust
// Register Java plugin
# [cfg(feature = "lang-java")]
{
    registry.register(Arc::new(cb_lang_java::JavaPlugin::new()));
    plugin_count += 1;
}
```text
## References

- [Language Plugin Guide](../README.md)
- [Scaffolding Guide](./SCAFFOLDING.md)
- [API Documentation](../../cb-plugin-api/src/lib.rs)
- Reference implementations: `cb-lang-go`, `cb-lang-typescript`
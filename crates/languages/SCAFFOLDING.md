# Scaffolding a New Language Plugin

This guide provides a complete, step-by-step process for adding a new language plugin to CodeBuddy. Following these instructions will ensure your new plugin is correctly scaffolded, integrated, and verified.

## 1. Automated Scaffolding

The easiest way to create a new language plugin is to use the provided scaffolding script. This script will generate the necessary directory structure and boilerplate code.

From the `crates/languages` directory, run:

```bash
./new-lang.sh <language-name>
```

**Example:** To create a plugin for Java, you would run:

```bash
./new-lang.sh java
```

This command will create a new crate at `crates/languages/cb-lang-java` with a pre-configured `Cargo.toml`, a skeleton `src/lib.rs`, placeholder `src/parser.rs` and `src/manifest.rs` files, and a `README.md`.

The script will also print the manual integration steps that you must perform next.

## 2. Manual Integration Steps

After scaffolding, you must manually integrate the new crate into the CodeBuddy workspace. The following files need to be modified:

### A. Root `Cargo.toml`

In the workspace root `Cargo.toml` file, you only need to add the new crate to the list of workspace dependencies.

**Add to `[workspace.dependencies]`**:
```toml
# ...
[workspace.dependencies]
# ...
cb-lang-go = { path = "crates/languages/cb-lang-go" }
cb-lang-java = { path = "crates/languages/cb-lang-java" } # <--- Add this line
cb-lang-rust = { path = "crates/languages/cb-lang-rust" }
# ...
```

### B. `crates/cb-handlers/Cargo.toml`

The `cb-handlers` crate needs to be aware of the new plugin.

1.  **Add as an optional dependency**:
    ```toml
    [dependencies]
    # ...
    cb-lang-go = { workspace = true, optional = true }
    cb-lang-java = { workspace = true, optional = true } # <--- Add this line
    cb-lang-rust = { workspace = true, optional = true }
    # ...
    ```

2.  **Add a corresponding feature flag**:
    ```toml
    [features]
    # ...
    lang-go = ["dep:cb-lang-go"]
    lang-java = ["dep:cb-lang-java"] # <--- Add this line
    lang-rust = ["dep:cb-lang-rust"]
    # ...
    ```

### C. Plugin Registration (`crates/cb-services/src/services/registry_builder.rs`)

Finally, you must register the new plugin in the central registry. This makes it available to the rest of the application.

Open `crates/cb-services/src/services/registry_builder.rs` and add the registration logic, guarded by the feature flag you just created.

```rust
// ... in build_language_plugin_registry() ...

// Register Go plugin
#[cfg(feature = "lang-go")]
{
    registry.register(Arc::new(cb_lang_go::GoPlugin::new()));
    plugin_count += 1;
}

// Register Java plugin
#[cfg(feature = "lang-java")] // <--- Add this block
{
    registry.register(Arc::new(cb_lang_java::JavaPlugin::new()));
    plugin_count += 1;
}

// Register Rust plugin
#[cfg(feature = "lang-rust")]
{
    registry.register(Arc::new(cb_lang_rust::RustPlugin::new()));
    plugin_count += 1;
}

// ...
```

## 3. Implementation Strategy

With the plugin scaffolded and integrated, you can now implement the core logic in your new crate's `src` directory. There are two primary architectural patterns to choose from:

### A. Dual-Mode Subprocess Parser (Recommended for most languages)

This approach uses a native, external parser for the language, which is invoked as a subprocess. This provides highly accurate AST parsing. A simpler regex-based parser is used as a fallback if the required language toolchain (e.g., `node`, `go`, `java`) is not available.

-   **Pros**: Leverages mature, language-specific parsers. Extremely accurate.
-   **Cons**: Requires the user to have the language toolchain installed. Adds subprocess overhead.
-   **Reference Implementations**: `cb-lang-go`, `cb-lang-typescript`.

### B. Pure-Rust Parser

This approach uses a parsing library written in pure Rust (e.g., `syn` for Rust, `nom` for others) to parse the source code.

-   **Pros**: No external dependencies. Fast and self-contained.
-   **Cons**: Requires writing or finding a suitable parser in Rust, which can be a significant effort for complex languages.
-   **Reference Implementation**: `cb-lang-rust`.

## 4. Verification

After completing the integration and implementation, you must verify that everything is configured correctly.

1.  **Run the Feature Check Script**:
    From the `crates/languages` directory, run:
    ```bash
    ./check-features.sh
    ```
    This script will validate that the new plugin is correctly registered in all the necessary `Cargo.toml` files and the registry builder.

2.  **Run Tests**:
    Run the tests for your new plugin to ensure its logic is sound:
    ```bash
    cargo test -p cb-lang-<language-name>
    ```
    Also, run the entire workspace test suite to catch any integration issues:
    ```bash
    cargo test --workspace
    ```

## 5. Final Checklist

-   [ ] Run `./new-lang.sh <language-name>` to scaffold the crate.
-   [ ] Add the new crate to `[workspace.dependencies]` in the root `Cargo.toml`.
-   [ ] Add a `lang-<language-name>` feature to the root `Cargo.toml`.
-   [ ] Add the new crate as an optional dependency in `crates/cb-handlers/Cargo.toml`.
-   [ ] Add a corresponding `lang-<language-name>` feature in `crates/cb-handlers/Cargo.toml`.
-   [ ] Register the plugin in `crates/cb-services/src/services/registry_builder.rs`.
-   [ ] Implement the `LanguageIntelligencePlugin` trait in `src/lib.rs`.
-   [ ] Implement parsing logic in `src/parser.rs` and `src/manifest.rs`.
-   [ ] Add comprehensive unit and integration tests.
-   [ ] Run `./check-features.sh` to validate configuration.
-   [ ] Ensure `cargo test --workspace` passes.
-   [ ] Update the plugin's `README.md` with implementation details.
# C# Language Plugin

Provides C# language support for Codebuddy.

## Features

- AST parsing with a fallback to regex for robustness.
- `.csproj` manifest analysis for dependency extraction.
- Symbol extraction for classes, interfaces, methods, and properties.

## External Dependencies

This plugin relies on an external parser utility written in C#. To build this utility, you must have the **.NET SDK (6.0 or newer)** installed on your system.

You can check for all required parser dependencies by running:
```bash
make check-parser-deps
```
## Build Process

1.  **Build the external parser:**
    The C# parser is not built by the standard `cargo build` command. You must first build it using the following command from the repository root:
    ```bash
    make build-parsers
    ```
    This command will compile the C# parser and place the executable in the correct location for the Rust plugin to find at runtime.

2.  **Build the plugin:**
    Once the parser is built, you can compile the workspace as usual:
    ```bash
    cargo build --features lang-csharp
    ```

## Testing

To run the tests specifically for this plugin, use the following command:

```bash
cargo test -p cb-lang-csharp
```
## Supported Operations

- ✅ Parse C# source code using Roslyn (via external utility).
- ✅ Extract symbols (classes, interfaces, methods, properties).
- ✅ Analyze `.csproj` manifests.
- ✅ Extract `PackageReference` and `ProjectReference` dependencies.
- ⏳ Import rewriting (planned for a future release).
- ⏳ Workspace operations (planned for a future release).
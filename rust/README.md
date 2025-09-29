# Codeflow Buddy

An intelligent, extensible, language-aware developer assistant server built in Rust that provides powerful code intelligence and automated refactoring capabilities through the Model Context Protocol (MCP).

## Core Features

- **🔄 Project-Wide Refactoring** - AST-powered, safe refactoring across multiple files with automatic import updates
- **🚀 High-Performance Caching** - Intelligent AST caching with 300x+ performance improvements for repeated operations
- **🔌 Extensible Plugin System** - Language-specific implementations without core code modifications
- **🌐 Multi-Language Support** - TypeScript, JavaScript, Python, Go, Rust via Language Server Protocol
- **⚡ Atomic File Operations** - Safe, concurrent file modifications with rollback capabilities
- **🔒 Enterprise Security** - JWT authentication, TLS/WSS support, project-level access control
- **📊 Comprehensive Testing** - End-to-end test coverage with performance validation

## Architecture Overview

Codeflow Buddy uses a multi-crate workspace architecture for modularity and maintainability:

- **`cb-core`** - Foundational types, configuration, and error handling
- **`cb-ast`** - Abstract Syntax Tree parsing, analysis, and transformation engine
- **`cb-plugins`** - Plugin architecture for language-specific implementations
- **`cb-server`** - Core server with MCP protocol handlers and LSP client management
- **`cb-client`** - Command-line client for server interaction
- **`tests`** - Comprehensive integration testing framework

## Getting Started

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- Language servers for desired languages (e.g., `typescript-language-server`, `pylsp`)

### Building

```bash
# Clone the repository
git clone <repository-url>
cd rust

# Build in release mode for optimal performance
cargo build --release
```

### Running the Server

```bash
# Start the MCP server (stdio mode)
cargo run --release

# Start WebSocket server for remote connections
cargo run --release -- serve --port 3000

# With authentication
cargo run --release -- serve --port 3000 --require-auth --jwt-secret "your-secret"
```

### Configuration

Run the setup wizard to configure language servers:

```bash
cargo run --release -- setup
```

This creates `.codebuddy/config.json` with your LSP server configurations.

### Using the Client

```bash
# Interactive session
cargo run --release --bin cb-client -- connect

# Execute a single command
cargo run --release --bin cb-client -- call find_definition '{"file_path": "src/main.rs", "line": 10, "character": 5}'
```

## Example Usage

### Refactoring

```json
// Rename a symbol across the entire project
{
  "tool": "rename_symbol_with_imports",
  "arguments": {
    "oldName": "getUserData",
    "newName": "fetchUserProfile",
    "sourceFile": "src/api/users.ts"
  }
}
```

### Code Intelligence

```json
// Find all references to a symbol
{
  "tool": "find_references",
  "arguments": {
    "file_path": "src/main.ts",
    "line": 25,
    "character": 10
  }
}
```

## Development

```bash
# Run tests
cargo test

# Run specific test suite
cargo test --test e2e_refactoring

# Format code
cargo fmt

# Run linter
cargo clippy

# Check build
cargo check
```

## Performance

- AST caching provides 300-400x speedup for repeated operations
- Concurrent file operations with intelligent locking
- Atomic multi-file edits with rollback on failure
- Native Rust performance with zero-cost abstractions

## License

[Your License Here]

## Contributing

Contributions are welcome! Please ensure:
- All tests pass (`cargo test`)
- Code is formatted (`cargo fmt`)
- No clippy warnings for new code (`cargo clippy`)

## Acknowledgments

Built with love using:
- [Tree-sitter](https://tree-sitter.github.io/) for AST parsing
- [Tower LSP](https://github.com/tower-lsp/tower-lsp) for Language Server Protocol
- [Tokio](https://tokio.rs/) for async runtime
- The Rust community for excellent tooling and libraries
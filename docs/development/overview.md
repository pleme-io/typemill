# Development Overview

**Quick start guide for TypeMill contributors**

---

## 🚀 Quick Setup (5 minutes)

### Prerequisites
- Rust toolchain (`rustup` recommended)
- Git
- Basic Cargo knowledge

### Clone & Build
```bash
git clone https://github.com/goobits/typemill
cd typemill
cargo build
cargo nextest run  # Run fast tests
```

That's it! You're ready to contribute.

---

## 📁 Project Structure

```
typemill/
├── apps/
│   └── mill/              # Main CLI binary
├── crates/
│   ├── mill-server/       # MCP server core
│   ├── mill-handlers/     # Tool implementations
│   ├── mill-services/     # Business logic layer
│   ├── mill-lsp/          # LSP client
│   ├── mill-ast/          # AST processing
│   ├── mill-plugin-*/     # Plugin system
│   ├── mill-lang-*/       # Language plugins (rust, typescript, python)
│   └── mill-*/            # Various utilities
├── analysis/              # Analysis tools (dead code, dependencies, etc.)
├── tests/e2e/             # End-to-end tests
├── docs/                  # Documentation (you are here!)
└── xtask/                 # Build automation tasks
```

---

## 🛠️ Common Development Tasks

### Running Tests
```bash
# Fast tests only (~10s)
cargo nextest run

# With LSP servers (~60s, requires installed LSP servers)
cargo nextest run --features lsp-tests

# Full suite (~80s)
cargo nextest run --all-features

# Specific package
cargo nextest run -p mill-handlers
```

### Code Quality
```bash
cargo fmt                  # Format code
cargo clippy               # Lint code
cargo check                # Type check without building

# Run all checks
cargo xtask check-all
```

### Building
```bash
cargo build                # Debug build
cargo build --release      # Optimized build
```

### Running Mill Locally
```bash
# Run from source
cargo run -- --help
cargo run -- setup
cargo run -- start

# Or use built binary
./target/debug/mill --help
./target/release/mill setup
```

---

## 📖 Key Documentation

### Getting Started
- **[Plugin Development](plugin-development.md)** - Create language plugins
- **[Testing Guide](testing.md)** - Test architecture & workflow
- **[Logging Guidelines](logging_guidelines.md)** - Structured logging

### For GitHub workflow
- **[contributing.md](https://github.com/goobits/typemill/blob/main/contributing.md)** - Full contributing guide

### Architecture
- **[Architecture Overview](../architecture/overview.md)** - System design
- **[API Contracts](../architecture/api_contracts.md)** - Handler patterns
- **[Lang Common API](../architecture/lang_common_api.md)** - Language abstraction

---

## 🎯 Contribution Paths

### Path 1: Fix a Bug
1. Find an issue labeled `good first issue` on GitHub
2. Comment that you're working on it
3. Create a branch: `git checkout -b fix/issue-123`
4. Write test that reproduces the bug
5. Fix the bug
6. Ensure tests pass: `cargo nextest run`
7. Submit PR

### Path 2: Add a Language Plugin
1. Read **[Plugin Development](plugin-development.md)**
2. Use `cargo xtask new-lang <language>` to scaffold
3. Implement `LanguagePlugin` trait
4. Add tests
5. Submit PR

Example:
```bash
cargo xtask new-lang python
# Edit crates/mill-lang-python/src/lib.rs
cargo build -p mill-lang-python
cargo nextest run -p mill-lang-python
```

### Path 3: Add a New MCP Tool
1. Read **[contributing.md](https://github.com/goobits/typemill/blob/main/contributing.md)** (section on adding tools)
2. Add handler to `crates/mill-handlers/src/`
3. Register in tool registry
4. Add tests in `tests/e2e/`
5. Document in `docs/tools/`
6. Submit PR

### Path 4: Improve Documentation
1. Find documentation that needs improvement
2. Edit markdown files in `docs/`
3. Test with `mill docs <topic>`
4. Submit PR

---

## 🧪 Testing Philosophy

**Test categories:**
- **Unit tests**: In same file as code (`#[cfg(test)]`)
- **Integration tests**: In `tests/e2e/src/`
- **Fast tests** (default): Mock-based, ~10s
- **LSP tests**: Require real LSP servers, ~60s
- **Heavy tests**: Performance benchmarks, optional

**When to write tests:**
- ✅ Always write tests for new features
- ✅ Always write tests for bug fixes
- ✅ Prefer fast tests over LSP tests (mocks are faster)
- ✅ Use LSP tests only when testing LSP integration

**Running specific tests:**
```bash
# Run tests for one package
cargo nextest run -p mill-handlers

# Run tests matching a pattern
cargo nextest run -E 'test(rename)'

# Run with output
cargo nextest run --no-capture
```

---

## 📦 Package Overview

### Core Packages

| Package | Purpose | When to Edit |
|---------|---------|--------------|
| `mill` | CLI binary | Adding CLI commands |
| `mill-server` | MCP server | Server infrastructure |
| `mill-handlers` | Tool implementations | Adding/modifying tools |
| `mill-services` | Business logic | Core functionality |
| `mill-lsp` | LSP client | LSP integration |

### Plugin System

| Package | Purpose | When to Edit |
|---------|---------|--------------|
| `mill-plugin-api` | Plugin traits | Never (stable API) |
| `mill-plugin-system` | Plugin registry | Rarely |
| `mill-plugin-bundle` | Plugin collection | Auto-generated |
| `mill-lang-*` | Language plugins | Adding language support |

### Analysis Tools

| Package | Purpose | When to Edit |
|---------|---------|--------------|
| `mill-analysis-common` | Shared analysis code | Common analysis logic |
| `mill-analysis-dead-code` | Dead code detection | Improving dead code analysis |
| `mill-analysis-graph` | Dependency graphs | Graph algorithms |
| `mill-analysis-circular-deps` | Circular dependency detection | Circular dep logic |

---

## 🔧 Build Automation (xtask)

Mill uses the **xtask pattern** for cross-platform build tasks:

```bash
cargo xtask install           # Install mill to ~/.local/bin
cargo xtask check-all         # Run all checks (fmt, clippy, test, deny)
cargo xtask check-duplicates  # Check for duplicate code
cargo xtask new-lang <name>   # Scaffold new language plugin
cargo xtask sync-languages    # Sync language registry
cargo xtask --help            # Show all tasks
```

**Why xtask?**
- Cross-platform (works on Windows, Mac, Linux)
- Written in Rust (no shell script dependencies)
- Easy to extend

---

## 🐛 Debugging Tips

### Print Debugging
```rust
tracing::debug!("Value: {:?}", value);
tracing::info!("Processing file: {}", path);
```

### Running with Logs
```bash
RUST_LOG=debug cargo run -- start
RUST_LOG=mill_handlers=trace cargo run -- tool find_definition '{...}'
```

### Test Debugging
```bash
# Run single test with output
cargo nextest run test_name --no-capture

# Run with backtrace
RUST_BACKTRACE=1 cargo nextest run test_name
```

### LSP Debugging
```bash
# Enable LSP client logs
RUST_LOG=mill_lsp=trace cargo run -- start
```

---

## 📝 Code Style

**Follow Rust conventions:**
- Use `rustfmt` for formatting (run `cargo fmt`)
- Use `clippy` for linting (run `cargo clippy`)
- Write doc comments for public APIs (`///`)
- Use structured logging (see [logging_guidelines.md](logging_guidelines.md))

**Naming:**
- `snake_case` for functions, variables, modules
- `PascalCase` for types, traits
- `SCREAMING_SNAKE_CASE` for constants

---

## 🚢 Release Process

**For maintainers only:**

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite: `cargo nextest run --all-features`
4. Build release: `cargo build --release`
5. Tag release: `git tag v0.x.0`
6. Push: `git push && git push --tags`
7. CI builds and publishes

---

## 🤝 Getting Help

**Stuck? Need guidance?**

1. **Check docs**: `mill docs` or browse `docs/`
2. **Search issues**: https://github.com/goobits/typemill/issues
3. **Ask on GitHub Discussions**: https://github.com/goobits/typemill/discussions
4. **Open an issue**: Describe what you're trying to do

**Before asking:**
- ✅ Search existing issues/discussions
- ✅ Include error messages
- ✅ Share relevant code snippets
- ✅ Mention what you've already tried

---

## 📚 Next Steps

1. **[Plugin Development](plugin-development.md)** - Create language plugins
2. **[Testing Guide](testing.md)** - Write tests
3. **[Logging Guidelines](logging_guidelines.md)** - Structured logging
4. **[Architecture Overview](../architecture/overview.md)** - Understand system design
5. **[contributing.md](https://github.com/goobits/typemill/blob/main/contributing.md)** - Full workflow

---

**Welcome to the TypeMill project! We're excited to have you contribute.** 🎉

# Contributing to TypeMill

> **📌 New to the project?** This guide is for developers building from source.
> End users: see [README.md](README.md) for installation instructions.

First off, thank you for considering contributing! It's people like you that make TypeMill such a great tool.

---

## 🚀 Quick Start

### Prerequisites

- **Rust Toolchain:** Get it from [rustup.rs](https://rustup.rs/)
- **Git:** For cloning the repository
- **Node.js & npm:** For TypeScript LSP (optional, can be auto-installed by `mill setup`)

### Setup Workflow

```bash
# 1. Clone the repository
git clone https://github.com/goobits/typemill.git
cd typemill

# 2. Build the project
cargo build

# 3. Run tests
cargo nextest run --workspace

# 4. Configure LSP servers (optional)
./target/debug/mill setup
```text
That's it! You're ready to contribute.

**For detailed setup including parser builds and development tools:**
- See **[docs/development/overview.md](docs/development/overview.md)** - Complete contributor quickstart

---

## 🧪 Running Tests

We use [cargo-nextest](https://nexte.st/) for faster test execution.

### Quick Commands

```bash
# Fast tests (recommended for local development)
cargo nextest run --workspace

# With LSP server tests (~60s, requires LSP servers installed)
cargo nextest run --workspace --features lsp-tests

# Full suite with all features (~80s)
cargo nextest run --workspace --all-features

# Specific package
cargo nextest run -p mill-handlers
```text
### Makefile Shortcuts

```bash
make test           # Run fast tests
make test-full      # Run all tests including skipped
make test-lsp       # Run tests requiring LSP servers
make check          # Run fmt + clippy + test + audit
```text
**For detailed testing workflows, watch mode, and focused development:**
- See **[docs/development/testing.md](docs/development/testing.md)** - Complete testing guide
- See **[docs/development/overview.md](docs/development/overview.md#running-tests)** - Quick test reference

---

## 🎨 Code Style and Linting

We use standard Rust formatting and linting tools to maintain a consistent codebase.

### Before Committing

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --all-targets -- -D warnings

# Run all checks
make check  # Runs fmt + clippy + test + audit + deny
```text
### Quality Checks

```bash
# Check for duplicate code and complexity
make check-duplicates

# Check cargo features
cargo xtask check-features

# Security audit
cargo audit
```text
**Structured Logging:**
Always use structured key-value logging:

```rust
// ✅ Good - structured logging
debug!(tool_name = %tool_call.name, file_path = %path, "Processing tool call");
error!(error = %e, tool = "get_diagnostics", "Tool execution failed");

// ❌ Bad - string interpolation
debug!("Processing tool call {} for file {}", tool_call.name, path);
```text
See **[docs/development/logging_guidelines.md](docs/development/logging_guidelines.md)** for complete logging standards.

---

## 🛠️ Build Automation (xtask)

This project uses the **xtask pattern** for build automation. Instead of shell scripts, we write automation tasks in Rust for cross-platform compatibility.

### Available Tasks

```bash
cargo xtask install              # Install mill to ~/.local/bin
cargo xtask check-all            # Run all checks (fmt, clippy, test, deny)
cargo xtask check-duplicates     # Check for duplicate code
cargo xtask check-features       # Validate cargo features
cargo xtask new-lang <language>  # Create new language plugin
cargo xtask --help               # Show all available commands
```text
### Why xtask?

- ✅ **Cross-platform**: Works on Windows, Linux, and macOS
- ✅ **Type-safe**: Full Rust IDE support with compile-time checking
- ✅ **Integrated**: Uses Rust ecosystem (cargo API, file operations)
- ✅ **Maintainable**: Easier to test and debug than shell scripts

**For details on adding new xtask commands:**
- See **[docs/development/overview.md#build-automation-xtask](docs/development/overview.md#build-automation-xtask)**

---

## 📦 Dependency Management

Before adding new dependencies to the project, please follow these guidelines:

1. **Check if functionality already exists** in the workspace or standard library
2. **Evaluate the dependency's**:
   - Maintenance status (recent commits, active maintainers)
   - License compatibility (MIT, Apache-2.0, BSD preferred)
   - Security track record
   - Binary size impact
3. **Run dependency checks** to ensure no issues are introduced:
   ```bash
   cargo deny check
   make deny
   ```

### Running Dependency Checks

```bash
# Check all: advisories, licenses, bans, sources
cargo deny check

# Check only security advisories
cargo deny check advisories

# Check only licenses
cargo deny check licenses

# Check only duplicate dependencies
cargo deny check bans

# Update advisory database
cargo deny fetch
```text
### Handling cargo-deny Failures

If `cargo deny check` fails:

- **Advisories (Security Vulnerabilities):**
  - Investigate the CVE/advisory details
  - Assess risk for our use case
  - Update dependency if patch is available
  - If no patch exists, document why it's accepted in `deny.toml`

- **Licenses:**
  - Ensure new dependency has compatible license (MIT/Apache-2.0/BSD)
  - Copyleft licenses (GPL, AGPL) are not allowed
  - Add license exceptions only with team approval

- **Bans (Duplicate Dependencies):**
  - Try to use workspace version instead of adding new version
  - Consolidate versions where possible
  - If duplicate is unavoidable (transitive dependency), document reason in `deny.toml`

- **Sources:**
  - Prefer crates.io over git dependencies
  - Git dependencies allowed only for patches/forks with clear justification
  - Document why git source is necessary

### Example: Adding a New Dependency

```toml
# Good - use workspace version
[dependencies]
serde = { workspace = true }

# Good - compatible license, latest stable
reqwest = { version = "0.12", features = ["rustls-tls"], default-features = false }

# Bad - introduces duplicate version
dashmap = "6.0"  # Workspace uses 5.5

# Bad - git dependency without justification
my-crate = { git = "https://github.com/..." }
```text
---

## 🔄 Pull Request Process

1. **Create a Feature Branch:**
   ```bash
   git checkout -b your-feature-name
   ```

2. **Commit Your Changes:** Make your changes and commit them with a descriptive message.
   ```bash
   git commit -m "feat: Add new feature" -m "Detailed description of the changes."
   ```

3. **Ensure Tests Pass:** Run the tests one last time to make sure everything is working correctly.
   ```bash
   make test
   ```

4. **Push to Your Branch:**
   ```bash
   git push origin your-feature-name
   ```

5. **Open a Pull Request:** Go to the repository on GitHub and open a new pull request. Provide a clear title and description of your changes.

### Commit Message Guidelines

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks

---

## 📖 Development Guides

### Adding New Language Plugins

To add support for a new programming language:

- **[docs/development/plugin-development.md](docs/development/plugin-development.md)** - Complete plugin development guide
  - Plugin structure and schema requirements
  - Required trait implementations (`LanguagePlugin`)
  - Data types (ParsedSource, Symbol, ManifestData)
  - Plugin registration and testing
  - Capability trait patterns (ManifestUpdater, ModuleLocator, RefactoringProvider)
  - Reference implementations (Rust, TypeScript, Python)

**Quick reference:**
```bash
# Create new language plugin scaffold
cargo xtask new-lang <language>

# Implement LanguagePlugin trait
# Edit crates/mill-lang-<language>/src/lib.rs

# Build and test
cargo build -p mill-lang-<language>
cargo nextest run -p mill-lang-<language>
```text
### Adding New MCP Tools

To add new tools and handlers to the system:

- **[docs/development/overview.md#adding-new-mcp-tools](docs/development/overview.md#adding-new-mcp-tools)** - Tool creation workflow
  - Understanding the Unified Refactoring API (dryRun pattern)
  - Understanding the Unified Analysis API
  - Adding a tool to an existing handler
  - Creating a new handler
  - Best practices for naming, logging, error handling, testing

**Handler organization:**
- Navigation tools → `crates/mill-handlers/src/handlers/tools/navigation.rs`
- Analysis tools → `crates/mill-handlers/src/handlers/tools/analysis/*.rs`
- Refactoring tools → `crates/mill-handlers/src/handlers/<operation>_handler.rs`
- System tools → `crates/mill-handlers/src/handlers/tools/system.rs`

### Testing Guide

Comprehensive testing documentation:

- **[docs/development/testing.md](docs/development/testing.md)** - Testing architecture and workflow
  - Test categories (fast, LSP, e2e, heavy)
  - Focused development workflows
  - Watch mode for incremental development
  - Integration test filtering
  - Mock-based testing patterns

---

## ⚡ Build Performance

### Quick Tips

```bash
# Fast feedback during development (doesn't build binaries)
cargo check

# Check sccache statistics
sccache --show-stats

# Clear cache if having issues
sccache --zero-stats
rm -rf target/
cargo build
```text
### Build Times Reference

With sccache and mold installed:

| Build Type | Time (First) | Time (Incremental) |
|------------|--------------|-------------------|
| `cargo check` | ~30s | 2-5s |
| `cargo build` | ~2m | 5-20s |
| `cargo build --release` | ~3m | 30-60s |
| `cargo nextest run` | ~2m | 8-25s |

**For detailed build optimization and troubleshooting:**
- See **[docs/development/overview.md#build-performance-tips](docs/development/overview.md#build-performance-tips)**
- Docker/container-specific issues covered in **[docs/operations/docker_deployment.md](docs/operations/docker_deployment.md)**

---

## 📚 Additional Resources

### Documentation

- **[docs/README.md](docs/README.md)** - Documentation index and navigation hub
- **[docs/user-guide/getting-started.md](docs/user-guide/getting-started.md)** - Complete setup guide
- **[docs/user-guide/cheatsheet.md](docs/user-guide/cheatsheet.md)** - Quick command reference
- **[docs/development/overview.md](docs/development/overview.md)** - Complete contributor quickstart
- **[docs/architecture/overview.md](docs/architecture/overview.md)** - System architecture

### CLI Documentation Viewer

View all documentation offline:

```bash
mill docs                          # List all topics
mill docs development/overview     # View contributor guide
mill docs tools/refactoring        # View refactoring tools
mill docs --search "plugin"        # Search all documentation
```text
### Getting Help

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

## 🙏 Thank You!

Your contributions make TypeMill better for everyone. We appreciate your time and effort in improving this project.

**Welcome to the TypeMill community!** 🎉
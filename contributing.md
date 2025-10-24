# Contributing to Codebuddy

> **📌 New to the project?** This guide is for developers building from source.
> End users: see [README.md](readme.md) for installation instructions.

First off, thank you for considering contributing! It's people like you that make Codebuddy such a great tool.

## Getting Started

### Prerequisites

Building the full project requires the following tools. You can verify them all at once by running `make check-parser-deps`.

**Note:** A complete build and a passing test suite require the installation of external SDKs (Java, .NET, Node.js). Without these, parser builds will fail, which will cause tests for the corresponding language plugins to fail.

- **Rust Toolchain:** Get it from [rustup.rs](https://rustup.rs/).
- **Java SDK & Maven:** Required to build the Java parser.
- **.NET SDK:** Required to build the C# parser.
- **Node.js & npm:** Required to build the TypeScript parser.
- **Git:** For cloning the repository.
- **(Optional) SourceKitten:** For Swift language support.

### Setup Tools Explained

We use a few different tools for setup. Here's what each one is for:

| Tool | Who is it for? | Purpose |
|---|---|---|
| `install.sh` | **End Users** | Automated installer. Builds from source and copies the `codebuddy` binary to your system. |
| `make first-time-setup` | **Developers** | **THE complete setup command**. Installs all dev tools (cargo-nextest, sccache, mold), builds parsers, builds project, installs LSP servers, validates everything (~3-5 min). |
| `make build-parsers` | **Developers** | Builds only the external language parsers (Java, C#, TypeScript). Usually not needed directly—included in `first-time-setup`. |
| `make build` | **Developers** | Builds the core Rust project only. |
| `codebuddy setup` | **Both** | A runtime configuration wizard that helps you configure Language Server Protocol (LSP) servers for your projects. |

### Developer Setup Workflow

For the best first-time setup experience, we recommend using the `Makefile` targets.

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/goobits/codebuddy.git
    cd codebuddy
    ```

2.  **Run the first-time setup command:**
    ```bash
    make first-time-setup
    ```
    This single command will:
    a. Check that you have all the necessary prerequisites.
    b. Install recommended development tools (`sccache`, `mold`).
    c. Build all the external language parsers.
    d. Build the main Rust project.

3.  **Configure Language Servers:**
    ```bash
    # Run the interactive setup wizard
    codebuddy setup
    ```
    This will detect your project languages and help you set up the necessary LSP servers.

## Running Tests

This project uses [cargo-nextest](https://nexte.st/) for running tests. It's faster, provides better output, and has become the standard for modern Rust projects.

### Installation

If you ran `make first-time-setup`, `cargo-nextest` is already installed. Otherwise, `make test` will auto-install it for you.

### Usage

The easiest way to run tests is with the `Makefile`:

```bash
# Run fast tests (recommended for local development)
make test

# Run the full test suite, including skipped tests
make test-full

# Run tests that require LSP servers
make test-lsp
```

You can also run `cargo-nextest` directly for more granular control:

```bash
# Run all workspace tests/e2e
cargo nextest run --workspace

# Run a specific test file
cargo nextest run --test lsp_features

# Run ignored/skipped tests/e2e
cargo nextest run --status-level skip
```

### Focused Development Workflows

For faster iteration when working on specific subsystems, use the focused test commands:

```bash
# Analysis crates only (extremely fast: ~0.02s for 21 tests)
make test-analysis
cargo test-analysis

# Handlers only - minimal features (fast: ~0.1s for 37 tests, 13s total)
make test-handlers
cargo test-handlers-core

# Core libraries (excludes integration tests: ~49s for 576 tests)
make test-core
cargo test-core

# Language plugins only (fast: ~0.3s for 193 tests)
make test-lang
cargo test-lang
```

**Watch mode for incremental development** (auto-rebuild on file changes, debug mode):

```bash
# Watch handlers (fastest iteration)
make dev-handlers

# Watch analysis crates
make dev-analysis

# Watch core libraries
make dev-core

# Watch language plugins
make dev-lang
```

**Check-only commands** (faster than testing, no binary builds):

```bash
make check-handlers    # Check handlers only
make check-analysis    # Check analysis only
make check-core        # Check core libraries
make check-lang        # Check language plugins
```

These focused commands exclude analysis features from handlers by default, significantly speeding up compilation times. They're perfect for tight iteration loops when working on specific parts of the codebase.

### Navigation-Only Builds (No Refactoring)

When working on navigation, analysis, or LSP features without needing refactoring operations, use the navigation-only builds:

```bash
# Check handlers without refactoring features (15-25% faster)
make check-handlers-nav
cargo check-handlers-nav

# Test handlers without refactoring features (15-25% faster)
make test-handlers-nav
cargo test-handlers-nav
```

**Performance gains:**
- **Compilation:** 15-25% faster than full handlers build
- **Why:** Excludes all refactoring handlers (rename, extract, inline, move, reorder, transform, delete)
- **When to use:** Navigation features, analysis tools, LSP integration, diagnostics

### Integration Test Filtering

For faster targeted testing of specific functionality:

```bash
# Test only refactoring operations (60-80% faster)
make test-integration-refactor
cargo test-integration-refactor

# Test only analysis operations (60-80% faster)
make test-integration-analysis
cargo test-integration-analysis

# Test only navigation operations (60-80% faster)
make test-integration-nav
cargo test-integration-nav
```

**Performance gains:**
- **Test execution:** 60-80% faster than running full integration test suite
- **Why:** Uses nextest's filter expressions to run only matching tests
- **When to use:** Iterating on specific features, debugging test failures, pre-commit checks

**Examples:**
```bash
# Working on rename functionality? Test only rename-related integration tests
make test-integration-refactor

# Working on dead code analysis? Test only analysis integration tests
make test-integration-analysis

# Working on find_definition? Test only navigation integration tests
make test-integration-nav
```

### Single-Language Builds (Not Currently Supported)

**Note:** Single-language builds (Rust-only or TypeScript-only) are **not currently feasible** without significant architectural changes. This limitation is documented here for future reference.

**Why not supported:**
- Language plugins are hard-wired as unconditional dependencies across multiple crates (`cb-ast`, `mill-services`, `cb-plugins`, `apps/codebuddy`)
- Code contains direct downcasts to concrete plugin types (e.g., `plugin.downcast_ref::<TypeScriptPlugin>()`)
- Services eagerly link all language crates and call directly into them
- Would require extensive refactoring to feature-gate all cross-language references

**Estimated effort to enable:** 2-3 weeks

**For details on blockers and a complete solution design**, see [proposals/07_single_language_builds.proposal.md](../proposals/07_single_language_builds.proposal.md).

**Key insight:** The solution involves replacing downcasting with capability traits (similar to LSP's capabilities model), which would also improve scalability as we add more languages.

## Code Style and Linting

We use the standard Rust formatting and linting tools to maintain a consistent codebase.

- **Formatting:** Before committing your changes, please format your code with `cargo fmt`.
  ```bash
  cargo fmt --all
  ```

- **Linting:** We use `clippy` for catching common mistakes and improving code quality.
  ```bash
  cargo clippy --all-targets -- -D warnings
  # Or use Makefile
  make clippy
  ```

- **Code Quality Checks:**
  ```bash
  make check                # Run fmt + clippy + test + audit + deny
  make check-duplicates     # Detect duplicate code & complexity
  ```

## Build Automation (xtask)

This project uses the **xtask pattern** for build automation. Instead of shell scripts, we write automation tasks in Rust for cross-platform compatibility and type safety.

### Available Tasks

```bash
# Install codebuddy
cargo xtask install

# Run all checks (fmt, clippy, test, deny)
cargo xtask check-all

# Check for duplicate code
cargo xtask check-duplicates

# Check cargo features
cargo xtask check-features

# Create new language plugin
cargo xtask new-lang <language>

# See all available commands
cargo xtask --help
```

### Why xtask?

- ✅ **Cross-platform**: Works on Windows, Linux, and macOS natively
- ✅ **Type-safe**: Full Rust IDE support with compile-time checking
- ✅ **Integrated**: Uses Rust ecosystem (cargo API, file operations)
- ✅ **Better error handling**: Result<T, E> instead of exit codes
- ✅ **Maintainable**: Easier to test and debug than shell scripts

### Adding New Tasks

1. Add a new module in `crates/xtask/src/<task>.rs`
2. Define the command structure:
   ```rust
   use anyhow::Result;
   use clap::Args;

   #[derive(Args)]
   pub struct YourTaskArgs {
       #[arg(long)]
       some_option: Option<String>,
   }

   pub fn run(args: YourTaskArgs) -> Result<()> {
       // Your implementation
       Ok(())
   }
   ```
3. Add the command to `crates/xtask/src/main.rs`:
   ```rust
   #[derive(Subcommand)]
   enum Command {
       // ... existing commands
       YourTask(your_task::YourTaskArgs),
   }

   // In main()
   match cli.command {
       // ... existing matches
       Command::YourTask(args) => your_task::run(args),
   }
   ```
4. Test your command: `cargo xtask your-task`

### Integration with Makefile

The Makefile uses a hybrid approach:
- **Simple commands** stay in Makefile (build, test, fmt)
- **Complex tasks** delegate to xtask (install, check-duplicates)

This provides convenience (`make install`) with cross-platform reliability (xtask handles the actual work).

## Dependency Management

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
   # Or use Makefile
   make deny
   ```

### Running Dependency Checks

```bash
# Check all: advisories, licenses, bans, sources
cargo deny check
make deny

# Check only security advisories
cargo deny check advisories

# Check only licenses
cargo deny check licenses

# Check only duplicate dependencies
cargo deny check bans

# Update advisory database
cargo deny fetch
make deny-update
```

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

If an exception is truly needed, update `deny.toml` with a clear justification comment.

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
```

## Pull Request Process

1.  **Create a Feature Branch:**
    ```bash
    git checkout -b your-feature-name
    ```

2.  **Commit Your Changes:** Make your changes and commit them with a descriptive message.
    ```bash
    git commit -m "feat: Add new feature" -m "Detailed description of the changes."
    ```

3.  **Ensure Tests Pass:** Run the tests one last time to make sure everything is working correctly.
    ```bash
    make test
    ```

4.  **Push to Your Branch:**
    ```bash
    git push origin your-feature-name
    ```

5.  **Open a Pull Request:** Go to the repository on GitHub and open a new pull request. Provide a clear title and description of your changes.

## Adding New Language Plugins

To add support for a new programming language, see the **[Language Plugins Guide](docs/development/plugin_development.md)** which provides:

- Complete plugin structure and schema requirements
- Required trait implementations (`LanguagePlugin`)
- Data types (ParsedSource, Symbol, ManifestData)
- Plugin registration steps
- Implementation patterns (dual-mode vs pure Rust)
- Reference implementations (Rust, Go, TypeScript)

### Implementing Capability Traits

The codebase uses a **capability-based dispatch pattern** where language plugins expose optional capabilities via traits. This enables language-agnostic shared code that works with any plugin without compile-time feature flags.

#### Why Capabilities?

**Before (downcasting + cfg guards):**
```rust
// ❌ Tightly coupled to specific plugin types
#[cfg(feature = "lang-rust")]
if let Some(rust_plugin) = plugin.as_any().downcast_ref::<RustPlugin>() {
    rust_plugin.update_manifest()?;
}
#[cfg(feature = "lang-typescript")]
if let Some(ts_plugin) = plugin.as_any().downcast_ref::<TypeScriptPlugin>() {
    ts_plugin.update_manifest()?;
}
```

**After (capability traits):**
```rust
// ✅ Language-agnostic, works with any plugin
if let Some(updater) = plugin.manifest_updater() {
    updater.update_dependency(...).await?;
}
```

**Benefits:**
- **Zero cfg guards** in shared code
- **Plug-and-play** language support
- **Type-safe** trait contracts
- **File-extension routing** selects correct plugin automatically

#### Available Capability Traits

Located in `crates/mill-plugin-api/src/capabilities.rs`:

**1. ManifestUpdater** - Update package manifests (Cargo.toml, package.json)
```rust
#[async_trait]
pub trait ManifestUpdater: Send + Sync {
    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String>;
}
```

**2. ModuleLocator** - Locate module files within packages
```rust
#[async_trait]
pub trait ModuleLocator: Send + Sync {
    async fn locate_module_files(
        &self,
        package_path: &Path,
        module_path: &str,
    ) -> PluginResult<Vec<PathBuf>>;
}
```

**3. RefactoringProvider** - AST refactoring operations
```rust
#[async_trait]
pub trait RefactoringProvider: Send + Sync {
    fn supports_inline_variable(&self) -> bool;
    fn supports_extract_function(&self) -> bool;
    fn supports_extract_variable(&self) -> bool;

    async fn plan_inline_variable(
        &self,
        file_path: &str,
        start_line: usize,
        start_col: usize,
    ) -> PluginResult<EditPlan>;

    async fn plan_extract_function(
        &self,
        file_path: &str,
        start_line: usize,
        end_line: usize,
        function_name: &str,
    ) -> PluginResult<EditPlan>;

    async fn plan_extract_variable(
        &self,
        file_path: &str,
        start_line: usize,
        start_col: usize,
        var_name: &str,
    ) -> PluginResult<EditPlan>;
}
```

#### Step-by-Step Implementation

**Step 1: Implement the capability trait on your plugin**

```rust
// Example: Implementing ManifestUpdater for a Python plugin
use async_trait::async_trait;
use cb_plugin_api::capabilities::ManifestUpdater;
use cb_plugin_api::{PluginResult, PluginError};

pub struct PythonPlugin {
    // Plugin fields
}

#[async_trait]
impl ManifestUpdater for PythonPlugin {
    async fn update_dependency(
        &self,
        manifest_path: &Path,
        old_name: &str,
        new_name: &str,
        new_version: Option<&str>,
    ) -> PluginResult<String> {
        // Read pyproject.toml or requirements.txt
        let content = tokio::fs::read_to_string(manifest_path).await?;

        // Parse and update dependencies
        let updated = if manifest_path.ends_with("pyproject.toml") {
            self.update_pyproject_toml(&content, old_name, new_name, new_version)?
        } else {
            self.update_requirements_txt(&content, old_name, new_name, new_version)?
        };

        // Write back
        tokio::fs::write(manifest_path, &updated).await?;

        Ok(updated)
    }
}
```

**Step 2: Expose the capability via LanguagePlugin trait**

```rust
impl LanguagePlugin for PythonPlugin {
    fn name(&self) -> &str {
        "python"
    }

    fn file_extensions(&self) -> Vec<String> {
        vec!["py".to_string(), "pyi".to_string()]
    }

    // Expose the capability
    fn manifest_updater(&self) -> Option<&dyn ManifestUpdater> {
        Some(self)  // Returns &PythonPlugin as &dyn ManifestUpdater
    }

    // Other capabilities this plugin doesn't support
    fn module_locator(&self) -> Option<&dyn ModuleLocator> {
        None  // Python plugin doesn't implement this yet
    }

    fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> {
        Some(self)  // If you also implement RefactoringProvider
    }
}
```

**Step 3: Shared code automatically discovers and uses the capability**

```rust
// In shared workspace.rs - no knowledge of PythonPlugin!
pub async fn update_dependency(
    plugins: &PluginRegistry,
    manifest_path: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<()> {
    // Get file extension from manifest path
    let extension = manifest_path.extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| anyhow!("Invalid manifest path"))?;

    // Find plugin by extension (returns PythonPlugin for .toml if pyproject.toml)
    let plugin = plugins.find_by_extension(extension)
        .ok_or_else(|| anyhow!("No plugin for .{} files", extension))?;

    // Query for capability
    let updater = plugin.manifest_updater()
        .ok_or_else(|| anyhow!("Plugin does not support manifest updates"))?;

    // Use the capability - works for ANY plugin that implements it!
    updater.update_dependency(manifest_path, old_name, new_name, None).await?;

    Ok(())
}
```

#### Implementing Multiple Capabilities

A plugin can implement multiple capabilities:

```rust
pub struct RustPlugin {
    // fields...
}

// Implement all three capabilities
#[async_trait]
impl ManifestUpdater for RustPlugin {
    async fn update_dependency(&self, ...) -> PluginResult<String> {
        // Cargo.toml update logic
    }
}

#[async_trait]
impl ModuleLocator for RustPlugin {
    async fn locate_module_files(&self, ...) -> PluginResult<Vec<PathBuf>> {
        // Rust module resolution logic
    }
}

#[async_trait]
impl RefactoringProvider for RustPlugin {
    fn supports_inline_variable(&self) -> bool { true }
    fn supports_extract_function(&self) -> bool { true }
    fn supports_extract_variable(&self) -> bool { true }

    async fn plan_inline_variable(&self, ...) -> PluginResult<EditPlan> {
        // Rust AST-based refactoring
    }
    // ... other refactoring methods
}

// Expose all capabilities
impl LanguagePlugin for RustPlugin {
    fn manifest_updater(&self) -> Option<&dyn ManifestUpdater> { Some(self) }
    fn module_locator(&self) -> Option<&dyn ModuleLocator> { Some(self) }
    fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> { Some(self) }
}
```

#### File-Extension-Based Routing

The plugin registry automatically routes to the correct plugin based on file extension:

```rust
// In refactoring code (extract_function.rs)
pub async fn extract_function(
    plugins: &PluginRegistry,
    file_path: &str,
    params: ExtractParams,
) -> AstResult<EditPlan> {
    // Automatically routes to correct plugin based on .rs, .ts, .py extension
    let provider = plugins.refactoring_provider_for_file(file_path)
        .ok_or_else(|| AstError::UnsupportedLanguage(file_path.to_string()))?;

    // Execute refactoring using the correct language plugin
    provider.plan_extract_function(
        file_path,
        params.start_line,
        params.end_line,
        &params.function_name,
    ).await
}
```

**Under the hood:**
1. Extract file extension from path ("rs", "ts", "py")
2. Find plugin registered for that extension
3. Query that specific plugin for the capability
4. Return `Some(&dyn Trait)` or `None`

#### Testing Capabilities

Write tests to verify capability routing and behavior:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_python_plugin_capabilities() {
        let plugin = PythonPlugin::new();

        // Verify capability exposure
        assert!(plugin.manifest_updater().is_some());
        assert!(plugin.refactoring_provider().is_some());
        assert!(plugin.module_locator().is_none());

        // Test manifest update capability
        let updater = plugin.manifest_updater().unwrap();
        let result = updater.update_dependency(
            Path::new("pyproject.toml"),
            "old-package",
            "new-package",
            Some("1.2.3"),
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_registry_routing() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(PythonPlugin::new()));

        // Test file-extension routing
        let provider = registry.refactoring_provider_for_file("script.py");
        assert!(provider.is_some());

        let provider = registry.refactoring_provider_for_file("unknown.xyz");
        assert!(provider.is_none());
    }
}
```

#### Best Practices

1. **Return None for unsupported capabilities** - Don't panic or return errors
   ```rust
   fn module_locator(&self) -> Option<&dyn ModuleLocator> {
       None  // This plugin doesn't support module location
   }
   ```

2. **Implement capabilities incrementally** - Start with one capability, add more over time
   ```rust
   // v1: Just manifest updates
   fn manifest_updater(&self) -> Option<&dyn ManifestUpdater> { Some(self) }
   fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> { None }

   // v2: Add refactoring later
   fn refactoring_provider(&self) -> Option<&dyn RefactoringProvider> { Some(self) }
   ```

3. **Use capability checks for feature detection**
   ```rust
   // Check if refactoring is available before offering it to users
   if let Some(provider) = plugin.refactoring_provider() {
       if provider.supports_extract_function() {
           // Show "Extract Function" option in UI
       }
   }
   ```

4. **Keep capability traits focused** - Each trait should represent a single cohesive capability

5. **Document capability support** - Update your plugin's README with supported capabilities

For complete examples, see:
- **[crates/mill-lang-rust/src/lib.rs](../crates/mill-lang-rust/src/lib.rs)** - Full capability implementation
- **[crates/mill-lang-typescript/src/lib.rs](../crates/mill-lang-typescript/src/lib.rs)** - Partial capability support
- **[crates/mill-plugin-api/src/capabilities.rs](../crates/mill-plugin-api/src/capabilities.rs)** - Capability trait definitions

## Adding New MCP Tools

This section explains how to add new tools and handlers to the system.

### Understanding the Unified Refactoring API

Codebuddy uses a **unified refactoring API** with a consistent `plan -> apply` pattern for all code refactorings. This architecture provides:

1. **Safety**: All `.plan` commands are read-only and never modify files
2. **Preview**: Users can inspect changes before applying them
3. **Atomicity**: `workspace.apply_edit` applies all changes atomically with automatic rollback on failure
4. **Consistency**: All refactoring operations follow the same pattern

**Current Refactoring Tools:**

| Tool | Purpose | Returns |
|------|---------|---------|
| `rename.plan` | Plan symbol/file/directory rename | `RenamePlan` |
| `extract.plan` | Plan extract function/variable/constant | `ExtractPlan` |
| `inline.plan` | Plan inline variable/function | `InlinePlan` |
| `move.plan` | Plan move symbol to another file | `MovePlan` |
| `reorder.plan` | Plan reorder parameters/imports | `ReorderPlan` |
| `transform.plan` | Plan transform (e.g., to async) | `TransformPlan` |
| `delete.plan` | Plan delete unused code/imports | `DeletePlan` |
| `workspace.apply_edit` | Execute any plan | Execution result |

**Example Flow:**
```bash
# Step 1: Generate a plan (read-only, safe to explore)
PLAN=$(codebuddy tool rename.plan '{
  "target": {"kind": "symbol", "path": "src/app.ts", "selector": {"position": {"line": 15, "character": 8}}},
  "newName": "newUser"
}')

# Step 2: Inspect the plan (it contains edits, summary, warnings)
echo $PLAN | jq .

# Step 3: Apply the plan (atomic, with rollback on failure)
codebuddy tool workspace.apply_edit "{\"plan\": $PLAN}"
```

**Internal Tools (Hidden from MCP):**
- Legacy tools like `rename_symbol_with_imports` are now internal and hidden from MCP `tools/list`
- These are used by the workflow system but not exposed to AI agents
- See [docs/architecture/INTERNAL_TOOLS.md](../docs/architecture/INTERNAL_TOOLS.md) for details

### Understanding the Unified Analysis API

Codebuddy uses a **unified analysis API** with a consistent `analyze.<category>(kind, scope, options) → AnalysisResult` pattern for all code analysis. This architecture provides:

1. **Consistency**: All analysis operations follow the same pattern and return the same result structure
2. **Composability**: Batch analysis with shared AST parsing for performance
3. **Read-only**: Analysis operations never modify files
4. **Actionable**: Results include suggestions linking directly to refactoring operations

**Current Analysis Tools:**

| Tool | Categories (kinds) | Returns |
|------|-------------------|---------|
| `analyze.quality` | complexity, smells, maintainability, readability | `AnalysisResult` |
| `analyze.dead_code` | unused_imports, unused_symbols, unreachable_code, unused_parameters, unused_types, unused_variables | `AnalysisResult` |
| `analyze.dependencies` | imports, graph, circular, coupling, cohesion, depth | `AnalysisResult` |
| `analyze.structure` | symbols, hierarchy, interfaces, inheritance, modules | `AnalysisResult` |
| `analyze.documentation` | coverage, quality, style, examples, todos | `AnalysisResult` |
| `analyze.tests` | coverage, quality, assertions, organization | `AnalysisResult` |
| `analyze.batch` | Multi-file analysis with optimized AST caching | `BatchAnalysisResult` |

**Example Flow:**
```bash
# Analyze code quality
codebuddy tool analyze.quality '{
  "kind": "complexity",
  "scope": {"type": "file", "path": "src/app.ts"}
}'

# Returns:
{
  "findings": [
    {
      "id": "complexity-1",
      "kind": "complexity_hotspot",
      "severity": "high",
      "location": {...},
      "metrics": {"cyclomatic_complexity": 25, ...},
      "message": "Function has high complexity",
      "suggestions": [
        {
          "action": "extract_function",
          "description": "Extract nested block",
          "refactor_call": {
            "command": "extract.plan",
            "arguments": {...}
          }
        }
      ]
    }
  ],
  "summary": {...},
  "metadata": {...}
}
```

**Analysis Handler Architecture:**

Analysis handlers are organized by category in `crates/mill-handlers/src/handlers/tools/analysis/`:

- `quality.rs` - Code quality analysis (4 kinds)
- `dead_code.rs` - Unused code detection (6 kinds)
- `dependencies.rs` - Dependency analysis (6 kinds)
- `structure.rs` - Code structure (5 kinds)
- `documentation.rs` - Documentation quality (5 kinds)
- `tests_handler.rs` - Test analysis (4 kinds)
- `batch.rs` - Batch analysis infrastructure with AST caching
- `config.rs` - Configuration loading (.codebuddy/analysis.toml)

**Adding a new detection kind:**

1. Add the kind to the appropriate category handler (e.g., `quality.rs`)
2. Implement a detection function with signature: `(complexity_report, content, symbols, language, file_path) -> Vec<Finding>`
3. Register the function in `batch.rs` helpers
4. Add test cases in `tests/e2e/src/test_analyze_<category>.rs`

See [40_PROPOSAL_UNIFIED_ANALYSIS_API.md](../40_PROPOSAL_UNIFIED_ANALYSIS_API.md) for complete architecture details.

### Adding a Tool to an Existing Handler

Adding a new tool to an existing handler requires modifying just one file.

#### Step 1: Choose the Appropriate Handler

Handlers are organized by functionality:

| Handler | Location | Purpose | Example Tools |
|---------|----------|---------|---------------|
| **AnalysisHandler** | `crates/mill-handlers/src/handlers/tools/analysis/*.rs` | Unified Analysis API (7 categories) | `analyze.quality`, `analyze.dead_code`, `analyze.dependencies`, `analyze.structure`, `analyze.documentation`, `analyze.tests`, `analyze.batch` |
| **AdvancedHandler** | `crates/mill-handlers/src/handlers/tools/advanced.rs` | Advanced operations | `apply_edits`, `batch_execute` |
| **FileOpsHandler** | `crates/mill-handlers/src/handlers/tools/file_ops.rs` | File operations | `create_file`, `read_file`, `write_file`, `delete_file`, `rename_file`, `list_files` |
| **InternalEditingHandler** | `crates/mill-handlers/src/handlers/tools/internal_editing.rs` | Internal editing tools (hidden from MCP) | `format_document`, `optimize_imports` |
| **InternalWorkspaceHandler** | `crates/mill-handlers/src/handlers/tools/internal_workspace.rs` | Internal workspace tools (hidden from MCP) | `rename_symbol_with_imports`, `apply_workspace_edit` |
| **LifecycleHandler** | `crates/mill-handlers/src/handlers/tools/lifecycle.rs` | File lifecycle events | `notify_file_opened`, `notify_file_saved`, `notify_file_closed` |
| **NavigationHandler** | `crates/mill-handlers/src/handlers/tools/navigation.rs` | Code navigation | `find_definition`, `find_references` |
| **RenameHandler** | `crates/mill-handlers/src/handlers/rename_handler.rs` | Rename refactoring (plan step) | `rename.plan` |
| **ExtractHandler** | `crates/mill-handlers/src/handlers/extract_handler.rs` | Extract refactoring (plan step) | `extract.plan` |
| **InlineHandler** | `crates/mill-handlers/src/handlers/inline_handler.rs` | Inline refactoring (plan step) | `inline.plan` |
| **MoveHandler** | `crates/mill-handlers/src/handlers/move_handler.rs` | Move refactoring (plan step) | `move.plan` |
| **ReorderHandler** | `crates/mill-handlers/src/handlers/reorder_handler.rs` | Reorder refactoring (plan step) | `reorder.plan` |
| **TransformHandler** | `crates/mill-handlers/src/handlers/transform_handler.rs` | Transform refactoring (plan step) | `transform.plan` |
| **DeleteHandler** | `crates/mill-handlers/src/handlers/delete_handler.rs` | Delete refactoring (plan step) | `delete.plan` |
| **WorkspaceApplyHandler** | `crates/mill-handlers/src/handlers/workspace_apply_handler.rs` | Apply refactoring plans | `workspace.apply_edit` |
| **SystemHandler** | `crates/mill-handlers/src/handlers/tools/system.rs` | System operations | `health_check`, `web_fetch`, `system_status` |
| **WorkspaceHandler** | `crates/mill-handlers/src/handlers/tools/workspace.rs` | Workspace operations | `rename_directory`, `analyze.dependencies`, `analyze.dead_code` |

#### Step 2: Add the Tool Name

Open the appropriate handler file and add your tool name to the `TOOL_NAMES` constant:

```rust
// Example: Adding a tool to NavigationHandler
// crates/mill-handlers/src/handlers/tools/navigation.rs

const TOOL_NAMES: &[&str] = &[
    "find_definition",
    "find_references",
    "get_call_graph", // ← Add your new tool here
];
```

**For refactoring tools**, handlers are in `crates/mill-handlers/src/handlers/` (not in `tools/` subdirectory):

```rust
// Example: RenameHandler
// crates/mill-handlers/src/handlers/rename_handler.rs

const TOOL_NAMES: &[&str] = &[
    "rename.plan",  // Note: Only the .plan command
];
```

#### Step 3: Implement the Handler Logic

Add a new match arm in the `handle_tool_call` method:

```rust
async fn handle_tool_call(
    &self,
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    match tool_call.name.as_str() {
        "find_definition" => self.find_definition(context, tool_call).await,
        "get_call_graph" => self.get_call_graph(context, tool_call).await, // ← Add match arm
        _ => Err(ServerError::Unsupported(format!(
            "Unsupported navigation tool: {}",
            tool_call.name
        ))),
    }
}
```

**For refactoring handlers**, the pattern is similar but returns a Plan object:

```rust
// Example from RenameHandler
async fn handle_tool_call(
    &self,
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    match tool_call.name.as_str() {
        "rename.plan" => self.handle_rename_plan(context, tool_call).await,
        _ => Err(ServerError::Unsupported(format!(
            "Unsupported rename tool: {}",
            tool_call.name
        ))),
    }
}
```

#### Step 4: Implement the Tool Method

Add the implementation as a private method:

**For standard tools** (navigation, analysis, file ops):

```rust
impl NavigationHandler {
    /// Get the call graph for a function
    async fn get_call_graph(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Extract parameters
        let args = tool_call.arguments.as_ref()
            .ok_or_else(|| ServerError::InvalidRequest("Missing arguments".to_string()))?;

        let file_path = args["file_path"]
            .as_str()
            .ok_or_else(|| ServerError::InvalidRequest("Missing file_path".to_string()))?;

        // Dispatch to plugin system
        let plugin_request = PluginRequest {
            method: "get_call_graph".to_string(),
            file_path: file_path.to_string(),
            params: json!({ /* parameters */ }),
            request_id: None,
        };

        match context.plugin_manager.handle_request(plugin_request).await {
            Ok(response) => Ok(json!({
                "content": response.data,
                "metadata": response.metadata
            })),
            Err(e) => Err(ServerError::Internal(format!("Plugin error: {}", e))),
        }
    }
}
```

**For refactoring plan handlers**, return a Plan structure:

```rust
impl RenameHandler {
    /// Generate a rename plan (read-only, never modifies files)
    async fn handle_rename_plan(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // 1. Parse and validate parameters
        let params: RenameParams = serde_json::from_value(
            tool_call.arguments.clone().unwrap_or_default()
        )?;

        // 2. Use LSP or plugin to generate workspace edits
        let workspace_edit = match params.target.kind {
            RenameKind::Symbol => {
                // Use LSP textDocument/rename
                context.lsp_client.rename(/* ... */).await?
            },
            RenameKind::File | RenameKind::Directory => {
                // Use file service + plugin system for import updates
                /* ... */
            }
        };

        // 3. Calculate file checksums for validation
        let checksums = calculate_checksums(&workspace_edit)?;

        // 4. Return a RenamePlan (never modifies files)
        Ok(json!({
            "plan_type": "RenamePlan",
            "edits": workspace_edit,
            "summary": {
                "files_affected": affected_files.len(),
                "created_files": created.len(),
                "deleted_files": deleted.len(),
            },
            "file_checksums": checksums,
            "warnings": warnings,
        }))
    }
}
```

### Creating a New Handler

Create a new handler when adding a category of related tools that doesn't fit existing handlers.

#### Handler Types

There are two main types of handlers:

1. **Standard Tool Handlers** (in `crates/mill-handlers/src/handlers/tools/`):
   - Navigation, analysis, file operations, etc.
   - Return immediate results
   - Example: `NavigationHandler`, `AnalysisHandler`

2. **Refactoring Plan Handlers** (in `crates/mill-handlers/src/handlers/`):
   - Part of the unified refactoring API
   - Generate read-only plans that must be applied with `workspace.apply_edit`
   - Example: `RenameHandler`, `ExtractHandler`, `InlineHandler`

#### Step 1: Create the Handler File

**For standard tools:**
```bash
touch crates/mill-handlers/src/handlers/tools/diagnostics.rs
```

**For refactoring tools:**
```bash
touch crates/mill-handlers/src/handlers/my_refactoring_handler.rs
```

#### Step 2: Define the Handler Struct

**For standard tools:**

```rust
//! Diagnostic tools for code quality and analysis

use super::{ToolHandler, ToolHandlerContext};
use crate::{ServerError, ServerResult};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::{json, Value};
use tracing::{debug, error};

/// Handler for diagnostic tools
pub struct DiagnosticsHandler;

const TOOL_NAMES: &[&str] = &[
    "get_diagnostics",
    "get_code_quality_metrics",
];

impl DiagnosticsHandler {
    pub fn new() -> Self {
        Self
    }
}
```

**For refactoring plan handlers:**

```rust
//! My refactoring operation handler (plan step)

use crate::handlers::ToolHandler;
use crate::{ServerError, ServerResult, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use serde_json::Value;
use tracing::{debug, error};

/// Handler for my_refactoring.plan
pub struct MyRefactoringHandler;

const TOOL_NAMES: &[&str] = &["my_refactoring.plan"];

impl MyRefactoringHandler {
    pub fn new() -> Self {
        Self
    }
}
```

#### Step 3: Implement the ToolHandler Trait

```rust
#[async_trait]
impl ToolHandler for DiagnosticsHandler {
    fn tool_names(&self) -> &[&str] {
        TOOL_NAMES
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "get_diagnostics" => self.get_diagnostics(context, tool_call).await,
            "get_code_quality_metrics" => self.get_code_quality_metrics(context, tool_call).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unsupported diagnostic tool: {}",
                tool_call.name
            ))),
        }
    }
}
```

#### Step 4: Register the Handler

Add to `crates/mill-handlers/src/handlers/tools/mod.rs`:

```rust
pub mod diagnostics;
pub use diagnostics::DiagnosticsHandler;
```

Add to the dispatcher in `crates/mill-handlers/src/handlers/plugin_dispatcher.rs`:

```rust
register_handlers_with_logging!(registry, {
    SystemHandler => "SystemHandler with 3 tools...",
    DiagnosticsHandler => "DiagnosticsHandler with 2 tools...", // ← Add this
});
```

### Best Practices

#### Naming Conventions
- **Tool names**: snake_case (e.g., `get_diagnostics`)
- **Refactoring plan tools**: `<operation>.plan` (e.g., `rename.plan`, `extract.plan`)
- **Handler names**: PascalCase with "Handler" suffix (e.g., `DiagnosticsHandler`, `RenameHandler`)
- **File names**: snake_case matching handler (e.g., `diagnostics.rs`, `rename_handler.rs`)

#### Refactoring Plan Structure

All refactoring `.plan` handlers must return a consistent plan structure:

```rust
// Required fields in all plan responses
{
    "plan_type": "RenamePlan",  // One of: RenamePlan, ExtractPlan, InlinePlan, MovePlan, etc.
    "edits": [/* LSP WorkspaceEdit array */],
    "summary": {
        "files_affected": 3,
        "created_files": 0,
        "deleted_files": 0
    },
    "file_checksums": {
        "src/app.ts": "sha256_hash",
        "src/utils.ts": "sha256_hash"
    },
    "warnings": ["Optional warning messages"],
    // Optional plan-specific fields
}
```

**Key principles:**
- **Read-only**: `.plan` commands must NEVER modify files
- **Idempotent**: Multiple calls with same params should produce same plan
- **Checksums**: Always include file checksums for validation
- **Summary**: Provide clear summary of what will change
- **Warnings**: Include any potential issues detected during planning

#### Structured Logging
Always use structured key-value logging (see [docs/development/LOGGING_GUIDELINES.md](logging_guidelines.md)):

```rust
// ✅ Good - structured logging
debug!(tool_name = %tool_call.name, file_path = %path, "Processing tool call");
error!(error = %e, tool = "get_diagnostics", "Tool execution failed");

// ❌ Bad - string interpolation
debug!("Processing tool call {} for file {}", tool_call.name, path);
```

#### Error Handling
Provide clear, actionable error messages:

```rust
// ✅ Good
let file_path = args["file_path"]
    .as_str()
    .ok_or_else(|| ServerError::InvalidRequest(
        "Missing required parameter 'file_path'"
    ))?;

// ❌ Bad
let file_path = args["file_path"].as_str().unwrap();
```

#### Documentation
Add doc comments explaining purpose, parameters, and return values:

```rust
/// Get diagnostic information for a file
///
/// # Arguments
///
/// * `context` - Handler context with access to services
/// * `tool_call` - The tool call with file_path parameter
///
/// # Returns
///
/// Returns diagnostic messages, or an error if the file cannot be analyzed.
async fn get_diagnostics(...) -> ServerResult<Value> {
    // ...
}
```

#### Testing
Add tests for your tools (see [tests/e2e/TESTING_GUIDE.md](tests/e2e/TESTING_GUIDE.md)):

**Standard tool tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_diagnostics() {
        let context = create_test_context().await;
        let handler = DiagnosticsHandler::new();

        let tool_call = ToolCall {
            name: "get_diagnostics".to_string(),
            arguments: Some(json!({"file_path": "test.ts"})),
        };

        let result = handler.handle_tool_call(&context, &tool_call).await;
        assert!(result.is_ok());
    }
}
```

**Refactoring plan handler tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rename_plan_generates_valid_plan() {
        let context = create_test_context().await;
        let handler = RenameHandler::new();

        let tool_call = ToolCall {
            name: "rename.plan".to_string(),
            arguments: Some(json!({
                "target": {
                    "kind": "symbol",
                    "path": "src/test.ts",
                    "selector": {"position": {"line": 10, "character": 5}}
                },
                "newName": "newName"
            })),
        };

        let result = handler.handle_tool_call(&context, &tool_call).await;
        assert!(result.is_ok());

        // Verify plan structure
        let plan = result.unwrap();
        assert_eq!(plan["plan_type"], "RenamePlan");
        assert!(plan["edits"].is_array());
        assert!(plan["summary"].is_object());
        assert!(plan["file_checksums"].is_object());

        // Verify read-only: Plan should not modify any files
        // (Test by checking file timestamps or content before/after)
    }
}
```

**Integration tests for unified refactoring API:**
- Test in `tests/e2e/src/test_<operation>_integration.rs`
- Cover both plan generation and application via `workspace.apply_edit`
- Test rollback behavior on errors
- Test checksum validation

## Build Performance Tips

### Optimization Tools (Configured Automatically)

The project uses several build optimizations configured in `.cargo/config.toml`:

- **sccache**: Compilation cache that dramatically speeds up rebuilds
- **mold**: Modern, fast linker (3-10x faster than traditional linkers)
- **Dependency optimization**: Dependencies compiled with `-O2` in dev mode

### Quick Commands

```bash
# Check sccache statistics
sccache --show-stats

# Clear sccache (if having cache issues)
sccache --zero-stats

# Fast feedback during development (doesn't build binaries)
cargo check

# Build only changed code (fastest)
cargo build

# Full rebuild (slow, use only when necessary)
cargo clean && cargo build
```

### Build Times Reference

With sccache and mold installed:

| Build Type | Time (First) | Time (Incremental) |
|------------|--------------|-------------------|
| `cargo check` | ~30s | 2-5s |
| `cargo build` | ~2m | 5-20s |
| `cargo build --release` | ~3m | 30-60s |
| `cargo nextest run` (`make test`) | ~2m | 8-25s |

**Note:** Times vary based on:
- CPU cores (6+ cores recommended)
- SSD vs HDD (SSD strongly recommended)
- Changes scope (few files vs many files)

### Troubleshooting Slow Builds

If builds are slower than expected:

1. **Verify sccache is working:**
   ```bash
   sccache --show-stats
   # Should show cache hits on second build
   ```

2. **Check mold is being used:**
   ```bash
   grep -r "fuse-ld=mold" .cargo/config.toml
   # Should show linker configuration
   ```

3. **Monitor build parallelism:**
   ```bash
   # Check CPU usage during builds
   # Should use 80-100% of all cores
   ```

4. **Clear cache if corrupted:**
   ```bash
   sccache --zero-stats
   rm -rf target/
   cargo build
   ```

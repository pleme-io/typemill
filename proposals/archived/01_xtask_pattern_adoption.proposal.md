# Proposal 08: Adopt xtask Pattern for Build Automation

**Status:** Draft
**Created:** 2025-10-13
**Author:** AI Assistant
**Tracking Issue:** TBD

## Summary

Introduce the `xtask` pattern to replace complex shell scripts with type-safe Rust automation tasks, improving cross-platform compatibility and maintainability.

## Motivation

**Current Issues:**
- 5 shell scripts with varying complexity (50-200+ lines)
- Windows compatibility concerns (bash scripts)
- Error handling is difficult in shell scripts
- No type safety or IDE support for build automation
- Duplicated logic across scripts

**Scripts to replace:**
```bash
scripts/
├── install.sh              # 200+ lines - Complex installation logic
├── check-duplicates.sh     # Runs jscpd for duplicate code detection
├── check-features.sh       # Validates cargo features
├── dotnet-install.sh       # Installs .NET SDK for C# plugin (legacy)
└── new-lang.sh            # Scaffolds new language plugin
```

**Goals:**
1. Type-safe build automation in Rust
2. Cross-platform compatibility (Windows, Linux, macOS)
3. Better error handling and user feedback
4. Leverage Rust ecosystem (cargo API, file operations)
5. Consistent experience with rest of codebase

## Background: What is `xtask`?

The `xtask` pattern is a Rust convention where you create a separate binary crate for project automation tasks.

**Instead of:**
```bash
./scripts/install.sh
./scripts/check-duplicates.sh
```

**You write:**
```bash
cargo xtask install
cargo xtask check-duplicates
```

**Used by:**
- [rust-analyzer](https://github.com/rust-lang/rust-analyzer/tree/master/xtask)
- [cargo](https://github.com/rust-lang/cargo/tree/master/xtask) (self-hosting!)
- [tokio](https://github.com/tokio-rs/tokio/tree/master/xtask)
- [serde](https://github.com/serde-rs/serde/tree/master/xtask)

## Detailed Design

### 1. Create `xtask` Crate

```bash
mkdir -p crates/xtask
```

**`crates/xtask/Cargo.toml`:**
```toml
[package]
name = "xtask"
version = "0.1.0"
edition = "2021"
publish = false  # Never publish xtask crate

[dependencies]
# Command-line parsing
clap = { version = "4", features = ["derive"] }

# File operations
walkdir = "2"
ignore = "0.4"  # Respects .gitignore

# Process spawning
duct = "0.13"  # Better than std::process

# Error handling
anyhow = "1"
thiserror = "2"

# Serialization for config reading
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Colored output
colored = "2"

# Path utilities
pathdiff = "0.2"
```

**`crates/xtask/src/main.rs`:**
```rust
use anyhow::Result;
use clap::{Parser, Subcommand};

mod install;
mod check_duplicates;
mod check_features;
mod new_lang;
mod utils;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "TypeMill build automation tasks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Install mill and set up development environment
    Install(install::InstallArgs),

    /// Check for duplicate code in the codebase
    CheckDuplicates(check_duplicates::Args),

    /// Check cargo feature configurations
    CheckFeatures(check_features::Args),

    /// Create a new language plugin scaffold
    NewLang(new_lang::Args),

    /// Run all checks (fmt, clippy, test, deny)
    CheckAll,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Install(args) => install::run(args),
        Command::CheckDuplicates(args) => check_duplicates::run(args),
        Command::CheckFeatures(args) => check_features::run(args),
        Command::NewLang(args) => new_lang::run(args),
        Command::CheckAll => run_all_checks(),
    }
}

fn run_all_checks() -> Result<()> {
    println!("Running all checks...\n");

    utils::run_cmd("cargo", &["fmt", "--check"])?;
    println!("✓ Format check passed\n");

    utils::run_cmd("cargo", &["clippy", "--all-targets", "--all-features"])?;
    println!("✓ Clippy passed\n");

    utils::run_cmd("cargo", &["nextest", "run", "--workspace"])?;
    println!("✓ Tests passed\n");

    utils::run_cmd("cargo", &["deny", "check"])?;
    println!("✓ Dependency audit passed\n");

    println!("✓ All checks passed!");
    Ok(())
}
```

---

### 2. Implement Each Task

#### 2.1: Install Task

**`crates/xtask/src/install.rs`:**
```rust
use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct InstallArgs {
    /// Install to a custom directory
    #[arg(long)]
    dest: Option<PathBuf>,

    /// Skip building, just link
    #[arg(long)]
    skip_build: bool,

    /// Development install (don't optimize)
    #[arg(long)]
    dev: bool,
}

pub fn run(args: InstallArgs) -> Result<()> {
    println!("Installing mill...\n");

    // Build
    if !args.skip_build {
        let profile = if args.dev { "dev" } else { "release" };
        println!("Building in {} mode...", profile);

        let mut cmd = vec!["build", "-p", "mill"];
        if !args.dev {
            cmd.push("--release");
        }

        crate::utils::run_cmd("cargo", &cmd)?;
    }

    // Determine binary location
    let profile_dir = if args.dev { "debug" } else { "release" };
    let binary_name = if cfg!(windows) { "mill.exe" } else { "mill" };
    let binary_path = PathBuf::from("target")
        .join(profile_dir)
        .join(binary_name);

    if !binary_path.exists() {
        anyhow::bail!("Binary not found at {:?}", binary_path);
    }

    // Install
    let dest = args.dest.unwrap_or_else(default_install_dir);
    std::fs::create_dir_all(&dest)
        .context("Failed to create install directory")?;

    let dest_binary = dest.join(binary_name);
    std::fs::copy(&binary_path, &dest_binary)
        .context("Failed to copy binary")?;

    println!("✓ Installed to: {}", dest_binary.display());

    // Check if in PATH
    if !is_in_path(&dest) {
        println!("\n⚠️  Install directory not in PATH");
        println!("Add to your shell profile:");
        println!("  export PATH=\"{}:$PATH\"", dest.display());
    }

    Ok(())
}

fn default_install_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local/bin")
    } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
        PathBuf::from(userprofile).join(".cargo").join("bin")
    } else {
        PathBuf::from("/usr/local/bin")
    }
}

fn is_in_path(dir: &PathBuf) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        path_var.split(':').any(|p| PathBuf::from(p) == *dir)
    } else {
        false
    }
}
```

#### 2.2: Check Duplicates Task

**`crates/xtask/src/check_duplicates.rs`:**
```rust
use anyhow::{Context, Result};
use clap::Args;

#[derive(Args)]
pub struct Args {
    /// Minimum token threshold for duplication
    #[arg(long, default_value = "50")]
    min_tokens: usize,

    /// Output format (json, console)
    #[arg(long, default_value = "console")]
    format: String,
}

pub fn run(args: Args) -> Result<()> {
    println!("Checking for duplicate code...\n");

    // Check if jscpd is installed
    if !is_jscpd_installed() {
        anyhow::bail!(
            "jscpd not found. Install with: npm install -g jscpd"
        );
    }

    let output = crate::utils::run_cmd_output(
        "jscpd",
        &[
            ".",
            "--min-tokens",
            &args.min_tokens.to_string(),
            "--format",
            &args.format,
        ],
    )?;

    println!("{}", output);

    // Parse output to determine if duplicates found
    if output.contains("duplicates") {
        println!("\n⚠️  Duplicates detected");
        std::process::exit(1);
    } else {
        println!("✓ No significant duplicates found");
        Ok(())
    }
}

fn is_jscpd_installed() -> bool {
    std::process::Command::new("jscpd")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
```

#### 2.3: New Language Plugin Task

**`crates/xtask/src/new_lang.rs`:**
```rust
use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct Args {
    /// Language name (e.g., "python", "go")
    language: String,

    /// Skip adding to workspace
    #[arg(long)]
    skip_workspace: bool,
}

pub fn run(args: Args) -> Result<()> {
    let lang = args.language.to_lowercase();
    let crate_name = format!("cb-lang-{}", lang);
    let crate_dir = PathBuf::from("crates").join(&crate_name);

    if crate_dir.exists() {
        anyhow::bail!("Crate {} already exists", crate_name);
    }

    println!("Creating language plugin: {}\n", crate_name);

    // Create directory structure
    fs::create_dir_all(&crate_dir)?;
    fs::create_dir_all(crate_dir.join("src"))?;
    fs::create_dir_all(crate_dir.join("tests"))?;

    // Generate Cargo.toml
    let cargo_toml = generate_cargo_toml(&crate_name);
    fs::write(crate_dir.join("Cargo.toml"), cargo_toml)?;

    // Generate lib.rs
    let lib_rs = generate_lib_rs(&lang);
    fs::write(crate_dir.join("src/lib.rs"), lib_rs)?;

    // Generate basic test
    let test_rs = generate_test(&lang);
    fs::write(crate_dir.join("../tests/e2e/integration_test.rs"), test_rs)?;

    // Add to workspace
    if !args.skip_workspace {
        add_to_workspace(&crate_name)?;
    }

    println!("✓ Created {}", crate_name);
    println!("\nNext steps:");
    println!("  1. Implement LanguagePlugin trait in src/lib.rs");
    println!("  2. Add parser dependencies to Cargo.toml");
    println!("  3. Run: cargo test -p {}", crate_name);

    Ok(())
}

fn generate_cargo_toml(crate_name: &str) -> String {
    format!(
        r#"[package]
name = "{crate_name}"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true

[dependencies]
mill-plugin-api = {{ path = "../mill-plugin-api" }}
mill-lang-common = {{ path = "../mill-lang-common" }}
cb-protocol = {{ path = "../cb-protocol" }}

async-trait = {{ workspace = true }}
tokio = {{ workspace = true }}
serde = {{ workspace = true }}
serde_json = {{ workspace = true }}
tracing = {{ workspace = true }}
thiserror = {{ workspace = true }}

# TODO: Add language-specific parser dependencies

[dev-dependencies]
tokio-test = "0.4"
"#,
        crate_name = crate_name
    )
}

fn generate_lib_rs(lang: &str) -> String {
    format!(
        r#"//! {} language plugin for TypeMill

use async_trait::async_trait;
use cb_plugin_api::{{LanguagePlugin, ParsedSource, Symbol}};
use cb_protocol::PluginError;

pub struct {}Plugin;

#[async_trait]
impl LanguagePlugin for {}Plugin {{
    fn name(&self) -> &str {{
        "{}"
    }}

    fn file_extensions(&self) -> &[&str] {{
        // TODO: Add file extensions for {}
        &[]
    }}

    async fn parse(&self, source: &str, file_path: &str) -> Result<ParsedSource, PluginError> {{
        // TODO: Implement parsing logic
        todo!("Implement {} parsing")
    }}

    async fn find_symbol_at_position(
        &self,
        source: &ParsedSource,
        line: usize,
        character: usize,
    ) -> Result<Option<Symbol>, PluginError> {{
        // TODO: Implement symbol lookup
        todo!("Implement symbol lookup for {}")
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_parse() {{
        let plugin = {}Plugin;
        // TODO: Add test cases
    }}
}}
"#,
        lang.to_uppercase(),
        lang.chars().next().unwrap().to_uppercase().to_string() + &lang[1..],
        lang.chars().next().unwrap().to_uppercase().to_string() + &lang[1..],
        lang,
        lang,
        lang,
        lang,
        lang.chars().next().unwrap().to_uppercase().to_string() + &lang[1..],
    )
}

fn generate_test(lang: &str) -> String {
    format!(
        r#"//! Integration tests for {} plugin

#[tokio::test]
async fn test_plugin_integration() {{
    // TODO: Add integration tests
}}
"#,
        lang
    )
}

fn add_to_workspace(crate_name: &str) -> Result<()> {
    let cargo_toml_path = PathBuf::from("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)?;

    // Find workspace members section and add new crate
    // This is simplified - production version would use toml_edit
    if !content.contains(&format!("\"crates/{}\"", crate_name)) {
        println!("\n⚠️  Please manually add to Cargo.toml workspace members:");
        println!("  \"crates/{}\"", crate_name);
    }

    Ok(())
}
```

#### 2.4: Utility Functions

**`crates/xtask/src/utils.rs`:**
```rust
use anyhow::{Context, Result};
use std::process::Command;

pub fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to run: {} {}", program, args.join(" ")))?;

    if !status.success() {
        anyhow::bail!("{} failed with status: {}", program, status);
    }

    Ok(())
}

pub fn run_cmd_output(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run: {} {}", program, args.join(" ")))?;

    if !output.status.success() {
        anyhow::bail!(
            "{} failed: {}",
            program,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn workspace_root() -> std::path::PathBuf {
    std::env::current_dir().expect("Failed to get current directory")
}
```

---

### 3. Update Workspace Configuration

**Add to `Cargo.toml`:**
```toml
[workspace]
members = [
    # ... existing members
    "crates/xtask",  # Add xtask crate
]
```

**Create `.cargo/config.toml` alias:**
```toml
[alias]
xtask = "run --package xtask --"
```

This allows `cargo xtask` instead of `cargo run --package xtask --`.

---

### 4. Update Makefile (Hybrid Approach)

Keep Makefile for common commands, use xtask for complex tasks:

```makefile
# Common commands (keep in Makefile)
.PHONY: build test fmt clippy

build:
	cargo build --release

test:
	cargo nextest run --workspace

fmt:
	cargo fmt

clippy:
	cargo clippy --all-targets --all-features

# Delegate complex tasks to xtask
.PHONY: install check-duplicates new-lang

install:
	cargo xtask install

check-duplicates:
	cargo xtask check-duplicates

new-lang:
	@if [ -z "$(LANG)" ]; then \
		echo "Usage: make new-lang LANG=<language>"; \
		exit 1; \
	fi
	cargo xtask new-lang $(LANG)

# Run all checks
check-all:
	cargo xtask check-all
```

---

### 5. Documentation Updates

#### 5.1: Update CONTRIBUTING.md

```markdown
## Build Automation (xtask)

This project uses the `xtask` pattern for build automation. Instead of shell scripts, we write automation tasks in Rust.

### Available Tasks

```bash
# Install mill
cargo xtask install

# Check for duplicate code
cargo xtask check-duplicates

# Check cargo features
cargo xtask check-features

# Create new language plugin
cargo xtask new-lang <language>

# Run all checks (fmt, clippy, test, deny)
cargo xtask check-all
```

### Adding New Tasks

1. Add subcommand to `crates/xtask/src/main.rs`
2. Implement task in `crates/xtask/src/<task>.rs`
3. Add documentation

### Why xtask?

- ✅ Cross-platform (Windows, Linux, macOS)
- ✅ Type-safe with IDE support
- ✅ Access to Rust ecosystem
- ✅ Better error handling than shell scripts
```

#### 5.2: Update README.md

```markdown
## Development

### Build Automation

Common tasks:
```bash
cargo xtask install           # Install mill
cargo xtask check-all          # Run all checks
cargo xtask new-lang python    # Scaffold new language plugin
```

See `cargo xtask --help` for all available tasks.
```

---

## Comparison: Before/After

### Before (Shell Scripts)
```bash
# Install (bash-specific)
./scripts/install.sh --dest ~/.local/bin

# Windows users: Need WSL or Git Bash
# Error handling: Exit codes only
# IDE support: None
# Testing: Manual
```

### After (xtask)
```bash
# Install (cross-platform)
cargo xtask install --dest ~/.local/bin

# Works on Windows natively
# Error handling: Result<T, E> with context
# IDE support: Full Rust IDE features
# Testing: Unit tests in Rust
```

---

## Risks and Mitigations

### Risk: Learning curve for contributors
**Likelihood:** Medium
**Impact:** Low
**Mitigation:**
- Keep Makefile for common commands
- Document xtask pattern in CONTRIBUTING.md
- Provide examples

### Risk: Increased compile time
**Likelihood:** Low
**Impact:** Low
**Mitigation:**
- xtask crate is small and compiles quickly
- Cache works well for incremental builds

### Risk: Windows compatibility issues
**Likelihood:** Low
**Impact:** Medium
**Mitigation:**
- Test on Windows before merging
- Use path utilities (pathdiff, std::path)
- Avoid platform-specific code

---

## Alternatives Considered

### Alternative 1: Keep shell scripts
**Pros:** No change required
**Cons:** Windows compatibility, no type safety

### Alternative 2: Use Make/GNU Make
**Pros:** Standard build tool
**Cons:** Not cross-platform (GNU Make on Windows is painful)

### Alternative 3: Use Just (justfile)
**Pros:** Modern, better than Make
**Cons:** Still shell scripts underneath, no type safety

### Alternative 4: Python scripts
**Pros:** Cross-platform
**Cons:** Extra dependency, not as integrated with Rust ecosystem

**Chosen:** xtask provides best integration with Rust workflow.

---

## Success Criteria

- [ ] xtask crate created and integrated
- [ ] All complex scripts migrated to xtask
- [ ] Makefile updated to use xtask for complex tasks
- [ ] CI/CD uses xtask
- [ ] Documentation updated
- [ ] Tested on Linux, macOS, and Windows
- [ ] Contributors can use `cargo xtask` commands

---

## References

- [matklad's blog: xtask pattern](https://matklad.github.io/2018/01/03/make-your-own-make.html)
- [rust-analyzer xtask](https://github.com/rust-lang/rust-analyzer/tree/master/xtask)
- [cargo xtask](https://github.com/rust-lang/cargo/tree/master/xtask)


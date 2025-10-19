# CodeBuddy Makefile
# Simple build automation for common development tasks

.PHONY: build release test test-fast test-full test-lsp install uninstall clean clean-cache first-time-setup install-lsp-servers dev-extras validate-setup help clippy fmt audit deny deny-update check check-duplicates dev watch ci build-parsers check-parser-deps check-analysis test-analysis check-handlers test-handlers check-core test-core check-lang test-lang dev-handlers dev-analysis dev-core dev-lang check-handlers-nav test-handlers-nav test-integration-refactor test-integration-analysis test-integration-nav

# Default target - show help
.DEFAULT_GOAL := help

# Configure sccache for faster builds (if installed)
SCCACHE_BIN := $(shell command -v sccache 2>/dev/null)
ifdef SCCACHE_BIN
    export RUSTC_WRAPPER=$(SCCACHE_BIN)
endif

# Default target
build:
	@command -v sccache >/dev/null 2>&1 || { echo "⚠️  Warning: sccache not found. Run 'make setup' for faster builds."; echo ""; }
	cargo build

# Optimized release build
release:
	@command -v sccache >/dev/null 2>&1 || { echo "⚠️  Warning: sccache not found. Run 'make setup' for faster builds."; echo ""; }
	cargo build --release

# Run fast tests (uses cargo-nextest). This is the recommended command for local development.
test:
	@if ! command -v cargo-nextest >/dev/null 2>&1; then \
		echo "⚠️  cargo-nextest not found. Installing now..."; \
		if command -v cargo-binstall >/dev/null 2>&1; then \
			cargo binstall --no-confirm cargo-nextest; \
		else \
			cargo install cargo-nextest --locked; \
		fi; \
		echo "✅ cargo-nextest installed"; \
	fi
	cargo nextest run --workspace

# Run the entire test suite, including ignored/skipped tests
test-full:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo nextest run --workspace --all-features --status-level skip

# Run tests requiring LSP servers
test-lsp:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo nextest run --workspace --features lsp-tests --status-level skip

# =============================================================================
# Fast-Path Development Targets - Focused Subsystem Workflows
# =============================================================================
# These targets use the cargo aliases defined in .cargo/config.toml
# to provide fast iteration on specific parts of the codebase

# Analysis subsystem
check-analysis:
	cargo check-analysis

test-analysis:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo test-analysis

# Handlers - minimal feature set (Rust only)
check-handlers:
	cargo check-handlers-core

test-handlers:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo test-handlers-core

# Core libraries (excluding integration tests and benchmarks)
check-core:
	cargo check-core

test-core:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo test-core

# Language plugins only
check-lang:
	cargo check-lang

test-lang:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo test-lang

# Navigation/analysis only (no refactoring - 15-25% faster)
check-handlers-nav:
	cargo check-handlers-nav

test-handlers-nav:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo test-handlers-nav

# Integration test filtering (60-80% faster for targeted tests)
test-integration-refactor:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo test-integration-refactor

test-integration-analysis:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo test-integration-analysis

test-integration-nav:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo test-integration-nav

# Install to ~/.local/bin (delegated to xtask for cross-platform support)
install:
	cargo xtask install

# Uninstall from ~/.local/bin
uninstall:
	@rm -f ~/.local/bin/codebuddy
	@echo "✓ Removed ~/.local/bin/codebuddy"

# Clean build artifacts
clean:
	cargo clean

# Clean build cache and reclaim disk space
clean-cache:
	@echo "🧹 Cleaning build cache..."
	cargo clean
	@echo "💡 Tip: Install cargo-sweep for smarter cleanup: cargo install cargo-sweep"

# Removed: Use 'make first-time-setup' instead (does everything)
# This provides a complete, one-command setup experience

# Install LSP servers for testing (TypeScript, Rust)
# Note: Language support temporarily reduced to TS + Rust during unified API refactoring
install-lsp-servers:
	@echo "🌐 Installing LSP servers for testing..."
	@echo ""
	@# TypeScript/JavaScript
	@if command -v npm >/dev/null 2>&1; then \
		if command -v typescript-language-server >/dev/null 2>&1; then \
			echo "  ✅ typescript-language-server already installed"; \
		else \
			echo "  → Installing typescript-language-server..."; \
			npm install -g typescript-language-server typescript && echo "  ✅ typescript-language-server installed" || echo "  ⚠️  Failed to install typescript-language-server"; \
		fi; \
	else \
		echo "  ⚠️  npm not found, skipping TypeScript LSP server"; \
		echo "     Install Node.js from: https://nodejs.org/"; \
	fi
	@echo ""
	@# Rust
	@if command -v rustup >/dev/null 2>&1; then \
		if command -v rust-analyzer >/dev/null 2>&1; then \
			echo "  ✅ rust-analyzer already installed"; \
		else \
			echo "  → Installing rust-analyzer..."; \
			rustup component add rust-analyzer && echo "  ✅ rust-analyzer installed" || echo "  ⚠️  Failed to install rust-analyzer"; \
		fi; \
	else \
		echo "  ⚠️  rustup not found, skipping Rust LSP server"; \
	fi
	@echo ""
	@echo "✅ LSP server installation complete!"
	@echo ""
	@echo "💡 Verify installation with: codebuddy status"
	@echo "📝 Note: Additional LSP servers (Python/pylsp, Go/gopls) available in git tag 'pre-language-reduction'"

# Install optional development tools (quality analysis and debugging)
dev-extras:
	@echo "🛠️  Installing optional development tools..."
	@echo ""
	@echo "📦 Code Quality & Analysis Tools:"
	@if ! command -v cargo-binstall >/dev/null 2>&1; then \
		echo "  ⚠️  cargo-binstall not found. Installing..."; \
		curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash; \
	fi
	@cargo binstall --no-confirm cargo-deny cargo-bloat
	@echo "  ✅ cargo-deny (dependency linting - licenses, security, bans)"
	@echo "  ✅ cargo-bloat (binary size analysis)"
	@echo ""
	@echo "🐛 Advanced Debugging Tools:"
	@cargo binstall --no-confirm cargo-expand cargo-flamegraph
	@echo "  ✅ cargo-expand (macro expansion for debugging)"
	@echo "  ✅ cargo-flamegraph (performance profiling)"
	@if [ "$$(uname)" = "Linux" ] && ! command -v perf >/dev/null 2>&1; then \
		echo ""; \
		echo "  ⚠️  Note: cargo-flamegraph requires 'perf' on Linux"; \
		echo "     Install with: sudo apt-get install linux-tools-generic linux-tools-common"; \
	fi
	@echo ""
	@echo "✅ Optional tools installed!"
	@echo ""
	@echo "📖 Usage Examples:"
	@echo "  cargo deny check                # Check dependencies for issues"
	@echo "  cargo bloat --release           # Analyze binary size"
	@echo "  cargo expand module::path       # Expand macros"
	@echo "  cargo flamegraph --bin codebuddy # Generate performance flamegraph"

# Code quality targets
clippy:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all --check

audit:
	@echo "🔒 Running security audit..."
	cargo audit

deny:
	@echo "🔒 Running cargo-deny checks..."
	@if ! command -v cargo-deny >/dev/null 2>&1; then \
		echo "⚠️  cargo-deny not found. Installing..."; \
		cargo install cargo-deny --locked; \
	fi
	cargo deny check

deny-update:
	@echo "📦 Updating advisory database..."
	cargo deny fetch

check: fmt clippy test audit deny

check-duplicates:
	cargo xtask check-duplicates

# Development watch mode - auto-rebuild on file changes
dev:
	@command -v cargo-watch >/dev/null 2>&1 || { echo "⚠️  cargo-watch not found. Run 'make setup' first."; exit 1; }
	@echo "🚀 Starting development watch mode..."
	cargo watch -x 'build --release'

# Alias for dev
watch: dev

# =============================================================================
# Watch Targets for Incremental Development
# =============================================================================
# These targets keep cargo-watch running in debug mode for fast iteration
# Usage: make dev-handlers, make dev-analysis, etc.

# Watch handlers with minimal features (fastest iteration)
dev-handlers:
	@command -v cargo-watch >/dev/null 2>&1 || { echo "⚠️  cargo-watch not found. Run 'make setup' first."; exit 1; }
	@echo "🚀 Watching handlers (Rust only, debug mode)..."
	cargo watch -x check-handlers-core

# Watch analysis crates only
dev-analysis:
	@command -v cargo-watch >/dev/null 2>&1 || { echo "⚠️  cargo-watch not found. Run 'make setup' first."; exit 1; }
	@echo "🚀 Watching analysis crates (debug mode)..."
	cargo watch -x check-analysis

# Watch core libraries (excludes integration tests)
dev-core:
	@command -v cargo-watch >/dev/null 2>&1 || { echo "⚠️  cargo-watch not found. Run 'make setup' first."; exit 1; }
	@echo "🚀 Watching core libraries (debug mode)..."
	cargo watch -x check-core

# Watch language plugins only
dev-lang:
	@command -v cargo-watch >/dev/null 2>&1 || { echo "⚠️  cargo-watch not found. Run 'make setup' first."; exit 1; }
	@echo "🚀 Watching language plugins (debug mode)..."
	cargo watch -x check-lang

# CI target - standardized checks for CI/CD
ci: test-full check
	@echo "✅ All CI checks passed"

# Build all external language parsers that require a separate build step
build-parsers:
	@echo "🔨 Building external language parsers..."
	@if [ -f "crates/cb-lang-java/resources/java-parser/pom.xml" ]; then \
		echo "  → Building Java parser..."; \
		(cd crates/cb-lang-java/resources/java-parser && mvn -q package) && echo "  ✅ Java parser built." || echo "  ⚠️  Java parser build failed."; \
	else \
		echo "  ⏭  Skipping Java parser (not found)."; \
	fi
	@if [ -d "crates/cb-lang-csharp/resources/csharp-parser" ]; then \
		echo "  → Building C# parser..."; \
		(cd crates/cb-lang-csharp/resources/csharp-parser && dotnet publish -c Release -r linux-x64 --self-contained > /dev/null) && \
		cp crates/cb-lang-csharp/resources/csharp-parser/bin/Release/net8.0/linux-x64/publish/csharp-parser crates/cb-lang-csharp/csharp-parser && \
		echo "  ✅ C# parser built." || echo "  ⚠️  C# parser build failed."; \
	else \
		echo "  ⏭  Skipping C# parser (not found)."; \
	fi
	@if [ -f "crates/cb-lang-typescript/resources/package.json" ]; then \
		echo "  → Installing TypeScript parser dependencies..."; \
		(cd crates/cb-lang-typescript/resources && npm install > /dev/null 2>&1) && echo "  ✅ TypeScript dependencies installed." || echo "  ⚠️  TypeScript dependencies installation failed."; \
	else \
		echo "  ⏭  Skipping TypeScript parser (not found)."; \
	fi
	@echo "✨ Parser build complete."

# Check for external dependencies required to build parsers
check-parser-deps:
	@echo "🔍 Checking for external parser build dependencies..."
	@command -v mvn >/dev/null 2>&1 && echo "  ✅ Maven (Java parser)" || echo "  ❌ Maven not found (needed for Java parser)"
	@command -v java >/dev/null 2>&1 && echo "  ✅ Java" || echo "  ❌ Java not found (needed for Java parser)"
	@command -v dotnet >/dev/null 2>&1 && echo "  ✅ .NET SDK (C# parser)" || echo "  ❌ .NET SDK not found (needed for C# parser)"
	@command -v node >/dev/null 2>&1 && echo "  ✅ Node.js (TypeScript parser)" || echo "  ✅ Node.js" || echo "  ❌ Node.js not found (needed for TypeScript parser)"
	@command -v sourcekitten >/dev/null 2>&1 && echo "  ✅ SourceKitten (Swift parser - optional)" || echo "  ⚠️  SourceKitten not found (optional for Swift)"
	@echo "✅ Dependency check complete."

# First-time developer setup workflow - THE complete setup command
first-time-setup:
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║  🚀 First-Time Developer Setup for Codebuddy            ║"
	@echo "║  This will install everything you need (~3-5 minutes)   ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "📋 Step 1/8: Checking parser build dependencies..."
	@make check-parser-deps
	@echo ""
	@echo "🔧 Step 2/8: Installing cargo-binstall (fast binary downloads)..."
	@if ! command -v cargo-binstall >/dev/null 2>&1; then \
		curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash; \
		echo "  ✅ cargo-binstall installed"; \
	else \
		echo "  ✅ cargo-binstall already installed"; \
	fi
	@echo ""
	@echo "🛠️  Step 3/8: Installing Rust development tools (pre-built binaries)..."
	@cargo binstall --no-confirm cargo-nextest sccache cargo-watch cargo-audit
	@echo "  ✅ Rust dev tools installed"
	@echo ""
	@echo "🔗 Step 4/8: Installing mold linker (3-10x faster linking)..."
	@if command -v mold >/dev/null 2>&1; then \
		echo "  ✅ mold already installed"; \
	elif command -v brew >/dev/null 2>&1; then \
		brew install mold && echo "  ✅ mold installed via Homebrew" || echo "  ⚠️  mold install failed (optional)"; \
	elif command -v apt-get >/dev/null 2>&1; then \
		sudo apt-get update -qq && sudo apt-get install -y mold clang && echo "  ✅ mold installed via apt" || echo "  ⚠️  mold install failed (optional)"; \
	elif command -v dnf >/dev/null 2>&1; then \
		sudo dnf install -y mold clang && echo "  ✅ mold installed via dnf" || echo "  ⚠️  mold install failed (optional)"; \
	elif command -v pacman >/dev/null 2>&1; then \
		sudo pacman -S --needed --noconfirm mold clang && echo "  ✅ mold installed via pacman" || echo "  ⚠️  mold install failed (optional)"; \
	else \
		echo "  ⚠️  No package manager found, skipping mold (optional)"; \
	fi
	@echo ""
	@echo "🔨 Step 5/8: Building external language parsers..."
	@make build-parsers
	@echo ""
	@echo "🏗️  Step 6/8: Building main Rust project (this may take a few minutes)..."
	@make build
	@echo ""
	@echo "🌐 Step 7/8: Installing LSP servers (for testing)..."
	@make install-lsp-servers
	@echo ""
	@echo "🔍 Step 8/8: Validating installation..."
	@make validate-setup
	@echo ""
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║  ✅ Setup Complete! Development Environment Ready       ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "🎉 Everything installed:"
	@echo "  • cargo-nextest, sccache, cargo-watch, cargo-audit"
	@echo "  • mold linker (if sudo available)"
	@echo "  • LSP servers: typescript-language-server, rust-analyzer"
	@echo "  • TypeScript parser (if Node.js available)"
	@echo ""
	@echo "📝 Note: Language support focused on TypeScript + Rust"
	@echo "   Additional languages available in git tag 'pre-language-reduction'"
	@echo ""
	@echo "🚀 Ready to develop!"
	@echo "  make test        - Run fast tests (~10s)"
	@echo "  make dev         - Auto-rebuild on file changes"
	@echo "  cargo build      - Build the project"
	@echo ""
	@echo "📚 See CONTRIBUTING.md for development workflow"

# Validate that the development environment is correctly configured
validate-setup:
	@echo "🕵️  Validating development environment..."
	@echo ""
	@echo "Checking Rust toolchain:"
	@command -v cargo >/dev/null 2>&1 && echo "  ✅ cargo" || echo "  ❌ cargo not found"
	@command -v rustc >/dev/null 2>&1 && echo "  ✅ rustc" || echo "  ❌ rustc not found"
	@command -v cargo-nextest >/dev/null 2>&1 && echo "  ✅ cargo-nextest" || echo "  ⚠️  cargo-nextest not installed (run: make setup)"
	@command -v sccache >/dev/null 2>&1 && echo "  ✅ sccache" || echo "  ⚠️  sccache not installed (run: make setup-full)"
	@echo ""
	@echo "Checking parser build dependencies:"
	@command -v mvn >/dev/null 2>&1 && echo "  ✅ Maven" || echo "  ⚠️  Maven not found (Java parser won't build)"
	@command -v java >/dev/null 2>&1 && echo "  ✅ Java" || echo "  ⚠️  Java not found (Java parser won't build)"
	@command -v dotnet >/dev/null 2>&1 && echo "  ✅ .NET SDK" || echo "  ⚠️  .NET SDK not found (C# parser won't build)"
	@command -v node >/dev/null 2>&1 && echo "  ✅ Node.js" || echo "  ⚠️  Node.js not found (TypeScript parser won't build)"
	@echo ""
	@echo "Checking LSP servers (for testing):"
	@command -v typescript-language-server >/dev/null 2>&1 && echo "  ✅ typescript-language-server" || echo "  ⚠️  typescript-language-server not installed (run: make install-lsp-servers)"
	@command -v rust-analyzer >/dev/null 2>&1 && echo "  ✅ rust-analyzer" || echo "  ⚠️  rust-analyzer not installed (run: make install-lsp-servers)"
	@echo "📝 Note: Language support focused on TypeScript + Rust"
	@echo ""
	@echo "Checking build artifacts:"
	@if [ -f "target/debug/codebuddy" ]; then \
		echo "  ✅ Debug binary (target/debug/codebuddy)"; \
		./target/debug/codebuddy --version | sed 's/^/     /'; \
	else \
		echo "  ❌ Debug binary not found (run: make build)"; \
	fi
	@if [ -f "target/release/codebuddy" ]; then \
		echo "  ✅ Release binary (target/release/codebuddy)"; \
	else \
		echo "  ⚠️  Release binary not found (run: make release)"; \
	fi
	@echo ""
	@if [ -f "target/debug/codebuddy" ] && command -v cargo-nextest >/dev/null 2>&1; then \
		echo "✅ Development environment is ready!"; \
		echo "   Run 'make test' to verify everything works."; \
		echo "   LSP tests require: make install-lsp-servers"; \
	else \
		echo "⚠️  Development environment has issues (see above)."; \
		echo "   Run 'make first-time-setup' to fix automatically."; \
	fi

# Show available commands
help:
	@echo "CodeBuddy - Available Commands"
	@echo "================================"
	@echo ""
	@echo "🚀 First-Time Setup:"
	@echo "  make first-time-setup  - Run this once to set up your entire development environment."
	@echo ""
	@echo "🔨 Build & Install:"
	@echo "  make build             - Build debug version"
	@echo "  make release           - Build optimized release version"
	@echo "  make install           - Install to ~/.local/bin (auto-configures PATH)"
	@echo "  make uninstall         - Remove installed binary"
	@echo ""
	@echo "💻 Development:"
	@echo "  make dev               - Build in watch mode (auto-rebuild on file changes)"
	@echo "  make install-lsp-servers - Install LSP servers for testing"
	@echo "  make dev-extras        - Install optional tools (cargo-deny, cargo-bloat, cargo-expand, cargo-flamegraph)"
	@echo ""
	@echo "✅ Testing (uses cargo-nextest):"
	@echo "  make test              - Run fast tests (~10s, auto-installs cargo-nextest)"
	@echo "  make test-lsp          - Run tests requiring LSP servers (~60s)"
	@echo "  make test-full         - Run the entire test suite, including skipped tests (~80s)"
	@echo ""
	@echo "⚡ Fast-Path Development (focused subsystems):"
	@echo "  make check-handlers    - Check handlers crate only (minimal features)"
	@echo "  make test-handlers     - Test handlers crate only (minimal features)"
	@echo "  make check-analysis    - Check all analysis crates"
	@echo "  make test-analysis     - Test all analysis crates"
	@echo "  make check-core        - Check core libraries (excludes integration tests)"
	@echo "  make test-core         - Test core libraries (excludes integration tests)"
	@echo "  make check-lang        - Check language plugins only"
	@echo "  make test-lang         - Test language plugins only"
	@echo ""
	@echo "🎯 Specialized Builds:"
	@echo "  make check-handlers-nav       - Navigation/analysis only (15-25% faster)"
	@echo "  make test-handlers-nav        - Test navigation/analysis only"
	@echo ""
	@echo "🔬 Integration Test Filtering (60-80% faster):"
	@echo "  make test-integration-refactor - Run refactoring tests only"
	@echo "  make test-integration-analysis - Run analysis tests only"
	@echo "  make test-integration-nav      - Run navigation tests only"
	@echo ""
	@echo "🔄 Watch Mode (auto-rebuild on changes, debug mode):"
	@echo "  make dev-handlers      - Watch handlers with minimal features (fastest)"
	@echo "  make dev-analysis      - Watch analysis crates only"
	@echo "  make dev-core          - Watch core libraries"
	@echo "  make dev-lang          - Watch language plugins"
	@echo ""
	@echo "🧹 Cleanup:"
	@echo "  make clean             - Remove build artifacts"
	@echo "  make clean-cache       - Remove all build artifacts (frees ~30-40GB)"
	@echo ""
	@echo "🔍 Code Quality & Validation:"
	@echo "  make clippy            - Run clippy linter"
	@echo "  make fmt               - Check code formatting"
	@echo "  make audit             - Run security audit (cargo-audit)"
	@echo "  make deny              - Run dependency checks (cargo-deny: licenses, advisories, bans)"
	@echo "  make deny-update       - Update advisory database for cargo-deny"
	@echo "  make check             - Run fmt + clippy + test + audit + deny"
	@echo "  make check-duplicates  - Detect duplicate code & complexity"
	@echo "  make validate-setup    - Check if your dev environment is set up correctly"
	@echo "  make ci                - Run all CI checks (for CI/CD)"
	@echo ""
	@echo "🔧 Language Parsers:"
	@echo "  make build-parsers     - Build all external language parsers"
	@echo "  make check-parser-deps - Check parser build dependencies"
	@echo ""
	@echo "🤖 Build Automation (xtask):"
	@echo "  cargo xtask install    - Install codebuddy to $$HOME/.local/bin"
	@echo "  cargo xtask check-all  - Run all checks (fmt, clippy, test, deny)"
	@echo "  cargo xtask new-lang <lang> - Scaffold new language plugin"
	@echo "  cargo xtask --help     - Show all xtask commands"
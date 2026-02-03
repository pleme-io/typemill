# TypeMill Makefile
# Simple build automation for common development tasks

.PHONY: build release release-rust release-npm release-all test test-fast test-full test-lsp install uninstall clean clean-cache clean-registry first-time-setup install-lsp-servers dev-extras validate-setup help clippy fmt audit deny deny-update check check-duplicates dev watch ci ci-local build-parsers check-parser-deps check-analysis test-analysis check-handlers test-handlers check-core test-core check-lang test-lang dev-handlers dev-analysis dev-core dev-lang check-handlers-nav test-handlers-nav test-integration-refactor test-integration-analysis test-integration-nav doctor

# Default target - show help
.DEFAULT_GOAL := help

# Configure sccache for faster builds (if installed)
SCCACHE_BIN := $(shell command -v sccache 2>/dev/null)
ifdef SCCACHE_BIN
    export RUSTC_WRAPPER=$(SCCACHE_BIN)
endif

# Ensure cargo is in PATH (handles fresh Rust installations)
CARGO := $(shell command -v cargo 2>/dev/null || echo "$$HOME/.cargo/bin/cargo")

# Default target
build:
	@command -v sccache >/dev/null 2>&1 || { echo "⚠️  Warning: sccache not found. Run 'make setup' for faster builds."; echo ""; }
	$(CARGO) build

# Optimized release build
release:
	@command -v sccache >/dev/null 2>&1 || { echo "⚠️  Warning: sccache not found. Run 'make setup' for faster builds."; echo ""; }
	$(CARGO) build --release

# Explicit release targets
NPM_DIR ?= packages/typemill
NPM_PKG ?= @goobits/typemill
NPM_VERSION ?= $(shell node -p "require('./$(NPM_DIR)/package.json').version" 2>/dev/null)
NPM_BUMP ?= patch
NPM_BIN_NAME ?= mill

release-rust:
	$(CARGO) build --release

release-npm:
	@command -v npm >/dev/null 2>&1 || { echo "❌ npm not found"; exit 1; }
	@npm whoami >/dev/null 2>&1 || { echo "❌ npm not logged in"; exit 1; }
	@test -f $(NPM_DIR)/package.json || { echo "❌ package.json not found in $(NPM_DIR)"; exit 1; }
	@npm pkg fix --silent --prefix $(NPM_DIR)
	@stashed=""; \
	if command -v git >/dev/null 2>&1; then \
		if [ -n "$$(git status --porcelain)" ]; then \
			echo "⚠️  Working tree dirty. Stashing before version bump/build..."; \
			git stash push -u -m "typemill-release-npm" >/dev/null 2>&1 && stashed="yes"; \
		fi; \
	fi; \
	pkg_path="$(NPM_DIR)/package.json"; \
	current_version=$$(node -p "require('./$$pkg_path').version"); \
	published_version=$$(npm view $(NPM_PKG) version 2>/dev/null || echo "0.0.0"); \
	needs_bump=""; \
	if [ "$$published_version" != "0.0.0" ]; then \
		latest=$$(printf '%s\n' "$$current_version" "$$published_version" | sort -V | tail -n1); \
		if [ "$$latest" = "$$published_version" ]; then \
			needs_bump="yes"; \
		fi; \
	fi; \
	while [ "$$needs_bump" = "yes" ]; do \
		echo "⚠️  $(NPM_PKG) version $$current_version already published (latest $$published_version). Auto-bumping $(NPM_BUMP)..."; \
		( cd $(NPM_DIR) && npm version $(NPM_BUMP) --no-git-tag-version ); \
		current_version=$$(node -p "require('./$$pkg_path').version"); \
		latest=$$(printf '%s\n' "$$current_version" "$$published_version" | sort -V | tail -n1); \
		if [ "$$latest" = "$$published_version" ]; then \
			needs_bump="yes"; \
		else \
			needs_bump=""; \
		fi; \
		echo "✅ Bumped to $$current_version"; \
	done; \
	node -e "const fs=require('fs');const p='Cargo.toml';const v='$$current_version';let s=fs.readFileSync(p,'utf8');s=s.replace(/^version = \\\"\\d+\\.\\d+\\.\\d+\\\"/m, 'version = \"'+v+'\"');fs.writeFileSync(p,s);"; \
	( cd $(NPM_DIR) && TYPEMILL_TARGETS=$${TYPEMILL_TARGETS:-aarch64-unknown-linux-gnu} TYPEMILL_ALLOW_DIRTY=1 TYPEMILL_SKIP_PUBLISH=1 TYPEMILL_SKIP_GIT=1 npm run release ); \
	status=$$?; \
	if [ "$$stashed" = "yes" ]; then \
		git stash pop >/dev/null 2>&1 || true; \
	fi; \
	if [ $$status -ne 0 ]; then \
		exit $$status; \
	fi; \
	current_version=$$(node -p "require('./$$pkg_path').version"); \
	platform_dir=$$(uname -s | tr '[:upper:]' '[:lower:]'); \
	arch=$$(uname -m); \
	case "$$platform_dir:$$arch" in \
		darwin:arm64) target_dir="aarch64-apple-darwin" ;; \
		darwin:x86_64) target_dir="x86_64-apple-darwin" ;; \
		linux:aarch64|linux:arm64) target_dir="aarch64-unknown-linux-gnu" ;; \
		linux:x86_64) target_dir="x86_64-unknown-linux-gnu" ;; \
		*) target_dir="" ;; \
	esac; \
	if [ -n "$$target_dir" ]; then \
		bin_path="$(NPM_DIR)/bin/$$target_dir/$(NPM_BIN_NAME)"; \
		if [ -x "$$bin_path" ]; then \
			expected=$$(node -p "require('./$$pkg_path').version"); \
			actual=$$($$bin_path --version 2>&1 | tail -n1); \
			if [ -z "$$actual" ] && command -v strings >/dev/null 2>&1; then \
				actual=$$(strings "$$bin_path" | grep -E "mill[[:space:]]+[0-9]+\\.[0-9]+\\.[0-9]+" | head -n1); \
			fi; \
			if [ -z "$$actual" ]; then \
				echo "⚠️  Could not read version from $$bin_path; skipping binary version check."; \
			elif ! echo "$$actual" | grep -q "$$expected"; then \
				echo "❌ $$bin_path reports '$$actual', expected $$expected"; \
				exit 1; \
			fi; \
		else \
			echo "⚠️  Skipping binary version check (missing or non-executable: $$bin_path)"; \
		fi; \
	else \
		echo "⚠️  Skipping binary version check (unknown platform)"; \
	fi; \
	npmrc_path="$(NPM_DIR)/.npmrc"; \
	used_npmrc=""; \
	if [ -n "$$NPM_TOKEN" ]; then \
		echo "//registry.npmjs.org/:_authToken=$$NPM_TOKEN" > "$$npmrc_path"; \
		used_npmrc="yes"; \
	fi; \
	cd $(NPM_DIR) && npm publish --access public; \
	status=$$?; \
	if [ "$$used_npmrc" = "yes" ]; then \
		rm -f "$$npmrc_path"; \
	fi; \
	exit $$status

release-all: release-rust release-npm

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
	@echo "🔍 Checking for debug binary (required for e2e tests)..."
	@if [ ! -f target/debug/mill ]; then \
		echo "📦 Debug binary not found, building now..."; \
		$(CARGO) build --workspace; \
		echo "✅ Debug build complete"; \
	else \
		echo "✅ Debug binary exists"; \
	fi
	cargo nextest run --workspace --release

# Run the entire test suite, including ignored/skipped tests
test-full:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	@echo "🔍 Checking for debug binary (required for e2e tests)..."
	@if [ ! -f target/debug/mill ]; then \
		echo "📦 Debug binary not found, building now..."; \
		$(CARGO) build --workspace; \
		echo "✅ Debug build complete"; \
	else \
		echo "✅ Debug binary exists"; \
	fi
	cargo nextest run --workspace --release --status-level skip

# Run tests requiring LSP servers
test-lsp:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	@echo "🔍 Checking for debug binary (required for e2e tests)..."
	@if [ ! -f target/debug/mill ]; then \
		echo "📦 Debug binary not found, building now..."; \
		$(CARGO) build --workspace; \
		echo "✅ Debug build complete"; \
	else \
		echo "✅ Debug binary exists"; \
	fi
	cargo nextest run --workspace --release --features lsp-tests --status-level skip

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
	@rm -f ~/.local/bin/mill
	@echo "✓ Removed ~/.local/bin/mill"

# Clean build artifacts
clean:
	cargo clean

# Clean build cache and reclaim disk space
clean-cache:
	@echo "🧹 Cleaning build cache..."
	cargo clean
	@echo "💡 Tip: Install cargo-sweep for smarter cleanup: cargo install cargo-sweep"

# Clean Cargo registry cache (use if crates are corrupted)
clean-registry:
	rm -rf ~/.cargo/registry
	@echo "✅ Cargo registry cache cleared"

# Quick toolchain sanity check
doctor:
	@command -v cargo >/dev/null 2>&1 || { echo "❌ cargo missing"; exit 1; }
	@command -v node >/dev/null 2>&1 || { echo "❌ node missing"; exit 1; }
	@command -v npm >/dev/null 2>&1 || { echo "❌ npm missing"; exit 1; }
	@echo "✅ toolchain ok"

# Removed: Use 'make first-time-setup' instead (does everything)
# This provides a complete, one-command setup experience

# Install LSP servers for language plugin development
# Installs LSP servers based on which language plugins are present and which tools are available
install-lsp-servers:
	@echo "🌐 Installing LSP servers for language plugin development..."
	@echo "💡 LSP servers are optional - only installing for available language plugins"
	@echo ""
	@# Rust (core language - always installed)
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
	@# TypeScript/JavaScript (if plugin present or Node.js available)
	@if [ -d "crates/mill-lang-typescript" ] || command -v npm >/dev/null 2>&1; then \
		if command -v npm >/dev/null 2>&1; then \
			if command -v typescript-language-server >/dev/null 2>&1; then \
				echo "  ✅ typescript-language-server already installed"; \
			else \
				echo "  → Installing typescript-language-server..."; \
				npm install -g typescript-language-server typescript && echo "  ✅ typescript-language-server installed" || echo "  ⚠️  Failed to install typescript-language-server"; \
			fi; \
		else \
			echo "  ⚠️  npm not found - skipping TypeScript LSP server"; \
			echo "     Install Node.js from: https://nodejs.org/"; \
		fi; \
	else \
		echo "  ⏭  TypeScript plugin not present - skipping"; \
	fi
	@echo ""
	@# Python (if plugin present or Python available)
	@if [ -d "crates/mill-lang-python" ] || command -v python3 >/dev/null 2>&1; then \
		if command -v python3 >/dev/null 2>&1; then \
			if command -v pylsp >/dev/null 2>&1; then \
				echo "  ✅ pylsp (Python LSP) already installed"; \
			else \
				echo "  → Installing pylsp (Python LSP)..."; \
				python3 -m pip install --user python-lsp-server && echo "  ✅ pylsp installed" || echo "  ⚠️  Failed to install pylsp"; \
			fi; \
		else \
			echo "  ⚠️  python3 not found - skipping Python LSP server"; \
		fi; \
	else \
		echo "  ⏭  Python plugin not present - skipping"; \
	fi
	@echo ""
	@# Go (if plugin present or Go available)
	@if [ -d "crates/mill-lang-go" ] || command -v go >/dev/null 2>&1; then \
		if command -v go >/dev/null 2>&1; then \
			if command -v gopls >/dev/null 2>&1; then \
				echo "  ✅ gopls (Go LSP) already installed"; \
			else \
				echo "  → Installing gopls (Go LSP)..."; \
				go install golang.org/x/tools/gopls@latest && echo "  ✅ gopls installed" || echo "  ⚠️  Failed to install gopls"; \
			fi; \
		else \
			echo "  ⚠️  go not found - skipping Go LSP server"; \
		fi; \
	else \
		echo "  ⏭  Go plugin not present - skipping"; \
	fi
	@echo ""
	@echo "✅ LSP server installation complete!"
	@echo ""
	@echo "💡 Verify installation with: mill status"

# Install optional development tools (quality analysis and debugging)
dev-extras:
	@echo "🛠️  Installing optional development tools..."
	@echo ""
	@echo "📦 Code Quality Tools:"
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
	@echo "  cargo flamegraph --bin mill # Generate performance flamegraph"

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

# Test CI checks locally - matches GitHub Actions exactly
ci-local:
	@echo "🧪 Running GitHub Actions CI checks locally..."
	@echo ""
	@echo "1️⃣  Format check (rustfmt)"
	@$(CARGO) fmt --all -- --check || { echo "❌ Format check failed"; exit 1; }
	@echo "✅ Format check passed\n"
	@echo "2️⃣  Lint check (clippy --all-targets --all-features)"
	@$(CARGO) clippy --all-targets --all-features -- -D warnings || { echo "❌ Clippy failed"; exit 1; }
	@echo "✅ Clippy passed\n"
	@echo "3️⃣  Build check"
	@$(CARGO) build --verbose || { echo "❌ Build failed"; exit 1; }
	@echo "✅ Build passed\n"
	@echo "4️⃣  Test suite"
	@if command -v cargo-nextest >/dev/null 2>&1; then \
		$(CARGO) nextest run --workspace || { echo "❌ Tests failed"; exit 1; }; \
	else \
		$(CARGO) test --workspace || { echo "❌ Tests failed"; exit 1; }; \
	fi
	@echo "✅ Tests passed\n"
	@echo "5️⃣  Doc tests"
	@$(CARGO) test --doc || { echo "❌ Doc tests failed"; exit 1; }
	@echo "✅ Doc tests passed\n"
	@echo ""
	@echo "✅ All CI checks passed! Safe to push to GitHub."

# Build all external language parsers that require a separate build step
# Language plugins are optional - this target detects and builds only what's available
build-parsers:
	@echo "🔨 Building available language parsers..."
	@echo "💡 Language plugins are optional - only building what's present in crates/mill-lang-*"
	@echo ""
	@# Java parser (requires Maven + Java)
	@if [ -d "crates/mill-lang-java" ]; then \
		if [ -f "crates/mill-lang-java/resources/java-parser/pom.xml" ]; then \
			if command -v mvn >/dev/null 2>&1; then \
				echo "  → Building Java parser..."; \
				(cd crates/mill-lang-java/resources/java-parser && mvn -q package) && echo "  ✅ Java parser built." || echo "  ⚠️  Java parser build failed."; \
			else \
				echo "  ⚠️  Maven not found - skipping Java parser (install: apt-get install maven)"; \
			fi; \
		else \
			echo "  ⚠️  Java parser source not found (missing pom.xml)"; \
		fi; \
	else \
		echo "  ⏭  Java plugin not present (crates/mill-lang-java)"; \
	fi
	@echo ""
	@# C# parser (requires .NET SDK)
	@if [ -d "crates/mill-lang-csharp" ]; then \
		if [ -d "crates/mill-lang-csharp/resources/csharp-parser" ]; then \
			if command -v dotnet >/dev/null 2>&1; then \
				echo "  → Building C# parser..."; \
				(cd crates/mill-lang-csharp/resources/csharp-parser && dotnet publish -c Release -r linux-x64 --self-contained > /dev/null) && \
				cp crates/mill-lang-csharp/resources/csharp-parser/bin/Release/net8.0/linux-x64/publish/csharp-parser crates/mill-lang-csharp/csharp-parser && \
				echo "  ✅ C# parser built." || echo "  ⚠️  C# parser build failed."; \
			else \
				echo "  ⚠️  .NET SDK not found - skipping C# parser (install: https://dotnet.microsoft.com/)"; \
			fi; \
		else \
			echo "  ⚠️  C# parser source not found"; \
		fi; \
	else \
		echo "  ⏭  C# plugin not present (crates/mill-lang-csharp)"; \
	fi
	@echo ""
	@# TypeScript parser (requires Node.js)
	@if [ -d "crates/mill-lang-typescript" ]; then \
		if [ -f "crates/mill-lang-typescript/resources/package.json" ]; then \
			if command -v npm >/dev/null 2>&1; then \
				echo "  → Installing TypeScript parser dependencies..."; \
				(cd crates/mill-lang-typescript/resources && npm install > /dev/null 2>&1) && echo "  ✅ TypeScript dependencies installed." || echo "  ⚠️  TypeScript dependencies installation failed."; \
			else \
				echo "  ⚠️  npm not found - skipping TypeScript parser (install: https://nodejs.org/)"; \
			fi; \
		else \
			echo "  ⚠️  TypeScript parser configuration not found (missing package.json)"; \
		fi; \
	else \
		echo "  ⏭  TypeScript plugin present (no external build needed)"; \
	fi
	@echo ""
	@# Python plugin (no external parser needed)
	@if [ -d "crates/mill-lang-python" ]; then \
		echo "  ✅ Python plugin present (no external build needed)"; \
	else \
		echo "  ⏭  Python plugin not present (crates/mill-lang-python)"; \
	fi
	@echo ""
	@# Go plugin (no external parser needed)
	@if [ -d "crates/mill-lang-go" ]; then \
		echo "  ✅ Go plugin present (no external build needed)"; \
	else \
		echo "  ⏭  Go plugin not present (crates/mill-lang-go)"; \
	fi
	@echo ""
	@echo "✨ Parser build complete."

# Check for external dependencies required to build parsers
# Language plugins are optional - shows what's available vs needed
check-parser-deps:
	@echo "🔍 Checking for external parser build dependencies..."
	@echo "💡 All language plugins are optional - install only what you need"
	@echo ""
	@echo "Core Requirements:"
	@command -v cargo >/dev/null 2>&1 && echo "  ✅ Rust toolchain" || echo "  ❌ Rust toolchain not found (REQUIRED)"
	@echo ""
	@echo "Language Plugin Dependencies (Optional):"
	@command -v mvn >/dev/null 2>&1 && echo "  ✅ Maven (for Java plugin)" || echo "  ⚠️  Maven not found (optional - needed for Java plugin)"
	@command -v java >/dev/null 2>&1 && echo "  ✅ Java (for Java plugin)" || echo "  ⚠️  Java not found (optional - needed for Java plugin)"
	@command -v dotnet >/dev/null 2>&1 && echo "  ✅ .NET SDK (for C# plugin)" || echo "  ⚠️  .NET SDK not found (optional - needed for C# plugin)"
	@command -v node >/dev/null 2>&1 && echo "  ✅ Node.js (for TypeScript plugin)" || echo "  ⚠️  Node.js not found (optional - needed for TypeScript plugin)"
	@command -v npm >/dev/null 2>&1 && echo "  ✅ npm (for TypeScript plugin)" || echo "  ⚠️  npm not found (optional - needed for TypeScript plugin)"
	@echo ""
	@echo "✅ Dependency check complete."

# First-time developer setup workflow - THE complete setup command
first-time-setup:
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║  🚀 First-Time Developer Setup for TypeMill             ║"
	@echo "║  This will install everything you need (~3-5 minutes)   ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "📦 Step 1/10: Initializing git submodules..."
	@if [ -d ".git" ]; then \
		git submodule update --init --recursive && echo "  ✅ Git submodules initialized"; \
	else \
		echo "  ⚠️  Not a git repository, skipping submodule init"; \
	fi
	@echo ""
	@echo "📋 Step 2/10: Checking parser build dependencies..."
	@make check-parser-deps
	@echo ""
	@echo "🦀 Step 3/10: Ensuring Rust toolchain is installed..."
	@if ! command -v cargo >/dev/null 2>&1; then \
		if [ -f "$$HOME/.cargo/env" ]; then \
			echo "  ℹ️  Rust installed but not in PATH, loading environment..."; \
			. "$$HOME/.cargo/env" && echo "  ✅ Rust environment loaded"; \
		else \
			echo "  → Installing Rust via rustup (this takes ~30 seconds)..."; \
			curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
			echo "  ✅ Rust installed successfully"; \
			. "$$HOME/.cargo/env"; \
			echo "  ✅ Rust environment loaded"; \
		fi; \
	else \
		echo "  ✅ Rust toolchain already installed"; \
	fi
	@echo ""
	@echo "🔧 Step 4/10: Installing cargo-binstall (fast binary downloads)..."
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	if ! command -v cargo-binstall >/dev/null 2>&1; then \
		curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash; \
		echo "  ✅ cargo-binstall installed"; \
	else \
		echo "  ✅ cargo-binstall already installed"; \
	fi
	@echo ""
	@echo "🛠️  Step 5/10: Installing Rust development tools (pre-built binaries)..."
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	cargo binstall --no-confirm cargo-nextest sccache cargo-watch cargo-audit; \
	echo "  ✅ Rust dev tools installed"
	@echo ""
	@echo "🔗 Step 6/10: Installing mold linker (3-10x faster linking)..."
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
	@echo "🔨 Step 7/10: Building external language parsers..."
	@make build-parsers
	@echo ""
	@echo "🏗️  Step 8/10: Building main Rust project (this may take a few minutes)..."
	@echo "  → Fetching dependencies..."
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	cargo fetch --locked || { echo "  ⚠️  cargo fetch failed, trying without --locked"; cargo fetch; }
	@echo "  → Building project (using parallel compilation for speed)..."
	@export PATH="$$HOME/.cargo/bin:$$PATH"; \
	cargo build --offline || cargo build
	@echo "  💡 Tip: If build fails with missing dependencies, run: cargo clean && cargo fetch"
	@echo ""
	@echo "🌐 Step 9/10: Installing LSP servers (for testing)..."
	@make install-lsp-servers
	@echo ""
	@echo "🔍 Step 10/10: Validating installation..."
	@make validate-setup
	@echo ""
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║  ✅ Setup Complete! Development Environment Ready       ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "🎉 Core tools installed:"
	@echo "  • cargo-nextest, sccache, cargo-watch, cargo-audit"
	@echo "  • mold linker (if sudo available)"
	@echo "  • LSP servers: typescript-language-server, rust-analyzer"
	@echo ""
	@echo "🔌 Language plugins:"
	@echo "  • TypeScript + Rust: Currently available"
	@echo "  • Python, Go, Java, C#: Optional plugins (add as needed)"
	@echo "  • Run 'make check-parser-deps' to see what's available"
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
	@if [ -f "target/debug/mill" ]; then \
		echo "  ✅ Debug binary (target/debug/mill)"; \
		./target/debug/mill --version | sed 's/^/     /'; \
	else \
		echo "  ❌ Debug binary not found (run: make build)"; \
	fi
	@if [ -f "target/release/mill" ]; then \
		echo "  ✅ Release binary (target/release/mill)"; \
	else \
		echo "  ⚠️  Release binary not found (run: make release)"; \
	fi
	@echo ""
	@if [ -f "target/debug/mill" ] && command -v cargo-nextest >/dev/null 2>&1; then \
		echo "✅ Development environment is ready!"; \
		echo "   Run 'make test' to verify everything works."; \
		echo "   LSP tests require: make install-lsp-servers"; \
	else \
		echo "⚠️  Development environment has issues (see above)."; \
		echo "   Run 'make first-time-setup' to fix automatically."; \
	fi

# Show available commands
help:
	@echo "TypeMill - Available Commands"
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
	@echo "  make ci-local          - Test GitHub Actions CI locally before pushing"
	@echo ""
	@echo "🔧 Language Parsers:"
	@echo "  make build-parsers     - Build all external language parsers"
	@echo "  make check-parser-deps - Check parser build dependencies"
	@echo ""
	@echo "🤖 Build Automation (xtask):"
	@echo "  cargo xtask install    - Install mill to $$HOME/.local/bin"
	@echo "  cargo xtask check-all  - Run all checks (fmt, clippy, test, deny)"
	@echo "  cargo xtask new-lang <lang> - Scaffold new language plugin"
	@echo "  cargo xtask --help     - Show all xtask commands"

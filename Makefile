# CodeBuddy Makefile
# Simple build automation for common development tasks

.PHONY: build release test test-fast test-full test-lsp install uninstall clean clean-cache setup help clippy fmt audit check check-duplicates dev watch ci build-parsers check-parser-deps

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

# Install to ~/.local/bin (ensure it's in your PATH)
install: release
	@mkdir -p ~/.local/bin
	@cp target/release/codebuddy ~/.local/bin/
	@echo "✓ Installed to ~/.local/bin/codebuddy"
	@echo ""
	@# Auto-detect and update shell config if needed
	@if ! echo "$$PATH" | grep -q "$$HOME/.local/bin"; then \
		if [ -f "$$HOME/.zshrc" ] && [ "$$SHELL" = "/bin/zsh" ] || [ "$$SHELL" = "/usr/bin/zsh" ]; then \
			if ! grep -q 'export PATH="$$HOME/.local/bin:' "$$HOME/.zshrc"; then \
				echo 'export PATH="$$HOME/.local/bin:$$PATH"' >> "$$HOME/.zshrc"; \
				echo "✓ Added ~/.local/bin to PATH in ~/.zshrc"; \
				echo "  Run: source ~/.zshrc"; \
			fi; \
		elif [ -f "$$HOME/.bashrc" ]; then \
			if ! grep -q 'export PATH="$$HOME/.local/bin:' "$$HOME/.bashrc"; then \
				echo 'export PATH="$$HOME/.local/bin:$$PATH"' >> "$$HOME/.bashrc"; \
				echo "✓ Added ~/.local/bin to PATH in ~/.bashrc"; \
				echo "  Run: source ~/.bashrc"; \
			fi; \
		elif [ -f "$$HOME/.bash_profile" ]; then \
			if ! grep -q 'export PATH="$$HOME/.local/bin:' "$$HOME/.bash_profile"; then \
				echo 'export PATH="$$HOME/.local/bin:$$PATH"' >> "$$HOME/.bash_profile"; \
				echo "✓ Added ~/.local/bin to PATH in ~/.bash_profile"; \
				echo "  Run: source ~/.bash_profile"; \
			fi; \
		else \
			echo "⚠️  Could not detect shell config. Manually add to PATH:"; \
			echo "  export PATH=\"\$$HOME/.local/bin:\$$PATH\""; \
		fi; \
	else \
		echo "✓ ~/.local/bin already in PATH"; \
	fi

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

# Quick setup - installs only essential tools (~30 seconds with binstall)
setup:
	@echo "📦 Installing essential build tools (fast setup)..."
	@echo ""
	@# Install cargo-binstall if not present (downloads pre-built binaries)
	@if ! command -v cargo-binstall >/dev/null 2>&1; then \
		echo "  → Installing cargo-binstall (enables fast binary downloads)..."; \
		curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash; \
		echo "  ✅ cargo-binstall installed"; \
	else \
		echo "  ✅ cargo-binstall already installed"; \
	fi
	@echo ""
	@# Install essential tools via binstall (pre-built binaries, super fast)
	@echo "  → Installing cargo-nextest (test runner)..."
	@cargo binstall --no-confirm cargo-nextest 2>/dev/null || cargo install cargo-nextest --locked
	@echo "  ✅ cargo-nextest installed"
	@echo ""
	@echo "✅ Essential setup complete! (~30 seconds)"
	@echo ""
	@echo "💡 Optional enhancements:"
	@echo "  make setup-full         - Install all dev tools (sccache, cargo-watch, etc.)"
	@echo "  make install-lsp-servers - Install LSP servers for testing"

# Full developer setup - installs all optimization tools (~2-3 minutes with binstall)
setup-full:
	@echo "📦 Installing full development environment..."
	@echo "   This includes sccache, mold, cargo-watch, and more (~2-3 min)"
	@echo ""
	@# Ensure binstall is available
	@if ! command -v cargo-binstall >/dev/null 2>&1; then \
		echo "  → Installing cargo-binstall first..."; \
		curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash; \
	fi
	@echo ""
	@echo "  → Installing cargo tools via binstall (pre-built binaries)..."
	@cargo binstall --no-confirm cargo-nextest sccache cargo-watch cargo-audit cargo-edit
	@echo ""
	@# Install mold linker (system package, requires sudo)
	@echo "  → Installing mold linker (fast linking)..."
	@if command -v mold >/dev/null 2>&1; then \
		echo "  ✅ mold already installed"; \
	elif command -v brew >/dev/null 2>&1; then \
		brew install mold && echo "  ✅ mold installed via Homebrew"; \
	elif command -v apt-get >/dev/null 2>&1; then \
		sudo apt-get update -qq && sudo apt-get install -y mold clang && echo "  ✅ mold installed via apt"; \
	elif command -v dnf >/dev/null 2>&1; then \
		sudo dnf install -y mold clang && echo "  ✅ mold installed via dnf"; \
	elif command -v pacman >/dev/null 2>&1; then \
		sudo pacman -S --needed --noconfirm mold clang && echo "  ✅ mold installed via pacman"; \
	else \
		echo "  ⚠️  No package manager found, skipping mold"; \
		echo "     Install manually: https://github.com/rui314/mold#installation"; \
	fi
	@echo ""
	@echo "✅ Full setup complete!"
	@echo ""
	@echo "💡 Next: Run 'make install-lsp-servers' for LSP testing support"

# Install LSP servers for testing (TypeScript, Python, Go, Rust)
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
	@# Python
	@if command -v pip >/dev/null 2>&1 || command -v pip3 >/dev/null 2>&1; then \
		if command -v pylsp >/dev/null 2>&1; then \
			echo "  ✅ pylsp already installed"; \
		else \
			echo "  → Installing python-lsp-server..."; \
			(pip install --user "python-lsp-server[all]" || pip3 install --user "python-lsp-server[all]") && echo "  ✅ pylsp installed" || echo "  ⚠️  Failed to install pylsp"; \
		fi; \
	else \
		echo "  ⚠️  pip not found, skipping Python LSP server"; \
		echo "     Install Python from: https://www.python.org/"; \
	fi
	@echo ""
	@# Go
	@if command -v go >/dev/null 2>&1; then \
		if command -v gopls >/dev/null 2>&1; then \
			echo "  ✅ gopls already installed"; \
		else \
			echo "  → Installing gopls..."; \
			go install golang.org/x/tools/gopls@latest && echo "  ✅ gopls installed" || echo "  ⚠️  Failed to install gopls"; \
		fi; \
	else \
		echo "  ⚠️  go not found, skipping Go LSP server"; \
		echo "     Install Go from: https://go.dev/"; \
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

# Code quality targets
clippy:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all --check

audit:
	@echo "🔒 Running security audit..."
	cargo audit

check: fmt clippy test audit

check-duplicates:
	@./scripts/check-duplicates.sh

# Development watch mode - auto-rebuild on file changes
dev:
	@command -v cargo-watch >/dev/null 2>&1 || { echo "⚠️  cargo-watch not found. Run 'make setup' first."; exit 1; }
	@echo "🚀 Starting development watch mode..."
	cargo watch -x 'build --release'

# Alias for dev
watch: dev

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

# First-time developer setup workflow
first-time-setup:
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║  🚀 First-Time Developer Setup for Codebuddy            ║"
	@echo "║  This will install tools and build the project (~5min)  ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "📋 Step 1/5: Checking parser build dependencies..."
	@make check-parser-deps
	@echo ""
	@echo "🔧 Step 2/5: Installing Rust development tools..."
	@make setup
	@echo ""
	@echo "🔨 Step 3/5: Building external language parsers..."
	@make build-parsers
	@echo ""
	@echo "🏗️  Step 4/5: Building main Rust project (this may take a few minutes)..."
	@make build
	@echo ""
	@echo "🔍 Step 5/5: Validating installation..."
	@make validate-setup
	@echo ""
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║  ✅ Setup Complete! Development Environment Ready       ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "📝 Next Steps:"
	@echo "  1. Configure LSP servers:  codebuddy setup"
	@echo "  2. Verify everything works: make test"
	@echo "  3. Start developing:        cargo build"
	@echo ""
	@echo "📚 Documentation:"
	@echo "  • Development workflow:  CONTRIBUTING.md"
	@echo "  • Project structure:     docs/architecture/ARCHITECTURE.md"
	@echo "  • Tool reference:        API_REFERENCE.md"
	@echo ""
	@echo "💡 Quick commands:"
	@echo "  make test        - Run fast tests (~10s)"
	@echo "  make dev         - Auto-rebuild on file changes"
	@echo "  make help        - Show all available commands"

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
	@command -v pylsp >/dev/null 2>&1 && echo "  ✅ pylsp" || echo "  ⚠️  pylsp not installed (run: make install-lsp-servers)"
	@command -v gopls >/dev/null 2>&1 && echo "  ✅ gopls" || echo "  ⚠️  gopls not installed (run: make install-lsp-servers)"
	@command -v rust-analyzer >/dev/null 2>&1 && echo "  ✅ rust-analyzer" || echo "  ⚠️  rust-analyzer not installed (run: make install-lsp-servers)"
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
	@echo "  make dev               - Build in watch mode (auto-rebuild on changes)"
	@echo "  make setup             - Quick setup: cargo-nextest only (~30s)"
	@echo "  make setup-full        - Full setup: sccache, mold, cargo-watch, etc. (~2-3min)"
	@echo "  make install-lsp-servers - Install LSP servers for testing"
	@echo ""
	@echo "✅ Testing (uses cargo-nextest):"
	@echo "  make test              - Run fast tests (~10s, auto-installs cargo-nextest)"
	@echo "  make test-lsp          - Run tests requiring LSP servers (~60s)"
	@echo "  make test-full         - Run the entire test suite, including skipped tests (~80s)"
	@echo ""
	@echo "🧹 Cleanup:"
	@echo "  make clean             - Remove build artifacts"
	@echo "  make clean-cache       - Remove all build artifacts (frees ~30-40GB)"
	@echo ""
	@echo "🔍 Code Quality & Validation:"
	@echo "  make clippy            - Run clippy linter"
	@echo "  make fmt               - Check code formatting"
	@echo "  make audit             - Run security audit (cargo-audit)"
	@echo "  make check             - Run fmt + clippy + test + audit"
	@echo "  make check-duplicates  - Detect duplicate code & complexity"
	@echo "  make validate-setup    - Check if your dev environment is set up correctly"
	@echo "  make ci                - Run all CI checks (for CI/CD)"
	@echo ""
	@echo "🔧 Language Parsers:"
	@echo "  make build-parsers     - Build all external language parsers"
	@echo "  make check-parser-deps - Check parser build dependencies"
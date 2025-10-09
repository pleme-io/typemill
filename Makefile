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

# Run all tests (legacy)
test:
	@echo "Running legacy test command. For faster tests, use 'make test-fast' or 'make test-full'."
	cargo test --workspace

# New, faster test targets
test-fast:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo nextest run --workspace

test-full:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo nextest run --workspace --all-features -- --include-ignored

test-lsp:
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "⚠️  cargo-nextest not found. Run 'make setup' first."; exit 1; }
	cargo nextest run --workspace --features lsp-tests -- --include-ignored

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

# One-time developer setup (installs sccache, cargo-watch, and cargo-nextest)
setup:
	@echo "📦 Installing build optimization tools..."
	@cargo install sccache 2>/dev/null || echo "✓ sccache already installed"
	@cargo install cargo-watch 2>/dev/null || echo "✓ cargo-watch already installed"
	@cargo install cargo-nextest 2>/dev/null || echo "✓ cargo-nextest already installed"
	@./scripts/setup-dev-tools.sh
	@echo "✅ Setup complete!"

# Code quality targets
clippy:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all --check

audit:
	@echo "🔒 Running security audit..."
	cargo audit

check: fmt clippy test-fast audit

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
	@echo "=== 🚀 First-Time Developer Setup ==="
	@make check-parser-deps
	@make setup
	@make build-parsers
	@make build
	@echo "✅ Setup complete! Next steps:"
	@echo "  1. Run 'codebuddy setup' to configure language servers."
	@echo "  2. Run 'make validate-setup' to verify your environment."

# Validate that the development environment is correctly configured
validate-setup:
	@echo "🕵️  Validating setup..."
	@make check-parser-deps
	@if [ -f "target/debug/codebuddy" ]; then \
		echo "  ✅ Main binary found."; \
	else \
		echo "  ❌ Main binary not found. Please run 'make build'."; \
	fi
	@echo "✅ Validation complete."

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
	@echo "  make setup             - Install build optimization tools (sccache, cargo-watch, cargo-nextest)"
	@echo ""
	@echo "✅ Testing:"
	@echo "  make test-fast         - Run fast tests (~10s, recommended for local dev)"
	@echo "  make test-lsp          - Run tests requiring LSP servers (~60s)"
	@echo "  make test-full         - Run the entire test suite (~80s)"
	@echo "  make test              - Run all tests with default cargo test (legacy)"
	@echo ""
	@echo "🧹 Cleanup:"
	@echo "  make clean             - Remove build artifacts"
	@echo "  make clean-cache       - Remove all build artifacts (frees ~30-40GB)"
	@echo ""
	@echo "🔍 Code Quality & Validation:"
	@echo "  make clippy            - Run clippy linter"
	@echo "  make fmt               - Check code formatting"
	@echo "  make audit             - Run security audit (cargo-audit)"
	@echo "  make check             - Run fmt + clippy + test-fast + audit"
	@echo "  make check-duplicates  - Detect duplicate code & complexity"
	@echo "  make validate-setup    - Check if your dev environment is set up correctly"
	@echo "  make ci                - Run all CI checks (for CI/CD)"
	@echo ""
	@echo "🔧 Language Parsers:"
	@echo "  make build-parsers     - Build all external language parsers"
	@echo "  make check-parser-deps - Check parser build dependencies"
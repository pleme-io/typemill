# CodeBuddy Makefile
# Simple build automation for common development tasks

.PHONY: build release test test-fast test-full test-lsp install uninstall clean clean-cache setup help clippy fmt audit check check-duplicates dev watch ci

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

# Show available commands
help:
	@echo "CodeBuddy - Available Commands"
	@echo "================================"
	@echo ""
	@echo "🔨 Build & Install:"
	@echo "  make build    - Build debug version"
	@echo "  make release  - Build optimized release version"
	@echo "  make install  - Install to ~/.local/bin (auto-configures PATH)"
	@echo "  make uninstall- Remove installed binary"
	@echo ""
	@echo "🚀 Development:"
	@echo "  make dev      - Build in watch mode (auto-rebuild on changes)"
	@echo "  make setup    - Install build optimization tools (sccache, cargo-watch, cargo-nextest)"
	@echo ""
	@echo "✅ Testing:"
	@echo "  make test-fast  - Run fast tests (~10s, recommended for local dev)"
	@echo "  make test-lsp   - Run tests requiring LSP servers (~60s)"
	@echo "  make test-full  - Run the entire test suite (~80s)"
	@echo "  make test       - Run all tests with default cargo test (legacy)"
	@echo ""
	@echo "🧹 Cleanup:"
	@echo "  make clean      - Remove build artifacts"
	@echo "  make clean-cache- Remove all build artifacts (frees ~30-40GB)"
	@echo ""
	@echo "🔍 Code Quality:"
	@echo "  make clippy   - Run clippy linter"
	@echo "  make fmt      - Check code formatting"
	@echo "  make audit    - Run security audit (cargo-audit)"
	@echo "  make check    - Run fmt + clippy + test-fast + audit"
	@echo "  make check-duplicates - Detect duplicate code & complexity"
	@echo "  make ci       - Run all CI checks (for CI/CD)"
	@echo ""
	@echo "💡 Quick Start:"
	@echo "  make setup      # First time only - install dev tools"
	@echo "  make dev        # Develop with auto-rebuild"
	@echo "  make test-fast  # Run quick tests before committing"
	@echo "  make install    # Deploy to system"
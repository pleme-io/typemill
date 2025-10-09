#!/bin/bash
# Post-create script for dev container setup
set -e

echo "🚀 Setting up Codebuddy development environment..."
echo ""

# Install cargo-nextest, sccache, and other dev tools
echo "📦 Installing Rust development tools..."
cargo install cargo-nextest --locked 2>/dev/null || echo "✓ cargo-nextest already installed"
cargo install sccache --locked 2>/dev/null || echo "✓ sccache already installed"
cargo install cargo-watch --locked 2>/dev/null || echo "✓ cargo-watch already installed"

# Install language servers for testing
echo ""
echo "🔧 Installing LSP servers..."

# TypeScript/JavaScript
if ! command -v typescript-language-server &> /dev/null; then
    echo "  → Installing typescript-language-server..."
    npm install -g typescript-language-server typescript
else
    echo "  ✓ typescript-language-server already installed"
fi

# Python
if ! command -v pylsp &> /dev/null; then
    echo "  → Installing python-lsp-server..."
    pip install --user "python-lsp-server[all]"
else
    echo "  ✓ pylsp already installed"
fi

# Go
if ! command -v gopls &> /dev/null; then
    echo "  → Installing gopls..."
    go install golang.org/x/tools/gopls@latest
else
    echo "  ✓ gopls already installed"
fi

# Rust (should already be installed via rustup)
if ! command -v rust-analyzer &> /dev/null; then
    echo "  → Installing rust-analyzer..."
    rustup component add rust-analyzer
else
    echo "  ✓ rust-analyzer already installed"
fi

# Build parsers
echo ""
echo "🔨 Building external language parsers..."
make check-parser-deps
make build-parsers

# Initial build (cached for faster subsequent builds)
echo ""
echo "🏗️  Running initial build (this may take a few minutes)..."
cargo build

# Run tests to verify everything works
echo ""
echo "✅ Running quick test suite to verify setup..."
cargo nextest run --workspace --no-fail-fast || {
    echo "⚠️  Some tests failed, but the environment is ready for development"
}

# Create default config
echo ""
echo "📝 Creating default configuration..."
mkdir -p .codebuddy
if [ ! -f .codebuddy/config.json ]; then
    cargo run -- setup
fi

echo ""
echo "✨ Development environment ready!"
echo ""
echo "Quick start:"
echo "  • Build: cargo build"
echo "  • Test:  make test"
echo "  • Run:   cargo run -- start"
echo ""
echo "See CONTRIBUTING.md for development workflow"

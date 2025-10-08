#!/bin/bash
# Start codebuddy with LSP servers in PATH

export PATH="$HOME/.cargo/bin:$HOME/.nvm/versions/node/v22.20.0/bin:$PATH"

# Stop any existing codebuddy instance
./target/release/codebuddy stop 2>/dev/null || true

# Start codebuddy server
./target/release/codebuddy start

echo "✅ LSP servers ready in PATH"
echo "   - rust-analyzer: $(which rust-analyzer 2>/dev/null || echo 'not found')"
echo "   - typescript-language-server: $(which typescript-language-server 2>/dev/null || echo 'not found')"

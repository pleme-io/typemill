#!/bin/bash
set -e

# Codeflow Buddy MCP Server Installer
# Builds and installs from local source

BINARY_NAME="codebuddy"
INSTALL_DIR="/usr/local/bin"
SOURCE_BINARY="/workspace/rust/target/release/cb-server"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warning() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

# Build the Rust project
build_project() {
    log_info "Building Rust project..."
    cd /workspace/rust
    cargo build --release --package cb-server

    if [ ! -f "$SOURCE_BINARY" ]; then
        log_error "Build failed - binary not found at $SOURCE_BINARY"
        exit 1
    fi
    log_success "Build completed successfully"
}

# Install binary
install_binary() {
    log_info "Installing binary to $INSTALL_DIR/$BINARY_NAME..."

    # Use sudo if needed for /usr/local/bin
    if [ -w "$INSTALL_DIR" ]; then
        cp "$SOURCE_BINARY" "$INSTALL_DIR/$BINARY_NAME"
    else
        sudo cp "$SOURCE_BINARY" "$INSTALL_DIR/$BINARY_NAME"
    fi

    if [ -w "$INSTALL_DIR" ]; then
        chmod +x "$INSTALL_DIR/$BINARY_NAME"
    else
        sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"
    fi
    log_success "Binary installed to $INSTALL_DIR/$BINARY_NAME"
}

# Setup MCP configuration in project
setup_project_mcp() {
    local project_dir="${1:-$(pwd)}"
    local mcp_config="$project_dir/.mcp.json"

    log_info "Setting up MCP configuration in $project_dir..."

    # Create or update MCP config with correct format
    cat > "$mcp_config" << EOF
{
  "mcpServers": {
    "codebuddy": {
      "type": "stdio",
      "command": "$INSTALL_DIR/$BINARY_NAME",
      "args": [
        "start"
      ],
      "env": {}
    }
  }
}
EOF

    log_success "MCP configuration created at $mcp_config"
}

# Setup LSP configuration
setup_lsp_config() {
    local project_dir="${1:-$(pwd)}"
    local config_dir="$project_dir/.codebuddy"
    local lsp_config="$config_dir/config.json"

    log_info "Setting up LSP configuration..."

    # Create config directory
    mkdir -p "$config_dir"

    # Detect project type and create appropriate config
    local has_ts=false
    local has_py=false
    local has_go=false
    local has_rs=false

    # Check for file types
    [ -n "$(find "$project_dir" -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" 2>/dev/null | head -1)" ] && has_ts=true
    [ -n "$(find "$project_dir" -name "*.py" 2>/dev/null | head -1)" ] && has_py=true
    [ -n "$(find "$project_dir" -name "*.go" 2>/dev/null | head -1)" ] && has_go=true
    [ -n "$(find "$project_dir" -name "*.rs" 2>/dev/null | head -1)" ] && has_rs=true

    # Start building config
    echo '{"servers":[' > "$lsp_config"
    local first=true

    # Add TypeScript/JavaScript server if needed
    if [ "$has_ts" = true ] && command -v typescript-language-server >/dev/null 2>&1; then
        [ "$first" = false ] && echo "," >> "$lsp_config"
        cat >> "$lsp_config" << 'EOF'
    {
      "extensions": ["ts", "tsx", "js", "jsx"],
      "command": ["typescript-language-server", "--stdio"]
    }
EOF
        first=false
        log_success "Added TypeScript language server"
    fi

    # Add Python server if needed
    if [ "$has_py" = true ] && command -v pylsp >/dev/null 2>&1; then
        [ "$first" = false ] && echo "," >> "$lsp_config"
        cat >> "$lsp_config" << 'EOF'
    {
      "extensions": ["py", "pyi"],
      "command": ["pylsp"]
    }
EOF
        first=false
        log_success "Added Python language server"
    fi

    # Add Go server if needed
    if [ "$has_go" = true ] && command -v gopls >/dev/null 2>&1; then
        [ "$first" = false ] && echo "," >> "$lsp_config"
        cat >> "$lsp_config" << 'EOF'
    {
      "extensions": ["go"],
      "command": ["gopls", "serve"]
    }
EOF
        first=false
        log_success "Added Go language server"
    fi

    # Add Rust server if needed
    if [ "$has_rs" = true ] && command -v rust-analyzer >/dev/null 2>&1; then
        [ "$first" = false ] && echo "," >> "$lsp_config"
        cat >> "$lsp_config" << 'EOF'
    {
      "extensions": ["rs"],
      "command": ["rust-analyzer"]
    }
EOF
        first=false
        log_success "Added Rust language server"
    fi

    echo ']' >> "$lsp_config"
    echo '}' >> "$lsp_config"

    log_success "LSP configuration created at $lsp_config"
}

# Test the installation
test_installation() {
    log_info "Testing MCP server..."

    # Test with proper JSON-RPC 2.0 protocol
    local response=$(echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{"roots":{}},"clientInfo":{"name":"test","version":"1.0.0"}}}' | \
        "$INSTALL_DIR/$BINARY_NAME" start 2>/dev/null | head -1)

    if echo "$response" | grep -q '"jsonrpc":"2.0"' && echo "$response" | grep -q '"protocolVersion":"2025-06-18"'; then
        log_success "MCP server is responding correctly with JSON-RPC 2.0 protocol"
        return 0
    else
        log_error "MCP server test failed. Response: $response"
        return 1
    fi
}

# Main installation flow
main() {
    log_info "Installing Codeflow Buddy MCP Server (Local Build)..."

    # Check we're in the right place
    if [ ! -f "/workspace/rust/Cargo.toml" ]; then
        log_error "This script must be run from /workspace directory"
        exit 1
    fi

    # Build and install
    build_project
    install_binary

    # Setup configurations
    local project_dir="${1:-/workspace}"
    setup_project_mcp "$project_dir"
    setup_lsp_config "$project_dir"

    # Test the installation
    if test_installation; then
        echo ""
        log_success "🎉 Codeflow Buddy MCP Server installed successfully!"
        echo ""
        echo "The server is configured with:"
        echo "  • Protocol: JSON-RPC 2.0"
        echo "  • Version: 2025-06-18"
        echo "  • Location: $INSTALL_DIR/$BINARY_NAME"
        echo ""
        echo "Configuration files created:"
        echo "  • MCP: $project_dir/.mcp.json"
        echo "  • LSP: $project_dir/.codebuddy/config.json"
        echo ""
        echo "To use with Claude Code:"
        echo "  1. Open Claude Code in this project"
        echo "  2. Use the /mcp command to connect"
        echo ""
    else
        log_error "Installation completed but server test failed"
        echo "Please check the server logs for errors"
        exit 1
    fi
}

# Run installer
main "$@"
#!/bin/bash
set -e

# Codeflow Buddy MCP Server Installer
# Builds and installs from local source

BINARY_NAME="codebuddy"
SOURCE_BINARY="/workspace/rust/target/release/cb-server"

# Detect operating system
detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux" ;;
        Darwin*)    echo "macos" ;;
        *)          echo "unknown" ;;
    esac
}

OS_TYPE=$(detect_os)

# Set install directory based on OS
if [ "$OS_TYPE" = "macos" ]; then
    # macOS: prefer /usr/local/bin (Homebrew standard)
    INSTALL_DIR="/usr/local/bin"
    # Check if we need sudo for /usr/local/bin
    if [ -w "$INSTALL_DIR" ]; then
        SUDO_CMD=""
    else
        SUDO_CMD="sudo"
    fi
else
    # Linux: always use /usr/local/bin with sudo
    INSTALL_DIR="/usr/local/bin"
    SUDO_CMD="sudo"
fi

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

# Check for unsupported OS
if [ "$OS_TYPE" = "unknown" ]; then
    log_error "Unsupported operating system: $(uname -s)"
    log_error "This installer supports Linux and macOS only"
    exit 1
fi

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

    # Use appropriate command based on permissions
    if [ -n "$SUDO_CMD" ]; then
        $SUDO_CMD cp "$SOURCE_BINARY" "$INSTALL_DIR/$BINARY_NAME"
        $SUDO_CMD chmod +x "$INSTALL_DIR/$BINARY_NAME"
    else
        cp "$SOURCE_BINARY" "$INSTALL_DIR/$BINARY_NAME"
        chmod +x "$INSTALL_DIR/$BINARY_NAME"
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

# Install system dependencies required for building
install_system_dependencies() {
    log_info "Installing system dependencies for $OS_TYPE..."

    if [ "$OS_TYPE" = "macos" ]; then
        install_macos_dependencies
    else
        install_linux_dependencies
    fi
}

# Install macOS dependencies
install_macos_dependencies() {
    # Check for Homebrew
    if ! command -v brew >/dev/null 2>&1; then
        log_error "Homebrew is not installed"
        log_info "Please install Homebrew first:"
        log_info "/bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
        exit 1
    fi

    log_info "Checking macOS dependencies..."
    local missing_packages=""

    # Check for required packages
    for pkg in pkg-config; do
        if ! brew list $pkg >/dev/null 2>&1; then
            missing_packages="$missing_packages $pkg"
        fi
    done

    # Check for macFUSE (for FUSE support on macOS)
    if ! brew list --cask macfuse >/dev/null 2>&1; then
        log_info "macFUSE is not installed (optional for FUSE support)"
        log_info "To install: brew install --cask macfuse"
    fi

    if [ -n "$missing_packages" ]; then
        log_info "Installing missing packages:$missing_packages"
        brew install$missing_packages
        log_success "macOS dependencies installed"
    else
        log_success "All macOS dependencies already installed"
    fi

    # Check for Xcode Command Line Tools
    if ! xcode-select -p >/dev/null 2>&1; then
        log_info "Installing Xcode Command Line Tools..."
        xcode-select --install
        log_warning "Please complete the Xcode installation and re-run this script"
        exit 1
    fi
}

# Install Linux dependencies
install_linux_dependencies() {
    # Detect Linux distribution
    if [ -f "/etc/os-release" ]; then
        . /etc/os-release
        OS_ID="$ID"
    else
        log_error "Cannot detect Linux distribution"
        exit 1
    fi

    # Define required packages for Ubuntu/Debian
    local packages="build-essential pkg-config libfuse-dev git curl python3-dev python3-pip"

    case "$OS_ID" in
        ubuntu|debian)
            # Check if packages are missing
            local missing_packages=""
            for pkg in $packages; do
                if ! dpkg -l | grep -q "^ii  $pkg "; then
                    missing_packages="$missing_packages $pkg"
                fi
            done

            if [ -n "$missing_packages" ]; then
                log_info "Installing missing system packages:$missing_packages"
                if sudo apt-get update && sudo apt-get install -y${missing_packages}; then
                    log_success "System dependencies installed successfully"
                else
                    log_error "Failed to install system dependencies"
                    log_error "Please run: sudo apt-get update && sudo apt-get install -y $packages"
                    exit 1
                fi
            else
                log_success "All system dependencies already installed"
            fi
            ;;
        fedora|rhel|centos)
            log_info "Installing packages for Fedora/RHEL/CentOS..."
            sudo dnf install -y gcc gcc-c++ make pkg-config fuse-devel git curl python3-devel python3-pip
            log_success "System dependencies installed"
            ;;
        arch|manjaro)
            log_info "Installing packages for Arch/Manjaro..."
            sudo pacman -Sy --needed base-devel pkg-config fuse2 git curl python python-pip
            log_success "System dependencies installed"
            ;;
        *)
            log_warning "Unsupported Linux distribution: $OS_ID"
            log_warning "Please ensure these packages are installed: $packages"
            ;;
    esac
}

# Install Rust toolchain if missing
install_rust() {
    log_info "Checking Rust installation..."

    if command -v rustc >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
        local rust_version=$(rustc --version)
        log_success "Rust already installed: $rust_version"
        return 0
    fi

    log_info "Installing Rust toolchain via package manager..."

    if [ "$OS_TYPE" = "macos" ]; then
        # macOS: Use Homebrew
        if command -v brew >/dev/null 2>&1; then
            brew install rust
            log_success "Rust installed via Homebrew"
        else
            log_error "Homebrew not found. Please install Homebrew first or install Rust manually"
            exit 1
        fi
    else
        # Linux: Use distribution package manager
        if [ -f "/etc/os-release" ]; then
            . /etc/os-release
            case "$ID" in
                ubuntu|debian)
                    # Ubuntu/Debian: Install via apt
                    sudo apt update
                    sudo apt install -y rustc cargo
                    log_success "Rust installed via apt"
                    ;;
                fedora|rhel|centos)
                    sudo dnf install -y rust cargo
                    log_success "Rust installed via dnf"
                    ;;
                arch|manjaro)
                    sudo pacman -S --needed rust
                    log_success "Rust installed via pacman"
                    ;;
                *)
                    log_warning "Unsupported Linux distribution. Falling back to rustup..."
                    # Fallback to rustup for unsupported distros
                    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                    if [ -f "$HOME/.cargo/env" ]; then
                        . "$HOME/.cargo/env"
                    fi
                    ;;
            esac
        else
            log_error "Cannot detect Linux distribution"
            exit 1
        fi
    fi
}

# Install Node.js and npm if missing
install_nodejs() {
    log_info "Checking Node.js installation..."

    if command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
        local node_version=$(node --version)
        local npm_version=$(npm --version)
        log_success "Node.js already installed: $node_version (npm: $npm_version)"
        return 0
    fi

    log_info "Installing Node.js LTS via package manager..."

    if [ "$OS_TYPE" = "macos" ]; then
        # macOS: Use Homebrew
        if command -v brew >/dev/null 2>&1; then
            brew install node
            log_success "Node.js installed via Homebrew"
        else
            log_error "Homebrew not found. Please install Homebrew first or install Node.js manually"
            exit 1
        fi
    else
        # Linux: Use distribution package manager
        if [ -f "/etc/os-release" ]; then
            . /etc/os-release
            case "$ID" in
                ubuntu|debian)
                    # Ubuntu/Debian: Install from default repos (simpler and more secure)
                    sudo apt update
                    sudo apt install -y nodejs npm
                    log_success "Node.js installed via apt"
                    ;;
                fedora|rhel|centos)
                    sudo dnf install -y nodejs npm
                    log_success "Node.js installed via dnf"
                    ;;
                arch|manjaro)
                    sudo pacman -S --needed nodejs npm
                    log_success "Node.js installed via pacman"
                    ;;
                *)
                    log_warning "Unsupported Linux distribution: $ID"
                    log_warning "Please install Node.js manually from https://nodejs.org/"
                    ;;
            esac
        else
            log_error "Cannot detect Linux distribution"
            exit 1
        fi
    fi
}

# Install pipx if missing
install_pipx() {
    log_info "Checking pipx installation..."

    if command -v pipx >/dev/null 2>&1; then
        log_success "pipx already installed"
        return 0
    fi

    log_info "Installing pipx via package manager..."

    if [ "$OS_TYPE" = "macos" ]; then
        # macOS: Use Homebrew
        if command -v brew >/dev/null 2>&1; then
            brew install pipx
            pipx ensurepath
            log_success "pipx installed via Homebrew"
        else
            log_error "Homebrew not found. Please install Homebrew first or install pipx manually"
            exit 1
        fi
    else
        # Linux: Use distribution package manager
        if [ -f "/etc/os-release" ]; then
            . /etc/os-release
            case "$ID" in
                ubuntu|debian)
                    # Ubuntu/Debian: Try package manager first, fallback to pip
                    if sudo apt update && sudo apt install -y pipx; then
                        log_success "pipx installed via apt"
                    elif python3 -m pip install --user pipx; then
                        log_success "pipx installed via pip"
                    else
                        log_error "Failed to install pipx"
                        exit 1
                    fi
                    ;;
                fedora|rhel|centos)
                    if sudo dnf install -y pipx; then
                        log_success "pipx installed via dnf"
                    elif python3 -m pip install --user pipx; then
                        log_success "pipx installed via pip"
                    else
                        log_error "Failed to install pipx"
                        exit 1
                    fi
                    ;;
                arch|manjaro)
                    if sudo pacman -S --needed python-pipx; then
                        log_success "pipx installed via pacman"
                    elif python3 -m pip install --user pipx; then
                        log_success "pipx installed via pip"
                    else
                        log_error "Failed to install pipx"
                        exit 1
                    fi
                    ;;
                *)
                    log_warning "Unsupported Linux distribution: $ID"
                    log_info "Trying pip installation..."
                    if python3 -m pip install --user pipx; then
                        log_success "pipx installed via pip"
                    else
                        log_error "Failed to install pipx"
                        exit 1
                    fi
                    ;;
            esac
        else
            log_error "Cannot detect Linux distribution"
            exit 1
        fi
    fi
}

# Ensure all tools are in PATH
ensure_tool_paths() {
    log_info "Setting up tool paths..."

    # Source Rust environment if available
    if [ -f "$HOME/.cargo/env" ]; then
        . "$HOME/.cargo/env"
    fi

    # Add user local bin to PATH for pipx
    if [ -d "$HOME/.local/bin" ]; then
        export PATH="$HOME/.local/bin:$PATH"
    fi

    # Persist PATH changes to shell configuration
    local shell_rc=""
    if [ "$OS_TYPE" = "macos" ]; then
        # macOS: check for .zprofile (default for macOS Catalina+) or .bash_profile
        if [ -n "$ZSH_VERSION" ] || [ -f "$HOME/.zprofile" ]; then
            shell_rc="$HOME/.zprofile"
        elif [ -n "$BASH_VERSION" ] || [ -f "$HOME/.bash_profile" ]; then
            shell_rc="$HOME/.bash_profile"
        else
            shell_rc="$HOME/.profile"
        fi
    else
        # Linux: use .bashrc or .zshrc
        if [ -n "$BASH_VERSION" ]; then
            shell_rc="$HOME/.bashrc"
        elif [ -n "$ZSH_VERSION" ]; then
            shell_rc="$HOME/.zshrc"
        else
            shell_rc="$HOME/.profile"
        fi
    fi

    # Add PATH export if not already present
    if [ -d "$HOME/.local/bin" ] && ! grep -q "export PATH.*/.local/bin" "$shell_rc" 2>/dev/null; then
        echo "" >> "$shell_rc"
        echo "# Added by Codebuddy installer" >> "$shell_rc"
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$shell_rc"
        log_info "Added ~/.local/bin to PATH in $shell_rc"
        log_warning "Please run 'source $shell_rc' or restart your shell for PATH changes to take effect"
    fi

    # Verify critical tools are now available
    local missing_tools=""
    for tool in rustc cargo; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing_tools="$missing_tools $tool"
        fi
    done

    if [ -n "$missing_tools" ]; then
        log_error "Critical tools still missing after installation:$missing_tools"
        log_error "Please check installation logs and PATH configuration"
        exit 1
    fi

    log_success "All development tools available in PATH"
}

# Install language servers for development
install_language_servers() {
    log_info "Installing development language servers..."

    # Install TypeScript language server
    if command -v npm >/dev/null 2>&1; then
        log_info "Installing TypeScript language server..."
        npm install -g typescript-language-server typescript
        log_success "TypeScript language server installed"
    else
        log_warning "npm not found - skipping TypeScript language server"
    fi

    # Install Python language server
    if command -v pipx >/dev/null 2>&1; then
        log_info "Installing Python language server..."
        pipx install "python-lsp-server[all]"

        # Verify pylsp is accessible
        if [ -f "$HOME/.local/bin/pylsp" ]; then
            export PATH="$HOME/.local/bin:$PATH"
            log_success "Python language server installed at ~/.local/bin/pylsp"
        elif command -v pylsp >/dev/null 2>&1; then
            log_success "Python language server installed and available in PATH"
        else
            log_warning "Python LSP installed but not found in PATH - you may need to add ~/.local/bin to your PATH"
        fi
    elif command -v pip >/dev/null 2>&1; then
        log_info "Installing Python language server with pip..."
        pip install --user "python-lsp-server[all]" || pip install --break-system-packages "python-lsp-server[all]"
        log_success "Python language server installed"
    else
        log_warning "pip/pipx not found - skipping Python language server"
    fi

    # Install Go language server
    if command -v go >/dev/null 2>&1; then
        log_info "Installing Go language server..."
        go install golang.org/x/tools/gopls@latest
        log_success "Go language server installed"
    else
        log_warning "go not found - skipping Go language server"
    fi

    # Install Rust language server (rust-analyzer)
    if command -v rustup >/dev/null 2>&1; then
        log_info "Installing Rust language server..."
        rustup component add rust-analyzer
        log_success "Rust language server installed"
    else
        log_warning "rustup not found - skipping Rust language server"
    fi
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

    # Check for file types (more comprehensive search for development)
    [ -n "$(find "$project_dir" -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" -o -name "package.json" -o -name "tsconfig.json" 2>/dev/null | head -1)" ] && has_ts=true
    [ -n "$(find "$project_dir" -name "*.py" -o -name "requirements.txt" -o -name "pyproject.toml" 2>/dev/null | head -1)" ] && has_py=true
    [ -n "$(find "$project_dir" -name "*.go" -o -name "go.mod" 2>/dev/null | head -1)" ] && has_go=true
    [ -n "$(find "$project_dir" -name "*.rs" -o -name "Cargo.toml" 2>/dev/null | head -1)" ] && has_rs=true

    # Start building config
    echo '{"servers":[' > "$lsp_config"
    local first=true

    # Add TypeScript/JavaScript server
    if [ "$has_ts" = true ]; then
        [ "$first" = false ] && echo "," >> "$lsp_config"
        cat >> "$lsp_config" << 'EOF'
    {
      "extensions": ["ts", "tsx", "js", "jsx"],
      "command": ["typescript-language-server", "--stdio"]
    }
EOF
        first=false
        log_success "Added TypeScript language server to config"
    fi

    # Add Python server
    if [ "$has_py" = true ]; then
        [ "$first" = false ] && echo "," >> "$lsp_config"
        # Dynamically detect pylsp path
        local pylsp_path=""
        if [ -f "$HOME/.local/bin/pylsp" ]; then
            pylsp_path="$HOME/.local/bin/pylsp"
        elif command -v pylsp >/dev/null 2>&1; then
            pylsp_path=$(command -v pylsp)
        else
            pylsp_path="pylsp"  # Fallback to hoping it's in PATH
        fi

        cat >> "$lsp_config" << EOF
    {
      "extensions": ["py", "pyi"],
      "command": ["$pylsp_path"]
    }
EOF
        first=false
        log_success "Added Python language server to config (using: $pylsp_path)"
    fi

    # Add Go server
    if [ "$has_go" = true ]; then
        [ "$first" = false ] && echo "," >> "$lsp_config"
        cat >> "$lsp_config" << 'EOF'
    {
      "extensions": ["go"],
      "command": ["gopls", "serve"]
    }
EOF
        first=false
        log_success "Added Go language server to config"
    fi

    # Add Rust server
    if [ "$has_rs" = true ]; then
        [ "$first" = false ] && echo "," >> "$lsp_config"
        cat >> "$lsp_config" << 'EOF'
    {
      "extensions": ["rs"],
      "command": ["rust-analyzer"]
    }
EOF
        first=false
        log_success "Added Rust language server to config"
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
    log_info "Installing Codebuddy MCP Server on $OS_TYPE with Complete Development Environment..."

    # Check we're in the right place
    if [ ! -f "/workspace/rust/Cargo.toml" ]; then
        log_error "This script must be run from /workspace directory"
        exit 1
    fi

    # Phase 1: Install prerequisites
    log_info "=== Phase 1: Installing Prerequisites ==="
    install_system_dependencies
    install_rust
    install_nodejs
    install_pipx
    ensure_tool_paths

    # Phase 2: Install language servers
    log_info "=== Phase 2: Installing Language Servers ==="
    install_language_servers

    # Verify LSP servers are accessible
    log_info "=== Verifying Language Server Installation ==="
    local lsp_status=""

    # Check TypeScript LSP
    if command -v typescript-language-server >/dev/null 2>&1; then
        lsp_status="${lsp_status}✓ TypeScript LSP: $(command -v typescript-language-server)\n"
    else
        lsp_status="${lsp_status}✗ TypeScript LSP: Not found in PATH\n"
    fi

    # Check Python LSP (with expanded PATH)
    export PATH="$HOME/.local/bin:$PATH"
    if command -v pylsp >/dev/null 2>&1; then
        lsp_status="${lsp_status}✓ Python LSP: $(command -v pylsp)\n"
    else
        lsp_status="${lsp_status}✗ Python LSP: Not found in PATH\n"
    fi

    # Check Rust analyzer
    if command -v rust-analyzer >/dev/null 2>&1; then
        lsp_status="${lsp_status}✓ Rust analyzer: $(command -v rust-analyzer)\n"
    else
        lsp_status="${lsp_status}✗ Rust analyzer: Not found in PATH\n"
    fi

    # Check Go LSP
    if command -v gopls >/dev/null 2>&1; then
        lsp_status="${lsp_status}✓ Go LSP: $(command -v gopls)\n"
    else
        lsp_status="${lsp_status}✗ Go LSP: Not found (optional)\n"
    fi

    echo -e "\nLanguage Server Status:\n$lsp_status"

    # Phase 3: Build and install
    log_info "=== Phase 3: Building and Installing Codebuddy ==="
    build_project
    install_binary

    # Phase 4: Setup configurations
    log_info "=== Phase 4: Setting up Configurations ==="
    local project_dir="${1:-/workspace}"
    setup_project_mcp "$project_dir"
    setup_lsp_config "$project_dir"

    # Test the installation
    if test_installation; then
        echo ""
        log_success "🎉 Codebuddy Complete Development Environment installed successfully!"
        echo ""
        echo "✅ INSTALLED COMPONENTS:"
        if [ "$OS_TYPE" = "macos" ]; then
            echo "  • System Dependencies: Xcode CLI Tools, pkg-config (via Homebrew)"
        else
            echo "  • System Dependencies: build-essential, pkg-config, libfuse-dev, git, curl"
        fi
        echo "  • Rust Toolchain: rustc, cargo, rust-analyzer"
        echo "  • Node.js: node, npm, typescript-language-server"
        echo "  • Python Tools: pipx, python-lsp-server"
        echo "  • Codebuddy Server: $INSTALL_DIR/$BINARY_NAME"
        echo ""
        echo "🔧 SERVER CONFIGURATION:"
        echo "  • Protocol: JSON-RPC 2.0"
        echo "  • Version: 2025-06-18"
        echo "  • Location: $INSTALL_DIR/$BINARY_NAME"
        echo ""
        echo "📁 CONFIGURATION FILES:"
        echo "  • MCP: $project_dir/.mcp.json"
        echo "  • LSP: $project_dir/.codebuddy/config.json"
        echo ""
        echo "🚀 GETTING STARTED:"
        echo "  1. Open Claude Code in this project"
        echo "  2. Use the /mcp command to connect"
        echo "  3. All language servers are pre-configured and ready!"
        echo ""
    else
        log_error "Installation completed but server test failed"
        echo "Please check the server logs for errors"
        exit 1
    fi
}

# Run installer
main "$@"
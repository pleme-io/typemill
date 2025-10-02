#!/bin/bash
set -e

# Development Tools Setup Script for CodeBuddy
# Installs and configures build optimization tools: sccache and mold
#
# This script is automatically referenced by rust/.cargo/config.toml
# Run this script once per machine to speed up Rust compilation significantly
#
# Usage:
#   ./scripts/setup-dev-tools.sh

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

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux" ;;
        Darwin*)    echo "macos" ;;
        *)          echo "unknown" ;;
    esac
}

OS_TYPE=$(detect_os)

# Check if running in a supported OS
if [ "$OS_TYPE" = "unknown" ]; then
    log_error "Unsupported operating system: $(uname -s)"
    log_error "This script supports Linux and macOS only"
    exit 1
fi

log_info "Setting up development tools for CodeBuddy on $OS_TYPE..."

# =============================================================================
# Install sccache (Compilation Cache)
# =============================================================================

install_sscache() {
    log_info "Checking sccache installation..."

    if command -v sccache >/dev/null 2>&1; then
        local version=$(sccache --version)
        log_success "sccache already installed: $version"
        return 0
    fi

    log_info "Installing sccache..."

    if [ "$OS_TYPE" = "macos" ]; then
        # macOS: Install via Homebrew
        if command -v brew >/dev/null 2>&1; then
            brew install sccache
            log_success "sccache installed via Homebrew"
        else
            log_warning "Homebrew not found, installing via cargo..."
            cargo install sccache
            log_success "sccache installed via cargo"
        fi
    else
        # Linux: Check for system package first, fallback to cargo
        if command -v apt-get >/dev/null 2>&1; then
            # Ubuntu/Debian - install dependencies first
            log_info "Installing OpenSSL development packages..."
            sudo apt-get update && sudo apt-get install -y libssl-dev pkg-config

            log_info "Installing sccache via cargo..."
            cargo install sccache
            log_success "sccache installed via cargo"
        elif command -v dnf >/dev/null 2>&1; then
            # Fedora/RHEL
            log_info "Installing OpenSSL development packages..."
            sudo dnf install -y openssl-devel pkg-config

            cargo install sccache
            log_success "sccache installed via cargo"
        elif command -v pacman >/dev/null 2>&1; then
            # Arch Linux
            sudo pacman -S --needed sccache
            log_success "sccache installed via pacman"
        else
            # Fallback to cargo
            log_info "Installing sccache via cargo..."
            cargo install sccache
            log_success "sccache installed via cargo"
        fi
    fi
}

# =============================================================================
# Install mold (Fast Linker)
# =============================================================================

install_mold() {
    log_info "Checking mold installation..."

    if command -v mold >/dev/null 2>&1; then
        local version=$(mold --version)
        log_success "mold already installed: $version"
        return 0
    fi

    log_info "Installing mold linker..."

    if [ "$OS_TYPE" = "macos" ]; then
        # macOS: Install via Homebrew
        if command -v brew >/dev/null 2>&1; then
            brew install mold
            log_success "mold installed via Homebrew"
        else
            log_error "Homebrew not found. Please install Homebrew first:"
            log_error "  /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            exit 1
        fi
    else
        # Linux: Install via package manager
        if command -v apt-get >/dev/null 2>&1; then
            # Ubuntu/Debian - also need clang for mold
            sudo apt-get update && sudo apt-get install -y mold clang
            log_success "mold and clang installed via apt"
        elif command -v dnf >/dev/null 2>&1; then
            # Fedora/RHEL
            sudo dnf install -y mold clang
            log_success "mold and clang installed via dnf"
        elif command -v pacman >/dev/null 2>&1; then
            # Arch Linux
            sudo pacman -S --needed mold clang
            log_success "mold and clang installed via pacman"
        else
            log_error "Package manager not detected. Please install mold manually:"
            log_error "  https://github.com/rui314/mold#installation"
            exit 1
        fi
    fi
}

# =============================================================================
# Verify Configuration
# =============================================================================

verify_configuration() {
    log_info "Verifying Cargo configuration..."

    local cargo_config="/workspace/rust/.cargo/config.toml"

    if [ ! -f "$cargo_config" ]; then
        log_error "Cargo configuration not found at $cargo_config"
        log_error "Please ensure you're in the codebuddy project root"
        exit 1
    fi

    # Check if sccache is configured
    if grep -q 'rustc-wrapper.*sccache' "$cargo_config"; then
        log_success "sccache is configured in Cargo"
    else
        log_warning "sccache is not configured in Cargo config"
        log_info "Add to $cargo_config:"
        log_info '  [build]'
        log_info '  rustc-wrapper = "sccache"'
    fi

    # Check if mold is configured
    if grep -q 'fuse-ld=mold' "$cargo_config"; then
        log_success "mold linker is configured in Cargo"
    else
        log_warning "mold is not configured in Cargo config"
        log_info "See $cargo_config for platform-specific configuration"
    fi
}

# =============================================================================
# Test Build Performance
# =============================================================================

test_build() {
    log_info "Testing build with optimizations..."

    cd /workspace/rust

    # Show sccache statistics before
    log_info "sccache stats before build:"
    sccache --show-stats

    # Clean build to test
    log_info "Running test build..."
    cargo check --quiet 2>&1 | tail -5

    log_success "Build completed successfully!"

    # Show sccache statistics after
    log_info "sccache stats after build:"
    sccache --show-stats
}

# =============================================================================
# Main Installation Flow
# =============================================================================

main() {
    echo ""
    log_info "🚀 CodeBuddy Development Tools Setup"
    echo ""

    # Check prerequisites
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Rust/Cargo is not installed"
        log_error "Please run ./install.sh first to install the complete development environment"
        exit 1
    fi

    # Install tools
    log_info "=== Installing Build Optimization Tools ==="
    install_sscache
    install_mold

    # Verify configuration
    log_info "=== Verifying Configuration ==="
    verify_configuration

    # Test build (optional)
    if [ "${SKIP_TEST_BUILD:-0}" != "1" ]; then
        log_info "=== Testing Build ==="
        test_build
    fi

    echo ""
    log_success "✅ Development tools setup complete!"
    echo ""
    echo "📊 EXPECTED IMPROVEMENTS:"
    echo "  • Incremental builds: 2-5x faster (via sccache)"
    echo "  • Link times: 3-10x faster (via mold)"
    echo "  • Clean builds: Cached across git branches"
    echo ""
    echo "🔧 INSTALLED TOOLS:"
    echo "  • sccache: $(which sccache)"
    echo "  • mold: $(which mold)"
    if [ "$OS_TYPE" = "linux" ]; then
        echo "  • clang: $(which clang)"
    fi
    echo ""
    echo "💡 TIPS:"
    echo "  • sccache stats: sccache --show-stats"
    echo "  • Clear cache: sccache --zero-stats"
    echo "  • Team members should run this script on their machines"
    echo ""
    echo "📖 Configuration: rust/.cargo/config.toml"
    echo ""
}

# Run main installation
main "$@"

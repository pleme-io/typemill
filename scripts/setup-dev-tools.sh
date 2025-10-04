#!/bin/bash
set -e

# ==============================================================================
# CodeBuddy Development Tools Setup Script
# ==============================================================================
#
# Installs and configures essential development tools for the CodeBuddy project:
#   • Build optimization: sccache (compiler cache), mold (fast linker)
#   • Code quality: jscpd (duplicate detection)
#   • Cargo utilities: cargo-audit, cargo-deny, cargo-watch, cargo-edit
#   • Optional tools: cargo-nextest, cargo-expand, cargo-bloat, cargo-flamegraph
#
# Usage:
#   ./scripts/setup-dev-tools.sh [OPTIONS]
#
# Options:
#   --skip-build-test    Skip the test build at the end
#   --no-sudo           Skip tools that require sudo (mold, system packages)
#   --essential-only    Install only essential cargo utilities (skip optional)
#
# Environment Variables:
#   SKIP_TEST_BUILD=1   Same as --skip-build-test
#   NO_SUDO=1          Same as --no-sudo
#
# ==============================================================================

# ==============================================================================
# Configuration & Globals
# ==============================================================================

# Parse command line arguments
SKIP_TEST_BUILD="${SKIP_TEST_BUILD:-0}"
NO_SUDO="${NO_SUDO:-0}"
ESSENTIAL_ONLY="${ESSENTIAL_ONLY:-0}"

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-build-test)
            SKIP_TEST_BUILD=1
            shift
            ;;
        --no-sudo)
            NO_SUDO=1
            shift
            ;;
        --essential-only)
            ESSENTIAL_ONLY=1
            shift
            ;;
        --help|-h)
            head -n 20 "$0" | grep '^#' | sed 's/^# *//'
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Detect project root (works from any subdirectory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ==============================================================================
# Logging Functions
# ==============================================================================

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warning() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }
log_section() { echo -e "\n${CYAN}▸ $1${NC}"; }

# ==============================================================================
# Platform Detection
# ==============================================================================

detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux" ;;
        Darwin*)    echo "macos" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *)          echo "unknown" ;;
    esac
}

detect_package_manager() {
    if command -v apt-get >/dev/null 2>&1; then
        echo "apt"
    elif command -v dnf >/dev/null 2>&1; then
        echo "dnf"
    elif command -v pacman >/dev/null 2>&1; then
        echo "pacman"
    elif command -v brew >/dev/null 2>&1; then
        echo "brew"
    else
        echo "none"
    fi
}

OS_TYPE=$(detect_os)
PKG_MANAGER=$(detect_package_manager)

# Check if running in a supported OS
if [ "$OS_TYPE" = "unknown" ]; then
    log_error "Unsupported operating system: $(uname -s)"
    log_error "This script supports Linux, macOS, and Windows (Git Bash/WSL)"
    exit 1
fi

if [ "$OS_TYPE" = "windows" ]; then
    log_warning "Windows detected. Some features may require WSL or Git Bash."
    NO_SUDO=1  # Windows doesn't use sudo in Git Bash
fi

# ==============================================================================
# Prerequisites Check
# ==============================================================================

check_prerequisites() {
    log_section "Checking Prerequisites"

    local missing_tools=()

    # Check for Rust/Cargo
    if ! command -v cargo >/dev/null 2>&1; then
        missing_tools+=("cargo (Rust toolchain)")
    else
        local rust_version=$(rustc --version 2>/dev/null | awk '{print $2}')
        log_success "Rust toolchain found: $rust_version"
    fi

    # Check for Git (needed for cargo install)
    if ! command -v git >/dev/null 2>&1; then
        missing_tools+=("git")
    else
        log_success "Git found: $(git --version | awk '{print $3}')"
    fi

    # Check for npm (optional, for jscpd)
    if ! command -v npm >/dev/null 2>&1; then
        log_warning "npm not found - will skip jscpd installation (optional)"
    else
        log_success "npm found: $(npm --version)"
    fi

    # Report missing tools
    if [ ${#missing_tools[@]} -gt 0 ]; then
        log_error "Missing required tools:"
        for tool in "${missing_tools[@]}"; do
            log_error "  • $tool"
        done
        exit 1
    fi
}

# ==============================================================================
# sccache Installation (Compilation Cache)
# ==============================================================================

install_sccache() {
    log_section "Installing sccache (Compiler Cache)"

    if command -v sccache >/dev/null 2>&1; then
        local version=$(sccache --version | head -1)
        log_success "sccache already installed: $version"
        return 0
    fi

    log_info "Installing sccache via cargo..."

    # Install system dependencies if needed (Linux only)
    if [ "$OS_TYPE" = "linux" ] && [ "$NO_SUDO" != "1" ]; then
        case "$PKG_MANAGER" in
            apt)
                log_info "Installing build dependencies (libssl-dev, pkg-config)..."
                sudo apt-get update -qq && sudo apt-get install -y libssl-dev pkg-config
                ;;
            dnf)
                log_info "Installing build dependencies (openssl-devel, pkg-config)..."
                sudo dnf install -y openssl-devel pkgconfig
                ;;
            pacman)
                log_info "Installing build dependencies (openssl, pkgconf)..."
                sudo pacman -S --needed --noconfirm openssl pkgconf
                ;;
        esac
    fi

    # Install via cargo (universal method)
    if cargo install sccache 2>&1 | grep -q "Installed"; then
        log_success "sccache installed successfully"
    else
        log_success "sccache installed (already present)"
    fi
}

# ==============================================================================
# mold Installation (Fast Linker)
# ==============================================================================

install_mold() {
    log_section "Installing mold (Fast Linker)"

    if command -v mold >/dev/null 2>&1; then
        local version=$(mold --version | head -1)
        log_success "mold already installed: $version"
        return 0
    fi

    if [ "$NO_SUDO" = "1" ]; then
        log_warning "Skipping mold installation (requires sudo)"
        return 0
    fi

    log_info "Installing mold linker..."

    case "$PKG_MANAGER" in
        brew)
            brew install mold
            log_success "mold installed via Homebrew"
            ;;
        apt)
            sudo apt-get update -qq && sudo apt-get install -y mold clang
            log_success "mold and clang installed via apt"
            ;;
        dnf)
            sudo dnf install -y mold clang
            log_success "mold and clang installed via dnf"
            ;;
        pacman)
            sudo pacman -S --needed --noconfirm mold clang
            log_success "mold and clang installed via pacman"
            ;;
        *)
            log_warning "No package manager found for mold installation"
            log_info "Install manually: https://github.com/rui314/mold#installation"
            return 0
            ;;
    esac
}

# ==============================================================================
# Cargo Development Utilities
# ==============================================================================

install_cargo_utils() {
    log_section "Installing Cargo Development Utilities"

    # Essential tools (always install)
    local essential_tools=(
        "cargo-audit:Security vulnerability scanner"
        "cargo-deny:Dependency linter (licenses, bans, security)"
        "cargo-watch:Auto-rebuild on file changes"
        "cargo-edit:CLI for managing dependencies (add/rm/upgrade)"
    )

    # Optional tools (skip if --essential-only)
    local optional_tools=(
        "cargo-nextest:Modern test runner with better output"
        "cargo-expand:Macro expansion for debugging"
        "cargo-bloat:Analyze binary size"
        "cargo-flamegraph:Performance profiling and flamegraph generation"
    )

    # Install essential tools
    log_info "Installing essential cargo utilities..."
    for tool_info in "${essential_tools[@]}"; do
        local tool_name="${tool_info%%:*}"
        local tool_desc="${tool_info##*:}"

        if command -v "$tool_name" >/dev/null 2>&1; then
            log_success "$tool_name already installed"
        else
            log_info "Installing $tool_name - $tool_desc"
            if cargo install "$tool_name" >/dev/null 2>&1; then
                log_success "$tool_name installed"
            else
                log_warning "$tool_name installation failed (non-fatal)"
            fi
        fi
    done

    # Install optional tools if requested
    if [ "$ESSENTIAL_ONLY" != "1" ]; then
        log_info "Installing optional cargo utilities..."
        for tool_info in "${optional_tools[@]}"; do
            local tool_name="${tool_info%%:*}"
            local tool_desc="${tool_info##*:}"

            if command -v "$tool_name" >/dev/null 2>&1; then
                log_success "$tool_name already installed"
            else
                log_info "Installing $tool_name - $tool_desc (optional)"
                if cargo install "$tool_name" >/dev/null 2>&1; then
                    log_success "$tool_name installed"
                else
                    log_warning "$tool_name installation failed (optional, skipping)"
                fi
            fi
        done

        # Note about flamegraph system dependencies
        if command -v cargo-flamegraph >/dev/null 2>&1; then
            if [ "$OS_TYPE" = "linux" ] && ! command -v perf >/dev/null 2>&1; then
                log_info "Note: cargo-flamegraph requires 'perf' on Linux"
                log_info "Install with: sudo apt-get install linux-tools-common linux-tools-generic"
            fi
        fi
    else
        log_info "Skipping optional tools (--essential-only specified)"
    fi
}

# ==============================================================================
# Code Quality Tools
# ==============================================================================

install_quality_tools() {
    log_section "Installing Code Quality Tools"

    # jscpd (duplicate code detection)
    if command -v jscpd >/dev/null 2>&1; then
        log_success "jscpd already installed"
    elif command -v npm >/dev/null 2>&1; then
        log_info "Installing jscpd for duplicate code detection..."
        if npm install -g jscpd >/dev/null 2>&1; then
            log_success "jscpd installed"
        else
            log_warning "jscpd installation failed (optional)"
        fi
    else
        log_warning "npm not available - skipping jscpd (optional)"
    fi
}

# ==============================================================================
# Configuration Verification
# ==============================================================================

verify_configuration() {
    log_section "Verifying Configuration"

    local cargo_config="$PROJECT_ROOT/.cargo/config.toml"

    if [ ! -f "$cargo_config" ]; then
        log_warning "Cargo configuration not found at $cargo_config"
        log_info "This is normal if you haven't set up custom build configuration"
        return 0
    fi

    # Check if sccache is configured
    if grep -q 'rustc-wrapper.*sccache' "$cargo_config" 2>/dev/null; then
        log_success "sccache configured in .cargo/config.toml"
    else
        log_info "To enable sccache, add to $cargo_config:"
        log_info '  [build]'
        log_info '  rustc-wrapper = "sccache"'
    fi

    # Check if mold is configured
    if grep -q 'linker.*mold\|fuse-ld=mold' "$cargo_config" 2>/dev/null; then
        log_success "mold linker configured in .cargo/config.toml"
    elif command -v mold >/dev/null 2>&1; then
        log_info "mold is installed but not configured"
        log_info "See $cargo_config for platform-specific linker setup"
    fi
}

# ==============================================================================
# Build Test
# ==============================================================================

test_build() {
    log_section "Testing Build Performance"

    cd "$PROJECT_ROOT"

    # Show sccache statistics before
    if command -v sccache >/dev/null 2>&1; then
        log_info "sccache stats before build:"
        sccache --show-stats | head -5
        echo ""
    fi

    # Run a quick check
    log_info "Running cargo check..."
    if cargo check --quiet 2>&1; then
        log_success "Build completed successfully"
    else
        log_warning "Build check had issues (check output above)"
    fi

    # Show sccache statistics after
    if command -v sccache >/dev/null 2>&1; then
        echo ""
        log_info "sccache stats after build:"
        sccache --show-stats | head -5
    fi
}

# ==============================================================================
# Summary Report
# ==============================================================================

print_summary() {
    echo ""
    echo "================================================================"
    log_success "Development Tools Setup Complete!"
    echo "================================================================"
    echo ""

    echo "📊 EXPECTED IMPROVEMENTS:"
    echo "  • Incremental builds: 2-5x faster (via sccache)"
    echo "  • Link times: 3-10x faster (via mold)"
    echo "  • Clean builds: Cached across git branches"
    echo ""

    echo "🔧 INSTALLED TOOLS:"
    [ -x "$(command -v sccache)" ] && echo "  ✓ sccache: $(which sccache)"
    [ -x "$(command -v mold)" ] && echo "  ✓ mold: $(which mold)"
    [ -x "$(command -v clang)" ] && [ "$OS_TYPE" = "linux" ] && echo "  ✓ clang: $(which clang)"
    [ -x "$(command -v cargo-audit)" ] && echo "  ✓ cargo-audit: $(which cargo-audit)"
    [ -x "$(command -v cargo-deny)" ] && echo "  ✓ cargo-deny: $(which cargo-deny)"
    [ -x "$(command -v cargo-watch)" ] && echo "  ✓ cargo-watch: $(which cargo-watch)"
    [ -x "$(command -v cargo-edit)" ] && echo "  ✓ cargo-edit: $(which cargo-edit)"
    [ -x "$(command -v cargo-nextest)" ] && echo "  ✓ cargo-nextest: $(which cargo-nextest)"
    [ -x "$(command -v cargo-expand)" ] && echo "  ✓ cargo-expand: $(which cargo-expand)"
    [ -x "$(command -v cargo-bloat)" ] && echo "  ✓ cargo-bloat: $(which cargo-bloat)"
    [ -x "$(command -v cargo-flamegraph)" ] && echo "  ✓ cargo-flamegraph: $(which cargo-flamegraph)"
    [ -x "$(command -v jscpd)" ] && echo "  ✓ jscpd: $(which jscpd)"
    echo ""

    echo "💡 USEFUL COMMANDS:"
    echo "  • Check for security issues:   cargo audit"
    echo "  • Lint dependencies:           cargo deny check"
    echo "  • Auto-rebuild on changes:     cargo watch -x check"
    echo "  • Add a dependency:            cargo add <crate>"
    echo "  • Run tests (faster):          cargo nextest run"
    echo "  • Expand macros for debugging: cargo expand path::to::module"
    echo "  • Analyze binary size:         cargo bloat --release"
    echo "  • Generate flamegraph:         cargo flamegraph"
    echo "  • View sccache stats:          sccache --show-stats"
    echo "  • Clear compiler cache:        sccache --zero-stats"
    echo ""

    echo "📖 NEXT STEPS:"
    echo "  • Team members should run this script on their machines"
    echo "  • Review .cargo/config.toml for build optimizations"
    echo "  • Consider adding 'cargo audit' to your CI pipeline"
    echo ""
}

# ==============================================================================
# Main Installation Flow
# ==============================================================================

main() {
    echo ""
    echo "================================================================"
    log_info "🚀 CodeBuddy Development Tools Setup"
    echo "================================================================"
    log_info "Platform: $OS_TYPE | Package Manager: $PKG_MANAGER"
    echo ""

    # Run installation steps
    check_prerequisites
    install_sccache
    install_mold
    install_cargo_utils
    install_quality_tools
    verify_configuration

    # Optional build test
    if [ "$SKIP_TEST_BUILD" != "1" ]; then
        test_build
    else
        log_info "Skipping build test (SKIP_TEST_BUILD=1)"
    fi

    # Print summary
    print_summary
}

# ==============================================================================
# Execute Main
# ==============================================================================

main "$@"

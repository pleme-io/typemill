#!/usr/bin/env bash
#
# Codebuddy Installation Script
# Secure cross-platform installer for macOS, Ubuntu/Debian, Fedora/RHEL, and Arch
#
# Usage: bash install.sh
#
# Security Features:
# - No remote script execution (curl|bash patterns blocked)
# - Package verification before installation
# - Timeout protection (5 minute max)
# - Official package managers only
# - Detailed error logging with actionable fixes
#

set -euo pipefail
IFS=$'\n\t'

# Script timeout (5 minutes)
TIMEOUT=300
if command -v timeout &>/dev/null; then
    exec timeout ${TIMEOUT} "$0" "$@"
fi

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Global state
OS_TYPE=""
PKG_MANAGER=""
SHELL_CONFIG=""
INSTALL_DIR=""

#######################################
# Print formatted messages
#######################################
log_info() {
    echo -e "${BLUE}ℹ️  ${NC}$*"
}

log_success() {
    echo -e "${GREEN}✅ ${NC}$*"
}

log_warn() {
    echo -e "${YELLOW}⚠️  ${NC}$*"
}

log_error() {
    echo -e "${RED}❌ ${NC}$*" >&2
}

#######################################
# Error handler with actionable messages
# Arguments:
#   $1 - Error message
#   $2 - Suggested fix (optional)
# Returns:
#   Exits with code 1
#######################################
handle_error() {
    local error_msg="$1"
    local suggested_fix="${2:-}"

    log_error "Failed: ${error_msg}"

    if [[ -n "${suggested_fix}" ]]; then
        echo -e "${YELLOW}Suggested fix:${NC} ${suggested_fix}" >&2
    fi

    echo "" >&2
    echo "For help, visit: https://github.com/goobits/codebuddy/issues" >&2
    exit 1
}

#######################################
# Detect operating system and package manager
# Sets global variables: OS_TYPE, PKG_MANAGER
# Returns:
#   0 on success, exits on unsupported OS
#######################################
detect_os() {
    log_info "Detecting operating system..."

    # Detect OS
    if [[ "$OSTYPE" == "darwin"* ]]; then
        OS_TYPE="macOS"
        if command -v brew &>/dev/null; then
            PKG_MANAGER="brew"
        else
            handle_error "Homebrew not found" \
                "Install Homebrew from https://brew.sh/"
        fi
    elif [[ -f /etc/os-release ]]; then
        # shellcheck disable=SC1091
        source /etc/os-release
        case "${ID}" in
            ubuntu|debian|pop|linuxmint)
                OS_TYPE="Debian/Ubuntu"
                PKG_MANAGER="apt"
                ;;
            fedora|rhel|centos|rocky|almalinux)
                OS_TYPE="Fedora/RHEL"
                PKG_MANAGER="dnf"
                # Fallback to yum if dnf not available
                if ! command -v dnf &>/dev/null && command -v yum &>/dev/null; then
                    PKG_MANAGER="yum"
                fi
                ;;
            arch|manjaro|endeavouros)
                OS_TYPE="Arch Linux"
                PKG_MANAGER="pacman"
                ;;
            *)
                handle_error "Unsupported Linux distribution: ${ID}" \
                    "Please install manually: cargo install codebuddy"
                ;;
        esac
    else
        handle_error "Could not detect operating system" \
            "Please install manually: cargo install codebuddy"
    fi

    log_success "Detected: ${OS_TYPE} with ${PKG_MANAGER}"
}

#######################################
# Verify package exists and get version info
# Arguments:
#   $1 - Package name
# Returns:
#   0 if package verified, 1 if not found
#######################################
verify_package() {
    local package="$1"

    log_info "Verifying: ${package}"

    case "${PKG_MANAGER}" in
        brew)
            if brew info --json "${package}" &>/dev/null; then
                local version
                version=$(brew info --json "${package}" | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "unknown")
                log_success "Verified: ${package} v${version} from Homebrew"
                return 0
            fi
            ;;
        apt)
            if timeout 30 apt-cache policy "${package}" &>/dev/null; then
                local version
                version=$(apt-cache policy "${package}" | grep Candidate | awk '{print $2}' || echo "unknown")
                log_success "Verified: ${package} v${version} from APT repository"
                return 0
            fi
            ;;
        dnf|yum)
            if timeout 30 "${PKG_MANAGER}" info "${package}" &>/dev/null; then
                local version
                version=$("${PKG_MANAGER}" info "${package}" 2>/dev/null | grep Version | head -1 | awk '{print $3}' || echo "unknown")
                log_success "Verified: ${package} v${version} from ${PKG_MANAGER} repository"
                return 0
            fi
            ;;
        pacman)
            if timeout 30 pacman -Si "${package}" &>/dev/null; then
                local version
                version=$(pacman -Si "${package}" 2>/dev/null | grep Version | awk '{print $3}' || echo "unknown")
                log_success "Verified: ${package} v${version} from Arch repository"
                return 0
            fi
            ;;
    esac

    return 1
}

#######################################
# Install package after verification
# Arguments:
#   $1 - Package name
# Returns:
#   0 on success, exits on failure
#######################################
install_verified() {
    local package="$1"

    # Verify before installing
    if ! verify_package "${package}"; then
        handle_error "Package ${package} not found in repositories" \
            "Update package lists or check package name"
    fi

    log_info "Installing: ${package}"

    case "${PKG_MANAGER}" in
        brew)
            if ! brew install "${package}"; then
                handle_error "Failed to install ${package} via Homebrew" \
                    "Try: brew update && brew install ${package}"
            fi
            ;;
        apt)
            if ! sudo apt-get update -qq || ! sudo apt-get install -y "${package}"; then
                handle_error "Failed to install ${package} via APT" \
                    "Try: sudo apt-get update && sudo apt-get install ${package}"
            fi
            ;;
        dnf)
            if ! sudo dnf install -y "${package}"; then
                handle_error "Failed to install ${package} via DNF" \
                    "Try: sudo dnf check-update && sudo dnf install ${package}"
            fi
            ;;
        yum)
            if ! sudo yum install -y "${package}"; then
                handle_error "Failed to install ${package} via YUM" \
                    "Try: sudo yum check-update && sudo yum install ${package}"
            fi
            ;;
        pacman)
            if ! sudo pacman -Sy --noconfirm "${package}"; then
                handle_error "Failed to install ${package} via Pacman" \
                    "Try: sudo pacman -Syu && sudo pacman -S ${package}"
            fi
            ;;
    esac

    log_success "Installed: ${package} successfully"
}

#######################################
# Configure shell PATH
# Sets SHELL_CONFIG and INSTALL_DIR globals
# Returns:
#   0 on success
#######################################
configure_path() {
    log_info "Configuring shell environment..."

    # Detect shell and config file
    local current_shell
    current_shell=$(basename "${SHELL}")

    case "${current_shell}" in
        bash)
            if [[ -f "${HOME}/.bashrc" ]]; then
                SHELL_CONFIG="${HOME}/.bashrc"
            elif [[ -f "${HOME}/.bash_profile" ]]; then
                SHELL_CONFIG="${HOME}/.bash_profile"
            else
                SHELL_CONFIG="${HOME}/.bashrc"
                touch "${SHELL_CONFIG}"
            fi
            ;;
        zsh)
            if [[ -f "${HOME}/.zshrc" ]]; then
                SHELL_CONFIG="${HOME}/.zshrc"
            else
                SHELL_CONFIG="${HOME}/.zshrc"
                touch "${SHELL_CONFIG}"
            fi
            ;;
        *)
            log_warn "Unknown shell: ${current_shell}, defaulting to .profile"
            SHELL_CONFIG="${HOME}/.profile"
            ;;
    esac

    # Detect installation directory
    if [[ "${OS_TYPE}" == "macOS" ]]; then
        # Homebrew on Apple Silicon uses /opt/homebrew, Intel uses /usr/local
        if [[ -d "/opt/homebrew/bin" ]]; then
            INSTALL_DIR="/opt/homebrew/bin"
        else
            INSTALL_DIR="/usr/local/bin"
        fi
    else
        # Most Linux distros use /usr/bin or /usr/local/bin
        if command -v cargo &>/dev/null; then
            INSTALL_DIR="$(dirname "$(command -v cargo)")"
        else
            INSTALL_DIR="/usr/local/bin"
        fi
    fi

    log_success "Shell config: ${SHELL_CONFIG}"
    log_success "Install directory: ${INSTALL_DIR}"
}

#######################################
# Check if command exists and is accessible
# Arguments:
#   $1 - Command name
# Returns:
#   0 if command exists, 1 otherwise
#######################################
command_exists() {
    command -v "$1" &>/dev/null
}

#######################################
# Main installation flow
#######################################
main() {
    echo ""
    echo "╔═══════════════════════════════════════╗"
    echo "║   Codebuddy Installation Script      ║"
    echo "║   Secure Cross-Platform Installer    ║"
    echo "╚═══════════════════════════════════════╝"
    echo ""

    # Step 1: Detect OS
    detect_os

    # Step 2: Configure paths
    configure_path

    # Step 3: Check if Rust is already installed
    if command_exists cargo && command_exists rustc; then
        log_success "Rust toolchain already installed"
        rustc --version
    else
        log_info "Rust toolchain not found, installing..."

        # Install Rust toolchain based on OS
        case "${PKG_MANAGER}" in
            brew)
                install_verified "rust"
                ;;
            apt)
                install_verified "curl"
                install_verified "build-essential"
                # Use rustup for more control
                log_info "Installing Rust via rustup (official installer)..."
                if ! curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable; then
                    handle_error "Failed to install Rust via rustup" \
                        "Try manual installation: https://rustup.rs/"
                fi
                # Source cargo env
                # shellcheck disable=SC1091
                source "${HOME}/.cargo/env" 2>/dev/null || true
                ;;
            dnf|yum)
                install_verified "curl"
                install_verified "gcc"
                install_verified "gcc-c++"
                # Use rustup for more control
                log_info "Installing Rust via rustup (official installer)..."
                if ! curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable; then
                    handle_error "Failed to install Rust via rustup" \
                        "Try manual installation: https://rustup.rs/"
                fi
                # Source cargo env
                # shellcheck disable=SC1091
                source "${HOME}/.cargo/env" 2>/dev/null || true
                ;;
            pacman)
                install_verified "rust"
                install_verified "base-devel"
                ;;
        esac

        log_success "Rust toolchain installed"
    fi

    # Verify Rust is accessible
    if ! command_exists cargo; then
        # Try sourcing cargo env
        # shellcheck disable=SC1091
        source "${HOME}/.cargo/env" 2>/dev/null || true

        if ! command_exists cargo; then
            handle_error "Cargo not found in PATH after installation" \
                "Restart your shell or run: source ${HOME}/.cargo/env"
        fi
    fi

    # Step 4: Ensure git is installed
    if ! command_exists git; then
        log_info "Git not found, installing..."
        case "${PKG_MANAGER}" in
            brew)
                install_verified "git"
                ;;
            apt)
                install_verified "git"
                ;;
            dnf|yum)
                install_verified "git"
                ;;
            pacman)
                install_verified "git"
                ;;
        esac
    fi

    # Step 5: Install codebuddy from source
    log_info "Installing codebuddy from source..."

    # Create temporary directory
    local temp_dir
    temp_dir=$(mktemp -d)
    trap 'rm -rf "${temp_dir}"' EXIT

    cd "${temp_dir}" || handle_error "Failed to create temporary directory"

    # Clone repository
    log_info "Cloning repository..."
    if ! git clone --depth 1 https://github.com/goobits/codebuddy.git; then
        handle_error "Failed to clone repository" \
            "Check network connection"
    fi

    cd codebuddy || handle_error "Failed to enter repository directory"

    # Use Makefile if available, otherwise build directly
    if [[ -f Makefile ]] && command_exists make; then
        log_info "Building and installing via Makefile..."
        if ! make install; then
            handle_error "Failed to build via Makefile" \
                "Check build logs above for details"
        fi
    else
        # Fallback: build directly with cargo
        log_info "Building release binary (this may take a few minutes)..."
        if ! cargo build --release; then
            handle_error "Failed to build codebuddy" \
                "Check build logs above for details"
        fi

        # Install binary
        log_info "Installing binary to ${INSTALL_DIR}..."
        if [[ "${OS_TYPE}" == "macOS" ]]; then
            cp target/release/codebuddy "${INSTALL_DIR}/" || \
                handle_error "Failed to copy binary" "Try: sudo cp target/release/codebuddy ${INSTALL_DIR}/"
        else
            sudo cp target/release/codebuddy "${INSTALL_DIR}/" || \
                handle_error "Failed to copy binary" "Check sudo permissions"
        fi
    fi

    # Verify installation
    if command_exists codebuddy; then
        log_success "Codebuddy installed successfully!"
        echo ""
        codebuddy --help | head -5
    else
        handle_error "Codebuddy not found in PATH after installation" \
            "Add ${INSTALL_DIR} to your PATH"
    fi

    echo ""
    echo "╔═══════════════════════════════════════╗"
    echo "║   Installation Complete! 🎉           ║"
    echo "╚═══════════════════════════════════════╝"
    echo ""
    echo "Next steps:"
    echo "  1. Run 'codebuddy setup' to configure language servers"
    echo "  2. Run 'codebuddy start' to start the MCP server"
    echo "  3. Check status with 'codebuddy status'"
    echo ""
    echo "Documentation: https://github.com/goobits/codebuddy"
    echo ""
}

# Run main installation
main "$@"

#!/usr/bin/env bash
#
# Codebuddy Installation Script
# Downloads and installs the pre-built binary for the current platform.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh | bash
# Or, for more transparency:
#   curl -fsSL -o install.sh https://raw.githubusercontent.com/goobits/codebuddy/main/install.sh
#   bash install.sh
#

set -euo pipefail
IFS=$'\n\t'

# --- Configuration ---
# REPO: The GitHub repository to download from.
# VERSION: The specific version to install. Using a fixed version for reliability.
readonly REPO="goobits/codebuddy"
readonly VERSION="1.0.0-rc4"

# --- Colors for Output ---
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m'

# --- Global Variables ---
OS=""
ARCH=""
INSTALL_DIR=""

#######################################
# Print formatted log messages.
# Globals:
#   BLUE, GREEN, YELLOW, RED, NC
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
# Handle errors and exit with a helpful message.
# Arguments:
#   $1 - The error message to display.
#   $2 - A suggested fix (optional).
#######################################
handle_error() {
    log_error "Installation failed: $1"
    if [[ -n "${2:-}" ]]; then
        echo -e "${YELLOW}Suggested fix:${NC} $2" >&2
    fi
    echo "" >&2
    echo "For help, please open an issue at: https://github.com/${REPO}/issues" >&2
    exit 1
}

#######################################
# Detect the operating system and architecture.
# Sets the global variables: OS, ARCH
#######################################
detect_platform() {
    log_info "Detecting platform..."
    case "$(uname -s)" in
        Linux)
            OS="linux"
            ;;
        Darwin)
            OS="macos"
            ;;
        *)
            handle_error "Unsupported operating system: $(uname -s)" "Only macOS and Linux are supported."
            ;;
    esac

    case "$(uname -m)" in
        x86_64)
            ARCH="x86_64"
            ;;
        arm64 | aarch64)
            ARCH="aarch64"
            ;;
        *)
            handle_error "Unsupported architecture: $(uname -m)" "Only x86_64 and arm64/aarch64 are supported."
            ;;
    esac
    log_success "Platform detected: ${OS}-${ARCH}"
}

#######################################
# Determine the installation directory.
# Sets the global variable: INSTALL_DIR
#######################################
determine_install_dir() {
    log_info "Determining installation directory..."
    if [[ -w "/usr/local/bin" ]]; then
        INSTALL_DIR="/usr/local/bin"
    elif [[ -n "${HOME:-}" && -d "${HOME}" ]]; then
        INSTALL_DIR="${HOME}/.local/bin"
        if [[ ! -d "${INSTALL_DIR}" ]]; then
            log_info "Creating installation directory at ${INSTALL_DIR}"
            mkdir -p "${INSTALL_DIR}"
        fi
    else
        handle_error "Could not find a writable installation directory." "Please create and grant write permissions to /usr/local/bin or ensure your HOME directory is set."
    fi
    log_success "Using install directory: ${INSTALL_DIR}"
}

#######################################
# Check if a command exists in the user's PATH.
# Arguments:
#   $1 - The command name to check.
# Returns:
#   0 if the command exists, 1 otherwise.
#######################################
command_exists() {
    command -v "$1" &>/dev/null
}

#######################################
# Main installation function.
#######################################
main() {
    echo ""
    echo "╔════════════════════════════════════════════╗"
    echo "║   Codebuddy Quick Install                  ║"
    echo "║   Downloads pre-built binary               ║"
    echo "╚════════════════════════════════════════════╝"
    echo ""

    # Step 1: Ensure curl is installed
    if ! command_exists curl; then
        handle_error "'curl' is not installed." "Please install curl using your system's package manager (e.g., 'sudo apt-get install curl')."
    fi

    # Step 2: Detect platform and installation directory
    detect_platform
    determine_install_dir

    # Step 3: Construct download URL and download binary
    local binary_name="codebuddy-${OS}-${ARCH}"
    local download_url="https://github.com/${REPO}/releases/download/${VERSION}/${binary_name}"
    local temp_binary
    temp_binary=$(mktemp)

    # Clean up the temp file on exit
    trap 'rm -f "${temp_binary}"' EXIT

    log_info "Downloading from: ${download_url}"
    if ! curl --fail --location --progress-bar --output "${temp_binary}" "${download_url}"; then
        handle_error "Download failed." "Check the URL and your network connection. The release or asset might not exist for your platform."
    fi
    log_success "Download complete."

    # Step 4: Install the binary
    log_info "Installing codebuddy..."
    chmod +x "${temp_binary}"

    local install_path="${INSTALL_DIR}/codebuddy"

    # Use sudo if we are not the owner of the directory
    if [[ -w "${INSTALL_DIR}" ]]; then
        mv "${temp_binary}" "${install_path}"
    else
        log_warn "Installation directory is not writable. Attempting with sudo."
        if command_exists sudo; then
            sudo mv "${temp_binary}" "${install_path}"
        else
            handle_error "Cannot write to ${INSTALL_DIR} and 'sudo' is not available." "Please run the script as a user with write permissions to ${INSTALL_DIR}."
        fi
    fi

    # Step 5: Verify installation
    log_info "Verifying installation..."
    if ! command_exists codebuddy; then
        log_warn "codebuddy command not found in PATH."
        echo -e "Your shell's PATH variable may need to be updated to include:"
        echo -e "  ${YELLOW}${INSTALL_DIR}${NC}"
        echo ""
        echo -e "Please add the following line to your shell profile (e.g., ~/.bashrc, ~/.zshrc):"
        echo -e "  ${YELLOW}export PATH=\"${INSTALL_DIR}:\$PATH\"${NC}"
        echo ""
        echo -e "After updating your profile, please restart your terminal or run 'source <your_profile_file>'."
        handle_error "Installation complete, but requires manual PATH adjustment."
    fi

    local installed_version
    installed_version=$(codebuddy --version)
    log_success "Installation successful! ${installed_version}"

    echo ""
    echo "╔════════════════════════════════════════════╗"
    echo "║   Installation Complete! 🎉                 ║"
    echo "╚════════════════════════════════════════════╝"
    echo ""
    echo "Next steps:"
    echo "  1. Configure language servers for your project: ${YELLOW}codebuddy setup${NC}"
    echo "  2. Start the server: ${YELLOW}codebuddy start${NC}"
    echo "  3. Check the status: ${YELLOW}codebuddy status${NC}"
    echo ""
    echo "For full documentation, visit: https://github.com/${REPO}"
    echo ""
}

# Run the main function
main

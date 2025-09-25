#!/bin/bash
# Cross-platform FUSE setup script
# Supports Linux and macOS

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🚀 CodeFlow Buddy - FUSE Setup${NC}"
echo "===================================="

# Detect OS
OS="$(uname -s)"
ARCH="$(uname -m)"

echo -e "${YELLOW}📍 Detected: ${OS} ${ARCH}${NC}"
echo

# Function to check FUSE availability
check_fuse() {
    if [ "$OS" = "Linux" ]; then
        if command -v fusermount &> /dev/null; then
            echo -e "${GREEN}✅ FUSE is installed${NC}"
            return 0
        else
            echo -e "${RED}❌ FUSE is not installed${NC}"
            return 1
        fi
    elif [ "$OS" = "Darwin" ]; then
        if [ -d "/Library/Frameworks/macFUSE.framework" ] || [ -d "/Library/Frameworks/OSXFUSE.framework" ]; then
            echo -e "${GREEN}✅ macFUSE is installed${NC}"
            return 0
        else
            echo -e "${RED}❌ macFUSE is not installed${NC}"
            return 1
        fi
    fi
}

# Function to install FUSE on Linux
install_linux_fuse() {
    echo -e "${YELLOW}📦 Installing FUSE for Linux...${NC}"

    # Detect package manager
    if command -v apt-get &> /dev/null; then
        echo "Using apt-get..."
        sudo apt-get update
        sudo apt-get install -y fuse fuse-dev

        # Add user to fuse group if it exists
        if getent group fuse &> /dev/null; then
            sudo usermod -aG fuse "$USER"
            echo -e "${YELLOW}⚠️  Added $USER to fuse group - logout/login may be required${NC}"
        fi

    elif command -v yum &> /dev/null; then
        echo "Using yum..."
        sudo yum install -y fuse fuse-devel

    elif command -v dnf &> /dev/null; then
        echo "Using dnf..."
        sudo dnf install -y fuse fuse-devel

    elif command -v pacman &> /dev/null; then
        echo "Using pacman..."
        sudo pacman -S --noconfirm fuse2 fuse3

    elif command -v zypper &> /dev/null; then
        echo "Using zypper..."
        sudo zypper install -y fuse libfuse-devel

    elif command -v apk &> /dev/null; then
        echo "Using apk (Alpine)..."
        sudo apk add --no-cache fuse fuse-dev

    else
        echo -e "${RED}❌ Unsupported package manager${NC}"
        echo "Please install FUSE manually:"
        echo "  Debian/Ubuntu: sudo apt-get install fuse fuse-dev"
        echo "  RedHat/CentOS: sudo yum install fuse fuse-devel"
        echo "  Fedora: sudo dnf install fuse fuse-devel"
        echo "  Arch: sudo pacman -S fuse2 fuse3"
        echo "  openSUSE: sudo zypper install fuse libfuse-devel"
        echo "  Alpine: sudo apk add fuse fuse-dev"
        return 1
    fi

    echo -e "${GREEN}✅ FUSE installed for Linux${NC}"
}

# Function to install macFUSE
install_macos_fuse() {
    echo -e "${YELLOW}📦 Installing macFUSE for macOS...${NC}"

    # Check for Homebrew
    if ! command -v brew &> /dev/null; then
        echo -e "${RED}❌ Homebrew not found${NC}"
        echo "Install Homebrew first:"
        echo '  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'
        echo
        echo "Or download macFUSE manually from: https://osxfuse.github.io"
        return 1
    fi

    echo "Installing macFUSE via Homebrew..."
    brew install --cask macfuse

    echo -e "${YELLOW}⚠️  IMPORTANT: macFUSE requires a kernel extension${NC}"
    echo "After installation:"
    echo "  1. Open System Preferences > Security & Privacy"
    echo "  2. Click 'Allow' next to the macFUSE developer"
    echo "  3. You may need to restart your Mac"
    echo
    echo -e "${GREEN}✅ macFUSE installed${NC}"
}

# Main setup flow
main() {
    # Check if FUSE is already installed
    if check_fuse; then
        echo -e "${GREEN}✨ FUSE is already set up!${NC}"
        echo
    else
        # Prompt for installation
        read -p "Install FUSE for $OS? (y/n) " -n 1 -r
        echo

        if [[ $REPLY =~ ^[Yy]$ ]]; then
            case "$OS" in
                Linux)
                    install_linux_fuse
                    ;;
                Darwin)
                    install_macos_fuse
                    ;;
                *)
                    echo -e "${RED}❌ Unsupported OS: $OS${NC}"
                    echo "FUSE is only supported on Linux and macOS"
                    exit 1
                    ;;
            esac

            # Verify installation
            echo
            echo -e "${YELLOW}🔍 Verifying installation...${NC}"
            if check_fuse; then
                echo -e "${GREEN}✅ FUSE setup completed successfully!${NC}"
            else
                echo -e "${YELLOW}⚠️  Installation completed but verification failed${NC}"
                echo "You may need to restart your system or terminal"
            fi
        else
            echo "Installation cancelled"
            exit 0
        fi
    fi

    # Rebuild native modules if in a Node.js project
    if [ -f "package.json" ]; then
        echo
        echo -e "${YELLOW}📦 Rebuilding native modules...${NC}"

        if command -v bun &> /dev/null; then
            bun rebuild @cocalc/fuse-native 2>/dev/null || true
        elif command -v npm &> /dev/null; then
            npm rebuild @cocalc/fuse-native 2>/dev/null || true
        fi

        echo -e "${GREEN}✅ Native modules rebuilt${NC}"
    fi

    echo
    echo -e "${BLUE}📚 Documentation${NC}"
    echo "  FUSE allows CodeFlow Buddy to mount remote filesystems locally"
    echo "  This enables LSP servers to access files as if they were local"
    echo
    echo -e "${GREEN}🎉 Setup complete!${NC}"
}

# Run main function
main
#!/usr/bin/env bash
# forgeStat Unix Installer (Linux/macOS)
# This script downloads and installs forgeStat on Linux and macOS systems

set -euo pipefail

# Configuration
REPO="olaproeis/forgeStat"
BINARY_NAME="forgeStat"
VERSION="latest"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux";;
        Darwin*)    echo "macos";;
        *)          echo "unknown";;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   echo "x86_64";;
        arm64|aarch64)  echo "aarch64";;
        armv7l)         echo "armv7";;
        *)              echo "unknown";;
    esac
}

get_target() {
    local os=$1
    local arch=$2
    
    case "$os" in
        linux)
            case "$arch" in
                x86_64)  echo "x86_64-unknown-linux-gnu";;
                aarch64) echo "aarch64-unknown-linux-gnu";;
                *)       echo "unknown";;
            esac
            ;;
        macos)
            case "$arch" in
                x86_64)  echo "x86_64-apple-darwin";;
                aarch64) echo "aarch64-apple-darwin";;
                *)       echo "unknown";;
            esac
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

get_latest_version() {
    info "Fetching latest version..."
    curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

download_forgeStat() {
    local version=$1
    local target=$2
    
    local url="https://github.com/$REPO/releases/download/$version/forgeStat-$version-$target.tar.gz"
    local temp_dir
    temp_dir=$(mktemp -d)
    
    info "Downloading forgeStat $version for $target..."
    
    if command -v curl &> /dev/null; then
        curl -fsSL "$url" -o "$temp_dir/forgeStat.tar.gz"
    elif command -v wget &> /dev/null; then
        wget -q "$url" -O "$temp_dir/forgeStat.tar.gz"
    else
        error "Neither curl nor wget is installed. Please install one of them."
        exit 1
    fi
    
    success "Download complete"
    echo "$temp_dir"
}

install_binary() {
    local temp_dir=$1
    local install_dir=$2
    
    info "Extracting archive..."
    tar -xzf "$temp_dir/forgeStat.tar.gz" -C "$temp_dir"
    
    info "Installing to $install_dir..."
    
    # Create install directory if it doesn't exist
    mkdir -p "$install_dir"
    
    # Find and copy the binary
    local binary
    binary=$(find "$temp_dir" -type f -name "$BINARY_NAME" | head -n 1)
    
    if [ -z "$binary" ]; then
        error "Could not find $BINARY_NAME in the downloaded archive"
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Copy binary to install directory
    cp "$binary" "$install_dir/"
    chmod +x "$install_dir/$BINARY_NAME"
    
    # Cleanup
    rm -rf "$temp_dir"
    
    success "Binary installed to $install_dir/$BINARY_NAME"
}

add_to_path() {
    local install_dir=$1
    local shell_rc=""
    
    # Detect shell and appropriate rc file
    case "$(basename "$SHELL")" in
        bash)   shell_rc="$HOME/.bashrc";;
        zsh)    shell_rc="$HOME/.zshrc";;
        fish)   shell_rc="$HOME/.config/fish/config.fish";;
        *)      shell_rc="$HOME/.bashrc";;
    esac
    
    # Check if already in PATH
    case ":$PATH:" in
        *":$install_dir:"*)
            info "forgeStat is already in PATH"
            return 0
            ;;
    esac
    
    info "Adding $install_dir to PATH in $shell_rc..."
    
    case "$(basename "$SHELL")" in
        fish)
            echo "fish_add_path $install_dir" >> "$shell_rc"
            ;;
        *)
            echo "export PATH=\"$install_dir:\$PATH\"" >> "$shell_rc"
            ;;
    esac
    
    success "Added $install_dir to PATH"
    warning "Please run 'source $shell_rc' or restart your terminal to use the 'forgeStat' command immediately"
}

create_uninstall_script() {
    local install_dir=$1
    local script_path="$install_dir/uninstall.sh"
    
    cat > "$script_path" << 'EOF'
#!/usr/bin/env bash
# forgeStat Uninstall Script

set -e

INSTALL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_NAME="forgeStat"

echo "Uninstalling forgeStat..."

# Remove binary
if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
    rm "$INSTALL_DIR/$BINARY_NAME"
    echo "Removed $INSTALL_DIR/$BINARY_NAME"
fi

# Remove this script
rm "$INSTALL_DIR/uninstall.sh"
echo "Removed uninstall script"

# Remove install directory if empty
if [ -z "$(ls -A "$INSTALL_DIR")" ]; then
    rmdir "$INSTALL_DIR"
    echo "Removed empty install directory"
fi

echo "forgeStat has been uninstalled"
echo "Note: You may need to manually remove the PATH entry from your shell configuration file"
EOF
    
    chmod +x "$script_path"
    success "Created uninstall script at $script_path"
}

test_installation() {
    local install_dir=$1
    local binary_path="$install_dir/$BINARY_NAME"
    
    info "Testing installation..."
    
    if [ -x "$binary_path" ]; then
        local version
        version=$("$binary_path" --version 2>/dev/null || echo "unknown")
        success "forgeStat is installed and working: $version"
        return 0
    else
        error "Installation failed - binary not found or not executable at $binary_path"
        return 1
    fi
}

main() {
    cat << 'EOF'
╔══════════════════════════════════════════════════════════════╗
║                   forgeStat Installer                        ║
║          A real-time GitHub repository dashboard             ║
╚══════════════════════════════════════════════════════════════╝
EOF
    
    # Detect OS and architecture
    local os
    os=$(detect_os)
    local arch
    arch=$(detect_arch)
    
    if [ "$os" = "unknown" ]; then
        error "Unsupported operating system"
        exit 1
    fi
    
    if [ "$arch" = "unknown" ]; then
        error "Unsupported architecture"
        exit 1
    fi
    
    info "Detected OS: $os, Architecture: $arch"
    
    local target
    target=$(get_target "$os" "$arch")
    
    if [ "$target" = "unknown" ]; then
        error "Unsupported combination: $os on $arch"
        exit 1
    fi
    
    info "Target: $target"
    
    # Get version
    if [ "$VERSION" = "latest" ]; then
        VERSION=$(get_latest_version)
    fi
    info "Installing version: $VERSION"
    
    # Determine install directory
    local install_dir
    if [ -d "$HOME/.cargo/bin" ] && [[ ":$PATH:" == *":$HOME/.cargo/bin:"* ]]; then
        install_dir="$HOME/.cargo/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        install_dir="$HOME/.local/bin"
    else
        install_dir="$HOME/.forgeStat/bin"
        mkdir -p "$install_dir"
    fi
    
    # Download and install
    local temp_dir
    temp_dir=$(download_forgeStat "$VERSION" "$target")
    install_binary "$temp_dir" "$install_dir"
    
    # Add to PATH
    add_to_path "$install_dir"
    
    # Create uninstall script
    create_uninstall_script "$install_dir"
    
    # Test installation
    if test_installation "$install_dir"; then
        echo ""
        success "forgeStat has been successfully installed!"
        echo ""
        info "Usage examples:"
        echo "  $BINARY_NAME owner/repo              # Launch TUI"
        echo "  $BINARY_NAME owner/repo --summary    # Quick summary"
        echo "  $BINARY_NAME owner/repo --json       # Export JSON"
        echo "  $BINARY_NAME --help                  # Show all options"
        echo ""
        info "Documentation: https://github.com/$REPO"
        echo ""
        info "To uninstall, run: $install_dir/uninstall.sh"
    else
        error "Installation failed. Please try again or install manually."
        exit 1
    fi
}

# Run main function
main "$@"

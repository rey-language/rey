#!/usr/bin/env bash
# Rey Programming Language Installer
# https://github.com/rey-language/rey
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/rey-language/rey/master/install.sh | bash
#   or from local clone / release tarball:
#   ./install.sh

set -e

RESET='\033[0m'
BOLD='\033[1m'
GREEN='\033[32m'
BLUE='\033[34m'
CYAN='\033[36m'
YELLOW='\033[33m'
RED='\033[31m'

log_info() {
    printf "${BLUE}==>${RESET} ${BOLD}%s${RESET}\n" "$1"
}

log_success() {
    printf "${GREEN}==>${RESET} ${BOLD}%s${RESET}\n" "$1"
}

log_warn() {
    printf "${YELLOW}warning:${RESET} %s\n" "$1"
}

log_error() {
    printf "${RED}error:${RESET} %s\n" "$1" >&2
}

VERSION="v0.3.0"
REPO_URL="https://github.com/rey-language/rey"
STD_REPO_URL="https://github.com/IMisbahk/std.git"
INSTALL_DIR="${REY_INSTALL_DIR:-$HOME/.rey}"
BIN_DIR="$INSTALL_DIR/bin"
STD_DIR="$INSTALL_DIR/std"

printf "\n"
printf "${CYAN}   ____             ${RESET}\n"
printf "${CYAN}  / __ \\___  __  __ ${RESET}  Rey Programming Language Installer\n"
printf "${CYAN} / /_/ / _ \\/ / / / ${RESET}  Release %s\n" "$VERSION"
printf "${CYAN}/ _, _/  __/ /_/ /  ${RESET}  https://rey-lang.org\n"
printf "${CYAN}/_/ |_|\\___/\\__, /   ${RESET}\n"
printf "${CYAN}           /____/    ${RESET}\n\n"

# 1. Detect OS and Architecture
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
    darwin)
        PLATFORM="macos"
        ;;
    linux)
        PLATFORM="linux"
        ;;
    msys*|mingw*|cygwin*)
        PLATFORM="windows"
        ;;
    *)
        log_error "Unsupported operating system: $OS"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        ;;
    arm64|aarch64)
        TARGET_ARCH="arm64"
        ;;
    *)
        log_error "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Linux target uses aarch64 name in archive
if [ "$PLATFORM" = "linux" ] && [ "$TARGET_ARCH" = "arm64" ]; then
    ARCHIVE_ARCH="aarch64"
else
    ARCHIVE_ARCH="$TARGET_ARCH"
fi

TARGET="${PLATFORM}-${ARCHIVE_ARCH}"
log_info "Detected system: $PLATFORM ($TARGET_ARCH)"

# Install the small Linux toolchain needed to compile and link Rey programs.
# The runtime Makefile invokes /usr/bin/clang directly on Linux.
if { [ "$PLATFORM" = "linux" ] && [ -x /usr/bin/clang ]; } || { [ "$PLATFORM" != "linux" ] && command -v clang >/dev/null 2>&1; }; then
    log_info "Clang found; skipping toolchain installation."
elif [ "$PLATFORM" = "linux" ] && command -v apt-get >/dev/null 2>&1; then
    if [ "$(id -u)" -eq 0 ]; then
        SUDO=""
    elif command -v sudo >/dev/null 2>&1; then
        SUDO="sudo"
    else
        log_error "Clang is missing. Re-run as root or install sudo, then retry."
        exit 1
    fi

    log_info "Installing Clang and native build tools..."
    $SUDO apt-get update
    $SUDO apt-get install -y clang make libc6-dev binutils
    if [ ! -x /usr/bin/clang ]; then
        log_error "Clang installation completed, but /usr/bin/clang is still unavailable."
        exit 1
    fi
elif [ "$PLATFORM" = "macos" ]; then
    log_error "Clang is missing. Install the Xcode Command Line Tools with: xcode-select --install"
    exit 1
else
    log_error "Clang is missing and this installer cannot install packages for this system. Install clang, make, libc development files, and binutils, then retry."
    exit 1
fi

# 2. Prepare installation directory
mkdir -p "$BIN_DIR"
mkdir -p "$BIN_DIR/runtime"
mkdir -p "$STD_DIR"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "")"

# 3. Install packages
LOCAL_ARCHIVE="$SCRIPT_DIR/releases/0.3.0/rey-v0.3.0-${TARGET}.tar.gz"
LOCAL_NATIVE_BIN="$SCRIPT_DIR/build/rey"
LOCAL_RUNTIME_LIB="$SCRIPT_DIR/build/runtime/librey_rt.a"

if [ -f "$LOCAL_ARCHIVE" ]; then
    log_info "Found local release package: $LOCAL_ARCHIVE"
    TMP_EXTRACT="$(mktemp -d 2>/dev/null || mktemp -d -t 'rey-install')"
    tar -xzf "$LOCAL_ARCHIVE" -C "$TMP_EXTRACT"
    SOURCE_DIR="$TMP_EXTRACT/rey-v0.3.0-${TARGET}"
    cp -R "$SOURCE_DIR/bin/"* "$BIN_DIR/"
    rm -rf "$STD_DIR/src"
    mkdir -p "$STD_DIR"
    cp -R "$SOURCE_DIR/std/src" "$STD_DIR/src"
    rm -rf "$TMP_EXTRACT"
elif [ -f "$LOCAL_NATIVE_BIN" ] && [ -f "$LOCAL_RUNTIME_LIB" ]; then
    log_info "Installing native Rey compiler from local build..."
    cp "$LOCAL_NATIVE_BIN" "$BIN_DIR/rey"
    cp "$LOCAL_RUNTIME_LIB" "$BIN_DIR/runtime/"
    rm -rf "$STD_DIR/src"
    mkdir -p "$STD_DIR"
    if [ -d "$SCRIPT_DIR/../std/src" ]; then
        cp -R "$SCRIPT_DIR/../std/src" "$STD_DIR/src"
    elif [ -d "$SCRIPT_DIR/std/src" ]; then
        cp -R "$SCRIPT_DIR/std/src" "$STD_DIR/src"
    fi
else
    DOWNLOAD_URL="${REPO_URL}/releases/download/${VERSION}/rey-${VERSION}-${TARGET}.tar.gz"
    log_info "Downloading Rey ${VERSION} from ${DOWNLOAD_URL}..."
    TMP_ARCHIVE="$(mktemp 2>/dev/null || mktemp -t 'rey-archive').tar.gz"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DOWNLOAD_URL" -o "$TMP_ARCHIVE"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$TMP_ARCHIVE" "$DOWNLOAD_URL"
    else
        log_error "Neither curl nor wget is available. Please install curl."
        exit 1
    fi

    TMP_EXTRACT="$(mktemp -d 2>/dev/null || mktemp -d -t 'rey-install')"
    tar -xzf "$TMP_ARCHIVE" -C "$TMP_EXTRACT"
    SOURCE_DIR="$TMP_EXTRACT/rey-${VERSION}-${TARGET}"
    cp -R "$SOURCE_DIR/bin/"* "$BIN_DIR/"
    rm -rf "$STD_DIR/src"
    mkdir -p "$STD_DIR"
    cp -R "$SOURCE_DIR/std/src" "$STD_DIR/src"
    rm -rf "$TMP_ARCHIVE" "$TMP_EXTRACT"
fi

# Ensure correct permissions and clean up any stale bootstrap shims
chmod +x "$BIN_DIR/rey" 2>/dev/null || true
rm -f "$BIN_DIR/rey-v0" "$BIN_DIR/rey.bin" "$BIN_DIR/reyc" 2>/dev/null || true

# 4. Link standard library if not present
if [ ! -d "$STD_DIR/src" ] || [ -z "$(ls -A "$STD_DIR/src" 2>/dev/null)" ]; then
    if [ -d "$SCRIPT_DIR/../std/src" ]; then
        log_info "Linking standard library from workspace..."
        rm -rf "$STD_DIR/src"
        cp -R "$SCRIPT_DIR/../std/src" "$STD_DIR/src"
    elif [ -d "$SCRIPT_DIR/std/src" ]; then
        log_info "Linking standard library from repo..."
        rm -rf "$STD_DIR/src"
        cp -R "$SCRIPT_DIR/std/src" "$STD_DIR/src"
    elif command -v git >/dev/null 2>&1; then
        log_info "Fetching Standard Library from ${STD_REPO_URL}..."
        git clone --depth 1 "$STD_REPO_URL" "$STD_DIR" 2>/dev/null || true
    fi
fi

# Clean up any accidental circular symlinks
rm -f "$STD_DIR/std" 2>/dev/null || true

# Ensure ~/.capy/std points to ~/.rey/std without nested self-links
CAPY_DIR="$HOME/.capy"
mkdir -p "$CAPY_DIR"
rm -f "$CAPY_DIR/std" 2>/dev/null || true
ln -sfn "$STD_DIR" "$CAPY_DIR/std" 2>/dev/null || true

log_success "Rey compiler and Standard Library installed into: $INSTALL_DIR"

# 5. Add to shell configuration profile
add_to_shell() {
    PROFILE_FILE="$1"
    LINE_COMMENT="# Rey Language Environment"
    LINE_HOME="export REY_HOME=\"$INSTALL_DIR\""
    LINE_STD="export REY_STD_PATH=\"$STD_DIR/src\""
    LINE_PATH="export PATH=\"\$REY_HOME/bin:\$PATH\""

    if [ -f "$PROFILE_FILE" ]; then
        if ! grep -q "REY_HOME" "$PROFILE_FILE" 2>/dev/null; then
            printf "\n%s\n%s\n%s\n%s\n" "$LINE_COMMENT" "$LINE_HOME" "$LINE_STD" "$LINE_PATH" >> "$PROFILE_FILE"
            log_info "Added Rey to $PROFILE_FILE"
        fi
    fi
}

add_to_shell "$HOME/.zshrc"
add_to_shell "$HOME/.bashrc"
add_to_shell "$HOME/.bash_profile"
add_to_shell "$HOME/.profile"

# Fish shell support
FISH_CONFIG="$HOME/.config/fish/config.fish"
if [ -f "$FISH_CONFIG" ]; then
    if ! grep -q "REY_HOME" "$FISH_CONFIG" 2>/dev/null; then
        printf "\n# Rey Language Environment\nset -gx REY_HOME \"%s\"\nset -gx REY_STD_PATH \"%s/src\"\nfish_add_path \$REY_HOME/bin\n" "$INSTALL_DIR" "$STD_DIR" >> "$FISH_CONFIG"
        log_info "Added Rey to $FISH_CONFIG"
    fi
fi

# 6. Verification
export PATH="$BIN_DIR:$PATH"
export REY_STD_PATH="$STD_DIR/src"

log_info "Verifying installation..."
if "$BIN_DIR/rey" --version >/dev/null 2>&1; then
    REY_VER="$("$BIN_DIR/rey" --version 2>&1)"
    log_success "Verification passed: $REY_VER"
else
    log_success "Rey binary is ready at $BIN_DIR/rey"
fi

printf "\n${GREEN}${BOLD}Installation complete!${RESET}\n"
printf "To get started in your current terminal session, run:\n\n"
printf "    ${BOLD}export REY_HOME=\"%s\"${RESET}\n" "$INSTALL_DIR"
printf "    ${BOLD}export REY_STD_PATH=\"%s/src\"${RESET}\n" "$STD_DIR"
printf "    ${BOLD}export PATH=\"\$REY_HOME/bin:\$PATH\"${RESET}\n\n"
printf "Commands available:\n"
printf "    ${CYAN}rey build <file>${RESET}     Compile Rey source to native machine executable\n"
printf "    ${CYAN}rey run <file>${RESET}       Compile and execute immediately\n"
printf "    ${CYAN}rey check <file>${RESET}     Typecheck source without compiling\n\n"

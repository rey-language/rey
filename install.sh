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
printf "${CYAN}  / __ \___  __  __ ${RESET}  Rey Programming Language Installer\n"
printf "${CYAN} / /_/ / _ \/ / / / ${RESET}  Release %s\n" "$VERSION"
printf "${CYAN}/ _, _/  __/ /_/ /  ${RESET}  https://rey-lang.org\n"
printf "${CYAN}/_/ |_|\___/\__, /   ${RESET}\n"
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

# 2. Prepare installation directory
mkdir -p "$BIN_DIR"
mkdir -p "$BIN_DIR/runtime"
mkdir -p "$STD_DIR"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "")"

# 3. Check for local release archive or build binary first
LOCAL_ARCHIVE="$SCRIPT_DIR/releases/0.3.0/rey-v0.3.0-${TARGET}.tar.gz"
LOCAL_NATIVE_BIN="$SCRIPT_DIR/build/rey"
LOCAL_RUNTIME_LIB="$SCRIPT_DIR/build/runtime/librey_rt.a"
LOCAL_BUILD_BIN="$SCRIPT_DIR/compiler/v1/target/release/rey-v0"

if [ -f "$LOCAL_NATIVE_BIN" ]; then
    log_info "Installing native Rey compiler from $LOCAL_NATIVE_BIN..."
    cp "$LOCAL_NATIVE_BIN" "$BIN_DIR/rey"
    [ -f "$LOCAL_BUILD_BIN" ] && cp "$LOCAL_BUILD_BIN" "$BIN_DIR/rey-v0"
    [ -f "$LOCAL_RUNTIME_LIB" ] && cp "$LOCAL_RUNTIME_LIB" "$BIN_DIR/runtime/"
    rm -rf "$STD_DIR/src"
    mkdir -p "$STD_DIR"
    if [ -d "$SCRIPT_DIR/std/src" ]; then
        cp -R "$SCRIPT_DIR/std/src" "$STD_DIR/src"
    elif [ -d "$SCRIPT_DIR/../std/src" ]; then
        cp -R "$SCRIPT_DIR/../std/src" "$STD_DIR/src"
    fi
elif [ -f "$LOCAL_ARCHIVE" ]; then
    log_info "Found local release package: $LOCAL_ARCHIVE"
    TMP_EXTRACT="$(mktemp -d 2>/dev/null || mktemp -d -t 'rey-install')"
    tar -xzf "$LOCAL_ARCHIVE" -C "$TMP_EXTRACT"
    SOURCE_DIR="$TMP_EXTRACT/rey-v0.3.0-${TARGET}"
    cp -R "$SOURCE_DIR/bin/"* "$BIN_DIR/"
    rm -rf "$STD_DIR/src"
    mkdir -p "$STD_DIR"
    cp -R "$SOURCE_DIR/std/src" "$STD_DIR/src"
    rm -rf "$TMP_EXTRACT"
else
    # Remote download from GitHub Releases
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

chmod +x "$BIN_DIR/rey" 2>/dev/null || true
[ -f "$BIN_DIR/rey-v0" ] && chmod +x "$BIN_DIR/rey-v0" 2>/dev/null || true

# 4. Create convenience symlinks
(
    cd "$BIN_DIR"
    ln -sf rey reyc
    [ ! -f rey-v0 ] && ln -sf rey rey-v0 || true
)

# 5. Link standard library
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
        log_info "Fetching Standard Library from ${STD_REPO_URL}...\"\n        git clone --depth 1 \"$STD_REPO_URL\" \"$STD_DIR\" 2>/dev/null || true\n    fi\nfi\n\n# Clean up any accidental circular symlinks\nrm -f \"$STD_DIR/std\" 2>/dev/null || true\n\n# Ensure ~/.capy/std points to ~/.rey/std without nested self-links\nCAPY_DIR=\"$HOME/.capy\"\nmkdir -p \"$CAPY_DIR\"\nrm -f \"$CAPY_DIR/std\" 2>/dev/null || true\nln -sfn \"$STD_DIR\" \"$CAPY_DIR/std\" 2>/dev/null || true\n\nlog_success \"Rey compiler and Standard Library installed into: $INSTALL_DIR\"\n\n# 6. Add to shell configuration profile\nadd_to_shell() {\n    PROFILE_FILE=\"$1\"\n    LINE_COMMENT=\"# Rey Language Environment\"\n    LINE_HOME=\"export REY_HOME=\\\"$INSTALL_DIR\\\"\"\n    LINE_STD=\"export REY_STD_PATH=\\\"$STD_DIR/src\\\"\"\n    LINE_PATH=\"export PATH=\\\"\\\$REY_HOME/bin:\\\$PATH\\\"\"\n\n    if [ -f \"$PROFILE_FILE\" ]; then\n        if ! grep -q \"REY_HOME\" \"$PROFILE_FILE\" 2>/dev/null; then\n            printf \"\\n%s\\n%s\\n%s\\n%s\\n\" \"$LINE_COMMENT\" \"$LINE_HOME\" \"$LINE_STD\" \"$LINE_PATH\" >> \"$PROFILE_FILE\"\n            log_info \"Added Rey to $PROFILE_FILE\"\n        fi\n    fi\n}\n\nadd_to_shell \"$HOME/.zshrc\"\nadd_to_shell \"$HOME/.bashrc\"\nadd_to_shell \"$HOME/.bash_profile\"\nadd_to_shell \"$HOME/.profile\"\n\n# Fish shell support\nFISH_CONFIG=\"$HOME/.config/fish/config.fish\"\nif [ -f \"$FISH_CONFIG\" ]; then\n    if ! grep -q \"REY_HOME\" \"$FISH_CONFIG\" 2>/dev/null; then\n        printf \"\\n# Rey Language Environment\\nset -gx REY_HOME \\\"%s\\\"\\nset -gx REY_STD_PATH \\\"%s/src\\\"\\nfish_add_path \\$REY_HOME/bin\\n\" \"$INSTALL_DIR\" \"$STD_DIR\" >> \"$FISH_CONFIG\"\n        log_info \"Added Rey to $FISH_CONFIG\"\n    fi\nfi\n\n# 7. Verification\nexport PATH=\"$BIN_DIR:$PATH\"\nexport REY_STD_PATH=\"$STD_DIR/src\"\n\nlog_info \"Verifying installation...\"\nif \"$BIN_DIR/rey\" --version >/dev/null 2>&1; then\n    REY_VER=\"$(\"$BIN_DIR/rey\" --version 2>&1)\"\n    log_success \"Verification passed: $REY_VER\"\nelse\n    log_success \"Rey binary is ready at $BIN_DIR/rey\"\nfi\n\nprintf \"\\n${GREEN}${BOLD}Installation complete!${RESET}\\n\"\nprintf \"To get started in your current terminal session, run:\\n\\n\"\nprintf \"    ${BOLD}export REY_HOME=\\\"%s\\\"${RESET}\\n\" \"$INSTALL_DIR\"\nprintf \"    ${BOLD}export REY_STD_PATH=\\\"%s/src\\\"${RESET}\\n\" \"$STD_DIR\"\nprintf \"    ${BOLD}export PATH=\\\"\\\$REY_HOME/bin:\\\$PATH\\\"${RESET}\\n\\n\"\nprintf \"Commands available:\\n\"\nprintf \"    ${CYAN}rey build <file>${RESET}     Compile Rey source to native machine executable\\n\"\nprintf \"    ${CYAN}rey run <file>${RESET}       Compile and execute immediately\\n\"\nprintf \"    ${CYAN}rey check <file>${RESET}     Typecheck source without compiling\\n\"\nprintf \"    ${CYAN}rey-v0 <file>${RESET}       Run via Rust reference bootstrap engine\\n\\n\"\n
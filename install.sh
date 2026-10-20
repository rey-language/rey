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

# Install the native compiler toolchain only when Clang is missing.
run_as_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    elif command -v sudo >/dev/null 2>&1; then
        sudo "$@"
    else
        log_error "Installing build tools requires root or sudo access."
        exit 1
    fi
}

if [ "$PLATFORM" = "linux" ]; then
    # The runtime Makefile invokes /usr/bin/clang directly on Linux.
    if [ -x /usr/bin/clang ]; then
        log_info "Clang found; skipping toolchain installation."
    else
        log_info "Installing Clang and native build tools..."
        if command -v apt-get >/dev/null 2>&1; then
            run_as_root apt-get update
            run_as_root apt-get install -y clang make libc6-dev binutils
        elif command -v dnf >/dev/null 2>&1; then
            run_as_root dnf install -y clang make glibc-devel binutils
        elif command -v yum >/dev/null 2>&1; then
            run_as_root yum install -y clang make glibc-devel binutils
        elif command -v apk >/dev/null 2>&1; then
            run_as_root apk add --no-cache clang make musl-dev binutils
        elif command -v pacman >/dev/null 2>&1; then
            run_as_root pacman -Syu --needed --noconfirm clang make glibc binutils
        elif command -v zypper >/dev/null 2>&1; then
            run_as_root zypper --non-interactive install clang make glibc-devel binutils
        else
            log_error "No supported Linux package manager found. Install clang, make, libc development files, and binutils, then retry."
            exit 1
        fi

        if [ ! -x /usr/bin/clang ]; then
            log_error "Clang installation completed, but /usr/bin/clang is still unavailable."
            exit 1
        fi
    fi
elif [ "$PLATFORM" = "macos" ]; then
    if xcode-select -p >/dev/null 2>&1 && /usr/bin/clang --version >/dev/null 2>&1; then
        log_info "Apple Clang found; skipping toolchain installation."
    else
        log_info "Starting the Xcode Command Line Tools installer..."
        xcode-select --install || true
        log_error "Finish the Xcode Command Line Tools installation, then rerun this installer."
        exit 1
    fi
elif [ "$PLATFORM" = "windows" ]; then
    if command -v clang.exe >/dev/null 2>&1; then
        log_info "Clang found; skipping toolchain installation."
    else
        if ! command -v powershell.exe >/dev/null 2>&1 || ! command -v cygpath >/dev/null 2>&1; then
            log_error "On Windows, run install.sh from Git Bash with Windows PowerShell and cygpath available."
            exit 1
        fi

        LOCAL_APPDATA_WIN="$(powershell.exe -NoProfile -Command '$env:LOCALAPPDATA' | tr -d '\r')"
        LOCAL_APPDATA="$(cygpath -u "$LOCAL_APPDATA_WIN")"
        if [ -n "${MSYS2_ROOT:-}" ]; then
            MSYS2_ROOT="$(cygpath -u "$MSYS2_ROOT")"
        elif [ -x /c/msys64/usr/bin/bash.exe ]; then
            MSYS2_ROOT=/c/msys64
        else
            MSYS2_ROOT="$LOCAL_APPDATA/Programs/MSYS2"
        fi

        if [ ! -x "$MSYS2_ROOT/usr/bin/bash.exe" ]; then
            if ! command -v winget.exe >/dev/null 2>&1; then
                log_error "Clang is missing. Install Windows Package Manager or MSYS2, then rerun install.sh."
                exit 1
            fi
            log_info "Installing MSYS2..."
            MSYS2_ROOT_WIN="$(cygpath -w "$MSYS2_ROOT")"
            WINGET_EXIT=0
            winget.exe install --id MSYS2.MSYS2 --exact --scope user --silent --location "$MSYS2_ROOT_WIN" --accept-source-agreements --accept-package-agreements || WINGET_EXIT=$?
            if [ "$WINGET_EXIT" -ne 0 ] && [ "$WINGET_EXIT" -ne 2 ]; then
                log_error "MSYS2 installation failed (winget exit $WINGET_EXIT)."
                exit 1
            fi
            ATTEMPT=0
            while [ ! -x "$MSYS2_ROOT/usr/bin/bash.exe" ] && [ "$ATTEMPT" -lt 60 ]; do
                sleep 2
                ATTEMPT=$((ATTEMPT + 1))
            done
        fi

        if [ ! -x "$MSYS2_ROOT/usr/bin/bash.exe" ]; then
            log_error "MSYS2 was installed, but bash.exe was not found. Set MSYS2_ROOT and retry."
            exit 1
        fi

        log_info "Updating MSYS2 and installing the UCRT64 Clang toolchain..."
        "$MSYS2_ROOT/usr/bin/bash.exe" -lc "pacman -Syu --noconfirm || true"
        "$MSYS2_ROOT/usr/bin/bash.exe" -lc "pacman -Syu --noconfirm"
        "$MSYS2_ROOT/usr/bin/bash.exe" -lc "pacman -S --needed --noconfirm mingw-w64-ucrt-x86_64-clang"
        CLANG_BIN="$MSYS2_ROOT/ucrt64/bin"
        export PATH="$CLANG_BIN:$PATH"
        if ! command -v clang.exe >/dev/null 2>&1; then
            log_error "Clang installation completed, but clang.exe is still unavailable."
            exit 1
        fi
    fi
fi

# 2. Prepare installation directory
mkdir -p "$BIN_DIR"
mkdir -p "$BIN_DIR/runtime"
mkdir -p "$STD_DIR"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "")"

# 3. Install packages
if [ "$PLATFORM" = "windows" ]; then
    LOCAL_ARCHIVE="$SCRIPT_DIR/releases/0.3.0/rey-v0.3.0-${TARGET}.zip"
    ARCHIVE_EXT="zip"
else
    LOCAL_ARCHIVE="$SCRIPT_DIR/releases/0.3.0/rey-v0.3.0-${TARGET}.tar.gz"
    ARCHIVE_EXT="tar.gz"
fi
LOCAL_NATIVE_BIN="$SCRIPT_DIR/build/rey"
LOCAL_RUNTIME_LIB="$SCRIPT_DIR/build/runtime/librey_rt.a"

extract_archive() {
    ARCHIVE_PATH="$1"
    DESTINATION_PATH="$2"
    if [ "$PLATFORM" = "windows" ]; then
        REY_ARCHIVE_WIN="$(cygpath -w "$ARCHIVE_PATH")"
        REY_DESTINATION_WIN="$(cygpath -w "$DESTINATION_PATH")"
        export REY_ARCHIVE_WIN REY_DESTINATION_WIN
        powershell.exe -NoProfile -Command 'Expand-Archive -LiteralPath $env:REY_ARCHIVE_WIN -DestinationPath $env:REY_DESTINATION_WIN -Force'
    else
        tar -xzf "$ARCHIVE_PATH" -C "$DESTINATION_PATH"
    fi
}

if [ -f "$LOCAL_ARCHIVE" ]; then
    log_info "Found local release package: $LOCAL_ARCHIVE"
    TMP_EXTRACT="$(mktemp -d 2>/dev/null || mktemp -d -t 'rey-install')"
    extract_archive "$LOCAL_ARCHIVE" "$TMP_EXTRACT"
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
    DOWNLOAD_URL="${REPO_URL}/releases/download/${VERSION}/rey-${VERSION}-${TARGET}.${ARCHIVE_EXT}"
    log_info "Downloading Rey ${VERSION} from ${DOWNLOAD_URL}..."
    TMP_ARCHIVE="$(mktemp 2>/dev/null || mktemp -t 'rey-archive').${ARCHIVE_EXT}"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DOWNLOAD_URL" -o "$TMP_ARCHIVE"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$TMP_ARCHIVE" "$DOWNLOAD_URL"
    else
        log_error "Neither curl nor wget is available. Please install curl."
        exit 1
    fi

    TMP_EXTRACT="$(mktemp -d 2>/dev/null || mktemp -d -t 'rey-install')"
    extract_archive "$TMP_ARCHIVE" "$TMP_EXTRACT"
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

if [ "$PLATFORM" = "windows" ] && [ -n "${CLANG_BIN:-}" ]; then
    for PROFILE_FILE in "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.profile"; do
        if [ -f "$PROFILE_FILE" ] && ! grep -Fq "$CLANG_BIN" "$PROFILE_FILE" 2>/dev/null; then
            printf '\n# Rey Clang toolchain\nexport PATH="%s:$PATH"\n' "$CLANG_BIN" >> "$PROFILE_FILE"
            log_info "Added Clang to $PROFILE_FILE"
        fi
    done
fi

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

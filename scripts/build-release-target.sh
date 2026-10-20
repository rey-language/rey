#!/usr/bin/env bash
set -euo pipefail

usage() {
    echo "usage: $0 <macos-arm64|macos-x86_64|linux-x86_64|linux-aarch64|windows-x86_64>" >&2
}

if [[ $# -ne 1 ]]; then
    usage
    exit 2
fi

target=$1
script_dir=$(cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(cd -- "$script_dir/.." && pwd)
release_version=v0.3.1
real_ir_clang=
ir_target=
ir_native=false
workspace=${REY_RELEASE_WORKSPACE:-"$repo_root/build/release/$release_version/$target"}
case "$workspace" in
    /*) ;;
    *) echo "error: REY_RELEASE_WORKSPACE must be an absolute path" >&2; exit 2 ;;
esac
package_root="$workspace/package/rey-${release_version}-${target}"
runtime_dir="$package_root/bin/runtime"
dist_dir="$workspace/dist"

case "$target" in
    macos-arm64)
        [[ $(uname -s) == Darwin && $(uname -m) == arm64 ]] || { echo "release target $target requires a native macOS arm64 runner" >&2; exit 2; }
        real_clang=$(command -v clang)
        ;;
    macos-x86_64)
        [[ $(uname -s) == Darwin && $(uname -m) == x86_64 ]] || { echo "release target $target requires a native macOS x86_64 runner" >&2; exit 2; }
        real_clang=$(command -v clang)
        ;;
    linux-x86_64)
        [[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || { echo "release target $target requires a native Linux x86_64 runner" >&2; exit 2; }
        real_clang="$repo_root/scripts/zig-ld-linux-x86_64.sh"
        real_ir_clang=$(command -v clang)
        ir_target=x86_64-linux-musl
        ;;
    linux-aarch64)
        [[ $(uname -s) == Linux && $(uname -m) == aarch64 ]] || { echo "release target $target requires a native Linux aarch64 runner" >&2; exit 2; }
        real_clang="$repo_root/scripts/zig-ld-linux-aarch64.sh"
        real_ir_clang=$(command -v clang)
        ir_target=aarch64-linux-musl
        ;;
    windows-x86_64)
        case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) ;; *) echo "release target $target requires a native Windows runner with MSYS2" >&2; exit 2 ;; esac
        [[ $(uname -m) == x86_64 ]] || { echo "release target $target requires a native Windows x86_64 runner" >&2; exit 2; }
        real_clang=$(command -v clang)
        real_ir_clang="$real_clang"
        ir_target=x86_64-w64-windows-gnu
        ;;
    *)
        usage
        exit 2
        ;;
esac

if [[ -e "$workspace" ]]; then
    echo "error: release workspace already exists; set REY_RELEASE_WORKSPACE to a new absolute path: $workspace" >&2
    exit 2
fi
mkdir -p "$runtime_dir" "$package_root/bin" "$package_root/std" "$dist_dir"

# Build each runtime from tracked source so its ABI and LLVM bitcode match the
# compiler on that target. Windows uses Win32-native file and process I/O.
runtime_source="$repo_root/rey-compiler/runtime"
compiler=${CC:-clang}
if [[ $target == windows-* ]]; then
    runtime_compiler="$compiler"
    runtime_archiver=${AR:-llvm-ar}
    runtime_io_source=io_windows.c
elif [[ $target == linux-x86_64 ]]; then
    runtime_compiler="$repo_root/scripts/zig-ld-linux-x86_64.sh"
    runtime_archiver="$repo_root/scripts/zig-ar.sh"
    runtime_io_source=io.c
elif [[ $target == linux-aarch64 ]]; then
    runtime_compiler="$repo_root/scripts/zig-ld-linux-aarch64.sh"
    runtime_archiver="$repo_root/scripts/zig-ar.sh"
    runtime_io_source=io.c
else
    runtime_compiler="$compiler"
    runtime_archiver=${AR:-ar}
    runtime_io_source=io.c
fi
command -v "$runtime_compiler" >/dev/null 2>&1 || { echo "error: C compiler not found: $runtime_compiler" >&2; exit 2; }
command -v "$runtime_archiver" >/dev/null 2>&1 || { echo "error: archiver not found: $runtime_archiver" >&2; exit 2; }
CC="$runtime_compiler" AR="$runtime_archiver" MAKE="${MAKE:-make}" REY_RUNTIME_IO_SOURCE="$runtime_io_source" \
    bash "$repo_root/scripts/build-runtime.sh" --output-dir "$runtime_dir"

# Rebuild the reference interpreter from tracked source. Never use ignored
# compiler-source snapshots or a prebuilt native compiler as the bootstrap.
command -v cargo >/dev/null 2>&1 || { echo "error: cargo is required to build the source bootstrap" >&2; exit 2; }
cargo build --release --manifest-path "$repo_root/compiler/v1/Cargo.toml" \
    --target-dir "$workspace/bootstrap-target"
oracle="$workspace/bootstrap-target/release/rey-v0"
if [[ $target == windows-* ]]; then oracle="$oracle.exe"; fi

# The compiler resolves clang through PATH. On Unix, use a shim to lower the
# generated IR with native Clang and route final Linux links through Zig. On
# Windows, MSYS2 Clang/LLVM is the compiler and linker directly; a .cmd shim
# cannot reliably preserve quoted MSYS paths passed through cmd.exe.
linker_path=$PATH
if [[ $target != windows-* ]]; then
    mkdir -p "$workspace/linker-bin"
    cp "$repo_root/scripts/release-clang-shim.sh" "$workspace/linker-bin/clang"
    cp "$repo_root/scripts/release-llc-shim.sh" "$workspace/linker-bin/llc"
    chmod +x "$workspace/linker-bin/clang" "$workspace/linker-bin/llc"
    export REY_RELEASE_REAL_CLANG="$real_clang"
    export REY_RELEASE_REAL_IR_CLANG="$real_ir_clang"
    export REY_RELEASE_IR_TARGET="$ir_target"
    export REY_RELEASE_IR_NATIVE="$ir_native"
    linker_path="$workspace/linker-bin:$PATH"
fi
if [[ $target == windows-* ]]; then
    release_binary="$package_root/bin/rey.exe"
    stage1_binary="$workspace/stage1/rey.exe"
else
    release_binary="$package_root/bin/rey"
    stage1_binary="$workspace/stage1/rey"
fi
mkdir -p "$workspace/stage1"
PATH="$linker_path" "$oracle" run "$repo_root/rey-compiler/main.rey" -- \
    build "$repo_root/rey-compiler/main.rey" -o "$stage1_binary" --runtime-dir "$runtime_dir"
[[ -s "$stage1_binary" ]] || { echo "error: source bootstrap produced no binary: $stage1_binary" >&2; exit 1; }
chmod +x "$stage1_binary"
PATH="$linker_path" "$stage1_binary" build "$repo_root/rey-compiler/main.rey" \
    -o "$release_binary" --runtime-dir "$runtime_dir"
[[ -s "$release_binary" ]] || { echo "error: compiler build produced no binary: $release_binary" >&2; exit 1; }
chmod +x "$release_binary"
actual_version=$("$release_binary" --version)
if [[ "$actual_version" != "rey 0.3.1" ]]; then
    echo "error: built compiler reports '$actual_version', expected 'rey 0.3.1'" >&2
    exit 1
fi
for intermediate in "$release_binary.ll" "$release_binary.o"; do
    if [[ -f "$intermediate" ]]; then mv "$intermediate" "$workspace/"; fi
done
cp -R "$repo_root/std/src" "$package_root/std/src"

if [[ $target == windows-* ]]; then
    archive="$dist_dir/rey-${release_version}-${target}.zip"
    (cd "$workspace/package" && zip -qr "$archive" "rey-${release_version}-${target}")
else
    archive="$dist_dir/rey-${release_version}-${target}.tar.gz"
    (cd "$workspace/package" && tar -czf "$archive" "rey-${release_version}-${target}")
fi

[[ -s "$archive" ]] || { echo "error: release archive was not produced: $archive" >&2; exit 1; }
printf '%s\n' "$archive"

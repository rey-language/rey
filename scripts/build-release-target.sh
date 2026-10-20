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
seed_version=v0.3.0
seed_root_name="rey-${seed_version}-${target}"
real_ir_clang=
ir_target=
ir_native=false
workspace=${REY_RELEASE_WORKSPACE:-"$repo_root/build/release/$release_version/$target"}
case "$workspace" in
    /*) ;;
    *) echo "error: REY_RELEASE_WORKSPACE must be an absolute path" >&2; exit 2 ;;
esac
seed_dir="$workspace/seed"
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
        real_clang="$repo_root/scripts/zig-ld-windows-x86_64.sh"
        real_ir_clang=$(command -v clang)
        ir_target=x86_64-w64-windows-gnu
        ir_native=true
        seed_archive="$repo_root/releases/0.3.0/rey-v0.3.0-$target.zip"
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
mkdir -p "$seed_dir" "$runtime_dir" "$package_root/bin" "$package_root/std" "$dist_dir"
if [[ $target == windows-* ]]; then
    seed_archive="$repo_root/releases/0.3.0/rey-v0.3.0-$target.zip"
    [[ -f "$seed_archive" ]] || { echo "error: matching v0.3.0 bootstrap package is missing: $seed_archive" >&2; exit 1; }
    unzip -q "$seed_archive" -d "$seed_dir"
    seed_package="$seed_dir/$seed_root_name"
    seed_runtime="$seed_package/bin/runtime/librey_rt.a"
    if [[ ! -f "$seed_runtime" ]]; then
        echo "error: v0.3.0 package is missing its runtime: $seed_package" >&2
        exit 1
    fi
    cp "$seed_runtime" "$runtime_dir/librey_rt.a"
fi

# Rebuild Unix runtime archives from current source so their LLVM bitcode is
# compatible with the target runner's compiler. The Windows archive retains
# its shipped I/O object because runtime/io.c is currently POSIX-only.
runtime_source="$repo_root/rey-compiler/runtime"
vec_object="$workspace/vec.o"
compiler=${CC:-clang}
if [[ $target == windows-* ]]; then
    archive_tool=(zig ar)
    command -v "$compiler" >/dev/null 2>&1 || { echo "error: C compiler not found: $compiler" >&2; exit 2; }
    command -v "${archive_tool[0]}" >/dev/null 2>&1 || { echo "error: archiver not found: ${archive_tool[0]}" >&2; exit 2; }
    "$repo_root/scripts/zig-ld-windows-x86_64.sh" \
        -O3 -flto -fPIC -D_GNU_SOURCE -Wall -Wextra -Wno-unused-parameter -std=c11 \
        -I "$runtime_source" -c "$runtime_source/vec.c" -o "$vec_object"
    "${archive_tool[@]}" d "$runtime_dir/librey_rt.a" vec.o
    "${archive_tool[@]}" rcs "$runtime_dir/librey_rt.a" "$vec_object"
else
    runtime_compiler="$compiler"
    runtime_archiver=${AR:-ar}
    if [[ $target == linux-x86_64 ]]; then
        runtime_compiler="$repo_root/scripts/zig-ld-linux-x86_64.sh"
        runtime_archiver="$repo_root/scripts/zig-ar.sh"
    elif [[ $target == linux-aarch64 ]]; then
        runtime_compiler="$repo_root/scripts/zig-ld-linux-aarch64.sh"
        runtime_archiver="$repo_root/scripts/zig-ar.sh"
    fi
    command -v "$runtime_compiler" >/dev/null 2>&1 || { echo "error: C compiler not found: $runtime_compiler" >&2; exit 2; }
    command -v "$runtime_archiver" >/dev/null 2>&1 || { echo "error: archiver not found: $runtime_archiver" >&2; exit 2; }
    CC="$runtime_compiler" AR="$runtime_archiver" MAKE="${MAKE:-make}" \
        bash "$repo_root/scripts/build-runtime.sh" --output-dir "$runtime_dir"
fi

# Rebuild the reference interpreter from tracked source. The v0.3.0 native
# seed hardcodes /usr/bin/clang and bypasses PATH, so it cannot reliably use
# the platform linker wrappers. Never use ignored compiler-source snapshots.
command -v cargo >/dev/null 2>&1 || { echo "error: cargo is required to build the source bootstrap" >&2; exit 2; }
cargo build --release --manifest-path "$repo_root/compiler/v1/Cargo.toml" \
    --target-dir "$workspace/bootstrap-target"
oracle="$workspace/bootstrap-target/release/rey-v0"
if [[ $target == windows-* ]]; then oracle="$oracle.exe"; fi

# The compiler source resolves clang through PATH; route it to the native
# toolchain (Zig musl/MinGW on Linux/Windows). The IR shim also removes any
# inherited target data layout before clang derives the native layout.
mkdir -p "$workspace/linker-bin"
if [[ $target == windows-* ]]; then
    repo_root_windows=$(cygpath -w "$repo_root")
    cat > "$workspace/linker-bin/clang.cmd" <<EOF
@echo off
bash "$repo_root_windows\\scripts\\release-clang-shim.sh" %*
exit /b %ERRORLEVEL%
EOF
    cat > "$workspace/linker-bin/llc.cmd" <<EOF
@echo off
bash "$repo_root_windows\\scripts\\release-llc-shim.sh" %*
exit /b %ERRORLEVEL%
EOF
else
    cp "$repo_root/scripts/release-clang-shim.sh" "$workspace/linker-bin/clang"
    cp "$repo_root/scripts/release-llc-shim.sh" "$workspace/linker-bin/llc"
    chmod +x "$workspace/linker-bin/clang" "$workspace/linker-bin/llc"
fi
export REY_RELEASE_REAL_CLANG="$real_clang"
export REY_RELEASE_REAL_IR_CLANG="$real_ir_clang"
export REY_RELEASE_IR_TARGET="$ir_target"
export REY_RELEASE_IR_NATIVE="$ir_native"
linker_path="$workspace/linker-bin:$PATH"
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

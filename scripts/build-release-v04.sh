#!/bin/sh
# Build local v0.4 probe packages and evaluate the hard release conditions.
# This script does not publish, does not upload, and does not replace
# scripts/build-release-target.sh. A blocked verdict is still written to
# build/v04/release/readiness.json.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RELEASE="$ROOT/build/v04/release"
ENGINE=${REY_ENGINE:-"$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey"}
RUNTIME=${REY_RUNTIME:-"$ROOT/build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime"}
mkdir -p "$RELEASE"
export PATH=/usr/bin:/bin:/usr/sbin:/sbin
export CC=/usr/bin/clang
export CXX=/usr/bin/clang++
if [ -d "$ROOT/std/src" ]; then
    export REY_STD_PATH="$ROOT/std/src"
fi

python3 - "$RELEASE" "$ROOT" <<'PY'
import json, sys
from pathlib import Path
release = Path(sys.argv[1])
root = Path(sys.argv[2])
facts = {
    "imageWriter": "rey",
    "externalLinkerForImages": False,
    "driverBuiltBy": "v0.3.1-seed",
    "seedInvokesClang": True,
    "conformanceComplete": False,
    "concurrencyBoundaries": "not-evidenced",
    "bootstrapReproduced": False,
    "cacheInvalidation": "not-evidenced",
    "trustedIsolation": "not-evidenced",
    "scorecardSamples": 0,
    "regressionsResolved": False,
    "published": False,
}
(release / "release-facts.json").write_text(json.dumps(facts, indent=2) + "\n")
notes = """# v0.4 local package notes

These directories are unpublished probe packages. They are not a release.

The experimental Rey image writers emitted one scalar `bin/probe` per
platform when the v0.3.1 seed was available. That seed may invoke Clang to
build the driver. The image writers themselves do not invoke Clang, a
system linker, or an assembler. `bin/rey` is absent, so this is not an
installable `rey-package-1` toolchain and Capy was not asked to install it.

No bootstrap hash, conformance corpus, or benchmark sample is attached.
Speed and RSS targets in the execution plan are targets, not observations.
Hard conditions stay blocked until `readiness.json` says otherwise.

Limitations: x64 emission is the scalar subset. Regions and managed
aggregates are not lowered by that backend. Darwin x64 still exits with a
direct syscall inside an image that also requests libSystem. Linux amd64
and Darwin x64 runs on this Apple Silicon host are emulation. A Linux arm64
container is a VM on the native CPU, not a Linux hardware host. Windows is
native only when the evidence file says so.
"""
(release / "release-notes.md").write_text(notes)
platforms = {
    "darwin-arm64": "darwin-arm64",
    "darwin-x86_64": "darwin-x64",
    "linux-aarch64": "linux-arm64",
    "linux-x86_64": "linux-x64",
    "windows-x86_64": "windows-x64",
}
for platform, target in platforms.items():
    (release / (platform + ".json")).write_text(json.dumps({
        "platform": platform,
        "target": target,
        "build": "blocked",
        "built": "not-run",
        "structural": "not-run",
        "executed": "not-run",
        "run": "blocked",
        "bootstrap": "blocked",
        "package": "blocked",
        "nativeHardware": False,
        "emulator": False,
        "executionKind": "not-run",
        "note": "probe package has not been built yet",
    }, indent=2) + "\n")
PY

if [ ! -x "$ENGINE" ] || [ ! -d "$RUNTIME" ]; then
    echo "seed compiler is missing; probe packages stay blocked" >&2
    python3 "$ROOT/scripts/v04-release-readiness.py" "$RELEASE" || true
    exit 2
fi

STAGE=$(mktemp -d "$RELEASE/stage.XXXXXX")
cleanup() { rm -rf "$STAGE"; }
trap cleanup EXIT
mkdir -p "$STAGE/src"
python3 "$ROOT/scripts/embed-runtime.py"
python3 "$ROOT/scripts/embed-stdlib.py"
cp -R "$ROOT/rey-compiler/src/." "$STAGE/src/"
cp "$ROOT/rey-compiler/ir-main.rey" "$STAGE/src/main.rey"
"$ENGINE" build "$STAGE/src/main.rey" -o "$STAGE/rey-ir" --runtime-dir "$RUNTIME"
printf 'func main(): void { return; }\n' > "$STAGE/scalar.rey"

write_package() {
    platform=$1
    target=$2
    dest="$RELEASE/$platform"
    rm -rf "$dest"
    mkdir -p "$dest/bin"
    cp "$ROOT/packaging/rey-package-1.toml" "$dest/package.toml"
    "$STAGE/rey-ir" emit "$STAGE/scalar.rey" -o "$dest/bin/probe" --target "$target"
    chmod 755 "$dest/bin/probe"
    note="$dest/PACKAGE.txt"
    cat > "$note" <<EOF
rey-package-1 layout is incomplete.
bin/rey is not in this directory.
bin/probe is a scalar image emitted for $target by rey-compiler/ir-main.rey.
It is not a compiler bootstrap and it was not installed by Capy.
EOF
    (
        cd "$dest"
        shasum -a 256 package.toml PACKAGE.txt bin/probe > SHA256SUMS
    )
    runJson=$(python3 "$ROOT/tests/v04/platforms/native_run.py" --probe "$dest/bin/probe" --target "$target" --expect 0 || true)
    python3 - "$RELEASE" "$platform" "$target" "$dest/bin/probe" "$runJson" <<'PY'
import hashlib, json, sys
from pathlib import Path
release, platform, target, probe, raw = sys.argv[1:]
try:
    ran = json.loads(raw)
except json.JSONDecodeError:
    ran = {"status": "fail", "reason": raw[-500:], "nativeHardware": False, "emulator": False, "executionKind": "unparsed"}
digest = hashlib.sha256(Path(probe).read_bytes()).hexdigest()
status = ran.get("status")
if status == "pass":
    executed = "pass"
elif status == "structural-only":
    executed = "structural-only"
elif status == "fail":
    executed = "fail"
else:
    executed = "not-run"
nativeHardware = ran.get("nativeHardware") is True and executed == "pass"
emulator = ran.get("emulator") is True
# The release run gate is a native-hardware run. Rosetta, a Linux VM, and
# Wine stay in executed so the observation is visible, and stay out of run.
releaseRun = "pass" if executed == "pass" and nativeHardware and not emulator else "blocked"
record = {
    "platform": platform,
    "target": target,
    "build": "pass",
    "built": "pass",
    "structural": "pass",
    "executed": executed,
    "buildScope": "scalar-probe",
    "run": releaseRun,
    "bootstrap": "blocked",
    "package": "blocked",
    "nativeHardware": nativeHardware,
    "emulator": emulator,
    "executionKind": ran.get("executionKind", status),
    "returnCode": ran.get("returnCode"),
    "probeSha256": digest,
    "note": ran.get("reason") or ran.get("stderr") or "package has no bin/rey and was not installed by Capy; bootstrap was not reproduced",
}
if status == "structural-only":
    record["note"] = ran.get("reason", "structural only")
Path(release, platform + ".json").write_text(json.dumps(record, indent=2) + "\n")
print(platform + " built=pass structural=pass executed=" + executed + " releaseRun=" + releaseRun + " kind=" + str(record["executionKind"]))
PY
}

write_package darwin-arm64 darwin-arm64
write_package darwin-x86_64 darwin-x64
write_package linux-aarch64 linux-arm64
write_package linux-x86_64 linux-x64
write_package windows-x86_64 windows-x64

(
    cd "$RELEASE"
    shasum -a 256 */SHA256SUMS */package.toml *.json release-notes.md release-facts.json > SHA256SUMS || true
)

python3 "$ROOT/scripts/v04-release-readiness.py" "$RELEASE"

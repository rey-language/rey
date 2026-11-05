#!/usr/bin/env python3
"""Run scalar images from the experimental drivers on this host.

darwin-arm64 runs directly. darwin-x64 runs under Rosetta. linux-arm64 uses
the local ubuntu:22.04 image inside the Colima aarch64 VM. linux-x64 uses
ubuntu:22.04-amd64. On this Mac Colima registers Rosetta for x86-64 ELF and
leaves qemu-x86_64 disabled, so an amd64 run is qemu only after that binfmt
is switched. windows-x64 uses Wine in rey-v04-wine when that image is present.
"""
import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

root = Path(__file__).resolve().parents[3]
enginePath = Path(os.environ.get(
    "REY_ENGINE",
    root / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/rey",
))
runtimePath = Path(os.environ.get(
    "REY_RUNTIME",
    root / "build/release/v0.3.1/macos-arm64-winruntime/package/rey-v0.3.1-macos-arm64/bin/runtime",
))
reportPath = root / "build/v04/platforms/native-runs.json"
targets = ("darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64", "windows-x64")


def sha256(path):
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def runCommand(args, timeout=180, env=None, inputBytes=None):
    completed = subprocess.run(args, capture_output=True, timeout=timeout, env=env, input=inputBytes)
    return {
        "returnCode": completed.returncode,
        "stdout": completed.stdout.decode(errors="replace"),
        "stderr": completed.stderr.decode(errors="replace"),
    }


def buildDriver(stage, entryName, source):
    staged = stage / "src"
    if staged.exists():
        shutil.rmtree(staged)
    shutil.copytree(root / "rey-compiler" / "src", staged)
    shutil.copy(source, staged / "main.rey")
    binary = stage / entryName
    env = os.environ.copy()
    env["PATH"] = "/usr/bin:/bin:/usr/sbin:/sbin"
    env["CC"] = "/usr/bin/clang"
    env["CXX"] = "/usr/bin/clang++"
    std = root / "std" / "src"
    if std.is_dir():
        env["REY_STD_PATH"] = str(std)
    built = runCommand(
        [str(enginePath), "build", str(staged / "main.rey"), "-o", str(binary), "--runtime-dir", str(runtimePath)],
        timeout=600,
        env=env,
    )
    if built["returnCode"] != 0 or not binary.is_file():
        raise SystemExit(entryName + " build failed:\n" + built["stderr"] + built["stdout"])
    binary.chmod(0o755)
    return binary, env


def emitImage(driver, source, output, target, env):
    command = [str(driver), "build", str(source), "-o", str(output), "--target", target]
    if driver.name.startswith("rey-ir"):
        command = [str(driver), "emit", str(source), "-o", str(output), "--target", target]
    emitted = runCommand(command, timeout=120, env=env)
    if emitted["returnCode"] != 0 or not output.is_file():
        raise SystemExit(target + " emit failed:\n" + emitted["stderr"] + emitted["stdout"])
    output.chmod(0o755)
    return output.read_bytes()


def dockerImageId():
    docker, imageId, _imageName = dockerImageFor("linux/arm64")
    return docker, imageId


def dockerImageFor(platformName):
    """Pick a local image whose architecture matches the docker platform."""
    docker = shutil.which("docker")
    if not docker:
        return "", "", ""
    if platformName == "linux/amd64":
        names = ["ubuntu:22.04-amd64", "ubuntu:22.04"]
        acceptedArch = ("amd64", "x86_64", "")
    else:
        names = ["ubuntu:22.04"]
        acceptedArch = ("arm64", "aarch64")
    for name in names:
        inspected = runCommand(
            [docker, "image", "inspect", name, "--format", "{{.Id}}|{{.Architecture}}|end"],
            timeout=30,
        )
        if inspected["returnCode"] != 0:
            continue
        fields = inspected["stdout"].strip().split("|")
        if len(fields) != 3 or not fields[0].startswith("sha256:"):
            continue
        imageId, arch, _end = fields
        if arch not in acceptedArch:
            continue
        # An empty Architecture is a manifest index. Only the amd64 tag is safe to use.
        if arch == "" and not name.endswith("-amd64"):
            continue
        return docker, imageId, name
    return docker, "", ""


def runDocker(docker, imageId, platformName, payload):
    shell = "tar -xf - -C /work || exit 90; uname -sm; /work/probe"
    return runCommand(
        [docker, "run", "--rm", "--pull", "never", "-i", "--platform", platformName,
         "--network", "none", "--read-only", "--tmpfs", "/work:exec", imageId, "/bin/sh", "-c", shell],
        timeout=180,
        inputBytes=payload,
    )


def probeBytes(data):
    payload = io.BytesIO()
    with tarfile.open(fileobj=payload, mode="w") as archive:
        entry = tarfile.TarInfo("probe")
        entry.size = len(data)
        entry.mode = 0o700
        archive.addfile(entry, io.BytesIO(data))
    return payload.getvalue()


def signDarwin(path):
    runCommand(["/usr/bin/codesign", "--sign", "-", "--force", str(path)], timeout=30)
    path.chmod(0o755)


def executeLocal(path, expect, arch=None):
    signDarwin(path)
    command = [str(path)]
    if arch:
        command = ["/usr/bin/arch", arch, str(path)]
    ran = runCommand(command, timeout=20)
    return {
        "status": "pass" if ran["returnCode"] == expect else "fail",
        "returnCode": ran["returnCode"],
        "expect": expect,
        "stderr": ran["stderr"][-400:],
        "emulator": arch == "-x86_64",
        "nativeHardware": arch is None,
        "executionKind": "rosetta" if arch else "darwin-arm64-hardware",
    }


def executeLinux(docker, imageId, data, expect, platformName, imageName=""):
    ran = runDocker(docker, imageId, platformName, probeBytes(data))
    lines = [line for line in ran["stdout"].splitlines() if line.strip()]
    uname = lines[0] if lines else ""
    emulated = platformName == "linux/amd64"
    return {
        "status": "pass" if ran["returnCode"] == expect else "fail",
        "returnCode": ran["returnCode"],
        "expect": expect,
        "uname": uname,
        "stderr": ran["stderr"][-400:],
        "emulator": emulated,
        "nativeHardware": False,
        "cpuNative": platformName == "linux/arm64",
        "executionKind": "docker-qemu" if emulated else "docker-linux-vm-native-cpu",
        "image": imageId,
        "imageName": imageName,
    }


def wineImage():
    docker = shutil.which("docker") or "docker"
    for name in ("rey-v04-wine", "ubuntu:22.04"):
        inspected = runCommand([docker, "image", "inspect", name, "--format", "{{.Id}}"], timeout=30)
        imageId = inspected["stdout"].strip()
        if inspected["returnCode"] != 0 or not imageId.startswith("sha256:"):
            continue
        ran = runCommand(
            [docker, "run", "--rm", "--pull", "never", "--platform", "linux/amd64", imageId,
             "/bin/sh", "-c", "command -v wine64 || command -v wine64-stable || command -v wine"],
            timeout=60,
        )
        if ran["returnCode"] == 0:
            binary = ran["stdout"].strip().splitlines()[-1].strip()
            if binary:
                return imageId, binary
    return "", ""


def executeWine(docker, imageId, wineBin, data, expect):
    payload = io.BytesIO()
    with tarfile.open(fileobj=payload, mode="w") as archive:
        entry = tarfile.TarInfo("probe.exe")
        entry.size = len(data)
        entry.mode = 0o700
        archive.addfile(entry, io.BytesIO(data))
    shell = (
        "tar -xf - -C /tmp || exit 90; export WINEDEBUG=-all; export HOME=/tmp; "
        + wineBin + " /tmp/probe.exe"
    )
    ran = runCommand(
        [docker, "run", "--rm", "--pull", "never", "-i", "--platform", "linux/amd64",
         "--network", "none", imageId, "/bin/sh", "-c", shell],
        timeout=180,
        inputBytes=payload.getvalue(),
    )
    return {
        "status": "pass" if ran["returnCode"] == expect else "fail",
        "returnCode": ran["returnCode"],
        "expect": expect,
        "stderr": ran["stderr"][-400:],
        "emulator": True,
        "nativeHardware": False,
        "executionKind": "wine-docker",
        "wine": wineBin,
    }


def structuralWindows(reason):
    return {
        "status": "structural-only",
        "returnCode": None,
        "emulator": False,
        "nativeHardware": False,
        "executionKind": "structural-only",
        "reason": reason,
    }


def executeOne(path, data, target, expect, docker, imageId, wineBin, wineImageId=""):
    if target == "darwin-arm64":
        return executeLocal(path, expect)
    if target == "darwin-x64":
        return executeLocal(path, expect, "-x86_64")
    if target == "linux-arm64":
        docker, imageId, imageName = dockerImageFor("linux/arm64")
        if not imageId:
            return {"status": "fail", "reason": "ubuntu:22.04 arm64 is not present locally", "emulator": False, "nativeHardware": False}
        return executeLinux(docker, imageId, data, expect, "linux/arm64", imageName)
    if target == "linux-x64":
        docker, imageId, imageName = dockerImageFor("linux/amd64")
        if not imageId:
            return {"status": "fail", "reason": "ubuntu:22.04-amd64 is not present locally", "emulator": True, "nativeHardware": False}
        return executeLinux(docker, imageId, data, expect, "linux/amd64", imageName)
    if wineBin:
        return executeWine(docker, wineImageId or imageId, wineBin, data, expect)
    return structuralWindows("Wine is not installed in ubuntu:22.04 or rey-v04-wine; PE header and .pdata were checked structurally")


def singleProbe(argv):
    target = argv[argv.index("--target") + 1]
    expect = int(argv[argv.index("--expect") + 1])
    probe = Path(argv[argv.index("--probe") + 1])
    docker, imageId = dockerImageId()
    wineImageId, wineBin = ("", "")
    if target == "windows-x64" and docker:
        wineImageId, wineBin = wineImage()
    result = executeOne(probe, probe.read_bytes(), target, expect, docker, imageId, wineBin, wineImageId)
    print(json.dumps(result))
    if result["status"] == "fail":
        return 1
    return 0


def main():
    if "--probe" in sys.argv:
        return singleProbe(sys.argv)
    if not enginePath.is_file() or not runtimePath.is_dir():
        print("missing REY_ENGINE or REY_RUNTIME", file=sys.stderr)
        return 1
    docker, imageId = dockerImageId()
    wineImageId, wineBin = wineImage() if docker else ("", "")
    reportPath.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix="v04-native-run-"))
    records = []
    try:
        source = stage / "scalar.rey"
        source.write_text("func main(): void { return; }\n")
        nativeDriver, env = buildDriver(stage / "native", "rey-native", root / "rey-compiler" / "native-main.rey")
        irDriver, env = buildDriver(stage / "ir", "rey-ir", root / "rey-compiler" / "ir-main.rey")
        fixture, env = buildDriver(stage / "fixture", "platform-fixture", root / "tests" / "v04" / "platforms" / "main.rey")
        imageDir = stage / "images"
        imageDir.mkdir()
        fixtureRun = runCommand([str(fixture), str(imageDir)], timeout=120, env=env)
        if fixtureRun["returnCode"] != 0:
            raise SystemExit("exit-42 fixture failed:\n" + fixtureRun["stderr"] + fixtureRun["stdout"])
        for target in targets:
            nativeOut = stage / ("native-" + target)
            irOut = stage / ("ir-" + target)
            nativeBytes = emitImage(nativeDriver, source, nativeOut, target, env)
            irBytes = emitImage(irDriver, source, irOut, target, env)
            same = nativeBytes == irBytes
            nativeResult = executeOne(nativeOut, nativeBytes, target, 0, docker, imageId, wineBin, wineImageId)
            exit42 = imageDir / {
                "darwin-arm64": "exit42-darwin-arm64",
                "darwin-x64": "exit42-darwin-x64",
                "linux-arm64": "exit42-linux-arm64",
                "linux-x64": "exit42-linux-x64",
                "windows-x64": "exit42-windows-x64",
            }[target]
            exit42.chmod(0o755)
            exitResult = executeOne(exit42, exit42.read_bytes(), target, 42, docker, imageId, wineBin, wineImageId)
            record = {
                "target": target,
                "driversAgree": same,
                "nativeBytes": len(nativeBytes),
                "sha256": hashlib.sha256(nativeBytes).hexdigest(),
                "voidExit": nativeResult,
                "exit42": exitResult,
            }
            records.append(record)
            print(target + " agree=" + str(same) + " void=" + str(nativeResult.get("status"))
                  + " exit42=" + str(exitResult.get("status")))
        evidenceDir = root / "build/v04/platforms/images"
        if evidenceDir.exists():
            shutil.rmtree(evidenceDir)
        shutil.copytree(imageDir, evidenceDir)
        for target in targets:
            shutil.copy(stage / ("native-" + target), evidenceDir / ("driver-" + target))
    finally:
        shutil.rmtree(stage, ignore_errors=True)
    failed = []
    for record in records:
        if not record["driversAgree"]:
            failed.append(record["target"] + " drivers differ")
        for label in ("voidExit", "exit42"):
            status = record[label]["status"]
            if record["target"] == "windows-x64" and status == "structural-only":
                continue
            if status != "pass":
                failed.append(record["target"] + " " + label + " " + status)
    report = {
        "passed": len(failed) == 0,
        "failures": failed,
        "dockerImage": imageId,
        "wine": wineBin,
        "records": records,
    }
    reportPath.write_text(json.dumps(report, indent=2) + "\n")
    print("wrote " + str(reportPath))
    if failed:
        print("\n".join(failed), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

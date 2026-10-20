# Rey Programming Language Installer for Windows (PowerShell)
# https://github.com/rey-language/rey
#
# Usage:
#   irm https://raw.githubusercontent.com/rey-language/rey/master/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Version = "v0.3.0"
$RepoUrl = "https://github.com/rey-language/rey"
$StdRepoUrl = "https://github.com/IMisbahk/std.git"

$InstallDir = if ($env:REY_INSTALL_DIR) { $env:REY_INSTALL_DIR } else { "$env:USERPROFILE\.rey" }
$BinDir = "$InstallDir\bin"
$StdDir = "$InstallDir\std"

Write-Host ""
Write-Host "   ____             " -ForegroundColor Cyan
Write-Host "  / __ \___  __  __   Rey Programming Language Installer" -ForegroundColor Cyan
Write-Host " / /_/ / _ \/ / / /   Release $Version" -ForegroundColor Cyan
Write-Host "/ _, _/  __/ /_/ /    https://rey-lang.org" -ForegroundColor Cyan
Write-Host "/_/ |_|\___/\__, /   " -ForegroundColor Cyan
Write-Host "           /____/    " -ForegroundColor Cyan
Write-Host ""

# Rey emits native code; on Windows use the MSYS2 UCRT64 Clang toolchain.
$Clang = Get-Command clang.exe -ErrorAction SilentlyContinue
if ($Clang) {
    Write-Host "==> Clang found; skipping toolchain installation." -ForegroundColor Blue
} else {
    $MsysCandidates = @(
        $env:MSYS2_ROOT,
        "$env:SystemDrive\msys64",
        "$env:ProgramFiles\MSYS2",
        "$env:LOCALAPPDATA\Programs\MSYS2",
        "C:\tools\msys64"
    ) | Where-Object { $_ }
    $MsysRoot = $MsysCandidates | Where-Object { Test-Path (Join-Path $_ "usr\bin\bash.exe") } | Select-Object -First 1

    if (-not $MsysRoot) {
        if (Get-Command winget.exe -ErrorAction SilentlyContinue) {
            Write-Host "==> Installing MSYS2..." -ForegroundColor Blue
            & winget.exe install --id MSYS2.MSYS2 --exact --scope user --silent --location "$env:LOCALAPPDATA\Programs\MSYS2" --accept-source-agreements --accept-package-agreements
            $WingetExit = $LASTEXITCODE
            if ($WingetExit -notin @(0, 2)) { throw "MSYS2 installation failed (winget exit $WingetExit)." }
        } else {
            throw "Clang is missing. Install MSYS2 or Windows Package Manager, then rerun this installer."
        }

        for ($Attempt = 0; -not $MsysRoot -and $Attempt -lt 60; $Attempt++) {
            Start-Sleep -Seconds 2
            $MsysRoot = $MsysCandidates | Where-Object { Test-Path (Join-Path $_ "usr\bin\bash.exe") } | Select-Object -First 1
        }
    }

    if (-not $MsysRoot) { throw "MSYS2 was installed, but bash.exe was not found. Set MSYS2_ROOT and retry." }

    $MsysBash = Join-Path $MsysRoot "usr\bin\bash.exe"
    Write-Host "==> Updating MSYS2 and installing Clang..." -ForegroundColor Blue
    & $MsysBash -lc "pacman -Syu --noconfirm || true"
    & $MsysBash -lc "pacman -Syu --noconfirm"
    if ($LASTEXITCODE -ne 0) { throw "MSYS2 package update failed (pacman exit $LASTEXITCODE)." }
    & $MsysBash -lc "pacman -S --needed --noconfirm mingw-w64-ucrt-x86_64-clang"
    if ($LASTEXITCODE -ne 0) { throw "MSYS2 Clang installation failed (pacman exit $LASTEXITCODE)." }

    $ClangBin = Join-Path $MsysRoot "ucrt64\bin"
    if (-not (Test-Path (Join-Path $ClangBin "clang.exe"))) { throw "MSYS2 Clang was not found under $ClangBin." }

    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (($UserPath -split ';') -notcontains $ClangBin) {
        [Environment]::SetEnvironmentVariable("Path", "$ClangBin;$UserPath", "User")
    }
    $env:Path = "$ClangBin;$env:Path"
    $Clang = Get-Command clang.exe -ErrorAction SilentlyContinue
    if (-not $Clang) { throw "Clang was installed but is still unavailable in PATH." }
}

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $StdDir | Out-Null

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path 2>$null

$LocalZip = "$ScriptDir\releases\0.3.0\rey-v0.3.0-windows-x86_64.zip"

if (Test-Path $LocalZip) {
    Write-Host "==> Found local release package: $LocalZip" -ForegroundColor Blue
    $TempExtract = Join-Path $env:TEMP "rey-install-$(Get-Random)"
    Expand-Archive -Path $LocalZip -DestinationPath $TempExtract -Force
    $SourceDir = Join-Path $TempExtract "rey-v0.3.0-windows-x86_64"
    Copy-Item "$SourceDir\bin\*" -Destination $BinDir -Recurse -Force
    if (Test-Path "$StdDir\src") { Remove-Item "$StdDir\src" -Recurse -Force }
    Copy-Item "$SourceDir\std\src" -Destination "$StdDir\src" -Recurse -Force
    Remove-Item $TempExtract -Recurse -Force
} else {
    $DownloadUrl = "$RepoUrl/releases/download/$Version/rey-$Version-windows-x86_64.zip"
    Write-Host "==> Downloading Rey $Version from $DownloadUrl..." -ForegroundColor Blue
    $TempZip = Join-Path $env:TEMP "rey-$Version.zip"
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempZip -UseBasicParsing
    $TempExtract = Join-Path $env:TEMP "rey-install-$(Get-Random)"
    Expand-Archive -Path $TempZip -DestinationPath $TempExtract -Force
    $SourceDir = Join-Path $TempExtract "rey-$Version-windows-x86_64"
    Copy-Item "$SourceDir\bin\*" -Destination $BinDir -Recurse -Force
    if (Test-Path "$StdDir\src") { Remove-Item "$StdDir\src" -Recurse -Force }
    Copy-Item "$SourceDir\std\src" -Destination "$StdDir\src" -Recurse -Force
    Remove-Item $TempZip -Force
    Remove-Item $TempExtract -Recurse -Force
}

# Set environment variables for current session and user profile
[Environment]::SetEnvironmentVariable("REY_HOME", $InstallDir, "User")
[Environment]::SetEnvironmentVariable("REY_STD_PATH", "$StdDir\src", "User")

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$BinDir;$UserPath", "User")
    Write-Host "==> Added $BinDir to User Path environment variable" -ForegroundColor Green
}

$env:REY_HOME = $InstallDir
$env:REY_STD_PATH = "$StdDir\src"
$env:Path = "$BinDir;$env:Path"

Write-Host "==> Rey compiler and Standard Library installed into: $InstallDir" -ForegroundColor Green
Write-Host ""
Write-Host "Installation complete! Restart your PowerShell or run:" -ForegroundColor Green
Write-Host "    `$env:Path = `"$BinDir;`$env:Path`""
Write-Host "Then run:"
Write-Host "    rey run main.rey" -ForegroundColor Cyan
Write-Host ""

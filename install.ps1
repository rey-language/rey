# Rey Programming Language Installer for Windows (PowerShell)
# https://github.com/rey-language/rey
#
# Usage:
#   irm https://raw.githubusercontent.com/rey-language/rey/master/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Version = "v0.3.1"
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

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $StdDir | Out-Null

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path 2>$null

$LocalZip = "$ScriptDir\releases\0.3.1\rey-v0.3.1-windows-x86_64.zip"

if (Test-Path $LocalZip) {
    Write-Host "==> Found local release package: $LocalZip" -ForegroundColor Blue
    $TempExtract = Join-Path $env:TEMP "rey-install-$(Get-Random)"
    Expand-Archive -Path $LocalZip -DestinationPath $TempExtract -Force
    $SourceDir = Join-Path $TempExtract "rey-v0.3.1-windows-x86_64"
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

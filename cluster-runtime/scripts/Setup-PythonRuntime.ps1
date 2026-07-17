# Python 3.13 Runtime Setup Script
# Downloads the python-build-standalone distribution and stages it
# in src-tauri/resources/python/ for bundling with the Tauri app.
#
# Usage:
#   scripts/Setup-PythonRuntime.ps1
#   scripts/Setup-PythonRuntime.ps1 -PythonVersion "3.13.4"
#   scripts/Setup-PythonRuntime.ps1 -Force   # re-download even if already staged

param(
    [string]$PythonVersion = "3.13.3",
    [switch]$Force = $false
)

$ErrorActionPreference = "Stop"

# ─── Configuration ────────────────────────────────────────────────────────────

$ScriptDir     = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot      = Split-Path -Parent $ScriptDir
$ResourcesDir  = Join-Path $RepoRoot "src-tauri" "resources"
$PythonDir     = Join-Path $ResourcesDir "python"
$TempDir       = Join-Path $env:TEMP "cluster_runtime_python_setup"

# python-build-standalone release page:
# https://github.com/indygreg/python-build-standalone/releases
$BaseUrl = "https://github.com/indygreg/python-build-standalone/releases/download"

# For Windows x86_64, the install_only flavour gives a lean self-contained
# Python without the full stdlib source tree.
$Arch    = "x86_64"
$Tag     = "20250702"  # Update this to the latest release tag as needed
$Flavour = "install_only"

# The download filename pattern for python-build-standalone:
# cpython-<version>+<tag>-<arch>-pc-windows-msvc-install_only.tar.gz
$FileName = "cpython-${PythonVersion}+${Tag}-${Arch}-pc-windows-msvc-${Flavour}.tar.gz"
$DownloadUrl = "${BaseUrl}/${Tag}/${FileName}"

# ─── Helpers ──────────────────────────────────────────────────────────────────

function Write-Step([string]$msg) {
    Write-Host ""
    Write-Host ">> $msg" -ForegroundColor Cyan
}

function Write-Ok([string]$msg) {
    Write-Host "   $msg" -ForegroundColor Green
}

function Write-Warn([string]$msg) {
    Write-Host "   WARNING: $msg" -ForegroundColor Yellow
}

# ─── Main ─────────────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "Cluster Runtime — Python $PythonVersion Setup" -ForegroundColor White
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray

# Check if already staged
$PythonExe = Join-Path $PythonDir "python.exe"
if ((Test-Path $PythonExe) -and -not $Force) {
    Write-Step "Python already staged"
    $VersionOutput = & $PythonExe --version 2>&1
    Write-Ok "Found: $VersionOutput"
    Write-Ok "Location: $PythonDir"
    Write-Host ""
    Write-Host "To re-download, run with -Force flag." -ForegroundColor DarkGray
    exit 0
}

# Ensure directories exist
Write-Step "Preparing directories"
New-Item -ItemType Directory -Force -Path $ResourcesDir | Out-Null
New-Item -ItemType Directory -Force -Path $TempDir      | Out-Null
Write-Ok "Resources dir: $ResourcesDir"

# Download
Write-Step "Downloading Python $PythonVersion"
Write-Host "   Source: $DownloadUrl" -ForegroundColor DarkGray
$ArchivePath = Join-Path $TempDir $FileName

if (Test-Path $ArchivePath) {
    Write-Warn "Cached archive found at $ArchivePath, skipping download."
    Write-Warn "Delete the file manually to force a fresh download."
} else {
    try {
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $ArchivePath -UseBasicParsing
        Write-Ok "Downloaded: $ArchivePath"
    } catch {
        Write-Host ""
        Write-Host "ERROR: Download failed." -ForegroundColor Red
        Write-Host "       URL: $DownloadUrl" -ForegroundColor Red
        Write-Host "       $_" -ForegroundColor Red
        Write-Host ""
        Write-Host "Possible fixes:" -ForegroundColor Yellow
        Write-Host "  1. Check the release tag at: https://github.com/indygreg/python-build-standalone/releases"
        Write-Host "  2. Update `$Tag` in this script to the latest release."
        Write-Host "  3. Check your internet connection."
        exit 1
    }
}

# Extract
Write-Step "Extracting archive"
$ExtractDir = Join-Path $TempDir "extract"
if (Test-Path $ExtractDir) { Remove-Item $ExtractDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $ExtractDir | Out-Null

try {
    tar -xzf $ArchivePath -C $ExtractDir
    Write-Ok "Extracted to: $ExtractDir"
} catch {
    Write-Host "ERROR: Extraction failed: $_" -ForegroundColor Red
    exit 1
}

# The archive extracts to a `python/` subdirectory
$ExtractedPython = Join-Path $ExtractDir "python"
if (-not (Test-Path $ExtractedPython)) {
    # Try listing what was actually extracted
    $Contents = Get-ChildItem $ExtractDir | Select-Object -ExpandProperty Name
    Write-Host "ERROR: Expected 'python/' in archive, got: $Contents" -ForegroundColor Red
    exit 1
}

# Stage
Write-Step "Staging Python distribution"
if (Test-Path $PythonDir) {
    Remove-Item $PythonDir -Recurse -Force
    Write-Ok "Removed old distribution."
}

Move-Item $ExtractedPython $PythonDir
Write-Ok "Staged to: $PythonDir"

# Verify
Write-Step "Verifying installation"
$PythonExe = Join-Path $PythonDir "python.exe"
if (-not (Test-Path $PythonExe)) {
    Write-Host "ERROR: python.exe not found at $PythonExe" -ForegroundColor Red
    exit 1
}

$VersionOutput = & $PythonExe --version 2>&1
Write-Ok "Verified: $VersionOutput"
Write-Ok "Executable: $PythonExe"

# Cleanup temp extraction folder (keep the downloaded archive for caching)
Remove-Item $ExtractDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host "Python $PythonVersion is ready." -ForegroundColor Green
Write-Host "Run 'cargo tauri dev' to start the application." -ForegroundColor White
Write-Host ""

# Python 3.10 Runtime Setup Script
# Downloads the python-build-standalone distribution and stages it
# in src-tauri/resources/python/ for bundling with the Tauri app.
#
# We ship Python 3.10.x so both Dask and Ray install cleanly on Windows
# (Ray Windows wheels do not support Python 3.13+).
#
# Usage:
#   scripts/Setup-PythonRuntime.ps1
#   scripts/Setup-PythonRuntime.ps1 -PythonVersion "3.10.11"
#   scripts/Setup-PythonRuntime.ps1 -Force   # re-download even if already staged

param(
    # 3.10.10 is not published by python-build-standalone; we remap to 3.10.11.
    [string]$PythonVersion = "3.10.10",
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
# https://github.com/astral-sh/python-build-standalone/releases
$BaseUrl = "https://github.com/astral-sh/python-build-standalone/releases/download"

# Known (version -> release tag) pairs for Windows x86_64 install_only builds.
$KnownBuilds = @{
    "3.10.11" = "20230507"
    "3.10.16" = "20241219"
    "3.10.19" = "20251010"
}

$Arch    = "x86_64"
$Flavour = "install_only"

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

function Resolve-PythonBuild([string]$requestedVersion) {
    if ($KnownBuilds.ContainsKey($requestedVersion)) {
        return @{
            Version = $requestedVersion
            Tag     = $KnownBuilds[$requestedVersion]
        }
    }

    if ($requestedVersion -eq "3.10.10") {
        Write-Warn "Exact Python 3.10.10 is not published by python-build-standalone."
        Write-Warn "Using 3.10.11 (closest available patch) instead."
        return @{
            Version = "3.10.11"
            Tag     = $KnownBuilds["3.10.11"]
        }
    }

    throw "Unknown Python version '$requestedVersion'. Add it to `$KnownBuilds` in this script, or pick one of: $($KnownBuilds.Keys -join ', ')"
}

# ─── Main ─────────────────────────────────────────────────────────────────────

$Build = Resolve-PythonBuild $PythonVersion
$PythonVersion = $Build.Version
$Tag = $Build.Tag

Write-Host ""
Write-Host "Cluster Runtime — Python $PythonVersion Setup" -ForegroundColor White
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host "Target: Python 3.10.x (Dask + Ray compatible on Windows)" -ForegroundColor DarkGray

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
$FileName = "cpython-${PythonVersion}+${Tag}-${Arch}-pc-windows-msvc-${Flavour}.tar.gz"
$DownloadUrl = "${BaseUrl}/${Tag}/${FileName}"

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
        Write-Host "  1. Check releases at: https://github.com/astral-sh/python-build-standalone/releases"
        Write-Host "  2. Add a matching version/tag pair to `$KnownBuilds` in this script."
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

if ($VersionOutput -notmatch "3\.10\.") {
    Write-Warn "Expected Python 3.10.x for Dask/Ray compatibility, got: $VersionOutput"
}

# Cleanup temp extraction folder (keep the downloaded archive for caching)
Remove-Item $ExtractDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host "Python $PythonVersion is ready." -ForegroundColor Green
Write-Host "Run 'cargo tauri dev' to start the application." -ForegroundColor White
Write-Host ""

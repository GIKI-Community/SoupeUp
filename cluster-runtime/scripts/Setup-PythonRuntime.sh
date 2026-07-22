#!/usr/bin/env bash
# Download python-build-standalone for Linux and stage it on this machine.
# Python is NOT baked into cluster-runtime-server — each host runs this once.
#
# Usage:
#   scripts/Setup-PythonRuntime.sh
#   scripts/Setup-PythonRuntime.sh --dest /var/lib/cluster-runtime/python
#   scripts/Setup-PythonRuntime.sh --force
#   CLUSTER_RUNTIME_DATA_DIR=/var/lib/cluster-runtime scripts/Setup-PythonRuntime.sh
#
# Default dest: $CLUSTER_RUNTIME_DATA_DIR/python, else ./data/python

set -euo pipefail

PYTHON_VERSION="${PYTHON_VERSION:-3.10.11}"
TAG="${TAG:-20230507}"
ARCH="x86_64"
FLAVOUR="install_only"
FORCE=0
DEST=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dest)
      DEST="$2"
      shift 2
      ;;
    --force|-f)
      FORCE=1
      shift
      ;;
    --version)
      PYTHON_VERSION="$2"
      shift 2
      ;;
    --help|-h)
      sed -n '2,12p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 1
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ -z "$DEST" ]]; then
  if [[ -n "${CLUSTER_RUNTIME_DATA_DIR:-}" ]]; then
    DEST="${CLUSTER_RUNTIME_DATA_DIR}/python"
  else
    DEST="${REPO_ROOT}/data/python"
  fi
fi

BASE_URL="https://github.com/astral-sh/python-build-standalone/releases/download"
FILE_NAME="cpython-${PYTHON_VERSION}+${TAG}-${ARCH}-unknown-linux-gnu-${FLAVOUR}.tar.gz"
DOWNLOAD_URL="${BASE_URL}/${TAG}/${FILE_NAME}"
TEMP_DIR="${TMPDIR:-/tmp}/cluster_runtime_python_setup"
ARCHIVE_PATH="${TEMP_DIR}/${FILE_NAME}"

step() { echo; echo ">> $*"; }
ok() { echo "   $*"; }
warn() { echo "   WARNING: $*" >&2; }

echo
echo "Cluster Runtime — Python ${PYTHON_VERSION} Setup (Linux)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

PYTHON_BIN="${DEST}/bin/python3"
if [[ -x "$PYTHON_BIN" && "$FORCE" -eq 0 ]]; then
  step "Python already staged"
  ok "Found: $($PYTHON_BIN --version 2>&1)"
  ok "Location: $DEST"
  echo
  echo "To re-download, run with --force."
  exit 0
fi

step "Preparing directories"
mkdir -p "$TEMP_DIR"
ok "Dest: $DEST"

step "Downloading Python ${PYTHON_VERSION}"
echo "   Source: $DOWNLOAD_URL"
if [[ -f "$ARCHIVE_PATH" && "$FORCE" -eq 0 ]]; then
  warn "Cached archive found at $ARCHIVE_PATH, skipping download."
else
  curl -fL --retry 3 -o "$ARCHIVE_PATH" "$DOWNLOAD_URL"
  ok "Downloaded: $ARCHIVE_PATH"
fi

step "Extracting archive"
EXTRACT_DIR="${TEMP_DIR}/extract"
rm -rf "$EXTRACT_DIR"
mkdir -p "$EXTRACT_DIR"
tar -xzf "$ARCHIVE_PATH" -C "$EXTRACT_DIR"

EXTRACTED="${EXTRACT_DIR}/python"
if [[ ! -d "$EXTRACTED" ]]; then
  echo "ERROR: Expected 'python/' in archive, contents:" >&2
  ls -la "$EXTRACT_DIR" >&2
  exit 1
fi

step "Staging Python distribution"
rm -rf "$DEST"
mkdir -p "$(dirname "$DEST")"
mv "$EXTRACTED" "$DEST"
ok "Staged to: $DEST"

step "Verifying installation"
PYTHON_BIN="${DEST}/bin/python3"
if [[ ! -x "$PYTHON_BIN" ]]; then
  # some builds use bin/python
  PYTHON_BIN="${DEST}/bin/python"
fi
if [[ ! -x "$PYTHON_BIN" ]]; then
  echo "ERROR: python binary not found under $DEST/bin" >&2
  exit 1
fi

VER="$("$PYTHON_BIN" --version 2>&1)"
ok "Verified: $VER"
ok "Executable: $PYTHON_BIN"

if [[ "$VER" != *"3.10."* ]]; then
  warn "Expected Python 3.10.x for Dask/Ray compatibility, got: $VER"
fi

rm -rf "$EXTRACT_DIR"

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Python ${PYTHON_VERSION} is ready on this machine."
echo "Point the server at it with:"
echo "  export CLUSTER_RUNTIME_PYTHON_DIR=$DEST"
echo "  # or: cluster-runtime-server --python-dir $DEST"
echo "  # or: export CLUSTER_RUNTIME_DATA_DIR=$(dirname "$DEST")   # if dest is \$DATA/python"
echo

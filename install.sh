#!/usr/bin/env bash
# install.sh — One-line installer for gotify-rmcp
#
# Downloads the gotify binary to ~/.local/bin and writes a starter .env file.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/jmagar/rgotify/main/install.sh | bash
#   # or locally:
#   bash install.sh [--version 0.1.0] [--bin-dir /usr/local/bin]
#
# Options:
#   --version VERSION   Install a specific release (default: latest)
#   --bin-dir DIR       Install binary to DIR (default: ~/.local/bin)
#   --no-env            Skip writing the starter ~/.gotify/.env file

set -euo pipefail

REPO="jmagar/rgotify"
BINARY="gotify"
SERVICE="gotify"
DEFAULT_BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/.${SERVICE}"
VERSION=""
BIN_DIR=""
WRITE_ENV=true

# ── Argument parsing ──────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="${2:?--version requires a value}"
      shift 2
      ;;
    --bin-dir)
      BIN_DIR="${2:?--bin-dir requires a value}"
      shift 2
      ;;
    --no-env)
      WRITE_ENV=false
      shift
      ;;
    -h|--help)
      grep '^#' "$0" | head -20 | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

BIN_DIR="${BIN_DIR:-${DEFAULT_BIN_DIR}}"

# ── Pre-flight checks ─────────────────────────────────────────────────────────
preflight() {
  local errors=0

  echo "Pre-flight checks..."

  # 1. OS / arch
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "${arch}" in
    x86_64)        arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *) echo "  ✗ Unsupported arch: ${arch}"; (( errors++ )) ;;
  esac
  if [[ "${os}" != "linux" && "${os}" != "darwin" ]]; then
    echo "  ✗ Unsupported OS: ${os} (linux or darwin required)"; (( errors++ ))
  else
    echo "  ✓ Platform: ${os}/${arch}"
  fi

  # 2. Required tools
  for cmd in curl tar grep; do
    if command -v "${cmd}" >/dev/null 2>&1; then
      echo "  ✓ ${cmd}: $(command -v "${cmd}")"
    else
      echo "  ✗ ${cmd}: not found (required)"; (( errors++ ))
    fi
  done

  # 3. Disk space (need at least 50 MB)
  local free_mb
  free_mb="$(df -k "${HOME}" | awk 'NR==2{printf "%d", $4/1024}')"
  if (( free_mb < 50 )); then
    echo "  ✗ Disk space: only ${free_mb}MB free in ${HOME} (need 50MB)"; (( errors++ ))
  else
    echo "  ✓ Disk space: ${free_mb}MB free"
  fi

  # 4. Install directory writable (or can be created)
  if mkdir -p "${BIN_DIR}" 2>/dev/null && [[ -w "${BIN_DIR}" ]]; then
    echo "  ✓ Install dir: ${BIN_DIR} (writable)"
  else
    echo "  ✗ Install dir: ${BIN_DIR} (not writable)"; (( errors++ ))
  fi

  # 5. PATH check (warn only)
  if echo ":${PATH}:" | grep -q ":${HOME}/.local/bin:"; then
    echo "  ✓ PATH: ~/.local/bin is present"
  else
    echo "  ⚠  PATH: ~/.local/bin not in PATH — will print instructions after install"
  fi

  # 6. Required env vars (warn only — can be set post-install)
  if [[ -n "${GOTIFY_URL:-}" ]]; then
    echo "  ✓ GOTIFY_URL: set"
  else
    echo "  ⚠  GOTIFY_URL: not set (required before running the server)"
  fi
  if [[ -n "${GOTIFY_CLIENT_TOKEN:-}" ]]; then
    echo "  ✓ GOTIFY_CLIENT_TOKEN: set"
  else
    echo "  ⚠  GOTIFY_CLIENT_TOKEN: not set (required for management operations)"
  fi
  if [[ -n "${GOTIFY_APP_TOKEN:-}" ]]; then
    echo "  ✓ GOTIFY_APP_TOKEN: set"
  else
    echo "  ⚠  GOTIFY_APP_TOKEN: not set (required for sending messages)"
  fi

  # 7. Port availability (warn only)
  local port="${GOTIFY_MCP_PORT:-9158}"
  if ss -tlnp 2>/dev/null | awk '{print $4}' | grep -q ":${port}$"; then
    echo "  ⚠  Port ${port}: already in use (change GOTIFY_MCP_PORT if needed)"
  else
    echo "  ✓ Port ${port}: available"
  fi

  echo ""
  if (( errors > 0 )); then
    echo "  ✗ Pre-flight failed with ${errors} error(s). Fix them and re-run."
    return 1
  fi
  echo "  ✓ Pre-flight passed — proceeding with install"
  return 0
}

preflight

# ── Detect target triple ──────────────────────────────────────────────────────
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "${ARCH}" in
  x86_64)        ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *)
    echo "Unsupported architecture: ${ARCH}" >&2
    exit 1
    ;;
esac

case "${OS}" in
  linux)  TARGET="${ARCH}-unknown-linux-musl" ;;
  darwin) TARGET="${ARCH}-apple-darwin" ;;
  *)
    echo "Unsupported OS: ${OS}" >&2
    exit 1
    ;;
esac

# ── Resolve version ───────────────────────────────────────────────────────────
if [[ -z "${VERSION}" ]]; then
  echo "Fetching latest release..."
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | \
    grep '"tag_name"' | sed 's/.*"tag_name": *"v\?\([^"]*\)".*/\1/')"
  if [[ -z "${VERSION}" ]]; then
    echo "Could not determine latest version. Use --version to specify one." >&2
    exit 1
  fi
fi
echo "Installing gotify-rmcp v${VERSION} (${TARGET})..."

# ── Download binary ───────────────────────────────────────────────────────────
ARCHIVE="${BINARY}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ARCHIVE}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "Downloading ${DOWNLOAD_URL}..."
curl -fsSL "${DOWNLOAD_URL}" -o "${TMP_DIR}/${ARCHIVE}"
tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "${TMP_DIR}"

BINARY_PATH="${TMP_DIR}/${BINARY}"
if [[ ! -f "${BINARY_PATH}" ]]; then
  # Some archives nest under a directory
  BINARY_PATH="$(find "${TMP_DIR}" -name "${BINARY}" -type f | head -1)"
fi
if [[ -z "${BINARY_PATH}" || ! -f "${BINARY_PATH}" ]]; then
  echo "Could not find ${BINARY} binary in archive ${ARCHIVE}" >&2
  exit 1
fi

# ── Install binary to ~/.local/bin ────────────────────────────────────────────
mkdir -p "${BIN_DIR}"
install -m 755 "${BINARY_PATH}" "${BIN_DIR}/${BINARY}"
echo "  ✓ Installed: ${BIN_DIR}/${BINARY}"

# ── Create data directory ─────────────────────────────────────────────────────
mkdir -p "${DATA_DIR}"
echo "  ✓ Data directory: ${DATA_DIR}"

# ── Write starter ~/.gotify/.env ──────────────────────────────────────────────
if [[ "${WRITE_ENV}" == "true" ]]; then
  ENV_FILE="${DATA_DIR}/.env"
  if [[ ! -f "${ENV_FILE}" ]]; then
    cat > "${ENV_FILE}" << 'EOF'
# gotify-rmcp — fill in your values, then: rgotify serve
#
# Token types:
#   Client tokens (C...) — management: list apps, clients, messages
#   App tokens    (A...) — sending:    push notifications to an app
#
# Get tokens from your Gotify dashboard → Apps / Clients.

GOTIFY_URL=
GOTIFY_CLIENT_TOKEN=
GOTIFY_APP_TOKEN=

# MCP bearer token (generate with: openssl rand -hex 32)
GOTIFY_MCP_TOKEN=

RUST_LOG=info
EOF
    chmod 600 "${ENV_FILE}"
    echo "  ✓ Wrote starter ${ENV_FILE} — fill in your credentials before starting"
  else
    echo "  ✓ ${ENV_FILE} already exists — skipping"
  fi
fi

# ── PATH warning ──────────────────────────────────────────────────────────────
if ! echo ":${PATH}:" | grep -q ":${BIN_DIR}:"; then
  echo ""
  echo "  NOTE: ${BIN_DIR} is not in your PATH."
  echo "  Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
  echo "    export PATH=\"\${HOME}/.local/bin:\${PATH}\""
fi

# ── Run doctor ────────────────────────────────────────────────────────────────
echo ""
echo "Running post-install doctor check..."
if "${BIN_DIR}/${BINARY}" doctor 2>/dev/null; then
  echo ""
  echo "  ✓ Installation complete and verified."
else
  echo ""
  echo "  ⚠  Installation complete but doctor found issues."
  echo "     Edit ${DATA_DIR}/.env with your credentials, then re-run: ${BINARY} doctor"
fi

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
echo "Token types:"
echo "  C... = client token  (GOTIFY_CLIENT_TOKEN — for management)"
echo "  A... = app token     (GOTIFY_APP_TOKEN    — for sending messages)"
echo ""
echo "Next steps:"
echo "  1. Edit ${DATA_DIR}/.env — set GOTIFY_URL, GOTIFY_CLIENT_TOKEN, GOTIFY_APP_TOKEN"
echo "  2. Generate an MCP bearer token: openssl rand -hex 32"
echo "     Add it as GOTIFY_MCP_TOKEN= in ${DATA_DIR}/.env"
echo "  3. Run: ${BINARY} doctor       # validate config"
echo "  4. Run: ${BINARY} serve        # start HTTP server"
echo "  5. Or:  ${BINARY} mcp          # stdio for Claude Code"
echo ""
echo "For Claude Code plugin setup, see plugins/gotify/.claude-plugin/plugin.json"

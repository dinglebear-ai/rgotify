#!/usr/bin/env bash
# SessionStart / ConfigChange hook for the Gotify plugin.
set -euo pipefail

binary="${RUSTIFY_MCP_BIN:-rgotify}"

if ! command -v "${binary}" >/dev/null 2>&1; then
  printf 'gotify plugin setup: rgotify is not installed or not on PATH.\n' >&2
  printf 'Install rgotify separately, then run: rgotify setup\n' >&2
  exit 0
fi

exec "${binary}" setup plugin-hook "$@"

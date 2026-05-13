#!/usr/bin/env bash
# =============================================================================
# test-tools.sh — Integration smoke-test for gotify-mcp MCP server tools
#
# Exercises non-destructive actions for the gotify MCP tool and validates
# that the Gotify REST API integration is working correctly (semantic checks,
# not just that the MCP server responds).
#
# Action inventory (non-destructive):
#   gotify health, gotify version, gotify me,
#   gotify messages, gotify applications, gotify clients, gotify help
#   Resource: gotify://schema/mcp-tool
#
# Usage:
#   ./tests/mcporter/test-tools.sh [--timeout-ms N] [--parallel] [--verbose]
#
# Options:
#   --timeout-ms N   Per-call timeout in milliseconds (default: 25000)
#   --parallel       Run independent test groups in parallel (default: off)
#   --verbose        Print raw mcporter output for each call
#
# Credentials sourced from .env in project root (falls back to environment):
#   GOTIFY_MCP_HOST   (default: localhost)
#   GOTIFY_MCP_PORT   (default: 9158)
#   GOTIFY_MCP_TOKEN  (optional bearer token)
#
# Exit codes:
#   0 — all tests passed or skipped
#   1 — one or more tests failed
#   2 — prerequisite check failed (mcporter not found, server unreachable)
# =============================================================================

set -uo pipefail

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_DIR="$(cd -- "${SCRIPT_DIR}/../.." && pwd -P)"
readonly SCRIPT_NAME="$(basename -- "${BASH_SOURCE[0]}")"
readonly TS_START="$(date +%s%N)"
readonly LOG_FILE="${TMPDIR:-/tmp}/${SCRIPT_NAME%.sh}.$(date +%Y%m%d-%H%M%S).log"
readonly ENV_FILE="${PROJECT_DIR}/.env"

# Colours (disabled automatically when stdout is not a terminal)
if [[ -t 1 ]]; then
  C_RESET='\033[0m'
  C_BOLD='\033[1m'
  C_GREEN='\033[0;32m'
  C_RED='\033[0;31m'
  C_YELLOW='\033[0;33m'
  C_CYAN='\033[0;36m'
  C_DIM='\033[2m'
else
  C_RESET='' C_BOLD='' C_GREEN='' C_RED='' C_YELLOW='' C_CYAN='' C_DIM=''
fi

# ---------------------------------------------------------------------------
# Defaults (overridable via flags)
# ---------------------------------------------------------------------------
CALL_TIMEOUT_MS=25000
USE_PARALLEL=false
VERBOSE=false

# ---------------------------------------------------------------------------
# Counters
# ---------------------------------------------------------------------------
PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0
declare -a FAIL_NAMES=()

# Runtime globals — populated after ENV load
MCP_URL=''
MCPORTER_HEADER_ARGS=()

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --timeout-ms)
        CALL_TIMEOUT_MS="${2:?--timeout-ms requires a value}"
        shift 2
        ;;
      --parallel)
        USE_PARALLEL=true
        shift
        ;;
      --verbose)
        VERBOSE=true
        shift
        ;;
      -h|--help)
        printf 'Usage: %s [--timeout-ms N] [--parallel] [--verbose]\n' "${SCRIPT_NAME}"
        exit 0
        ;;
      *)
        printf '[ERROR] Unknown argument: %s\n' "$1" >&2
        exit 2
        ;;
    esac
  done
}

# ---------------------------------------------------------------------------
# Logging helpers
# ---------------------------------------------------------------------------
log_info()  { printf "${C_CYAN}[INFO]${C_RESET}  %s\n" "$*" | tee -a "${LOG_FILE}"; }
log_warn()  { printf "${C_YELLOW}[WARN]${C_RESET}  %s\n" "$*" | tee -a "${LOG_FILE}"; }
log_error() { printf "${C_RED}[ERROR]${C_RESET} %s\n" "$*" | tee -a "${LOG_FILE}" >&2; }

elapsed_ms() {
  local now
  now="$(date +%s%N)"
  printf '%d' "$(( (now - TS_START) / 1000000 ))"
}

# ---------------------------------------------------------------------------
# Cleanup trap
# ---------------------------------------------------------------------------
cleanup() {
  local rc=$?
  if [[ $rc -ne 0 ]]; then
    log_warn "Script exited with rc=${rc}. Log: ${LOG_FILE}"
  fi
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# Load environment and build MCP URL + auth headers
# ---------------------------------------------------------------------------
load_env() {
  if [[ -f "${ENV_FILE}" ]]; then
    # shellcheck disable=SC1090
    set -a
    source "${ENV_FILE}"
    set +a
    log_info "Loaded credentials from ${ENV_FILE}"
  else
    log_warn "${ENV_FILE} not found — using defaults / environment"
  fi

  local host="${GOTIFY_MCP_HOST:-localhost}"
  # Remap bind address 0.0.0.0 → localhost for outbound connections.
  if [[ "${host}" == "0.0.0.0" ]]; then
    host="localhost"
  fi
  local port="${GOTIFY_MCP_PORT:-9158}"
  MCP_URL="http://${host}:${port}/mcp"

  local token="${GOTIFY_MCP_TOKEN:-}"

  MCPORTER_HEADER_ARGS=()
  if [[ -n "${token}" ]]; then
    MCPORTER_HEADER_ARGS+=(--header "Authorization: Bearer ${token}")
  fi

  log_info "MCP URL: ${MCP_URL}"
  if [[ ${#MCPORTER_HEADER_ARGS[@]} -gt 0 ]]; then
    log_info "Auth: Bearer token configured"
  else
    log_info "Auth: none (GOTIFY_MCP_TOKEN unset)"
  fi
}

# ---------------------------------------------------------------------------
# Prerequisite checks
# ---------------------------------------------------------------------------
check_prerequisites() {
  local missing=false

  if ! command -v mcporter &>/dev/null; then
    log_error "mcporter not found in PATH. Install it and re-run."
    missing=true
  fi

  if ! command -v python3 &>/dev/null; then
    log_error "python3 not found in PATH."
    missing=true
  fi

  if ! command -v curl &>/dev/null; then
    log_error "curl not found in PATH."
    missing=true
  fi

  if [[ "${missing}" == true ]]; then
    return 2
  fi
}

# ---------------------------------------------------------------------------
# Server connectivity smoke-test
# ---------------------------------------------------------------------------
smoke_test_server() {
  log_info "Smoke-testing server connectivity..."

  local base_url="${MCP_URL%/mcp}"

  # 1. Health endpoint (no auth required)
  local health_raw health_status
  health_raw="$(curl -sf --max-time 10 "${base_url}/health" 2>/dev/null)" || health_raw=''
  health_status="$(printf '%s' "${health_raw}" | \
    python3 -c "import sys,json; print(json.load(sys.stdin).get('status',''))" 2>/dev/null)" || health_status=''

  if [[ "${health_status}" != "ok" ]]; then
    log_error "Health endpoint at ${base_url}/health did not return status=ok (got: '${health_status}')"
    log_error "Is gotify-mcp running?  gotify serve mcp  or  just docker-up"
    return 2
  fi
  log_info "Health endpoint OK"

  # 2. tools/list to confirm MCP layer responds
  local tool_count
  tool_count="$(
    curl -sf --max-time 10 \
      -X POST "${MCP_URL}" \
      -H "Content-Type: application/json" \
      -H "Accept: application/json, text/event-stream" \
      ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
      -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' 2>/dev/null | \
    python3 -c "
import sys, json
d = json.load(sys.stdin)
tools = d.get('result', {}).get('tools', [])
print(len(tools))
" 2>/dev/null
  )" || tool_count=0

  if [[ "${tool_count}" -lt 1 ]] 2>/dev/null; then
    log_error "tools/list returned ${tool_count} tools — expected at least 1"
    return 2
  fi

  log_info "Server OK — ${tool_count} tools available"
  return 0
}

# ---------------------------------------------------------------------------
# mcporter call wrapper
# ---------------------------------------------------------------------------
mcporter_call() {
  local tool="${1:?tool required}"
  shift
  local args_json="${1:?args_json required}"

  mcporter call \
    --http-url "${MCP_URL}" \
    --allow-http \
    ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
    --tool "${tool}" \
    --args "${args_json}" \
    --timeout "${CALL_TIMEOUT_MS}" \
    --output json \
    2>>"${LOG_FILE}"
}

# ---------------------------------------------------------------------------
# Semantic validation wrapper
#   python_check is Python code with variable d (parsed JSON).
#   Must print "ok" on success, anything else on failure.
# ---------------------------------------------------------------------------
run_test_semantic() {
  local label="${1:?label required}"
  local args_json="${2:?args_json required}"
  local py_check="${3:?python check required}"

  local t0
  t0="$(date +%s%N)"

  local output
  output="$(mcporter_call gotify "${args_json}")" || true

  local elapsed_ms
  elapsed_ms="$(( ( $(date +%s%N) - t0 ) / 1000000 ))"

  if [[ "${VERBOSE}" == true ]]; then
    printf '%s\n' "${output}" | tee -a "${LOG_FILE}"
  else
    printf '%s\n' "${output}" >> "${LOG_FILE}"
  fi

  local result
  result="$(
    printf '%s' "${output}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    if isinstance(d, dict) and ('error' in d or d.get('kind') == 'error'):
        print('error: ' + str(d.get('error', d.get('message', 'unknown error'))))
        sys.exit(0)
    ${py_check}
except Exception as e:
    print('check_error: ' + str(e))
" 2>/dev/null
  )" || result="parse_error"

  if [[ "${result}" != "ok" ]]; then
    printf "${C_RED}[FAIL]${C_RESET} %-60s ${C_DIM}%dms${C_RESET}\n" \
      "${label}" "${elapsed_ms}" | tee -a "${LOG_FILE}"
    printf '       semantic check failed: %s\n' "${result}" | tee -a "${LOG_FILE}"
    FAIL_COUNT=$(( FAIL_COUNT + 1 ))
    FAIL_NAMES+=("${label}")
    return 1
  fi

  printf "${C_GREEN}[PASS]${C_RESET} %-60s ${C_DIM}%dms${C_RESET}\n" \
    "${label}" "${elapsed_ms}" | tee -a "${LOG_FILE}"
  PASS_COUNT=$(( PASS_COUNT + 1 ))
  return 0
}

# ---------------------------------------------------------------------------
# Standard test runner (key presence check)
# ---------------------------------------------------------------------------
run_test() {
  local label="${1:?label required}"
  local args_json="${2:?args_json required}"
  local expected_key="${3:-}"

  local t0
  t0="$(date +%s%N)"

  local output
  output="$(mcporter_call gotify "${args_json}")" || true

  local elapsed_ms
  elapsed_ms="$(( ( $(date +%s%N) - t0 ) / 1000000 ))"

  if [[ "${VERBOSE}" == true ]]; then
    printf '%s\n' "${output}" | tee -a "${LOG_FILE}"
  else
    printf '%s\n' "${output}" >> "${LOG_FILE}"
  fi

  local json_check
  json_check="$(
    printf '%s' "${output}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    if isinstance(d, dict) and ('error' in d or d.get('kind') == 'error'):
        print('error: ' + str(d.get('error', d.get('message', 'unknown error'))))
    else:
        print('ok')
except Exception as e:
    print('invalid_json: ' + str(e))
" 2>/dev/null
  )" || json_check="parse_error"

  if [[ "${json_check}" != "ok" ]]; then
    printf "${C_RED}[FAIL]${C_RESET} %-60s ${C_DIM}%dms${C_RESET}\n" \
      "${label}" "${elapsed_ms}" | tee -a "${LOG_FILE}"
    printf '       response validation failed: %s\n' "${json_check}" | tee -a "${LOG_FILE}"
    FAIL_COUNT=$(( FAIL_COUNT + 1 ))
    FAIL_NAMES+=("${label}")
    return 1
  fi

  if [[ -n "${expected_key}" ]]; then
    local key_check
    key_check="$(
      printf '%s' "${output}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    keys = '${expected_key}'.split('.')
    node = d
    for k in keys:
        if k:
            node = node[int(k)] if (isinstance(node, list) and k.isdigit()) else node[k]
    print('ok')
except Exception as e:
    print('missing: ' + str(e))
" 2>/dev/null
    )" || key_check="parse_error"

    if [[ "${key_check}" != "ok" ]]; then
      printf "${C_RED}[FAIL]${C_RESET} %-60s ${C_DIM}%dms${C_RESET}\n" \
        "${label}" "${elapsed_ms}" | tee -a "${LOG_FILE}"
      printf '       expected key .%s not found: %s\n' "${expected_key}" "${key_check}" | tee -a "${LOG_FILE}"
      FAIL_COUNT=$(( FAIL_COUNT + 1 ))
      FAIL_NAMES+=("${label}")
      return 1
    fi
  fi

  printf "${C_GREEN}[PASS]${C_RESET} %-60s ${C_DIM}%dms${C_RESET}\n" \
    "${label}" "${elapsed_ms}" | tee -a "${LOG_FILE}"
  PASS_COUNT=$(( PASS_COUNT + 1 ))
  return 0
}

# ---------------------------------------------------------------------------
# Skip helper
# ---------------------------------------------------------------------------
skip_test() {
  local label="${1:?label required}"
  local reason="${2:-prerequisite returned empty}"
  printf "${C_YELLOW}[SKIP]${C_RESET} %-60s %s\n" "${label}" "${reason}" | tee -a "${LOG_FILE}"
  SKIP_COUNT=$(( SKIP_COUNT + 1 ))
}

# ---------------------------------------------------------------------------
# Test suites
# ---------------------------------------------------------------------------

suite_health() {
  printf '\n%b== health ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  # health action calls the upstream Gotify /health endpoint.
  # Gotify returns: {"health":"green","database":"green"} or similar.
  # gotify-mcp passes this through so we validate the Gotify fields directly.
  run_test_semantic \
    'gotify health: upstream Gotify health is green' \
    '{"action":"health"}' \
    '
health_val = d.get("health") or d.get("status") or ""
database_val = d.get("database", "")
if health_val.lower() in ("green", "ok", "up") or database_val.lower() in ("green", "ok", "up"):
    print("ok")
else:
    print("health not green: " + str(d))
'

  run_test_semantic \
    'gotify health: database field is green (or absent on older Gotify)' \
    '{"action":"health"}' \
    '
db = d.get("database", "green")  # absent = assume ok on older Gotify
if db.lower() in ("green", "ok", "up"):
    print("ok")
else:
    print("database not green: database=" + str(db))
'
}

suite_version() {
  printf '\n%b== version ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  # version action calls the upstream Gotify /version endpoint.
  # Returns: {"version":"2.9.1","commit":"...","buildDate":"..."}
  run_test_semantic \
    'gotify version: version key is a non-empty string' \
    '{"action":"version"}' \
    '
version = d.get("version", "")
if isinstance(version, str) and len(version) > 0:
    print("ok")
else:
    print("version field missing or empty: " + str(d))
'

  run_test "gotify version: version key present" \
    '{"action":"version"}' "version"
}

suite_me() {
  printf '\n%b== me ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  # me action calls Gotify /current/user.
  # Returns: {"id":1,"name":"admin","admin":true,...}
  run_test_semantic \
    'gotify me: name field is a non-empty string' \
    '{"action":"me"}' \
    '
name = d.get("name", "")
if isinstance(name, str) and name:
    print("ok")
else:
    print("name field missing or empty: " + str(d))
'

  run_test_semantic \
    'gotify me: admin field is a boolean' \
    '{"action":"me"}' \
    '
if "admin" in d and isinstance(d["admin"], bool):
    print("ok")
else:
    print("admin field missing or not boolean: " + str(d))
'

  run_test "gotify me: name key present" '{"action":"me"}' "name"
  run_test "gotify me: admin key present" '{"action":"me"}' "admin"
}

suite_applications() {
  printf '\n%b== applications ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  # applications action calls Gotify /application.
  # Returns an array of: {"id":N,"name":"...","defaultPriority":N,"token":"..."}
  run_test_semantic \
    'gotify applications: response is an array' \
    '{"action":"applications"}' \
    '
if isinstance(d, list):
    print("ok")
elif isinstance(d, dict):
    apps = d.get("applications") or d.get("apps") or d.get("data") or d.get("items")
    if isinstance(apps, list):
        print("ok")
    else:
        print("not an array, keys=" + str(list(d.keys())))
else:
    print("unexpected type: " + str(type(d)))
'

  run_test_semantic \
    'gotify applications: each item has name and id fields' \
    '{"action":"applications"}' \
    '
items = d if isinstance(d, list) else (d.get("applications") or d.get("apps") or d.get("data") or [])
if len(items) == 0:
    # Empty is acceptable — Gotify server might have no apps configured
    print("ok")
elif all("name" in item and "id" in item for item in items):
    print("ok")
else:
    bad = [i for i in items if "name" not in i or "id" not in i]
    print("items missing name/id: " + str(bad[:2]))
'
}

suite_messages() {
  printf '\n%b== messages ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  # messages action calls Gotify /message.
  # Returns: {"messages":[{"id":N,"message":"...","appid":N,...}],"paging":{...}}
  run_test_semantic \
    'gotify messages: messages array present (may be empty)' \
    '{"action":"messages","limit":10}' \
    '
msgs = d.get("messages")
if isinstance(msgs, list):
    print("ok")
elif isinstance(d, list):
    print("ok")
else:
    print("messages field missing or not array: type=" + str(type(d.get("messages"))))
'

  run_test "gotify messages: messages key present" \
    '{"action":"messages","limit":5}' "messages"

  run_test "gotify messages: limit parameter accepted" \
    '{"action":"messages","limit":1}' "messages"

  # Validate message structure only if messages exist
  local msgs_raw
  msgs_raw="$(mcporter_call gotify '{"action":"messages","limit":5}'  2>/dev/null)" || msgs_raw=''
  local has_messages
  has_messages="$(printf '%s' "${msgs_raw}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    msgs = d.get('messages', []) if isinstance(d, dict) else (d if isinstance(d, list) else [])
    print('yes' if len(msgs) > 0 else 'no')
except:
    print('no')
" 2>/dev/null)" || has_messages='no'

  if [[ "${has_messages}" == "yes" ]]; then
    run_test "gotify messages: first message has id"      '{"action":"messages","limit":1}' "messages.0.id"
    run_test "gotify messages: first message has message" '{"action":"messages","limit":1}' "messages.0.message"
    run_test "gotify messages: first message has appid"   '{"action":"messages","limit":1}' "messages.0.appid"
  else
    skip_test "gotify messages: message structure (id, message, appid)" "no messages in Gotify (empty server)"
  fi
}

suite_clients() {
  printf '\n%b== clients ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  # clients action calls Gotify /client.
  # Returns an array of: {"id":N,"name":"...","token":"..."}
  run_test_semantic \
    'gotify clients: response is an array' \
    '{"action":"clients"}' \
    '
if isinstance(d, list):
    print("ok")
elif isinstance(d, dict):
    items = d.get("clients") or d.get("data") or d.get("items")
    if isinstance(items, list):
        print("ok")
    else:
        print("not a list: keys=" + str(list(d.keys())))
else:
    print("unexpected type: " + str(type(d)))
'

  run_test_semantic \
    'gotify clients: each client has id and name (if any exist)' \
    '{"action":"clients"}' \
    '
items = d if isinstance(d, list) else (d.get("clients") or d.get("data") or [])
if len(items) == 0:
    print("ok")
elif all("id" in c and "name" in c for c in items):
    print("ok")
else:
    bad = [c for c in items if "id" not in c or "name" not in c]
    print("clients missing id/name: " + str(bad[:2]))
'
}

suite_help() {
  printf '\n%b== help ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  # help action returns built-in documentation.
  # May be a top-level string, or {"help":"..."}
  run_test_semantic \
    'gotify help: returns non-empty help text' \
    '{"action":"help"}' \
    '
help_text = d if isinstance(d, str) else d.get("help", "")
if isinstance(help_text, str) and len(help_text) > 10:
    print("ok")
else:
    print("help field missing or too short: " + repr(d)[:200])
'

  run_test "gotify help: response is parseable JSON" '{"action":"help"}' ""
}

suite_schema_resource() {
  printf '\n%b== schema resource ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  # Test the gotify://schema/mcp-tool resource via resources/read JSON-RPC
  local resource_result
  resource_result="$(
    curl -sf --max-time 15 \
      -X POST "${MCP_URL}" \
      -H "Content-Type: application/json" \
      -H "Accept: application/json, text/event-stream" \
      ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
      -d '{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"gotify://schema/mcp-tool"}}' \
      2>/dev/null
  )" || resource_result=''

  local schema_ok
  schema_ok="$(printf '%s' "${resource_result}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    result = d.get('result', {})
    contents = result.get('contents', [])
    if len(contents) > 0:
        text = contents[0].get('text', '')
        if text and len(text) > 10:
            print('ok')
        else:
            print('resource text too short: ' + repr(text)[:100])
    elif 'error' in d:
        # Server returned JSON-RPC error — resource not registered, treat as skip
        print('no_resource')
    else:
        print('no_resource')
except Exception as e:
    print('parse_error: ' + str(e))
" 2>/dev/null)" || schema_ok="parse_error"

  if [[ "${schema_ok}" == "ok" ]]; then
    printf "${C_GREEN}[PASS]${C_RESET} %-60s\n" "gotify://schema/mcp-tool resource readable" | tee -a "${LOG_FILE}"
    PASS_COUNT=$(( PASS_COUNT + 1 ))
  elif [[ "${schema_ok}" == "no_resource" ]]; then
    skip_test "gotify://schema/mcp-tool resource" "resource not registered on this server"
  else
    printf "${C_RED}[FAIL]${C_RESET} %-60s\n" "gotify://schema/mcp-tool resource readable" | tee -a "${LOG_FILE}"
    printf '       result=%s\n' "${schema_ok}" | tee -a "${LOG_FILE}"
    FAIL_COUNT=$(( FAIL_COUNT + 1 ))
    FAIL_NAMES+=("gotify://schema/mcp-tool resource readable")
  fi
}

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
print_summary() {
  local total=$(( PASS_COUNT + FAIL_COUNT + SKIP_COUNT ))
  local elapsed_total
  elapsed_total="$(elapsed_ms)"

  printf '\n%b=== Summary ===%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"
  printf 'Total: %d   ' "${total}" | tee -a "${LOG_FILE}"
  printf "${C_GREEN}Pass: %d${C_RESET}   " "${PASS_COUNT}" | tee -a "${LOG_FILE}"
  printf "${C_RED}Fail: %d${C_RESET}   " "${FAIL_COUNT}" | tee -a "${LOG_FILE}"
  printf "${C_YELLOW}Skip: %d${C_RESET}   " "${SKIP_COUNT}" | tee -a "${LOG_FILE}"
  printf "${C_DIM}%dms${C_RESET}\n" "${elapsed_total}" | tee -a "${LOG_FILE}"

  if [[ ${#FAIL_NAMES[@]} -gt 0 ]]; then
    printf '\nFailed tests:\n' | tee -a "${LOG_FILE}"
    for name in "${FAIL_NAMES[@]}"; do
      printf '  %b- %s%b\n' "${C_RED}" "${name}" "${C_RESET}" | tee -a "${LOG_FILE}"
    done
  fi

  printf '\nLog: %s\n' "${LOG_FILE}" | tee -a "${LOG_FILE}"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  parse_args "$@"

  printf '%b%s%b\n' "${C_BOLD}" "gotify-mcp integration tests" "${C_RESET}" | tee "${LOG_FILE}"
  printf 'Started: %s\n\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" | tee -a "${LOG_FILE}"

  load_env
  check_prerequisites || exit $?
  smoke_test_server   || exit $?

  if [[ "${USE_PARALLEL}" == true ]]; then
    suite_health &
    suite_version &
    wait
    suite_me &
    suite_applications &
    suite_messages &
    suite_clients &
    suite_help &
    wait
    suite_schema_resource
  else
    suite_health
    suite_version
    suite_me
    suite_applications
    suite_messages
    suite_clients
    suite_help
    suite_schema_resource
  fi

  print_summary

  if [[ "${FAIL_COUNT}" -gt 0 ]]; then
    exit 1
  fi
  exit 0
}

main "$@"

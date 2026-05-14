#!/usr/bin/env bash
# refresh-docs.sh — Refresh local reference documentation for gotify-mcp
# Pattern: §38 in docs/PATTERNS.md (rmcp-template)
# Usage: scripts/refresh-docs.sh [--dry-run] [--skip-crawl] [--skip-repomix]
set -Eeuo pipefail; IFS=$'\n\t'
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
REF_DIR="$ROOT_DIR/docs/references"; CHANGES_FILE="$REF_DIR/CHANGES.md"
AXON_OUTPUT_DIR="${AXON_OUTPUT_DIR:-$HOME/.axon/output}"
DRY_RUN=false; SKIP_CRAWL=false; SKIP_REPOMIX=false
usage() { cat <<'EOF'
Usage: scripts/refresh-docs.sh [OPTIONS]
Refresh docs/references/ with Gotify API docs and MCP SDK packs.
  Crawled: https://gotify.net/docs, https://modelcontextprotocol.io
  Repomix: gotify/server, gotify/android, modelcontextprotocol/rust-sdk
Options: --dry-run, --skip-crawl, --skip-repomix, -h
EOF
}
while [[ $# -gt 0 ]]; do case "$1" in
  --dry-run) DRY_RUN=true;shift ;; --skip-crawl) SKIP_CRAWL=true;shift ;;
  --skip-repomix) SKIP_REPOMIX=true;shift ;; -h|--help) usage;exit 0 ;;
  *) echo "ERROR: unknown: $1" >&2;exit 2 ;; esac; done
[[ "$SKIP_CRAWL" == true && "$SKIP_REPOMIX" == true ]] && { echo "ERROR: cannot combine --skip-crawl and --skip-repomix" >&2;exit 2; }
log() { printf '[refresh-docs] %s\n' "$*"; }
refresh_scope() { if [[ "$SKIP_CRAWL" == true ]]; then printf repomix-only; elif [[ "$SKIP_REPOMIX" == true ]]; then printf crawl-only; else printf full; fi; }
require_cmd() { command -v "$1" >/dev/null 2>&1 || { echo "ERROR: $1 not found" >&2;exit 1; }; }
make_tmpdir() { mktemp -d "${TMPDIR:-/tmp}/gotify-refresh-docs.XXXXXX"; }
atomic_replace_dir() {
  local src="$1" dst="$2" parent backup
  parent="$(dirname -- "$dst")"; mkdir -p "$parent"
  backup="$(mktemp -d "$parent/.$(basename "$dst").backup.XXXXXX")"; rmdir "$backup"
  [[ -e "$dst" ]] && mv -- "$dst" "$backup"
  if mv -- "$src" "$dst"; then rm -rf -- "$backup"; else [[ -e "$backup" ]] && mv -- "$backup" "$dst"; return 1; fi
}
copy_job_output_to_layout() {
  local sd="$1" td="$2" tmp
  [[ -f "$sd/manifest.jsonl" ]] || { echo "ERROR: missing manifest: $sd/manifest.jsonl" >&2;return 1; }
  [[ -d "$sd/markdown" ]]       || { echo "ERROR: missing markdown dir" >&2;return 1; }
  tmp="$(make_tmpdir)"; cp -a "$sd/." "$tmp/"; atomic_replace_dir "$tmp" "$td"
}
newest_domain_run() {
  local dd="$AXON_OUTPUT_DIR/domains/$1"
  [[ -d "$dd" ]] || return 1
  find "$dd" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' | sort -nr | awk 'NR==1{$1="";sub(/^ /,"");print}'
}
crawl_docs() {
  local url="$1" domain="$2" tr="$3" td="$REF_DIR/$3" out job sd
  log "crawl $url -> docs/references/$tr"
  [[ "$DRY_RUN" == true ]] && return 0; require_cmd axon
  out="$(axon crawl "$url" --wait true --yes 2>&1)"; printf '%s\n' "$out"
  job="$(awk '/^Job ID:/{print $3}' <<<"$out" | tail -1)"
  if [[ -n "$job" && -d "$AXON_OUTPUT_DIR/domains/$domain/$job" ]]; then sd="$AXON_OUTPUT_DIR/domains/$domain/$job"; else sd="$(newest_domain_run "$domain")"; fi
  [[ -n "$sd" && -d "$sd" ]] || { echo "ERROR: no Axon output for $domain" >&2;return 1; }
  copy_job_output_to_layout "$sd" "$td"
}
repomix_command() {
  if [[ -n "${REPOMIX_BIN:-}" ]]; then "$REPOMIX_BIN" "$@"; elif command -v repomix >/dev/null 2>&1; then repomix "$@"; else require_cmd npx; npx --yes repomix "$@"; fi
}
pack_repo() {
  local remote="$1" tr="$2" inc="${3:-}" ign="${4:-}" tf="$REF_DIR/$2" tmp_dir tmp_file
  log "pack $remote -> docs/references/$tr"
  [[ -n "$inc" ]] && log "  include: $inc"; [[ -n "$ign" ]] && log "  ignore: $ign"
  [[ "$DRY_RUN" == true ]] && return 0
  tmp_dir="$(make_tmpdir)"; tmp_file="$tmp_dir/out.xml"
  local args=(--remote "$remote" --style xml --output "$tmp_file" --top-files-len 10)
  [[ -n "$inc" ]] && args+=(--include "$inc"); [[ -n "$ign" ]] && args+=(--ignore "$ign")
  repomix_command "${args[@]}"
  [[ -s "$tmp_file" ]] || { echo "ERROR: no output for $remote" >&2;rm -rf -- "$tmp_dir";return 1; }
  mkdir -p "$(dirname -- "$tf")"; mv -- "$tmp_file" "$tf"; rm -rf -- "$tmp_dir"
}
write_index() {
  local g=0 m=0
  [[ -d "$REF_DIR/gotify/docs" ]] && g="$(find "$REF_DIR/gotify/docs" -type f|wc -l|tr -d ' ')"
  [[ -d "$REF_DIR/mcp/docs"   ]] && m="$(find "$REF_DIR/mcp/docs"   -type f|wc -l|tr -d ' ')"
  cat > "$REF_DIR/INDEX.md" <<EOF
# Reference Index — gotify-mcp
| Path | Contents | Source |
|---|---|---|
| \`gotify/docs/\` | Axon-crawled Gotify docs | https://gotify.net/docs |
| \`gotify/repos/\` | Repomix packs (server + android) | gotify/* |
| \`mcp/docs/\` | MCP protocol docs | https://modelcontextprotocol.io |
| \`mcp/repos/\` | Repomix: rust-sdk, registry | modelcontextprotocol/* |
## File Counts
| gotify/docs/ | $g | mcp/docs/ | $m |
_Updated: $(date -u +%Y-%m-%dT%H:%M:%SZ)_
EOF
}
snapshot_references() {
  [[ ! -d "$REF_DIR" ]] && { :>"$1";return 0; }
  (cd "$REF_DIR";find . -type f ! -path './CHANGES.md' -print0|sort -z|xargs -0 -r sha256sum|sed 's#  \./#  #') > "$1"
}
snapshot_paths() { awk '{$1="";sub(/^  /,"");print}' "$1"; }
ensure_changes_file() {
  mkdir -p "$REF_DIR"; [[ -f "$CHANGES_FILE" ]] && return 0
  cat > "$CHANGES_FILE" <<EOF
---
title: Reference Refresh Change Log — gotify-mcp
generated_by: scripts/refresh-docs.sh
created_at: $(date -u +%Y-%m-%dT%H:%M:%SZ)
---
EOF
}
append_changes_log() {
  ensure_changes_file
  { printf '\n## %s\n\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf -- '- scope: `%s`\n' "$(refresh_scope)"
    printf -- '- summary: `%s added, %s modified, %s removed`\n' "$4" "$5" "$6"
  } >> "$CHANGES_FILE"
}
summarize_reference_changes() {
  local b="$1" a="$2" td; td="$(make_tmpdir)"
  local bp="$td/b" ap="$td/a" added="$td/add" removed="$td/rm" common="$td/com" modified="$td/mod"
  snapshot_paths "$b"|sort>"$bp"; snapshot_paths "$a"|sort>"$ap"
  comm -13 "$bp" "$ap">"$added"; comm -23 "$bp" "$ap">"$removed"; comm -12 "$bp" "$ap">"$common"; :>"$modified"
  while IFS= read -r p; do
    [[ "$(grep -F "  $p" "$b"|cut -d' ' -f1)" != "$(grep -F "  $p" "$a"|cut -d' ' -f1)" ]] && printf '%s\n' "$p">>"$modified"
  done <"$common"
  local ac rc mc; ac="$(wc -l<"$added"|tr -d ' ')"; rc="$(wc -l<"$removed"|tr -d ' ')"; mc="$(wc -l<"$modified"|tr -d ' ')"
  log "change summary: $ac added, $mc modified, $rc removed"
  append_changes_log "$added" "$modified" "$removed" "$ac" "$mc" "$rc"; rm -rf -- "$td"
}
main() {
  local sd bs as
  if [[ "$DRY_RUN" != true ]]; then sd="$(make_tmpdir)"; bs="$sd/before.sha256"; as="$sd/after.sha256"; snapshot_references "$bs"; fi
  mkdir -p "$REF_DIR/gotify/docs" "$REF_DIR/gotify/repos" "$REF_DIR/mcp/docs" "$REF_DIR/mcp/repos"
  if [[ "$SKIP_CRAWL" != true ]]; then
    crawl_docs "https://gotify.net/docs" || log "WARN: gotify docs crawl failed, continuing"            "gotify.net"              "gotify/docs"
    crawl_docs "https://modelcontextprotocol.io"    "modelcontextprotocol.io" "mcp/docs" || log "WARN: mcp docs crawl failed, continuing"
  fi
  if [[ "$SKIP_REPOMIX" != true ]]; then
    pack_repo "gotify/server"                      "gotify/repos/gotify-server.xml"             "api/**,router/**,model/**" "node_modules/**"
    pack_repo "gotify/android"                     "gotify/repos/gotify-android.xml"            "app/src/**" ""
    pack_repo "modelcontextprotocol/rust-sdk"      "mcp/repos/modelcontextprotocol-rust-sdk.xml"
    pack_repo "modelcontextprotocol/registry"      "mcp/repos/modelcontextprotocol-registry.xml"
  fi
  if [[ "$DRY_RUN" != true ]]; then
    write_index; snapshot_references "$as"; summarize_reference_changes "$bs" "$as"; rm -rf -- "$sd"
  fi
  log "done"
}
main "$@"

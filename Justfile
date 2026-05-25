dev:
    cargo run -- serve mcp

build:
    cargo build

release:
    cargo build --release

check:
    cargo check

lint:
    cargo clippy -- -D warnings

fmt:
    cargo fmt

test:
    cargo test

docker-build:
    docker build -f config/Dockerfile -t gotify-mcp .

docker-up:
    docker compose up -d

docker-down:
    docker compose down

up:
    docker compose up -d

down:
    docker compose down

restart:
    docker compose restart

logs:
    docker compose logs -f

health:
    curl -sf http://localhost:40020/health | jq .

setup:
    cp -n .env.example .env || true

install: release
    install -m 755 target/release/gotify ~/.local/bin/gotify
    @echo "Installed: ~/.local/bin/gotify"

gen-token:
    openssl rand -hex 32

repair:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Checking gotify-mcp health..."
    if curl -sf http://localhost:40020/health >/dev/null 2>&1; then
      echo "Server is healthy."
    else
      echo "Server unreachable — restarting..."
      if docker ps --filter 'name=gotify-mcp' --quiet 2>/dev/null | grep -q .; then
        docker compose restart gotify-mcp
      elif systemctl --user is-active --quiet gotify-mcp.service 2>/dev/null; then
        systemctl --user restart gotify-mcp
      else
        echo "No running service found. Start with: just dev  or  just docker-up"
        exit 1
      fi
    fi

test-mcporter:
    bash tests/mcporter/test-tools.sh


validate-skills:
    bash scripts/validate-plugin-layout.sh

validate-plugin: validate-skills

runtime-current:
    bash scripts/check-runtime-current.sh --unit gotify-mcp.service --service gotify-mcp --expected-binary target/release/gotify

# Generate a standalone CLI for this server (requires running server; HTTP-only transport)
generate-cli:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "⚠  Server must be running on port 40020 (run 'just dev' first)"
    echo "⚠  Generated CLI embeds your token — do not commit or share"
    mkdir -p dist dist/.cache
    current_hash=$(timeout 10 curl -sf \
      -H "Authorization: Bearer ${GOTIFY_MCP_TOKEN:-}" \
      -H "Accept: application/json, text/event-stream" \
      http://localhost:40020/mcp/tools/list 2>/dev/null | sha256sum | cut -d' ' -f1 || echo "nohash")
    cache_file="dist/.cache/gotify-cli.schema_hash"
    if [[ -f "$cache_file" ]] && [[ "$(cat "$cache_file")" == "$current_hash" ]] && [[ -f "dist/gotify-cli" ]]; then
      echo "SKIP: gotify tool schema unchanged — use existing dist/gotify-cli"
      exit 0
    fi
    timeout 30 mcporter generate-cli \
      --command http://localhost:40020/mcp \
      --header "Authorization: Bearer ${GOTIFY_MCP_TOKEN:-}" \
      --name gotify-cli \
      --output dist/gotify-cli
    printf '%s' "$current_hash" > "$cache_file"
    echo "✓ Generated dist/gotify-cli (requires bun at runtime)"

clean:
    cargo clean
    rm -rf .cache/

# Linux only — Windows would need .exe binaries; requires git lfs install
build-plugin: release
    #!/bin/sh
    set -eu
    target_dir="${CARGO_TARGET_DIR:-target}"
    if [ ! -x "$target_dir/release/gotify" ] && [ -x ".cache/cargo/release/gotify" ]; then
      target_dir=".cache/cargo"
    fi
    mkdir -p bin plugins/gotify/bin
    install -m 755 "$target_dir/release/gotify" bin/gotify
    install -m 755 "$target_dir/release/gotify" plugins/gotify/bin/gotify

# Publish: bump version, tag, push (triggers crates.io + Docker publish)
publish bump="patch":
    #!/usr/bin/env bash
    set -euo pipefail
    [ "$(git branch --show-current)" = "main" ] || { echo "Switch to main first"; exit 1; }
    [ -z "$(git status --porcelain)" ] || { echo "Commit or stash changes first"; exit 1; }
    git pull origin main
    CURRENT=$(grep -m1 "^version" Cargo.toml | sed "s/.*\"\(.*\)\".*/\1/")
    IFS="." read -r major minor patch <<< "$CURRENT"
    case "{{bump}}" in
      major) major=$((major+1)); minor=0; patch=0 ;;
      minor) minor=$((minor+1)); patch=0 ;;
      patch) patch=$((patch+1)) ;;
      *) echo "Usage: just publish [major|minor|patch]"; exit 1 ;;
    esac
    NEW="${major}.${minor}.${patch}"
    echo "Version: ${CURRENT} → ${NEW}"
    sed -i "s/^version = \"${CURRENT}\"/version = \"${NEW}\"/" Cargo.toml
    cargo check 2>/dev/null || true
    git add -A && git commit -m "release: v${NEW}" && git tag "v${NEW}" && git push origin main --tags
    echo "Tagged v${NEW} — publish workflow will run automatically"

# Refresh local reference documentation (crawls + repomix)
refresh-docs:
    bash scripts/refresh-docs.sh

# Refresh docs — repomix packs only (no crawl)
refresh-docs-repomix:
    bash scripts/refresh-docs.sh --skip-crawl

# Refresh docs — crawl only (no repomix)
refresh-docs-crawl:
    bash scripts/refresh-docs.sh --skip-repomix

# Dry-run: print what would be refreshed
refresh-docs-dry:
    bash scripts/refresh-docs.sh --dry-run

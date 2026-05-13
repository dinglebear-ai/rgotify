# AGENTS.md — gotify-mcp

Agent instructions for this repository.

## What this repo is

`gotify-mcp` is a small Rust crate: a single binary (`gotify`) that bridges MCP clients (Claude, Codex, etc.) to a self-hosted Gotify push notification server via REST. No database, no background tasks, no daemon. The binary is an HTTP MCP server, a stdio MCP server, or a CLI — depending on its first argument.

## Key facts for agents

- **Binary name**: `gotify` (from `[[bin]] name = "gotify"` in `Cargo.toml`)
- **Crate name**: `gotify-mcp`
- **Version**: `0.1.0`
- **Port**: `9158` (MCP HTTP)
- **Auth header to Gotify**: `X-Gotify-Key`
- **Do not touch**: anything in `src/` — docs-only task

## Architecture in one paragraph

`gotify.rs` is the HTTP REST client with `X-Gotify-Key` header. `app.rs` wraps it in `GotifyService` which adds the destructive operation gate. `mcp/tools.rs` and `cli.rs` are identical thin shims: parse input, call service, return result. `main.rs` reads the first argument and routes to HTTP MCP server, stdio MCP server, or CLI.

## How to add a new action (4 steps)

1. `src/gotify.rs` — add a method to `GotifyClient` calling the REST endpoint with the correct token.
2. `src/app.rs` — add a method to `GotifyService` calling the client (add `destructive_gate` if needed).
3. `src/mcp/tools.rs` — add a match arm in `dispatch()`.
4. `src/cli.rs` — add a match arm or subcommand.

Update `docs/INVENTORY.md` to document the new action.

## Token model gotcha

Two tokens, two purposes — never mix:
- `GOTIFY_CLIENT_TOKEN` (starts `C`) — all GET/PUT/DELETE operations
- `GOTIFY_APP_TOKEN` (starts `A`) — POST /message only (sending)

If `send` is called with the client token, Gotify returns 401. If management calls use the app token, they also return 401.

## Build and test

```bash
cargo check          # fast typecheck
cargo clippy         # lint — no warnings
cargo fmt            # format
cargo test           # test suite
cargo build --release
```

## Environment variables (required)

```
GOTIFY_URL             Gotify server URL
GOTIFY_CLIENT_TOKEN    Client token (management)
GOTIFY_APP_TOKEN       App token (send)
```

Optional:
```
GOTIFY_ALLOW_DESTRUCTIVE   Skip confirm gate
GOTIFY_MCP_TOKEN           Bearer token for MCP HTTP auth
GOTIFY_MCP_PORT            Bind port (default 9158)
RUST_LOG                   Log level
```

## File layout

```
gotify-mcp/
├── src/
│   ├── main.rs            Mode dispatch
│   ├── config.rs          Config loading (config.toml + env)
│   ├── gotify.rs          HTTP REST client
│   ├── app.rs             GotifyService + destructive gate
│   ├── cli.rs             CLI shim
│   ├── lib.rs             Library root
│   └── mcp/
│       ├── tools.rs       MCP shim (action dispatch)
│       ├── routes.rs      HTTP route handlers
│       ├── rmcp_server.rs RMCP transport
│       ├── schemas.rs     JSON Schema definitions
│       └── prompts.rs     MCP prompts
├── docs/
│   ├── INVENTORY.md       Full action/command/env inventory
│   ├── QUICKSTART.md      5-minute setup guide
│   └── stack/
│       ├── ARCH.md        Architecture diagrams
│       └── TECH.md        Technology choices
├── README.md              User-facing overview and reference
├── CLAUDE.md              Claude Code dev instructions
├── AGENTS.md              This file
├── CHANGELOG.md           Release history
└── Cargo.toml             Crate metadata
```

## Destructive operation safety

`GotifyService::destructive_gate(confirm)` blocks unless `confirm == true` or `allow_destructive == true`. The error message is: `"destructive operation — pass confirm=true or set GOTIFY_ALLOW_DESTRUCTIVE=true"`. Never bypass this gate without user intent.

## Docs to keep current

When adding actions or changing env vars, update:
- `docs/INVENTORY.md` — authoritative inventory
- `README.md` — user-facing reference
- `CLAUDE.md` — dev reference
- `CHANGELOG.md` — add an entry under `[Unreleased]`

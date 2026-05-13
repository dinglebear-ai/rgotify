# CLAUDE.md — gotify-mcp

Rust MCP server bridging Claude to a Gotify push notification server. One tool (`gotify`) with action dispatch, a thin CLI, and a REST HTTP client. No database, no background tasks.

## Commands

```bash
cargo build                      # debug build
cargo build --release            # release build
cargo check                      # typecheck
cargo clippy                     # lint — must pass before committing
cargo fmt                        # format — enforced
cargo test                       # test suite
cargo run                        # start HTTP MCP server (port 9158)
cargo run -- mcp                 # start stdio MCP transport
cargo run -- health              # run CLI health command
```

## Module Map

| File | Role |
|------|------|
| `src/main.rs` | Mode dispatch: `serve_mcp` / `serve_stdio_mcp` / `run_cli` |
| `src/config.rs` | Config struct — loads `config.toml` then env vars |
| `src/gotify.rs` | `GotifyClient` — HTTP REST client, `X-Gotify-Key` header |
| `src/app.rs` | `GotifyService` — business logic, destructive gate |
| `src/mcp/tools.rs` | MCP shim — parse JSON args → call service → return `Value` |
| `src/cli.rs` | CLI shim — parse CLI args → call service → format/print |
| `src/mcp.rs` | Axum router, auth middleware, RMCP adapter |

## Architecture Pattern

```
CLI args / MCP JSON args
        ↓
  thin shim (cli.rs or mcp/tools.rs)   ← parse only, no logic
        ↓
  GotifyService (app.rs)               ← all business logic, destructive gate
        ↓
  GotifyClient (gotify.rs)             ← HTTP calls to Gotify REST API
        ↓
  Gotify server (X-Gotify-Key header)
```

CLI and MCP are intentionally identical thin shims. All logic lives in `GotifyService`.

## Adding a New Action (4 steps)

1. **`gotify.rs`** — add a method to `GotifyClient` calling the appropriate REST endpoint with the right token.
2. **`app.rs`** — add a method to `GotifyService` that calls the client method (add `destructive_gate` if needed).
3. **`mcp/tools.rs`** — add a match arm in `dispatch()` that calls the service method.
4. **`cli.rs`** — add a match arm or subcommand that calls the service method and formats output.

## Token Model (Critical)

Two tokens, two purposes — never mix them:

| Token | Env var | Used for |
|-------|---------|----------|
| Client token (starts `C`) | `GOTIFY_CLIENT_TOKEN` | GET/PUT/DELETE — management: list, create, delete apps/clients/messages, read user |
| App token (starts `A`) | `GOTIFY_APP_TOKEN` | POST /message only — sending notifications |

`send_message` in `gotify.rs` uses `self.app_token` explicitly. All other requests use `self.client_token`. If the wrong token is used, Gotify returns 401.

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `GOTIFY_URL` | yes | — | Gotify server base URL |
| `GOTIFY_CLIENT_TOKEN` | yes | — | Client token for management |
| `GOTIFY_APP_TOKEN` | yes | — | App token for send |
| `GOTIFY_ALLOW_DESTRUCTIVE` | no | `false` | Skip confirm gate |
| `GOTIFY_MCP_HOST` | no | `0.0.0.0` | MCP bind host |
| `GOTIFY_MCP_PORT` | no | `9158` | MCP bind port |
| `GOTIFY_MCP_TOKEN` | no | — | Bearer token for MCP auth |
| `GOTIFY_MCP_NO_AUTH` | no | `false` | Disable MCP auth |
| `RUST_LOG` | no | `info` | Log filter |

## Destructive Gate

`GotifyService::destructive_gate(confirm)` returns an error unless `confirm == true` or `self.allow_destructive == true`. This is checked by every destructive method before calling the client. The error message is: `"destructive operation — pass confirm=true or set GOTIFY_ALLOW_DESTRUCTIVE=true"`.

## Ports

| Port | Purpose |
|------|---------|
| 9158 | RMCP Streamable HTTP (`POST /mcp`, `GET /health`) |

## MCP Tool: `gotify` Actions

**Read:** `health`, `version`, `me`, `messages`, `applications`, `clients`
**Write:** `send`, `create_application`, `update_application`, `create_client`
**Destructive:** `delete_message`, `delete_all_messages`, `delete_application`, `delete_client`
**Meta:** `help`

## CLI ↔ MCP Action Parity

Every MCP action has a CLI equivalent (and vice versa). Both shims call the same `GotifyService` method.

| Service Method | MCP Action | CLI Command |
|---|---|---|
| `service.health()` | `gotify(action="health")` | `gotify health` |
| `service.version()` | `gotify(action="version")` | `gotify version` |
| `service.me()` | `gotify(action="me")` | `gotify me` |
| `service.messages(app_id, limit, since)` | `gotify(action="messages", app_id=N, limit=N, since=N)` | `gotify messages [--app-id N] [--limit N] [--since N]` |
| `service.applications()` | `gotify(action="applications")` | `gotify applications` |
| `service.clients()` | `gotify(action="clients")` | `gotify clients` |
| `service.send(msg, title, priority, extras)` | `gotify(action="send", message="...", title="...", priority=N)` | `gotify send <message> [--title T] [--priority N]` |
| `service.create_application(name, desc, pri)` | `gotify(action="create_application", name="...", description="...", default_priority=N)` | `gotify create app <name> [--description D] [--priority N]` |
| `service.update_application(id, name, desc, pri)` | `gotify(action="update_application", app_id=N, name="...", description="...", default_priority=N)` | `gotify update app <id> [--name N] [--description D] [--priority N]` |
| `service.create_client(name)` | `gotify(action="create_client", name="...")` | `gotify create client <name>` |
| `service.delete_message(id, confirm)` | `gotify(action="delete_message", id=N, confirm=true)` | `gotify delete message <id> [--confirm]` |
| `service.delete_all_messages(confirm)` | `gotify(action="delete_all_messages", confirm=true)` | `gotify delete all [--confirm]` |
| `service.delete_application(id, confirm)` | `gotify(action="delete_application", app_id=N, confirm=true)` | `gotify delete app <id> [--confirm]` |
| `service.delete_client(id, confirm)` | `gotify(action="delete_client", client_id=N, confirm=true)` | `gotify delete client <id> [--confirm]` |
| _(MCP-only)_ | `gotify(action="help")` | `gotify --help` |

## Key Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Crate metadata, dependencies |
| `config.toml` | Local dev config (not committed with secrets) |
| `README.md` | User-facing overview, quickstart, full reference |
| `docs/QUICKSTART.md` | 5-minute setup guide |
| `docs/INVENTORY.md` | Complete action/command/env inventory |
| `docs/stack/ARCH.md` | Architecture diagrams |
| `docs/stack/TECH.md` | Technology choices and rationale |

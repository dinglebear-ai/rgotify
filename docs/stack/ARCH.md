# Architecture Overview — gotify-mcp

## System diagram

```
Claude / MCP client  (Claude Code, Codex, curl)
         │
         ▼
HTTP Transport (axum, port 9158)
         │
         ▼
Auth Middleware  (bearer token check)
         │
         ▼
RMCP Streamable HTTP service  (stateless JSON-response mode)
         │
         ▼
mcp/tools.rs  (thin shim: parse JSON args → call service)
         │
         ▼
GotifyService (app.rs)  (business logic, destructive gate)
         │
         ▼
GotifyClient (gotify.rs)  (reqwest HTTP client)
         │  X-Gotify-Key: <token>
         ▼
Gotify REST API  (external server)
```

## Mode dispatch (main.rs)

`main.rs` reads the first CLI argument and routes to one of three entry points:

| Args | Entry point | Description |
|------|-------------|-------------|
| (none), `serve`, `serve mcp` | `serve_mcp()` | RMCP Streamable HTTP on port 9158 |
| `mcp` | `serve_stdio_mcp()` | RMCP stdio transport for local child-process clients |
| anything else | `run_cli()` | Direct CLI execution |

All three entry points load the same `Config` and construct the same `GotifyService`. The transport layer is the only difference.

## Layer responsibilities

### gotify.rs — HTTP REST client

- Owns the `reqwest::Client` instance and base URL.
- Implements all Gotify REST API calls: GET, POST, PUT, DELETE.
- Attaches `X-Gotify-Key` header with the appropriate token per operation type.
- `send_message` uses `app_token`; all other methods use `client_token`.
- HTTP 204 responses are normalized to `{"status":"ok"}`.
- Non-2xx responses bail with the HTTP status and response body.
- No business logic — only HTTP mechanics.

### app.rs — GotifyService (business layer)

- Single `GotifyService` struct shared by CLI and MCP.
- `destructive_gate(confirm)` — central safety check for all destructive operations. Returns `Err` unless `confirm == true` or `allow_destructive == true`.
- All methods delegate directly to `GotifyClient`. No logic beyond the gate.

### mcp/tools.rs — MCP shim

- `execute_tool(state, name, args)` — top-level tool dispatch. Only tool name is `"gotify"`.
- `dispatch(state, args)` — matches on `action` string, extracts typed args from `serde_json::Value`, calls `GotifyService`.
- Zero business logic. If parsing fails, returns a typed `anyhow::Error`.

### cli.rs — CLI shim

- Parses `Vec<String>` args into a typed `CliCommand` enum.
- Calls `GotifyService` methods identically to the MCP shim.
- Formats output as human-readable text or JSON (`--json` flag).

### mcp.rs — HTTP server

- Builds the `axum::Router` with `POST /mcp` and `GET /health` routes.
- `AuthPolicy` enum selects bearer-token enforcement or loopback-dev bypass.
- Mounts the RMCP service as a tower-compatible layer.

## Request flow (MCP)

```
POST /mcp  { "method": "tools/call", "params": { "name": "gotify", "arguments": { "action": "send", ... } } }
   ↓
Auth middleware  (checks Authorization: Bearer header)
   ↓
RMCP layer  (validates JSON-RPC envelope, routes to tool handler)
   ↓
execute_tool("gotify", args)
   ↓
dispatch() → match "send" → GotifyService::send(...)
   ↓
GotifyClient::send_message(...)  →  POST /message  (X-Gotify-Key: <app_token>)
   ↓
Gotify server returns created message JSON
   ↓
Value returned up the call stack → wrapped in MCP content → JSON-RPC response
```

## Auth model

| Condition | Auth behavior |
|-----------|---------------|
| `GOTIFY_MCP_TOKEN` set | Bearer token required on all `/mcp` requests |
| `GOTIFY_MCP_NO_AUTH=true` | Auth disabled |
| Bind host starts with `127.` | Auth disabled (loopback dev mode) |
| stdio mode | No auth (process boundary is the trust boundary) |

`/health` is always unauthenticated.

## Error handling

| Source | Error | Behavior |
|--------|-------|----------|
| Auth middleware | Missing/invalid bearer token | HTTP 401 |
| Tool dispatch | Unknown tool name | RMCP tool error |
| Tool handler | Missing required arg | RMCP invalid params error |
| Destructive gate | No confirm, no env flag | `anyhow::Error` → MCP `isError: true` |
| Gotify client | Non-2xx HTTP response | `anyhow::bail!` with status + body |
| Config | Empty `GOTIFY_URL` | `anyhow::bail!` at startup |

## Cross-references

- [TECH.md](TECH.md) — technology choices and rationale
- [../INVENTORY.md](../INVENTORY.md) — complete action/command/env inventory
- [../../README.md](../../README.md) — user-facing reference

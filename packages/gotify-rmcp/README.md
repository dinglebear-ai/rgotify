# gotify-rmcp

Rust MCP server that bridges Claude (and other MCP clients) to a self-hosted [Gotify](https://gotify.net/) push notification server. Send notifications, manage applications and clients, and query messages — all from Claude or the CLI.


## npm / npx

Run the stdio MCP server or CLI without a manual binary install:

```bash
npx -y gotify-rmcp --help
```

MCP clients can use the same launcher:

```json
{
  "mcpServers": {
    "gotify-rmcp": {
      "command": "npx",
      "args": ["-y", "gotify-rmcp"]
    }
  }
}
```

The npm package downloads the `rgotify` binary from GitHub Releases during `postinstall` and keeps the release tag aligned with `packages/gotify-rmcp/package.json`.

Naming follows the rmcp family pattern: repo and npm package use `<service>-rmcp` (`gotify-rmcp`), while the CLI keeps the short Rust binary alias `r<service>` (`rgotify`). Launcher install overrides use the `GOTIFY_RMCP_*` env prefix.

## Overview

```
Claude / MCP client
       │
       ▼
RMCP Streamable HTTP :9158/mcp   (or stdio for local clients)
       │
       ▼
  gotify-rmcp (GotifyService)
       │  X-Gotify-Key header
       ▼
  Gotify REST API
```

The binary runs in three modes:

| Mode | Invocation | Use case |
|------|------------|----------|
| HTTP MCP server | `gotify` or `gotify serve` | Claude Code, remote MCP clients |
| stdio MCP server | `gotify mcp` | Local child-process MCP clients |
| CLI | `gotify <command>` | Direct shell use |

---

## Token Model

Gotify uses two distinct token types. `gotify-rmcp` requires both.

| Token | Env var | Prefix | Used for |
|-------|---------|--------|----------|
| Client token | `GOTIFY_CLIENT_TOKEN` | `C` | Management: list, create, delete messages/apps/clients, read current user |
| App token | `GOTIFY_APP_TOKEN` | `A` | Sending messages (`POST /message` only) |

The client token authorizes all read and management operations. The app token is used exclusively for `send`. Both must be set; if the app token is empty, `send` fails with an explicit error.

---

## Quickstart

1. Create a Gotify client token and an app token in your Gotify web UI (or see [docs/QUICKSTART.md](docs/QUICKSTART.md)).

2. Set environment variables:

```bash
export GOTIFY_URL=https://gotify.example.com
export GOTIFY_CLIENT_TOKEN=Cxxxxxxxxxxxxxxxx
export GOTIFY_APP_TOKEN=Axxxxxxxxxxxxxxxx
```

3. Run the MCP server:

```bash
# Build
cargo build --release

# HTTP MCP server (port 9158)
./target/release/gotify

# Or with an MCP bearer token
GOTIFY_MCP_TOKEN=my-secret ./target/release/gotify
```

4. Connect Claude Code (`~/.claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "gotify": {
      "type": "http",
      "url": "http://localhost:9158/mcp",
      "headers": {
        "Authorization": "Bearer my-secret"
      }
    }
  }
}
```

For stdio mode:

```json
{
  "mcpServers": {
    "gotify": {
      "command": "/path/to/gotify",
      "args": ["mcp"],
      "env": {
        "GOTIFY_URL": "https://gotify.example.com",
        "GOTIFY_CLIENT_TOKEN": "Cxxxxxxxxxxxxxxxx",
        "GOTIFY_APP_TOKEN": "Axxxxxxxxxxxxxxxx"
      }
    }
  }
}
```

---

## MCP Tool Reference

One MCP tool is exposed: `gotify`. Use the required `action` argument to select the operation.

### Read actions

| Action | Description | Required params | Optional params |
|--------|-------------|-----------------|-----------------|
| `health` | Server health check (no auth) | — | — |
| `version` | Server version (no auth) | — | — |
| `me` | Current authenticated user | — | — |
| `messages` | List messages | — | `app_id`, `limit` (default 50), `since` |
| `applications` | List all applications | — | — |
| `clients` | List all clients | — | — |

### Write actions

| Action | Description | Required params | Optional params |
|--------|-------------|-----------------|-----------------|
| `send` | Send a push notification | `message` | `title`, `priority`, `extras` |
| `create_application` | Create an application | `name` | `description`, `default_priority` |
| `update_application` | Update an application | `app_id` | `name`, `description`, `default_priority` |
| `create_client` | Create a client | `name` | — |

### Destructive actions

These require either `confirm=true` in the call arguments or `GOTIFY_ALLOW_DESTRUCTIVE=true` in the environment. Without one of these, the call returns: `"destructive operation — pass confirm=true or set GOTIFY_ALLOW_DESTRUCTIVE=true"`.

| Action | Description | Required params |
|--------|-------------|-----------------|
| `delete_message` | Delete one message | `id`, `confirm` |
| `delete_all_messages` | Delete all messages | `confirm` |
| `delete_application` | Delete an application | `app_id`, `confirm` |
| `delete_client` | Delete a client | `client_id`, `confirm` |

### Meta

| Action | Description |
|--------|-------------|
| `help` | Returns built-in markdown documentation |

### MCP call examples (raw JSON-RPC)

```bash
# Health check
curl -s -X POST http://localhost:9158/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"gotify","arguments":{"action":"health"}}}'

# Send a notification
curl -s -X POST http://localhost:9158/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Authorization: Bearer my-secret" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"gotify","arguments":{"action":"send","message":"Deploy complete","title":"CI","priority":5}}}'

# List recent messages
curl -s -X POST http://localhost:9158/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Authorization: Bearer my-secret" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"gotify","arguments":{"action":"messages","limit":10}}}'
```

---

## CLI Reference

The same binary provides a direct CLI. All commands use the same service layer as the MCP tool.

### Read commands

```bash
gotify health [--json]
gotify version [--json]
gotify me [--json]
gotify messages [--app-id N] [--limit N] [--since N] [--json]
gotify applications [--json]
gotify clients [--json]
```

### Write commands

```bash
gotify send <message> [--title T] [--priority N] [--json]
gotify create app <name> [--description D] [--priority N] [--json]
gotify create client <name> [--json]
```

### Destructive commands (add `--confirm` or set `GOTIFY_ALLOW_DESTRUCTIVE=true`)

```bash
gotify delete message <id> [--confirm] [--json]
gotify delete all [--confirm] [--json]
gotify delete app <app_id> [--confirm] [--json]
gotify delete client <client_id> [--confirm] [--json]
```

### Server modes

```bash
gotify                  # Start HTTP MCP server (port 9158)
gotify serve            # Same as above
gotify serve mcp        # Same as above
gotify mcp              # Start stdio MCP transport
gotify --help           # Print usage
gotify --version        # Print version
```

---

## Destructive Operation Safety

Destructive operations (delete) require explicit confirmation to prevent accidental data loss. Two ways to authorize:

**Per-call**: Pass `confirm=true` in MCP args or `--confirm` on the CLI.

**Global**: Set `GOTIFY_ALLOW_DESTRUCTIVE=true` to skip the gate for all operations in the session. Use this in automated pipelines only.

Without confirmation, the error is: `"destructive operation — pass confirm=true or set GOTIFY_ALLOW_DESTRUCTIVE=true"`.

---

## Configuration Reference

Configuration is loaded from `config.toml` (if present), then overridden by environment variables.

### Required

| Variable | Description |
|----------|-------------|
| `GOTIFY_URL` | Gotify server base URL, e.g. `https://gotify.example.com` |
| `GOTIFY_CLIENT_TOKEN` | Client token (starts with `C`) for management operations |
| `GOTIFY_APP_TOKEN` | App token (starts with `A`) for sending messages |

### Gotify behavior

| Variable | Default | Description |
|----------|---------|-------------|
| `GOTIFY_ALLOW_DESTRUCTIVE` | `false` | Skip confirm gate for destructive operations |

### MCP server

| Variable | Default | Description |
|----------|---------|-------------|
| `GOTIFY_MCP_HOST` | `0.0.0.0` | Bind host for the HTTP MCP server |
| `GOTIFY_MCP_PORT` | `9158` | Bind port for the HTTP MCP server |
| `GOTIFY_MCP_TOKEN` | — | Static bearer token for MCP auth. Omit to disable auth. |
| `GOTIFY_MCP_NO_AUTH` | `false` | Disable MCP authentication entirely |
| `RUST_LOG` | `info` | Log filter (`trace`, `debug`, `info`, `warn`, `error`) |

### HTTP endpoints

| Endpoint | Auth | Description |
|----------|------|-------------|
| `POST /mcp` | Bearer token (when set) | RMCP Streamable HTTP endpoint |
| `GET /health` | None | Health probe — always returns `{"status":"ok"}` |

---

## Authentication

When `GOTIFY_MCP_TOKEN` is set, all requests to `/mcp` must include:

```
Authorization: Bearer <token>
```

`/health` is always unauthenticated.

When the server binds to a loopback address (`127.x.x.x`) or `GOTIFY_MCP_NO_AUTH=true`, authentication is disabled.

stdio mode does not use bearer auth — the local process boundary is the trust boundary.

Generate a token:

```bash
openssl rand -hex 32
```

---

## Development

```bash
cargo build              # debug build
cargo build --release    # release build
cargo check              # typecheck without linking
cargo clippy             # lint (no warnings allowed)
cargo fmt                # format
cargo test               # test suite
```

---

## License

MIT

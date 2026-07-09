# Component Inventory — gotify-rmcp

Complete listing of all MCP actions, CLI commands, environment variables, and HTTP endpoints.

## MCP tool: `gotify`

One MCP tool is exposed. The required `action` argument selects the operation.

### Read actions

| Action | Description | Required params | Optional params |
|--------|-------------|-----------------|-----------------|
| `health` | Server health check (no Gotify auth) | — | — |
| `version` | Server version (no Gotify auth) | — | — |
| `me` | Current authenticated user | — | — |
| `messages` | List messages | — | `app_id` (integer), `limit` (integer, default 50), `since` (integer message ID) |
| `applications` | List all applications | — | — |
| `clients` | List all clients | — | — |

### Write actions

| Action | Description | Required params | Optional params |
|--------|-------------|-----------------|-----------------|
| `send` | Send a push notification | `message` (string) | `title` (string), `priority` (integer), `extras` (object) |
| `create_application` | Create an application | `name` (string) | `description` (string), `default_priority` (integer) |
| `update_application` | Update an application | `app_id` (integer) | `name` (string), `description` (string), `default_priority` (integer) |
| `create_client` | Create a client | `name` (string) | — |

### Destructive actions

Require `confirm=true` or `GOTIFY_ALLOW_DESTRUCTIVE=true`. Without either, returns error: `"destructive operation — pass confirm=true or set GOTIFY_ALLOW_DESTRUCTIVE=true"`.

| Action | Description | Required params |
|--------|-------------|-----------------|
| `delete_message` | Delete one message | `id` (integer), `confirm` (boolean) |
| `delete_all_messages` | Delete all messages | `confirm` (boolean) |
| `delete_application` | Delete an application and all its messages | `app_id` (integer), `confirm` (boolean) |
| `delete_client` | Delete a client | `client_id` (integer), `confirm` (boolean) |

### Meta

| Action | Description |
|--------|-------------|
| `help` | Built-in markdown documentation for all actions |

## CLI commands

All CLI commands call the same `GotifyService` methods as the MCP actions.

### Read commands

```
gotify health [--json]
gotify version [--json]
gotify me [--json]
gotify messages [--app-id N] [--limit N] [--since N] [--json]
gotify applications [--json]
gotify clients [--json]
```

### Write commands

```
gotify send <message> [--title T] [--priority N] [--json]
gotify create app <name> [--description D] [--priority N] [--json]
gotify create client <name> [--json]
```

### Destructive commands

```
gotify delete message <id> [--confirm] [--json]
gotify delete all [--confirm] [--json]
gotify delete app <app_id> [--confirm] [--json]
gotify delete client <client_id> [--confirm] [--json]
```

### Server modes

```
gotify                  # HTTP MCP server (port 9158)
gotify serve            # Same
gotify serve mcp        # Same
gotify mcp              # stdio MCP transport
gotify --help
gotify --version
```

## Environment variables

| Variable | Required | Default | Sensitive | Description |
|----------|----------|---------|-----------|-------------|
| `GOTIFY_URL` | yes | — | no | Gotify server base URL |
| `GOTIFY_CLIENT_TOKEN` | yes | — | yes | Client token (starts `C`) for management |
| `GOTIFY_APP_TOKEN` | yes for send | — | yes | App token (starts `A`) for sending |
| `GOTIFY_ALLOW_DESTRUCTIVE` | no | `false` | no | Skip confirm gate for destructive ops |
| `GOTIFY_MCP_HOST` | no | `0.0.0.0` | no | MCP HTTP bind host |
| `GOTIFY_MCP_PORT` | no | `9158` | no | MCP HTTP bind port |
| `GOTIFY_MCP_TOKEN` | no | — | yes | Static bearer token for MCP auth |
| `GOTIFY_MCP_NO_AUTH` | no | `false` | no | Disable MCP authentication |
| `GOTIFY_MCP_PUBLIC_URL` | no | — | no | Public URL (reserved for OAuth mode) |
| `GOTIFY_MCP_AUTH_ADMIN_EMAIL` | no | — | no | Admin email (reserved for OAuth mode) |
| `RUST_LOG` | no | `info` | no | Log filter |

## HTTP endpoints

| Endpoint | Method | Auth required | Description |
|----------|--------|---------------|-------------|
| `/mcp` | POST | Yes (when token set) | RMCP Streamable HTTP endpoint |
| `/health` | GET | No | Health probe — always `{"status":"ok"}` |

## Network ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 9158 | TCP | RMCP Streamable HTTP MCP server |

## Runtime dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `axum` | HTTP framework |
| `rmcp` | MCP SDK (Streamable HTTP + stdio) |
| `tower-http` | CORS and tracing middleware |
| `reqwest` | Gotify REST API HTTP client |
| `serde` / `serde_json` | Serialization |
| `chrono` | Timestamps |
| `toml` | Config file parsing |
| `lab-auth` | OAuth/JWT auth |
| `tracing` / `tracing-subscriber` | Structured logging |
| `anyhow` | Error handling |
| `url` | URL validation |

## Development dependencies

| Crate | Purpose |
|-------|---------|
| `tempfile` | Temporary directories for tests |
| `tower` | HTTP testing utilities |
| `rmcp` (client features) | MCP client for integration tests |

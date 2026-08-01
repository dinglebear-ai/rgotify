# gotify-rmcp

MCP server and CLI for Gotify: send push notifications and manage messages, apps, and clients over stdio or streamable HTTP, with auth.

It exposes one MCP tool, `gotify`, plus the `rgotify` CLI. Agents can send
notifications, inspect server health, list messages, and manage Gotify apps and
clients through stdio MCP, Streamable HTTP MCP, or direct shell commands.

**30-second path:** set `GOTIFY_URL`, then run `npx -y @dinglebear/rgotify health --json`
-> start loopback HTTP with `GOTIFY_MCP_HOST=127.0.0.1 npx -y @dinglebear/rgotify serve`
-> call `tools/call` with `{"action":"health"}`.

**Status:** operational RMCP upstream-client server. Write-capable; destructive
delete actions are gated by explicit confirmation. HTTP MCP supports loopback
dev mode, static bearer tokens, and Google OAuth through `lab-auth`.

**Not for:** replacing Gotify, storing notifications independently, generic
webhook routing, scheduling reminders, multi-tenant isolation, or passing Gotify
tokens through MCP tool arguments.

## Contents

- [Naming](#naming)
- [Capabilities And Boundaries](#capabilities-and-boundaries)
- [Install](#install)
- [Quickstart](#quickstart)
- [Client Configuration](#client-configuration)
- [Runtime Surfaces](#runtime-surfaces)
- [MCP Tool Reference](#mcp-tool-reference)
- [CLI Reference](#cli-reference)
- [Configuration](#configuration)
- [Authentication](#authentication)
- [Safety And Trust Model](#safety-and-trust-model)
- [Architecture](#architecture)
- [Distribution Contract](#distribution-contract)
- [Development](#development)
- [Verification](#verification)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)
- [Related Servers](#related-servers)
- [Documentation](#documentation)
- [License](#license)

## Naming

| Surface | This repo |
|---|---|
| Repository | `dinglebear-ai/rgotify` |
| Rust crate (Cargo package) | `gotify-mcp` |
| Binary / CLI | `rgotify` |
| npm package | `@dinglebear/rgotify` |
| npm binary aliases | `gotify-rmcp`, `rgotify` |
| MCP tool | `gotify` |
| MCP registry name | `ai.dinglebear/rgotify` |
| Config home | `~/.gotify` on hosts, `/data` in containers |
| Env prefixes | `GOTIFY_*`, `GOTIFY_MCP_*`, `GOTIFY_RMCP_*` for npm launcher controls |

These names intentionally differ. The npm package and registry entry use the
RMCP family name, the Cargo package is `gotify-mcp`, the git repo is `rgotify`,
and the shipped binary uses the short Rust CLI name `rgotify`.

## Capabilities And Boundaries

- Send Gotify push notifications with message, title, priority, and extras.
- Read server health, runtime status, server version, current user, messages,
  applications, and clients.
- Create or update applications and create clients.
- Delete messages, all messages, applications, or clients only after explicit
  destructive confirmation.
- Expose MCP prompts for common workflows and a resource containing the current
  tool schema.

| This repo owns | Gotify owns | Explicitly out of scope |
|---|---|---|
| MCP/CLI projection, request validation, auth policy, response shaping, setup checks, destructive gates. | Notification storage, delivery, Gotify users, token issuance, app/client state, upstream API semantics. | Notification scheduling, independent persistence, arbitrary webhook relay behavior, multi-tenant sandboxing, credential brokerage. |

## Install

| Path | Command | Best for | Notes |
|---|---|---|---|
| npm / npx | `npx -y @dinglebear/rgotify --help` | Local MCP clients and quick trials. | Downloads the matching `rgotify` binary from GitHub Releases. |
| Release installer | `curl -fsSL https://raw.githubusercontent.com/dinglebear-ai/rgotify/main/scripts/install.sh \| bash` | Host installs without Node. | Installs `rgotify` for the current Linux host. |
| Docker / Compose | `docker compose up -d` | Shared HTTP MCP deployments. | Reads `.env` and exposes container port `40020`. |
| Build from source | `cargo build --release` | Development and audits. | Produces `target/release/rgotify`. |
| Plugin | `claude plugin install plugins/gotify` | Claude Code local plugin setup from this checkout. | Ships no hooks — run `rgotify setup repair` once by hand afterwards. |

### npm / npx

Run the stdio MCP server or CLI without a manual binary install:

```bash
npx -y @dinglebear/rgotify --help
npx -y @dinglebear/rgotify mcp
npx -y @dinglebear/rgotify health --json
```

The npm package downloads `rgotify` during `postinstall`. Override download
behavior only when testing packaging:

| Variable | Purpose |
|---|---|
| `GOTIFY_RMCP_SKIP_DOWNLOAD=1` | Skip postinstall binary download. |
| `GOTIFY_RMCP_VERSION` or `GOTIFY_RMCP_BINARY_VERSION` | Select the GitHub Release tag. |
| `GOTIFY_RMCP_REPO` | Select the GitHub repo used for release downloads. |
| `GOTIFY_RMCP_RELEASE_BASE_URL` | Select a custom release base URL. |

### Build From Source

```bash
git clone https://github.com/dinglebear-ai/rgotify
cd rgotify
cargo build --release
./target/release/rgotify --help
```

Minimum supported Rust version: 1.86.

## Quickstart

### 1. Configure Gotify

For the safest first call, only `GOTIFY_URL` is required:

```bash
export GOTIFY_URL=https://gotify.example.com
```

Create tokens in the Gotify web UI before using management or send actions:

```bash
export GOTIFY_CLIENT_TOKEN=Cxxxxxxxxxxxxxxxx
export GOTIFY_APP_TOKEN=Axxxxxxxxxxxxxxxx
```

Token roles:

| Token | Env var | Used for |
|---|---|---|
| Client token | `GOTIFY_CLIENT_TOKEN` | Read and management actions such as messages, apps, clients, and current user. |
| App token | `GOTIFY_APP_TOKEN` | Sending notifications with `send`. |

### 2. Run A Safe CLI Call

```bash
npx -y @dinglebear/rgotify health --json
```

### 3. Start Loopback HTTP MCP

```bash
GOTIFY_MCP_HOST=127.0.0.1 npx -y @dinglebear/rgotify serve
```

In another shell:

```bash
curl -sf http://127.0.0.1:40020/health
```

### 4. Make A First MCP Call

```bash
curl -s -X POST http://127.0.0.1:40020/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"gotify","arguments":{"action":"health"}}}'
```

## Client Configuration

### Claude Code Stdio

```json
{
  "mcpServers": {
    "gotify": {
      "command": "npx",
      "args": ["-y", "gotify-rmcp", "mcp"],
      "env": {
        "GOTIFY_URL": "https://gotify.example.com",
        "GOTIFY_CLIENT_TOKEN": "Cxxxxxxxxxxxxxxxx",
        "GOTIFY_APP_TOKEN": "Axxxxxxxxxxxxxxxx"
      }
    }
  }
}
```

### Claude Code HTTP

```json
{
  "mcpServers": {
    "gotify": {
      "type": "http",
      "url": "http://127.0.0.1:40020/mcp",
      "headers": {
        "Authorization": "Bearer ${GOTIFY_MCP_TOKEN}"
      }
    }
  }
}
```

### Codex / Labby Gateway

Register Gotify through Labby as an HTTP upstream when sharing one long-running
server, or run it directly as stdio for local-only use.

```toml
[mcp_servers.gotify]
command = "npx"
args = ["-y", "gotify-rmcp", "mcp"]
```

### Generic MCP JSON

```json
{
  "command": "rgotify",
  "args": ["mcp"],
  "env": {
    "GOTIFY_URL": "https://gotify.example.com"
  }
}
```

Do not put API keys, passwords, OAuth secrets, SSH keys, Gotify client tokens,
Gotify app tokens, or upstream bearer tokens in MCP tool arguments. Use env,
config files, or the MCP client's secret storage.

## Runtime Surfaces

| Surface | Status | Entry point | Purpose |
|---|---:|---|---|
| MCP stdio | Supported | `rgotify mcp`, `npx -y @dinglebear/rgotify mcp` | Local child-process MCP clients. |
| MCP HTTP | Supported | `rgotify serve`, `POST /mcp` | Streamable HTTP MCP for local or shared server deployments. |
| CLI | Supported | `rgotify <command>` | Scriptable parity and debugging. |
| Prompts | Supported | `send_notification`, `check_status` | Reusable agent prompts. |
| Resource | Supported | `gotify://schema/mcp-tool` | JSON schema for the `gotify` tool. |
| REST API | Not shipped | N/A | Gotify already owns the REST API. |
| Web UI | Not shipped | N/A | Gotify already owns the web UI. |

## MCP Tool Reference

One MCP tool is exposed: `gotify`. Pass the required `action` argument to select
the operation.

### Read Actions

| Action | Description | Required params | Optional params |
|---|---|---|---|
| `health` | Gotify server health check. | none | none |
| `version` | Gotify server version. | none | none |
| `me` | Current authenticated user. | none | none |
| `messages` | List messages. | none | `app_id`, `limit`, `since` |
| `applications` | List applications. | none | none |
| `clients` | List clients. | none | none |
| `status` | Return runtime status, config snapshot, and counters. | none | none |

### Write Actions

| Action | Description | Required params | Optional params |
|---|---|---|---|
| `send` | Send a push notification. | `message` | `title`, `priority`, `extras` |
| `create_application` | Create an application. | `name` | `description`, `default_priority` |
| `update_application` | Update an application. | `app_id` | `name`, `description`, `default_priority` |
| `create_client` | Create a client. | `name` | none |

### Destructive Actions

Destructive actions require `confirm=true` in MCP arguments, `--confirm` on the
CLI, or `GOTIFY_ALLOW_DESTRUCTIVE=true` in the process environment.

| Action | Description | Required params |
|---|---|---|
| `delete_message` | Delete one message. | `id`, `confirm` |
| `delete_all_messages` | Delete all messages. | `confirm` |
| `delete_application` | Delete an application and its messages. | `app_id`, `confirm` |
| `delete_client` | Delete a client. | `client_id`, `confirm` |

### Meta, Prompts, And Resource

| Primitive | Name / URI | Purpose |
|---|---|---|
| Tool action | `help` | Return built-in markdown tool help. |
| Prompt | `send_notification` | Guide an agent through a notification send. |
| Prompt | `check_status` | Check health and recent messages. |
| Resource | `gotify://schema/mcp-tool` | Return the current action-based JSON schema. |

Curated action summaries live here. The current branch source code and
`docs/INVENTORY.md` are the source of truth for complete parameters until a
generated `docs/MCP_SCHEMA.md` is added.

## CLI Reference

The CLI calls the same service methods as the MCP tool.

```bash
rgotify health [--json]
rgotify version [--json]
rgotify me [--json]
rgotify messages [--app-id N] [--limit N] [--since N] [--json]
rgotify applications [--json]
rgotify clients [--json]

rgotify send <message> [--title T] [--priority N] [--json]
rgotify create app <name> [--description D] [--priority N] [--json]
rgotify update app <app_id> [--name N] [--description D] [--priority N] [--json]
rgotify create client <name> [--json]

rgotify delete message <id> [--confirm] [--json]
rgotify delete all [--confirm] [--json]
rgotify delete app <app_id> [--confirm] [--json]
rgotify delete client <client_id> [--confirm] [--json]

rgotify serve
rgotify serve mcp
rgotify mcp
rgotify doctor [--json]
rgotify setup check [--json]
rgotify setup repair [--json]
rgotify setup install [--json]
rgotify setup plugin-hook [--no-repair] [--json]
```

Hyphenated aliases are accepted for the two-word forms: `create-app`,
`update-app`, `create-client`, `delete-message`, `delete-all`, `delete-app`,
`delete-client`.

Known parity exception: MCP `action=status` is MCP-only observability. The CLI
equivalent for operator checks is `rgotify doctor --json`.

## Configuration

Configuration loads from `config.toml` when present, then environment variables
override those values. On startup, the binary also loads `~/.gotify/.env` on
hosts or `/data/.env` in containers without overriding already-set variables.

### Required Upstream Variables

| Variable | Required | Description |
|---|---:|---|
| `GOTIFY_URL` | yes | Gotify server base URL, for example `https://gotify.example.com`. |
| `GOTIFY_CLIENT_TOKEN` | for management | Gotify client token for read and management actions. |
| `GOTIFY_APP_TOKEN` | for `send` | Gotify app token used only to send notifications. |

### Runtime Variables

| Variable | Default | Description |
|---|---|---|
| `GOTIFY_ALLOW_DESTRUCTIVE` | `false` | Skip destructive confirmation gates. |
| `GOTIFY_MCP_HOST` | `0.0.0.0` | HTTP MCP bind host. |
| `GOTIFY_MCP_PORT` | `40020` | HTTP MCP bind port. |
| `GOTIFY_MCP_TOKEN` | empty | Static bearer token for HTTP MCP when not in loopback dev mode. |
| `GOTIFY_MCP_NO_AUTH` | `false` | Disable HTTP MCP auth. Use only on loopback or behind a trusted gateway. |
| `GOTIFY_MCP_AUTH_MODE` | `bearer` | Set to `oauth` for Google OAuth through `lab-auth`. |
| `GOTIFY_MCP_PUBLIC_URL` | empty | Public URL for OAuth metadata and protected-resource discovery. |
| `GOTIFY_MCP_GOOGLE_CLIENT_ID` | empty | Google OAuth client ID. |
| `GOTIFY_MCP_GOOGLE_CLIENT_SECRET` | empty | Google OAuth client secret. |
| `GOTIFY_MCP_AUTH_ADMIN_EMAIL` | empty | Initial/admin OAuth email. |
| `GOTIFY_MCP_AUTH_SQLITE_PATH` | `<data>/auth.db` | OAuth state database path. |
| `GOTIFY_MCP_AUTH_KEY_PATH` | `<data>/auth-jwt.pem` | OAuth JWT signing key path. |
| `GOTIFY_MCP_ALLOWED_HOSTS` | empty | Comma-separated `Host` header allowlist. |
| `GOTIFY_MCP_ALLOWED_ORIGINS` | empty | Comma-separated `Origin` header allowlist. |
| `GOTIFY_NOAUTH` | `false` | Escape hatch permitting a non-loopback bind with no auth. See below. |
| `GOTIFY_MCP_HOME` | `~/.gotify` or `/data` | Override the appdata dir used by `rgotify setup`. |
| `RUNNING_IN_CONTAINER` | unset | Forces the `/data` appdata path. |
| `RUST_LOG` | `info` | Rust log filter. Stdio logs must stay off stdout. |

`GOTIFY_MCP` is also the `lab-auth` env prefix, so `lab-auth` reads further
`GOTIFY_MCP_*` keys beyond those listed here.

### Startup Bind Guard

The server refuses to start when it would bind a non-loopback host with no
authentication configured. To bind `0.0.0.0`, set `GOTIFY_MCP_TOKEN`, or use
`GOTIFY_MCP_AUTH_MODE=oauth`, or — only when an upstream gateway genuinely
enforces auth — set `GOTIFY_NOAUTH=true`.

## Authentication

| Policy | When | Effect |
|---|---|---|
| Loopback development | `GOTIFY_MCP_HOST` starts with `127.` or `GOTIFY_MCP_NO_AUTH=true` | No HTTP auth layer is mounted. Use for local testing only. |
| Static bearer | `GOTIFY_MCP_TOKEN` is set and the server is not loopback dev | `/mcp` requires `Authorization: Bearer <token>`. |
| OAuth | `GOTIFY_MCP_AUTH_MODE=oauth` plus Google OAuth settings | `/mcp` uses `lab-auth` OAuth and scoped bearer tokens. |
| Stdio | `rgotify mcp` | The local child-process boundary is the trust boundary. |

MCP scopes are `gotify:read` and `gotify:write`. The static bearer token grants
both scopes. OAuth tokens are checked before MCP calls are dispatched.

## Safety And Trust Model

- MCP callers never provide Gotify client tokens, Gotify app tokens, OAuth
  secrets, static bearer tokens, passwords, or API keys as tool arguments.
- Upstream credentials are loaded from env/config only.
- Delete actions require `confirm=true`, `--confirm`, or the explicit
  `GOTIFY_ALLOW_DESTRUCTIVE=true` process override.
- Gotify is the durable source of notification state; this server is a thin
  projection over that API.
- Stdio mode runs with the user's local permissions and is not a sandbox.
- HTTP mode should not be exposed beyond loopback without bearer or OAuth auth
  plus TLS from an upstream reverse proxy.

## Architecture

```text
MCP client / CLI
       |
       v
rgotify
       |
       +-- MCP shim: JSON args -> GotifyService -> structured result
       +-- CLI shim: argv -> GotifyService -> stdout
       |
       v
GotifyService
       |
       v
GotifyClient
       |
       v
Gotify REST API
```

| Path | Role |
|---|---|
| `src/app.rs` | Business service layer, destructive gate, response shaping. |
| `src/gotify.rs` | Gotify REST client. |
| `src/mcp/` | RMCP tool, prompts, resource, schema, and auth checks. |
| `src/cli/` | CLI parser, doctor, setup helpers, and output formatting. |
| `src/config.rs` | Env/config loading and defaults. |
| `packages/gotify-rmcp/` | npm launcher and release-binary downloader. |

The thin-shim rule is intentional: MCP and CLI parse inputs, call
`GotifyService`, and return output. Credential handling, destructive gates, and
Gotify API behavior stay outside the MCP and CLI shims.

## Distribution Contract

| Artifact | File(s) | Must align with |
|---|---|---|
| Rust crate/binary | `Cargo.toml`, `Cargo.lock` | Git tag, release assets, CLI docs, install scripts. |
| npm launcher | `packages/gotify-rmcp/package.json`, `bin/rgotify.js`, `lib/platform.js`, `scripts/install.js` | GitHub Release tag and assets named `rgotify-x86_64.tar.gz` and `rgotify-windows-x86_64.tar.gz`. |
| GitHub Releases | `.github/workflows/*`, `scripts/install.sh` | Package version, binary name, checksums, supported platforms. |
| Docker / Compose | `config/Dockerfile`, `docker-compose*.yml` | Exposed port `40020`, healthcheck `/health`, env file contract. |
| MCP registry | `server.json` | Server identity `tv.tootie/gotify-rmcp`, env vars, transport URL, package version. |
| Plugin | `plugins/gotify` | Runtime command, user config, bundled metadata. No hooks are shipped. |
| Docs | `README.md`, `docs/INVENTORY.md`, `docs/QUICKSTART.md` | Current binary name, default port, action list, and env names. |

Release invariant: npm package version, Rust crate version, `server.json.version`,
GitHub Release tag, release asset names, and README install examples should move
together. README examples must use canonical repo and binary names, not older
aliases.

## Development

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
npm --prefix packages/gotify-rmcp run check
```

## Verification

```bash
# Binary and CLI
cargo build --release
./target/release/rgotify --version
GOTIFY_URL=https://gotify.example.com ./target/release/rgotify health --json

# HTTP health
GOTIFY_URL=https://gotify.example.com GOTIFY_MCP_HOST=127.0.0.1 ./target/release/rgotify serve
curl -sf http://127.0.0.1:40020/health

# MCP tool call
curl -s -X POST http://127.0.0.1:40020/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"gotify","arguments":{"action":"health"}}}'
```

For live send or management tests, add `GOTIFY_CLIENT_TOKEN` and
`GOTIFY_APP_TOKEN` from your Gotify instance.

## Deployment

### Docker / Compose

```bash
cp .env.example .env
$EDITOR .env
docker compose up -d
curl -sf http://127.0.0.1:40020/health
```

The container stores app data under `/data`, normally mounted from
`${HOME}/.gotify`.

### Reverse Proxy

Expose only `/mcp` and `/health`. Preserve Streamable HTTP headers, require TLS,
and configure bearer or OAuth auth before exposing the server beyond loopback.

### Plugin

The plugin ships **no Claude Code hooks**, so nothing runs setup for you. Run it
once by hand after installing or updating the plugin:

```bash
claude plugin install plugins/gotify

rgotify setup repair     # create ~/.gotify and its .env, then re-check
rgotify setup install    # copy the binary into ~/.local/bin so it is on PATH
rgotify setup check      # read-only verification
```

`rgotify setup repair` creates the appdata dir and a placeholder `.env`;
`rgotify setup install` keeps a terminal-callable copy in `~/.local/bin` (repeat
it after `/plugin update`). `rgotify setup check` verifies appdata, `.env`,
binary-on-PATH, and that port 40020 is free. The server itself takes its config
from the plugin's `.mcp.json` `${user_config.*}` block, so these commands
bootstrap the local environment rather than configure the server.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `401` from `/mcp` | Missing or wrong bearer/OAuth token. | Check `GOTIFY_MCP_TOKEN` and client headers, or use loopback dev mode locally. |
| CLI health fails | `GOTIFY_URL` is missing or unreachable. | Export `GOTIFY_URL` and confirm Gotify is reachable from this host. |
| `send` fails with auth error | Wrong token type. | Use `GOTIFY_APP_TOKEN` for send and `GOTIFY_CLIENT_TOKEN` for management. |
| Destructive action is blocked | Confirmation gate is working. | Add `confirm=true`, `--confirm`, or a deliberate `GOTIFY_ALLOW_DESTRUCTIVE=true`. |
| stdio MCP JSON parse errors | Logs went to stdout. | Keep protocol logs off stdout and lower `RUST_LOG` if needed. |
| npm launcher cannot find binary | Release asset download failed or was skipped. | Reinstall, check `GOTIFY_RMCP_VERSION`, or build `rgotify` from source. |

## Related Servers

- [soma](https://github.com/dinglebear-ai/soma) - RMCP runtime for provider-backed MCP servers.
- [unifi-rmcp](https://github.com/dinglebear-ai/runifi) - UniFi controller REST API bridge.
- [tailscale-rmcp](https://github.com/dinglebear-ai/rtailscale) - Tailscale API bridge for devices, users, and tailnet operations.
- [unraid](https://github.com/dinglebear-ai/unraid) - Unraid monorepo: GraphQL MCP bridges (`runraid`) and Unraid plugins.
- [apprise-rmcp](https://github.com/dinglebear-ai/rapprise) - Apprise notification fan-out bridge for many delivery backends.
- [arcane-rmcp](https://github.com/dinglebear-ai/rarcane) - Arcane Docker management bridge for containers and related resources.
- [yarr](https://github.com/dinglebear-ai/yarr) - Media-stack bridge for Sonarr, Radarr, Prowlarr, Plex, and related services.
- [ytdl-rmcp](https://github.com/dinglebear-ai/rytdl) - Media download and metadata workflow server.
- [synapse-rmcp](https://github.com/dinglebear-ai/synapse) - Local Synapse workflow server for scout and flux actions.
- [cortex](https://github.com/dinglebear-ai/cortex) - Syslog and homelab log aggregation MCP server.
- [axon](https://github.com/dinglebear-ai/axon) - RAG, crawl, scrape, extract, and semantic search project.
- [labby](https://github.com/dinglebear-ai/labby) - Homelab control plane and MCP gateway project.
- [lumen](https://github.com/dinglebear-ai/lumen) - Local semantic code search MCP server.

## Documentation

Start here:

- [`docs/QUICKSTART.md`](docs/QUICKSTART.md) - focused setup flow.
- [`docs/INVENTORY.md`](docs/INVENTORY.md) - component inventory for actions,
  CLI commands, env vars, and endpoints.
- [`docs/RUST.md`](docs/RUST.md) - Rust development notes.
- [`docs/stack/ARCH.md`](docs/stack/ARCH.md) - stack architecture details.
- [`server.json`](server.json) - MCP registry metadata.
- [`packages/gotify-rmcp/README.md`](packages/gotify-rmcp/README.md) - npm
  package launcher notes.

This README is curated. Generated or exhaustive catalogs should be refreshed in
their own files and treated as the source of truth for current branch details.

## License

MIT

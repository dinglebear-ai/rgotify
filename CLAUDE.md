# CLAUDE.md — rgotify

Rust MCP server bridging agents to a [Gotify](https://gotify.net/) push notification server. One MCP tool (`gotify`) with action dispatch, a CLI at full parity, and a REST HTTP client. No database, no background tasks.

## Repo Facts

| Fact | Value |
|------|-------|
| Remote | `git@github.com:dinglebear-ai/rgotify.git` |
| Default branch | `main` |
| Layout | **Single crate — not a cargo workspace** |
| Cargo package | `gotify-mcp` |
| Binary / CLI | `rgotify` (`[[bin]]`, `autobins = false`) |
| npm package | `gotify-rmcp` |
| MCP tool | `gotify` |
| Edition | 2021 |
| MSRV | 1.86 |
| Service port | **40020** (RMCP Streamable HTTP) |
| Config home | `~/.gotify` on hosts, `/data` in containers |

The repo, crate, binary, and npm package deliberately have four different names. Don't "fix" one to match another.

### rmcp version

`Cargo.toml` **declares** `rmcp = "1.6.0"`, but the caret range resolves to **1.7.0** in `Cargo.lock`. The lock is the truth — when checking API compatibility, read 1.7.0 docs, not 1.6.0. The declared version is not a real pin.

`lab-auth` is pinned by git rev to `dinglebear-ai/labby` (crate `lab-auth`, distinct from that workspace's own `labby-auth`).

## Commands

```bash
cargo check                      # typecheck
cargo clippy --all-targets -- -D warnings   # lint — must pass before committing
cargo fmt                        # format — enforced
cargo test                       # test suite
cargo build --release            # release build

cargo run                        # start HTTP MCP server (port 40020)
cargo run -- serve               # same, explicit
cargo run -- mcp                 # start stdio MCP transport
cargo run -- health              # run CLI health command
cargo run -- doctor --json       # operator diagnostics

just validate-plugin             # validate plugin manifests / MCP config / skills
```

## Module Map

| File | Role |
|------|------|
| `src/main.rs` | Mode dispatch: `serve_mcp` / `serve_stdio_mcp` / `run_cli`; `validate_bind_security` |
| `src/lib.rs` | Library root; also the `testing` helpers behind `feature = "test-support"` |
| `src/config.rs` | `Config` — loads `config.toml`, then env overrides; `load_dotenv`, `default_data_dir` |
| `src/gotify.rs` | `GotifyClient` — HTTP REST client, `X-Gotify-Key` header |
| `src/app.rs` | `GotifyService` — business logic, destructive gate |
| `src/mcp.rs` | Axum router, auth middleware, RMCP adapter, `AppState` / `AuthPolicy` |
| `src/mcp/tools.rs` | MCP shim — parse JSON args → call service → return `Value` |
| `src/mcp/routes.rs` | HTTP route handlers (`/mcp`, `/health`) |
| `src/mcp/rmcp_server.rs` | RMCP transport wiring |
| `src/mcp/schemas.rs` | JSON Schema for the `gotify` tool |
| `src/mcp/prompts.rs` | MCP prompts |
| `src/cli/mod.rs` | CLI shim — parse argv → call service → format/print |
| `src/cli/setup.rs` | `setup check` / `repair` / `install` / `plugin-hook` |
| `src/cli/doctor.rs` | `doctor` diagnostics |
| `src/observability.rs` | Runtime counters behind `action=status` |
| `src/token_limit.rs` | Response size guard |
| `src/logging.rs`, `src/logging/{file,aurora}.rs` | Tracing setup, file sink, Aurora-themed output |

Note: `src/cli/` is declared in `main.rs`, not `lib.rs` — it is binary-only and not part of the library API.

## Architecture Pattern

```
CLI args / MCP JSON args
        ↓
  thin shim (cli/mod.rs or mcp/tools.rs)   ← parse only, no logic
        ↓
  GotifyService (app.rs)                   ← all business logic, destructive gate
        ↓
  GotifyClient (gotify.rs)                 ← HTTP calls to Gotify REST API
        ↓
  Gotify server (X-Gotify-Key header)
```

CLI and MCP are intentionally identical thin shims. All logic lives in `GotifyService`.

## Adding a New Action (4 steps)

1. **`gotify.rs`** — add a method to `GotifyClient` calling the appropriate REST endpoint with the right token.
2. **`app.rs`** — add a method to `GotifyService` that calls the client method (add `destructive_gate` if needed).
3. **`mcp/tools.rs`** — add a match arm in `dispatch()` that calls the service method.
4. **`cli/mod.rs`** — add a match arm or subcommand that calls the service method and formats output.

Then update `docs/INVENTORY.md`, `README.md`, this file, and `CHANGELOG.md` (`[Unreleased]`).

## Token Model (Critical)

Two tokens, two purposes — never mix them:

| Token | Env var | Used for |
|-------|---------|----------|
| Client token (starts `C`) | `GOTIFY_CLIENT_TOKEN` | GET/PUT/DELETE — management: list, create, delete apps/clients/messages, read user |
| App token (starts `A`) | `GOTIFY_APP_TOKEN` | POST /message only — sending notifications |

`send_message` in `gotify.rs` uses `self.app_token` explicitly. All other requests use `self.client_token`. If the wrong token is used, Gotify returns 401.

## Environment Variables

### Upstream Gotify

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `GOTIFY_URL` | yes | — | Gotify server base URL |
| `GOTIFY_CLIENT_TOKEN` | for management | — | Client token |
| `GOTIFY_APP_TOKEN` | for `send` | — | App token |
| `GOTIFY_ALLOW_DESTRUCTIVE` | no | `false` | Skip confirm gate |

### MCP HTTP server

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `GOTIFY_MCP_HOST` | no | `0.0.0.0` | MCP bind host |
| `GOTIFY_MCP_PORT` | no | `40020` | MCP bind port |
| `GOTIFY_MCP_TOKEN` | no | — | Static bearer token for MCP auth |
| `GOTIFY_MCP_NO_AUTH` | no | `false` | Disable MCP auth layer |
| `GOTIFY_MCP_ALLOWED_HOSTS` | no | empty | Comma-separated Host allowlist |
| `GOTIFY_MCP_ALLOWED_ORIGINS` | no | empty | Comma-separated Origin allowlist |
| `GOTIFY_NOAUTH` | no | `false` | **Escape hatch.** Permits binding a non-loopback host with no auth (see below) |

### OAuth (`lab-auth`)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `GOTIFY_MCP_AUTH_MODE` | no | `bearer` | Set to `oauth` to enable Google OAuth |
| `GOTIFY_MCP_PUBLIC_URL` | for oauth | — | Public base URL for OAuth issuer + resource metadata |
| `GOTIFY_MCP_GOOGLE_CLIENT_ID` | for oauth | — | Google OAuth client ID |
| `GOTIFY_MCP_GOOGLE_CLIENT_SECRET` | for oauth | — | Google OAuth client secret |
| `GOTIFY_MCP_AUTH_ADMIN_EMAIL` | for oauth | — | Bootstrap allowlisted account |
| `GOTIFY_MCP_AUTH_SQLITE_PATH` | no | `<data>/auth.db` | OAuth state DB |
| `GOTIFY_MCP_AUTH_KEY_PATH` | no | `<data>/auth-jwt.pem` | JWT signing key |

MCP scopes are `gotify:read` and `gotify:write`. `GOTIFY_MCP` is the `lab-auth` env prefix, so `lab-auth` reads additional `GOTIFY_MCP_*` keys beyond those listed.

### Misc

| Variable | Default | Description |
|----------|---------|-------------|
| `GOTIFY_MCP_HOME` | `~/.gotify` or `/data` | Override the appdata dir used by `setup` |
| `RUNNING_IN_CONTAINER` | unset | Forces the `/data` appdata path |
| `RUST_LOG` | `info` | Log filter. Stdio mode must keep logs off stdout |

## Bind Security Gate

`validate_bind_security` (`main.rs`) refuses to start the HTTP server when **all** of these hold: the bind host is not loopback, no auth is configured (no `GOTIFY_MCP_TOKEN`, or `GOTIFY_MCP_NO_AUTH=true`, and not OAuth mode), and `GOTIFY_NOAUTH` is not truthy. Only set `GOTIFY_NOAUTH=true` when an upstream gateway genuinely enforces auth.

## Destructive Gate

`GotifyService::destructive_gate(confirm)` returns an error unless `confirm == true` or `self.allow_destructive == true`. Every destructive method checks it before calling the client. The error message is: `"destructive operation — pass confirm=true or set GOTIFY_ALLOW_DESTRUCTIVE=true"`. Never bypass without explicit user intent.

## Ports

| Port | Purpose |
|------|---------|
| 40020 | RMCP Streamable HTTP (`POST /mcp`, `GET /health`) |

## MCP Tool: `gotify` Actions

**Read:** `health`, `version`, `me`, `messages`, `applications`, `clients`, `status`
**Write:** `send`, `create_application`, `update_application`, `create_client`
**Destructive:** `delete_message`, `delete_all_messages`, `delete_application`, `delete_client`
**Meta:** `help`

Prompts: `send_notification`, `check_status`. Resource: `gotify://schema/mcp-tool`.

## CLI ↔ MCP Action Parity

Every MCP action has a CLI equivalent (and vice versa), except where noted. Both shims call the same `GotifyService` method.

| Service Method | MCP Action | CLI Command |
|---|---|---|
| `service.health()` | `gotify(action="health")` | `rgotify health` |
| `service.version()` | `gotify(action="version")` | `rgotify version` |
| `service.me()` | `gotify(action="me")` | `rgotify me` |
| `service.messages(app_id, limit, since)` | `gotify(action="messages", app_id=N, limit=N, since=N)` | `rgotify messages [--app-id N] [--limit N] [--since N]` |
| `service.applications()` | `gotify(action="applications")` | `rgotify applications` |
| `service.clients()` | `gotify(action="clients")` | `rgotify clients` |
| `service.send(msg, title, priority, extras)` | `gotify(action="send", message="...", title="...", priority=N)` | `rgotify send <message> [--title T] [--priority N]` |
| `service.create_application(name, desc, pri)` | `gotify(action="create_application", name="...", description="...", default_priority=N)` | `rgotify create app <name> [--description D] [--priority N]` |
| `service.update_application(id, name, desc, pri)` | `gotify(action="update_application", app_id=N, name="...", description="...", default_priority=N)` | `rgotify update app <id> [--name N] [--description D] [--priority N]` |
| `service.create_client(name)` | `gotify(action="create_client", name="...")` | `rgotify create client <name>` |
| `service.delete_message(id, confirm)` | `gotify(action="delete_message", id=N, confirm=true)` | `rgotify delete message <id> [--confirm]` |
| `service.delete_all_messages(confirm)` | `gotify(action="delete_all_messages", confirm=true)` | `rgotify delete all [--confirm]` |
| `service.delete_application(id, confirm)` | `gotify(action="delete_application", app_id=N, confirm=true)` | `rgotify delete app <id> [--confirm]` |
| `service.delete_client(id, confirm)` | `gotify(action="delete_client", client_id=N, confirm=true)` | `rgotify delete client <id> [--confirm]` |
| _(MCP-only)_ | `gotify(action="help")` | `rgotify --help` |
| _(MCP-only)_ | `gotify(action="status")` | `rgotify doctor --json` (nearest operator equivalent) |

CLI subcommands with no MCP counterpart: `serve`, `serve mcp`, `mcp`, `doctor`, `setup {check,repair,install,plugin-hook}`.

## Plugin Setup — Manual, No Hooks

**This plugin ships no Claude Code hooks.** `plugins/gotify/hooks/hooks.json` (a `SessionStart` hook plus a `ConfigChange`/`user_settings` hook, both running `rgotify setup plugin-hook`) was removed, along with the `plugins/gotify/scripts/plugin-setup.sh` wrapper.

What that automation used to do, every session start and on every user-settings change:

1. Map `CLAUDE_PLUGIN_OPTION_*` values into the `GOTIFY_*` env vars the setup checks read (`apply_plugin_options()` in `src/cli/setup.rs`).
2. Self-install the running binary into `~/.local/bin/rgotify` so it stayed callable from a plain terminal and survived `/plugin update` (`install_self()`).
3. Run the setup checks (`appdata_dir`, `env_file`, `binary`, `mcp_port`) and auto-`repair` — creating `~/.gotify/` and a placeholder `.env` — when a blocking check failed.

**Manual fallback.** The `setup` subcommands still exist and are unchanged; only the automatic invocation is gone. None of these are required to configure the server — run them when you want the appdata bootstrap, the `~/.local/bin` refresh, or a preflight check:

```bash
rgotify setup repair     # create ~/.gotify + .env, then re-check
rgotify setup install    # copy the binary into ~/.local/bin
rgotify setup check      # read-only verification
```

`rgotify setup plugin-hook [--no-repair]` also still works if you want the exact former behavior in one shot. Note the server itself receives its config from `.mcp.json`'s `${user_config.*}` block, not from the hook's process env — so the practical loss is the `~/.gotify` bootstrap, the `~/.local/bin` refresh, and the port/binary preflight warnings, not server configuration.

Do not add Docker Compose, systemd, or service-bootstrap logic into the `setup` path.

## Key Files

| File | Purpose |
|------|---------|
| `Cargo.toml` / `Cargo.lock` | Crate metadata, dependencies (lock is the version truth) |
| `config.toml` | Local dev config (not committed with secrets) |
| `.env.example` | Documented env template |
| `server.json` | MCP registry metadata (`ai.dinglebear/gotify-rmcp`) |
| `Justfile` | Task runner — build, validate, sync-bin, release helpers |
| `install.sh` / `scripts/install.sh` | Host installer |
| `scripts/validate-plugin-layout.sh` | Plugin manifest / MCP config / skills validator |
| `plugins/gotify/` | Claude Code + Codex plugin (manifests, `.mcp.json`, skill) |
| `packages/gotify-rmcp/` | npm launcher and release-binary downloader |
| `README.md` | User-facing overview, quickstart, full reference |
| `docs/QUICKSTART.md` | 5-minute setup guide |
| `docs/INVENTORY.md` | Complete action/command/env inventory |
| `docs/RUST.md` | Rust development notes |
| `docs/stack/ARCH.md` | Architecture diagrams |
| `docs/stack/TECH.md` | Technology choices and rationale |

`docs/references/` is a large vendored corpus of upstream Gotify and MCP spec docs. It is reference material, not this project's documentation — don't grep it when looking for repo facts.

## Agent Memory Files

`AGENTS.md` and `GEMINI.md` are symlinks to this file. Never edit them directly — edit `CLAUDE.md`. If a symlink is missing:

```bash
ln -sf CLAUDE.md AGENTS.md
ln -sf CLAUDE.md GEMINI.md
```

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->

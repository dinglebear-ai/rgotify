# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-06-01

### Changed

- Plugin `SessionStart`/`ConfigChange` hooks now call `${CLAUDE_PLUGIN_ROOT}/bin/rgotify setup plugin-hook` directly instead of going through the `plugin-setup.sh` shell wrapper. The env-var mapping the script performed (`CLAUDE_PLUGIN_OPTION_*` → `GOTIFY_*`, plus `CLAUDE_PLUGIN_DATA` → `GOTIFY_MCP_HOME`) now lives in `apply_plugin_options()` in `src/cli/setup.rs`, applied at the top of the plugin-hook path. The script's `.env`-fallback was dropped (immaterial: the binary never persists option values to `.env` and the setup checks read live process env).

### Removed

- `plugins/gotify/hooks/plugin-setup.sh` — the wrapper was a pure env-mapping middleman now handled by the binary's `setup plugin-hook` command.

## [0.1.0] - 2026-05-13

### Added

- Initial release of `gotify-mcp`: Rust MCP server bridging Claude to a Gotify push notification server.
- MCP tool `gotify` with action dispatch: `health`, `version`, `me`, `messages`, `applications`, `clients`, `send`, `create_application`, `update_application`, `create_client`, `delete_message`, `delete_all_messages`, `delete_application`, `delete_client`, `help`.
- Destructive operation safety gate: requires `confirm=true` or `GOTIFY_ALLOW_DESTRUCTIVE=true`.
- RMCP Streamable HTTP transport on port 9158.
- stdio MCP transport for local child-process clients.
- CLI with matching commands for all actions.
- Dual-token model: `GOTIFY_CLIENT_TOKEN` for management, `GOTIFY_APP_TOKEN` for sending.

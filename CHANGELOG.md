# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-05-13

### Added

- Initial release of `gotify-mcp`: Rust MCP server bridging Claude to a Gotify push notification server.
- MCP tool `gotify` with action dispatch: `health`, `version`, `me`, `messages`, `applications`, `clients`, `send`, `create_application`, `update_application`, `create_client`, `delete_message`, `delete_all_messages`, `delete_application`, `delete_client`, `help`.
- Destructive operation safety gate: requires `confirm=true` or `GOTIFY_ALLOW_DESTRUCTIVE=true`.
- RMCP Streamable HTTP transport on port 9158.
- stdio MCP transport for local child-process clients.
- CLI with matching commands for all actions.
- Dual-token model: `GOTIFY_CLIENT_TOKEN` for management, `GOTIFY_APP_TOKEN` for sending.

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0](https://github.com/dinglebear-ai/rgotify/compare/v0.1.2...v0.2.0) (2026-08-05)


### Added

* **release:** publish canonical MCP Registry metadata ([#20](https://github.com/dinglebear-ai/rgotify/issues/20)) ([0f5abc0](https://github.com/dinglebear-ai/rgotify/commit/0f5abc02c341fc386a5172353f2df2a479cbbf3b))


### Fixed

* **ci:** preserve existing Kache config ([#19](https://github.com/dinglebear-ai/rgotify/issues/19)) ([30afba9](https://github.com/dinglebear-ai/rgotify/commit/30afba9dfd6fe9a0db3ac2e5d49b672772f0a78b))
* **compose:** bind MCP to trusted interfaces ([#22](https://github.com/dinglebear-ai/rgotify/issues/22)) ([f705529](https://github.com/dinglebear-ai/rgotify/commit/f70552945677077402df04513f98e182e6531e57))
* **mcp:** use DNS-only Registry publisher ([#21](https://github.com/dinglebear-ai/rgotify/issues/21)) ([c77b440](https://github.com/dinglebear-ai/rgotify/commit/c77b4402f2fbfb872048adec667c7a969177ea0a))
* publish npm launcher as @dinglebear/rgotify ([a73f7e1](https://github.com/dinglebear-ai/rgotify/commit/a73f7e1f7716e90643d4580ace32cef3e1064c45))
* **release:** sync launcher binary version ([#25](https://github.com/dinglebear-ai/rgotify/issues/25)) ([6ebdab4](https://github.com/dinglebear-ai/rgotify/commit/6ebdab439573318df808476ad37011d3a774df88))
* **release:** update workspace-owned version ([#23](https://github.com/dinglebear-ai/rgotify/issues/23)) ([922b34f](https://github.com/dinglebear-ai/rgotify/commit/922b34fc7c9a76986b9e8d24d8b27d67575b84bf))
* route rust builds through sccache wrapper ([1b01ba0](https://github.com/dinglebear-ai/rgotify/commit/1b01ba0343043777f71105235d1cb16524451e84))
* **version:** resync server.json/package.json to the released 0.1.2 ([#10](https://github.com/dinglebear-ai/rgotify/issues/10)) ([2dd84ee](https://github.com/dinglebear-ai/rgotify/commit/2dd84ee96974381a7fe206bc5b87a8c0d88adb7b))

## [0.1.2](https://github.com/jmagar/rgotify/compare/v0.1.1...v0.1.2) (2026-07-08)


### Fixed

* **ci:** switch OpenWiki to local openai-compatible proxy ([e7591aa](https://github.com/jmagar/rgotify/commit/e7591aafefec7b9bbf75e198b5b0cf2a86bb23a3))

## [Unreleased]

### Fixed

- Keep the npm launcher `binaryVersion` synchronized with the release tag so it downloads the matching native binary.
- Update the workspace-owned Cargo version explicitly during release-please runs instead of treating the inherited package version as directly tagged.

### Changed

- Pin the shared Rust cache action to Kache 0.13.0 so hosted and self-hosted jobs use the same stabilized daemon protocol.
- Bind the production MCP port only to DEVHOST's Tailscale and LAN addresses instead of every host interface.

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

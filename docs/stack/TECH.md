# Technology Choices — gotify-rmcp

Technology stack reference and rationale.

## Language: Rust

- Single static binary — no runtime to install on the server.
- Memory safety without GC — no unexpected pauses in the HTTP path.
- Strong typing catches API contract violations at compile time.
- `reqwest` provides an ergonomic async HTTP client backed by `rustls`.

## Async runtime: tokio

Full-featured async runtime. The `full` feature enables:
- `tokio::net` — TCP listener for the HTTP server.
- `tokio::signal` — graceful shutdown on SIGTERM/CTRL-C.
- `tokio::task` — async task spawning for the RMCP service.

## HTTP framework: axum

Minimal, composable HTTP framework:
- Type-safe state extraction via `State<AppState>`.
- Composable router for `POST /mcp` and `GET /health`.
- Native tower middleware support (CORS, tracing via `tower-http`).
- Mounts the RMCP service as a tower-compatible handler.

## MCP SDK: rmcp

RMCP owns the MCP protocol lifecycle:
- Streamable HTTP framing (stateless JSON-response mode).
- stdio transport for local child-process clients.
- Host/Origin validation.
- Tool listing and tool call routing.

`gotify-rmcp` uses stateless JSON-response mode: `POST /mcp` returns `Content-Type: application/json`. No SSE streams.

## HTTP client: reqwest

Async HTTP client for Gotify REST API calls:
- `json` feature for automatic JSON serialization/deserialization.
- `rustls-tls` feature for TLS without depending on OpenSSL.
- Custom header support for `X-Gotify-Key` authentication.

This replaces any GraphQL or database driver — Gotify is a pure REST API. No SQL, no ORM, no connection pool.

## Serialization: serde + serde_json + toml

- `serde` — derive macros for all config and model structs.
- `serde_json` — MCP tool argument/result payloads; Gotify API request/response bodies.
- `toml` — `config.toml` parsing for local development.

## Time: chrono

RFC 3339 timestamp parsing and formatting. Used for:
- `since` parameter in message queries.
- Timestamp fields in Gotify API responses.

## Auth: lab-auth

OAuth/JWT auth library from the homelab monorepo. Used for:
- Bearer token validation in the MCP HTTP server.
- Optional OAuth mode (reserved for future use).

Static bearer token comparison uses constant-time equality to prevent timing attacks.

## Logging: tracing + tracing-subscriber

Structured, span-based logging:
- `RUST_LOG` directive for level control.
- Human-readable console output with timestamps.
- Stderr output only — stdout is reserved for stdio MCP protocol messages.

## Error handling: anyhow

`anyhow::Result` throughout application code:
- Config loading errors.
- HTTP client errors.
- Tool execution errors propagated with `?`.

## Runtime dependencies summary

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `axum` | HTTP framework |
| `rmcp` | MCP SDK (Streamable HTTP + stdio) |
| `tower-http` | CORS and tracing middleware |
| `reqwest` | Gotify REST API client |
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

## See also

- [ARCH.md](ARCH.md) — architecture overview
- [../INVENTORY.md](../INVENTORY.md) — complete dependency listing

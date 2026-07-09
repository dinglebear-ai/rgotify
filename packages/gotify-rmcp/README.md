# gotify-rmcp

Node launcher for the `rgotify` Rust MCP server and CLI binary.

```bash
npx -y gotify-rmcp --help
```

The package downloads the matching GitHub Release binary during `postinstall`.

## MCP stdio

Use the package directly as an MCP command:

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

## Environment

- `GOTIFY_RMCP_BINARY_VERSION`: release tag/version to download, defaulting to this npm package version.
- `GOTIFY_RMCP_VERSION`: alias for `GOTIFY_RMCP_BINARY_VERSION`.
- `GOTIFY_RMCP_REPO`: GitHub `owner/repo`, defaulting to `jmagar/gotify-rmcp`.
- `GOTIFY_RMCP_RELEASE_BASE_URL`: full release download base URL.
- `GOTIFY_RMCP_SKIP_DOWNLOAD=1`: skip postinstall download.

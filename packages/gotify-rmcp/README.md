# gotify-rmcp

Node launcher for the `gotify-rmcp` Rust MCP server and `rgotify` CLI binary.

```bash
npx -y gotify-rmcp --help
npx -y gotify-rmcp health --json
npx -y gotify-rmcp mcp
```

The package downloads the matching `rgotify` release binary during
`postinstall`. It exposes two npm binary aliases:

| Alias | Runs |
|---|---|
| `gotify-rmcp` | bundled `rgotify` binary |
| `rgotify` | bundled `rgotify` binary |

## MCP Client Example

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

## Launcher Controls

| Variable | Purpose |
|---|---|
| `GOTIFY_RMCP_SKIP_DOWNLOAD=1` | Skip binary download during `postinstall`. |
| `GOTIFY_RMCP_VERSION` or `GOTIFY_RMCP_BINARY_VERSION` | Override the release tag used for downloads. |
| `GOTIFY_RMCP_REPO` | Override the GitHub repo used for downloads. |
| `GOTIFY_RMCP_RELEASE_BASE_URL` | Override the release download base URL. |

Full documentation lives in the repository README:
https://github.com/jmagar/gotify-rmcp#readme

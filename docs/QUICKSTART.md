# Quickstart — gotify-rmcp

Get Claude sending push notifications to your Gotify server in 5 minutes.

## Prerequisites

- A running [Gotify](https://gotify.net/) server
- Rust 1.86+ (`rustup update stable`)

## Step 1: Get your tokens from Gotify

You need two tokens. Create them in the Gotify web UI:

**Client token** — for management (list/create/delete):
1. Log in to Gotify → click your username → "Clients"
2. Create a new client, e.g. `claude`
3. Copy the token — it starts with `C`

**App token** — for sending notifications:
1. Go to "Apps" → "Create Application"
2. Give it a name, e.g. `claude-notifications`
3. Copy the token — it starts with `A`

## Step 2: Build

```bash
git clone https://github.com/jmagar/rgotify
cd gotify-rmcp
cargo build --release
```

The binary is at `./target/release/rgotify`.

## Step 3: Configure environment

```bash
export GOTIFY_URL=https://gotify.example.com
export GOTIFY_CLIENT_TOKEN=Cxxxxxxxxxxxxxxxx   # client token from step 1
export GOTIFY_APP_TOKEN=Axxxxxxxxxxxxxxxx      # app token from step 1
export GOTIFY_MCP_TOKEN=$(openssl rand -hex 32)  # bearer token for MCP auth
echo "MCP token: $GOTIFY_MCP_TOKEN"
```

## Step 4: Start the server

```bash
./target/release/rgotify
# -> gotify-rmcp starting on 0.0.0.0:40020
```

## Step 5: Verify

```bash
# Health check (no auth needed)
curl -sf http://localhost:40020/health
# -> {"status":"ok"}

# Send a test notification
curl -s -X POST http://localhost:40020/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Authorization: Bearer $GOTIFY_MCP_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "gotify",
      "arguments": {
        "action": "send",
        "message": "Hello from gotify-rmcp!",
        "title": "Test"
      }
    }
  }'
```

Check your Gotify server — the notification should appear.

## Step 6: Connect Claude Code

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gotify": {
      "type": "http",
      "url": "http://localhost:40020/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_MCP_TOKEN_HERE"
      }
    }
  }
}
```

Or use stdio mode (no separate server process needed):

```json
{
  "mcpServers": {
    "gotify": {
      "command": "/path/to/rgotify",
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

Restart Claude Code. Ask Claude: "Send a Gotify notification saying the deployment is complete."

## Next steps

- See [../README.md](../README.md) for the full MCP tool and CLI reference.
- See [../docs/INVENTORY.md](INVENTORY.md) for every action, parameter, and env var.
- See [../docs/stack/ARCH.md](stack/ARCH.md) for architecture details.

---
name: gotify
description: |
  How to interact with the Gotify self-hosted push notification server via the gotify-mcp MCP tool, CLI binary, or direct REST API. Use this skill whenever the user wants to send a push notification, check notification history, manage Gotify applications or clients, check Gotify server health, delete messages, or do anything with Gotify — even if they just say "send an alert" or "notify me when X is done". Also invoke this when the user asks about notification history, wants to know if their Gotify server is up, or mentions monitoring, alerting, or push notifications in a self-hosted context.
---

# Gotify — Push Notification Server

Gotify is a self-hosted push notification server. The gotify-mcp bridge exposes it as a single MCP tool (`gotify`) with action-based dispatch. A CLI binary and direct REST API are available as fallbacks.

## Token model (critical — read this first)

Two tokens, two purposes. Using the wrong token returns 401.

| Token | Env var | Starts with | Used for |
|-------|---------|-------------|----------|
| Client token | `GOTIFY_CLIENT_TOKEN` | `C` | All management: list/create/delete apps, clients, messages; read user |
| App token | `GOTIFY_APP_TOKEN` | `A` | **Sending only** — `POST /message` |

`health` and `version` need no token at all.

---

## Tier 1 — MCP tool `gotify` (preferred)

Single tool, `action` selects the operation. All other parameters are optional unless noted.

### Read actions

| Action | Parameters | Notes |
|--------|-----------|-------|
| `health` | — | No auth required |
| `version` | — | No auth required |
| `me` | — | Returns current user info |
| `messages` | `app_id?`, `limit?` (default 50), `since?` | `since` is a message-ID cursor for pagination |
| `applications` | — | Returns array with id, name, defaultPriority, token |
| `clients` | — | Returns array with id, name, token |

### Write actions

| Action | Required | Optional |
|--------|----------|---------|
| `send` | `message` | `title`, `priority` (0–10), `extras` (object) |
| `create_application` | `name` | `description`, `default_priority` |
| `update_application` | `app_id` | `name`, `description`, `default_priority` |
| `create_client` | `name` | — |

### Destructive actions — require `confirm=true`

These will fail unless `confirm=true` is passed **or** `GOTIFY_ALLOW_DESTRUCTIVE=true` is set in the environment. Always confirm with the user before passing `confirm=true`, especially for `delete_all_messages`.

| Action | Required | What it does |
|--------|----------|-------------|
| `delete_message` | `id`, `confirm=true` | Deletes one message |
| `delete_all_messages` | `confirm=true` | Wipes **all** messages — irreversible |
| `delete_application` | `app_id`, `confirm=true` | Deletes application and its token |
| `delete_client` | `client_id`, `confirm=true` | Deletes client and its token |

### Meta

- `help` — Returns full built-in documentation.

### Examples

```python
# Send an urgent alert
gotify(action="send", message="Disk usage at 95%", title="ALERT", priority=8)

# Send with rich extras (markdown content)
gotify(action="send", message="Build failed", title="CI", priority=5,
       extras={"client::display": {"contentType": "text/markdown"}})

# Check server health
gotify(action="health")

# List last 20 messages
gotify(action="messages", limit=20)

# Messages from a specific app only
gotify(action="messages", app_id=3, limit=10)

# Paginate: messages after ID 100
gotify(action="messages", since=100)

# Discover apps and their IDs
gotify(action="applications")

# Who am I?
gotify(action="me")

# Create a monitoring application
gotify(action="create_application", name="Monitoring", description="System alerts", default_priority=5)

# Update an app's default priority
gotify(action="update_application", app_id=2, default_priority=8)

# Delete a specific message (user confirmed)
gotify(action="delete_message", id=42, confirm=True)

# Wipe all messages (user explicitly confirmed)
gotify(action="delete_all_messages", confirm=True)

# Delete an app
gotify(action="delete_application", app_id=3, confirm=True)

# Create a client token for a new subscriber
gotify(action="create_client", name="HomeAssistant")

# Delete a client
gotify(action="delete_client", client_id=2, confirm=True)
```

---

## Tier 2 — CLI binary (secondary)

Binary: `~/workspace/rustify/target/release/rgotify` (or `/usr/local/bin/rgotify` in Docker)

Add `--json` to any command for raw JSON output. Add `--confirm` to destructive commands.

```bash
# Server info
rgotify health
rgotify version
rgotify me

# Messages
rgotify messages                           # last 50
rgotify messages --limit 20
rgotify messages --app-id 3               # filter by app
rgotify messages --since 100              # cursor pagination
rgotify messages --json                   # raw JSON

# Applications
rgotify applications

# Send a notification
rgotify send "Server rebooted" --title "Alert" --priority 8

# Create / delete apps
rgotify create app "Monitoring" --description "System alerts" --priority 5
rgotify create-app "Monitoring"           # short alias
rgotify delete app 3 --confirm
rgotify delete-app 3 --confirm            # short alias

# Delete messages
rgotify delete message 42 --confirm
rgotify delete-message 42 --confirm       # short alias
rgotify delete all --confirm              # wipe everything
rgotify delete-all --confirm              # short alias

# Clients
rgotify clients
rgotify create client "MyApp"
rgotify create-client "MyApp"             # short alias
rgotify delete client 2 --confirm
rgotify delete-client 2 --confirm         # short alias
```

---

## Tier 3 — Direct REST API (fallback)

Use when neither the MCP tool nor the CLI binary is available.

Auth header: `X-Gotify-Key: <token>`

```bash
# No-auth endpoints
curl "$GOTIFY_URL/health"
curl "$GOTIFY_URL/version"

# Send a notification — uses APP token
curl -X POST "$GOTIFY_URL/message" \
  -H "X-Gotify-Key: $GOTIFY_APP_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message":"Server rebooted","title":"Alert","priority":5}'

# List messages — uses CLIENT token
curl "$GOTIFY_URL/message?limit=20" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN"

# Messages for a specific app
curl "$GOTIFY_URL/application/3/message?limit=10" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN"

# List applications
curl "$GOTIFY_URL/application" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN"

# List clients
curl "$GOTIFY_URL/client" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN"

# Current user
curl "$GOTIFY_URL/current/user" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN"

# Create application
curl -X POST "$GOTIFY_URL/application" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Monitoring","description":"System alerts","defaultPriority":5}'

# Update application
curl -X PUT "$GOTIFY_URL/application/2" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"defaultPriority":8}'

# Delete one message
curl -X DELETE "$GOTIFY_URL/message/42" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN"

# Delete all messages
curl -X DELETE "$GOTIFY_URL/message" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN"

# Delete application
curl -X DELETE "$GOTIFY_URL/application/3" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN"

# Create client
curl -X POST "$GOTIFY_URL/client" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"HomeAssistant"}'

# Delete client
curl -X DELETE "$GOTIFY_URL/client/2" \
  -H "X-Gotify-Key: $GOTIFY_CLIENT_TOKEN"
```

---

## Gotchas

1. **Wrong token = 401.** `send` uses `GOTIFY_APP_TOKEN`. Everything else uses `GOTIFY_CLIENT_TOKEN`. Do not mix them.

2. **Destructive gate.** Delete operations require `confirm=True` (MCP) or `--confirm` (CLI) **unless** `GOTIFY_ALLOW_DESTRUCTIVE=true` is set. The error message when missing is: `"destructive operation — pass confirm=true or set GOTIFY_ALLOW_DESTRUCTIVE=true"`.

3. **Priority scale.** 0 = silent/low, 5 = normal, 8+ = high/urgent. The Gotify Android client shows heads-up notifications at priority ≥ 8. Default is whatever the application's `defaultPriority` is set to.

4. **`delete_all_messages` is irreversible.** Always confirm with the user before passing `confirm=True`.

5. **`extras` for rich content.** Pass an object like `{"client::display": {"contentType": "text/markdown"}}` to render markdown in compatible clients. See [Gotify extras docs](https://gotify.net/docs/msgextras) for the full spec.

6. **Pagination.** `messages` returns up to `limit` items. The response includes a `paging.next` cursor — pass it as `since` to fetch the next page.

7. **MCP server port.** Runs on `9158` by default (`GOTIFY_MCP_PORT` to override).

8. **No-auth endpoints.** `health` and `version` work without any token — useful for liveness probes.

9. **`update_application` is a PATCH-style PUT.** Only fields you include are updated; omitted fields keep their current values.

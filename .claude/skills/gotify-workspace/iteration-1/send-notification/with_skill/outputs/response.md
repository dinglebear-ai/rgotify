# Response: Send Push Notification

I'll send the notification using the `gotify` MCP tool with `action=send`.

## Tool call

```python
gotify(
    action="send",
    message="Backup completed successfully",
    title="Backup",
    priority=5
)
```

**Note on tokens:** `send` uses `GOTIFY_APP_TOKEN` (the app token starting with `A`), not the client token. This is handled automatically by the MCP server.

## Expected response

```json
{
  "id": 42,
  "appid": 1,
  "message": "Backup completed successfully",
  "title": "Backup",
  "priority": 5,
  "date": "2026-05-13T12:00:00Z"
}
```

The response includes the message ID (here `42`) which you can use later to delete this specific message if needed.

**Priority 5** is normal/medium priority — your Gotify clients will display it as a standard notification without a heads-up interrupt. For urgent alerts, use priority 8+.

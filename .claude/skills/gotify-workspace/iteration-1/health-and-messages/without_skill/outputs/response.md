# Response (baseline — no skill): Check Health and Messages

To check your Gotify server health:

```bash
curl http://your-gotify-server/health
```

This should return something like `{"database":"green","health":"green"}`.

To get the last 10 messages, you'll need your client token:

```bash
curl "http://your-gotify-server/message?limit=10" \
  -H "X-Gotify-Key: YOUR_CLIENT_TOKEN"
```

If you have the `gotify` MCP tool configured, I might be able to call it directly. Do you have that set up, or would you prefer I guide you through the REST API?

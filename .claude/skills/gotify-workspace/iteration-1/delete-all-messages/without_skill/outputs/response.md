# Response (baseline — no skill): Clean Up Gotify

Sure! Let me help you clean up your Gotify server.

First, to see all your applications:

```bash
curl "http://your-gotify-server/application" \
  -H "X-Gotify-Key: YOUR_CLIENT_TOKEN"
```

Then to delete all messages:

```bash
curl -X DELETE "http://your-gotify-server/message" \
  -H "X-Gotify-Key: YOUR_CLIENT_TOKEN"
```

This will delete all messages on your server. Note that this is permanent and cannot be undone.

If you have an MCP tool configured, I could try using that instead. What's your setup?

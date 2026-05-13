# Response (baseline — no skill): Send Push Notification

To send a push notification to your Gotify server, you'd use the Gotify REST API. Here's how:

```bash
curl -X POST "http://your-gotify-server/message" \
  -H "Content-Type: application/json" \
  -H "X-Gotify-Key: YOUR_APP_TOKEN" \
  -d '{
    "message": "Backup completed successfully",
    "title": "Backup",
    "priority": 5
  }'
```

Replace `your-gotify-server` with your server's address and `YOUR_APP_TOKEN` with your Gotify application token.

If you have the Gotify CLI installed, you can also do:
```bash
gotify push -t "Backup" -p 5 "Backup completed successfully"
```

Or if there's an MCP tool for Gotify available, I can try using that — let me know what you have set up.

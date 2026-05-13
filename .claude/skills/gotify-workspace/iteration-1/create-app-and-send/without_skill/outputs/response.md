# Response (baseline — no skill): Create App and Send Notification

To create a new Gotify application:

```bash
curl -X POST "http://your-gotify-server/application" \
  -H "X-Gotify-Key: YOUR_CLIENT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "HomeAutomation",
    "description": "Smart home events",
    "defaultPriority": 3
  }'
```

This will return a response with the new application's ID and token.

Then to send a test notification, use the app token from the response:

```bash
curl -X POST "http://your-gotify-server/message" \
  -H "X-Gotify-Key: APP_TOKEN_FROM_RESPONSE" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Test notification from HomeAutomation",
    "title": "HomeAutomation Test",
    "priority": 3
  }'
```

Note that you need a client token for creating apps, but an app token for sending messages — these are different tokens with different permissions.

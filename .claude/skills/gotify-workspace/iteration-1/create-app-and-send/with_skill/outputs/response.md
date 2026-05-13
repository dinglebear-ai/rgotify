# Response: Create Application and Send Test Notification

## Step 1: Create the application

```python
gotify(
    action="create_application",
    name="HomeAutomation",
    description="Smart home events",
    default_priority=3
)
```

This uses the **client token** (`GOTIFY_CLIENT_TOKEN`) for management. Expected response:
```json
{
  "id": 3,
  "name": "HomeAutomation",
  "description": "Smart home events",
  "defaultPriority": 3,
  "token": "A_newapptoken_xyz",
  "internal": false,
  "image": "static/defaultapp.png"
}
```

The response includes the new application's `id` (here `3`) and its **app token** (`token` field). That app token is what you'd use to send notifications on behalf of the HomeAutomation app specifically.

## Step 2: Send a test notification

For the test notification, I'll use the configured `GOTIFY_APP_TOKEN` (the app token from your environment):

```python
gotify(
    action="send",
    message="HomeAutomation app created — test notification",
    title="HomeAutomation Test",
    priority=3
)
```

**Important token note:** `send` always uses `GOTIFY_APP_TOKEN` from your environment. If you want to send specifically *as* the new HomeAutomation app (using its own token from step 1), you'd need to update `GOTIFY_APP_TOKEN` to the new token, or use the REST API directly with the new token. The MCP tool uses whichever token is configured in the environment.

## What this looks like end-to-end

1. Application "HomeAutomation" is created with default priority 3
2. A test notification fires with the same priority
3. Your Gotify clients will show it as a low-priority notification (priority 3 is below the heads-up threshold of ~8)

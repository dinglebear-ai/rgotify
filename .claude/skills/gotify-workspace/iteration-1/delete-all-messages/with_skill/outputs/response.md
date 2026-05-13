# Response: Clean Up Gotify Server

## Step 1: List all applications

```python
gotify(action="applications")
```

This shows all your apps and their IDs so you know what's there before making any changes.

Expected response:
```json
[
  {"id": 1, "name": "Monitoring", "description": "System alerts", "defaultPriority": 5},
  {"id": 2, "name": "HomeAutomation", "description": "Smart home events", "defaultPriority": 3}
]
```

## Step 2: Delete all messages — STOP

Before I do this, I need to flag something important:

**`delete_all_messages` is irreversible.** This wipes every message from your Gotify server permanently — there is no undo, no trash, no recovery. The prompt says "be careful about the delete", so I want to be explicit:

- All messages from ALL applications will be deleted
- This cannot be undone
- If you need any messages for logs or auditing, export/copy them first

Do you want me to proceed? If yes, I'll call:

```python
gotify(action="delete_all_messages", confirm=True)
```

Expected response: `{"status": "ok"}` (HTTP 204 from the Gotify server, translated to an ok status).

---

*(If you only want to delete messages from a specific app, use `delete_message` with specific IDs instead, or filter with `messages(app_id=N)` first to review them.)*

use serde_json::{Value, json};

pub(super) const GOTIFY_ACTIONS: &[&str] = &[
    "health",
    "version",
    "me",
    "messages",
    "applications",
    "clients",
    "status",
    "send",
    "create_application",
    "update_application",
    "delete_application",
    "create_client",
    "delete_client",
    "delete_message",
    "delete_all_messages",
    "help",
];

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![json!({
        "name": "gotify",
        "description": "Interact with a Gotify push notification server. Use action=help for documentation.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Operation to perform.",
                    "enum": GOTIFY_ACTIONS
                },
                "message": { "type": "string", "description": "Notification body (send)." },
                "title":   { "type": "string", "description": "Notification title (send)." },
                "priority":{ "type": "integer","description": "Priority 0-10 (send)." },
                "extras":  { "type": "object", "description": "Extra metadata (send)." },
                "name":    { "type": "string", "description": "Application or client name." },
                "description": { "type": "string", "description": "Application description." },
                "default_priority": { "type": "integer", "description": "Default priority for app." },
                "app_id":  { "type": "integer", "description": "Application ID." },
                "client_id":{ "type": "integer","description": "Client ID." },
                "id":      { "type": "integer", "description": "Message ID (delete_message)." },
                "limit":   { "type": "integer", "description": "Max messages to return (messages)." },
                "since":   { "type": "integer", "description": "Message ID cursor for pagination (messages)." },
                "confirm": { "type": "boolean", "description": "Required true for destructive operations." }
            },
            "required": ["action"]
        }
    })]
}

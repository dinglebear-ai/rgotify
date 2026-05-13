use serde_json::{json, Value};

use super::AppState;

/// Thin shim — parse args, call service, return Value. No logic here.
pub(super) async fn execute_tool(
    state: &AppState,
    name: &str,
    args: Value,
) -> anyhow::Result<Value> {
    state.service.counters.inc_request();
    let result = match name {
        "gotify" => dispatch(state, args).await,
        _ => Err(anyhow::anyhow!(
            "unknown tool: {name}\n\
             Hint: the only registered tool is 'gotify'. Use action=help for documentation."
        )),
    };
    if result.is_err() {
        state.service.counters.inc_error();
    }
    result
}

async fn dispatch(state: &AppState, args: Value) -> anyhow::Result<Value> {
    let action = string_arg(&args, "action").ok_or_else(|| {
        anyhow::anyhow!(
            "action is required — pass action=<name>\n\
             Valid actions: health, version, me, messages, applications, clients, \
             send, create_application, update_application, delete_application, \
             create_client, delete_client, delete_message, delete_all_messages, \
             status, help"
        )
    })?;

    match action.as_str() {
        // read
        "health" => state.service.health().await,
        "version" => state.service.version().await,
        "me" => state.service.me().await,
        "messages" => {
            state
                .service
                .messages(
                    i64_arg(&args, "app_id")?,
                    i64_arg(&args, "limit")?,
                    i64_arg(&args, "since")?,
                    i64_arg(&args, "offset")?,
                    string_arg(&args, "query").as_deref(),
                )
                .await
        }
        "applications" => {
            state
                .service
                .applications(string_arg(&args, "name").as_deref())
                .await
        }
        "clients" => state.service.clients().await,

        // write
        "send" => {
            let message = string_arg(&args, "message").ok_or_else(|| {
                anyhow::anyhow!(
                    "send: message is required — pass message=\"your text\"\n\
                     Example: action=send message=\"Hello!\" title=\"Alert\" priority=5"
                )
            })?;
            state
                .service
                .send(
                    &message,
                    string_arg(&args, "title").as_deref(),
                    i64_arg(&args, "priority")?,
                    args.get("extras").cloned(),
                )
                .await
        }
        "delete_message" => {
            let id = i64_arg(&args, "id")?.ok_or_else(|| {
                anyhow::anyhow!(
                    "delete_message: id is required — pass id=<message_id>\n\
                     Hint: use action=messages to list message IDs first.\n\
                     Also requires confirm=true or GOTIFY_ALLOW_DESTRUCTIVE=true."
                )
            })?;
            state
                .service
                .delete_message(id, bool_arg(&args, "confirm"))
                .await
        }
        "delete_all_messages" => {
            state
                .service
                .delete_all_messages(bool_arg(&args, "confirm"))
                .await
        }
        "create_application" => {
            let name = string_arg(&args, "name").ok_or_else(|| {
                anyhow::anyhow!(
                    "create_application: name is required — pass name=\"My App\"\n\
                     Optional: description=\"...\", default_priority=5"
                )
            })?;
            state
                .service
                .create_application(
                    &name,
                    string_arg(&args, "description").as_deref(),
                    i64_arg(&args, "default_priority")?,
                )
                .await
        }
        "update_application" => {
            let app_id = i64_arg(&args, "app_id")?.ok_or_else(|| {
                anyhow::anyhow!(
                    "update_application: app_id is required — pass app_id=<id>\n\
                     Hint: use action=applications to list application IDs."
                )
            })?;
            state
                .service
                .update_application(
                    app_id,
                    string_arg(&args, "name").as_deref(),
                    string_arg(&args, "description").as_deref(),
                    i64_arg(&args, "default_priority")?,
                )
                .await
        }
        "delete_application" => {
            let app_id = i64_arg(&args, "app_id")?.ok_or_else(|| {
                anyhow::anyhow!(
                    "delete_application: app_id is required — pass app_id=<id>\n\
                     Hint: use action=applications to list IDs.\n\
                     Also requires confirm=true or GOTIFY_ALLOW_DESTRUCTIVE=true."
                )
            })?;
            state
                .service
                .delete_application(app_id, bool_arg(&args, "confirm"))
                .await
        }
        "create_client" => {
            let name = string_arg(&args, "name").ok_or_else(|| {
                anyhow::anyhow!("create_client: name is required — pass name=\"My Client\"")
            })?;
            state.service.create_client(&name).await
        }
        "delete_client" => {
            let client_id = i64_arg(&args, "client_id")?.ok_or_else(|| {
                anyhow::anyhow!(
                    "delete_client: client_id is required — pass client_id=<id>\n\
                     Hint: use action=clients to list client IDs.\n\
                     Also requires confirm=true or GOTIFY_ALLOW_DESTRUCTIVE=true."
                )
            })?;
            state
                .service
                .delete_client(client_id, bool_arg(&args, "confirm"))
                .await
        }

        // observability
        "status" => {
            let config_snapshot = serde_json::json!({
                "host": state.config.host,
                "port": state.config.port,
                "auth_mode": format!("{:?}", state.auth_policy).split('{').next().unwrap_or("unknown").trim(),
            });
            state.service.status(config_snapshot).await
        }

        "help" => Ok(json!({ "help": HELP_TEXT })),

        other => Err(anyhow::anyhow!(
            "unknown gotify action: \"{other}\"\n\
             Valid actions: health, version, me, messages, applications, clients, \
             send, create_application, update_application, delete_application, \
             create_client, delete_client, delete_message, delete_all_messages, \
             status, help\n\
             See: action=help for full documentation."
        )),
    }
}

fn string_arg(args: &Value, name: &str) -> Option<String> {
    args.get(name).and_then(|v| v.as_str()).map(String::from)
}

fn i64_arg(args: &Value, name: &str) -> anyhow::Result<Option<i64>> {
    let Some(v) = args.get(name) else {
        return Ok(None);
    };
    v.as_i64()
        .map(Some)
        .ok_or_else(|| anyhow::anyhow!("`{name}` must be an integer, got {v:?}"))
}

fn bool_arg(args: &Value, name: &str) -> bool {
    args.get(name).and_then(|v| v.as_bool()).unwrap_or(false)
}

const HELP_TEXT: &str = r#"# gotify MCP Tool

Interact with a Gotify push notification server.
Set the required `action` argument to select the operation.

## Read actions
- `health`              — Server health check (no auth required)
- `version`             — Server version (no auth required)
- `me`                  — Current authenticated user (requires GOTIFY_CLIENT_TOKEN)
- `messages`            — List messages (optional: app_id, limit, since, offset, query)
- `applications`        — List applications (optional: name filter; requires GOTIFY_CLIENT_TOKEN)
- `clients`             — List clients (requires GOTIFY_CLIENT_TOKEN)

## Write actions
- `send`                — Send a notification (requires message; optional: title, priority, extras)
                          Requires GOTIFY_APP_TOKEN (application token, not client token)
- `create_application`  — Create an application (requires name; optional: description, default_priority)
- `update_application`  — Update an application (requires app_id; optional: name, description, default_priority)
- `create_client`       — Create a client (requires name)

## Destructive actions (require confirm=true or GOTIFY_ALLOW_DESTRUCTIVE=true)
- `delete_message`      — Delete one message (requires id, confirm=true)
- `delete_all_messages` — Delete all messages (requires confirm=true)
- `delete_application`  — Delete an application (requires app_id, confirm=true)
- `delete_client`       — Delete a client (requires client_id, confirm=true)

## Observability
- `status`              — Runtime status, counters, and upstream reachability

## Meta
- `help`                — This documentation

## Token types
- GOTIFY_CLIENT_TOKEN: management token — use for messages, applications, clients, me
- GOTIFY_APP_TOKEN:    application token — use only for send (posting messages)

## Pagination (messages)
- limit  (int): max messages per page, default 50, max 200
- offset (int): skip first N results
- since  (int): only messages with id < since (Gotify cursor)
- query  (str): text search in message body and title (case-insensitive)

## Filtering (applications)
- name (str): substring filter on application name (case-insensitive)
"#;

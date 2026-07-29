use rmcp::model::{
    GetPromptRequestParams, GetPromptResult, ListPromptsResult, Prompt, PromptMessage,
    Role,
};

pub(super) fn list_prompts() -> ListPromptsResult {
    ListPromptsResult {
        prompts: vec![
            Prompt::new(
                "send_notification",
                Some("Send a Gotify push notification"),
                None,
            ),
            Prompt::new(
                "check_status",
                Some("Check Gotify server health and recent messages"),
                None,
            ),
        ],
        ..Default::default()
    }
}

pub(super) fn get_prompt(request: GetPromptRequestParams) -> anyhow::Result<GetPromptResult> {
    match request.name.as_str() {
        "send_notification" => Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(
                Role::User,
                "Use the gotify tool with action=send to send a push notification. \
                 Required: message. Optional: title (default: 'Notification'), priority (0=min, 10=max).",
            ),
        ]).with_description("Send a Gotify push notification")),
        "check_status" => Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(
                Role::User,
                "Use the gotify tool to: (1) action=health to check server status, \
                 (2) action=messages with limit=10 to show recent notifications, \
                 then summarize the server state and any notable recent messages.",
            ),
        ]).with_description("Check Gotify server health and recent messages")),
        other => Err(anyhow::anyhow!("unknown prompt: {other}")),
    }
}

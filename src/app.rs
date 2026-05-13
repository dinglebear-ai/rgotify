use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    gotify::GotifyClient,
    observability::{Counters, CountersSnapshot},
};

/// Business service layer. All logic lives here.
/// CLI and MCP are thin shims that call into this.
#[derive(Clone)]
pub struct GotifyService {
    client: GotifyClient,
    allow_destructive: bool,
    pub counters: Arc<Counters>,
    started_at: std::time::Instant,
}

impl GotifyService {
    pub fn new(client: GotifyClient, allow_destructive: bool) -> Self {
        Self {
            client,
            allow_destructive,
            counters: Counters::new(),
            started_at: std::time::Instant::now(),
        }
    }

    /// Create with a shared counters instance (used when AppState owns counters).
    pub fn with_counters(
        client: GotifyClient,
        allow_destructive: bool,
        counters: Arc<Counters>,
    ) -> Self {
        Self {
            client,
            allow_destructive,
            counters,
            started_at: std::time::Instant::now(),
        }
    }

    fn destructive_gate(&self, confirm: bool) -> Result<()> {
        if self.allow_destructive || confirm {
            return Ok(());
        }
        bail!(
            "destructive operation blocked\n\
             Hint: pass confirm=true to proceed with this action, OR set \
             GOTIFY_ALLOW_DESTRUCTIVE=true in your environment to skip confirmation globally.\n\
             Example: action=delete_message id=42 confirm=true"
        )
    }

    // ── read ──────────────────────────────────────────────────────────────────

    pub async fn health(&self) -> Result<Value> {
        self.client.health().await
    }

    pub async fn version(&self) -> Result<Value> {
        self.client.version().await
    }

    pub async fn me(&self) -> Result<Value> {
        self.client.me().await.map_err(|e| {
            if e.to_string().contains("401") || e.to_string().contains("403") {
                anyhow::anyhow!(
                    "me: authentication failed\n\
                     Reason: {e}\n\
                     Hint: GOTIFY_CLIENT_TOKEN is required for this action — \
                     set it to a valid client token from your Gotify server."
                )
            } else {
                e
            }
        })
    }

    pub async fn applications(&self, name_filter: Option<&str>) -> Result<Value> {
        let result = self.client.applications().await.map_err(|e| {
            if e.to_string().contains("401") || e.to_string().contains("403") {
                anyhow::anyhow!(
                    "applications: authentication failed\n\
                     Reason: {e}\n\
                     Hint: GOTIFY_CLIENT_TOKEN is required — this must be a client \
                     token (not an app token). Check your Gotify server's client management page."
                )
            } else {
                e
            }
        })?;

        // Apply name filter if provided
        if let Some(filter) = name_filter {
            let filter_lower = filter.to_lowercase();
            let filtered: Vec<Value> = result
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|app| {
                    app["name"]
                        .as_str()
                        .map(|n| n.to_lowercase().contains(&filter_lower))
                        .unwrap_or(false)
                })
                .collect();

            if filtered.is_empty() {
                return Ok(json!({
                    "applications": [],
                    "total": 0,
                    "hint": format!(
                        "No applications matched filter {:?}. \
                         Use action=applications without a name filter to list all.",
                        filter
                    )
                }));
            }
            return Ok(json!({
                "applications": filtered,
                "total": filtered.len(),
                "filter_name": filter,
            }));
        }

        // Hint if empty
        if result.as_array().map(|a| a.is_empty()).unwrap_or(false) {
            return Ok(json!({
                "applications": [],
                "total": 0,
                "hint": "No applications found. Check GOTIFY_CLIENT_TOKEN is a valid client token \
                         and that the Gotify server has applications configured."
            }));
        }

        Ok(result)
    }

    pub async fn clients(&self) -> Result<Value> {
        self.client.clients().await.map_err(|e| {
            if e.to_string().contains("401") || e.to_string().contains("403") {
                anyhow::anyhow!(
                    "clients: authentication failed\n\
                     Reason: {e}\n\
                     Hint: GOTIFY_CLIENT_TOKEN is required — this must be a client \
                     token (not an app token). Check your Gotify server's client management page."
                )
            } else {
                e
            }
        }).and_then(|result| {
            if result.as_array().map(|a| a.is_empty()).unwrap_or(false) {
                Ok(json!({
                    "clients": [],
                    "total": 0,
                    "hint": "No clients found. Check GOTIFY_CLIENT_TOKEN is a valid client token \
                             and that the Gotify server has clients configured."
                }))
            } else {
                Ok(result)
            }
        })
    }

    pub async fn messages(
        &self,
        app_id: Option<i64>,
        limit: Option<i64>,
        since: Option<i64>,
        offset: Option<i64>,
        query: Option<&str>,
    ) -> Result<Value> {
        let limit = limit.unwrap_or(50).min(200);
        let result = self.client.messages(app_id, Some(limit), since).await?;

        // Extract message list from Gotify's response shape
        let mut messages: Vec<Value> = if let Some(arr) = result["messages"].as_array() {
            arr.clone()
        } else if let Some(arr) = result.as_array() {
            arr.clone()
        } else {
            vec![]
        };

        let total_before_filter = messages.len();

        // Apply text query filter
        if let Some(q) = query {
            let q_lower = q.to_lowercase();
            messages.retain(|m| {
                let msg_text = m["message"].as_str().unwrap_or("").to_lowercase();
                let title = m["title"].as_str().unwrap_or("").to_lowercase();
                msg_text.contains(&q_lower) || title.contains(&q_lower)
            });
        }

        // Apply offset
        let offset = offset.unwrap_or(0).max(0) as usize;
        if offset < messages.len() {
            messages = messages[offset..].to_vec();
        } else {
            messages = vec![];
        }

        let total = if query.is_some() {
            messages.len() as i64
        } else {
            result["paging"]["size"]
                .as_i64()
                .unwrap_or(total_before_filter as i64)
        };

        let has_more = messages.len() as i64 >= limit;
        let next_offset = offset as i64 + messages.len() as i64;

        Ok(json!({
            "messages": messages,
            "total": total,
            "limit": limit,
            "offset": offset,
            "has_more": has_more,
            "next_offset": next_offset,
        }))
    }

    // ── write ─────────────────────────────────────────────────────────────────

    pub async fn send(
        &self,
        message: &str,
        title: Option<&str>,
        priority: Option<i64>,
        extras: Option<Value>,
    ) -> Result<Value> {
        self.client
            .send_message(message, title, priority, extras)
            .await
            .map_err(|e| {
                if e.to_string().contains("401") || e.to_string().contains("403") {
                    anyhow::anyhow!(
                        "send: authentication failed\n\
                         Reason: {e}\n\
                         Hint: GOTIFY_APP_TOKEN is required for send — this must be an \
                         application token (not a client token). Create one on your Gotify \
                         server under Applications → token."
                    )
                } else {
                    e
                }
            })
    }

    pub async fn delete_message(&self, id: i64, confirm: bool) -> Result<Value> {
        self.destructive_gate(confirm)?;
        self.client.delete_message(id).await
    }

    pub async fn delete_all_messages(&self, confirm: bool) -> Result<Value> {
        self.destructive_gate(confirm)?;
        self.client.delete_all_messages().await
    }

    pub async fn create_application(
        &self,
        name: &str,
        description: Option<&str>,
        default_priority: Option<i64>,
    ) -> Result<Value> {
        self.client
            .create_application(name, description, default_priority)
            .await
    }

    pub async fn update_application(
        &self,
        app_id: i64,
        name: Option<&str>,
        description: Option<&str>,
        default_priority: Option<i64>,
    ) -> Result<Value> {
        self.client
            .update_application(app_id, name, description, default_priority)
            .await
    }

    pub async fn delete_application(&self, app_id: i64, confirm: bool) -> Result<Value> {
        self.destructive_gate(confirm)?;
        self.client.delete_application(app_id).await
    }

    pub async fn create_client(&self, name: &str) -> Result<Value> {
        self.client.create_client(name).await
    }

    pub async fn delete_client(&self, client_id: i64, confirm: bool) -> Result<Value> {
        self.destructive_gate(confirm)?;
        self.client.delete_client(client_id).await
    }

    // ── observability ─────────────────────────────────────────────────────────

    pub async fn status(&self, config_snapshot: serde_json::Value) -> Result<Value> {
        let uptime = self.started_at.elapsed().as_secs();
        let counters: CountersSnapshot = self.counters.snapshot();

        // Quick reachability probe with short timeout
        let upstream_reachable =
            tokio::time::timeout(std::time::Duration::from_secs(3), self.client.health())
                .await
                .is_ok_and(|r| r.is_ok());

        Ok(json!({
            "status": if upstream_reachable { "ok" } else { "degraded" },
            "server": {
                "version": env!("CARGO_PKG_VERSION"),
                "uptime_secs": uptime,
                "pid": std::process::id(),
                "data_dir": crate::config::default_data_dir().to_string_lossy(),
            },
            "config": config_snapshot,
            "counters": {
                "requests_total": counters.requests_total,
                "errors_total": counters.errors_total,
                "upstream_calls": counters.upstream_calls,
                "upstream_errors": counters.upstream_errors,
            },
            "upstream": {
                "reachable": upstream_reachable,
            }
        }))
    }
}

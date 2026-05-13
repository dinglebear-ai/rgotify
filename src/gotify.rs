use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};

use crate::config::GotifyConfig;
use crate::observability::Counters;

#[derive(Clone)]
pub struct GotifyClient {
    client: Client,
    base_url: String,
    client_token: String,
    app_token: String,
    pub counters: Arc<Counters>,
}

impl GotifyClient {
    pub fn new(cfg: &GotifyConfig) -> Result<Self> {
        // Graceful degradation: warn on missing URL instead of panicking.
        // The empty-URL check is deferred to the first actual request.
        if cfg.url.is_empty() {
            tracing::warn!(
                "GOTIFY_URL is not set — all API calls will fail until it is configured"
            );
        }

        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            client,
            base_url: cfg.url.trim_end_matches('/').to_string(),
            client_token: cfg.client_token.clone(),
            app_token: cfg.app_token.clone(),
            counters: Counters::new(),
        })
    }

    /// Create with a shared counters instance.
    pub fn with_counters(cfg: &GotifyConfig, counters: Arc<Counters>) -> Result<Self> {
        let mut c = Self::new(cfg)?;
        c.counters = counters;
        Ok(c)
    }

    fn require_url(&self) -> Result<()> {
        if self.base_url.is_empty() {
            anyhow::bail!(
                "GOTIFY_URL is not configured\n\
                 Hint: set GOTIFY_URL=https://your-gotify-server in your environment \
                 or config.toml before starting gotify-mcp."
            );
        }
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Value> {
        self.request("GET", path, &self.client_token, None).await
    }

    async fn post(&self, path: &str, token: &str, body: Value) -> Result<Value> {
        self.request("POST", path, token, Some(body)).await
    }

    async fn put(&self, path: &str, body: Value) -> Result<Value> {
        self.request("PUT", path, &self.client_token, Some(body))
            .await
    }

    async fn delete(&self, path: &str) -> Result<Value> {
        self.request("DELETE", path, &self.client_token, None).await
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        token: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        self.require_url()?;

        if token.is_empty() {
            anyhow::bail!(
                "no token configured for {method} {path}\n\
                 Hint: set GOTIFY_CLIENT_TOKEN for management operations, \
                 or GOTIFY_APP_TOKEN for sending messages."
            );
        }

        self.counters.inc_upstream();

        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let mut req = self
            .client
            .request(method.parse()?, &url)
            .header("X-Gotify-Key", token);
        if let Some(b) = body {
            req = req.json(&b);
        }

        let resp = req.send().await.map_err(|e| {
            self.counters.inc_upstream_error();
            anyhow::anyhow!(
                "Gotify request failed: {e}\n\
                 Hint: check that GOTIFY_URL ({}) is reachable and correct.",
                self.base_url
            )
        })?;

        let status = resp.status();
        if status.as_u16() == 204 {
            return Ok(json!({ "status": "ok" }));
        }

        let body: Value = resp
            .json()
            .await
            .context("failed to parse Gotify response")?;

        if !status.is_success() {
            self.counters.inc_upstream_error();
            if status.as_u16() == 401 {
                anyhow::bail!(
                    "Gotify HTTP 401 Unauthorized: {body}\n\
                     Hint: the token provided was rejected. Check that GOTIFY_CLIENT_TOKEN \
                     (for management) or GOTIFY_APP_TOKEN (for send) is correct and not expired."
                );
            }
            anyhow::bail!("Gotify HTTP {status}: {body}");
        }

        Ok(body)
    }

    // ── no-auth endpoints ─────────────────────────────────────────────────────

    pub async fn health(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.health");
        let _guard = span.enter();

        if self.base_url.is_empty() {
            return Ok(json!({ "status": "degraded", "reason": "GOTIFY_URL not configured" }));
        }

        let url = format!("{}/health", self.base_url);
        tracing::debug!(url = %url, "calling upstream health");

        let result = self
            .client
            .get(&url)
            .send()
            .await
            .context("health check failed")
            .and_then(|r| {
                let _ = r.status();
                // parse synchronously not possible here — handled below
                Ok(r)
            });

        match result {
            Ok(resp) => {
                let val: Value = resp.json().await.unwrap_or(json!({ "status": "ok" }));
                tracing::debug!("upstream health ok");
                Ok(val)
            }
            Err(e) => {
                tracing::warn!(error = %e, "upstream health check failed");
                Err(e)
            }
        }
    }

    pub async fn version(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.version");
        let _guard = span.enter();
        self.require_url()?;

        let url = format!("{}/version", self.base_url);
        tracing::debug!(url = %url, "calling upstream version");

        let result = self.client.get(&url).send().await;
        match result {
            Ok(resp) => {
                let val: Value = resp
                    .json()
                    .await
                    .context("failed to parse version response")?;
                tracing::debug!("upstream version ok");
                Ok(val)
            }
            Err(e) => {
                tracing::warn!(error = %e, "upstream version call failed");
                Err(anyhow::anyhow!("version check failed: {e}"))
            }
        }
    }

    // ── messages ──────────────────────────────────────────────────────────────

    pub async fn messages(
        &self,
        app_id: Option<i64>,
        limit: Option<i64>,
        since: Option<i64>,
    ) -> Result<Value> {
        let span = tracing::info_span!("upstream.messages");
        let _guard = span.enter();
        self.require_url()?;

        if self.client_token.is_empty() {
            anyhow::bail!(
                "messages: GOTIFY_CLIENT_TOKEN is required\n\
                 Hint: set GOTIFY_CLIENT_TOKEN to a client token from your Gotify server. \
                 Client tokens are different from app tokens — create one under Clients in the \
                 Gotify web UI."
            );
        }

        let path = if let Some(id) = app_id {
            format!("application/{id}/message")
        } else {
            "message".to_string()
        };
        let limit = limit.unwrap_or(50);
        let mut url = format!("{}/{}?limit={limit}", self.base_url, path);
        if let Some(s) = since {
            url.push_str(&format!("&since={s}"));
        }

        tracing::debug!(url = %url, "calling upstream messages");
        self.counters.inc_upstream();

        let resp = self
            .client
            .get(&url)
            .header("X-Gotify-Key", &self.client_token)
            .send()
            .await
            .map_err(|e| {
                self.counters.inc_upstream_error();
                anyhow::anyhow!("messages request failed: {e}")
            })?;

        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .context("failed to parse messages response")?;

        if !status.is_success() {
            self.counters.inc_upstream_error();
            anyhow::bail!("Gotify HTTP {status}: {body}");
        }

        let count = body["messages"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or_default();
        tracing::debug!(count, "upstream messages ok");
        Ok(body)
    }

    pub async fn send_message(
        &self,
        message: &str,
        title: Option<&str>,
        priority: Option<i64>,
        extras: Option<Value>,
    ) -> Result<Value> {
        let span = tracing::info_span!("upstream.send_message");
        let _guard = span.enter();

        if self.app_token.is_empty() {
            anyhow::bail!(
                "send: GOTIFY_APP_TOKEN is required\n\
                 Hint: GOTIFY_APP_TOKEN must be an application token (not a client token). \
                 Create one under Applications in the Gotify web UI."
            );
        }

        let mut body = json!({ "message": message });
        if let Some(t) = title {
            body["title"] = json!(t);
        }
        if let Some(p) = priority {
            body["priority"] = json!(p);
        }
        if let Some(e) = extras {
            body["extras"] = e;
        }

        tracing::debug!("sending message to upstream");
        let result = self.post("message", &self.app_token, body).await;
        match &result {
            Ok(_) => tracing::debug!("upstream send_message ok"),
            Err(e) => tracing::warn!(error = %e, "upstream send_message failed"),
        }
        result
    }

    pub async fn delete_message(&self, id: i64) -> Result<Value> {
        let span = tracing::info_span!("upstream.delete_message", id);
        let _guard = span.enter();
        self.delete(&format!("message/{id}")).await
    }

    pub async fn delete_all_messages(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.delete_all_messages");
        let _guard = span.enter();
        self.delete("message").await
    }

    // ── applications ──────────────────────────────────────────────────────────

    pub async fn applications(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.applications");
        let _guard = span.enter();
        tracing::debug!("calling upstream applications");
        let result = self.get("application").await;
        match &result {
            Ok(v) => tracing::debug!(
                count = v.as_array().map(|a| a.len()).unwrap_or(0),
                "upstream applications ok"
            ),
            Err(e) => tracing::warn!(error = %e, "upstream applications failed"),
        }
        result
    }

    pub async fn create_application(
        &self,
        name: &str,
        description: Option<&str>,
        default_priority: Option<i64>,
    ) -> Result<Value> {
        let span = tracing::info_span!("upstream.create_application");
        let _guard = span.enter();
        let mut body = json!({ "name": name });
        if let Some(d) = description {
            body["description"] = json!(d);
        }
        if let Some(p) = default_priority {
            body["defaultPriority"] = json!(p);
        }
        self.post("application", &self.client_token, body).await
    }

    pub async fn update_application(
        &self,
        app_id: i64,
        name: Option<&str>,
        description: Option<&str>,
        default_priority: Option<i64>,
    ) -> Result<Value> {
        let span = tracing::info_span!("upstream.update_application", app_id);
        let _guard = span.enter();
        let mut body = json!({});
        if let Some(n) = name {
            body["name"] = json!(n);
        }
        if let Some(d) = description {
            body["description"] = json!(d);
        }
        if let Some(p) = default_priority {
            body["defaultPriority"] = json!(p);
        }
        self.put(&format!("application/{app_id}"), body).await
    }

    pub async fn delete_application(&self, app_id: i64) -> Result<Value> {
        let span = tracing::info_span!("upstream.delete_application", app_id);
        let _guard = span.enter();
        self.delete(&format!("application/{app_id}")).await
    }

    // ── clients ───────────────────────────────────────────────────────────────

    pub async fn clients(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.clients");
        let _guard = span.enter();
        tracing::debug!("calling upstream clients");
        let result = self.get("client").await;
        match &result {
            Ok(v) => tracing::debug!(
                count = v.as_array().map(|a| a.len()).unwrap_or(0),
                "upstream clients ok"
            ),
            Err(e) => tracing::warn!(error = %e, "upstream clients failed"),
        }
        result
    }

    pub async fn create_client(&self, name: &str) -> Result<Value> {
        let span = tracing::info_span!("upstream.create_client");
        let _guard = span.enter();
        self.post("client", &self.client_token, json!({ "name": name }))
            .await
    }

    pub async fn delete_client(&self, client_id: i64) -> Result<Value> {
        let span = tracing::info_span!("upstream.delete_client", client_id);
        let _guard = span.enter();
        self.delete(&format!("client/{client_id}")).await
    }

    // ── user ──────────────────────────────────────────────────────────────────

    pub async fn me(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.me");
        let _guard = span.enter();
        tracing::debug!("calling upstream me");
        let result = self.get("current/user").await;
        match &result {
            Ok(_) => tracing::debug!("upstream me ok"),
            Err(e) => tracing::warn!(error = %e, "upstream me failed"),
        }
        result
    }
}

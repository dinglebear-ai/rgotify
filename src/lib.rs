//! Core Gotify client, service, configuration, and MCP transport library.

pub mod app;
pub mod config;
pub mod gotify;
pub mod logging;
pub mod mcp;
pub mod observability;
pub mod token_limit;

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod testing {
    use std::sync::Arc;

    use crate::{
        app::GotifyService,
        config::{GotifyConfig, McpConfig},
        gotify::GotifyClient,
        mcp::{AppState, AuthPolicy},
    };

    fn stub_service() -> GotifyService {
        let client = GotifyClient::new(&GotifyConfig {
            url: "http://localhost:1".into(),
            client_token: "test".into(),
            app_token: "test".into(),
            allow_destructive: false,
        })
        .expect("stub client should build");
        GotifyService::new(client, false)
    }

    pub fn loopback_state() -> AppState {
        AppState {
            config: McpConfig::default(),
            auth_policy: AuthPolicy::LoopbackDev,
            service: stub_service(),
        }
    }

    pub fn bearer_state(token: &str) -> AppState {
        AppState {
            config: McpConfig {
                api_token: Some(token.to_string()),
                ..McpConfig::default()
            },
            auth_policy: AuthPolicy::Mounted { auth_state: None },
            service: stub_service(),
        }
    }

    pub async fn oauth_state(data_dir: &std::path::Path) -> AppState {
        let (state, _) = oauth_state_with_auth_state(data_dir).await;
        state
    }

    pub async fn oauth_state_with_auth_state(
        data_dir: &std::path::Path,
    ) -> (AppState, Arc<lab_auth::state::AuthState>) {
        let auth_state = Arc::new(build_auth_state(data_dir).await);
        let state = AppState {
            config: McpConfig {
                auth: crate::config::AuthConfig {
                    public_url: Some("https://gotify.example.com".to_string()),
                    ..Default::default()
                },
                ..McpConfig::default()
            },
            auth_policy: AuthPolicy::Mounted {
                auth_state: Some(auth_state.clone()),
            },
            service: stub_service(),
        };
        (state, auth_state)
    }

    pub async fn build_auth_state(data_dir: &std::path::Path) -> lab_auth::state::AuthState {
        let vars: Vec<(String, String)> = vec![
            ("GOTIFY_MCP_AUTH_MODE".into(), "oauth".into()),
            (
                "GOTIFY_MCP_PUBLIC_URL".into(),
                "https://gotify.example.com".into(),
            ),
            (
                "GOTIFY_MCP_GOOGLE_CLIENT_ID".into(),
                "test-client-id".into(),
            ),
            (
                "GOTIFY_MCP_GOOGLE_CLIENT_SECRET".into(),
                "test-client-secret".into(),
            ),
            (
                "GOTIFY_MCP_AUTH_ADMIN_EMAIL".into(),
                "admin@example.com".into(),
            ),
            (
                "GOTIFY_MCP_AUTH_SQLITE_PATH".into(),
                data_dir.join("auth.db").to_str().unwrap().into(),
            ),
            (
                "GOTIFY_MCP_AUTH_KEY_PATH".into(),
                data_dir.join("auth-jwt.pem").to_str().unwrap().into(),
            ),
        ];
        let auth_config = lab_auth::config::AuthConfigBuilder::new()
            .env_prefix("GOTIFY_MCP")
            .session_cookie_name("gotify_mcp_session")
            .scopes_supported(vec!["gotify:read".into(), "gotify:write".into()])
            .default_scope("gotify:read")
            .resource_path("/mcp")
            .build_from_sources(vars)
            .expect("test auth config should build");
        lab_auth::state::AuthState::new(auth_config)
            .await
            .expect("test auth state should init")
    }
}

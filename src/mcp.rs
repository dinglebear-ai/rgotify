use std::sync::Arc;

use lab_auth::AuthLayer;

use crate::{app::GotifyService, config::McpConfig};

mod prompts;
mod rmcp_server;
mod routes;
mod schemas;
mod tools;

pub use rmcp_server::{
    rmcp_server, streamable_http_config, streamable_http_service, GotifyRmcpServer,
};
pub use routes::router;

#[derive(Clone)]
pub enum AuthPolicy {
    LoopbackDev,
    Mounted {
        auth_state: Option<Arc<lab_auth::state::AuthState>>,
    },
}

impl std::fmt::Debug for AuthPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthPolicy::LoopbackDev => f.write_str("AuthPolicy::LoopbackDev"),
            AuthPolicy::Mounted {
                auth_state: Some(_),
            } => f.write_str("AuthPolicy::Mounted { auth_state: Some(<AuthState>) }"),
            AuthPolicy::Mounted { auth_state: None } => {
                f.write_str("AuthPolicy::Mounted { auth_state: None }")
            }
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: McpConfig,
    pub auth_policy: AuthPolicy,
    pub service: GotifyService,
}

pub fn build_auth_layer(
    policy: &AuthPolicy,
    static_token: Option<Arc<str>>,
    resource_url: Option<Arc<str>>,
) -> Option<AuthLayer> {
    match policy {
        AuthPolicy::LoopbackDev => None,
        AuthPolicy::Mounted { auth_state } => Some(
            AuthLayer::new()
                .with_static_token(static_token)
                .with_auth_state(auth_state.clone())
                .with_static_token_scopes(vec!["gotify:read".into(), "gotify:write".into()])
                .with_resource_url(resource_url)
                .with_allow_session_cookie(false),
        ),
    }
}

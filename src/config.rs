use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub mcp: McpConfig,
    pub gotify: GotifyConfig,
}

/// Gotify server connection config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GotifyConfig {
    /// Gotify base URL (GOTIFY_URL)
    pub url: String,
    /// Client token for management operations (GOTIFY_CLIENT_TOKEN)
    pub client_token: String,
    /// App token for sending messages (GOTIFY_APP_TOKEN)
    pub app_token: String,
    /// Allow destructive operations without confirm flag (GOTIFY_ALLOW_DESTRUCTIVE)
    pub allow_destructive: bool,
}

impl Default for GotifyConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            client_token: String::new(),
            app_token: String::new(),
            allow_destructive: false,
        }
    }
}

/// MCP HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpConfig {
    #[serde(default = "default_mcp_host")]
    pub host: String,
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    #[serde(default = "default_server_name")]
    pub server_name: String,
    pub no_auth: bool,
    pub api_token: Option<String>,
    pub allowed_hosts: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub auth: AuthConfig,
}

impl McpConfig {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub public_url: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub admin_email: String,
    pub allowed_emails: Vec<String>,
    pub sqlite_path: String,
    pub key_path: String,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
    pub auth_code_ttl_secs: u64,
    pub register_rpm: u32,
    pub authorize_rpm: u32,
    pub disable_static_token_with_oauth: bool,
    pub allowed_client_redirect_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    #[default]
    Bearer,
    OAuth,
}

/// Returns the default data directory for gotify-mcp.
///
/// In a container (`/.dockerenv` present or `RUNNING_IN_CONTAINER` set): `/data`
/// On bare metal: `~/.gotify`
pub fn default_data_dir() -> PathBuf {
    if std::path::Path::new("/.dockerenv").exists() || std::env::var("RUNNING_IN_CONTAINER").is_ok()
    {
        return PathBuf::from("/data");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".gotify")
}

fn default_mcp_host() -> String {
    "0.0.0.0".into()
}
fn default_mcp_port() -> u16 {
    40020
}
fn default_server_name() -> String {
    "gotify-mcp".into()
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            host: default_mcp_host(),
            port: default_mcp_port(),
            server_name: default_server_name(),
            no_auth: false,
            api_token: None,
            allowed_hosts: Vec::new(),
            allowed_origins: Vec::new(),
            auth: AuthConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::default(),
            public_url: None,
            google_client_id: None,
            google_client_secret: None,
            admin_email: String::new(),
            allowed_emails: Vec::new(),
            sqlite_path: default_data_dir()
                .join("auth.db")
                .to_string_lossy()
                .into_owned(),
            key_path: default_data_dir()
                .join("auth-jwt.pem")
                .to_string_lossy()
                .into_owned(),
            access_token_ttl_secs: 3600,
            refresh_token_ttl_secs: 86400 * 30,
            auth_code_ttl_secs: 300,
            register_rpm: 10,
            authorize_rpm: 60,
            disable_static_token_with_oauth: true,
            allowed_client_redirect_uris: Vec::new(),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Config::default();

        match std::fs::read_to_string("config.toml") {
            Ok(contents) => {
                config = toml::from_str(&contents)
                    .map_err(|e| anyhow::anyhow!("Failed to parse config.toml: {e}"))?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(anyhow::anyhow!("Failed to read config.toml: {e}")),
        }

        env_str("GOTIFY_URL", &mut config.gotify.url);
        env_str("GOTIFY_CLIENT_TOKEN", &mut config.gotify.client_token);
        env_str("GOTIFY_APP_TOKEN", &mut config.gotify.app_token);
        env_bool(
            "GOTIFY_ALLOW_DESTRUCTIVE",
            &mut config.gotify.allow_destructive,
        )?;

        env_str("GOTIFY_MCP_HOST", &mut config.mcp.host);
        env_parse("GOTIFY_MCP_PORT", &mut config.mcp.port)?;
        env_bool("GOTIFY_MCP_NO_AUTH", &mut config.mcp.no_auth)?;
        env_opt_str("GOTIFY_MCP_TOKEN", &mut config.mcp.api_token);
        env_list("GOTIFY_MCP_ALLOWED_HOSTS", &mut config.mcp.allowed_hosts);
        env_list(
            "GOTIFY_MCP_ALLOWED_ORIGINS",
            &mut config.mcp.allowed_origins,
        );
        env_opt_str("GOTIFY_MCP_PUBLIC_URL", &mut config.mcp.auth.public_url);
        env_str(
            "GOTIFY_MCP_AUTH_ADMIN_EMAIL",
            &mut config.mcp.auth.admin_email,
        );

        Ok(config)
    }
}

fn env_str(key: &str, target: &mut String) {
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            *target = v;
        }
    }
}

fn env_opt_str(key: &str, target: &mut Option<String>) {
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            *target = Some(v);
        }
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, target: &mut T) -> anyhow::Result<()> {
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            *target = v
                .parse()
                .map_err(|_| anyhow::anyhow!("{key}: invalid value {v:?}"))?;
        }
    }
    Ok(())
}

fn env_bool(key: &str, target: &mut bool) -> anyhow::Result<()> {
    if let Ok(v) = std::env::var(key) {
        match v.to_lowercase().as_str() {
            "1" | "true" | "yes" => *target = true,
            "0" | "false" | "no" => *target = false,
            other => anyhow::bail!("{key}: expected bool, got {other:?}"),
        }
    }
    Ok(())
}

fn env_list(key: &str, target: &mut Vec<String>) {
    if let Ok(v) = std::env::var(key) {
        let items: Vec<String> = v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !items.is_empty() {
            *target = items;
        }
    }
}

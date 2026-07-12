use anyhow::Result;
use std::sync::Arc;

use gotify_mcp::{
    app::GotifyService,
    config::{AuthMode, Config},
    gotify::GotifyClient,
    mcp::{self, AppState, AuthPolicy},
};
use rmcp::{transport::stdio, ServiceExt};
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [f] if matches!(f.as_str(), "--help" | "-h" | "help") => {
            print_usage();
            return Ok(());
        }
        [f] if matches!(f.as_str(), "--version" | "-V") => {
            println!("gotify-mcp {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    if let Some((command, json)) = cli::setup::SetupCommand::parse(&args)? {
        return cli::setup::run(command, json);
    }

    // Load ~/.gotify/.env (or /data/.env in a container) before any Config::load
    // so the binary works on bare metal without a process manager injecting env.
    // Non-overriding: explicit process env still wins.
    gotify_mcp::config::load_dotenv();

    let stdio_mode = matches!(args.as_slice(), [c] if c == "mcp");
    let serve_mode = args.is_empty()
        || matches!(args.as_slice(), [c] if c == "serve")
        || matches!(args.as_slice(), [a, b] if a == "serve" && b == "mcp");

    let log_level = if stdio_mode || !serve_mode {
        "warn"
    } else {
        "info"
    };
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .with_writer(std::io::stderr)
        .with_target(true)
        .init();

    if serve_mode {
        serve_mcp().await
    } else if stdio_mode {
        serve_stdio_mcp().await
    } else {
        run_cli(args).await
    }
}

fn validate_bind_security(config: &Config) -> anyhow::Result<()> {
    let is_loopback = config.mcp.host.starts_with("127.") || config.mcp.host == "::1";
    let has_auth = (!config.mcp.no_auth && config.mcp.api_token.is_some())
        || config.mcp.auth.mode == AuthMode::OAuth;
    let noauth_override = std::env::var("GOTIFY_NOAUTH")
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false);

    if !is_loopback && !has_auth && !noauth_override {
        anyhow::bail!(
            "Refusing to bind MCP server to {} without authentication.\n\
             Set GOTIFY_MCP_TOKEN, use auth_mode=oauth, or set GOTIFY_NOAUTH=true \
             if an upstream gateway handles auth.",
            config.mcp.host
        );
    }
    Ok(())
}

async fn serve_mcp() -> Result<()> {
    let config = Config::load()?;
    validate_bind_security(&config)?;
    let state = build_state(config).await?;
    info!(bind = %state.config.bind_addr(), server_name = %state.config.server_name, "gotify-mcp starting");
    let bind = state.config.bind_addr();
    let app = mcp::router(state).layer(tower_http::trace::TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    info!(bind = %bind, "MCP HTTP server listening");
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn serve_stdio_mcp() -> Result<()> {
    // Stdio is always LoopbackDev — trusted local pipe, no HTTP auth context.
    let config = Config::load()?;
    let service = build_service(&config)?;
    #[allow(clippy::useless_conversion)]
    let state = AppState {
        config: config.mcp,
        auth_policy: AuthPolicy::LoopbackDev,
        service,
    };
    let _ = (); // appease compiler
    let svc = mcp::rmcp_server(state).serve(stdio()).await?;
    svc.waiting().await?;
    Ok(())
}

async fn run_cli(args: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let service = build_service(&config)?;
    let (cmd, json) = cli::CliCommand::parse(&args)?;
    cli::run(&service, cmd, json).await
}

fn build_service(config: &Config) -> Result<GotifyService> {
    let client = GotifyClient::new(&config.gotify)?;
    Ok(GotifyService::new(client, config.gotify.allow_destructive))
}

async fn build_state(config: Config) -> Result<AppState> {
    let auth_policy = build_auth_policy(&config).await?;
    Ok(AppState {
        service: build_service(&config)?,
        config: config.mcp,
        auth_policy,
    })
}

async fn build_auth_policy(config: &Config) -> Result<AuthPolicy> {
    if config.mcp.no_auth || config.mcp.host.starts_with("127.") {
        return Ok(AuthPolicy::LoopbackDev);
    }
    if config.mcp.auth.mode == AuthMode::OAuth {
        let auth_cfg = lab_auth::config::AuthConfigBuilder::new()
            .env_prefix("GOTIFY_MCP")
            .session_cookie_name("gotify_mcp_session")
            .scopes_supported(vec!["gotify:read".into(), "gotify:write".into()])
            .default_scope("gotify:read")
            .resource_path("/mcp")
            .enable_dynamic_registration(true)
            .build_from_sources(std::env::vars())
            .map_err(|e| anyhow::anyhow!("OAuth config error: {e}"))?;
        let auth_state = lab_auth::state::AuthState::new(auth_cfg)
            .await
            .map_err(|e| anyhow::anyhow!("OAuth state init error: {e}"))?;
        Ok(AuthPolicy::Mounted {
            auth_state: Some(Arc::new(auth_state)),
        })
    } else {
        Ok(AuthPolicy::Mounted { auth_state: None })
    }
}

fn print_usage() {
    eprintln!(
        "Usage:
  rgotify [serve]                          Start MCP HTTP server
  rgotify mcp                              Start MCP stdio transport
  rgotify doctor [--json]                  Pre-flight environment check
  rgotify setup check [--json]             Check local plugin setup
  rgotify setup repair [--json]            Repair local plugin setup
  rgotify setup plugin-hook [--no-repair] [--json]

Read:
  rgotify health [--json]                  Server health
  rgotify version [--json]                 Server version
  rgotify me [--json]                      Current user
  rgotify messages [--app-id N] [--limit N] [--since N] [--json]
  rgotify applications [--json]            List applications
  rgotify clients [--json]                 List clients

Write:
  rgotify send <message> [--title T] [--priority N] [--json]
  rgotify create app <name> [--description D] [--priority N] [--json]
  rgotify create client <name> [--json]

Destructive (add --confirm):
  rgotify delete message <id> [--confirm] [--json]
  rgotify delete all [--confirm] [--json]
  rgotify delete app <app_id> [--confirm] [--json]
  rgotify delete client <client_id> [--confirm] [--json]

Environment:
  GOTIFY_URL                  Gotify server URL (required)
  GOTIFY_CLIENT_TOKEN         Client token for management (required)
  GOTIFY_APP_TOKEN            App token for sending messages (required)
  GOTIFY_ALLOW_DESTRUCTIVE    Skip confirm gate for destructive ops
  GOTIFY_MCP_HOST             Bind host (default 0.0.0.0)
  GOTIFY_MCP_PORT             Bind port (default 40020)
  GOTIFY_MCP_TOKEN            Static bearer token for MCP auth
  GOTIFY_MCP_NO_AUTH          Disable MCP auth
  RUST_LOG                    Log filter"
    );
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "CTRL+C handler failed");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "SIGTERM handler failed");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("Shutdown signal received");
}

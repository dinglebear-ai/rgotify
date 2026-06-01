use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use serde::Serialize;

/// A single doctor check result.
#[derive(Serialize)]
pub struct DoctorCheck {
    pub category: &'static str,
    pub name: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Warn-level: shown as ⚠ but does NOT count as a failure.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub warn_only: bool,
}

impl DoctorCheck {
    fn pass(category: &'static str, name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            ok: true,
            value: Some(value.into()),
            hint: None,
            latency_ms: None,
            warn_only: false,
        }
    }

    fn fail(category: &'static str, name: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            ok: false,
            value: None,
            hint: Some(hint.into()),
            latency_ms: None,
            warn_only: false,
        }
    }

    fn warn(category: &'static str, name: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            ok: false,
            value: None,
            hint: Some(hint.into()),
            latency_ms: None,
            warn_only: true,
        }
    }
}

// ── Individual check helpers ──────────────────────────────────────────────────

fn check_config_file(data_dir: &PathBuf) -> DoctorCheck {
    let path = data_dir.join("config.toml");
    if path.exists() {
        DoctorCheck::pass("config", "Config file", path.display().to_string())
    } else {
        DoctorCheck::warn(
            "config",
            "Config file",
            format!("{} not found — using defaults and env vars", path.display()),
        )
    }
}

fn check_dir_writable(label: &'static str, dir: &PathBuf) -> DoctorCheck {
    if let Err(e) = std::fs::create_dir_all(dir) {
        return DoctorCheck::fail(
            "config",
            label,
            format!("cannot create {}: {}", dir.display(), e),
        );
    }
    // Test by writing a temp file.
    let probe = dir.join(".doctor_write_test");
    match std::fs::write(&probe, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            // Report size if it's the log dir
            let suffix = if dir.ends_with("logs") {
                dir_size_hint(dir)
            } else {
                String::new()
            };
            DoctorCheck::pass("config", label, format!("{}{}", dir.display(), suffix))
        }
        Err(e) => DoctorCheck::fail(
            "config",
            label,
            format!("{} is not writable: {}", dir.display(), e),
        ),
    }
}

fn dir_size_hint(dir: &PathBuf) -> String {
    let total: u64 = walkdir_size(dir);
    if total == 0 {
        return String::new();
    }
    if total < 1024 * 1024 {
        format!(" ({} KB)", total / 1024)
    } else {
        format!(" ({:.1} MB)", total as f64 / 1_048_576.0)
    }
}

fn walkdir_size(dir: &PathBuf) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut total = 0u64;
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                total += meta.len();
            } else if meta.is_dir() {
                total += walkdir_size(&entry.path());
            }
        }
    }
    total
}

fn check_binary_in_path(binary: &str) -> DoctorCheck {
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in path_var.split(':') {
        let candidate = PathBuf::from(dir).join(binary);
        if candidate.is_file() {
            if let Ok(meta) = std::fs::metadata(&candidate) {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o111 != 0 {
                    return DoctorCheck::pass(
                        "config",
                        format!("Binary in PATH: {binary}"),
                        candidate.display().to_string(),
                    );
                }
            }
        }
    }
    DoctorCheck::fail(
        "config",
        format!("Binary in PATH: {binary}"),
        format!("`{binary}` not found in $PATH — add ~/.local/bin to PATH or re-run install.sh"),
    )
}

fn check_required_env(var_name: &str, hint: &str) -> DoctorCheck {
    match std::env::var(var_name) {
        Ok(v) if !v.is_empty() => DoctorCheck::pass(
            "credentials",
            var_name,
            format!(
                "{} (set)",
                &v[..v.len().min(8).max(1)].replace(|_: char| true, "*")
            ),
        ),
        _ => DoctorCheck::fail("credentials", var_name, hint),
    }
}

fn check_optional_env(var_name: &str, hint: &str) -> DoctorCheck {
    match std::env::var(var_name) {
        Ok(v) if !v.is_empty() => DoctorCheck::pass(
            "credentials",
            var_name,
            format!(
                "{} (set)",
                &v[..v.len().min(8).max(1)].replace(|_: char| true, "*")
            ),
        ),
        _ => DoctorCheck::warn("credentials", var_name, hint),
    }
}

fn check_token_prefix(var_name: &str, expected_prefix: char, label: &str) -> Option<DoctorCheck> {
    let Ok(val) = std::env::var(var_name) else {
        return None;
    };
    if val.is_empty() {
        return None;
    }
    if !val.starts_with(expected_prefix) {
        Some(DoctorCheck::warn(
            "credentials",
            format!("{var_name} format"),
            format!(
                "{label} tokens usually start with '{expected_prefix}' — got '{}'... (check token type in Gotify dashboard)",
                &val[..val.len().min(2)]
            ),
        ))
    } else {
        None
    }
}

async fn check_upstream(url: &str) -> DoctorCheck {
    if url.is_empty() {
        return DoctorCheck::fail(
            "connectivity",
            "Upstream reachable",
            "GOTIFY_URL is not set — cannot test connectivity",
        );
    }
    let health_url = format!("{}/health", url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return DoctorCheck::fail(
                "connectivity",
                "Upstream reachable",
                format!("failed to build HTTP client: {e}"),
            )
        }
    };
    let start = Instant::now();
    match client.get(&health_url).send().await {
        Ok(resp) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let status = resp.status();
            let mut check = if status.is_success() {
                DoctorCheck::pass(
                    "connectivity",
                    "Upstream reachable",
                    format!("{health_url} → {} ({elapsed} ms)", status),
                )
            } else {
                DoctorCheck::fail(
                    "connectivity",
                    "Upstream reachable",
                    format!("{health_url} returned {status} — check GOTIFY_URL and server status"),
                )
            };
            check.latency_ms = Some(elapsed);
            check
        }
        Err(e) => {
            let mut check = DoctorCheck::fail(
                "connectivity",
                "Upstream reachable",
                format!("could not reach {health_url}: {e}"),
            );
            check.latency_ms = Some(start.elapsed().as_millis() as u64);
            check
        }
    }
}

fn check_port_available(port: u16) -> DoctorCheck {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => DoctorCheck::pass("mcp_server", format!("MCP port {port}"), "available"),
        Err(_) => DoctorCheck::warn(
            "mcp_server",
            format!("MCP port {port}"),
            format!("port {port} is already in use — set GOTIFY_MCP_PORT to a different port"),
        ),
    }
}

fn check_auth_config() -> DoctorCheck {
    let token_set = std::env::var("GOTIFY_MCP_TOKEN")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let noauth = std::env::var("GOTIFY_MCP_NO_AUTH")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);
    let noauth2 = std::env::var("GOTIFY_NOAUTH")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);

    if token_set {
        DoctorCheck::pass(
            "mcp_server",
            "Auth config",
            "bearer token (GOTIFY_MCP_TOKEN)",
        )
    } else if noauth || noauth2 {
        DoctorCheck::pass(
            "mcp_server",
            "Auth config",
            "no-auth (GOTIFY_MCP_NO_AUTH / GOTIFY_NOAUTH)",
        )
    } else {
        DoctorCheck::warn(
            "mcp_server",
            "Auth config",
            "no auth configured — set GOTIFY_MCP_TOKEN or GOTIFY_MCP_NO_AUTH=true \
             (required when binding to non-loopback address)",
        )
    }
}

// ── Report printing ───────────────────────────────────────────────────────────

fn print_doctor_report(checks: &[DoctorCheck]) {
    let version = env!("CARGO_PKG_VERSION");
    println!("\ngotify-mcp v{version} — environment check\n");

    let categories = [
        ("config", "Config"),
        ("credentials", "Service credentials"),
        ("connectivity", "Connectivity"),
        ("mcp_server", "MCP server"),
    ];

    for (cat_key, cat_label) in &categories {
        let cat_checks: Vec<&DoctorCheck> =
            checks.iter().filter(|c| c.category == *cat_key).collect();
        if cat_checks.is_empty() {
            continue;
        }
        println!("  {cat_label}");
        println!("  {}", "─".repeat(46));
        for c in &cat_checks {
            let icon = if c.ok {
                "✓"
            } else if c.warn_only {
                "⚠"
            } else {
                "✗"
            };
            let name_padded = format!("{:<24}", format!("{}:", c.name));
            if c.ok {
                println!(
                    "  {icon} {name_padded} {}",
                    c.value.as_deref().unwrap_or("")
                );
            } else {
                println!("  {icon} {name_padded} {}", c.hint.as_deref().unwrap_or(""));
            }
        }
        println!();
    }

    let failures: Vec<&DoctorCheck> = checks.iter().filter(|c| !c.ok && !c.warn_only).collect();
    let warnings: Vec<&DoctorCheck> = checks.iter().filter(|c| !c.ok && c.warn_only).collect();

    println!("  {}", "━".repeat(48));
    if failures.is_empty() && warnings.is_empty() {
        println!("  All checks passed — ready to run: gotify serve\n");
    } else if failures.is_empty() {
        println!(
            "  {} warning(s) — review above, then run: gotify serve\n",
            warnings.len()
        );
    } else {
        println!(
            "  {} issue(s) found. Fix {} before running: gotify serve\n",
            failures.len(),
            if failures.len() == 1 { "it" } else { "them" }
        );
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub async fn run_doctor(mcp_port: u16, json: bool) -> Result<()> {
    let data_dir = gotify_mcp::config::default_data_dir();
    let mut checks: Vec<DoctorCheck> = Vec::new();

    // ── Config ────────────────────────────────────────────────────────────────
    checks.push(check_config_file(&data_dir));
    checks.push(check_dir_writable("Data directory", &data_dir));
    checks.push(check_dir_writable("Log directory", &data_dir.join("logs")));
    checks.push(check_binary_in_path("rgotify"));

    // ── Credentials ───────────────────────────────────────────────────────────
    checks.push(check_required_env(
        "GOTIFY_URL",
        "Set GOTIFY_URL in ~/.gotify/.env or your environment (e.g. https://gotify.example.com)",
    ));
    checks.push(check_required_env(
        "GOTIFY_CLIENT_TOKEN",
        "Set GOTIFY_CLIENT_TOKEN — a client token (starts with C) from the Gotify dashboard",
    ));
    checks.push(check_optional_env(
        "GOTIFY_APP_TOKEN",
        "Set GOTIFY_APP_TOKEN to send messages — an app token (starts with A) from the Gotify dashboard",
    ));

    // Token prefix validation (warn only)
    if let Some(c) = check_token_prefix("GOTIFY_CLIENT_TOKEN", 'C', "Client") {
        checks.push(c);
    }
    if let Some(c) = check_token_prefix("GOTIFY_APP_TOKEN", 'A', "App") {
        checks.push(c);
    }

    // ── Connectivity ──────────────────────────────────────────────────────────
    let gotify_url = std::env::var("GOTIFY_URL").unwrap_or_default();
    checks.push(check_upstream(&gotify_url).await);

    // ── MCP server ────────────────────────────────────────────────────────────
    checks.push(check_port_available(mcp_port));
    checks.push(check_auth_config());

    // ── Report ────────────────────────────────────────────────────────────────
    let failures = checks.iter().filter(|c| !c.ok && !c.warn_only).count();

    if json {
        println!("{}", serde_json::to_string_pretty(&checks)?);
    } else {
        print_doctor_report(&checks);
    }

    if failures > 0 {
        std::process::exit(1);
    }
    Ok(())
}

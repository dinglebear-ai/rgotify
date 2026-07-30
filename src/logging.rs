use std::path::Path;
use std::sync::Arc;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub mod aurora;
mod file;

use file::{CappedFileWriter, SharedFileWriter};

const LOG_MAX_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

/// Returns true if stderr should be colorized.
pub fn should_colorize() -> bool {
    // Respect NO_COLOR (https://no-color.org)
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    // Force color in containers / CI
    if std::env::var("FORCE_COLOR").is_ok() {
        return true;
    }
    use std::io::IsTerminal;
    std::io::stderr().is_terminal()
}

/// Initialize dual logging: colored console (stderr) + structured JSON file.
///
/// - Console: human-readable, aurora colors, goes to stderr
/// - File:    JSON structured, `{data_dir}/logs/gotify.log`, 10 MB cap
///
/// Call this once from `main.rs` before any log output.
pub fn init_logging(data_dir: &Path) -> anyhow::Result<()> {
    let log_path = data_dir.join("logs").join("gotify.log");

    let file_writer = Arc::new(
        CappedFileWriter::open(log_path, LOG_MAX_BYTES)
            .map_err(|e| anyhow::anyhow!("failed to open log file: {e}"))?,
    );
    let shared = SharedFileWriter(file_writer);

    let console_ansi = should_colorize();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            // Console: pretty, colored, human-readable — always stderr
            tracing_subscriber::fmt::layer()
                .with_ansi(console_ansi)
                .with_writer(std::io::stderr),
        )
        .with(
            // File: structured JSON, no ANSI
            tracing_subscriber::fmt::layer()
                .json()
                .with_ansi(false)
                .with_writer(shared),
        )
        .init();

    Ok(())
}

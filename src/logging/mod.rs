//! Queryable file-based logging system for Rose Online Game Client.
//!
//! This module provides structured logging in JSON Lines format that is easy
//! for LLMs to search and analyze. Each client session creates a timestamped
//! folder containing:
//! - `session.json`: Session metadata
//! - `structured.jsonl`: JSON Lines format log entries

mod json_format;

use std::fs::{self, File, OpenOptions};
use std::path::PathBuf;
use std::time::SystemTime;

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};

pub use json_format::TagExtractingJsonFormat;

/// Session metadata written to `session.json`
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier (folder name)
    pub session_id: String,
    /// Session start time in UTC
    pub start_time_utc: String,
    /// Session start time in local timezone
    pub start_time_local: String,
    /// Hostname of the machine
    pub hostname: String,
    /// Command line arguments
    pub command_line: String,
    /// Mode the client is running in (Game, ZoneViewer, MapEditor, ModelViewer)
    pub mode: String,
    /// Bevy version
    pub bevy_version: String,
    /// Rust version (if available)
    pub rust_version: String,
    /// Operating system info
    pub os: String,
    /// Additional configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

impl SessionInfo {
    /// Creates a new SessionInfo with current timestamp and system info
    pub fn new(mode: &str, config: Option<serde_json::Value>) -> Self {
        let now = SystemTime::now();
        let utc: DateTime<Utc> = now.into();
        let local: DateTime<Local> = now.into();

        let session_id = local.format("%Y-%m-%d_%H-%M-%S").to_string();

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let command_line = std::env::args().collect::<Vec<_>>().join(" ");

        Self {
            session_id,
            start_time_utc: utc.to_rfc3339(),
            start_time_local: local.to_rfc3339(),
            hostname,
            command_line,
            mode: mode.to_string(),
            bevy_version: "0.16.1".to_string(),
            rust_version: rustc_version_runtime::version().to_string(),
            os: std::env::consts::OS.to_string(),
            config,
        }
    }
}

/// Handle to the logging session. Keep this alive for the duration of the app.
pub struct LoggingGuard {
    /// Guard for the non-blocking file writer - must be kept alive
    _file_guard: WorkerGuard,
    /// Path to the session directory
    pub session_dir: PathBuf,
    /// Session information
    pub session_info: SessionInfo,
}

/// Configuration for the logging system
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Base directory for log folders (default: "logs")
    pub log_directory: PathBuf,
    /// Minimum log level (default: "debug")
    pub level: String,
    /// Whether to also output to console (default: true)
    pub console_output: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_directory: PathBuf::from("logs"),
            level: "info".to_string(),
            console_output: true,
        }
    }
}

/// Initialize session-based logging with JSON Lines output.
///
/// This function:
/// 1. Creates a timestamped folder under the logs directory
/// 2. Sets up JSON Lines output to `structured.jsonl`
/// 3. Creates a `session.json` metadata file
/// 4. Maintains console output simultaneously
/// 5. Uses non-blocking writes for performance
///
/// # Arguments
/// * `mode` - The mode the client is running in (e.g., "Game", "ZoneViewer")
/// * `config` - Optional logging configuration
/// * `session_config` - Optional JSON value to include in session metadata
///
/// # Returns
/// A `LoggingGuard` that must be kept alive for the duration of the application.
/// When dropped, it will ensure all logs are flushed.
///
/// # Example
/// ```no_run
/// use rose_offline_client::logging::{init_session_logging, LoggingConfig};
///
/// let _guard = init_session_logging("Game", None, None);
/// // Application runs with structured logging enabled
/// ```
pub fn init_session_logging(
    mode: &str,
    config: Option<LoggingConfig>,
    session_config: Option<serde_json::Value>,
) -> Result<LoggingGuard, anyhow::Error> {
    let config = config.unwrap_or_default();

    // Create session info
    let session_info = SessionInfo::new(mode, session_config);
    let session_id = &session_info.session_id;

    // Create session directory
    let session_dir = config.log_directory.join(session_id);
    fs::create_dir_all(&session_dir)?;

    // Write session metadata
    let session_json_path = session_dir.join("session.json");
    let session_json = File::create(&session_json_path)?;
    serde_json::to_writer_pretty(session_json, &session_info)?;

    // Create non-blocking file writer for JSON logs
    let jsonl_path = session_dir.join("structured.jsonl");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&jsonl_path)?;

    let (non_blocking, file_guard) = tracing_appender::non_blocking(file);

    // Build the subscriber with layers
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    // JSON Lines layer for file output with tag extraction
    let json_layer = fmt::layer()
        .event_format(TagExtractingJsonFormat::new())
        .fmt_fields(tracing_subscriber::fmt::format::JsonFields::new())
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_filter(env_filter.clone());

    if config.console_output {
        // Console layer with standard formatting
        let console_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_ansi(true)
            .with_filter(env_filter);

        tracing_subscriber::registry()
            .with(json_layer)
            .with(console_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(json_layer)
            .init();
    }

    log::info!(
        "[LOGGING] Session logging initialized: {}",
        session_dir.display()
    );

    Ok(LoggingGuard {
        _file_guard: file_guard,
        session_dir,
        session_info,
    })
}

/// Initialize logging for Bevy applications.
///
/// This is designed to work alongside Bevy's logging system. It should be
/// called before Bevy's DefaultPlugins are built.
///
/// Note: When using this with Bevy, you may need to disable Bevy's `bevy_log`
/// feature or coordinate with it appropriately.
pub fn init_bevy_logging(
    mode: &str,
    config: Option<LoggingConfig>,
    session_config: Option<serde_json::Value>,
) -> Result<LoggingGuard, anyhow::Error> {
    init_session_logging(mode, config, session_config)
}

/// Get the path to the current session's log directory (if initialized)
pub fn get_session_log_path() -> Option<PathBuf> {
    // This is a placeholder - in a real implementation, you'd store this
    // in a thread-local or global variable
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_info_creation() {
        let info = SessionInfo::new("Test", None);
        assert!(info.session_id.contains('-')); // Date format has dashes
        assert_eq!(info.mode, "Test");
        assert_eq!(info.bevy_version, "0.16.1");
    }

    #[test]
    fn test_tag_extraction() {
        use crate::logging::json_format::extract_tag;
        
        assert_eq!(extract_tag("[ZONE LOADER] Loading zone 1"), Some("ZONE LOADER"));
        assert_eq!(extract_tag("[VFS] File not found"), Some("VFS"));
        assert_eq!(extract_tag("No tag here"), None);
        assert_eq!(extract_tag(""), None);
    }
}

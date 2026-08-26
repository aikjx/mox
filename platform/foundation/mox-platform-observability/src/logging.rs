//! Structured logging configuration.
//!
//! Supports JSON (production) and pretty (development) formats.
//! All log entries include service_name, timestamp, level, and trace_id.

use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Clone)]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

impl Default for LogFormat {
    fn default() -> Self {
        if std::env::var("MOX_ENV").as_deref() == Ok("production") {
            LogFormat::Json
        } else {
            LogFormat::Compact
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogConfig {
    pub format: LogFormat,
    pub filter: String,
    pub include_target: bool,
    pub include_thread_id: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::default(),
            filter: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            include_target: true,
            include_thread_id: false,
        }
    }
}

/// Initialize the global tracing subscriber with the given configuration.
///
/// # Panics
/// Panics if called more than once (tracing subscriber can only be set once).
pub fn init_logging(service_name: &str, config: &LogConfig) {
    let filter = EnvFilter::try_new(&config.filter)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let base = fmt()
        .with_env_filter(filter)
        .with_target(config.include_target)
        .with_thread_ids(config.include_thread_id)
        .with_file(false)
        .with_line_number(false);

    match config.format {
        LogFormat::Json => {
            base.json()
                .with_current_span(true)
                .with_span_list(true)
                .init();
        }
        LogFormat::Pretty => {
            base.pretty().init();
        }
        LogFormat::Compact => {
            base.compact().init();
        }
    }

    tracing::info!(
        service.name = service_name,
        "logging initialized (format={:?}, filter={})",
        config.format,
        config.filter
    );
}

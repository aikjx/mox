//! 结构化日志初始化 — JSON格式，可对接Loki/ELK，零配置

use tracing_subscriber::{fmt, EnvFilter};

/// 初始化日志系统（零配置，默认JSON格式+info级别）
pub fn init_logging(service_name: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let is_json = std::env::var("MOX_LOG_FORMAT")
        .map(|v| v == "json")
        .unwrap_or(true);

    if is_json {
        fmt()
            .json()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .init();
    } else {
        fmt()
            .with_env_filter(filter)
            .with_target(true)
            .init();
    }

    tracing::info!(service = service_name, "logging initialized");
}

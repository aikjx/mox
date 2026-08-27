// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 企业级可观测性：结构化日志初始化与请求追踪
//!
//! - 日志级别由 [`crate::config::Config::log_level`] 控制；
//! - `json_log=true` 时输出 JSON 行（对接 Loki/ELK），否则人类可读；
//! - 每个 HTTP 请求自动打点（见 [`request_span`] 中间件），便于链路追踪与排障。

use tracing::Instrument;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// 初始化全局 tracing 订阅者。
///
/// 在 `main` 尽早调用一次。失败（如重复初始化）返回错误但不影响业务启动。
pub fn init(level: &str, json_log: bool) -> anyhow::Result<()> {
    let lvl = if level.is_empty() { "info" } else { level };
    let filter = EnvFilter::try_new(format!("primiflow_fusion={lvl},ous={lvl},warn"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry().with(filter);

    if json_log {
        registry
            .with(fmt::layer().json())
            .try_init()
            .map_err(|e| anyhow::anyhow!("tracing 初始化失败：{e}"))?;
    } else {
        registry
            .with(fmt::layer().with_target(true))
            .try_init()
            .map_err(|e| anyhow::anyhow!("tracing 初始化失败：{e}"))?;
    }
    Ok(())
}

/// 为一次请求构造 tracing span，记录方法/路径/状态码/耗时。
///
/// 设计为 axum `middleware::from_fn` 使用：在 [`crate::server::build_router`] 中挂载。
pub async fn request_span(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let span =
        tracing::info_span!("http", method = %method, path = %path, status = tracing::field::Empty);
    let _enter = span.enter();

    let start = std::time::Instant::now();
    let resp = next.run(req).instrument(span.clone()).await;
    let elapsed = start.elapsed();

    let status = resp.status();
    span.record("status", status.as_u16());
    tracing::info!(latency_ms = elapsed.as_millis() as u64, "handled");
    resp
}

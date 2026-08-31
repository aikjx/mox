//! # mox-platform-gateway-runtime
//!
//! 网关运行时 — HTTP/gRPC 统一入口，路由分发，中间件装配
//!
//! ## 功能特性
//! - HTTP RESTful API 服务
//! - gRPC 内部服务
//! - 统一请求 ID 与日志追踪
//! - CORS 与鉴权中间件
//! - 健康检查与指标

#![warn(missing_docs)]
#![warn(clippy::all)]

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

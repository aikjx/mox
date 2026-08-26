//! # mox-framework
//!
//! mox 企业级基础框架层 — 所有服务共享的基础设施
//!
//! ## 模块
//! - `config` — 统一配置管理（YAML/JSON/TOML/环境变量）
//! - `logging` — 结构化日志（JSON格式，可对接Loki/ELK）
//! - `error` — 统一错误类型 + 错误码体系
//! - `health` — 健康检查（存活/就绪/详细）
//! - `metrics` — 指标收集（Prometheus格式）
//! - `tracing` — 分布式追踪（OpenTelemetry）
//! - `auth` — 认证授权（JWT + RBAC + API Key）
//! - `tenant` — 多租户（三档隔离：逻辑/Schema/集群）
//! - `resilience` — 弹性容错（限流/熔断/降级/重试/超时/舱壁）
//! - `server` — 标准化服务器启动器（统一生命周期/优雅关停）

pub mod auth;
pub mod config;
pub mod error;
pub mod health;
pub mod logging;
pub mod metrics;
pub mod resilience;
pub mod server;
pub mod tenant;
pub mod tracing;

/// 框架版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 框架名称
pub const NAME: &str = "mox-framework";

/// 统一的 Result 类型
pub type FrameworkResult<T> = Result<T, error::FrameworkError>;

/// 重导出常用类型
pub use config::FrameworkConfig;
pub use error::FrameworkError;
pub use logging::init_logging;
pub use server::FrameworkServer;
pub use tenant::TenantContext;

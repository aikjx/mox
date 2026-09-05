// =============================================================================
// MOX 统一服务运行时（mox-server-runtime）
// =============================================================================
//
// 所有独立微服务 binary 复用的基座，提供：
//
// 1. **统一配置**（config）— 环境变量 + TOML 文件 + 默认值三级加载
// 2. **HTTP 服务器**（server）— axum 统一启动，CORS/限流/超时中间件
// 3. **健康检查**（health）— /health/live, /health/ready, /health/metrics
// 4. **认证中间件**（auth）— JWT 验证 + RBAC 权限校验
// 5. **可观测性**（observability）— 结构化日志初始化 + tracing
// 6. **优雅停机**（shutdown）— SIGTERM/SIGINT 信号处理，连接 draining
// 7. **服务特征**（ServiceModule）— 每个模块实现此 trait 即可注册为独立服务
//
// 设计原则：
// - 约定优于配置：合理默认值，最小配置即可启动
// - 模块化：每个业务模块实现 ServiceModule trait，运行时负责基础设施
// - 可观测：所有服务统一日志格式、指标端点、追踪上下文
// - 企业级：安全头、限流、超时、优雅停机、审计日志
// =============================================================================

pub mod config;
pub mod server;
pub mod health;
pub mod shutdown;
pub mod cache_factory;
pub mod rate_limit;
pub mod tracing_utils;

pub use config::{ServerConfig, DatabaseConfig, CacheConfig, AuthConfig, ObservabilityConfig};
pub use server::Server;
pub use health::HealthRegistry;
pub use cache_factory::{CacheHandle, CacheBackend};
pub use rate_limit::{RateLimitLayer, RateLimiter, RateLimitConfig};
pub use tracing_utils::{extract_trace_id, generate_trace_id, inject_trace_id, make_request_span, record_trace_id};

use async_trait::async_trait;
use axum::Router;

/// Crate 元数据
pub const CRATE_ID: &str = "mox-server-runtime";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 服务模块特征：每个独立微服务实现此 trait
///
/// # 示例
/// ```ignore
/// pub struct KgModule;
///
/// #[async_trait]
/// impl ServiceModule for KgModule {
///     fn name(&self) -> &str { "mox-kg-server" }
///     fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
///     async fn routes(&self, config: &ServerConfig) -> Router {
///         Router::new().nest("/api/v1/kg", kg_routes())
///     }
/// }
/// ```
#[async_trait]
pub trait ServiceModule: Send + Sync {
    /// 服务名称（用于日志、指标标签、默认配置文件名）
    fn name(&self) -> &str;

    /// 服务版本
    fn version(&self) -> &str {
        "unknown"
    }

    /// 业务路由（运行时会自动挂载 /health 前缀和全局中间件）
    async fn routes(&self, config: &ServerConfig) -> Router;

    /// 服务初始化（数据库连接、缓存连接等），在启动路由前调用
    async fn init(&self, _config: &ServerConfig) -> Result<(), RuntimeError> {
        Ok(())
    }

    /// 服务优雅关闭（释放资源），在收到停机信号后调用
    async fn shutdown(&self) {
        // 默认空实现
    }

    /// 就绪检查依赖（数据库连通性、缓存连通性等）
    async fn ready_checks(&self) -> Vec<(&'static str, bool)> {
        vec![]
    }
}

/// 运行时错误
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("配置错误: {0}")]
    ConfigError(String),
    #[error("服务器启动失败: {0}")]
    ServerError(String),
    #[error("模块初始化失败: {0}")]
    InitError(String),
    #[error("数据库连接失败: {0}")]
    DatabaseError(String),
    #[error("缓存连接失败: {0}")]
    CacheError(String),
    #[error("内部错误: {0}")]
    InternalError(String),
}

/// 运行时结果类型
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// 初始化全局日志（所有服务统一调用）
pub fn init_logging(service_name: &str, json_format: bool, level: &str) {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));
    if json_format {
        fmt()
            .json()
            .with_env_filter(filter)
            .with_target(true)
            .with_current_span(true)
            .init();
    } else {
        fmt()
            .with_env_filter(filter)
            .with_target(true)
            .init();
    }
    tracing::info!(service = service_name, "MOX 服务日志系统已初始化");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_metadata() {
        assert_eq!(CRATE_ID, "mox-server-runtime");
        assert!(!CRATE_VERSION.is_empty());
    }

    #[test]
    fn runtime_error_display() {
        let err = RuntimeError::ConfigError("test".to_string());
        assert!(format!("{err}").contains("配置错误"));
    }
}

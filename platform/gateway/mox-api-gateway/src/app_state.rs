// =============================================================================
// 应用状态（AppState）
// =============================================================================

use mox_auth_core::{AccessControl, AuthMiddleware, JwtManager, PasswordManager};
use mox_config_core::ConfigManager;
use mox_observability_core::{HealthChecker, MetricsCollector};
use std::sync::Arc;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    /// 配置管理器
    pub config: Arc<ConfigManager>,
    /// JWT 管理器
    pub jwt: Arc<JwtManager>,
    /// 认证中间件
    pub auth: Arc<AuthMiddleware>,
    /// 访问控制器
    pub access_control: Arc<AccessControl>,
    /// 密码管理器
    pub password: Arc<PasswordManager>,
    /// 指标收集器
    pub metrics: Arc<MetricsCollector>,
    /// 健康检查器
    pub health: Arc<HealthChecker>,
    /// 服务名称
    pub service_name: String,
    /// 服务版本
    pub version: String,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new() -> Self {
        let config = Arc::new(ConfigManager::new("prod"));

        // JWT 配置
        let jwt_config = mox_auth_core::JwtConfig::new(
            std::env::var("MOX_JWT_SECRET")
                .unwrap_or_else(|_| "mox-default-jwt-secret-change-in-production-12345".to_string()),
        )
        .with_issuer("mox-platform")
        .with_access_ttl(3600)
        .with_refresh_ttl(604800);

        let jwt = Arc::new(JwtManager::new(jwt_config.clone()).expect("JWT 管理器初始化失败"));

        let access_control = Arc::new(AccessControl::new());
        let auth = Arc::new(AuthMiddleware::new(
            JwtManager::new(jwt_config.clone()).unwrap(),
            AccessControl::new(),
        ));
        let password = Arc::new(PasswordManager::new());

        let metrics = Arc::new(MetricsCollector::new("mox_gateway"));
        let health = Arc::new(HealthChecker::new("mox-api-gateway", env!("CARGO_PKG_VERSION")));

        Self {
            config,
            jwt,
            auth,
            access_control,
            password,
            metrics,
            health,
            service_name: "mox-api-gateway".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 记录请求指标
    pub fn record_request(&self, method: &str, path: &str, status: u16, duration_ms: u64) {
        self.metrics.record_request(method, path, status, duration_ms);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("service", &self.service_name)
            .field("version", &self.version)
            .finish()
    }
}

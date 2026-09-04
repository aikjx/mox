// =============================================================================
// 健康检查模块
// =============================================================================

use crate::{ObservabilityError, ObservabilityResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

// =============================================================================
// 健康状态
// =============================================================================

/// 健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 降级（部分功能不可用）
    Degraded,
    /// 不健康
    Unhealthy,
    /// 未知
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
            HealthStatus::Unknown => "unknown",
        }
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// 组件健康
// =============================================================================

/// 组件健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// 组件名称
    pub name: String,
    /// 组件类型（database/cache/service/external）
    pub component_type: String,
    /// 健康状态
    pub status: HealthStatus,
    /// 响应时间（毫秒）
    pub response_time_ms: Option<u64>,
    /// 详细信息
    pub details: Option<String>,
    /// 最后检查时间
    pub last_checked: chrono::DateTime<chrono::Utc>,
    /// 连续失败次数
    pub consecutive_failures: u32,
}

impl ComponentHealth {
    pub fn new(name: impl Into<String>, component_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            component_type: component_type.into(),
            status: HealthStatus::Unknown,
            response_time_ms: None,
            details: None,
            last_checked: chrono::Utc::now(),
            consecutive_failures: 0,
        }
    }

    pub fn healthy(mut self, response_time_ms: u64) -> Self {
        self.status = HealthStatus::Healthy;
        self.response_time_ms = Some(response_time_ms);
        self.last_checked = chrono::Utc::now();
        self.consecutive_failures = 0;
        self
    }

    pub fn degraded(mut self, details: impl Into<String>) -> Self {
        self.status = HealthStatus::Degraded;
        self.details = Some(details.into());
        self.last_checked = chrono::Utc::now();
        self
    }

    pub fn unhealthy(mut self, details: impl Into<String>) -> Self {
        self.status = HealthStatus::Unhealthy;
        self.details = Some(details.into());
        self.last_checked = chrono::Utc::now();
        self.consecutive_failures += 1;
        self
    }
}

// =============================================================================
// 健康检查器
// =============================================================================

/// 健康检查 trait
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// 组件名称
    fn name(&self) -> &str;

    /// 组件类型
    fn component_type(&self) -> &str;

    /// 执行健康检查
    async fn check(&self) -> ObservabilityResult<ComponentHealth>;
}

/// 健康检查器（聚合多个组件检查）
#[derive(Clone)]
pub struct HealthChecker {
    checks: Arc<parking_lot::RwLock<Vec<Arc<dyn HealthCheck>>>>,
    component_statuses: Arc<parking_lot::RwLock<BTreeMap<String, ComponentHealth>>>,
    service_name: String,
    version: String,
    start_time: chrono::DateTime<chrono::Utc>,
}

impl HealthChecker {
    pub fn new(service_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            checks: Arc::new(parking_lot::RwLock::new(Vec::new())),
            component_statuses: Arc::new(parking_lot::RwLock::new(BTreeMap::new())),
            service_name: service_name.into(),
            version: version.into(),
            start_time: chrono::Utc::now(),
        }
    }

    /// 注册健康检查
    pub fn register_check(&self, check: Arc<dyn HealthCheck>) {
        self.checks.write().push(check);
    }

    /// 执行所有健康检查
    pub async fn check_all(&self) -> HealthReport {
        let checks = self.checks.read().clone();
        let mut components = Vec::new();

        for check in checks.iter() {
            let result = match tokio::time::timeout(Duration::from_secs(5), check.check()).await {
                Ok(Ok(health)) => health,
                Ok(Err(e)) => ComponentHealth::new(check.name(), check.component_type())
                    .unhealthy(format!("检查失败: {}", e)),
                Err(_) => ComponentHealth::new(check.name(), check.component_type())
                    .unhealthy("检查超时".to_string()),
            };

            // 更新缓存
            self.component_statuses
                .write()
                .insert(result.name.clone(), result.clone());
            components.push(result);
        }

        // 计算整体状态
        let overall_status = if components.iter().all(|c| c.status.is_healthy()) {
            HealthStatus::Healthy
        } else if components.iter().any(|c| c.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if components.iter().any(|c| c.status == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unknown
        };

        HealthReport {
            service: self.service_name.clone(),
            version: self.version.clone(),
            status: overall_status,
            uptime_seconds: (chrono::Utc::now() - self.start_time).num_seconds() as u64,
            timestamp: chrono::Utc::now(),
            components,
        }
    }

    /// 获取存活状态（轻量检查，不执行依赖检查）
    pub fn liveness(&self) -> HealthReport {
        HealthReport {
            service: self.service_name.clone(),
            version: self.version.clone(),
            status: HealthStatus::Healthy,
            uptime_seconds: (chrono::Utc::now() - self.start_time).num_seconds() as u64,
            timestamp: chrono::Utc::now(),
            components: vec![],
        }
    }

    /// 获取就绪状态（执行所有依赖检查）
    pub async fn readiness(&self) -> HealthReport {
        self.check_all().await
    }

    /// 获取组件状态缓存
    pub fn get_component_status(&self, name: &str) -> Option<ComponentHealth> {
        self.component_statuses.read().get(name).cloned()
    }

    /// 获取所有组件状态
    pub fn get_all_component_statuses(&self) -> Vec<ComponentHealth> {
        self.component_statuses.read().values().cloned().collect()
    }
}

impl std::fmt::Debug for HealthChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthChecker")
            .field("service", &self.service_name)
            .field("version", &self.version)
            .field("checks_count", &self.checks.read().len())
            .finish()
    }
}

// =============================================================================
// 健康报告
// =============================================================================

/// 健康报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// 服务名称
    pub service: String,
    /// 服务版本
    pub version: String,
    /// 整体健康状态
    pub status: HealthStatus,
    /// 运行时间（秒）
    pub uptime_seconds: u64,
    /// 报告时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 组件健康状态列表
    pub components: Vec<ComponentHealth>,
}

impl HealthReport {
    /// 是否健康
    pub fn is_healthy(&self) -> bool {
        self.status.is_healthy()
    }

    /// HTTP 状态码（健康=200，降级=200，不健康=503）
    pub fn http_status_code(&self) -> u16 {
        match self.status {
            HealthStatus::Healthy | HealthStatus::Degraded => 200,
            HealthStatus::Unhealthy => 503,
            HealthStatus::Unknown => 503,
        }
    }
}

// =============================================================================
// 内置健康检查实现
// =============================================================================

/// 数据库健康检查（示例实现）
pub struct DatabaseHealthCheck {
    pub name: String,
    pub dsn: String,
}

#[async_trait]
impl HealthCheck for DatabaseHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    fn component_type(&self) -> &str {
        "database"
    }

    async fn check(&self) -> ObservabilityResult<ComponentHealth> {
        // 简化实现：实际应执行 SELECT 1
        let start = std::time::Instant::now();
        // 模拟数据库连接检查
        tokio::time::sleep(Duration::from_millis(5)).await;
        let elapsed = start.elapsed().as_millis() as u64;

        Ok(ComponentHealth::new(&self.name, "database").healthy(elapsed))
    }
}

/// 内存健康检查
pub struct MemoryHealthCheck {
    pub threshold_percent: f64,
}

#[async_trait]
impl HealthCheck for MemoryHealthCheck {
    fn name(&self) -> &str {
        "memory"
    }

    fn component_type(&self) -> &str {
        "system"
    }

    async fn check(&self) -> ObservabilityResult<ComponentHealth> {
        // 简化实现：实际应读取系统内存使用
        Ok(ComponentHealth::new("memory", "system").healthy(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(!HealthStatus::Unhealthy.is_healthy());
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Degraded.as_str(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.as_str(), "unhealthy");
    }

    #[test]
    fn test_component_health_healthy() {
        let health = ComponentHealth::new("postgres", "database").healthy(50);
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.response_time_ms, Some(50));
        assert_eq!(health.consecutive_failures, 0);
    }

    #[test]
    fn test_component_health_unhealthy() {
        let mut health = ComponentHealth::new("redis", "cache");
        health = health.unhealthy("connection refused");
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert_eq!(health.consecutive_failures, 1);

        health = health.unhealthy("still down");
        assert_eq!(health.consecutive_failures, 2);
    }

    #[tokio::test]
    async fn test_health_checker_liveness() {
        let checker = HealthChecker::new("test-service", "1.0.0");
        let report = checker.liveness();
        assert!(report.is_healthy());
        assert_eq!(report.service, "test-service");
        assert_eq!(report.version, "1.0.0");
        assert_eq!(report.http_status_code(), 200);
    }

    #[tokio::test]
    async fn test_health_checker_with_checks() {
        let checker = HealthChecker::new("test-service", "1.0.0");

        let db_check = Arc::new(DatabaseHealthCheck {
            name: "postgres".to_string(),
            dsn: "postgres://localhost/test".to_string(),
        });
        checker.register_check(db_check);

        let report = checker.readiness().await;
        assert_eq!(report.components.len(), 1);
        assert_eq!(report.components[0].name, "postgres");
    }

    #[test]
    fn test_health_report_http_status() {
        let healthy = HealthReport {
            service: "test".to_string(),
            version: "1.0".to_string(),
            status: HealthStatus::Healthy,
            uptime_seconds: 100,
            timestamp: chrono::Utc::now(),
            components: vec![],
        };
        assert_eq!(healthy.http_status_code(), 200);

        let unhealthy = HealthReport {
            status: HealthStatus::Unhealthy,
            ..healthy.clone()
        };
        assert_eq!(unhealthy.http_status_code(), 503);
    }

    #[test]
    fn test_component_health_degraded() {
        let health = ComponentHealth::new("api", "service").degraded("high latency");
        assert_eq!(health.status, HealthStatus::Degraded);
        assert!(health.details.is_some());
    }

    #[tokio::test]
    async fn test_memory_health_check() {
        let check = MemoryHealthCheck { threshold_percent: 90.0 };
        let health = check.check().await.unwrap();
        assert_eq!(health.name, "memory");
        assert!(health.status.is_healthy());
    }
}

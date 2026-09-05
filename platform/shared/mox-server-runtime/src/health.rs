// =============================================================================
// 健康检查注册表（HealthRegistry）
// =============================================================================
//
// 管理服务的存活/就绪检查，提供 Prometheus 格式指标输出。
// 每个服务模块可注册自定义检查项（数据库连通性、缓存连通性等）。
// =============================================================================

use crate::config::ServerConfig;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// 单个健康检查项
pub struct HealthCheck {
    pub name: &'static str,
    pub check_fn: Box<dyn Fn() -> bool + Send + Sync>,
}

/// 健康检查注册表
pub struct HealthRegistry {
    service_name: String,
    checks: RwLock<HashMap<&'static str, HealthCheck>>,
    start_time: Instant,
}

impl HealthRegistry {
    /// 创建健康检查注册表
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
            checks: RwLock::new(HashMap::new()),
            start_time: Instant::now(),
        }
    }

    /// 注册一个检查项
    pub fn register<F>(&self, name: &'static str, check_fn: F)
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        let mut checks = self.checks.write().unwrap();
        checks.insert(
            name,
            HealthCheck {
                name,
                check_fn: Box::new(check_fn),
            },
        );
    }

    /// 执行所有检查，返回 (名称, 是否通过)
    pub async fn check_all(&self) -> Vec<(&'static str, bool)> {
        let checks = self.checks.read().unwrap();
        checks
            .values()
            .map(|c| (c.name, (c.check_fn)()))
            .collect()
    }

    /// 生成 Prometheus 格式指标
    pub fn metrics(&self) -> String {
        let uptime_secs = self.start_time.elapsed().as_secs();
        format!(
            "# HELP mox_service_uptime_seconds Service uptime in seconds\n\
             # TYPE mox_service_uptime_seconds gauge\n\
             mox_service_uptime_seconds{{service=\"{name}\"}} {uptime}\n\
             # HELP mox_service_start_time Service start timestamp\n\
             # TYPE mox_service_start_time gauge\n\
             mox_service_start_time{{service=\"{name}\"}} {start}\n",
            name = self.service_name,
            uptime = uptime_secs,
            start = chrono::Utc::now().timestamp() - uptime_secs as i64,
        )
    }

    /// 获取服务运行时间（秒）
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_check() {
        let registry = HealthRegistry::new("test".to_string());
        registry.register("db", || true);
        registry.register("cache", || false);
        // check_all is async, test with tokio
        let rt = tokio::runtime::Runtime::new().unwrap();
        let results = rt.block_on(registry.check_all());
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|(n, ok)| *n == "db" && *ok));
        assert!(results.iter().any(|(n, ok)| *n == "cache" && !*ok));
    }

    #[test]
    fn test_metrics_output() {
        let registry = HealthRegistry::new("test".to_string());
        let metrics = registry.metrics();
        assert!(metrics.contains("mox_service_uptime_seconds{service=\"test\"}"));
    }

    #[test]
    fn test_uptime() {
        let registry = HealthRegistry::new("test".to_string());
        assert!(registry.uptime_secs() < 10);
    }
}

// =============================================================================
// 可扩展指标注册表（Metrics Registry）
// =============================================================================
//
// 提供统一的指标收集机制，各模块可以注册自己的 Prometheus 指标提供者，
// /metrics 端点会收集所有注册提供者的指标并合并输出。
//
// 使用方式：
// ```ignore
// use mox_server_runtime::metrics::{MetricsRegistry, MetricsProvider};
//
// struct DsqlMetricsProvider { manager: Arc<DsqlManager> }
//
// #[async_trait]
// impl MetricsProvider for DsqlMetricsProvider {
//     fn name(&self) -> &str { "dsql" }
//     fn gather(&self) -> String { self.manager.gather_metrics() }
// }
//
// let registry = MetricsRegistry::new();
// registry.register(Arc::new(DsqlMetricsProvider { manager }));
// ```
// =============================================================================

use parking_lot::RwLock;
use std::sync::Arc;

/// 指标提供者 trait
pub trait MetricsProvider: Send + Sync {
    /// 提供者名称（用于标识和去重）
    fn name(&self) -> &str;
    /// 收集 Prometheus 文本格式指标
    fn gather(&self) -> String;
}

/// 可扩展指标注册表
#[derive(Clone, Default)]
pub struct MetricsRegistry {
    providers: Arc<RwLock<Vec<Arc<dyn MetricsProvider>>>>,
}

impl MetricsRegistry {
    /// 创建空的指标注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册指标提供者
    pub fn register(&self, provider: Arc<dyn MetricsProvider>) {
        let name = provider.name().to_string();
        let mut providers = self.providers.write();
        // 移除同名的旧提供者
        providers.retain(|p| p.name() != name);
        providers.push(provider);
        tracing::info!(provider = %name, "指标提供者已注册");
    }

    /// 注销指标提供者
    pub fn unregister(&self, name: &str) {
        let mut providers = self.providers.write();
        providers.retain(|p| p.name() != name);
    }

    /// 收集所有注册提供者的指标
    pub fn gather_all(&self) -> String {
        let providers = self.providers.read();
        let mut output = String::new();
        for provider in providers.iter() {
            let metrics = provider.gather();
            if !metrics.is_empty() {
                output.push_str(&metrics);
                if !metrics.ends_with('\n') {
                    output.push('\n');
                }
            }
        }
        output
    }

    /// 获取已注册的提供者名称列表
    pub fn provider_names(&self) -> Vec<String> {
        self.providers
            .read()
            .iter()
            .map(|p| p.name().to_string())
            .collect()
    }

    /// 已注册提供者数量
    pub fn len(&self) -> usize {
        self.providers.read().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.providers.read().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider {
        name: String,
        metrics: String,
    }

    impl MetricsProvider for TestProvider {
        fn name(&self) -> &str {
            &self.name
        }
        fn gather(&self) -> String {
            self.metrics.clone()
        }
    }

    #[test]
    fn test_registry_register_and_gather() {
        let registry = MetricsRegistry::new();
        assert!(registry.is_empty());

        registry.register(Arc::new(TestProvider {
            name: "test1".to_string(),
            metrics: "test_metric 1\n".to_string(),
        }));
        assert_eq!(registry.len(), 1);

        registry.register(Arc::new(TestProvider {
            name: "test2".to_string(),
            metrics: "test_metric2 2\n".to_string(),
        }));
        assert_eq!(registry.len(), 2);

        let output = registry.gather_all();
        assert!(output.contains("test_metric 1"));
        assert!(output.contains("test_metric2 2"));
    }

    #[test]
    fn test_registry_register_duplicate_name() {
        let registry = MetricsRegistry::new();

        registry.register(Arc::new(TestProvider {
            name: "test".to_string(),
            metrics: "old 1\n".to_string(),
        }));
        registry.register(Arc::new(TestProvider {
            name: "test".to_string(),
            metrics: "new 2\n".to_string(),
        }));

        assert_eq!(registry.len(), 1);
        let output = registry.gather_all();
        assert!(output.contains("new 2"));
        assert!(!output.contains("old 1"));
    }

    #[test]
    fn test_registry_unregister() {
        let registry = MetricsRegistry::new();
        registry.register(Arc::new(TestProvider {
            name: "test".to_string(),
            metrics: "metric 1\n".to_string(),
        }));
        assert_eq!(registry.len(), 1);

        registry.unregister("test");
        assert!(registry.is_empty());
        assert_eq!(registry.gather_all(), "");
    }

    #[test]
    fn test_registry_provider_names() {
        let registry = MetricsRegistry::new();
        registry.register(Arc::new(TestProvider {
            name: "a".to_string(),
            metrics: "".to_string(),
        }));
        registry.register(Arc::new(TestProvider {
            name: "b".to_string(),
            metrics: "".to_string(),
        }));

        let names = registry.provider_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
    }

    #[test]
    fn test_registry_empty_output() {
        let registry = MetricsRegistry::new();
        assert_eq!(registry.gather_all(), "");
    }

    #[test]
    fn test_registry_trailing_newline_added() {
        let registry = MetricsRegistry::new();
        registry.register(Arc::new(TestProvider {
            name: "test".to_string(),
            metrics: "metric 1".to_string(), // 没有换行
        }));
        let output = registry.gather_all();
        assert!(output.ends_with('\n'));
    }
}

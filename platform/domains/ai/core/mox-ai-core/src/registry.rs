//! AI Provider 注册表 — 运行时动态注册，新增Provider零改动核心

use crate::providers::dto::Capability;
use crate::providers::error::{AiError, AiResult};
use crate::providers::traits::AiProvider;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Provider注册表
///
/// 所有AI Provider在此注册，路由器从注册表查找可用Provider。
/// 新增Provider只需调用 `registry.register(Arc::new(MyProvider::new(...)))`。
pub struct ProviderRegistry {
    providers: RwLock<HashMap<String, Arc<dyn AiProvider>>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { providers: RwLock::new(HashMap::new()) }
    }

    /// 注册Provider（覆盖同ID）
    pub fn register(&self, provider: Arc<dyn AiProvider>) {
        let id = provider.provider_id().to_string();
        tracing::info!("register AI provider: {} ({})", provider.provider_name(), id);
        self.providers.write().insert(id, provider);
    }

    /// 注销Provider
    pub fn unregister(&self, provider_id: &str) -> Option<Arc<dyn AiProvider>> {
        self.providers.write().remove(provider_id)
    }

    /// 获取Provider
    pub fn get(&self, provider_id: &str) -> AiResult<Arc<dyn AiProvider>> {
        self.providers.read()
            .get(provider_id)
            .cloned()
            .ok_or_else(|| AiError::ProviderNotFound(provider_id.into()))
    }

    /// 列出所有已注册Provider
    pub fn list(&self) -> Vec<Arc<dyn AiProvider>> {
        self.providers.read().values().cloned().collect()
    }

    /// 按能力筛选Provider
    pub fn list_by_capability(&self, cap: Capability) -> Vec<Arc<dyn AiProvider>> {
        self.providers.read()
            .values()
            .filter(|p| p.supports(cap))
            .cloned()
            .collect()
    }

    /// 检查Provider是否已注册
    pub fn contains(&self, provider_id: &str) -> bool {
        self.providers.read().contains_key(provider_id)
    }

    /// 已注册Provider数量
    pub fn len(&self) -> usize {
        self.providers.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.read().is_empty()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider { id: &'static str, name: &'static str }
    #[async_trait::async_trait]
    impl AiProvider for MockProvider {
        fn provider_id(&self) -> &'static str { self.id }
        fn provider_name(&self) -> &'static str { self.name }
        fn capabilities(&self) -> Vec<Capability> { vec![Capability::Chat] }
        fn available_models(&self) -> Vec<String> { vec!["mock".into()] }
        async fn chat(&self, _req: &crate::providers::dto::ChatRequest) -> AiResult<crate::providers::dto::ChatResponse> {
            Err(AiError::Other("mock".into()))
        }
        async fn chat_stream(&self, _req: &crate::providers::dto::ChatRequest) -> AiResult<futures::stream::BoxStream<'_, AiResult<crate::providers::dto::StreamChunk>>> {
            Err(AiError::Other("mock".into()))
        }
        async fn health_check(&self) -> crate::providers::dto::HealthStatus {
            crate::providers::dto::HealthStatus::Healthy
        }
    }

    #[test]
    fn test_register_and_get() {
        let registry = ProviderRegistry::new();
        let p = Arc::new(MockProvider { id: "mock", name: "Mock" });
        registry.register(p.clone());
        assert!(registry.contains("mock"));
        assert_eq!(registry.len(), 1);
        let got = registry.get("mock").unwrap();
        assert_eq!(got.provider_id(), "mock");
    }

    #[test]
    fn test_unregister() {
        let registry = ProviderRegistry::new();
        let p = Arc::new(MockProvider { id: "mock", name: "Mock" });
        registry.register(p);
        assert!(registry.unregister("mock").is_some());
        assert!(!registry.contains("mock"));
    }

    #[test]
    fn test_list_by_capability() {
        let registry = ProviderRegistry::new();
        let p = Arc::new(MockProvider { id: "mock", name: "Mock" });
        registry.register(p);
        let chat_providers = registry.list_by_capability(Capability::Chat);
        assert_eq!(chat_providers.len(), 1);
        let embed_providers = registry.list_by_capability(Capability::Embedding);
        assert_eq!(embed_providers.len(), 0);
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 统一架构核心 (Unified Architecture Core)
// Licensed under the MIT License.

//! 连接器框架
//!
//! 第三方系统对接的标准框架：
//! - 统一连接器 Trait
//! - 连接器注册中心
//! - 连接配置管理
//! - 健康检查

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{ArchError, ArchResult};
use crate::types::{ConnectorCategory, ConnectorInfo, UnifiedResource, now_ms};

/// 连接器配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorConfig {
    /// 配置键值对
    pub values: HashMap<String, String>,
    /// 是否加密存储
    pub encrypted: bool,
    /// 最后更新时间
    pub updated_at: u64,
}

impl ConnectorConfig {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            encrypted: false,
            updated_at: now_ms(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
        self.updated_at = now_ms();
    }
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// 连接器操作结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorOperationResult {
    /// 是否成功
    pub success: bool,
    /// 结果数据
    pub data: Option<serde_json::Value>,
    /// 错误消息
    pub error: Option<String>,
    /// 耗时（毫秒）
    pub duration_ms: u64,
}

/// 连接器 Trait
///
/// 所有第三方系统连接器都需要实现这个 trait，
/// 提供统一的操作接口。
#[async_trait]
pub trait Connector: Send + Sync {
    /// 连接器信息
    fn info(&self) -> &ConnectorInfo;

    /// 初始化连接器
    async fn initialize(&self, config: ConnectorConfig) -> ArchResult<()>;

    /// 测试连接
    async fn test_connection(&self) -> ArchResult<bool>;

    /// 执行操作
    async fn execute(
        &self,
        operation: &str,
        params: serde_json::Value,
    ) -> ArchResult<ConnectorOperationResult>;

    /// 列出资源
    async fn list_resources(
        &self,
        resource_type: &str,
        limit: Option<usize>,
        cursor: Option<String>,
    ) -> ArchResult<Vec<UnifiedResource>>;

    /// 获取资源
    async fn get_resource(&self, resource_type: &str, resource_id: &str) -> ArchResult<UnifiedResource>;

    /// 创建资源
    async fn create_resource(
        &self,
        resource_type: &str,
        properties: serde_json::Value,
    ) -> ArchResult<UnifiedResource>;

    /// 更新资源
    async fn update_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
        properties: serde_json::Value,
    ) -> ArchResult<UnifiedResource>;

    /// 删除资源
    async fn delete_resource(&self, resource_type: &str, resource_id: &str) -> ArchResult<bool>;

    /// 健康检查
    async fn health_check(&self) -> ArchResult<ConnectorHealth>;

    /// 支持的资源类型
    fn supported_resource_types(&self) -> Vec<String>;

    /// 支持的操作
    fn supported_operations(&self) -> Vec<String>;
}

/// 连接器健康状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorHealth {
    /// 是否健康
    pub healthy: bool,
    /// 状态描述
    pub status: String,
    /// 延迟（毫秒）
    pub latency_ms: Option<u64>,
    /// 最后检查时间
    pub last_check: u64,
    /// 详细信息
    pub details: HashMap<String, String>,
}

impl ConnectorHealth {
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            status: "healthy".to_string(),
            latency_ms: None,
            last_check: now_ms(),
            details: HashMap::new(),
        }
    }

    pub fn unhealthy(reason: &str) -> Self {
        Self {
            healthy: false,
            status: reason.to_string(),
            latency_ms: None,
            last_check: now_ms(),
            details: HashMap::new(),
        }
    }
}

/// 连接器实例
struct ConnectorInstance {
    connector: Arc<dyn Connector>,
    config: ConnectorConfig,
    initialized: bool,
}

/// 连接器注册中心
///
/// 管理所有已注册的第三方系统连接器，
/// 提供连接器的发现、配置、健康检查等能力。
pub struct ConnectorRegistry {
    connectors: RwLock<HashMap<String, ConnectorInstance>>,
    categories: RwLock<HashMap<ConnectorCategory, Vec<String>>>,
}

impl ConnectorRegistry {
    /// 创建连接器注册中心
    pub fn new() -> Self {
        Self {
            connectors: RwLock::new(HashMap::new()),
            categories: RwLock::new(HashMap::new()),
        }
    }

    /// 注册连接器
    pub async fn register(
        &self,
        connector: Arc<dyn Connector>,
        config: ConnectorConfig,
    ) -> ArchResult<()> {
        let info = connector.info();
        let id = info.id.clone();
        let category = info.category;

        if self.connectors.read().contains_key(&id) {
            return Err(ArchError::AlreadyExists(format!(
                "connector '{}' already registered",
                id
            )));
        }

        // 初始化连接器
        connector.initialize(config.clone()).await?;

        let instance = ConnectorInstance {
            connector,
            config,
            initialized: true,
        };

        self.connectors.write().insert(id.clone(), instance);
        self.categories
            .write()
            .entry(category)
            .or_default()
            .push(id);

        Ok(())
    }

    /// 获取连接器
    pub fn get(&self, id: &str) -> ArchResult<Arc<dyn Connector>> {
        self.connectors
            .read()
            .get(id)
            .map(|inst| inst.connector.clone())
            .ok_or_else(|| ArchError::ConnectorNotFound(id.to_string()))
    }

    /// 检查连接器是否存在
    pub fn exists(&self, id: &str) -> bool {
        self.connectors.read().contains_key(id)
    }

    /// 获取连接器信息
    pub fn get_info(&self, id: &str) -> ArchResult<ConnectorInfo> {
        Ok(self.get(id)?.info().clone())
    }

    /// 按类别列出连接器
    pub fn list_by_category(&self, category: ConnectorCategory) -> Vec<ConnectorInfo> {
        let categories = self.categories.read();
        let connectors = self.connectors.read();

        categories
            .get(&category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| connectors.get(id).map(|inst| inst.connector.info().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 列出所有连接器
    pub fn list_all(&self) -> Vec<ConnectorInfo> {
        self.connectors
            .read()
            .values()
            .map(|inst| inst.connector.info().clone())
            .collect()
    }

    /// 取消注册
    pub fn unregister(&self, id: &str) -> ArchResult<bool> {
        let mut connectors = self.connectors.write();
        if let Some(instance) = connectors.remove(id) {
            let category = instance.connector.info().category;
            let mut categories = self.categories.write();
            if let Some(ids) = categories.get_mut(&category) {
                ids.retain(|x| x != id);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 执行连接器操作
    pub async fn execute(
        &self,
        connector_id: &str,
        operation: &str,
        params: serde_json::Value,
    ) -> ArchResult<ConnectorOperationResult> {
        let connector = self.get(connector_id)?;
        connector.execute(operation, params).await
    }

    /// 健康检查所有连接器
    pub async fn health_check_all(&self) -> HashMap<String, ConnectorHealth> {
        let connectors = self
            .connectors
            .read()
            .iter()
            .map(|(id, inst)| (id.clone(), inst.connector.clone()))
            .collect::<Vec<_>>();

        let mut results = HashMap::new();
        for (id, connector) in connectors {
            let health = connector
                .health_check()
                .await
                .unwrap_or_else(|e| ConnectorHealth::unhealthy(&e.to_string()));
            results.insert(id, health);
        }

        results
    }

    /// 连接器总数
    pub fn count(&self) -> usize {
        self.connectors.read().len()
    }

    /// 更新连接器配置
    pub async fn update_config(
        &self,
        connector_id: &str,
        config: ConnectorConfig,
    ) -> ArchResult<()> {
        let connector = self.get(connector_id)?;
        connector.initialize(config.clone()).await?;

        if let Some(inst) = self.connectors.write().get_mut(connector_id) {
            inst.config = config;
        }

        Ok(())
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockConnector {
        info: ConnectorInfo,
    }

    impl MockConnector {
        fn new(id: &str, name: &str, category: ConnectorCategory) -> Self {
            Self {
                info: ConnectorInfo {
                    id: id.to_string(),
                    name: name.to_string(),
                    category,
                    version: "1.0.0".to_string(),
                    description: "Mock connector".to_string(),
                    vendor: "Test".to_string(),
                    icon_url: None,
                    operations: vec!["test".to_string()],
                    config_schema: serde_json::json!({}),
                    enabled: true,
                    auth_type: "none".to_string(),
                    tags: vec![],
                },
            }
        }
    }

    #[async_trait]
    impl Connector for MockConnector {
        fn info(&self) -> &ConnectorInfo {
            &self.info
        }

        async fn initialize(&self, _config: ConnectorConfig) -> ArchResult<()> {
            Ok(())
        }

        async fn test_connection(&self) -> ArchResult<bool> {
            Ok(true)
        }

        async fn execute(
            &self,
            _operation: &str,
            _params: serde_json::Value,
        ) -> ArchResult<ConnectorOperationResult> {
            Ok(ConnectorOperationResult {
                success: true,
                data: Some(serde_json::json!({ "result": "ok" })),
                error: None,
                duration_ms: 10,
            })
        }

        async fn list_resources(
            &self,
            _resource_type: &str,
            _limit: Option<usize>,
            _cursor: Option<String>,
        ) -> ArchResult<Vec<UnifiedResource>> {
            Ok(Vec::new())
        }

        async fn get_resource(
            &self,
            _resource_type: &str,
            resource_id: &str,
        ) -> ArchResult<UnifiedResource> {
            Ok(UnifiedResource {
                id: resource_id.to_string(),
                resource_type: "test".to_string(),
                name: "Test Resource".to_string(),
                connector_id: self.info.id.clone(),
                external_id: resource_id.to_string(),
                properties: HashMap::new(),
                status: "active".to_string(),
                created_at: now_ms(),
                updated_at: now_ms(),
                supported_operations: vec![],
            })
        }

        async fn create_resource(
            &self,
            _resource_type: &str,
            _properties: serde_json::Value,
        ) -> ArchResult<UnifiedResource> {
            Err(ArchError::UnsupportedOperation("create".to_string()))
        }

        async fn update_resource(
            &self,
            _resource_type: &str,
            _resource_id: &str,
            _properties: serde_json::Value,
        ) -> ArchResult<UnifiedResource> {
            Err(ArchError::UnsupportedOperation("update".to_string()))
        }

        async fn delete_resource(
            &self,
            _resource_type: &str,
            _resource_id: &str,
        ) -> ArchResult<bool> {
            Ok(true)
        }

        async fn health_check(&self) -> ArchResult<ConnectorHealth> {
            Ok(ConnectorHealth::healthy())
        }

        fn supported_resource_types(&self) -> Vec<String> {
            vec!["test".to_string()]
        }

        fn supported_operations(&self) -> Vec<String> {
            vec!["test".to_string()]
        }
    }

    #[tokio::test]
    async fn test_connector_registry() {
        let registry = ConnectorRegistry::new();

        let connector = Arc::new(MockConnector::new("test-1", "Test Connector", ConnectorCategory::DataSource));
        registry
            .register(connector.clone(), ConnectorConfig::new())
            .await
            .unwrap();

        assert_eq!(registry.count(), 1);
        assert!(registry.exists("test-1"));

        let info = registry.get_info("test-1").unwrap();
        assert_eq!(info.name, "Test Connector");
    }

    #[tokio::test]
    async fn test_connector_execute() {
        let registry = ConnectorRegistry::new();
        let connector = Arc::new(MockConnector::new("exec-1", "Exec", ConnectorCategory::AiService));
        registry
            .register(connector, ConnectorConfig::new())
            .await
            .unwrap();

        let result = registry
            .execute("exec-1", "test", serde_json::json!({}))
            .await
            .unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_list_by_category() {
        let registry = ConnectorRegistry::new();

        registry
            .register(
                Arc::new(MockConnector::new("ds1", "DS1", ConnectorCategory::DataSource)),
                ConnectorConfig::new(),
            )
            .await
            .unwrap();
        registry
            .register(
                Arc::new(MockConnector::new("ai1", "AI1", ConnectorCategory::AiService)),
                ConnectorConfig::new(),
            )
            .await
            .unwrap();

        let ds = registry.list_by_category(ConnectorCategory::DataSource);
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].id, "ds1");
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = ConnectorRegistry::new();
        registry
            .register(
                Arc::new(MockConnector::new("rm1", "RemoveMe", ConnectorCategory::Other)),
                ConnectorConfig::new(),
            )
            .await
            .unwrap();

        assert!(registry.unregister("rm1").unwrap());
        assert!(!registry.exists("rm1"));
    }

    #[tokio::test]
    async fn test_health_check() {
        let registry = ConnectorRegistry::new();
        registry
            .register(
                Arc::new(MockConnector::new("hc1", "HC", ConnectorCategory::Storage)),
                ConnectorConfig::new(),
            )
            .await
            .unwrap();

        let health = registry.health_check_all().await;
        assert_eq!(health.len(), 1);
        assert!(health.get("hc1").unwrap().healthy);
    }
}

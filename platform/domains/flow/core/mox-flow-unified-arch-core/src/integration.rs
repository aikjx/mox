// Copyright (c) 2026 璇玑 RelGraph · 统一架构核心 (Unified Architecture Core)
// Licensed under the MIT License.

//! 集成管理器
//!
//! 管理第三方系统集成的全生命周期：
//! - 集成配置管理
//! - 数据同步任务
//! - Webhook 回调
//! - 事件订阅
//! - 集成状态监控

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::connector::{ConnectorConfig, ConnectorRegistry};
use crate::error::{ArchError, ArchResult};
use crate::types::{ConnectorCategory, now_ms};

/// 集成状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IntegrationStatus {
    /// 未配置
    NotConfigured,
    /// 已配置但未启用
    Configured,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 错误
    Error,
    /// 已断开
    Disconnected,
}

impl IntegrationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IntegrationStatus::NotConfigured => "not_configured",
            IntegrationStatus::Configured => "configured",
            IntegrationStatus::Running => "running",
            IntegrationStatus::Paused => "paused",
            IntegrationStatus::Error => "error",
            IntegrationStatus::Disconnected => "disconnected",
        }
    }
}

/// 同步方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SyncDirection {
    /// 从外部系统同步到平台
    Inbound,
    /// 从平台同步到外部系统
    Outbound,
    /// 双向同步
    Bidirectional,
}

/// 集成配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IntegrationConfig {
    /// 集成 ID
    pub id: String,
    /// 集成名称
    pub name: String,
    /// 关联的连接器 ID
    pub connector_id: String,
    /// 连接器类别
    pub category: ConnectorCategory,
    /// 集成状态
    pub status: IntegrationStatus,
    /// 连接配置
    pub config: ConnectorConfig,
    /// 同步方向
    pub sync_direction: SyncDirection,
    /// 同步调度（cron 表达式）
    pub sync_schedule: Option<String>,
    /// 启用 Webhook
    pub webhook_enabled: bool,
    /// Webhook URL
    pub webhook_url: Option<String>,
    /// 订阅的事件类型
    pub subscribed_events: Vec<String>,
    /// 自定义配置
    pub custom_settings: HashMap<String, String>,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
    /// 最后同步时间
    pub last_sync_at: Option<u64>,
    /// 错误消息
    pub error_message: Option<String>,
}

impl IntegrationConfig {
    /// 创建新的集成配置
    pub fn new(id: &str, name: &str, connector_id: &str, category: ConnectorCategory) -> Self {
        let now = now_ms();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            connector_id: connector_id.to_string(),
            category,
            status: IntegrationStatus::NotConfigured,
            config: ConnectorConfig::new(),
            sync_direction: SyncDirection::Inbound,
            sync_schedule: None,
            webhook_enabled: false,
            webhook_url: None,
            subscribed_events: Vec::new(),
            custom_settings: HashMap::new(),
            created_at: now,
            updated_at: now,
            last_sync_at: None,
            error_message: None,
        }
    }

    /// 设置配置
    pub fn with_config(mut self, config: ConnectorConfig) -> Self {
        self.config = config;
        self.status = IntegrationStatus::Configured;
        self
    }

    /// 启用 Webhook
    pub fn with_webhook(mut self, url: &str) -> Self {
        self.webhook_enabled = true;
        self.webhook_url = Some(url.to_string());
        self
    }

    /// 设置同步调度
    pub fn with_schedule(mut self, cron: &str) -> Self {
        self.sync_schedule = Some(cron.to_string());
        self
    }

    /// 订阅事件
    pub fn subscribe_event(&mut self, event: &str) {
        if !self.subscribed_events.iter().any(|e| e == event) {
            self.subscribed_events.push(event.to_string());
        }
    }

    /// 取消订阅
    pub fn unsubscribe_event(&mut self, event: &str) {
        self.subscribed_events.retain(|e| e != event);
    }
}

/// 集成统计
#[derive(Debug, Clone, Default)]
pub struct IntegrationStats {
    /// 总集成数
    pub total: usize,
    /// 运行中
    pub running: usize,
    /// 已暂停
    pub paused: usize,
    /// 错误状态
    pub error: usize,
    /// 未配置
    pub not_configured: usize,
    /// 按类别统计
    pub by_category: HashMap<ConnectorCategory, usize>,
}

/// 集成管理器
///
/// 管理所有第三方系统集成的配置、运行和监控。
pub struct IntegrationManager {
    /// 集成配置表
    integrations: RwLock<HashMap<String, IntegrationConfig>>,
    /// 连接器注册中心引用
    connectors: Arc<ConnectorRegistry>,
    /// 集成事件回调
    event_handlers: RwLock<Vec<Arc<dyn Fn(IntegrationEvent) + Send + Sync>>>,
}

/// 集成事件
#[derive(Debug, Clone)]
pub enum IntegrationEvent {
    /// 集成已创建
    Created { id: String, connector_id: String },
    /// 集成已启用
    Enabled { id: String },
    /// 集成已禁用
    Disabled { id: String },
    /// 同步开始
    SyncStarted { id: String },
    /// 同步完成
    SyncCompleted { id: String, records: u64 },
    /// 同步失败
    SyncFailed { id: String, error: String },
    /// 配置已更新
    ConfigUpdated { id: String },
}

impl IntegrationManager {
    /// 创建集成管理器
    pub fn new(connectors: Arc<ConnectorRegistry>) -> Self {
        Self {
            integrations: RwLock::new(HashMap::new()),
            connectors,
            event_handlers: RwLock::new(Vec::new()),
        }
    }

    /// 创建集成
    pub async fn create_integration(
        &self,
        id: &str,
        name: &str,
        connector_id: &str,
        config: ConnectorConfig,
    ) -> ArchResult<IntegrationConfig> {
        // 检查连接器是否存在
        if !self.connectors.exists(connector_id) {
            return Err(ArchError::ConnectorNotFound(connector_id.to_string()));
        }

        // 检查集成是否已存在
        if self.integrations.read().contains_key(id) {
            return Err(ArchError::AlreadyExists(format!(
                "integration '{}' already exists",
                id
            )));
        }

        let info = self.connectors.get_info(connector_id)?;
        let mut integration =
            IntegrationConfig::new(id, name, connector_id, info.category).with_config(config);

        // 测试连接
        let connector = self.connectors.get(connector_id)?;
        match connector.test_connection().await {
            Ok(true) => {
                integration.status = IntegrationStatus::Running;
            }
            Ok(false) => {
                integration.status = IntegrationStatus::Error;
                integration.error_message = Some("connection test failed".to_string());
            }
            Err(e) => {
                integration.status = IntegrationStatus::Error;
                integration.error_message = Some(e.to_string());
            }
        }

        integration.updated_at = now_ms();

        self.integrations
            .write()
            .insert(id.to_string(), integration.clone());

        self.emit_event(IntegrationEvent::Created {
            id: id.to_string(),
            connector_id: connector_id.to_string(),
        });

        Ok(integration)
    }

    /// 获取集成配置
    pub fn get_integration(&self, id: &str) -> ArchResult<IntegrationConfig> {
        self.integrations
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| ArchError::NotFound(format!("integration '{}' not found", id)))
    }

    /// 检查集成是否存在
    pub fn integration_exists(&self, id: &str) -> bool {
        self.integrations.read().contains_key(id)
    }

    /// 列出所有集成
    pub fn list_integrations(&self) -> Vec<IntegrationConfig> {
        self.integrations.read().values().cloned().collect()
    }

    /// 按类别列出集成
    pub fn list_by_category(&self, category: ConnectorCategory) -> Vec<IntegrationConfig> {
        self.integrations
            .read()
            .values()
            .filter(|i| i.category == category)
            .cloned()
            .collect()
    }

    /// 更新集成配置
    pub fn update_integration(
        &self,
        id: &str,
        mut update: IntegrationConfig,
    ) -> ArchResult<IntegrationConfig> {
        let mut integrations = self.integrations.write();
        let existing = integrations
            .get_mut(id)
            .ok_or_else(|| ArchError::NotFound(format!("integration '{}' not found", id)))?;

        update.id = id.to_string();
        update.updated_at = now_ms();
        *existing = update.clone();

        self.emit_event(IntegrationEvent::ConfigUpdated { id: id.to_string() });

        Ok(update)
    }

    /// 启用集成
    pub async fn enable_integration(&self, id: &str) -> ArchResult<()> {
        let mut integrations = self.integrations.write();
        let integration = integrations
            .get_mut(id)
            .ok_or_else(|| ArchError::NotFound(format!("integration '{}' not found", id)))?;

        integration.status = IntegrationStatus::Running;
        integration.error_message = None;
        integration.updated_at = now_ms();

        drop(integrations);
        self.emit_event(IntegrationEvent::Enabled { id: id.to_string() });

        Ok(())
    }

    /// 禁用集成
    pub fn disable_integration(&self, id: &str) -> ArchResult<()> {
        let mut integrations = self.integrations.write();
        let integration = integrations
            .get_mut(id)
            .ok_or_else(|| ArchError::NotFound(format!("integration '{}' not found", id)))?;

        integration.status = IntegrationStatus::Paused;
        integration.updated_at = now_ms();

        drop(integrations);
        self.emit_event(IntegrationEvent::Disabled { id: id.to_string() });

        Ok(())
    }

    /// 删除集成
    pub fn delete_integration(&self, id: &str) -> ArchResult<bool> {
        Ok(self.integrations.write().remove(id).is_some())
    }

    /// 记录同步结果
    pub fn record_sync_result(&self, id: &str, success: bool, records: u64, error: Option<&str>) {
        let mut integrations = self.integrations.write();
        if let Some(integration) = integrations.get_mut(id) {
            integration.last_sync_at = Some(now_ms());
            if success {
                integration.status = IntegrationStatus::Running;
                integration.error_message = None;
                self.emit_event(IntegrationEvent::SyncCompleted {
                    id: id.to_string(),
                    records,
                });
            } else {
                integration.status = IntegrationStatus::Error;
                integration.error_message = error.map(|e| e.to_string());
                self.emit_event(IntegrationEvent::SyncFailed {
                    id: id.to_string(),
                    error: error.unwrap_or("unknown error").to_string(),
                });
            }
            integration.updated_at = now_ms();
        }
    }

    /// 获取统计信息
    pub fn stats(&self) -> IntegrationStats {
        let mut stats = IntegrationStats::default();
        let integrations = self.integrations.read();

        for integration in integrations.values() {
            stats.total += 1;
            match integration.status {
                IntegrationStatus::Running => stats.running += 1,
                IntegrationStatus::Paused => stats.paused += 1,
                IntegrationStatus::Error => stats.error += 1,
                IntegrationStatus::NotConfigured => stats.not_configured += 1,
                _ => {}
            }
            *stats.by_category.entry(integration.category).or_insert(0) += 1;
        }

        stats
    }

    /// 注册事件处理器
    pub fn on_event<F>(&self, handler: F)
    where
        F: Fn(IntegrationEvent) + Send + Sync + 'static,
    {
        self.event_handlers.write().push(Arc::new(handler));
    }

    /// 触发事件
    fn emit_event(&self, event: IntegrationEvent) {
        let handlers = self.event_handlers.read().clone();
        for handler in handlers {
            handler(event.clone());
        }
    }

    /// 集成总数
    pub fn count(&self) -> usize {
        self.integrations.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connector::{Connector, ConnectorHealth, ConnectorOperationResult};
    use crate::types::ConnectorInfo;
    use async_trait::async_trait;

    struct TestConnector {
        info: ConnectorInfo,
    }

    impl TestConnector {
        fn new() -> Self {
            Self {
                info: ConnectorInfo {
                    id: "test-conn".to_string(),
                    name: "Test Connector".to_string(),
                    category: ConnectorCategory::DataSource,
                    version: "1.0".to_string(),
                    description: "Test".to_string(),
                    vendor: "Test".to_string(),
                    icon_url: None,
                    operations: vec![],
                    config_schema: serde_json::json!({}),
                    enabled: true,
                    auth_type: "none".to_string(),
                    tags: vec![],
                },
            }
        }
    }

    #[async_trait]
    impl Connector for TestConnector {
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
            Err(ArchError::UnsupportedOperation("not implemented".to_string()))
        }

        async fn list_resources(
            &self,
            _resource_type: &str,
            _limit: Option<usize>,
            _cursor: Option<String>,
        ) -> ArchResult<Vec<crate::types::UnifiedResource>> {
            Ok(vec![])
        }

        async fn get_resource(
            &self,
            _resource_type: &str,
            _resource_id: &str,
        ) -> ArchResult<crate::types::UnifiedResource> {
            Err(ArchError::NotFound("not found".to_string()))
        }

        async fn create_resource(
            &self,
            _resource_type: &str,
            _properties: serde_json::Value,
        ) -> ArchResult<crate::types::UnifiedResource> {
            Err(ArchError::UnsupportedOperation("create".to_string()))
        }

        async fn update_resource(
            &self,
            _resource_type: &str,
            _resource_id: &str,
            _properties: serde_json::Value,
        ) -> ArchResult<crate::types::UnifiedResource> {
            Err(ArchError::UnsupportedOperation("update".to_string()))
        }

        async fn delete_resource(
            &self,
            _resource_type: &str,
            _resource_id: &str,
        ) -> ArchResult<bool> {
            Ok(false)
        }

        async fn health_check(&self) -> ArchResult<ConnectorHealth> {
            Ok(ConnectorHealth::healthy())
        }

        fn supported_resource_types(&self) -> Vec<String> {
            vec![]
        }

        fn supported_operations(&self) -> Vec<String> {
            vec![]
        }
    }

    #[tokio::test]
    async fn test_create_integration() {
        let connectors = Arc::new(ConnectorRegistry::new());
        connectors
            .register(Arc::new(TestConnector::new()), ConnectorConfig::new())
            .await
            .unwrap();

        let manager = IntegrationManager::new(connectors);
        let config = ConnectorConfig::new();

        let integration = manager
            .create_integration("int-1", "My Integration", "test-conn", config)
            .await
            .unwrap();

        assert_eq!(integration.name, "My Integration");
        assert_eq!(integration.status, IntegrationStatus::Running);
        assert_eq!(manager.count(), 1);
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let connectors = Arc::new(ConnectorRegistry::new());
        connectors
            .register(Arc::new(TestConnector::new()), ConnectorConfig::new())
            .await
            .unwrap();

        let manager = IntegrationManager::new(connectors);
        manager
            .create_integration("int-1", "Test", "test-conn", ConnectorConfig::new())
            .await
            .unwrap();

        manager.disable_integration("int-1").unwrap();
        let integration = manager.get_integration("int-1").unwrap();
        assert_eq!(integration.status, IntegrationStatus::Paused);

        manager.enable_integration("int-1").await.unwrap();
        let integration = manager.get_integration("int-1").unwrap();
        assert_eq!(integration.status, IntegrationStatus::Running);
    }

    #[tokio::test]
    async fn test_stats() {
        let connectors = Arc::new(ConnectorRegistry::new());
        connectors
            .register(Arc::new(TestConnector::new()), ConnectorConfig::new())
            .await
            .unwrap();

        let manager = IntegrationManager::new(connectors);

        manager
            .create_integration("int-1", "One", "test-conn", ConnectorConfig::new())
            .await
            .unwrap();
        manager
            .create_integration("int-2", "Two", "test-conn", ConnectorConfig::new())
            .await
            .unwrap();

        manager.disable_integration("int-2").unwrap();

        let stats = manager.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.running, 1);
        assert_eq!(stats.paused, 1);
    }

    #[tokio::test]
    async fn test_record_sync_result() {
        let connectors = Arc::new(ConnectorRegistry::new());
        connectors
            .register(Arc::new(TestConnector::new()), ConnectorConfig::new())
            .await
            .unwrap();

        let manager = IntegrationManager::new(connectors);
        manager
            .create_integration("int-1", "Test", "test-conn", ConnectorConfig::new())
            .await
            .unwrap();

        manager.record_sync_result("int-1", true, 100, None);

        let integration = manager.get_integration("int-1").unwrap();
        assert_eq!(integration.status, IntegrationStatus::Running);
        assert!(integration.last_sync_at.is_some());
    }

    #[tokio::test]
    async fn test_delete_integration() {
        let connectors = Arc::new(ConnectorRegistry::new());
        connectors
            .register(Arc::new(TestConnector::new()), ConnectorConfig::new())
            .await
            .unwrap();

        let manager = IntegrationManager::new(connectors);
        manager
            .create_integration("int-1", "Test", "test-conn", ConnectorConfig::new())
            .await
            .unwrap();

        assert!(manager.delete_integration("int-1").unwrap());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_integration_status() {
        assert_eq!(IntegrationStatus::Running.as_str(), "running");
        assert_eq!(IntegrationStatus::Error.as_str(), "error");
    }
}

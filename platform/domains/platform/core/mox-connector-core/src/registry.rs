// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 连接器注册表 — 管理所有已注册的连接器

use crate::traits::{Connector, ConnectorError, ConnectorResult, ConnectorType};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// 连接器注册表
pub struct ConnectorRegistry {
    connectors: RwLock<HashMap<String, Arc<dyn Connector>>>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self { connectors: RwLock::new(HashMap::new()) }
    }

    /// 注册连接器
    pub fn register(&self, connector: Arc<dyn Connector>) {
        let id = connector.connector_id().to_string();
        tracing::info!("register connector: {} ({})", connector.connector_name(), id);
        self.connectors.write().insert(id, connector);
    }

    /// 注销连接器
    pub fn unregister(&self, connector_id: &str) -> Option<Arc<dyn Connector>> {
        self.connectors.write().remove(connector_id)
    }

    /// 获取连接器
    pub fn get(&self, connector_id: &str) -> ConnectorResult<Arc<dyn Connector>> {
        self.connectors.read()
            .get(connector_id)
            .cloned()
            .ok_or_else(|| ConnectorError::NotFound(connector_id.into()))
    }

    /// 列出所有连接器
    pub fn list(&self) -> Vec<Arc<dyn Connector>> {
        self.connectors.read().values().cloned().collect()
    }

    /// 按类型筛选
    pub fn list_by_type(&self, connector_type: ConnectorType) -> Vec<Arc<dyn Connector>> {
        self.connectors.read()
            .values()
            .filter(|c| c.connector_type() == connector_type)
            .cloned()
            .collect()
    }

    /// 按协议筛选
    pub fn list_by_protocol(&self, protocol: &str) -> Vec<Arc<dyn Connector>> {
        self.connectors.read()
            .values()
            .filter(|c| c.supported_protocols().iter().any(|p| p == protocol))
            .cloned()
            .collect()
    }

    /// 检查连接器是否存在
    pub fn contains(&self, connector_id: &str) -> bool {
        self.connectors.read().contains_key(connector_id)
    }

    /// 连接器数量
    pub fn len(&self) -> usize {
        self.connectors.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.connectors.read().is_empty()
    }

    /// 全部健康检查
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();
        for connector in self.list() {
            let healthy = connector.health_check().await.unwrap_or(false);
            results.insert(connector.connector_id().to_string(), healthy);
        }
        results
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self { Self::new() }
}

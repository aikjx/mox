//! OA/ERP 连接器 trait 与具体适配器实现

use crate::integration::model::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 连接器 trait —— 所有 OA/ERP 适配器必须实现
#[async_trait]
pub trait OaErpConnector: Send + Sync {
    /// 连接器类型
    fn connector_type(&self) -> ConnectorType;

    /// 测试连接
    async fn test_connection(&self, config: &ConnectorConfig) -> ConnectorTestResult;

    /// 拉取组织架构数据
    async fn pull_organizations(&self, config: &ConnectorConfig, params: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String>;

    /// 拉取部门数据
    async fn pull_departments(&self, config: &ConnectorConfig, params: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String>;

    /// 拉取用户数据
    async fn pull_users(&self, config: &ConnectorConfig, params: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String>;

    /// 推送组织架构数据
    async fn push_organizations(&self, config: &ConnectorConfig, data: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String>;

    /// 推送部门数据
    async fn push_departments(&self, config: &ConnectorConfig, data: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String>;

    /// 推送用户数据
    async fn push_users(&self, config: &ConnectorConfig, data: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String>;

    /// 通用查询（自定义实体）
    async fn query(&self, config: &ConnectorConfig, entity: &str, params: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String>;
}

/// 同步统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncStats {
    pub total: i64,
    pub success: i64,
    pub failed: i64,
    pub skipped: i64,
}

/// 通用 REST API 连接器（基础实现，可被其他连接器继承）
pub struct GenericRestConnector {
    client: reqwest::Client,
}

impl GenericRestConnector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    fn build_headers(&self, auth: &HashMap<String, String>) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::CONTENT_TYPE, "application/json".parse().unwrap());
        if let Some(token) = auth.get("api_key") {
            if let Ok(v) = format!("Bearer {}", token).parse() {
                headers.insert(reqwest::header::AUTHORIZATION, v);
            }
        }
        if let Some(token) = auth.get("token") {
            if let Ok(v) = token.parse() {
                headers.insert("X-Auth-Token", v);
            }
        }
        headers
    }
}

#[async_trait]
impl OaErpConnector for GenericRestConnector {
    fn connector_type(&self) -> ConnectorType {
        ConnectorType::GenericRest
    }

    async fn test_connection(&self, config: &ConnectorConfig) -> ConnectorTestResult {
        let start = std::time::Instant::now();
        let url = config.connection.get("base_url").cloned().unwrap_or_default();
        if url.is_empty() {
            return ConnectorTestResult {
                success: false,
                response_time_ms: start.elapsed().as_millis() as i64,
                message: "base_url 未配置".to_string(),
                details: None,
            };
        }
        let test_url = format!("{}/{}", url.trim_end_matches('/'), config.connection.get("test_path").unwrap_or(&"health".to_string()));
        match self.client.get(&test_url).headers(self.build_headers(&config.auth)).send().await {
            Ok(resp) if resp.status().is_success() => ConnectorTestResult {
                success: true,
                response_time_ms: start.elapsed().as_millis() as i64,
                message: format!("连接成功，HTTP {}", resp.status()),
                details: None,
            },
            Ok(resp) => ConnectorTestResult {
                success: false,
                response_time_ms: start.elapsed().as_millis() as i64,
                message: format!("连接失败，HTTP {}", resp.status()),
                details: None,
            },
            Err(e) => ConnectorTestResult {
                success: false,
                response_time_ms: start.elapsed().as_millis() as i64,
                message: format!("连接异常：{}", e),
                details: None,
            },
        }
    }

    async fn pull_organizations(&self, _config: &ConnectorConfig, _params: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        Ok(vec![])
    }

    async fn pull_departments(&self, _config: &ConnectorConfig, _params: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        Ok(vec![])
    }

    async fn pull_users(&self, _config: &ConnectorConfig, _params: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        Ok(vec![])
    }

    async fn push_organizations(&self, _config: &ConnectorConfig, _data: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> {
        Ok(SyncStats::default())
    }

    async fn push_departments(&self, _config: &ConnectorConfig, _data: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> {
        Ok(SyncStats::default())
    }

    async fn push_users(&self, _config: &ConnectorConfig, _data: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> {
        Ok(SyncStats::default())
    }

    async fn query(&self, _config: &ConnectorConfig, _entity: &str, _params: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        Ok(vec![])
    }
}

/// SAP ERP 连接器
pub struct SapConnector {
    inner: GenericRestConnector,
}

impl SapConnector {
    pub fn new() -> Self {
        Self { inner: GenericRestConnector::new() }
    }
}

#[async_trait]
impl OaErpConnector for SapConnector {
    fn connector_type(&self) -> ConnectorType { ConnectorType::Sap }
    async fn test_connection(&self, config: &ConnectorConfig) -> ConnectorTestResult {
        let mut result = self.inner.test_connection(config).await;
        result.message = format!("[SAP] {}", result.message);
        result
    }
    async fn pull_organizations(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        self.inner.pull_organizations(c, p).await
    }
    async fn pull_departments(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        self.inner.pull_departments(c, p).await
    }
    async fn pull_users(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        self.inner.pull_users(c, p).await
    }
    async fn push_organizations(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> {
        self.inner.push_organizations(c, d).await
    }
    async fn push_departments(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> {
        self.inner.push_departments(c, d).await
    }
    async fn push_users(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> {
        self.inner.push_users(c, d).await
    }
    async fn query(&self, c: &ConnectorConfig, e: &str, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        self.inner.query(c, e, p).await
    }
}

/// Oracle ERP/HCM 连接器
pub struct OracleConnector { inner: GenericRestConnector }
impl OracleConnector { pub fn new() -> Self { Self { inner: GenericRestConnector::new() } } }
#[async_trait]
impl OaErpConnector for OracleConnector {
    fn connector_type(&self) -> ConnectorType { ConnectorType::Oracle }
    async fn test_connection(&self, config: &ConnectorConfig) -> ConnectorTestResult {
        let mut r = self.inner.test_connection(config).await;
        r.message = format!("[Oracle] {}", r.message);
        r
    }
    async fn pull_organizations(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.pull_organizations(c, p).await }
    async fn pull_departments(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.pull_departments(c, p).await }
    async fn pull_users(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.pull_users(c, p).await }
    async fn push_organizations(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> { self.inner.push_organizations(c, d).await }
    async fn push_departments(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> { self.inner.push_departments(c, d).await }
    async fn push_users(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> { self.inner.push_users(c, d).await }
    async fn query(&self, c: &ConnectorConfig, e: &str, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.query(c, e, p).await }
}

/// Workday HCM 连接器
pub struct WorkdayConnector { inner: GenericRestConnector }
impl WorkdayConnector { pub fn new() -> Self { Self { inner: GenericRestConnector::new() } } }
#[async_trait]
impl OaErpConnector for WorkdayConnector {
    fn connector_type(&self) -> ConnectorType { ConnectorType::Workday }
    async fn test_connection(&self, config: &ConnectorConfig) -> ConnectorTestResult {
        let mut r = self.inner.test_connection(config).await;
        r.message = format!("[Workday] {}", r.message);
        r
    }
    async fn pull_organizations(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.pull_organizations(c, p).await }
    async fn pull_departments(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.pull_departments(c, p).await }
    async fn pull_users(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.pull_users(c, p).await }
    async fn push_organizations(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> { self.inner.push_organizations(c, d).await }
    async fn push_departments(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> { self.inner.push_departments(c, d).await }
    async fn push_users(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> { self.inner.push_users(c, d).await }
    async fn query(&self, c: &ConnectorConfig, e: &str, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.query(c, e, p).await }
}

/// 北森 HR 连接器
pub struct BeisenConnector { inner: GenericRestConnector }
impl BeisenConnector { pub fn new() -> Self { Self { inner: GenericRestConnector::new() } } }
#[async_trait]
impl OaErpConnector for BeisenConnector {
    fn connector_type(&self) -> ConnectorType { ConnectorType::Beisen }
    async fn test_connection(&self, config: &ConnectorConfig) -> ConnectorTestResult {
        let mut r = self.inner.test_connection(config).await;
        r.message = format!("[北森] {}", r.message);
        r
    }
    async fn pull_organizations(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.pull_organizations(c, p).await }
    async fn pull_departments(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.pull_departments(c, p).await }
    async fn pull_users(&self, c: &ConnectorConfig, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.pull_users(c, p).await }
    async fn push_organizations(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> { self.inner.push_organizations(c, d).await }
    async fn push_departments(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> { self.inner.push_departments(c, d).await }
    async fn push_users(&self, c: &ConnectorConfig, d: &[HashMap<String, serde_json::Value>]) -> Result<SyncStats, String> { self.inner.push_users(c, d).await }
    async fn query(&self, c: &ConnectorConfig, e: &str, p: &HashMap<String, String>) -> Result<Vec<HashMap<String, serde_json::Value>>, String> { self.inner.query(c, e, p).await }
}

/// 连接器注册表 —— 管理所有连接器实例
pub struct ConnectorRegistry {
    connectors: RwLock<HashMap<ConnectorType, Arc<dyn OaErpConnector>>>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            connectors: RwLock::new(HashMap::new()),
        };
        registry.register(Arc::new(GenericRestConnector::new()));
        registry.register(Arc::new(SapConnector::new()));
        registry.register(Arc::new(OracleConnector::new()));
        registry.register(Arc::new(WorkdayConnector::new()));
        registry.register(Arc::new(BeisenConnector::new()));
        registry
    }

    pub fn register(&self, connector: Arc<dyn OaErpConnector>) {
        let ct = connector.connector_type();
        self.connectors.blocking_write().insert(ct, connector);
    }

    pub async fn get(&self, connector_type: &ConnectorType) -> Option<Arc<dyn OaErpConnector>> {
        self.connectors.read().await.get(connector_type).cloned()
    }

    pub async fn list_types(&self) -> Vec<ConnectorType> {
        self.connectors.read().await.keys().cloned().collect()
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

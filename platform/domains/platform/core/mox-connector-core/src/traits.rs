//! Connector 统一 trait — 所有第三方系统连接器实现此接口

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 连接器类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorType {
    /// 数据源（读取）
    Source,
    /// 数据汇（写入）
    Sink,
    /// 双向（读写）
    Both,
    /// API调用
    Api,
    /// 消息队列
    Queue,
    /// 数据库
    Database,
    /// 文件存储
    Storage,
    /// Webhook
    Webhook,
    /// 自定义
    Custom,
}

impl ConnectorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectorType::Source => "source",
            ConnectorType::Sink => "sink",
            ConnectorType::Both => "both",
            ConnectorType::Api => "api",
            ConnectorType::Queue => "queue",
            ConnectorType::Database => "database",
            ConnectorType::Storage => "storage",
            ConnectorType::Webhook => "webhook",
            ConnectorType::Custom => "custom",
        }
    }
}

/// 连接器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    /// 连接器唯一ID
    pub id: String,
    /// 连接器名称
    pub name: String,
    /// 连接器类型
    pub connector_type: ConnectorType,
    /// 协议（rest/grpc/websocket/soap/file）
    pub protocol: String,
    /// 端点URL
    pub endpoint: String,
    /// 认证方式（none/bearer/basic/api_key/oauth2）
    pub auth_type: String,
    /// 认证凭证
    #[serde(default)]
    pub credentials: HashMap<String, String>,
    /// 请求头
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// 超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// 重试次数
    #[serde(default)]
    pub max_retries: u32,
    /// 额外配置
    #[serde(default)]
    pub extra: HashMap<String, String>,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_timeout() -> u64 { 30 }
fn default_true() -> bool { true }

/// 连接器请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorRequest {
    /// 操作（如query/insert/update/delete/execute）
    pub operation: String,
    /// 请求体
    pub body: serde_json::Value,
    /// 请求参数
    #[serde(default)]
    pub params: HashMap<String, String>,
    /// 请求头（覆盖配置中的头）
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// 追踪ID
    #[serde(default)]
    pub trace_id: Option<String>,
    /// 租户ID
    #[serde(default)]
    pub tenant_id: Option<String>,
}

/// 连接器响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorResponse {
    /// 是否成功
    pub success: bool,
    /// 状态码
    pub status_code: u16,
    /// 响应体
    pub body: serde_json::Value,
    /// 响应头
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// 耗时（毫秒）
    pub latency_ms: u64,
    /// 错误信息（失败时）
    #[serde(default)]
    pub error: Option<String>,
    /// 重试次数
    #[serde(default)]
    pub retries: u32,
}

/// 连接器错误
#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("配置错误: {0}")]
    ConfigError(String),
    #[error("连接失败: {0}")]
    ConnectionError(String),
    #[error("认证失败: {0}")]
    AuthError(String),
    #[error("请求超时")]
    Timeout,
    #[error("请求失败: HTTP {status}: {message}")]
    RequestFailed { status: u16, message: String },
    #[error("响应解析失败: {0}")]
    ParseError(String),
    #[error("连接器未找到: {0}")]
    NotFound(String),
    #[error("连接器已禁用: {0}")]
    Disabled(String),
    #[error("不支持的操作: {0}")]
    UnsupportedOperation(String),
    #[error("重试耗尽: {0}")]
    RetryExhausted(String),
    #[error("其他错误: {0}")]
    Other(String),
}

impl From<reqwest::Error> for ConnectorError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            ConnectorError::Timeout
        } else if e.is_connect() {
            ConnectorError::ConnectionError(e.to_string())
        } else {
            ConnectorError::RequestFailed {
                status: e.status().map(|s| s.as_u16()).unwrap_or(0),
                message: e.to_string(),
            }
        }
    }
}

pub type ConnectorResult<T> = Result<T, ConnectorError>;

/// Connector 统一 trait
#[async_trait]
pub trait Connector: Send + Sync {
    /// 连接器唯一ID
    fn connector_id(&self) -> &str;
    /// 连接器名称
    fn connector_name(&self) -> &str;
    /// 连接器类型
    fn connector_type(&self) -> ConnectorType;
    /// 支持的协议
    fn supported_protocols(&self) -> Vec<String>;
    /// 支持的操作
    fn supported_operations(&self) -> Vec<String>;

    /// 初始化连接
    async fn connect(&self) -> ConnectorResult<()>;

    /// 执行请求
    async fn execute(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse>;

    /// 健康检查
    async fn health_check(&self) -> ConnectorResult<bool>;

    /// 关闭连接
    async fn close(&self) -> ConnectorResult<()>;

    /// 是否支持某操作
    fn supports_operation(&self, operation: &str) -> bool {
        self.supported_operations().iter().any(|o| o == operation)
    }

    /// 带重试的执行
    async fn execute_with_retry(&self, request: &ConnectorRequest, max_retries: u32) -> ConnectorResult<ConnectorResponse> {
        let mut last_error = None;
        for attempt in 0..=max_retries {
            match self.execute(request).await {
                Ok(mut resp) => {
                    resp.retries = attempt;
                    return Ok(resp);
                }
                Err(e) => {
                    tracing::warn!("connector {} attempt {}/{} failed: {}", self.connector_id(), attempt + 1, max_retries + 1, e);
                    last_error = Some(e);
                    if attempt < max_retries {
                        tokio::time::sleep(std::time::Duration::from_millis(100 * 2u64.pow(attempt))).await;
                    }
                }
            }
        }
        Err(ConnectorError::RetryExhausted(last_error.map(|e| e.to_string()).unwrap_or_default()))
    }
}

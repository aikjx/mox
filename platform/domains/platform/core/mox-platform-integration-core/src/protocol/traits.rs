//! 协议处理器抽象 — Protocol Handler Traits

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 协议类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    /// REST (HTTP/JSON)
    Rest,
    /// gRPC (HTTP/2/Protobuf)
    Grpc,
    /// GraphQL
    GraphQL,
    /// WebSocket
    WebSocket,
    /// SOAP (XML)
    Soap,
    /// MQTT
    Mqtt,
    /// 自定义协议
    Custom,
}

impl ProtocolType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolType::Rest => "rest",
            ProtocolType::Grpc => "grpc",
            ProtocolType::GraphQL => "graphql",
            ProtocolType::WebSocket => "websocket",
            ProtocolType::Soap => "soap",
            ProtocolType::Mqtt => "mqtt",
            ProtocolType::Custom => "custom",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            ProtocolType::Rest => 8080,
            ProtocolType::Grpc => 50051,
            ProtocolType::GraphQL => 8080,
            ProtocolType::WebSocket => 8080,
            ProtocolType::Soap => 8080,
            ProtocolType::Mqtt => 1883,
            ProtocolType::Custom => 0,
        }
    }
}

/// 协议请求（统一抽象）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolRequest {
    /// 请求ID
    pub request_id: String,
    /// 协议类型
    pub protocol: ProtocolType,
    /// 请求方法（REST: GET/POST, gRPC: service/method, GraphQL: query/mutation）
    pub method: String,
    /// 请求路径
    pub path: String,
    /// 请求头
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// 请求体（JSON格式）
    #[serde(default)]
    pub body: serde_json::Value,
    /// 查询参数
    #[serde(default)]
    pub query_params: HashMap<String, String>,
    /// 追踪ID
    #[serde(default)]
    pub trace_id: Option<String>,
    /// 租户ID
    #[serde(default)]
    pub tenant_id: Option<String>,
    /// 客户端地址
    #[serde(default)]
    pub client_addr: Option<String>,
}

impl ProtocolRequest {
    /// 创建基础请求
    pub fn new(protocol: ProtocolType, method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            protocol,
            method: method.into(),
            path: path.into(),
            headers: HashMap::new(),
            body: serde_json::Value::Null,
            query_params: HashMap::new(),
            trace_id: None,
            tenant_id: None,
            client_addr: None,
        }
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_body(mut self, body: serde_json::Value) -> Self {
        self.body = body;
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

/// 协议响应（统一抽象）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolResponse {
    /// 对应请求ID
    pub request_id: String,
    /// 状态码（HTTP状态码风格）
    pub status_code: u16,
    /// 响应头
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// 响应体（JSON格式）
    #[serde(default)]
    pub body: serde_json::Value,
    /// 耗时（毫秒）
    pub latency_ms: u64,
    /// 错误信息（失败时）
    #[serde(default)]
    pub error: Option<String>,
}

impl ProtocolResponse {
    /// 创建成功响应
    pub fn ok(request_id: impl Into<String>, body: serde_json::Value) -> Self {
        Self {
            request_id: request_id.into(),
            status_code: 200,
            headers: HashMap::new(),
            body,
            latency_ms: 0,
            error: None,
        }
    }

    /// 创建错误响应
    pub fn error(request_id: impl Into<String>, status_code: u16, message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            request_id: request_id.into(),
            status_code,
            headers: HashMap::new(),
            body: serde_json::json!({"error": msg.clone()}),
            latency_ms: 0,
            error: Some(msg),
        }
    }

    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }
}

/// 协议处理器trait
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// 协议类型
    fn protocol_type(&self) -> ProtocolType;

    /// 处理请求
    async fn handle(&self, request: ProtocolRequest) -> ProtocolResponse;

    /// 健康检查
    async fn health_check(&self) -> bool { true }

    /// 获取支持的路径/方法列表
    fn supported_routes(&self) -> Vec<String> { Vec::new() }
}

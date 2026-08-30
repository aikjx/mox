// Copyright (c) 2026 璇玑 RelGraph · 统一架构核心 (Unified Architecture Core)
// Licensed under the MIT License.

//! 架构归一化类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolType {
    /// RESTful HTTP
    Rest,
    /// GraphQL
    GraphQL,
    /// gRPC
    Grpc,
    /// WebSocket
    WebSocket,
    /// Webhook
    Webhook,
    /// MQTT
    Mqtt,
    /// AMQP
    Amqp,
    /// SSE (Server-Sent Events)
    Sse,
}

impl ProtocolType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolType::Rest => "rest",
            ProtocolType::GraphQL => "graphql",
            ProtocolType::Grpc => "grpc",
            ProtocolType::WebSocket => "websocket",
            ProtocolType::Webhook => "webhook",
            ProtocolType::Mqtt => "mqtt",
            ProtocolType::Amqp => "amqp",
            ProtocolType::Sse => "sse",
        }
    }
}

/// API 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiStatus {
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 处理中
    Processing,
    /// 已接受（异步处理）
    Accepted,
    /// 部分成功
    PartialSuccess,
}

impl ApiStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiStatus::Success => "success",
            ApiStatus::Failed => "failed",
            ApiStatus::Processing => "processing",
            ApiStatus::Accepted => "accepted",
            ApiStatus::PartialSuccess => "partial_success",
        }
    }
}

/// 统一 API 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    /// 请求 ID
    pub request_id: String,
    /// 协议类型
    pub protocol: ProtocolType,
    /// 资源类型
    pub resource_type: String,
    /// 操作类型
    pub operation: String,
    /// 请求体
    pub payload: serde_json::Value,
    /// 请求头/元数据
    pub metadata: HashMap<String, String>,
    /// 租户 ID
    pub tenant_id: Option<String>,
    /// 用户 ID
    pub user_id: Option<String>,
    /// 时间戳
    pub timestamp: u64,
    /// 超时设置（毫秒）
    pub timeout_ms: Option<u64>,
}

impl ApiRequest {
    /// 创建新请求
    pub fn new(resource_type: &str, operation: &str, payload: serde_json::Value) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            protocol: ProtocolType::Rest,
            resource_type: resource_type.to_string(),
            operation: operation.to_string(),
            payload,
            metadata: HashMap::new(),
            tenant_id: None,
            user_id: None,
            timestamp: now_ms(),
            timeout_ms: None,
        }
    }

    /// 设置租户
    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    /// 设置用户
    pub fn with_user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// 统一 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    /// 请求 ID
    pub request_id: String,
    /// 状态
    pub status: ApiStatus,
    /// 错误码
    pub error_code: Option<String>,
    /// 错误消息
    pub error_message: Option<String>,
    /// 响应数据
    pub data: Option<serde_json::Value>,
    /// 响应元数据
    pub metadata: HashMap<String, String>,
    /// 时间戳
    pub timestamp: u64,
    /// 处理耗时（毫秒）
    pub duration_ms: Option<u64>,
}

impl ApiResponse {
    /// 成功响应
    pub fn success(request_id: &str, data: serde_json::Value) -> Self {
        Self {
            request_id: request_id.to_string(),
            status: ApiStatus::Success,
            error_code: None,
            error_message: None,
            data: Some(data),
            metadata: HashMap::new(),
            timestamp: now_ms(),
            duration_ms: None,
        }
    }

    /// 错误响应
    pub fn error(request_id: &str, code: &str, message: &str) -> Self {
        Self {
            request_id: request_id.to_string(),
            status: ApiStatus::Failed,
            error_code: Some(code.to_string()),
            error_message: Some(message.to_string()),
            data: None,
            metadata: HashMap::new(),
            timestamp: now_ms(),
            duration_ms: None,
        }
    }

    /// 已接受响应（异步）
    pub fn accepted(request_id: &str) -> Self {
        Self {
            request_id: request_id.to_string(),
            status: ApiStatus::Accepted,
            error_code: None,
            error_message: None,
            data: None,
            metadata: HashMap::new(),
            timestamp: now_ms(),
            duration_ms: None,
        }
    }
}

/// 连接器类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConnectorCategory {
    /// 数据源
    DataSource,
    /// 身份认证
    Identity,
    /// 消息通知
    Notification,
    /// 存储服务
    Storage,
    /// AI 服务
    AiService,
    /// 企业系统
    Enterprise,
    /// 协作工具
    Collaboration,
    /// 开发工具
    DevTools,
    /// 支付服务
    Payment,
    /// 物联网
    Iot,
    /// 其他
    Other,
}

impl ConnectorCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectorCategory::DataSource => "data_source",
            ConnectorCategory::Identity => "identity",
            ConnectorCategory::Notification => "notification",
            ConnectorCategory::Storage => "storage",
            ConnectorCategory::AiService => "ai_service",
            ConnectorCategory::Enterprise => "enterprise",
            ConnectorCategory::Collaboration => "collaboration",
            ConnectorCategory::DevTools => "dev_tools",
            ConnectorCategory::Payment => "payment",
            ConnectorCategory::Iot => "iot",
            ConnectorCategory::Other => "other",
        }
    }
}

/// 连接器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInfo {
    /// 连接器 ID
    pub id: String,
    /// 连接器名称
    pub name: String,
    /// 类别
    pub category: ConnectorCategory,
    /// 版本
    pub version: String,
    /// 描述
    pub description: String,
    /// 供应商
    pub vendor: String,
    /// 图标 URL
    pub icon_url: Option<String>,
    /// 支持的操作
    pub operations: Vec<String>,
    /// 配置参数规格
    pub config_schema: serde_json::Value,
    /// 是否启用
    pub enabled: bool,
    /// 认证方式
    pub auth_type: String,
    /// 标签
    pub tags: Vec<String>,
}

/// 统一资源描述
///
/// 所有外部系统接入后，统一映射为资源模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedResource {
    /// 资源 ID
    pub id: String,
    /// 资源类型
    pub resource_type: String,
    /// 资源名称
    pub name: String,
    /// 所属连接器
    pub connector_id: String,
    /// 外部系统中的原始 ID
    pub external_id: String,
    /// 资源属性
    pub properties: HashMap<String, serde_json::Value>,
    /// 资源状态
    pub status: String,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
    /// 支持的操作
    pub supported_operations: Vec<String>,
}

/// 当前时间戳（毫秒）
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

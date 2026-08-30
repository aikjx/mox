// Copyright (c) 2026 璇玑 RelGraph · 统一架构核心 (Unified Architecture Core)
// Licensed under the MIT License.

//! 架构归一化错误类型

use std::fmt;

/// 架构统一错误
#[derive(Debug)]
pub enum ArchError {
    /// 无效请求
    InvalidRequest(String),
    /// 资源不存在
    NotFound(String),
    /// 资源已存在
    AlreadyExists(String),
    /// 权限不足
    PermissionDenied(String),
    /// 协议错误
    ProtocolError(String),
    /// 连接器错误
    ConnectorError(String),
    /// 连接器未找到
    ConnectorNotFound(String),
    /// 适配器错误
    AdapterError(String),
    /// 集成错误
    IntegrationError(String),
    /// 无效参数
    InvalidParameter { param: String, reason: String },
    /// 操作超时
    Timeout(String),
    /// 速率限制
    RateLimited { limit: u64, retry_after: Option<u64> },
    /// 内部错误
    InternalError(String),
    /// 不支持的操作
    UnsupportedOperation(String),
    /// 验证失败
    ValidationError(String),
}

pub type ArchResult<T> = Result<T, ArchError>;

impl fmt::Display for ArchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchError::InvalidRequest(msg) => write!(f, "invalid request: {}", msg),
            ArchError::NotFound(msg) => write!(f, "not found: {}", msg),
            ArchError::AlreadyExists(msg) => write!(f, "already exists: {}", msg),
            ArchError::PermissionDenied(msg) => write!(f, "permission denied: {}", msg),
            ArchError::ProtocolError(msg) => write!(f, "protocol error: {}", msg),
            ArchError::ConnectorError(msg) => write!(f, "connector error: {}", msg),
            ArchError::ConnectorNotFound(id) => write!(f, "connector not found: {}", id),
            ArchError::AdapterError(msg) => write!(f, "adapter error: {}", msg),
            ArchError::IntegrationError(msg) => write!(f, "integration error: {}", msg),
            ArchError::InvalidParameter { param, reason } => {
                write!(f, "invalid parameter '{}': {}", param, reason)
            }
            ArchError::Timeout(msg) => write!(f, "timeout: {}", msg),
            ArchError::RateLimited { limit, retry_after } => {
                write!(f, "rate limited (limit: {})", limit)?;
                if let Some(retry) = retry_after {
                    write!(f, ", retry after {}s", retry)?;
                }
                Ok(())
            }
            ArchError::InternalError(msg) => write!(f, "internal error: {}", msg),
            ArchError::UnsupportedOperation(op) => write!(f, "unsupported operation: {}", op),
            ArchError::ValidationError(msg) => write!(f, "validation error: {}", msg),
        }
    }
}

impl std::error::Error for ArchError {}

impl ArchError {
    /// 错误码（用于 API 响应）
    pub fn code(&self) -> &'static str {
        match self {
            ArchError::InvalidRequest(_) => "INVALID_REQUEST",
            ArchError::NotFound(_) => "NOT_FOUND",
            ArchError::AlreadyExists(_) => "ALREADY_EXISTS",
            ArchError::PermissionDenied(_) => "PERMISSION_DENIED",
            ArchError::ProtocolError(_) => "PROTOCOL_ERROR",
            ArchError::ConnectorError(_) => "CONNECTOR_ERROR",
            ArchError::ConnectorNotFound(_) => "CONNECTOR_NOT_FOUND",
            ArchError::AdapterError(_) => "ADAPTER_ERROR",
            ArchError::IntegrationError(_) => "INTEGRATION_ERROR",
            ArchError::InvalidParameter { .. } => "INVALID_PARAMETER",
            ArchError::Timeout(_) => "TIMEOUT",
            ArchError::RateLimited { .. } => "RATE_LIMITED",
            ArchError::InternalError(_) => "INTERNAL_ERROR",
            ArchError::UnsupportedOperation(_) => "UNSUPPORTED_OPERATION",
            ArchError::ValidationError(_) => "VALIDATION_ERROR",
        }
    }

    /// HTTP 状态码
    pub fn http_status(&self) -> u16 {
        match self {
            ArchError::InvalidRequest(_) => 400,
            ArchError::NotFound(_) => 404,
            ArchError::AlreadyExists(_) => 409,
            ArchError::PermissionDenied(_) => 403,
            ArchError::ProtocolError(_) => 400,
            ArchError::ConnectorError(_) => 502,
            ArchError::ConnectorNotFound(_) => 404,
            ArchError::AdapterError(_) => 500,
            ArchError::IntegrationError(_) => 502,
            ArchError::InvalidParameter { .. } => 400,
            ArchError::Timeout(_) => 504,
            ArchError::RateLimited { .. } => 429,
            ArchError::InternalError(_) => 500,
            ArchError::UnsupportedOperation(_) => 405,
            ArchError::ValidationError(_) => 422,
        }
    }
}

//! 插件错误类型

use thiserror::Error;

/// 插件错误
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("host API error: {0}")]
    HostApiError(String),

    #[error("AI call failed: {0}")]
    AiError(String),

    #[error("event publish failed: {0}")]
    EventError(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("other error: {0}")]
    Other(String),
}

impl From<String> for PluginError {
    fn from(s: String) -> Self { PluginError::Other(s) }
}

impl From<&str> for PluginError {
    fn from(s: &str) -> Self { PluginError::Other(s.to_string()) }
}

impl From<serde_json::Error> for PluginError {
    fn from(e: serde_json::Error) -> Self { PluginError::Other(e.to_string()) }
}

/// 插件结果类型
pub type PluginResult<T> = Result<T, PluginError>;

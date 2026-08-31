//! AI 错误类型

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("模型调用失败: {0}")]
    ProviderError(String),

    #[error("请求超时")]
    Timeout,

    #[error("请求被限流")]
    RateLimited,

    #[error("内容安全过滤")]
    ContentFiltered,

    #[error("参数错误: {0}")]
    InvalidParameter(String),
}

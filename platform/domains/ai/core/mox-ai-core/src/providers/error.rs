// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! AI Provider 统一错误类型

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("认证失败: {0}")]
    AuthError(String),

    #[error("网络错误: {0}")]
    NetworkError(String),

    #[error("模型未找到: {0}")]
    ModelNotFound(String),

    #[error("限流: {message} (retry_after: {retry_after_secs:?}s)")]
    RateLimited {
        retry_after_secs: Option<u64>,
        message: String,
    },

    #[error("内容被过滤: {0}")]
    ContentFiltered(String),

    #[error("请求超时")]
    Timeout,

    #[error("超出配额")]
    QuotaExceeded,

    #[error("服务不可用: {0}")]
    ServiceUnavailable(String),

    #[error("响应解析失败: {0}")]
    ParseError(String),

    #[error("Provider未注册: {0}")]
    ProviderNotFound(String),

    #[error("所有Provider均不可用")]
    AllProvidersFailed,

    #[error("不支持的能力: {0}")]
    UnsupportedCapability(String),

    #[error("其他错误: {0}")]
    Other(String),
}

impl From<reqwest::Error> for AiError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            AiError::Timeout
        } else if e.is_connect() {
            AiError::NetworkError(e.to_string())
        } else if e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) {
            AiError::RateLimited {
                retry_after_secs: None,
                message: "rate limited".into(),
            }
        } else if e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) {
            AiError::AuthError("invalid api key".into())
        } else {
            AiError::NetworkError(e.to_string())
        }
    }
}

impl From<serde_json::Error> for AiError {
    fn from(e: serde_json::Error) -> Self {
        AiError::ParseError(e.to_string())
    }
}

pub type AiResult<T> = Result<T, AiError>;

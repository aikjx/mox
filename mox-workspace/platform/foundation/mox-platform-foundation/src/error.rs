//! 统一错误类型与错误码体系
//!
//! 错误码格式：6 位数字
//! - 第 1 位：业务域（1=系统, 2=图谱, 3=AI, 4=流程, 5=数据, 6=云存储, 9=集成）
//! - 第 2 位：错误类型（0=参数, 1=认证, 2=权限, 3=不存在, 4=冲突, 5=内部, 6=超时, 7=限流）
//! - 第 3-6 位：顺序编号

use thiserror::Error;

/// 统一错误类型
#[derive(Debug, Error)]
pub enum MoxError {
    /// 参数错误 (10xxx)
    #[error("参数错误: {0}")]
    InvalidParameter(String),

    /// 未认证 (11xxx)
    #[error("未授权访问")]
    Unauthorized,

    /// 权限不足 (12xxx)
    #[error("权限不足: {0}")]
    PermissionDenied(String),

    /// 资源不存在 (13xxx)
    #[error("资源不存在: {0}")]
    NotFound(String),

    /// 资源冲突 (14xxx)
    #[error("资源冲突: {0}")]
    Conflict(String),

    /// 内部错误 (15xxx)
    #[error("内部错误: {0}")]
    Internal(String),

    /// 超时 (16xxx)
    #[error("操作超时")]
    Timeout,

    /// 限流 (17xxx)
    #[error("请求过于频繁，请稍后再试")]
    RateLimited,
}

impl MoxError {
    /// 获取错误码
    pub fn code(&self) -> i32 {
        match self {
            MoxError::InvalidParameter(_) => 10001,
            MoxError::Unauthorized => 11001,
            MoxError::PermissionDenied(_) => 12001,
            MoxError::NotFound(_) => 13001,
            MoxError::Conflict(_) => 14001,
            MoxError::Internal(_) => 15001,
            MoxError::Timeout => 16001,
            MoxError::RateLimited => 17001,
        }
    }

    /// HTTP 状态码
    pub fn http_status(&self) -> u16 {
        match self {
            MoxError::InvalidParameter(_) => 400,
            MoxError::Unauthorized => 401,
            MoxError::PermissionDenied(_) => 403,
            MoxError::NotFound(_) => 404,
            MoxError::Conflict(_) => 409,
            MoxError::Internal(_) => 500,
            MoxError::Timeout => 504,
            MoxError::RateLimited => 429,
        }
    }
}

pub type MoxResult<T> = Result<T, MoxError>;

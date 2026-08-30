// Copyright (c) 2026 璇玑 RelGraph · 前端功能归一化核心 (Unified Frontend Core)
// Licensed under the MIT License.

//! 错误类型

use thiserror::Error;

pub type FrontendResult<T> = Result<T, FrontendError>;

#[derive(Debug, Error)]
pub enum FrontendError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("validation error: {0}")]
    ValidationError(String),

    #[error("internal error: {0}")]
    InternalError(String),
}

impl FrontendError {
    pub fn code(&self) -> &'static str {
        match self {
            FrontendError::NotFound(_) => "NOT_FOUND",
            FrontendError::AlreadyExists(_) => "ALREADY_EXISTS",
            FrontendError::InvalidConfig(_) => "INVALID_CONFIG",
            FrontendError::ValidationError(_) => "VALIDATION_ERROR",
            FrontendError::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}

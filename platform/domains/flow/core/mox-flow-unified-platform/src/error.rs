// Copyright (c) 2026 璇玑 RelGraph · 全维归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 平台错误类型

use thiserror::Error;

pub type PlatformResult<T> = Result<T, PlatformError>;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("module not found: {0}")]
    ModuleNotFound(String),

    #[error("module already exists: {0}")]
    ModuleAlreadyExists(String),

    #[error("initialization error: {0}")]
    InitError(String),

    #[error("configuration error: {0}")]
    ConfigError(String),

    #[error("operation error: {0}")]
    OperationError(String),

    #[error("dependency error: {0}")]
    DependencyError(String),

    #[error("platform not initialized")]
    NotInitialized,

    #[error("internal error: {0}")]
    InternalError(String),
}

impl PlatformError {
    pub fn code(&self) -> &'static str {
        match self {
            PlatformError::ModuleNotFound(_) => "MODULE_NOT_FOUND",
            PlatformError::ModuleAlreadyExists(_) => "MODULE_ALREADY_EXISTS",
            PlatformError::InitError(_) => "INIT_ERROR",
            PlatformError::ConfigError(_) => "CONFIG_ERROR",
            PlatformError::OperationError(_) => "OPERATION_ERROR",
            PlatformError::DependencyError(_) => "DEPENDENCY_ERROR",
            PlatformError::NotInitialized => "NOT_INITIALIZED",
            PlatformError::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}

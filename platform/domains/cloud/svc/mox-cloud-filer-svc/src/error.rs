// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Filer 错误枚举。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FilerError {
    #[error("not found")]
    NotFound,
    #[error("permission denied")]
    PermissionDenied,
    #[error("directory not empty")]
    NotEmpty,
    #[error("metadata error: {0}")]
    Metadata(String),
    #[error("backend switch error: {0}")]
    BackendSwitch(String),
    #[error("fuse error: {0}")]
    Fuse(String),
    #[error("invalid attribute")]
    AttrInvalid,
    #[error("{0}")]
    Other(String),
}

impl FilerError {
    pub fn errno(&self) -> i32 {
        match self {
            FilerError::NotFound => 2,          // ENOENT
            FilerError::PermissionDenied => 13, // EACCES
            FilerError::NotEmpty => 39,         // ENOTEMPTY
            FilerError::AttrInvalid => 22,      // EINVAL
            _ => 1,
        }
    }
}

pub type FilerResult<T> = Result<T, FilerError>;

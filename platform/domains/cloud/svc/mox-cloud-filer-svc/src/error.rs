// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

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
    #[error("unsupported operation: {0}")]
    Unsupported(String),
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

impl From<FilerError> for mox_cloud_domain_traits::CloudError {
    fn from(e: FilerError) -> Self {
        match e {
            FilerError::NotFound => {
                mox_cloud_domain_traits::CloudError::NotFound("filer entry".into())
            }
            FilerError::AttrInvalid => {
                mox_cloud_domain_traits::CloudError::InvalidInput(e.to_string())
            }
            FilerError::Unsupported(msg) => {
                mox_cloud_domain_traits::CloudError::Unsupported(msg)
            }
            other => mox_cloud_domain_traits::CloudError::Filer(other.to_string()),
        }
    }
}

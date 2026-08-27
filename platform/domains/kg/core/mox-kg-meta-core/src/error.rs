// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 璇玑 R1 Meta Service 统一错误枚举。
//!
//! AIS L4 自研：只依赖本 crate 与标准库，不含任何成品图数据库。
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MetaError {
    #[error("SpaceExists: space {0} already exists")]
    SpaceExists(String),

    #[error("SpaceNotFound: space {0} not found")]
    SpaceNotFound(String),

    #[error("TagExists: tag {0} exists in space {1}")]
    TagExists(String, String),

    #[error("TagNotFound: tag {0} not found in space {1}")]
    TagNotFound(String, String),

    #[error("EdgeExists: edge type {0} exists in space {1}")]
    EdgeExists(String, String),

    #[error("EdgeNotFound: edge type {0} not found in space {1}")]
    EdgeNotFound(String, String),

    #[error("RaftError: {0}")]
    RaftError(String),

    #[error("AuthDenied: user {user} action {action} resource {resource}")]
    AuthDenied {
        user: String,
        action: String,
        resource: String,
    },

    #[error("UserNotFound: user {0}")]
    UserNotFound(String),

    #[error("AuthenticationFailed: user {0}")]
    AuthenticationFailed(String),

    #[error("StorageHostMissing: no storage host registered")]
    StorageHostMissing,

    #[error("PartitionInvalid: {0}")]
    PartitionInvalid(String),

    #[error("InvalidArgument: {0}")]
    InvalidArgument(String),

    #[error("Internal: {0}")]
    Internal(String),
}

pub type MetaResult<T> = Result<T, MetaError>;

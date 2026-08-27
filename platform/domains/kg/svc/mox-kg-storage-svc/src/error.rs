// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 错误枚举（璇玑 R2 Storage Service）
//!
//! 自研边界：仅基于 thiserror，零引用任何第三方商业/开源成品图数据库实现。

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StorageError {
    #[error("ShardNotFound: shard {0}")]
    ShardNotFound(u16),

    #[error("VidNotFound: vid {0}")]
    VidNotFound(String),

    #[error("EdgeNotFound: {src} -> {dst} @{etype} rank={rank}")]
    EdgeNotFound {
        src: String,
        dst: String,
        etype: String,
        rank: i64,
    },

    #[error("RaftApplyError: {0}")]
    RaftApplyError(String),

    #[error("CodecError: {0}")]
    CodecError(String),

    #[error("ConsumerLagOverThreshold: consumer={0} lag_ms={1}")]
    ConsumerLagOverThreshold(u64, u128),

    #[error("InvalidArgument: {0}")]
    InvalidArgument(String),

    #[error("Internal: {0}")]
    Internal(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

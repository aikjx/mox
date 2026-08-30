// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use thiserror::Error;

/// Unified error type for the cloud SDK.
#[derive(Debug, Error)]
pub enum CloudError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("service error: {0}")]
    Service(String),
    #[error("sts rejected: {0}")]
    StsRejected(String),
    #[error("iam deny: {0}")]
    IamDeny(String),
    #[error("quota exceeded: retry-after={0}s")]
    QuotaExceeded(u64),
    #[error("worm locked: {0}")]
    WormLocked(String),
    #[error("hashchain verify failed: {0}")]
    HashChainVerifyFailed(String),
    #[error("lock poison: {0}")]
    Lock(String),
}

pub type Result<T> = std::result::Result<T, CloudError>;

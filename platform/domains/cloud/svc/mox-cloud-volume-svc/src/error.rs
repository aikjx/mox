// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VolumeError {
    ChunkNotFound(String),
    CapacityExceeded(String),
    IOError(String),
    RebuildFailed(String),
    CrcMismatch(String),
    Internal(String),
    /// 写入被背压机制拒绝（达到最大并发写入数）
    BackpressureRejected(String),
}

impl fmt::Display for VolumeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VolumeError::ChunkNotFound(id) => write!(f, "Chunk not found: {}", id),
            VolumeError::CapacityExceeded(msg) => write!(f, "Capacity exceeded: {}", msg),
            VolumeError::IOError(msg) => write!(f, "IO error: {}", msg),
            VolumeError::RebuildFailed(msg) => write!(f, "Rebuild failed: {}", msg),
            VolumeError::CrcMismatch(msg) => write!(f, "CRC mismatch: {}", msg),
            VolumeError::Internal(msg) => write!(f, "Internal error: {}", msg),
            VolumeError::BackpressureRejected(msg) => write!(f, "Backpressure rejected: {}", msg),
        }
    }
}

impl Error for VolumeError {}

impl From<VolumeError> for mox_cloud_domain_traits::CloudError {
    fn from(e: VolumeError) -> Self {
        match e {
            VolumeError::BackpressureRejected(msg) => {
                mox_cloud_domain_traits::CloudError::BackpressureRejected(msg)
            },
            VolumeError::ChunkNotFound(id) => {
                mox_cloud_domain_traits::CloudError::NotFound(format!("chunk: {id}"))
            },
            other => mox_cloud_domain_traits::CloudError::Volume(other.to_string()),
        }
    }
}

pub type VolumeResult<T> = Result<T, VolumeError>;

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VolumeError {
    ChunkNotFound(String),
    CapacityExceeded(String),
    IOError(String),
    RebuildFailed(String),
    CrcMismatch(String),
    Internal(String),
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
        }
    }
}

impl Error for VolumeError {}

pub type VolumeResult<T> = Result<T, VolumeError>;

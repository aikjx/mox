// Copyright (c) 2026 璇玑 RelGraph · 统一元数据层 (Unified Metadata Layer)
// Licensed under the MIT License.

//! 元数据错误类型

use std::fmt;

/// 元数据错误
#[derive(Debug)]
pub enum MetaError {
    /// 实体不存在
    EntityNotFound(String),
    /// 实体已存在
    EntityAlreadyExists(String),
    /// Schema 不存在
    SchemaNotFound(String),
    /// Schema 已存在
    SchemaAlreadyExists(String),
    /// 无效参数
    InvalidParameter { param: String, reason: String },
    /// 版本冲突
    VersionConflict {
        entity_id: String,
        expected: u64,
        actual: u64,
    },
    /// 验证失败
    ValidationError(String),
    /// 存储错误
    StorageError(String),
    /// Raft 共识错误
    RaftError(String),
    /// 索引错误
    IndexError(String),
    /// 不支持的操作
    UnsupportedOperation(String),
}

pub type MetaResult<T> = Result<T, MetaError>;

impl fmt::Display for MetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaError::EntityNotFound(id) => write!(f, "entity not found: {}", id),
            MetaError::EntityAlreadyExists(id) => write!(f, "entity already exists: {}", id),
            MetaError::SchemaNotFound(id) => write!(f, "schema not found: {}", id),
            MetaError::SchemaAlreadyExists(id) => write!(f, "schema already exists: {}", id),
            MetaError::InvalidParameter { param, reason } => {
                write!(f, "invalid parameter '{}': {}", param, reason)
            }
            MetaError::VersionConflict {
                entity_id,
                expected,
                actual,
            } => write!(
                f,
                "version conflict for entity {}: expected {}, actual {}",
                entity_id, expected, actual
            ),
            MetaError::ValidationError(msg) => write!(f, "validation error: {}", msg),
            MetaError::StorageError(msg) => write!(f, "storage error: {}", msg),
            MetaError::RaftError(msg) => write!(f, "raft error: {}", msg),
            MetaError::IndexError(msg) => write!(f, "index error: {}", msg),
            MetaError::UnsupportedOperation(op) => write!(f, "unsupported operation: {}", op),
        }
    }
}

impl std::error::Error for MetaError {}

impl From<mox_flow_unified_storage_core::error::StorageError> for MetaError {
    fn from(e: mox_flow_unified_storage_core::error::StorageError) -> Self {
        MetaError::StorageError(e.to_string())
    }
}

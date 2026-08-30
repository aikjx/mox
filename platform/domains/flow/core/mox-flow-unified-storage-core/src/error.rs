// Copyright (c) 2026 璇玑 RelGraph · 统一存储引擎 (Unified Storage Engine)
// Licensed under the MIT License.

//! 存储错误类型

use std::fmt;

/// 存储错误
#[derive(Debug)]
pub enum StorageError {
    /// 键不存在
    KeyNotFound(String),
    /// 对象不存在
    ObjectNotFound(String),
    /// 节点不存在
    NodeNotFound(String),
    /// 边不存在
    EdgeNotFound(String),
    /// 键已存在
    KeyAlreadyExists(String),
    /// 无效参数
    InvalidParameter { param: String, reason: String },
    /// 后端错误
    BackendError(String),
    /// 不支持的操作
    UnsupportedOperation(String),
    /// IO 错误
    IoError(String),
    /// 序列化错误
    SerializationError(String),
    /// 容量不足
    CapacityExceeded { required: u64, available: u64 },
    /// 事务错误
    TransactionError(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::KeyNotFound(k) => write!(f, "key not found: {}", k),
            StorageError::ObjectNotFound(o) => write!(f, "object not found: {}", o),
            StorageError::NodeNotFound(n) => write!(f, "node not found: {}", n),
            StorageError::EdgeNotFound(e) => write!(f, "edge not found: {}", e),
            StorageError::KeyAlreadyExists(k) => write!(f, "key already exists: {}", k),
            StorageError::InvalidParameter { param, reason } => {
                write!(f, "invalid parameter '{}': {}", param, reason)
            }
            StorageError::BackendError(e) => write!(f, "backend error: {}", e),
            StorageError::UnsupportedOperation(op) => {
                write!(f, "unsupported operation: {}", op)
            }
            StorageError::IoError(e) => write!(f, "IO error: {}", e),
            StorageError::SerializationError(e) => write!(f, "serialization error: {}", e),
            StorageError::CapacityExceeded { required, available } => {
                write!(
                    f,
                    "capacity exceeded: required {} bytes, available {} bytes",
                    required, available
                )
            }
            StorageError::TransactionError(e) => write!(f, "transaction error: {}", e),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self {
        StorageError::SerializationError(e.to_string())
    }
}

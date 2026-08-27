// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 审计错误类型
//!
//! 各 Sink（Syslog/S3/Kafka）与外部审计链共享同一错误枚举，
//! 便于调用方统一处理连接失败、写入失败、序列化失败等情形。

use std::fmt;

/// 审计子系统错误
#[derive(Debug, Clone)]
pub enum AuditError {
    /// TCP/Unix 连接失败
    Connection(String),
    /// 写入目标失败
    WriteFailed(String),
    /// 事件序列化失败
    Serialization(String),
    /// 内部哈希链一致性校验失败（已被篡改）
    ChainInconsistency(String),
    /// Sink 处于禁用状态且无可用 sink
    Disabled,
    /// 其它未分类错误
    Other(String),
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditError::Connection(s) => write!(f, "审计连接失败: {s}"),
            AuditError::WriteFailed(s) => write!(f, "审计写入失败: {s}"),
            AuditError::Serialization(s) => write!(f, "审计序列化失败: {s}"),
            AuditError::ChainInconsistency(s) => write!(f, "审计链不一致(可能篡改): {s}"),
            AuditError::Disabled => write!(f, "审计 Sink 已禁用"),
            AuditError::Other(s) => write!(f, "审计错误: {s}"),
        }
    }
}

impl std::error::Error for AuditError {}

impl From<serde_json::Error> for AuditError {
    fn from(e: serde_json::Error) -> Self {
        AuditError::Serialization(e.to_string())
    }
}

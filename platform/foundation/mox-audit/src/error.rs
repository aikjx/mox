// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 审计错误类型
//!
//! 各 Sink（Syslog/S3）与审计链共享同一错误枚举，
//! 便于调用方统一处理连接失败、写入失败、序列化失败等情形。
//!
//! 同时提供向 `MoxError` 的转换，支持平台级错误码体系。

use mox_error::{define_domain_errors, ErrorDomain, MoxError};
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
    /// 签名验证失败
    SignatureInvalid(String),
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
            AuditError::SignatureInvalid(s) => write!(f, "审计签名验证失败: {s}"),
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

// ── 平台级错误码：审计域（Platform 域·审计模块 03） ──────────────────

define_domain_errors!(AuditErrors, Platform,
    // 审计模块 (03)
    CONNECTION_FAILED:     (03, 001, "审计连接失败", 500, Error),
    WRITE_FAILED:          (03, 002, "审计写入失败", 500, Error),
    SERIALIZATION_FAILED:  (03, 003, "审计序列化失败", 500, Error),
    CHAIN_TAMPERED:        (03, 004, "审计链被篡改", 500, Critical),
    SINK_DISABLED:         (03, 005, "审计 Sink 已禁用", 503, Warning),
    SIGNATURE_INVALID:     (03, 006, "审计签名无效", 401, Warning),
);

impl From<AuditError> for MoxError {
    fn from(err: AuditError) -> Self {
        match err {
            AuditError::Connection(msg) => {
                AuditErrors::CONNECTION_FAILED().with_detail(msg)
            }
            AuditError::WriteFailed(msg) => {
                AuditErrors::WRITE_FAILED().with_detail(msg)
            }
            AuditError::Serialization(msg) => {
                AuditErrors::SERIALIZATION_FAILED().with_detail(msg)
            }
            AuditError::ChainInconsistency(msg) => {
                AuditErrors::CHAIN_TAMPERED().with_detail(msg)
            }
            AuditError::Disabled => AuditErrors::SINK_DISABLED(),
            AuditError::SignatureInvalid(msg) => {
                AuditErrors::SIGNATURE_INVALID().with_detail(msg)
            }
            AuditError::Other(msg) => {
                MoxError::internal(ErrorDomain::Platform, 03, 999, msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_variants() {
        assert!(format!("{}", AuditError::Connection("x".into())).contains("审计连接失败"));
        assert!(format!("{}", AuditError::WriteFailed("x".into())).contains("审计写入失败"));
        assert!(format!("{}", AuditError::Serialization("x".into())).contains("审计序列化失败"));
        assert!(format!("{}", AuditError::ChainInconsistency("x".into())).contains("审计链不一致"));
        assert!(format!("{}", AuditError::Disabled).contains("已禁用"));
        assert!(format!("{}", AuditError::SignatureInvalid("x".into())).contains("签名验证失败"));
        assert!(format!("{}", AuditError::Other("x".into())).contains("审计错误"));
    }

    #[test]
    fn serde_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let audit_err: AuditError = json_err.into();
        assert!(matches!(audit_err, AuditError::Serialization(_)));
    }

    #[test]
    fn into_mox_error_has_platform_domain() {
        use mox_error::{ErrorDomain, ErrorLevel};
        let mox: MoxError = AuditError::ChainInconsistency("test".into()).into();
        assert_eq!(mox.domain, ErrorDomain::Platform);
        assert_eq!(mox.level, ErrorLevel::Critical);
        assert_eq!(mox.code, "PL03004");
    }

    #[test]
    fn error_impl_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(AuditError::Disabled);
        assert!(e.to_string().contains("已禁用"));
    }
}

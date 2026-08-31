// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX 平台级审计基础设施
//!
//! 统一审计事件模型、SHA-256 哈希链、多 Sink 分发（Syslog/S3/自定义）。
//!
//! # 设计目标
//!
//! - 平台级基础设施，不依赖任何领域 crate
//! - 可独立编译和测试
//! - 满足 SOC2 Type II / GDPR / ISO27001 / HIPAA 合规要求
//!
//! # 模块结构
//!
//! - [`event`] — 统一审计事件模型 + 相关枚举
//! - [`chain`] — 审计哈希链（SHA-256）
//! - [`sink`] — `AuditSink` trait + `MultiSink` + `NoopSink`
//! - [`syslog`] — `SyslogSink`（RFC 5424）
//! - [`s3`] — `S3Sink`（S3 兼容对象存储，WORM 合规）
//! - [`context`] — `AuditContext`（链 + Sink 双写，统一入口）
//! - [`error`] — `AuditError`（基于 `mox-error`）
//!
//! # 快速开始
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use mox_audit::{
//!     AuditContext, AuditEvent, AuditActor, AuditAction,
//!     AuditResource, AuditOutcome, AuditSeverity,
//!     MultiSink, NoopSink,
//! };
//!
//! // 配置 Sink（示例用 NoopSink，生产环境替换为 SyslogSink / S3Sink）
//! let sink = MultiSink::new().with_sink(Box::new(NoopSink));
//!
//! // 创建审计上下文
//! let ctx = AuditContext::new(Arc::new(sink))
//!     .with_hmac_secret("your-hmac-secret".into());
//!
//! // 发射审计事件
//! let event = AuditEvent::new(
//!     AuditActor::human("alice", "admin"),
//!     AuditAction::FlowCreated,
//!     AuditResource::flow("my-flow", "tenant-1"),
//!     AuditOutcome::Success,
//!     AuditSeverity::Info,
//!     "tenant-1".into(),
//! );
//! ctx.emit(event).unwrap();
//!
//! // 验证链完整性
//! assert!(ctx.verify_chain().is_ok());
//! ```

// ── 模块声明 ────────────────────────────────────────────────────

pub mod error;
pub mod event;
pub mod chain;
pub mod sink;
pub mod syslog;
pub mod context;

#[cfg(feature = "s3-sink")]
pub mod s3;

// ── 重导出 ──────────────────────────────────────────────────────

// 错误
pub use error::{AuditError, AuditErrors};

// 事件模型
pub use event::{
    ActorSource, AuditAction, AuditActor, AuditEvent, AuditOutcome, AuditResource,
    AuditSeverity,
};

// 哈希链
pub use chain::{AuditChain, GENESIS_HASH};

// Sink
pub use sink::{AuditSink, FlushPolicy, MultiSink, NoopSink};

// Syslog Sink
pub use syslog::{SyslogProtocol, SyslogSink};

// S3 Sink（可选 feature）
#[cfg(feature = "s3-sink")]
pub use s3::{S3Credentials, S3Sink};

// 审计上下文
pub use context::{Audited, AuditContext};

// ── 兼容别名 ────────────────────────────────────────────────────

/// 兼容别名：原 `audit::ExtAuditEvent` → 统一 `AuditEvent`
///
/// 在 expert-svc 迁移期间，可通过此别名平滑过渡。
pub type ExtAuditEvent = AuditEvent;

/// 兼容别名：原 `pipeline_core::UnifiedAuditEvent` → 统一 `AuditEvent`
pub type UnifiedAuditEvent = AuditEvent;

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod lib_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn ext_audit_event_is_alias() {
        let ev: ExtAuditEvent = AuditEvent::new(
            AuditActor::system(),
            AuditAction::FlowCreated,
            AuditResource::flow("f1", "t1"),
            AuditOutcome::Success,
            AuditSeverity::Info,
            "t1".into(),
        );
        assert_eq!(ev.event_id, ev.event_id); // 类型一致
    }

    #[test]
    fn unified_audit_event_is_alias() {
        let ev: UnifiedAuditEvent = AuditEvent::new(
            AuditActor::system(),
            AuditAction::PhaseStart,
            AuditResource::pipeline("t1", "t1"),
            AuditOutcome::Pending,
            AuditSeverity::Debug,
            "t1".into(),
        );
        assert!(matches!(ev.action, AuditAction::PhaseStart));
    }

    #[test]
    fn full_context_integration_test() {
        let sink = MultiSink::new().with_sink(Box::new(NoopSink));
        let ctx = AuditContext::new(Arc::new(sink)).with_hmac_secret("secret".into());

        // 发射多个事件
        for i in 0..5 {
            let ev = AuditEvent::new(
                AuditActor::human(&format!("user-{i}"), "admin"),
                AuditAction::FlowCreated,
                AuditResource::flow(&format!("flow-{i}"), "tenant-1"),
                AuditOutcome::Success,
                AuditSeverity::Info,
                "tenant-1".into(),
            )
            .with_session(format!("sess-{i}"))
            .with_client_ip(format!("10.0.0.{i}"))
            .with_extra("index", i.into());

            ctx.emit(ev).unwrap();
        }

        // 验证链完整性
        assert_eq!(ctx.chain_len(), 5);
        assert!(ctx.verify_chain().is_ok());

        // 刷新
        assert!(ctx.flush().is_ok());
    }

    #[test]
    fn audit_error_into_mox_error() {
        use mox_error::MoxError;
        let audit_err = AuditError::ChainInconsistency("test".into());
        let mox_err: MoxError = audit_err.into();
        assert_eq!(mox_err.code, "PL03004");
    }
}

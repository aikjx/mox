// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 外部审计 Sink 模块
//!
//! AuditChain（内存哈希链）是内部自验工具，不是合规证据。
//! 本模块提供外部持久化审计接口，对接 Syslog / S3(WORM)，
//! 满足 SOC2 Type II / GDPR / ISO27001 / HIPAA 合规要求。
//!
//! 核心类型：
//! - ExtAuditEvent：外部合规标准事件（独立于内部 govern::AuditEvent）
//! - AuditSink trait：任意存储后端实现此 trait 即可
//! - AuditContext：内部链 + 外部 sink 双写，统一入口
//!
//! 后端对比（均为真实可用实现）：
//! | 后端 | 部署依赖 | 持久化 | 适用场景 |
//! |------|---------|--------|---------|
//! | SyslogSink | syslog 服务器（如 rsyslog → ELK/SIEM） | 否（轮询） | 实时告警 |
//! | S3Sink | S3 兼容存储（MinIO/COS/OBS） | WORM | 合规存档 |
//!
//! > 说明：NATS / RabbitMQ 后端曾为占位伪代码（无依赖、publish 恒报错），
//! > 已按"禁伪代码"原则删除；如确有消息队列审计需求，可基于
//! > [`AuditSink`] trait 在部署侧接入（trait 契约稳定，实现即插即用）。
//!
//! 使用示例：
//! ```rust
//! use std::sync::Arc;
//! use mox_ai_expert_svc::audit::{
//!     AuditContext, AuditSink, MultiSink, NoopSink, ExtAuditEvent,
//!     AuditActor, AuditAction, AuditOutcome, AuditSeverity, AuditResource,
//! };
//!
//! // 组合多个 sink（示例用 NoopSink，真实场景替换为 SyslogSink/S3Sink）
//! let multi = MultiSink::new()
//!     .with_sink(Box::new(NoopSink));
//!
//! let ctx = AuditContext::new(Arc::new(multi))
//!     .with_hmac_secret("your-secret".into());
//!
//! let event = ExtAuditEvent::new(
//!     AuditActor::human("alice", "gov"),
//!     AuditAction::MoxOptimize,
//!     AuditResource::flow("gov-pii", "gov"),
//!     AuditOutcome::Success,
//!     AuditSeverity::Info,
//!     "gov".into(),
//! );
//! ctx.log(event).unwrap();
//! ```

pub mod error;
mod event;
pub mod integration;
mod s3;
mod sink;
mod syslog;

pub use error::AuditError;
pub use event::{
    ActorSource, AuditAction, AuditActor, AuditEvent, AuditOutcome, AuditResource, AuditSeverity,
    ExtAuditEvent,
};
pub use integration::AuditContext;
pub use s3::S3Sink;
pub use sink::{AuditSink, FlushPolicy, MultiSink, NoopSink};
pub use syslog::SyslogSink;

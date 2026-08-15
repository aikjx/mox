//! 外部审计 Sink 模块
//!
//! AuditChain（内存哈希链）是内部自验工具，不是合规证据。
//! 本模块提供外部持久化审计接口，对接 Syslog / S3(WORM) / NATS / RabbitMQ，
//! 满足 SOC2 Type II / GDPR / ISO27001 / HIPAA 合规要求。
//!
//! 核心类型：
//! - ExtAuditEvent：外部合规标准事件（独立于内部 govern::AuditEvent）
//! - AuditSink trait：任意存储后端实现此 trait 即可
//! - AuditContext：内部链 + 外部 sink 双写，统一入口
//!
//! 四个后端对比：
//! | 后端 | 部署依赖 | 持久化 | 适用场景 |
//! |------|---------|--------|---------|
//! | SyslogSink | syslog 服务器（如 rsyslog → ELK/SIEM） | 否（轮询） | 实时告警 |
//! | S3Sink | S3 兼容存储（MinIO/COS/OBS） | WORM | 合规存档 |
//! | NatsSink | NATS 单二进制（JetStream） | JetStream | 高吞吐事件流 |
//! | RabbitMqSink | RabbitMQ（AMQP 0-9-1） | durable queue | 企业消息中间件（已有RabbitMQ时复用） |
//!
//! 使用示例：
//! ```rust
//! use std::sync::Arc;
//! use expert_alliance::audit::{
//!     AuditContext, AuditSink, MultiSink, NoopSink, ExtAuditEvent,
//!     AuditActor, AuditAction, AuditOutcome, AuditSeverity, AuditResource,
//! };
//!
//! // 组合多个 sink（示例用 NoopSink，真实场景替换为 SyslogSink/S3Sink/NatsSink/RabbitMqSink）
//! let multi = MultiSink::new()
//!     .add(Box::new(NoopSink));
//!
//! let ctx = AuditContext::new(Arc::new(multi))
//!     .with_hmac_secret("your-secret".into());
//!
//! let event = ExtAuditEvent::new(
//!     AuditActor::human("alice", "gov"),
//!     AuditAction::AllianceOptimize,
//!     AuditResource::flow("gov-pii", "gov"),
//!     AuditOutcome::Success,
//!     AuditSeverity::Info,
//!     "gov".into(),
//! );
//! ctx.log(event).unwrap();
//! ```

mod sink;
mod event;
mod syslog;
mod s3;
mod nats;
mod rabbitmq;
pub mod error;
pub mod integration;

pub use error::AuditError;
pub use sink::{AuditSink, FlushPolicy, MultiSink, NoopSink};
pub use event::{
    ExtAuditEvent, AuditEvent, AuditActor, AuditAction, AuditOutcome,
    AuditSeverity, AuditResource, ActorSource,
};
pub use integration::AuditContext;
pub use syslog::SyslogSink;
pub use s3::S3Sink;
pub use nats::NatsSink;
pub use rabbitmq::RabbitMqSink;

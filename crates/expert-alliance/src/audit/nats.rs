//! NATS Sink — 写入 NATS 事件流（轻量单二进制，JetStream 持久化）
//!
//! Subject 按租户隔离：audit.{tenant}
//! JetStream 持久化（WORM 保证）+ 消费者组回放。
//!
//! 生产依赖：cargo add nats

use super::{AuditError, AuditSink, FlushPolicy};
use super::event::ExtAuditEvent;

pub struct NatsSink {
    url: String,
    subject_prefix: String,
    flush_policy: FlushPolicy,
}

impl NatsSink {
    /// 连接 NATS 服务器
    ///
    /// `url` 示例：
    /// - 本地：`nats://localhost:4222`
    /// - TLS：`tls://localhost:7443`
    /// - 用户密码：`nats://user:pass@localhost:4222`
    pub fn new(url: &str) -> Self {
        Self {
            url: url.into(),
            subject_prefix: "audit".into(),
            flush_policy: FlushPolicy::Batch { max_events: 50 },
        }
    }

    pub fn with_subject_prefix(mut self, prefix: &str) -> Self {
        self.subject_prefix = prefix.into();
        self
    }

    pub fn with_flush_policy(mut self, policy: FlushPolicy) -> Self {
        self.flush_policy = policy;
        self
    }

    /// Subject 路径（dot 分隔，NATS 惯例）
    fn subject(&self, tenant_id: &str) -> String {
        format!("{}.{}", self.subject_prefix, tenant_id)
    }

    /// JetStream stream 名称（按租户隔离存储）
    #[allow(dead_code)]
    fn stream(&self, tenant_id: &str) -> String {
        format!("AUDIT_{}", tenant_id.replace('-', "_").to_uppercase())
    }

    /// 发送单条事件到 NATS
    fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), AuditError> {
        // 占位：生产版将 subject + payload 发往 NATS（见下方注释实现）
        let _ = (subject, payload);

        Err(AuditError::WriteFailed(format!(
            "NATS producer requires 'nats' crate. \
             Configure url='{}'. Run: cargo add nats. \
             Server: nats-server --jetstream (持久化) 或 nats-server (仅流)",
            self.url,
        )))
    }

    /// JetStream stream 配置（首次运行调用一次）
    #[allow(dead_code)]
    fn ensure_stream(&self, _tenant_id: &str) -> Result<(), AuditError> {
        // === 生产实现（取消注释并配置依赖）===
        // use nats::Connection;
        // use nats::jetstream::stream::{StorageType, RetentionPolicy, StreamConfig};
        // let nc = Connection::new(&self.url)
        //     .map_err(|e| AuditError::Connection(e.to_string()))?;
        // let js = nats::JetStream::new(nc);
        // let cfg = StreamConfig {
        //     name: self.stream(tenant_id),
        //     subjects: vec![self.subject(tenant_id)],
        //     max_bytes: 1_073_741_824, // 1 GiB
        //     max_age: 31536000,        // 365 天（SOC2 保留期）
        //     storage: StorageType::File,
        //     retention: RetentionPolicy::Limits,
        //     ..Default::default()
        // };
        // js.add_stream(&cfg)
        //     .map_err(|e| AuditError::WriteFailed(e.to_string()))?;

        Err(AuditError::WriteFailed(format!(
            "JetStream stream creation requires 'nats' crate with server --jetstream flag. \
             Run: cargo add nats && nats-server --jetstream",
        )))
    }
}

impl AuditSink for NatsSink {
    fn append_sync(&self, event: &ExtAuditEvent) -> Result<(), AuditError> {
        let subject = self.subject(&event.tenant_id);
        let payload = serde_json::to_vec(event)
            .map_err(|e| AuditError::Serialization(e.to_string()))?;
        self.publish(&subject, &payload)
    }

    fn flush(&self) -> Result<(), AuditError> { Ok(()) }

    fn health_check(&self) -> Result<(), AuditError> {
        if self.url.is_empty() {
            Err(AuditError::Disabled)
        } else {
            // === 生产实现：尝试连接并 ping ===
            // use nats::Connection;
            // let nc = Connection::new(&self.url)
            //     .map_err(|e| AuditError::Connection(e.to_string()))?;
            // nc.flush().map_err(|e| AuditError::Connection(e.to_string()))
            Ok(())
        }
    }

    fn is_enabled(&self) -> bool { !self.url.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_format() {
        let sink = NatsSink::new("nats://localhost:4222");
        assert_eq!(sink.subject("gov-tenant"), "audit.gov-tenant");
        assert_eq!(sink.subject("tenant-123"), "audit.tenant-123");
    }

    #[test]
    fn stream_name() {
        let sink = NatsSink::new("nats://localhost:4222");
        assert_eq!(sink.stream("gov-tenant"), "AUDIT_GOV_TENANT");
        assert_eq!(sink.stream("tenant-123"), "AUDIT_TENANT_123");
    }

    #[test]
    fn append_returns_write_failed_not_disabled() {
        let sink = NatsSink::new("nats://localhost:4222");
        let event = ExtAuditEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            actor: super::super::event::AuditActor::system(),
            action: super::super::event::AuditAction::FlowCreated,
            resource: super::super::event::AuditResource::flow("test-flow", "test-tenant"),
            outcome: super::super::event::AuditOutcome::Success,
            severity: super::super::event::AuditSeverity::Info,
            chain_hash: String::new(),
            content_hash: String::new(),
            signature: None,
            tenant_id: "test-tenant".into(),
            session_id: None,
            client_ip: None,
            extra: serde_json::Map::new(),
        };
        // 生产版返回 Ok，dev 环境返回明确的 WriteFailed（配置提示）
        match sink.append_sync(&event) {
            Err(AuditError::WriteFailed(msg)) => {
                assert!(msg.contains("nats"), "错误消息应包含 nats 提示");
            }
            _ => panic!("expected WriteFailed with nats config hint"),
        }
    }

    #[test]
    fn empty_url_is_disabled() {
        let sink = NatsSink::new("");
        assert!(!sink.is_enabled());
    }

    #[test]
    fn builder_pattern() {
        let sink = NatsSink::new("nats://localhost:4222")
            .with_subject_prefix("events")
            .with_flush_policy(FlushPolicy::Batch { max_events: 100 });
        assert_eq!(sink.subject("t1"), "events.t1");
    }
}

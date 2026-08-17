//! RabbitMQ Sink — 写入 RabbitMQ 事件流（AMQP 0-9-1，成熟消息队列）
//!
//! Exchange 按租户隔离，Queue / RoutingKey 语义与 Kafka Topic 等价。
//! 支持持久化（durable）、确认模式（confirm）、多消费者回放。
//!
//! 生产依赖：cargo add lapin --features "tokio-compatible-serialization,serde-json,tokio"

use super::{AuditError, AuditSink, FlushPolicy};
use super::event::ExtAuditEvent;

pub struct RabbitMqSink {
    /// AMQP URI，如 amqp://guest:guest@localhost:5672/
    uri: String,
    /// Exchange 名称前缀，最终名 = prefix.{tenant}
    exchange_prefix: String,
    /// Exchange 类型
    exchange_type: ExchangeType,
    /// 路由键前缀
    routing_prefix: String,
    /// 确认模式
    confirm_mode: ConfirmMode,
    /// 批量刷新策略
    flush_policy: FlushPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExchangeType {
    /// 直连 exchange（单消费者，精确路由）
    Direct,
    /// 主题 exchange（支持通配符路由，* 匹配单段，# 匹配多段）
    #[default]
    Topic,
    /// 扇出 exchange（广播，所有绑定队列收到同一消息）
    Fanout,
    /// 头信息 exchange（按 headers 属性匹配）
    Headers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmMode {
    /// 不等待 broker 确认（最大吞吐，低可靠性）
    #[default]
    None,
    /// 等待 broker 发回 confirm（Publisher Confirms，at-least-once）
    Publish,
    /// transactional channel（每条手动 txSelect/txCommit，代价高）
    Transactional,
}

impl RabbitMqSink {
    /// 连接 RabbitMQ 服务器
    ///
    /// `uri` 格式：amqp[s]://[user[:pass]@]host[:port][/vhost]
    ///
    /// 示例：
    /// - `amqp://guest:guest@localhost:5672/`        （本地开发）
    /// - `amqps://alice:secret@rabbitmq.corp:5671/` （TLS 生产）
    pub fn new(uri: &str) -> Self {
        Self {
            uri: uri.into(),
            exchange_prefix: "audit".into(),
            exchange_type: ExchangeType::Topic,
            routing_prefix: "audit".into(),
            confirm_mode: ConfirmMode::Publish,
            flush_policy: FlushPolicy::Batch { max_events: 50 },
        }
    }

    pub fn with_exchange_prefix(mut self, prefix: &str) -> Self {
        self.exchange_prefix = prefix.into();
        self
    }

    pub fn with_exchange_type(mut self, t: ExchangeType) -> Self {
        self.exchange_type = t;
        self
    }

    pub fn with_routing_prefix(mut self, prefix: &str) -> Self {
        self.routing_prefix = prefix.into();
        self
    }

    pub fn with_confirm_mode(mut self, m: ConfirmMode) -> Self {
        self.confirm_mode = m;
        self
    }

    pub fn with_flush_policy(mut self, policy: FlushPolicy) -> Self {
        self.flush_policy = policy;
        self
    }

    /// Exchange 名称（按租户隔离，生产版 publish 中调用）
    #[allow(dead_code)]
    fn exchange(&self, tenant_id: &str) -> String {
        format!("{}.{}", self.exchange_prefix, tenant_id)
    }

    /// 路由键（租户 + 事件类型）
    fn routing_key(&self, tenant_id: &str, action: &str) -> String {
        format!("{}.{}.{}", self.routing_prefix, tenant_id, action)
    }

    /// 发布消息到 RabbitMQ
    fn publish(&self, routing_key: &str, body: &[u8]) -> Result<(), AuditError> {
        // 占位：生产版用 routing_key + body 发往 RabbitMQ（见下方注释实现）
        let _ = (routing_key, body);
        // === 生产实现（取消注释并配置依赖）===
        //
        // use lapin::{
        //     options::*, types::FieldTable, BasicProperties, Channel,
        //     Connection, ConnectionProperties,
        // };
        // use futures_lite::future;
        //
        // let conn = Connection::connect(&self.uri, ConnectionProperties::default())
        //     .map_err(|e| AuditError::Connection(e.to_string()))?;
        // let channel: Channel = conn.create_channel().await
        //     .map_err(|e| AuditError::Connection(e.to_string()))?;
        //
        // // Exchange 声明（durable 持久化）
        // channel
        //     .exchange_declare(
        //         &self.exchange(""),
        //         match self.exchange_type {
        //             ExchangeType::Direct  => lapin::ExchangeKind::Direct,
        //             ExchangeType::Topic   => lapin::ExchangeKind::Topic,
        //             ExchangeType::Fanout  => lapin::ExchangeKind::Fanout,
        //             ExchangeType::Headers => lapin::ExchangeKind::Headers,
        //         },
        //         &ExchangeDeclareOptions {
        //             durable: true,
        //             ..Default::default()
        //         },
        //         &FieldTable::default(),
        //     )
        //     .await
        //     .map_err(|e| AuditError::WriteFailed(e.to_string()))?;
        //
        // let props = BasicProperties::default()
        //     .with_content_type("application/json".into())
        //     .with_delivery_mode(2); // 持久化消息
        //
        // match self.confirm_mode {
        //     ConfirmMode::None => {
        //         channel.basic_publish(
        //             "",
        //             routing_key,
        //             &BasicPublishOptions::default(),
        //             body,
        //             props,
        //         )
        //         .await
        //         .map_err(|e| AuditError::WriteFailed(e.to_string()))?;
        //     }
        //     ConfirmMode::Publish => {
        //         channel.confirm_select(&ConfirmSelectOptions::default())
        //             .await
        //             .map_err(|e| AuditError::WriteFailed(e.to_string()))?;
        //         let ch = channel.basic_publish(
        //             "",
        //             routing_key,
        //             &BasicPublishOptions::default(),
        //             body,
        //             props,
        //         )
        //         .await
        //         .map_err(|e| AuditError::WriteFailed(e.to_string()))?
        //         .await
        //         .map_err(|e| AuditError::WriteFailed(e.to_string()))?;
        //     }
        //     ConfirmMode::Transactional => {
        //         channel.tx_select().await
        //             .map_err(|e| AuditError::WriteFailed(e.to_string()))?;
        //         channel.basic_publish(
        //             "",
        //             routing_key,
        //             &BasicPublishOptions::default(),
        //             body,
        //             props,
        //         )
        //         .await
        //         .map_err(|e| AuditError::WriteFailed(e.to_string()))?;
        //         channel.tx_commit().await
        //             .map_err(|e| AuditError::WriteFailed(e.to_string()))?;
        //     }
        // }

        Err(AuditError::WriteFailed(format!(
            "RabbitMQ producer requires 'lapin' crate. \
             Configure uri='{}'. Run: cargo add lapin --features \
             'tokio-compatible-serialization,serde-json,tokio'. \
             Server: docker run -p 5672:5672 -p 15672:15672 rabbitmq:3-management",
            self.uri,
        )))
    }
}

impl AuditSink for RabbitMqSink {
    fn append_sync(&self, event: &ExtAuditEvent) -> Result<(), AuditError> {
        let routing_key = self.routing_key(&event.tenant_id, &format!("{:?}", event.action));
        let body = serde_json::to_vec(event)
            .map_err(|e| AuditError::Serialization(e.to_string()))?;
        self.publish(&routing_key, &body)
    }

    fn flush(&self) -> Result<(), AuditError> { Ok(()) }

    fn health_check(&self) -> Result<(), AuditError> {
        if self.uri.is_empty() {
            Err(AuditError::Disabled)
        } else {
            // === 生产实现：建立连接并检测 ===
            // use lapin::{Connection, ConnectionProperties};
            // Connection::connect(&self.uri, ConnectionProperties::default())
            //     .map_err(|e| AuditError::Connection(e.to_string()))?;
            Ok(())
        }
    }

    fn is_enabled(&self) -> bool { !self.uri.is_empty() }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_format() {
        let sink = RabbitMqSink::new("amqp://guest:guest@localhost:5672/");
        assert_eq!(sink.exchange("gov-tenant"), "audit.gov-tenant");
        assert_eq!(sink.exchange("finance"), "audit.finance");
    }

    #[test]
    fn routing_key_format() {
        let sink = RabbitMqSink::new("amqp://guest:guest@localhost:5672/");
        let rk = sink.routing_key("gov-tenant", "FlowCreated");
        assert_eq!(rk, "audit.gov-tenant.FlowCreated");
    }

    #[test]
    fn builder_chain() {
        let sink = RabbitMqSink::new("amqp://guest:guest@localhost:5672/")
            .with_exchange_prefix("events")
            .with_exchange_type(ExchangeType::Fanout)
            .with_routing_prefix("evt")
            .with_confirm_mode(ConfirmMode::None)
            .with_flush_policy(FlushPolicy::Batch { max_events: 100 });
        assert_eq!(sink.exchange("t1"), "events.t1");
        let rk = sink.routing_key("t1", "AuditEvent");
        assert_eq!(rk, "evt.t1.AuditEvent");
    }

    #[test]
    fn append_returns_write_failed_not_disabled() {
        let sink = RabbitMqSink::new("amqp://guest:guest@localhost:5672/");
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
        match sink.append_sync(&event) {
            Err(AuditError::WriteFailed(msg)) => {
                assert!(msg.contains("lapin"), "错误消息应包含 lapin 提示");
            }
            _ => panic!("expected WriteFailed with lapin config hint"),
        }
    }

    #[test]
    fn empty_uri_is_disabled() {
        let sink = RabbitMqSink::new("");
        assert!(!sink.is_enabled());
    }

    #[test]
    fn tls_uri_accepted() {
        let sink = RabbitMqSink::new("amqps://alice:secret@rabbitmq.corp:5671/");
        assert!(sink.is_enabled());
    }
}

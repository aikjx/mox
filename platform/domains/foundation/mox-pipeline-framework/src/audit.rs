// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 统一审计基础设施（管线框架内置）
//!
//! 管线框架内置轻量审计链，用于：
//! - 阶段事件追踪（每个阶段的开始/结束）
//! - 哈希链完整性验证（防篡改）
//! - 多 Sink 输出（可桥接到外部审计系统）
//!
//! 当启用 `audit` feature 时，可桥接到 `mox-audit` 平台级审计系统，
//! 实现合规级别的持久化存储（Syslog/S3 等）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::phase::{PhaseId, PhaseStatus};

// ================== 统一审计事件 ==================

/// 统一审计事件（管线框架内置）
///
/// 包含完整的审计字段：行动者、操作、资源、结果、严重程度、
/// 哈希链（prev_hash + content_hash）、trace_id、阶段信息等。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAuditEvent {
    /// 全局唯一事件 ID
    pub event_id: String,
    /// 事件时间（ISO8601，UTC）
    pub timestamp: DateTime<Utc>,
    /// 行动者
    pub actor: AuditActor,
    /// 操作类型
    pub action: String,
    /// 操作对象类型
    pub resource_type: String,
    /// 操作对象 ID
    pub resource_id: String,
    /// 操作结果
    pub outcome: AuditOutcome,
    /// 严重程度
    pub severity: AuditSeverity,
    /// 前一个事件的哈希（链式追溯）
    pub prev_hash: String,
    /// 本事件内容哈希（防篡改）
    pub content_hash: String,
    /// 租户 ID（多租户隔离）
    pub tenant_id: String,
    /// trace id（管线关联）
    pub trace_id: Uuid,
    /// 所属阶段（管线事件有，其他事件可选）
    #[serde(default)]
    pub phase: Option<String>,
    /// 额外上下文
    #[serde(default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl UnifiedAuditEvent {
    fn compute_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.event_id.as_bytes());
        h.update(self.timestamp.to_rfc3339().as_bytes());
        h.update(self.actor.id.as_bytes());
        h.update(self.action.as_bytes());
        h.update(self.resource_type.as_bytes());
        h.update(self.resource_id.as_bytes());
        h.update(format!("{:?}", self.outcome).as_bytes());
        h.update(self.tenant_id.as_bytes());
        h.update(self.trace_id.as_bytes());
        h.update(self.prev_hash.as_bytes());
        format!("{:064x}", h.finalize())
    }
}

/// 行动者
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActor {
    pub id: String,
    pub role: String,
    pub source: ActorSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorSource {
    Human,
    Ai,
    System,
    Unknown,
}

/// 操作结果
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
    Blocked,
    Skipped,
    Pending,
}

/// 审计严重程度（RFC 5424 优先级映射）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Emergency = 0,
    Alert = 1,
    Critical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Info = 6,
    Debug = 7,
}

impl From<PhaseStatus> for AuditOutcome {
    fn from(status: PhaseStatus) -> Self {
        match status {
            PhaseStatus::Success => AuditOutcome::Success,
            PhaseStatus::Failed => AuditOutcome::Failure,
            PhaseStatus::Blocked => AuditOutcome::Blocked,
            PhaseStatus::Skipped => AuditOutcome::Skipped,
            PhaseStatus::Pending | PhaseStatus::Running => AuditOutcome::Pending,
        }
    }
}

impl From<AuditOutcome> for AuditSeverity {
    fn from(outcome: AuditOutcome) -> Self {
        match outcome {
            AuditOutcome::Success => AuditSeverity::Info,
            AuditOutcome::Failure => AuditSeverity::Warning,
            AuditOutcome::Blocked => AuditSeverity::Critical,
            AuditOutcome::Skipped => AuditSeverity::Debug,
            AuditOutcome::Pending => AuditSeverity::Debug,
        }
    }
}

// ================== 统一审计链 ==================

/// 统一审计链（管线框架内置）
///
/// 提供：
/// - 内部哈希链完整性验证（SHA-256）
/// - 阶段开始/结束的便捷记录方法
/// - 外部 Sink 桥接（可注册自定义 Sink）
///
/// 当启用 `audit` feature 时，可通过 `MoxAuditSink` 桥接到
/// 平台级 `mox-audit` 系统，实现合规持久化。
pub struct UnifiedAuditChain {
    events: Vec<UnifiedAuditEvent>,
    /// 外部 sink（可选）
    external_sinks: Vec<Box<dyn AuditSink + Send + Sync>>,
    /// 起始哈希
    genesis_hash: String,
}

impl Default for UnifiedAuditChain {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for UnifiedAuditChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnifiedAuditChain")
            .field("event_count", &self.events.len())
            .field("external_sinks", &self.external_sinks.len())
            .finish()
    }
}

impl UnifiedAuditChain {
    /// 创建空的审计链
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            external_sinks: Vec::new(),
            genesis_hash: "GENESIS".to_string(),
        }
    }

    /// 添加外部审计 sink
    pub fn add_sink(&mut self, sink: Box<dyn AuditSink + Send + Sync>) {
        self.external_sinks.push(sink);
    }

    /// 追加一个审计事件
    pub fn append(&mut self, mut event: UnifiedAuditEvent) -> &UnifiedAuditEvent {
        let prev_hash = self
            .events
            .last()
            .map(|e| e.content_hash.clone())
            .unwrap_or_else(|| self.genesis_hash.clone());
        event.prev_hash = prev_hash;
        event.content_hash = event.compute_hash();

        // 写入外部 sink
        for sink in &self.external_sinks {
            if let Err(e) = sink.write(&event) {
                tracing::warn!(target: "pipeline_audit", "external sink write failed: {}", e);
            }
        }

        self.events.push(event);
        self.events.last().unwrap()
    }

    /// 记录阶段开始
    pub fn record_phase_start<P: PhaseId>(&mut self, phase: &P, trace_id: Uuid) {
        let event = UnifiedAuditEvent {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            actor: AuditActor {
                id: "system".into(),
                role: "pipeline".into(),
                source: ActorSource::System,
            },
            action: format!("phase.{}", phase.name()),
            resource_type: "pipeline".into(),
            resource_id: trace_id.to_string(),
            outcome: AuditOutcome::Pending,
            severity: AuditSeverity::Debug,
            prev_hash: String::new(),
            content_hash: String::new(),
            tenant_id: String::new(),
            trace_id,
            phase: Some(phase.name().to_string()),
            extra: serde_json::Map::new(),
        };
        self.append(event);
    }

    /// 记录阶段结束
    pub fn record_phase_end<P: PhaseId>(
        &mut self,
        phase: &P,
        status: PhaseStatus,
        latency_ms: u64,
        trace_id: Uuid,
    ) {
        let outcome: AuditOutcome = status.into();
        let severity: AuditSeverity = outcome.into();
        let mut extra = serde_json::Map::new();
        extra.insert(
            "latency_ms".to_string(),
            serde_json::Value::Number(latency_ms.into()),
        );

        let event = UnifiedAuditEvent {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            actor: AuditActor {
                id: "system".into(),
                role: "pipeline".into(),
                source: ActorSource::System,
            },
            action: format!("phase.{}.done", phase.name()),
            resource_type: "pipeline".into(),
            resource_id: trace_id.to_string(),
            outcome,
            severity,
            prev_hash: String::new(),
            content_hash: String::new(),
            tenant_id: String::new(),
            trace_id,
            phase: Some(phase.name().to_string()),
            extra,
        };
        self.append(event);
    }

    /// 事件数量
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// 获取所有事件
    pub fn events(&self) -> &[UnifiedAuditEvent] {
        &self.events
    }

    /// 校验链完整性（防篡改）
    pub fn verify(&self) -> bool {
        let mut prev = self.genesis_hash.clone();
        for event in &self.events {
            if event.prev_hash != prev {
                return false;
            }
            // 重新计算哈希以验证
            let re = event.clone();
            let expected = re.compute_hash();
            if event.content_hash != expected {
                return false;
            }
            prev = event.content_hash.clone();
        }
        true
    }

    /// 返回链上最新事件的哈希
    pub fn latest_hash(&self) -> Option<&str> {
        self.events.last().map(|e| e.content_hash.as_str())
    }
}

// ================== 审计 Sink ==================

/// 外部审计 sink 接口
///
/// 实现此 trait 可将审计事件写入外部系统（Syslog、S3、数据库等）。
/// 管线框架内置的审计链通过 Sink 桥接到外部系统。
pub trait AuditSink {
    /// 写入一个审计事件
    fn write(&self, event: &UnifiedAuditEvent) -> Result<(), String>;

    /// sink 名称（用于日志和调试）
    fn name(&self) -> &str {
        "unnamed_sink"
    }
}

// ── mox-audit 桥接 Sink（可选 feature） ─────────────────────────

#[cfg(feature = "audit")]
pub mod mox_audit_bridge {
    use super::*;

    /// 将管线审计事件桥接到 mox-audit 平台级审计系统的 Sink
    ///
    /// 当启用 `audit` feature 时可用。
    /// 将管线内部的 `UnifiedAuditEvent` 转换为 `mox_audit::AuditEvent`
    /// 并写入到 `mox_audit::AuditContext`。
    pub struct MoxAuditSink {
        pub audit_ctx: std::sync::Arc<mox_audit::AuditContext>,
    }

    impl MoxAuditSink {
        pub fn new(audit_ctx: std::sync::Arc<mox_audit::AuditContext>) -> Self {
            Self { audit_ctx }
        }
    }

    impl AuditSink for MoxAuditSink {
        fn write(&self, event: &UnifiedAuditEvent) -> Result<(), String> {
            let mox_event = mox_audit::AuditEvent::new(
                mox_audit::AuditActor {
                    id: event.actor.id.clone(),
                    role: event.actor.role.clone(),
                    source: match event.actor.source {
                        ActorSource::Human => mox_audit::ActorSource::Human,
                        ActorSource::Ai => mox_audit::ActorSource::Ai,
                        ActorSource::System => mox_audit::ActorSource::System,
                        ActorSource::Unknown => mox_audit::ActorSource::Unknown,
                    },
                },
                mox_audit::AuditAction::Unknown(event.action.clone()),
                mox_audit::AuditResource {
                    resource_type: event.resource_type.clone(),
                    resource_id: event.resource_id.clone(),
                    tenant_id: event.tenant_id.clone(),
                    name: None,
                },
                match event.outcome {
                    AuditOutcome::Success => mox_audit::AuditOutcome::Success,
                    AuditOutcome::Failure => mox_audit::AuditOutcome::Failure,
                    AuditOutcome::Blocked => mox_audit::AuditOutcome::Blocked,
                    AuditOutcome::Skipped => mox_audit::AuditOutcome::Skipped,
                    AuditOutcome::Pending => mox_audit::AuditOutcome::Pending,
                },
                match event.severity {
                    AuditSeverity::Emergency => mox_audit::AuditSeverity::Emergency,
                    AuditSeverity::Alert => mox_audit::AuditSeverity::Alert,
                    AuditSeverity::Critical => mox_audit::AuditSeverity::Critical,
                    AuditSeverity::Error => mox_audit::AuditSeverity::Error,
                    AuditSeverity::Warning => mox_audit::AuditSeverity::Warning,
                    AuditSeverity::Notice => mox_audit::AuditSeverity::Notice,
                    AuditSeverity::Info => mox_audit::AuditSeverity::Info,
                    AuditSeverity::Debug => mox_audit::AuditSeverity::Debug,
                },
                event.tenant_id.clone(),
            )
            .with_trace_id(event.trace_id);

            // 添加 phase 和 extra
            let mut mox_event = if let Some(ref phase) = event.phase {
                mox_event.with_phase(phase.clone())
            } else {
                mox_event
            };

            for (k, v) in &event.extra {
                mox_event = mox_event.with_extra(k.clone(), v.clone());
            }

            self.audit_ctx
                .emit(mox_event)
                .map_err(|e| format!("mox-audit emit failed: {e}"))?;

            Ok(())
        }

        fn name(&self) -> &str {
            "mox_audit_sink"
        }
    }
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::NamedPhase;

    #[test]
    fn audit_chain_tamper_detected() {
        let mut chain = UnifiedAuditChain::new();
        let trace_id = Uuid::new_v4();
        let phase = NamedPhase::new("analyze");

        chain.record_phase_start(&phase, trace_id);
        chain.record_phase_end(&phase, PhaseStatus::Success, 100, trace_id);

        assert!(chain.verify(), "初始链应完整");
        assert_eq!(chain.len(), 2);

        // 篡改中间事件
        let mut events: Vec<UnifiedAuditEvent> = std::mem::take(&mut chain.events);
        if let Some(ev) = events.first_mut() {
            ev.action = "hacked".to_string();
        }
        chain.events = events;

        assert!(!chain.verify(), "篡改后应检测到");
    }

    #[test]
    fn audit_chain_empty_is_valid() {
        let chain = UnifiedAuditChain::new();
        assert!(chain.verify());
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
        assert!(chain.latest_hash().is_none());
    }

    #[test]
    fn record_phase_start_and_end() {
        let mut chain = UnifiedAuditChain::new();
        let trace_id = Uuid::new_v4();
        let phase = NamedPhase::new("normalize");

        chain.record_phase_start(&phase, trace_id);
        assert_eq!(chain.len(), 1);

        chain.record_phase_end(&phase, PhaseStatus::Success, 50, trace_id);
        assert_eq!(chain.len(), 2);

        // 验证链完整性
        assert!(chain.verify());

        // 检查事件内容
        let events = chain.events();
        assert_eq!(events[0].phase.as_deref(), Some("normalize"));
        assert_eq!(events[0].outcome, AuditOutcome::Pending);
        assert_eq!(events[1].outcome, AuditOutcome::Success);
        assert_eq!(events[1].severity, AuditSeverity::Info);
    }

    #[test]
    fn phase_status_to_outcome() {
        assert_eq!(
            AuditOutcome::from(PhaseStatus::Success),
            AuditOutcome::Success
        );
        assert_eq!(
            AuditOutcome::from(PhaseStatus::Blocked),
            AuditOutcome::Blocked
        );
        assert_eq!(
            AuditOutcome::from(PhaseStatus::Skipped),
            AuditOutcome::Skipped
        );
        assert_eq!(
            AuditOutcome::from(PhaseStatus::Running),
            AuditOutcome::Pending
        );
    }

    #[test]
    fn outcome_to_severity() {
        let sev: AuditSeverity = AuditOutcome::Success.into();
        assert_eq!(sev, AuditSeverity::Info);

        let sev: AuditSeverity = AuditOutcome::Blocked.into();
        assert_eq!(sev, AuditSeverity::Critical);

        let sev: AuditSeverity = AuditOutcome::Failure.into();
        assert_eq!(sev, AuditSeverity::Warning);
    }

    #[test]
    fn custom_sink_receives_events() {
        use std::sync::{Arc, Mutex};

        struct TestSink {
            events: Arc<Mutex<Vec<String>>>,
        }

        impl AuditSink for TestSink {
            fn write(&self, event: &UnifiedAuditEvent) -> Result<(), String> {
                self.events.lock().unwrap().push(event.action.clone());
                Ok(())
            }
            fn name(&self) -> &str {
                "test_sink"
            }
        }

        let received = Arc::new(Mutex::new(Vec::new()));
        let mut chain = UnifiedAuditChain::new();
        chain.add_sink(Box::new(TestSink {
            events: received.clone(),
        }));

        let trace_id = Uuid::new_v4();
        let phase = NamedPhase::new("analyze");
        chain.record_phase_start(&phase, trace_id);

        let events = received.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].contains("analyze"));
    }

    #[cfg(feature = "audit")]
    #[test]
    fn mox_audit_sink_bridge() {
        use mox_audit::{MultiSink, NoopSink};
        use std::sync::Arc;

        let sink = MultiSink::new().with_sink(Box::new(NoopSink));
        let mox_ctx = Arc::new(
            mox_audit::AuditContext::new(Arc::new(sink))
                .with_hmac_secret("test-secret".into()),
        );

        let mut chain = UnifiedAuditChain::new();
        chain.add_sink(Box::new(mox_audit_bridge::MoxAuditSink::new(mox_ctx.clone())));

        let trace_id = Uuid::new_v4();
        let phase = NamedPhase::new("analyze");
        chain.record_phase_start(&phase, trace_id);
        chain.record_phase_end(&phase, PhaseStatus::Success, 100, trace_id);

        // 内部链有 2 个事件
        assert_eq!(chain.len(), 2);
        // mox-audit 链也应有 2 个事件（通过桥接 Sink 写入）
        assert_eq!(mox_ctx.chain_len(), 2);
        assert!(mox_ctx.verify_chain().is_ok());
    }
}

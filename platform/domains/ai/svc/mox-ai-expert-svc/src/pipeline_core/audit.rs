// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 统一审计基础设施
//!
//! 将三套审计事件模型收敛为统一模型：
//! - `govern::AuditChain`（内部哈希链，防篡改）
//! - `alliance::gate::AuditEvent`（管线阶段审计，7 类事件）
//! - `audit::event::ExtAuditEvent`（外部合规标准，SOC2/GDPR）
//!
//! 统一策略：
//! - 事件结构：基于 ExtAuditEvent（最完整，符合合规标准）
//! - 链式结构：保留 AuditChain 的 prev_hash 哈希链机制
//! - 多 sink 输出：内部链 + 外部 sink（Syslog/S3/自定义）双写
//! - 自动发射：管线核心在每个阶段前后自动生成审计事件

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::pipeline_core::phase::{Phase, PhaseStatus};

// ================== 统一审计事件 ==================

/// 统一审计事件
///
/// 整合三套审计模型的优点：
/// - 来自 ExtAuditEvent 的完整合规字段（actor/action/resource/outcome/severity）
/// - 来自 AuditChain 的 prev_hash 链式哈希
/// - 来自 alliance AuditEvent 的 trace_id 和阶段关联
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

/// 统一审计链
///
/// 整合：
/// - govern::AuditChain 的哈希链完整性验证
/// - audit::event 的外部合规结构
/// - alliance 的阶段事件概念
///
/// 内部链始终在内存中维护，用于自验证。
/// 外部 sink 可通过 `with_external_sink` 注册，实现双写。
pub struct UnifiedAuditChain {
    events: Vec<UnifiedAuditEvent>,
    /// 外部 sink（可选）
    external_sinks: Vec<Box<dyn AuditSink + Send + Sync>>,
    /// 起始哈希
    genesis_hash: String,
}

impl std::fmt::Debug for UnifiedAuditChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnifiedAuditChain")
            .field("event_count", &self.events.len())
            .field("external_sinks", &self.external_sinks.len())
            .finish()
    }
}

impl Default for UnifiedAuditChain {
    fn default() -> Self {
        Self::new()
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
                tracing::warn!(target: "audit", "external sink write failed: {}", e);
            }
        }

        self.events.push(event);
        self.events.last().unwrap()
    }

    /// 记录阶段开始
    pub fn record_phase_start(&mut self, phase: Phase, trace_id: Uuid) {
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
    pub fn record_phase_end(
        &mut self,
        phase: Phase,
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
            let mut re = event.clone();
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
pub trait AuditSink {
    /// 写入一个审计事件
    fn write(&self, event: &UnifiedAuditEvent) -> Result<(), String>;

    /// sink 名称（用于日志和调试）
    fn name(&self) -> &str {
        "unnamed_sink"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_chain_tamper_detected() {
        let mut chain = UnifiedAuditChain::new();
        let trace_id = Uuid::new_v4();

        chain.record_phase_start(Phase::Analyze, trace_id);
        chain.record_phase_end(Phase::Analyze, PhaseStatus::Success, 100, trace_id);

        assert!(chain.verify(), "初始链应完整");
        assert_eq!(chain.len(), 2);

        // 篡改中间事件
        let mut events = std::mem::take(&mut chain.events);
        if let Some(ev) = events.first_mut() {
            ev.action = "hacked".to_string();
        }
        chain.events = events;

        assert!(!chain.verify(), "篡改后应检测到");
    }
}

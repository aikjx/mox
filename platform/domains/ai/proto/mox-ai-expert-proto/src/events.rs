// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 领域事件协议
//!
//! 定义专家领域的标准事件类型，供下游系统订阅和处理。
//! 这些事件可转换为统一审计事件（mox-audit::AuditEvent）。
//!
//! ## 设计原则
//! - 事件是不可变的值对象
//! - 事件包含必要的业务上下文（不泄露内部实现细节）
//! - 事件可序列化为 JSON，便于跨服务传递
//! - 事件可转换为审计事件（满足合规要求）

use crate::types::Dimension;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// 领域事件枚举
// ============================================================================

/// 专家领域事件类型
///
/// 使用 tag 模式的 enum，便于 JSON 序列化时区分事件类型。
/// 下游可通过 match 处理不同事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum ExpertDomainEvent {
    /// 专家注册事件
    ExpertRegistered {
        event_id: String,
        timestamp: String,
        expert_id: String,
        expert_name: String,
        dimension: String,
        domain: String,
        registered_by: String,
    },

    /// 咨询开始事件
    ConsultStarted {
        event_id: String,
        timestamp: String,
        consult_id: String,
        expert_id: String,
        query_preview: String,
        tenant_id: String,
    },

    /// 咨询完成事件
    ConsultCompleted {
        event_id: String,
        timestamp: String,
        consult_id: String,
        expert_id: String,
        score: f64,
        vetoed: bool,
        latency_ms: u64,
        tenant_id: String,
    },

    /// 治理裁决事件
    GovernVerdictIssued {
        event_id: String,
        timestamp: String,
        gate_id: String,
        level: String,
        score: f64,
        reasons: Vec<String>,
        flow_id: String,
        tenant_id: String,
    },

    /// 联盟分析开始事件
    AllianceStarted {
        event_id: String,
        timestamp: String,
        trace_id: String,
        team_size: usize,
        scenario: String,
        tenant_id: String,
    },

    /// 联盟分析完成事件
    AllianceCompleted {
        event_id: String,
        timestamp: String,
        trace_id: String,
        gate_grade: String,
        gate_passed: bool,
        total_ms: u64,
        consensus: f64,
        tenant_id: String,
    },

    /// 专家辩论事件
    ExpertDebateHeld {
        event_id: String,
        timestamp: String,
        trace_id: String,
        participants: Vec<String>,
        rounds: u32,
        consensus: f64,
        tenant_id: String,
    },
}

impl ExpertDomainEvent {
    /// 获取事件 ID
    pub fn event_id(&self) -> &str {
        match self {
            ExpertDomainEvent::ExpertRegistered { event_id, .. } => event_id,
            ExpertDomainEvent::ConsultStarted { event_id, .. } => event_id,
            ExpertDomainEvent::ConsultCompleted { event_id, .. } => event_id,
            ExpertDomainEvent::GovernVerdictIssued { event_id, .. } => event_id,
            ExpertDomainEvent::AllianceStarted { event_id, .. } => event_id,
            ExpertDomainEvent::AllianceCompleted { event_id, .. } => event_id,
            ExpertDomainEvent::ExpertDebateHeld { event_id, .. } => event_id,
        }
    }

    /// 获取事件类型名称
    pub fn event_type(&self) -> &'static str {
        match self {
            ExpertDomainEvent::ExpertRegistered { .. } => "expert_registered",
            ExpertDomainEvent::ConsultStarted { .. } => "consult_started",
            ExpertDomainEvent::ConsultCompleted { .. } => "consult_completed",
            ExpertDomainEvent::GovernVerdictIssued { .. } => "govern_verdict_issued",
            ExpertDomainEvent::AllianceStarted { .. } => "alliance_started",
            ExpertDomainEvent::AllianceCompleted { .. } => "alliance_completed",
            ExpertDomainEvent::ExpertDebateHeld { .. } => "expert_debate_held",
        }
    }

    /// 获取租户 ID
    pub fn tenant_id(&self) -> &str {
        match self {
            ExpertDomainEvent::ExpertRegistered { .. } => "system",
            ExpertDomainEvent::ConsultStarted { tenant_id, .. } => tenant_id,
            ExpertDomainEvent::ConsultCompleted { tenant_id, .. } => tenant_id,
            ExpertDomainEvent::GovernVerdictIssued { tenant_id, .. } => tenant_id,
            ExpertDomainEvent::AllianceStarted { tenant_id, .. } => tenant_id,
            ExpertDomainEvent::AllianceCompleted { tenant_id, .. } => tenant_id,
            ExpertDomainEvent::ExpertDebateHeld { tenant_id, .. } => tenant_id,
        }
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 从 JSON 字符串反序列化
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ============================================================================
// 事件构建器（便捷构造）
// ============================================================================

/// 领域事件构建器
pub struct EventBuilder {
    tenant_id: String,
    operator: String,
}

impl EventBuilder {
    pub fn new(tenant_id: impl Into<String>, operator: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            operator: operator.into(),
        }
    }

    pub fn expert_registered(&self, expert_id: &str, expert_name: &str, dimension: Dimension, domain: &str) -> ExpertDomainEvent {
        ExpertDomainEvent::ExpertRegistered {
            event_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            expert_id: expert_id.to_string(),
            expert_name: expert_name.to_string(),
            dimension: format!("{:?}", dimension),
            domain: domain.to_string(),
            registered_by: self.operator.clone(),
        }
    }

    pub fn consult_completed(&self, consult_id: &str, expert_id: &str, score: f64, vetoed: bool, latency_ms: u64) -> ExpertDomainEvent {
        ExpertDomainEvent::ConsultCompleted {
            event_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            consult_id: consult_id.to_string(),
            expert_id: expert_id.to_string(),
            score,
            vetoed,
            latency_ms,
            tenant_id: self.tenant_id.clone(),
        }
    }

    pub fn alliance_started(&self, trace_id: &str, team_size: usize, scenario: &str) -> ExpertDomainEvent {
        ExpertDomainEvent::AllianceStarted {
            event_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.to_string(),
            team_size,
            scenario: scenario.to_string(),
            tenant_id: self.tenant_id.clone(),
        }
    }

    pub fn alliance_completed(&self, trace_id: &str, gate_grade: &str, gate_passed: bool, total_ms: u64, consensus: f64) -> ExpertDomainEvent {
        ExpertDomainEvent::AllianceCompleted {
            event_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.to_string(),
            gate_grade: gate_grade.to_string(),
            gate_passed,
            total_ms,
            consensus,
            tenant_id: self.tenant_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_serialization() {
        let event = ExpertDomainEvent::ConsultCompleted {
            event_id: "evt-123".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            consult_id: "c-1".into(),
            expert_id: "security".into(),
            score: 0.85,
            vetoed: false,
            latency_ms: 1200,
            tenant_id: "tenant-a".into(),
        };

        let json = event.to_json().unwrap();
        assert!(json.contains("\"event_type\":\"consult_completed\""));
        assert!(json.contains("\"score\":0.85"));

        let parsed = ExpertDomainEvent::from_json(&json).unwrap();
        assert_eq!(parsed.event_type(), "consult_completed");
        assert_eq!(parsed.tenant_id(), "tenant-a");
    }

    #[test]
    fn event_builder_creates_valid_events() {
        let builder = EventBuilder::new("test-tenant", "test-user");
        let event = builder.expert_registered(
            "sec-001",
            "安全专家",
            Dimension::Security,
            "gov",
        );
        assert_eq!(event.event_type(), "expert_registered");
        assert!(!event.event_id().is_empty());
    }
}

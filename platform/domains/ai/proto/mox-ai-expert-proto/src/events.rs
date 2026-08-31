// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 领域事件协议
//!
//! 定义专家领域的标准事件类型，供下游系统订阅和处理。
//! 这些事件可转换为统一审计事件（mox-audit::AuditEvent）。
//!
//! ## 设计原则
//! - **不可变值对象**：事件一经产生不可修改
//! - **业务上下文完整**：携带必要的业务字段，但不泄露内部实现细节
//! - **可序列化**：支持 JSON 序列化，便于跨服务 / 消息队列传递
//! - **分布式追踪**：每个事件携带 trace_id，支持全链路追踪
//! - **可审计**：可转换为审计事件，满足合规要求
//!
//! ## 事件分类
//!
//! | 事件类型               | 触发时机             | 关键字段                          |
//! |-----------------------|---------------------|-----------------------------------|
//! | ExpertRegistered      | 专家注册成功         | expert_id, dimension, domain      |
//! | ConsultStarted        | 咨询请求开始         | consult_id, expert_id, query      |
//! | ConsultCompleted      | 咨询请求完成         | consult_id, score, latency_ms     |
//! | GovernVerdictIssued   | 治理闸门裁决         | gate_id, level, score, reasons    |
//! | AllianceStarted       | 联盟分析开始         | trace_id, team_size, scenario     |
//! | AllianceCompleted     | 联盟分析完成         | trace_id, gate_grade, consensus   |
//! | ExpertDebateHeld      | 专家辩论结束         | trace_id, participants, rounds    |
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mox_ai_expert_proto::events::{EventBuilder, ExpertDomainEvent};
//!
//! let builder = EventBuilder::new("tenant-1", "user-1");
//! let event = builder.consult_completed("c-001", "security", 0.92, false, 1500);
//!
//! // 序列化为 JSON
//! let json = event.to_json().unwrap();
//!
//! // 从 JSON 反序列化
//! let parsed = ExpertDomainEvent::from_json(&json).unwrap();
//! ```

use crate::types::Dimension;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// 领域事件枚举
// ============================================================================

/// 专家领域事件类型
///
/// 使用 `tag` 模式的 enum，JSON 序列化时通过 `event_type` 字段区分事件类型。
/// 下游可通过 `match` 分发给不同的事件处理器。
///
/// 所有事件都包含以下公共字段（通过方法访问）：
/// - `event_id()` — 事件唯一 ID（UUID）
/// - `event_type()` — 事件类型名称
/// - `timestamp()` — 事件发生时间（RFC3339）
/// - `trace_id()` — 分布式追踪 ID
/// - `tenant_id()` — 租户 ID
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum ExpertDomainEvent {
    /// 专家注册事件
    ///
    /// 当一位新专家成功注册到注册表时触发。
    /// 下游可用于刷新专家列表缓存、发送通知等。
    ExpertRegistered {
        /// 事件唯一 ID
        event_id: String,
        /// 事件发生时间（RFC3339）
        timestamp: String,
        /// 分布式追踪 ID
        trace_id: String,
        /// 专家 ID
        expert_id: String,
        /// 专家名称
        expert_name: String,
        /// 所属维度（字符串形式，如 "security"）
        dimension: String,
        /// 所属领域（如 "gov"、"finance"、"*"）
        domain: String,
        /// 注册操作人
        registered_by: String,
    },

    /// 咨询开始事件
    ///
    /// 当一次专家咨询请求开始处理时触发。
    /// 下游可用于统计 QPS、启动超时监控等。
    ConsultStarted {
        /// 事件唯一 ID
        event_id: String,
        /// 事件发生时间（RFC3339）
        timestamp: String,
        /// 分布式追踪 ID
        trace_id: String,
        /// 咨询会话 ID
        consult_id: String,
        /// 专家 ID
        expert_id: String,
        /// 查询预览（截断，不泄露完整内容）
        query_preview: String,
        /// 租户 ID
        tenant_id: String,
    },

    /// 咨询完成事件
    ///
    /// 当一次专家咨询请求完成（成功或失败）时触发。
    /// 下游可用于统计成功率、延迟分布、质量评分等。
    ConsultCompleted {
        /// 事件唯一 ID
        event_id: String,
        /// 事件发生时间（RFC3339）
        timestamp: String,
        /// 分布式追踪 ID
        trace_id: String,
        /// 咨询会话 ID
        consult_id: String,
        /// 专家 ID
        expert_id: String,
        /// 综合评分 0..1
        score: f64,
        /// 是否被否决
        vetoed: bool,
        /// 延迟（毫秒）
        latency_ms: u64,
        /// 租户 ID
        tenant_id: String,
    },

    /// 治理裁决事件
    ///
    /// 当治理闸门对一次请求做出裁决时触发。
    /// 下游可用于审计、合规报告、SLA 监控等。
    GovernVerdictIssued {
        /// 事件唯一 ID
        event_id: String,
        /// 事件发生时间（RFC3339）
        timestamp: String,
        /// 分布式追踪 ID
        trace_id: String,
        /// 闸门 ID
        gate_id: String,
        /// 治理等级（pass / warn / block）
        level: String,
        /// 治理评分 0..1
        score: f64,
        /// 裁决理由列表
        reasons: Vec<String>,
        /// 关联的流程图 ID
        flow_id: String,
        /// 租户 ID
        tenant_id: String,
    },

    /// 联盟分析开始事件
    ///
    /// 当一次多专家联盟分析任务启动时触发。
    AllianceStarted {
        /// 事件唯一 ID
        event_id: String,
        /// 事件发生时间（RFC3339）
        timestamp: String,
        /// 分布式追踪 ID
        trace_id: String,
        /// 参与专家数量
        team_size: usize,
        /// 业务场景
        scenario: String,
        /// 租户 ID
        tenant_id: String,
    },

    /// 联盟分析完成事件
    ///
    /// 当一次多专家联盟分析任务完成时触发。
    /// 下游可用于统计联盟分析质量、性能、通过率等。
    AllianceCompleted {
        /// 事件唯一 ID
        event_id: String,
        /// 事件发生时间（RFC3339）
        timestamp: String,
        /// 分布式追踪 ID
        trace_id: String,
        /// 质量门禁等级
        gate_grade: String,
        /// 是否通过质量门禁
        gate_passed: bool,
        /// 总耗时（毫秒）
        total_ms: u64,
        /// 专家共识度 0..1
        consensus: f64,
        /// 租户 ID
        tenant_id: String,
    },

    /// 专家辩论事件
    ///
    /// 当一轮或多轮专家辩论结束时触发。
    /// 下游可用于分析辩论质量、专家分歧度等。
    ExpertDebateHeld {
        /// 事件唯一 ID
        event_id: String,
        /// 事件发生时间（RFC3339）
        timestamp: String,
        /// 分布式追踪 ID
        trace_id: String,
        /// 参与辩论的专家 ID 列表
        participants: Vec<String>,
        /// 辩论轮次
        rounds: u32,
        /// 最终共识度 0..1
        consensus: f64,
        /// 租户 ID
        tenant_id: String,
    },
}

impl ExpertDomainEvent {
    /// 获取事件唯一 ID
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

    /// 获取事件类型名称（snake_case）
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

    /// 获取事件发生时间（RFC3339 格式）
    pub fn timestamp(&self) -> &str {
        match self {
            ExpertDomainEvent::ExpertRegistered { timestamp, .. } => timestamp,
            ExpertDomainEvent::ConsultStarted { timestamp, .. } => timestamp,
            ExpertDomainEvent::ConsultCompleted { timestamp, .. } => timestamp,
            ExpertDomainEvent::GovernVerdictIssued { timestamp, .. } => timestamp,
            ExpertDomainEvent::AllianceStarted { timestamp, .. } => timestamp,
            ExpertDomainEvent::AllianceCompleted { timestamp, .. } => timestamp,
            ExpertDomainEvent::ExpertDebateHeld { timestamp, .. } => timestamp,
        }
    }

    /// 获取分布式追踪 ID
    pub fn trace_id(&self) -> &str {
        match self {
            ExpertDomainEvent::ExpertRegistered { trace_id, .. } => trace_id,
            ExpertDomainEvent::ConsultStarted { trace_id, .. } => trace_id,
            ExpertDomainEvent::ConsultCompleted { trace_id, .. } => trace_id,
            ExpertDomainEvent::GovernVerdictIssued { trace_id, .. } => trace_id,
            ExpertDomainEvent::AllianceStarted { trace_id, .. } => trace_id,
            ExpertDomainEvent::AllianceCompleted { trace_id, .. } => trace_id,
            ExpertDomainEvent::ExpertDebateHeld { trace_id, .. } => trace_id,
        }
    }

    /// 获取租户 ID
    ///
    /// 对于 `ExpertRegistered` 等系统级事件，返回 `"system"`。
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

    /// 序列化为带缩进的 JSON 字符串（便于调试 / 日志）
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 从 JSON 字符串反序列化
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 判断事件是否为终端事件（表示某个流程已结束）
    ///
    /// 终端事件：ConsultCompleted、AllianceCompleted、GovernVerdictIssued、ExpertDebateHeld
    /// 开始事件：ConsultStarted、AllianceStarted
    /// 注册事件：ExpertRegistered（生命周期事件）
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ExpertDomainEvent::ConsultCompleted { .. }
                | ExpertDomainEvent::AllianceCompleted { .. }
                | ExpertDomainEvent::GovernVerdictIssued { .. }
                | ExpertDomainEvent::ExpertDebateHeld { .. }
        )
    }

    /// 获取事件的人类可读描述
    pub fn description(&self) -> String {
        match self {
            ExpertDomainEvent::ExpertRegistered { expert_name, expert_id, .. } => {
                format!("专家 [{} ({})] 已注册", expert_name, expert_id)
            }
            ExpertDomainEvent::ConsultStarted { consult_id, expert_id, .. } => {
                format!("咨询 [{}] 开始，专家: {}", consult_id, expert_id)
            }
            ExpertDomainEvent::ConsultCompleted { consult_id, expert_id, score, vetoed, .. } => {
                format!(
                    "咨询 [{}] 完成，专家: {}，分数: {:.2}，否决: {}",
                    consult_id, expert_id, score, vetoed
                )
            }
            ExpertDomainEvent::GovernVerdictIssued { gate_id, level, score, .. } => {
                format!("治理闸门 [{}] 裁决: {} (分数: {:.2})", gate_id, level, score)
            }
            ExpertDomainEvent::AllianceStarted { scenario, team_size, .. } => {
                format!("联盟分析启动，场景: {}，专家数: {}", scenario, team_size)
            }
            ExpertDomainEvent::AllianceCompleted { gate_grade, gate_passed, consensus, .. } => {
                format!(
                    "联盟分析完成，等级: {}，通过: {}，共识: {:.2}",
                    gate_grade, gate_passed, consensus
                )
            }
            ExpertDomainEvent::ExpertDebateHeld { participants, rounds, consensus, .. } => {
                format!(
                    "专家辩论结束，参与: {} 人，轮次: {}，共识: {:.2}",
                    participants.len(),
                    rounds,
                    consensus
                )
            }
        }
    }
}

// ============================================================================
// 事件构建器（EventBuilder 模式）
// ============================================================================

/// 领域事件构建器
///
/// 提供流式 API 构造各类领域事件。
/// 封装 event_id、timestamp、trace_id 等公共字段的生成逻辑，
/// 使业务代码专注于业务字段的填充。
///
/// # 示例
///
/// ```rust,ignore
/// let builder = EventBuilder::new("tenant-1", "user-1");
///
/// // 构造咨询完成事件
/// let event = builder.consult_completed("c-001", "security", 0.92, false, 1500);
///
/// // 构造联盟完成事件
/// let event = builder
///     .with_trace_id("trace-abc")
///     .alliance_completed("A", true, 5000, 0.85);
/// ```
pub struct EventBuilder {
    tenant_id: String,
    operator: String,
    trace_id: Option<String>,
}

impl EventBuilder {
    /// 创建新的事件构建器
    ///
    /// # 参数
    /// - `tenant_id`: 租户 ID
    /// - `operator`: 操作人 / 触发者标识
    pub fn new(tenant_id: impl Into<String>, operator: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            operator: operator.into(),
            trace_id: None,
        }
    }

    /// 设置自定义 trace_id（不设置则自动生成 UUID）
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// 获取或生成 trace_id
    fn trace_id_or_default(&self) -> String {
        self.trace_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string())
    }

    // ------------------------------------------------------------------
    // ExpertRegistered
    // ------------------------------------------------------------------

    /// 构建「专家注册」事件
    pub fn expert_registered(
        &self,
        expert_id: &str,
        expert_name: &str,
        dimension: Dimension,
        domain: &str,
    ) -> ExpertDomainEvent {
        ExpertDomainEvent::ExpertRegistered {
            event_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: self.trace_id_or_default(),
            expert_id: expert_id.to_string(),
            expert_name: expert_name.to_string(),
            dimension: dimension_name(dimension),
            domain: domain.to_string(),
            registered_by: self.operator.clone(),
        }
    }

    // ------------------------------------------------------------------
    // ConsultStarted
    // ------------------------------------------------------------------

    /// 构建「咨询开始」事件
    pub fn consult_started(
        &self,
        consult_id: &str,
        expert_id: &str,
        query_preview: &str,
    ) -> ExpertDomainEvent {
        ExpertDomainEvent::ConsultStarted {
            event_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: self.trace_id_or_default(),
            consult_id: consult_id.to_string(),
            expert_id: expert_id.to_string(),
            query_preview: truncate_preview(query_preview, 100),
            tenant_id: self.tenant_id.clone(),
        }
    }

    // ------------------------------------------------------------------
    // ConsultCompleted
    // ------------------------------------------------------------------

    /// 构建「咨询完成」事件
    pub fn consult_completed(
        &self,
        consult_id: &str,
        expert_id: &str,
        score: f64,
        vetoed: bool,
        latency_ms: u64,
    ) -> ExpertDomainEvent {
        ExpertDomainEvent::ConsultCompleted {
            event_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: self.trace_id_or_default(),
            consult_id: consult_id.to_string(),
            expert_id: expert_id.to_string(),
            score,
            vetoed,
            latency_ms,
            tenant_id: self.tenant_id.clone(),
        }
    }

    // ------------------------------------------------------------------
    // GovernVerdictIssued
    // ------------------------------------------------------------------

    /// 构建「治理裁决」事件
    pub fn govern_verdict_issued(
        &self,
        gate_id: &str,
        level: &str,
        score: f64,
        reasons: Vec<String>,
        flow_id: &str,
    ) -> ExpertDomainEvent {
        ExpertDomainEvent::GovernVerdictIssued {
            event_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: self.trace_id_or_default(),
            gate_id: gate_id.to_string(),
            level: level.to_string(),
            score,
            reasons,
            flow_id: flow_id.to_string(),
            tenant_id: self.tenant_id.clone(),
        }
    }

    // ------------------------------------------------------------------
    // AllianceStarted
    // ------------------------------------------------------------------

    /// 构建「联盟分析开始」事件
    pub fn alliance_started(
        &self,
        trace_id: &str,
        team_size: usize,
        scenario: &str,
    ) -> ExpertDomainEvent {
        ExpertDomainEvent::AllianceStarted {
            event_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.to_string(),
            team_size,
            scenario: scenario.to_string(),
            tenant_id: self.tenant_id.clone(),
        }
    }

    // ------------------------------------------------------------------
    // AllianceCompleted
    // ------------------------------------------------------------------

    /// 构建「联盟分析完成」事件
    pub fn alliance_completed(
        &self,
        trace_id: &str,
        gate_grade: &str,
        gate_passed: bool,
        total_ms: u64,
        consensus: f64,
    ) -> ExpertDomainEvent {
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

    // ------------------------------------------------------------------
    // ExpertDebateHeld
    // ------------------------------------------------------------------

    /// 构建「专家辩论」事件
    pub fn expert_debate_held(
        &self,
        trace_id: &str,
        participants: Vec<String>,
        rounds: u32,
        consensus: f64,
    ) -> ExpertDomainEvent {
        ExpertDomainEvent::ExpertDebateHeld {
            event_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.to_string(),
            participants,
            rounds,
            consensus,
            tenant_id: self.tenant_id.clone(),
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 获取维度名称（小写 snake_case，与 serde 序列化一致）
fn dimension_name(dim: Dimension) -> String {
    // Dimension 的 serde 重命名规则是 snake_case
    // 这里手动映射以避免依赖 serde_json 做转换
    match dim {
        Dimension::Business => "business",
        Dimension::Algorithm => "algorithm",
        Dimension::Permission => "permission",
        Dimension::Resource => "resource",
        Dimension::Security => "security",
        Dimension::Data => "data",
        Dimension::Observability => "observability",
        Dimension::Architecture => "architecture",
        Dimension::SecurityCode => "security_code",
        Dimension::CodeQuality => "code_quality",
        Dimension::Performance => "performance",
        Dimension::Testing => "testing",
        Dimension::Documentation => "documentation",
        Dimension::Maintainability => "maintainability",
    }
    .to_string()
}

/// 截断查询预览文本（避免日志中泄露过长内容）
fn truncate_preview(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- 事件类型与序列化 --

    #[test]
    fn event_type_serialization_tag() {
        // 验证每种事件的 event_type 标签正确
        let events = vec![
            (ExpertDomainEvent::ExpertRegistered {
                event_id: "e1".into(),
                timestamp: "t1".into(),
                trace_id: "tr1".into(),
                expert_id: "ex1".into(),
                expert_name: "专家1".into(),
                dimension: "security".into(),
                domain: "gov".into(),
                registered_by: "admin".into(),
            }, "expert_registered"),
            (ExpertDomainEvent::ConsultStarted {
                event_id: "e2".into(),
                timestamp: "t2".into(),
                trace_id: "tr2".into(),
                consult_id: "c1".into(),
                expert_id: "ex1".into(),
                query_preview: "q".into(),
                tenant_id: "t1".into(),
            }, "consult_started"),
            (ExpertDomainEvent::ConsultCompleted {
                event_id: "e3".into(),
                timestamp: "t3".into(),
                trace_id: "tr3".into(),
                consult_id: "c1".into(),
                expert_id: "ex1".into(),
                score: 0.9,
                vetoed: false,
                latency_ms: 100,
                tenant_id: "t1".into(),
            }, "consult_completed"),
            (ExpertDomainEvent::GovernVerdictIssued {
                event_id: "e4".into(),
                timestamp: "t4".into(),
                trace_id: "tr4".into(),
                gate_id: "g1".into(),
                level: "pass".into(),
                score: 0.95,
                reasons: vec![],
                flow_id: "f1".into(),
                tenant_id: "t1".into(),
            }, "govern_verdict_issued"),
            (ExpertDomainEvent::AllianceStarted {
                event_id: "e5".into(),
                timestamp: "t5".into(),
                trace_id: "tr5".into(),
                team_size: 3,
                scenario: "test".into(),
                tenant_id: "t1".into(),
            }, "alliance_started"),
            (ExpertDomainEvent::AllianceCompleted {
                event_id: "e6".into(),
                timestamp: "t6".into(),
                trace_id: "tr6".into(),
                gate_grade: "A".into(),
                gate_passed: true,
                total_ms: 1000,
                consensus: 0.8,
                tenant_id: "t1".into(),
            }, "alliance_completed"),
            (ExpertDomainEvent::ExpertDebateHeld {
                event_id: "e7".into(),
                timestamp: "t7".into(),
                trace_id: "tr7".into(),
                participants: vec!["a".into(), "b".into()],
                rounds: 3,
                consensus: 0.7,
                tenant_id: "t1".into(),
            }, "expert_debate_held"),
        ];

        for (event, expected_type) in events {
            assert_eq!(event.event_type(), expected_type);
            let json = event.to_json().unwrap();
            assert!(
                json.contains(&format!("\"event_type\":\"{}\"", expected_type)),
                "JSON 中应包含 event_type={}: {}",
                expected_type,
                json
            );
        }
    }

    #[test]
    fn consult_completed_roundtrip() {
        let event = ExpertDomainEvent::ConsultCompleted {
            event_id: "evt-123".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            trace_id: "trace-abc".into(),
            consult_id: "c-1".into(),
            expert_id: "security".into(),
            score: 0.85,
            vetoed: false,
            latency_ms: 1200,
            tenant_id: "tenant-a".into(),
        };

        let json = event.to_json().unwrap();
        assert!(json.contains("\"score\":0.85"));
        assert!(json.contains("\"trace_id\":\"trace-abc\""));

        let parsed = ExpertDomainEvent::from_json(&json).unwrap();
        assert_eq!(parsed.event_type(), "consult_completed");
        assert_eq!(parsed.event_id(), "evt-123");
        assert_eq!(parsed.trace_id(), "trace-abc");
        assert_eq!(parsed.tenant_id(), "tenant-a");
    }

    // -- 公共字段访问器 --

    #[test]
    fn all_events_have_event_id() {
        let builder = EventBuilder::new("test", "op");
        let events: Vec<ExpertDomainEvent> = vec![
            builder.expert_registered("e1", "专家", Dimension::Security, "gov"),
            builder.consult_started("c1", "e1", "hello"),
            builder.consult_completed("c1", "e1", 0.9, false, 100),
            builder.govern_verdict_issued("g1", "pass", 0.95, vec![], "f1"),
            builder.alliance_started("tr1", 3, "test"),
            builder.alliance_completed("tr1", "A", true, 5000, 0.85),
            builder.expert_debate_held("tr1", vec!["a".into(), "b".into()], 2, 0.7),
        ];

        for event in &events {
            assert!(!event.event_id().is_empty(), "{} 应有 event_id", event.event_type());
            assert!(!event.timestamp().is_empty(), "{} 应有 timestamp", event.event_type());
            assert!(!event.trace_id().is_empty(), "{} 应有 trace_id", event.event_type());
        }
    }

    #[test]
    fn tenant_id_is_correct() {
        let builder = EventBuilder::new("tenant-x", "op");

        // 系统级事件：tenant_id 为 "system"
        let reg = builder.expert_registered("e1", "专家", Dimension::Security, "gov");
        assert_eq!(reg.tenant_id(), "system");

        // 业务事件：tenant_id 为 builder 中设置的值
        let started = builder.consult_started("c1", "e1", "q");
        assert_eq!(started.tenant_id(), "tenant-x");

        let completed = builder.consult_completed("c1", "e1", 0.9, false, 100);
        assert_eq!(completed.tenant_id(), "tenant-x");
    }

    // -- EventBuilder 完整覆盖测试 --

    #[test]
    fn event_builder_expert_registered() {
        let builder = EventBuilder::new("t1", "admin");
        let event = builder.expert_registered("sec-001", "安全专家", Dimension::Security, "gov");

        assert_eq!(event.event_type(), "expert_registered");
        assert!(!event.event_id().is_empty());
        assert!(!event.timestamp().is_empty());
        assert!(!event.trace_id().is_empty());

        if let ExpertDomainEvent::ExpertRegistered { expert_id, expert_name, dimension, domain, registered_by, .. } = &event {
            assert_eq!(expert_id, "sec-001");
            assert_eq!(expert_name, "安全专家");
            assert_eq!(dimension, "security");
            assert_eq!(domain, "gov");
            assert_eq!(registered_by, "admin");
        } else {
            panic!("应该是 ExpertRegistered 类型");
        }
    }

    #[test]
    fn event_builder_consult_started() {
        let builder = EventBuilder::new("t1", "user-1");
        let event = builder.consult_started("c-42", "algorithm", "请分析这个流程图");

        assert_eq!(event.event_type(), "consult_started");
        if let ExpertDomainEvent::ConsultStarted { consult_id, expert_id, query_preview, .. } = &event {
            assert_eq!(consult_id, "c-42");
            assert_eq!(expert_id, "algorithm");
            assert_eq!(query_preview, "请分析这个流程图");
        } else {
            panic!("应该是 ConsultStarted 类型");
        }
    }

    #[test]
    fn event_builder_consult_completed() {
        let builder = EventBuilder::new("t1", "user-1");
        let event = builder.consult_completed("c-42", "security", 0.92, false, 1500);

        assert_eq!(event.event_type(), "consult_completed");
        if let ExpertDomainEvent::ConsultCompleted { score, vetoed, latency_ms, .. } = &event {
            assert!((score - 0.92).abs() < 1e-9);
            assert!(!vetoed);
            assert_eq!(*latency_ms, 1500);
        } else {
            panic!("应该是 ConsultCompleted 类型");
        }
    }

    #[test]
    fn event_builder_govern_verdict_issued() {
        let builder = EventBuilder::new("t1", "gov-engine");
        let reasons = vec!["SLA 超限".into(), "预算不足".into()];
        let event = builder.govern_verdict_issued("main-gate", "warn", 0.65, reasons.clone(), "flow-1");

        assert_eq!(event.event_type(), "govern_verdict_issued");
        if let ExpertDomainEvent::GovernVerdictIssued { gate_id, level, score, reasons, flow_id, .. } = &event {
            assert_eq!(gate_id, "main-gate");
            assert_eq!(level, "warn");
            assert!((score - 0.65).abs() < 1e-9);
            assert_eq!(reasons.len(), 2);
            assert_eq!(flow_id, "flow-1");
        } else {
            panic!("应该是 GovernVerdictIssued 类型");
        }
    }

    #[test]
    fn event_builder_alliance_started() {
        let builder = EventBuilder::new("t1", "orchestrator");
        let event = builder.alliance_started("trace-xyz", 5, "gov-pii");

        assert_eq!(event.event_type(), "alliance_started");
        if let ExpertDomainEvent::AllianceStarted { team_size, scenario, trace_id, .. } = &event {
            assert_eq!(*team_size, 5);
            assert_eq!(scenario, "gov-pii");
            assert_eq!(trace_id, "trace-xyz");
        } else {
            panic!("应该是 AllianceStarted 类型");
        }
    }

    #[test]
    fn event_builder_alliance_completed() {
        let builder = EventBuilder::new("t1", "orchestrator");
        let event = builder.alliance_completed("trace-xyz", "S", true, 8000, 0.91);

        assert_eq!(event.event_type(), "alliance_completed");
        if let ExpertDomainEvent::AllianceCompleted { gate_grade, gate_passed, total_ms, consensus, .. } = &event {
            assert_eq!(gate_grade, "S");
            assert!(gate_passed);
            assert_eq!(*total_ms, 8000);
            assert!((consensus - 0.91).abs() < 1e-9);
        } else {
            panic!("应该是 AllianceCompleted 类型");
        }
    }

    #[test]
    fn event_builder_expert_debate_held() {
        let builder = EventBuilder::new("t1", "debate-engine");
        let participants = vec!["security".into(), "performance".into(), "architecture".into()];
        let event = builder.expert_debate_held("trace-d", participants.clone(), 5, 0.72);

        assert_eq!(event.event_type(), "expert_debate_held");
        if let ExpertDomainEvent::ExpertDebateHeld { participants, rounds, consensus, .. } = &event {
            assert_eq!(participants.len(), 3);
            assert_eq!(*rounds, 5);
            assert!((consensus - 0.72).abs() < 1e-9);
        } else {
            panic!("应该是 ExpertDebateHeld 类型");
        }
    }

    // -- with_trace_id 覆盖测试 --

    #[test]
    fn builder_with_trace_id_overrides_default() {
        let builder = EventBuilder::new("t1", "op").with_trace_id("custom-trace-123");
        let event = builder.consult_completed("c1", "e1", 0.9, false, 100);
        assert_eq!(event.trace_id(), "custom-trace-123");
    }

    #[test]
    fn builder_without_trace_id_generates_uuid() {
        let builder = EventBuilder::new("t1", "op");
        let event = builder.consult_completed("c1", "e1", 0.9, false, 100);
        // UUID v4 格式
        assert_eq!(event.trace_id().len(), 36);
    }

    // -- 终端事件判断 --

    #[test]
    fn terminal_event_detection() {
        let builder = EventBuilder::new("t1", "op");

        // 开始事件：不是终端事件
        assert!(!builder.consult_started("c1", "e1", "q").is_terminal());
        assert!(!builder.alliance_started("tr1", 3, "s").is_terminal());

        // 注册事件：不是终端事件（生命周期事件）
        assert!(!builder.expert_registered("e1", "专家", Dimension::Security, "gov").is_terminal());

        // 完成/裁决事件：是终端事件
        assert!(builder.consult_completed("c1", "e1", 0.9, false, 100).is_terminal());
        assert!(builder.alliance_completed("tr1", "A", true, 1000, 0.8).is_terminal());
        assert!(builder.govern_verdict_issued("g1", "pass", 0.9, vec![], "f1").is_terminal());
        assert!(builder.expert_debate_held("tr1", vec!["a".into()], 1, 0.7).is_terminal());
    }

    // -- 描述文本 --

    #[test]
    fn description_is_human_readable() {
        let builder = EventBuilder::new("t1", "op");

        let desc = builder.consult_completed("c1", "sec", 0.85, false, 200).description();
        assert!(desc.contains("咨询"));
        assert!(desc.contains("完成"));
        assert!(desc.contains("0.85"));

        let desc2 = builder.alliance_completed("tr1", "A", true, 5000, 0.92).description();
        assert!(desc2.contains("联盟"));
        assert!(desc2.contains("完成"));
        assert!(desc2.contains("A"));
    }

    // -- 查询预览截断 --

    #[test]
    fn query_preview_truncation() {
        let long_query = "a".repeat(200);
        let builder = EventBuilder::new("t1", "op");
        let event = builder.consult_started("c1", "e1", &long_query);

        if let ExpertDomainEvent::ConsultStarted { query_preview, .. } = &event {
            // 截断到 100 字符 + "..." = 103
            assert_eq!(query_preview.len(), 103);
            assert!(query_preview.ends_with("..."));
        } else {
            panic!("应该是 ConsultStarted 类型");
        }
    }

    #[test]
    fn short_query_not_truncated() {
        let short_query = "hello";
        let builder = EventBuilder::new("t1", "op");
        let event = builder.consult_started("c1", "e1", short_query);

        if let ExpertDomainEvent::ConsultStarted { query_preview, .. } = &event {
            assert_eq!(query_preview, "hello");
            assert!(!query_preview.ends_with("..."));
        } else {
            panic!("应该是 ConsultStarted 类型");
        }
    }

    // -- Pretty JSON --

    #[test]
    fn pretty_json_contains_newlines() {
        let builder = EventBuilder::new("t1", "op");
        let event = builder.consult_completed("c1", "e1", 0.9, false, 100);
        let pretty = event.to_json_pretty().unwrap();
        assert!(pretty.contains('\n'), "pretty JSON 应包含换行符");
    }

    // -- 所有维度的 dimension_name --

    #[test]
    fn dimension_name_matches_all_variants() {
        use Dimension::*;
        let dims = vec![
            (Business, "business"),
            (Algorithm, "algorithm"),
            (Permission, "permission"),
            (Resource, "resource"),
            (Security, "security"),
            (Data, "data"),
            (Observability, "observability"),
            (Architecture, "architecture"),
            (SecurityCode, "security_code"),
            (CodeQuality, "code_quality"),
            (Performance, "performance"),
            (Testing, "testing"),
            (Documentation, "documentation"),
            (Maintainability, "maintainability"),
        ];

        for (dim, expected) in dims {
            assert_eq!(dimension_name(dim), expected);
        }
    }

    // -- 序列化反序列化一致性 --

    #[test]
    fn all_event_types_roundtrip() {
        let builder = EventBuilder::new("tenant-test", "op-test");
        let events: Vec<ExpertDomainEvent> = vec![
            builder.expert_registered("e1", "专家1", Dimension::Security, "gov"),
            builder.consult_started("c1", "e1", "测试查询"),
            builder.consult_completed("c1", "e1", 0.88, true, 2500),
            builder.govern_verdict_issued("g1", "block", 0.3, vec!["违规A".into()], "f-99"),
            builder.alliance_started("tr-001", 7, "gov-pii"),
            builder.alliance_completed("tr-001", "B", false, 12000, 0.65),
            builder.expert_debate_held("tr-002", vec!["a".into(), "b".into(), "c".into()], 4, 0.55),
        ];

        for event in events {
            let event_type = event.event_type().to_string();
            let json = event.to_json().unwrap();
            let parsed = ExpertDomainEvent::from_json(&json).unwrap_or_else(|e| {
                panic!("{} 反序列化失败: {}\nJSON: {}", event_type, e, json)
            });
            assert_eq!(parsed.event_type(), event_type, "事件类型应一致");
            assert_eq!(parsed.event_id(), event.event_id(), "event_id 应一致");
            assert_eq!(parsed.trace_id(), event.trace_id(), "trace_id 应一致");
            assert_eq!(parsed.tenant_id(), event.tenant_id(), "tenant_id 应一致");
        }
    }
}

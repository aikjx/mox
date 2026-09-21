// =============================================================================
// 统一事件格式（AllianceEvent / StreamEvent）
// =============================================================================
// 跨端对齐：Python 和 前端必须使用相同的事件 JSON 结构。
// =============================================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// 事件阶段（7阶段管线）
// =============================================================================

/// 事件阶段（7阶段管线）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPhase {
    /// 意图识别
    Intent = 0,
    /// 组队匹配
    Team = 1,
    /// 专家辩论
    Debate = 2,
    /// 综合归纳
    Synthesize = 3,
    /// 质量门禁
    Gate = 4,
    /// 知识学习
    Learn = 5,
    /// 完成
    Done = 6,
}

impl EventPhase {
    /// 阶段顺序下标（0-6）
    pub fn index(&self) -> usize {
        *self as usize
    }

    /// 阶段名（snake_case，用于 SSE event 字段）
    pub fn name(&self) -> &'static str {
        match self {
            EventPhase::Intent => "intent",
            EventPhase::Team => "team",
            EventPhase::Debate => "debate",
            EventPhase::Synthesize => "synthesize",
            EventPhase::Gate => "gate",
            EventPhase::Learn => "learn",
            EventPhase::Done => "done",
        }
    }

    /// 阶段中文名
    pub fn label(&self) -> &'static str {
        match self {
            EventPhase::Intent => "意图识别",
            EventPhase::Team => "组队匹配",
            EventPhase::Debate => "专家辩论",
            EventPhase::Synthesize => "综合归纳",
            EventPhase::Gate => "质量门禁",
            EventPhase::Learn => "知识学习",
            EventPhase::Done => "完成",
        }
    }

    /// 阶段图标（前端用）
    pub fn icon(&self) -> &'static str {
        match self {
            EventPhase::Intent => "🎯",
            EventPhase::Team => "👥",
            EventPhase::Debate => "💬",
            EventPhase::Synthesize => "📝",
            EventPhase::Gate => "🚦",
            EventPhase::Learn => "🧠",
            EventPhase::Done => "✅",
        }
    }

    /// 阶段颜色（前端用）
    pub fn color(&self) -> &'static str {
        match self {
            EventPhase::Intent => "#6366f1",
            EventPhase::Team => "#06b6d4",
            EventPhase::Debate => "#f59e0b",
            EventPhase::Synthesize => "#10b981",
            EventPhase::Gate => "#ef4444",
            EventPhase::Learn => "#8b5cf6",
            EventPhase::Done => "#10b981",
        }
    }

    /// 所有阶段（按顺序）
    pub fn all() -> [EventPhase; 7] {
        [
            EventPhase::Intent,
            EventPhase::Team,
            EventPhase::Debate,
            EventPhase::Synthesize,
            EventPhase::Gate,
            EventPhase::Learn,
            EventPhase::Done,
        ]
    }
}

impl std::fmt::Display for EventPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// 阶段名与审计事件名（SSOT-5）
// =============================================================================
//
// 历史问题：`PHASE_NAMES` 在 AI 引擎与专家服务各有一份常量数组、`AUDIT_EVENTS_7`
// 同样两份，且对外契约（orchestrator `/ai/engine/alliance/capabilities`）混用两个
// crate 的同义常量。三份副本值虽相同，但没有任何机制阻止它们各自漂移。
//
// 收敛方式：本文件是唯一真源，各域改为引用；一致性由下方用例守护。

/// 7 阶段管线名（稳定字符串，用于事件与审计）
///
/// 必须与 `EventPhase::all()` 的 `name()` 严格一一对应
/// （由 `phase_names_match_event_phase` 用例守护）。
pub const PHASE_NAMES: [&str; 7] = [
    "intent",     // 01 意图识别
    "team",       // 02 组队路由
    "debate",     // 03 并行咨询 + 辩论
    "synthesize", // 04 归一合成
    "gate",       // 05 质量门禁
    "learn",      // 06 指标学习
    "done",       // 07 终态
];

/// 7 类审计事件名（FR-CORE-07，缺任意一项企业基线不过）
pub const AUDIT_EVENTS_7: [&str; 7] = [
    "ALLIANCE_START",
    "INTENT_DONE",
    "TEAM_DONE",
    "DEBATE_DONE",
    "GATE_DONE",
    "LEARN_DONE",
    "ALLIANCE_DONE",
];

// =============================================================================
// 事件类型
// =============================================================================

/// 事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// 阶段开始
    PhaseStarted,
    /// 阶段数据
    PhaseData,
    /// 进度更新
    Progress,
    /// 完成
    Complete,
    /// 错误
    Error,
    /// 审计事件
    Audit,
}

// =============================================================================
// 统一事件（MoxEvent）
// =============================================================================

/// MOX 统一事件格式
///
/// 所有业务域的事件必须使用此格式，禁止自定义事件结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoxEvent {
    /// 事件唯一 ID
    pub event_id: Uuid,
    /// 事件类型
    pub event_type: EventType,
    /// 事件阶段（7阶段之一）
    pub phase: EventPhase,
    /// 全局追踪 ID（全链路透传）
    pub trace_id: Uuid,
    /// 事件载荷（JSON，各阶段自定义结构）
    pub payload: serde_json::Value,
    /// 阶段耗时（毫秒）
    #[serde(default)]
    pub latency_ms: u64,
    /// 事件时间戳（ISO-8601）
    pub timestamp: String,
    /// 是否降级模式
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degraded: Option<bool>,
    /// 降级原因
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degrade_reason: Option<String>,
}

impl MoxEvent {
    /// 创建新事件
    pub fn new(
        event_type: EventType,
        phase: EventPhase,
        trace_id: Uuid,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type,
            phase,
            trace_id,
            payload,
            latency_ms: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            degraded: None,
            degrade_reason: None,
        }
    }

    /// 设置耗时
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// 标记降级
    pub fn with_degraded(mut self, reason: impl Into<String>) -> Self {
        self.degraded = Some(true);
        self.degrade_reason = Some(reason.into());
        self
    }

    /// 序列化为 SSE 格式
    pub fn to_sse(&self) -> String {
        let data = serde_json::to_string(self).unwrap_or_default();
        format!("event: {}\ndata: {}\n\n", self.phase.name(), data)
    }
}

// =============================================================================
// 流式事件（StreamEvent）- SSE 专用
// =============================================================================

/// SSE 流式事件（轻量级，用于前端 EventSource 解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// 阶段名（snake_case）
    pub phase: String,
    /// 追踪 ID
    pub trace_id: String,
    /// 事件载荷
    #[serde(default)]
    pub payload: serde_json::Value,
    /// 阶段耗时
    #[serde(default)]
    pub latency_ms: u64,
    /// 是否降级
    #[serde(default)]
    pub degraded: bool,
}

impl StreamEvent {
    /// 从 MoxEvent 转换
    pub fn from_mox_event(event: &MoxEvent) -> Self {
        Self {
            phase: event.phase.name().to_string(),
            trace_id: event.trace_id.to_string(),
            payload: event.payload.clone(),
            latency_ms: event.latency_ms,
            degraded: event.degraded.unwrap_or(false),
        }
    }
}

// =============================================================================
// 进度事件（ProgressEvent）
// =============================================================================

/// 进度事件（用于辩论等长耗时阶段的进度更新）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// 阶段
    pub phase: String,
    /// 当前进度
    pub current: u32,
    /// 总进度
    pub total: u32,
    /// 进度消息
    #[serde(default)]
    pub message: String,
    /// 追踪 ID
    pub trace_id: String,
}

impl ProgressEvent {
    /// 百分比（0-100）
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.current as f64 / self.total as f64) * 100.0
        }
    }
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_phase_order() {
        assert_eq!(EventPhase::Intent.index(), 0);
        assert_eq!(EventPhase::Done.index(), 6);
        assert!(EventPhase::Intent < EventPhase::Team);
    }

    #[test]
    fn event_phase_names() {
        assert_eq!(EventPhase::Intent.name(), "intent");
        assert_eq!(EventPhase::Debate.name(), "debate");
        assert_eq!(EventPhase::Done.name(), "done");
    }

    #[test]
    fn all_phases_count() {
        assert_eq!(EventPhase::all().len(), 7);
    }

    #[test]
    fn phase_names_match_event_phase() {
        // SSOT-5 守护：常量数组与阶段枚举必须严格一一对应，防止两处漂移
        let all = EventPhase::all();
        assert_eq!(PHASE_NAMES.len(), all.len());
        for (i, p) in all.iter().enumerate() {
            assert_eq!(PHASE_NAMES[i], p.name(), "PHASE_NAMES[{}] 与 EventPhase 不一致", i);
        }
    }

    #[test]
    fn audit_events_are_unique_and_stable() {
        // 审计事件名不得重复（否则审计计数与去重会失真），且端点顺序稳定
        let mut seen = std::collections::HashSet::new();
        for e in AUDIT_EVENTS_7.iter() {
            assert!(seen.insert(*e), "审计事件名重复: {}", e);
        }
        assert_eq!(AUDIT_EVENTS_7.len(), 7);
        assert_eq!(AUDIT_EVENTS_7[0], "ALLIANCE_START");
        assert_eq!(AUDIT_EVENTS_7[6], "ALLIANCE_DONE");
    }

    #[test]
    fn mox_event_creation() {
        let trace_id = Uuid::new_v4();
        let event = MoxEvent::new(
            EventType::PhaseData,
            EventPhase::Intent,
            trace_id,
            serde_json::json!({"intent": "code"}),
        );
        assert_eq!(event.phase, EventPhase::Intent);
        assert_eq!(event.trace_id, trace_id);
        assert_eq!(event.payload["intent"], "code");
    }

    #[test]
    fn mox_event_sse_format() {
        let trace_id = Uuid::new_v4();
        let event = MoxEvent::new(
            EventType::PhaseData,
            EventPhase::Debate,
            trace_id,
            serde_json::json!({}),
        );
        let sse = event.to_sse();
        assert!(sse.starts_with("event: debate\n"));
        assert!(sse.contains("data: "));
        assert!(sse.ends_with("\n\n"));
    }

    #[test]
    fn progress_event_percentage() {
        let progress = ProgressEvent {
            phase: "debate".to_string(),
            current: 2,
            total: 4,
            message: "第 2/4 位专家".to_string(),
            trace_id: "test".to_string(),
        };
        assert!((progress.percentage() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_phase_serialization() {
        let json = serde_json::to_string(&EventPhase::Intent).unwrap();
        assert_eq!(json, "\"intent\"");
        let parsed: EventPhase = serde_json::from_str("\"debate\"").unwrap();
        assert_eq!(parsed, EventPhase::Debate);
    }
}

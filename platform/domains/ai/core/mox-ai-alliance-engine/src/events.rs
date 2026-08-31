// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 联盟事件系统（SSE 流式输出 + 审计事件）
//!
//! - `AlliancePhase` — 7 阶段枚举（Intent/Team/Debate/Synthesize/Gate/Learn/Done）
//! - `AllianceEvent` — SSE 每帧事件（trace_id 全链路一致）
//! - `AuditEvent` — 7 类审计事件（FR-CORE-07）
//! - `StreamEvent` — 流式事件枚举（包含进度、数据、错误、完成）

use crate::constants::{AUDIT_EVENTS_7, PHASE_NAMES};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

// ================== 管线阶段枚举 ==================

/// 管线阶段枚举：与 [`PHASE_NAMES`] 索引严格一一对应
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlliancePhase {
    Intent = 0,
    Team = 1,
    Debate = 2,
    Synthesize = 3,
    Gate = 4,
    Learn = 5,
    Done = 6,
}

impl AlliancePhase {
    /// 返回阶段在 7 个事件流中的顺序下标（0..=6）
    pub fn index(&self) -> usize {
        *self as usize
    }

    /// 返回稳定的阶段名（用于 SSE event 字段与审计键）
    pub fn name(&self) -> &'static str {
        PHASE_NAMES[self.index()]
    }

    /// 下一阶段，Done 返回自己（循环安全）
    pub fn next(&self) -> Self {
        match self {
            Self::Intent => Self::Team,
            Self::Team => Self::Debate,
            Self::Debate => Self::Synthesize,
            Self::Synthesize => Self::Gate,
            Self::Gate => Self::Learn,
            Self::Learn | Self::Done => Self::Done,
        }
    }

    /// 是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done)
    }

    /// 所有阶段的有序列表
    pub fn all() -> &'static [AlliancePhase] {
        const ALL: [AlliancePhase; 7] = [
            AlliancePhase::Intent,
            AlliancePhase::Team,
            AlliancePhase::Debate,
            AlliancePhase::Synthesize,
            AlliancePhase::Gate,
            AlliancePhase::Learn,
            AlliancePhase::Done,
        ];
        &ALL
    }
}

// ================== SSE 事件 ==================

/// 全维分析事件（SSE 每帧一条；trace_id 全链路一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceEvent {
    /// 当前阶段（7 种之一）
    pub phase: AlliancePhase,
    /// 该阶段结果负载（IntentResult / TeamResult / ... / GateResult / LearnResult）
    pub payload: serde_json::Value,
    /// 全局唯一 trace id（UUID v4），贯穿 6 阶段 + 审计汇
    pub trace_id: Uuid,
    /// 该阶段耗时毫秒（从阶段开始到事件发出）
    pub latency_ms: u64,
    /// 事件生成时间戳（UTC ISO-8601 可溯源）
    pub ts: DateTime<Utc>,
    /// 是否为降级模式（如 graph 不可用 / ai-agent 不可用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degraded: Option<bool>,
    /// 降级原因（非空当且仅当 degraded=Some(true)）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degrade_reason: Option<String>,
}

// ================== 审计事件 ==================

/// 审计事件（FR-CORE-07 7 类）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event: String,
    pub trace_id: Uuid,
    pub ts_ms: u128,
    pub payload: serde_json::Value,
}

impl AuditEvent {
    /// 7 类审计事件名（与 AUDIT_EVENTS_7 常量对齐）
    pub fn event_names() -> &'static [&'static str] {
        &AUDIT_EVENTS_7
    }
}

// ================== 流式事件类型 ==================

/// 流式事件枚举（用于 SSE / tokio::sync::mpsc 流式输出）
///
/// 前端可按事件类型渲染不同 UI：
/// - `PhaseStarted` — 显示阶段进度条
/// - `PhaseData` — 显示阶段结果
/// - `Progress` — 显示阶段内进度（如辩论的专家数）
/// - `Error` — 显示错误提示
/// - `Complete` — 显示完成状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// 阶段开始
    PhaseStarted {
        phase: AlliancePhase,
        trace_id: Uuid,
        ts: DateTime<Utc>,
    },
    /// 阶段数据（完整结果）
    PhaseData(AllianceEvent),
    /// 阶段内进度（如辩论中第 N 位专家返回）
    Progress {
        phase: AlliancePhase,
        trace_id: Uuid,
        current: usize,
        total: usize,
        message: String,
        ts: DateTime<Utc>,
    },
    /// 错误（管线终止）
    Error {
        trace_id: Uuid,
        code: String,
        message: String,
        ts: DateTime<Utc>,
    },
    /// 完成（管线正常结束）
    Complete {
        trace_id: Uuid,
        total_ms: u64,
        gate_passed: bool,
        gate_grade: String,
        ts: DateTime<Utc>,
    },
}

impl StreamEvent {
    /// 创建阶段开始事件
    pub fn phase_started(phase: AlliancePhase, trace_id: Uuid) -> Self {
        StreamEvent::PhaseStarted {
            phase,
            trace_id,
            ts: Utc::now(),
        }
    }

    /// 创建阶段数据事件
    pub fn phase_data(event: AllianceEvent) -> Self {
        StreamEvent::PhaseData(event)
    }

    /// 创建进度事件
    pub fn progress(phase: AlliancePhase, trace_id: Uuid, current: usize, total: usize, message: String) -> Self {
        StreamEvent::Progress {
            phase,
            trace_id,
            current,
            total,
            message,
            ts: Utc::now(),
        }
    }

    /// 创建错误事件
    pub fn error(trace_id: Uuid, code: impl Into<String>, message: impl Into<String>) -> Self {
        StreamEvent::Error {
            trace_id,
            code: code.into(),
            message: message.into(),
            ts: Utc::now(),
        }
    }

    /// 创建完成事件
    pub fn complete(trace_id: Uuid, total_ms: u64, gate_passed: bool, gate_grade: impl Into<String>) -> Self {
        StreamEvent::Complete {
            trace_id,
            total_ms,
            gate_passed,
            gate_grade: gate_grade.into(),
            ts: Utc::now(),
        }
    }

    /// 获取 trace_id
    pub fn trace_id(&self) -> Uuid {
        match self {
            StreamEvent::PhaseStarted { trace_id, .. } => *trace_id,
            StreamEvent::PhaseData(e) => e.trace_id,
            StreamEvent::Progress { trace_id, .. } => *trace_id,
            StreamEvent::Error { trace_id, .. } => *trace_id,
            StreamEvent::Complete { trace_id, .. } => *trace_id,
        }
    }
}

// ================== 请求与选项类型 ==================

/// 6 阶段完整请求体（alliance/full 入口，FR-CORE-01）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AllianceRequest {
    /// 用户原始查询语句（UTF-8，中文优先）
    pub query: String,
    /// 会话 ID（前端 ChatView sessionId），用于学习关联与缓存
    #[serde(default)]
    pub session_id: Option<String>,
    /// 幂等 key：建议 = sha256(query + session_id + 上下文摘要)
    #[serde(default)]
    pub idempotency_key: Option<String>,
    /// 自由上下文：如 { project_id, domain, selected_expert_preset }
    #[serde(default)]
    pub context: BTreeMap<String, String>,
    /// 运行选项
    #[serde(default)]
    pub options: AllianceOptions,
}

/// 管线运行可调选项（phase 内部可读取；默认值保守：无 LLM、不重试 C）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AllianceOptions {
    /// 是否启用 LLM 真实辩论（默认 false：走纯本地维度加权投票，避免大模型成本/依赖）
    #[serde(default)]
    pub enable_llm_debate: bool,
    /// 质量门禁 C 级是否触发重试（默认 true：EAF-STD 4.6 C 级单次重试闭环）
    #[serde(default = "default_true")]
    pub retry_on_c: bool,
    /// 期望组队专家数（3~7；默认 4；超过注册上限则自动取最大）
    #[serde(default = "default_team_size_4")]
    pub team_size: usize,
    /// 需要激活扩散（默认 true；false=纯关键词，性能模式）
    #[serde(default = "default_true")]
    pub enable_spread: bool,
}

impl Default for AllianceOptions {
    fn default() -> Self {
        Self {
            enable_llm_debate: false,
            retry_on_c: true,
            team_size: 4,
            enable_spread: true,
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_team_size_4() -> usize {
    4
}

// ================== 管线框架集成：PhaseId trait ==================

/// 为 `AlliancePhase` 实现 `mox-pipeline-framework` 的 `PhaseId` trait。
///
/// 这使得联盟引擎的 6 阶段管线可以直接接入统一管线框架，
/// 享受管线编排、审计、超时、容错等框架能力。
impl mox_pipeline_framework::PhaseId for AlliancePhase {
    fn name(&self) -> &str {
        PHASE_NAMES[self.index()]
    }

    fn is_terminal(&self) -> bool {
        matches!(self, AlliancePhase::Done)
    }

    fn is_blocking(&self) -> bool {
        matches!(self, AlliancePhase::Gate)
    }

    fn order(&self) -> u32 {
        self.index() as u32
    }
}

// ================== 测试 ==================

#[cfg(test)]
mod tests {
    use super::*;

    /// AlliancePhase 顺序与 next 正确（防人为调换 phase 枚举顺序）
    #[test]
    fn phase_next_and_names_match_constants() {
        let mut p = AlliancePhase::Intent;
        for i in 0..6 {
            assert_eq!(p.name(), PHASE_NAMES[i]);
            p = p.next();
        }
        // 循环安全：next(Done) = Done
        assert_eq!(p, AlliancePhase::Done);
        assert_eq!(p.next(), AlliancePhase::Done);
        assert!(p.is_terminal());
    }

    #[test]
    fn phase_all_contains_seven() {
        assert_eq!(AlliancePhase::all().len(), 7);
        assert_eq!(AlliancePhase::all()[0], AlliancePhase::Intent);
        assert_eq!(AlliancePhase::all()[6], AlliancePhase::Done);
    }

    #[test]
    fn stream_event_trace_id_consistent() {
        let tid = Uuid::new_v4();
        let ev = StreamEvent::phase_started(AlliancePhase::Intent, tid);
        assert_eq!(ev.trace_id(), tid);

        let all_event = AllianceEvent {
            phase: AlliancePhase::Team,
            payload: serde_json::json!({}),
            trace_id: tid,
            latency_ms: 10,
            ts: Utc::now(),
            degraded: None,
            degrade_reason: None,
        };
        let ev2 = StreamEvent::phase_data(all_event);
        assert_eq!(ev2.trace_id(), tid);
    }

    #[test]
    fn audit_event_names_match_constants() {
        assert_eq!(AuditEvent::event_names().len(), 7);
        for (i, name) in AuditEvent::event_names().iter().enumerate() {
            assert_eq!(*name, AUDIT_EVENTS_7[i]);
        }
    }

    #[test]
    fn alliance_options_default() {
        let opts = AllianceOptions::default();
        assert!(!opts.enable_llm_debate);
        assert!(opts.retry_on_c);
        assert_eq!(opts.team_size, 4);
        assert!(opts.enable_spread);
    }
}

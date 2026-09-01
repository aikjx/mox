// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 开发专家联盟 · 全维分析 6 阶段管线编排器
//!
//! 阶段顺序（严格，SSE 事件按此顺序发射）：
//!   Intent（意图识别） → Team（专家组队） → Debate（并行咨询+辩论） →
//!   Synthesize（归一合成） → Gate（质量门禁） → Learn（指标学习） → Done
//!
//! 所有硬编码参数见 [`constants`]（HC-2/HC-5/HC-8/HC-9 锁死常量）。

pub mod algorithm;
pub mod constants;
pub mod debate;
pub mod gate;
pub mod intent;
pub mod kg_connector;
pub mod orchestration;
pub mod team;

use self::constants::PHASE_NAMES;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

// ================== 公共类型（跨 crate 使用） ==================

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
    pub fn index(&self) -> usize { *self as usize }
    /// 返回稳定的阶段名（用于 SSE event 字段与审计键）
    pub fn name(&self) -> &'static str { PHASE_NAMES[self.index()] }
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
}

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

/// 6 阶段完整请求体（/ai/engine/alliance/full 入口，FR-CORE-01）
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

fn default_true() -> bool { true }
fn default_team_size_4() -> usize { 4 }

// ================== 引擎（占位骨架，T2~T5 填充） ==================

/// 专家联盟全维分析引擎（对外入口结构体）
///
/// 生产环境由 mox_platform_orchestrator_svc 网关的 AiEngineState 持 Arc<AllianceEngine>。
/// 字段会在 Task 2~5 逐步追加（intent_classifier, expert_registry, harness_ctx, audit_sink 等）。
#[derive(Debug, Default, Clone)]
pub struct AllianceEngine {
    /// 引擎启动时间戳（用于 metrics / 健康）
    #[allow(dead_code)] // reserved: metrics/health in Task 2~5
    started_at: DateTime<Utc>,
}

impl AllianceEngine {
    pub fn new() -> Self {
        Self { started_at: Utc::now() }
    }

    /// 运行完整的 6 阶段全维分析，返回事件流（SSE 友好）。
    ///
    /// T5 已填充为真实管线：gate::run_full_pipeline 返回 7 SSE 事件 + 7 审计事件。
    pub async fn run_full_analysis(
        &self,
        req: AllianceRequest,
    ) -> Result<Vec<AllianceEvent>, AllianceError> {
        let (events, _audits) = gate::run_full_pipeline(req).await?;
        Ok(events)
    }
}

// ================== 错误（统一 thiserror 枚举，便于 ?） ==================

/// 专家联盟管线错误枚举（所有阶段错误的统一集合；后续 Task 会追加）
#[derive(Debug, thiserror::Error)]
pub enum AllianceError {
    #[error("query 不能为空")]
    EmptyQuery,
    #[error("意图分类失败：{0}")]
    IntentClassify(String),
    #[error("组队失败：{0}")]
    TeamBuild(String),
    #[error("专家咨询超时（{secs}s 隔离）")]
    ExpertTimeout { secs: u64, expert: String },
    #[error("质量门禁不通过（Gate={gate:?}，retried={retried}）")]
    GateBlocked { gate: String, retried: bool },
    #[error("RBAC 未授权：需要权限 {perm:?}")]
    Unauthorized { perm: String },
    #[error("内部错误：{0}")]
    Internal(#[from] anyhow::Error),
}

// ================== tests：占位骨架可工作 ==================

#[cfg(test)]
mod tests {
    use super::*;

    /// 占位 run_full_analysis：空 query 必须 Err（EmptyQuery）
    #[tokio::test]
    async fn empty_query_rejected() {
        let eng = AllianceEngine::new();
        let req = AllianceRequest {
            query: "  ".to_string(),
            session_id: None,
            idempotency_key: None,
            context: BTreeMap::new(),
            options: AllianceOptions::default(),
        };
        let res = eng.run_full_analysis(req).await;
        assert!(matches!(res, Err(AllianceError::EmptyQuery)));
    }

    /// 占位 run_full_analysis：正常 query 返回 7 事件，phase 严格 0..=6 顺序，trace_id 全相同
    #[tokio::test]
    async fn skeleton_emits_seven_phases_in_order_and_same_trace() {
        let eng = AllianceEngine::new();
        let req = AllianceRequest {
            query: "帮我做 Rust 企业级服务全维分析".to_string(),
            session_id: Some("sess-1".to_string()),
            idempotency_key: None,
            context: BTreeMap::new(),
            options: AllianceOptions::default(),
        };
        let events = eng.run_full_analysis(req).await.expect("ok");
        assert_eq!(events.len(), 7, "6 stages + done = 7 events");
        // phase 顺序
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.phase.index(), i, "phase index mismatch at {}", i);
        }
        // trace_id 全部相同
        let first_id = events[0].trace_id;
        assert!(events.iter().all(|e| e.trace_id == first_id));
        // Done 事件必须包含 QUALITY_FORMULA 原文（AC-09 早期基线）
        let done = events.last().unwrap();
        let done_str = serde_json::to_string(&done.payload).unwrap();
        assert!(done_str.contains("0.55×Quality + 0.20×Speed + 0.10×TokenEfficiency + 0.15×Stability"));
    }

    /// AlliancePhase 顺序与 next 正确（防人为调换 phase 枚举顺序）
    #[test]
    fn phase_next_and_names_match_constants() {
        let mut p = AlliancePhase::Intent;
        for &name in PHASE_NAMES.iter().take(6) {
            assert_eq!(p.name(), name);
            p = p.next();
        }
        // 循环安全：next(Done) = Done
        assert_eq!(p, AlliancePhase::Done);
        assert_eq!(p.next(), AlliancePhase::Done);
    }
}

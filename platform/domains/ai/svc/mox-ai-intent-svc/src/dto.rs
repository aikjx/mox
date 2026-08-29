// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! HTTP DTO：请求 / 响应结构，与前端契约对齐。

use serde::{Deserialize, Serialize};

// ─── 通用响应 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { code: 0, message: "ok".into(), data: Some(data) }
    }

    pub fn error(code: i32, message: &str) -> Self {
        Self { code, message: message.into(), data: None }
    }
}

// ─── 意图理解 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct UnderstandRequest {
    /// 用户输入文本
    pub query: String,
    /// 会话 ID（可选，用于上下文继承）
    pub session_id: Option<String>,
    /// 用户 ID
    #[serde(default)]
    pub user_id: String,
    /// 租户 ID（可选）
    pub tenant_id: Option<String>,
    /// 返回 TOP-K 相关 Agent（缺省 3）
    #[serde(default = "default_top_k")]
    pub top_k_agents: usize,
}

fn default_top_k() -> usize { 3 }

#[derive(Debug, Clone, Serialize)]
pub struct UnderstandResponse {
    /// 请求 ID
    pub request_id: String,
    /// 意图分类结果
    pub intent: IntentInfo,
    /// 提取到的实体
    pub entities: Vec<EntityInfo>,
    /// 任务计划
    pub task_plan: TaskPlanInfo,
    /// 推荐 Agent
    pub recommended_agents: Vec<AgentInfo>,
    /// 人机协同建议（驱动四向弹框）
    pub collaboration: CollaborationInfo,
    /// 整体置信度 0..1
    pub confidence: f32,
    /// 各阶段耗时（ms）
    pub timing: TimingInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntentInfo {
    pub primary: String,
    pub secondary: Vec<String>,
    pub confidence: f32,
    pub matched_keywords: Vec<String>,
    pub capability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub etype: String,
    pub etype_label: String,
    pub text: String,
    pub normalized: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskPlanInfo {
    pub plan_id: String,
    pub intent: String,
    pub steps: Vec<TaskStepInfo>,
    pub requires_overall_confirmation: bool,
    pub parallel_groups: Vec<Vec<String>>,
    pub total_est_duration_sec: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskStepInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub capability: String,
    pub risk: String,
    pub status: String,
    pub depends_on: Vec<String>,
    pub params: std::collections::HashMap<String, String>,
    pub est_duration_sec: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    pub id: String,
    pub score: f32,
    pub reasons: Vec<String>,
    pub breakdown: ScoreBreakdownInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreBreakdownInfo {
    pub match_score: i32,
    pub performance: f32,
    pub confidence: f32,
    pub total_score: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollaborationInfo {
    pub max_risk: String,
    pub needs_confirmation: bool,
    pub confirmation_steps: Vec<String>,
    /// 建议弹框方向：right / top / bottom / center / inline
    pub suggested_panel: String,
    /// 交互模式：auto_execute / one_click_confirm / double_confirm / multi_turn_clarify
    pub interaction_mode: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimingInfo {
    pub preprocess_ms: u64,
    pub classify_ms: u64,
    pub entity_extract_ms: u64,
    pub task_decompose_ms: u64,
    pub agent_match_ms: u64,
    pub total_ms: u64,
}

// ─── 实体提取（独立接口） ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractEntitiesRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractEntitiesResponse {
    pub entities: Vec<EntityInfo>,
}

// ─── 任务拆解（独立接口） ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct DecomposeRequest {
    pub intent: String,
    pub entities: Vec<EntityInfo>,
    pub user_query: String,
}

// ─── 会话管理 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub user_id: String,
    pub title: String,
    pub created_at: i64,
    pub last_active_at: i64,
    pub turn_count: usize,
    pub state: String,
    pub has_active_task: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TurnInfo {
    pub turn_id: usize,
    pub user_message: String,
    pub ai_message: Option<String>,
    pub timestamp: i64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSessionRequest {
    pub user_id: String,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatRequest {
    pub session_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub session_id: String,
    pub turn_id: usize,
    pub reply: String,
    pub understanding: Option<UnderstandResponse>,
}

// ─── 内置意图列表 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct IntentDefinitionInfo {
    pub id: String,
    pub domain: String,
    pub name: String,
    pub description: String,
    pub intent_key: String,
    pub keywords: Vec<String>,
    pub task_template: String,
    pub default_risk: String,
    pub icon: String,
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! L3 领域层对外统一数据类型（SSOT）
//!
//! 三个对外抽象 trait（`ExpertRegistry` / `ExpertConsultant` / `AllianceOrchestrator`）
//! 统一使用本模块的类型，避免下游 crate 直接依赖 mox-expert 的内部 concrete struct
//! （如 `GovernanceReport` / `Dimension` 等实现细节），完成 DIP 反转：下游只依赖 trait 抽象，
//! 不依赖具体实现。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ExpertRegistry 相关类型：专家元数据
// ---------------------------------------------------------------------------

/// 专家元数据：可注册 / 可查询 / 可路由的最小专家画像。
///
/// 对应内部 `crate::ir::Dimension` + `crate::expert::Expert` 的对外投影，
/// 但只暴露「id/名称/域/能力标签」等最小信息，隐藏引擎内部分析细节。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpertMeta {
    /// 全局唯一专家 id，如 `security` / `algorithm` / `architecture-code`
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 所属领域（如 `gov` / `finance`；`*` 表示通用）
    pub domain: String,
    /// 能力标签：用于 find/list 的关键词匹配（如 `["security","pii","authz"]`）
    pub capabilities: Vec<String>,
    /// 可选：描述文本
    #[serde(default)]
    pub description: String,
    /// 可选：维度（对齐 crate::ir::Dimension；缺省为空）
    #[serde(default)]
    pub dimension: Option<String>,
}

impl ExpertMeta {
    pub fn new(id: impl Into<String>, name: impl Into<String>, domain: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            domain: domain.into(),
            capabilities: Vec::new(),
            description: String::new(),
            dimension: None,
        }
    }
    pub fn with_capabilities(mut self, caps: impl IntoIterator<Item = String>) -> Self {
        self.capabilities = caps.into_iter().collect();
        self
    }
}

// ---------------------------------------------------------------------------
// ExpertConsultant 相关类型：咨询输入 / 输出
// ---------------------------------------------------------------------------

/// 咨询查询：把外部咨询请求抽象成最小可计算请求。
///
/// 真实请求可能是 FlowGraph 或代码片段；trait 不直接依赖 mox_ai_flow_svc::FlowGraph，
/// 下游可按需构造。`ctx` 携带主体/租户/配额等治理上下文的序列化投影。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultQuery {
    /// 请求唯一 id（调用方生成，ConsultReport 原样返回以匹配）
    pub id: String,
    /// 自然语言 / DSL 查询字符串
    pub query: String,
    /// 可选：附加上下文键值对（租户、主体、配额等治理参数的通用载体）
    #[serde(default)]
    pub ctx: HashMap<String, String>,
}

/// 咨询报告：专家咨询后的对外归一化输出。
///
/// 隐藏内部 `GovernanceReport` 的复杂字段（expert_scores / optimization / algo / gate / audit），
/// 只暴露：报告 id、执行步骤（可读摘要）、综合分 0..1、以及是否否决。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultReport {
    /// 报告 id（通常等于 ConsultQuery.id，便于关联）
    pub report_id: String,
    /// 执行步骤摘要（可读文本，例如"14 位专家并行诊断 → 权限/安全裁决 → 算法验证通过"）
    pub steps: Vec<String>,
    /// 综合健康分 0..1（1 = 完全健康；0 = 完全否决）
    pub score: f64,
    /// 是否被算法验证或治理闸门否决（veto=true 时下游应强制拦截）
    pub vetoed: bool,
    /// 可选：否决/警告的原因
    #[serde(default)]
    pub reason: Option<String>,
}

impl Default for ConsultReport {
    fn default() -> Self {
        Self {
            report_id: String::new(),
            steps: Vec::new(),
            score: 1.0,
            vetoed: false,
            reason: None,
        }
    }
}

// ---------------------------------------------------------------------------
// AllianceOrchestrator 相关类型：任务路由输入 / 输出
// ---------------------------------------------------------------------------

/// 任务规格：联盟编排器（AllianceOrchestrator）的路由输入。
///
/// 把"要做什么"（scenario / constraints）最小化表达为可路由请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    /// 任务唯一 id
    pub task_id: String,
    /// 业务场景：如 `gov-pii` / `etl` / `mcp-orchestration`
    pub scenario: String,
    /// 约束键值对：如 `{"regulated":"true","sla_ms":"30000"}`
    #[serde(default)]
    pub constraints: HashMap<String, String>,
}

/// 路由决策：联盟编排器选择最合适的专家后的返回。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// 选中的专家 id（对应 ExpertMeta.id）
    pub expert_id: String,
    /// 路由置信度 0..1（1 = 确信匹配）
    pub confidence: f64,
    /// 路由理由（可读文本，便于审计与调试）
    pub reason: String,
}

// ---------------------------------------------------------------------------
// 专家联盟 API 请求/响应类型（HTTP 层 DTO）
// ---------------------------------------------------------------------------

/// 专家注册请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterExpertRequest {
    pub id: String,
    pub name: String,
    #[serde(default = "default_domain_star")]
    pub domain: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub dimension: Option<String>,
}

fn default_domain_star() -> String { "*".into() }

/// 专家注册响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterExpertResponse {
    pub success: bool,
    pub expert_id: String,
    pub message: String,
}

/// 专家列表查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertListQuery {
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page() -> usize { 1 }
fn default_page_size() -> usize { 20 }

/// 专家列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertListResponse {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub experts: Vec<ExpertMeta>,
}

/// 专家详情响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertDetailResponse {
    pub expert: Option<ExpertMeta>,
    pub found: bool,
}

/// 专家咨询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultExpertRequest {
    pub query: String,
    pub expert_id: Option<String>,
    #[serde(default)]
    pub ctx: HashMap<String, String>,
    /// 可选：FlowGraph JSON 字符串（用于真实引擎分析）
    #[serde(default)]
    pub flow_json: Option<String>,
}

/// 专家咨询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultExpertResponse {
    pub report: ConsultReport,
    pub expert_id: String,
    pub expert_name: String,
}

/// 多专家协同咨询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiExpertConsultRequest {
    pub query: String,
    /// 指定专家 id 列表（为空则自动路由选择 top N）
    #[serde(default)]
    pub expert_ids: Vec<String>,
    /// 自动选择的专家数量（expert_ids 为空时生效）
    #[serde(default = "default_team_size_4")]
    pub team_size: usize,
    #[serde(default)]
    pub ctx: HashMap<String, String>,
    #[serde(default)]
    pub flow_json: Option<String>,
    /// 是否并行执行（默认 true）
    #[serde(default = "default_true")]
    pub parallel: bool,
}

fn default_team_size_4() -> usize { 4 }
fn default_true() -> bool { true }

/// 单专家咨询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleExpertResult {
    pub expert_id: String,
    pub expert_name: String,
    pub report: ConsultReport,
    pub latency_ms: u64,
}

/// 多专家协同响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiExpertConsultResponse {
    pub results: Vec<SingleExpertResult>,
    pub consensus: f64,
    pub overall_score: f64,
    pub overall_vetoed: bool,
    pub total_latency_ms: u64,
    pub synthesis: String,
}

/// 智能路由请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteExpertsRequest {
    pub query: String,
    pub scenario: Option<String>,
    #[serde(default)]
    pub constraints: HashMap<String, String>,
    /// 返回前 N 个最佳匹配
    #[serde(default = "default_top_3")]
    pub top_n: usize,
}

fn default_top_3() -> usize { 3 }

/// 路由匹配项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMatch {
    pub expert: ExpertMeta,
    pub confidence: f64,
    pub reason: String,
}

/// 智能路由响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteExpertsResponse {
    pub matches: Vec<RouteMatch>,
    pub query: String,
    pub method: String,
}

/// 专家辩论请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertDebateRequest {
    pub query: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default = "default_team_size_4")]
    pub team_size: usize,
    #[serde(default)]
    pub enable_llm_debate: bool,
    #[serde(default = "default_true")]
    pub enable_spread: bool,
    #[serde(default)]
    pub context: HashMap<String, String>,
}

/// 专家观点（API 层）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertOpinionView {
    pub expert_id: String,
    pub dimension: String,
    pub answer: String,
    pub score: f64,
    pub confidence: f64,
    pub latency_ms: u64,
    pub timed_out: bool,
    pub tokens_approx: usize,
}

/// 辩论结果响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertDebateResponse {
    pub trace_id: String,
    pub opinions: Vec<ExpertOpinionView>,
    pub consensus: f64,
    pub debate_rounds: u32,
    pub synthesis: String,
    pub synthesis_reasoning: String,
    pub gate_grade: String,
    pub gate_total: f64,
    pub total_latency_ms: u64,
}

/// 辩论 SSE 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateSseEvent {
    pub phase: String,
    pub payload: serde_json::Value,
    pub trace_id: String,
    pub latency_ms: u64,
}

/// 算法分析请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmAnalysisRequest {
    pub query: String,
    /// 分析维度：如 complexity, correctness, optimization, security, all
    #[serde(default = "default_algo_dim_all")]
    pub dimension: String,
    #[serde(default)]
    pub code_snippet: Option<String>,
    #[serde(default)]
    pub flow_json: Option<String>,
    #[serde(default)]
    pub context: HashMap<String, String>,
}

fn default_algo_dim_all() -> String { "all".into() }

/// 算法检查项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgoCheckItem {
    pub name: String,
    pub passed: bool,
    pub blocking: bool,
    pub detail: String,
    pub severity: String,
}

/// 算法分析响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmAnalysisResponse {
    pub analysis_id: String,
    pub dimension: String,
    pub checks: Vec<AlgoCheckItem>,
    pub all_passed: bool,
    pub vetoed: bool,
    pub summary: String,
    pub suggestions: Vec<String>,
    pub latency_ms: u64,
}

/// 任务编排请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRequest {
    pub task_id: String,
    pub scenario: String,
    pub query: String,
    #[serde(default)]
    pub constraints: HashMap<String, String>,
    #[serde(default)]
    pub context: HashMap<String, String>,
    /// 编排策略：sequential / parallel / pipeline
    #[serde(default = "default_strategy_pipeline")]
    pub strategy: String,
    #[serde(default = "default_team_size_4")]
    pub team_size: usize,
}

fn default_strategy_pipeline() -> String { "pipeline".into() }

/// 编排步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationStep {
    pub step_id: String,
    pub step_name: String,
    pub expert_id: String,
    pub status: String, // pending / running / completed / failed
    pub result: Option<serde_json::Value>,
    pub latency_ms: u64,
}

/// 任务编排响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResponse {
    pub task_id: String,
    pub scenario: String,
    pub steps: Vec<OrchestrationStep>,
    pub overall_status: String,
    pub overall_score: f64,
    pub total_latency_ms: u64,
    pub final_report: Option<String>,
}

/// 全维分析请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullAnalysisRequest {
    pub query: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub context: HashMap<String, String>,
    #[serde(default)]
    pub options: FullAnalysisOptions,
}

/// 全维分析选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullAnalysisOptions {
    #[serde(default)]
    pub enable_llm_debate: bool,
    #[serde(default = "default_true")]
    pub retry_on_c: bool,
    #[serde(default = "default_team_size_4")]
    pub team_size: usize,
    #[serde(default = "default_true")]
    pub enable_spread: bool,
}

impl Default for FullAnalysisOptions {
    fn default() -> Self {
        Self {
            enable_llm_debate: false,
            retry_on_c: true,
            team_size: 4,
            enable_spread: true,
        }
    }
}

/// 全维分析响应（非流式，完整结果）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullAnalysisResponse {
    pub trace_id: String,
    pub intent: serde_json::Value,
    pub team: serde_json::Value,
    pub debate: serde_json::Value,
    pub synthesis: String,
    pub gate: serde_json::Value,
    pub learn: serde_json::Value,
    pub total_ms: u64,
    pub gate_passed: bool,
    pub gate_grade: String,
    pub quality_formula: String,
}

/// 专家概览响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceOverview {
    pub total_experts: usize,
    pub total_domains: usize,
    pub total_capabilities: usize,
    /// 各维度专家数
    pub dimension_counts: HashMap<String, usize>,
    /// 各领域专家数
    pub domain_counts: HashMap<String, usize>,
    /// 平均专家能力标签数
    pub avg_capabilities_per_expert: f64,
    /// 引擎运行时长（秒）
    pub uptime_secs: u64,
    /// 累计咨询次数
    pub total_consultations: u64,
    /// 累计辩论次数
    pub total_debates: u64,
}

/// 专家指标响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertMetrics {
    pub expert_id: String,
    pub expert_name: String,
    pub consultation_count: u64,
    pub avg_score: f64,
    pub avg_latency_ms: u64,
    pub avg_confidence: f64,
    pub veto_rate: f64,
    pub gate_a_rate: f64,
}

/// 联盟指标响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceMetricsResponse {
    pub total_requests: u64,
    pub avg_consensus: f64,
    pub avg_gate_score: f64,
    pub gate_pass_rate: f64,
    pub avg_latency_ms: u64,
    pub expert_metrics: Vec<ExpertMetrics>,
    pub intent_distribution: HashMap<String, u64>,
}

// ---------------------------------------------------------------------------
// 通用 Result（trait 签名统一使用）
// ---------------------------------------------------------------------------

/// 对外 trait 的统一 Result：任何错误退化为 anyhow::Error，保持 trait 签名简洁。
pub type Result<T> = anyhow::Result<T>;

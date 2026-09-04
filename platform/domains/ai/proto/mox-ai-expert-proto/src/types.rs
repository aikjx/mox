// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 领域值类型（SSOT）
//!
//! 所有对外 trait 的入参/出参统一使用本模块类型。
//! 下游 crate 只依赖这些类型，不依赖内部 concrete struct。
//!
//! 本模块是璇玑专家领域的**单一真相源**：Dimension、专家观点、约束、风险、
//! 治理裁决等核心领域概念均在此定义，expert-svc / core / 下游服务都通过
//! 本 crate 引用，避免类型漂移和重复定义。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Dimension — 维度枚举（璇玑的 14 个镜头）
// ============================================================================

/// 十四个优化维度（业务七维 + 开发七维）
///
/// 业务七维（分析流程图）+ 开发七维（分析代码 IR）= 共十四专家，
/// 复用同一引擎 / 裁决 / 验证 / 治理 / 审计链。
///
/// 业务与开发璇玑**非冗余、互补**：前者管「做什么/是否合规」，
/// 后者管「怎么做/是否优质」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    // ---- 业务七维（分析流程图）----
    Business,
    Algorithm,
    Permission,
    Resource,
    Security,
    Data,
    Observability,
    // ---- 开发七维（分析代码）----
    Architecture,
    SecurityCode,
    CodeQuality,
    Performance,
    Testing,
    Documentation,
    Maintainability,
}

impl Dimension {
    /// mox 模块化系统架构裁决时的优先级权重：值越大越优先被采纳（权限/安全不可被性能绕过）。
    ///
    /// 单一数据源：`crate::constants::DIM_PRIORITY`（集中常量，避免散落魔法数字）。
    pub fn priority(&self) -> i32 {
        crate::constants::dim_priority(*self)
    }

    /// 是否为「开发璇玑」维度（分析代码，而非流程图）
    pub fn is_code_dimension(&self) -> bool {
        matches!(
            self,
            Dimension::Architecture
                | Dimension::SecurityCode
                | Dimension::CodeQuality
                | Dimension::Performance
                | Dimension::Testing
                | Dimension::Documentation
                | Dimension::Maintainability
        )
    }

    /// 是否为「业务璇玑」维度（分析流程图）
    pub fn is_business_dimension(&self) -> bool {
        matches!(
            self,
            Dimension::Business
                | Dimension::Algorithm
                | Dimension::Permission
                | Dimension::Resource
                | Dimension::Security
                | Dimension::Data
                | Dimension::Observability
        )
    }

    /// 维度显示名称（中文，用于 UI / 日志 / 报告）
    pub fn display_name(&self) -> &'static str {
        match self {
            Dimension::Business => "业务专家",
            Dimension::Algorithm => "算法专家",
            Dimension::Permission => "权限专家",
            Dimension::Resource => "资源专家",
            Dimension::Security => "安全专家",
            Dimension::Data => "数据专家",
            Dimension::Observability => "可观测专家",
            Dimension::Architecture => "架构专家",
            Dimension::SecurityCode => "代码安全专家",
            Dimension::CodeQuality => "代码质量专家",
            Dimension::Performance => "性能专家",
            Dimension::Testing => "测试专家",
            Dimension::Documentation => "文档专家",
            Dimension::Maintainability => "可维护性专家",
        }
    }
}

// ============================================================================
// 基础类型别名
// ============================================================================

/// 专家标识
pub type ExpertId = String;
/// 策略标识
pub type PolicyId = String;

// ============================================================================
// DimensionTag — 维度着色标签
// ============================================================================

/// 节点/边上的维度着色标签
///
/// 设计铁律：四种流程图（业务/算法/权限/资源）在内存里是**同一个 FlowGraph**，
/// 维度只是节点/边上的标签。物理节点唯一，因此「改一处，mox 模块化系统架构同步」天然成立。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionTag {
    pub dimension: Dimension,
    pub owner_expert: ExpertId,
    pub policy_refs: Vec<PolicyId>,
    /// 该维度在此节点的相对重要性 0..1
    pub weight: f64,
}

// ============================================================================
// Severity — 风险严重等级
// ============================================================================

/// 风险严重等级
///
/// 用于统一描述风险、约束违反、代码问题等的严重程度。
/// 与 flow-svc 的 Severity 语义对齐，作为领域级 SSOT 定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    /// 阻断级：必须在生成代码前修复
    Blocking,
}

// ============================================================================
// ModelTier — 模型算力档位
// ============================================================================

/// 模型算力档位
///
/// 按任务语义把 LLM 调用路由到不同规格模型，降低算力成本。
/// 与 flow-svc 的 ModelTier 语义对齐，作为领域级 SSOT 定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    /// 简单问答 / 分类 / 抽取
    Light,
    /// 标准业务推理
    Standard,
    /// 代码生成 / 流程建模等重型任务
    Heavy,
}

// ============================================================================
// NodeEdge — 节点间有向引用
// ============================================================================

/// 节点间有向引用（本地轻量结构）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEdge {
    pub from: String,
    pub to: String,
}

// ============================================================================
// Constraint — 约束（归一化裁决的最小合并单元）
// ============================================================================

/// 约束：归一化裁决的最小合并单元
///
/// 每位专家输出一组约束，裁决器按维度优先级合并冲突约束，
/// 最终形成统一的约束集供优化器执行。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint {
    /// 强制顺序边
    MustOrder(NodeEdge),
    /// 前置拦截节点（标签如 "desensitize" / "authz"）
    MustGuard(String, Vec<String>),
    /// 互斥串行（资源/事务冲突修复）
    MustSerialize(NodeEdge),
    /// 隔离执行（沙箱）
    MustIsolate(String),
    /// 强制审计点
    MustAudit(String),
    /// 资源池上限（来自租户配额）
    ResourceCap(String, u32),
    /// 合规策略绑定
    Compliance(PolicyId),
    /// 建议绑定的算力档位
    RouteModel(String, ModelTier),
}

impl Constraint {
    /// 返回该约束涉及的节点 id 集合（用于冲突检测与审计溯源）
    pub fn nodes(&self) -> Vec<String> {
        match self {
            Constraint::MustOrder(e) => vec![e.from.clone(), e.to.clone()],
            Constraint::MustGuard(t, _) => vec![t.clone()],
            Constraint::MustSerialize(e) => vec![e.from.clone(), e.to.clone()],
            Constraint::MustIsolate(t) => vec![t.clone()],
            Constraint::MustAudit(t) => vec![t.clone()],
            Constraint::ResourceCap(_, _) => vec![],
            Constraint::Compliance(_) => vec![],
            Constraint::RouteModel(n, _) => vec![n.clone()],
        }
    }
}

// ============================================================================
// Risk — 风险（发现的问题）
// ============================================================================

/// 风险：专家发现的问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub severity: Severity,
    pub nodes: Vec<String>,
    pub dimension: Dimension,
    pub message: String,
    pub remediation: Option<String>,
    /// 是否为「否决级」风险：专家判定此风险不可自动修复、必须人工审批/禁止出码。
    ///
    /// 由优化管线汇总后并入算法验证否决（algo.vetoed），治理闸门不可覆盖。
    /// 默认 false = 仅作建议/可经约束自动修复。
    #[serde(default)]
    pub veto: bool,
}

// ============================================================================
// Suggestion — 建议（非强制优化提议）
// ============================================================================

/// 建议：非强制优化提议
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Suggestion {
    Parallelize,
    Cache,
    Split,
    Merge,
    Offload(ModelTier),
    Retry,
    Debounce,
}

// ============================================================================
// ExpertOpinion — 专家观点
// ============================================================================

/// 专家观点：单个专家的完整分析输出
///
/// 每位专家无状态、只读分析，输出 `ExpertOpinion`，由裁决器归一合并。
/// 专家之间互不调用，保证可并行派发。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertOpinion {
    pub expert: ExpertId,
    pub dimension: Dimension,
    pub constraints: Vec<Constraint>,
    pub risks: Vec<Risk>,
    /// 健康分 0..1
    pub score: f64,
    pub metrics: HashMap<String, f64>,
    pub suggestions: Vec<Suggestion>,
    /// 若本专家因权限不足跳过分析，置 true
    #[serde(default)]
    pub skipped: bool,
    #[serde(default)]
    pub skip_reason: Option<String>,
}

impl ExpertOpinion {
    /// 创建空观点（健康分 1.0，无风险/约束/建议）
    pub fn empty(expert: impl Into<String>, dimension: Dimension) -> Self {
        Self {
            expert: expert.into(),
            dimension,
            constraints: Vec::new(),
            risks: Vec::new(),
            score: 1.0,
            metrics: HashMap::new(),
            suggestions: Vec::new(),
            skipped: false,
            skip_reason: None,
        }
    }

    /// 创建跳过观点（因权限不足等原因未执行分析）
    pub fn skipped(
        expert: impl Into<String>,
        dimension: Dimension,
        reason: impl Into<String>,
    ) -> Self {
        let mut o = Self::empty(expert, dimension);
        o.skipped = true;
        o.skip_reason = Some(reason.into());
        o.score = 0.0;
        o
    }

    /// 添加一条风险，并根据严重程度扣减健康分
    pub fn push_risk(
        &mut self,
        severity: Severity,
        nodes: Vec<String>,
        msg: impl Into<String>,
        rem: Option<String>,
    ) {
        self.risks.push(Risk {
            severity,
            nodes,
            dimension: self.dimension,
            message: msg.into(),
            remediation: rem,
            veto: false,
        });
        if severity == Severity::Blocking {
            self.score = (self.score - 0.5).max(0.0);
        } else if severity == Severity::Warning {
            self.score = (self.score - 0.2).max(0.0);
        }
    }

    /// 添加否决级风险：专家判定不可自动修复、必须禁止出码（并入 algo.vetoed）。
    ///
    /// 与 `push_risk` 区别仅在 `veto = true`（且强制 Blocking 级）。
    pub fn push_veto(&mut self, nodes: Vec<String>, msg: impl Into<String>, rem: Option<String>) {
        self.risks.push(Risk {
            severity: Severity::Blocking,
            nodes,
            dimension: self.dimension,
            message: msg.into(),
            remediation: rem,
            veto: true,
        });
        self.score = (self.score - 0.5).max(0.0);
    }
}

// ============================================================================
// ExpertMeta — 专家元数据
// ============================================================================

/// 专家元数据：可注册 / 可查询 / 可路由的最小专家画像。
///
/// 对应内部引擎 Expert 的对外投影，但只暴露「id/名称/域/能力标签」等
/// 最小信息，隐藏引擎内部分析细节。
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
    /// 可选：维度（对齐 Dimension；缺省为空）
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

    pub fn with_dimension(mut self, dim: Dimension) -> Self {
        self.dimension = Some(format!("{:?}", dim));
        self
    }
}

// ============================================================================
// ConsultQuery / ConsultReport — 咨询输入输出
// ============================================================================

/// 咨询查询：把外部咨询请求抽象成最小可计算请求。
///
/// 真实请求可能是 FlowGraph 或代码片段；trait 不直接依赖具体引擎类型，
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
/// 隐藏内部 `GovernanceReport` 的复杂字段（expert_scores / optimization /
/// algo / gate / audit），只暴露：报告 id、执行步骤（可读摘要）、
/// 综合分 0..1、以及是否否决。
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

// ============================================================================
// TaskSpec / RoutingDecision — 联盟编排输入输出
// ============================================================================

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

// ============================================================================
// GovernLevel / GovernVerdict — 治理裁决值类型
// ============================================================================

/// 治理等级
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernLevel {
    Pass,
    Warn,
    Block,
}

/// 治理裁决：治理专家的输出值对象
///
/// 这是一个纯值类型，可以脱离 concrete 实现使用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernVerdict {
    pub level: GovernLevel,
    pub score: f64,
    pub reasons: Vec<String>,
    pub gate_id: String,
}

impl Default for GovernVerdict {
    fn default() -> Self {
        Self {
            level: GovernLevel::Pass,
            score: 1.0,
            reasons: Vec::new(),
            gate_id: "default".into(),
        }
    }
}

impl fmt::Display for GovernVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:?}] score={:.3} gate={} reasons={}",
            self.level, self.score, self.gate_id, self.reasons.len()
        )
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Dimension 测试 ----

    #[test]
    fn dimension_priority_matches_ssot() {
        assert_eq!(Dimension::Permission.priority(), 100);
        assert_eq!(Dimension::Security.priority(), 100);
        assert_eq!(Dimension::Performance.priority(), 60);
    }

    #[test]
    fn dimension_classification() {
        assert!(Dimension::Security.is_business_dimension());
        assert!(!Dimension::Security.is_code_dimension());
        assert!(Dimension::SecurityCode.is_code_dimension());
        assert!(!Dimension::SecurityCode.is_business_dimension());
    }

    #[test]
    fn dimension_display_name_is_non_empty() {
        assert!(!Dimension::Business.display_name().is_empty());
        assert!(!Dimension::Architecture.display_name().is_empty());
    }

    // ---- ExpertMeta 测试 ----

    #[test]
    fn expert_meta_builder() {
        let m = ExpertMeta::new("sec", "安全专家", "gov")
            .with_capabilities(["pii".into(), "auth".into()])
            .with_dimension(Dimension::Security);
        assert_eq!(m.id, "sec");
        assert_eq!(m.capabilities.len(), 2);
        assert!(m.dimension.is_some());
    }

    // ---- ConsultReport 测试 ----

    #[test]
    fn consult_report_default_is_healthy() {
        let r = ConsultReport::default();
        assert!((r.score - 1.0).abs() < 1e-9);
        assert!(!r.vetoed);
    }

    // ---- ExpertOpinion 测试 ----

    #[test]
    fn expert_opinion_empty_is_healthy() {
        let o = ExpertOpinion::empty("test-expert", Dimension::Business);
        assert!((o.score - 1.0).abs() < 1e-9);
        assert!(o.risks.is_empty());
        assert!(o.constraints.is_empty());
        assert!(!o.skipped);
    }

    #[test]
    fn expert_opinion_skipped_has_zero_score() {
        let o = ExpertOpinion::skipped("test-expert", Dimension::Permission, "no access");
        assert!(o.skipped);
        assert_eq!(o.skip_reason.as_deref(), Some("no access"));
        assert!((o.score - 0.0).abs() < 1e-9);
    }

    #[test]
    fn blocking_risk_lowers_score() {
        let mut o = ExpertOpinion::empty("test", Dimension::Security);
        o.push_risk(Severity::Blocking, vec!["n1".into()], "test risk", None);
        assert!(o.score < 1.0);
        assert!(o.risks.iter().any(|r| r.severity == Severity::Blocking));
    }

    #[test]
    fn warning_risk_lowers_score_less() {
        let mut o = ExpertOpinion::empty("test", Dimension::Security);
        o.push_risk(Severity::Warning, vec!["n1".into()], "test warn", None);
        assert!(o.score > 0.5); // Warning 只扣 0.2
        assert!(o.score < 1.0);
    }

    #[test]
    fn veto_risk_sets_veto_flag() {
        let mut o = ExpertOpinion::empty("test", Dimension::Security);
        o.push_veto(vec!["n1".into()], "critical", None);
        assert!(o.risks.iter().any(|r| r.veto));
        assert_eq!(o.risks[0].severity, Severity::Blocking);
    }

    // ---- Constraint 测试 ----

    #[test]
    fn constraint_nodes_must_order_returns_two() {
        let c = Constraint::MustOrder(NodeEdge {
            from: "a".into(),
            to: "b".into(),
        });
        assert_eq!(c.nodes(), vec!["a", "b"]);
    }

    #[test]
    fn constraint_compliance_returns_no_nodes() {
        let c = Constraint::Compliance("policy-1".into());
        assert!(c.nodes().is_empty());
    }

    // ---- GovernVerdict 测试 ----

    #[test]
    fn verdict_default_is_pass() {
        let v = GovernVerdict::default();
        assert_eq!(v.level, GovernLevel::Pass);
        assert!((v.score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn verdict_display_format() {
        let v = GovernVerdict::default();
        let s = format!("{}", v);
        assert!(s.contains("Pass"));
        assert!(s.contains("default"));
    }

    // ---- Severity / ModelTier 测试 ----

    #[test]
    fn severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Blocking);
    }

    #[test]
    fn suggestion_offload_carry_tier() {
        let s = Suggestion::Offload(ModelTier::Heavy);
        assert!(matches!(s, Suggestion::Offload(ModelTier::Heavy)));
    }
}

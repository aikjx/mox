// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 领域值类型（SSOT）
//!
//! 所有对外 trait 的入参/出参统一使用本模块类型。
//! 下游 crate 只依赖这些类型，不依赖内部 concrete struct。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Dimension — 维度枚举（璇玑的 14 个镜头）
// ============================================================================

/// 十四个优化维度（业务七维 + 开发七维）
///
/// 业务七维（分析 FlowGraph）+ 开发七维（分析 CodeIR）= 共十四专家，
/// 复用同一引擎 / 裁决 / 验证 / 治理 / 审计链。
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
    /// 维度优先级权重（调用 SSOT 常量）
    pub fn priority(&self) -> i32 {
        crate::dim_priority(*self)
    }

    /// 是否为「开发璇玑」维度（分析代码）
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

    /// 维度显示名称
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
// DimensionTag / DimensionedFlow — 维度标记
// ============================================================================

/// 维度标签：给节点/边标注所属维度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DimensionTag {
    pub dimension: Dimension,
    pub confidence: f64,
    pub tags: Vec<String>,
}

/// 维度着色后的流程图（投影类型，内部引擎用完整结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionedFlow {
    pub flow_id: String,
    pub dimensions: Vec<Dimension>,
    pub tag_count: usize,
}

// ============================================================================
// ExpertMeta — 专家元数据
// ============================================================================

/// 专家元数据：可注册 / 可查询 / 可路由的最小专家画像
///
/// 对应内部引擎 Expert 的对外投影，只暴露最小信息，隐藏引擎内部分析细节。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpertMeta {
    /// 全局唯一专家 id，如 `security` / `algorithm` / `architecture-code`
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 所属领域（如 `gov` / `finance`；`*` 表示通用）
    pub domain: String,
    /// 能力标签：用于 find/list 的关键词匹配
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

/// 咨询查询：把外部咨询请求抽象成最小可计算请求
///
/// 真实请求可能是 FlowGraph 或代码片段；trait 不直接依赖具体引擎类型，
/// 下游可按需构造。`ctx` 携带治理上下文的序列化投影。
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

/// 咨询报告：专家咨询后的对外归一化输出
///
/// 隐藏内部复杂字段（expert_scores / optimization / algo / gate / audit），
/// 只暴露：报告 id、执行步骤、综合分 0..1、以及是否否决。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultReport {
    /// 报告 id（通常等于 ConsultQuery.id，便于关联）
    pub report_id: String,
    /// 执行步骤摘要（可读文本）
    pub steps: Vec<String>,
    /// 综合健康分 0..1（1 = 完全健康；0 = 完全否决）
    pub score: f64,
    /// 是否被算法验证或治理闸门否决
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

/// 任务规格：联盟编排器（AllianceOrchestrator）的路由输入
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

/// 路由决策：联盟编排器选择最合适的专家后的返回
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
// 专家意见 / 约束 / 风险 / 建议（领域值对象）
// ============================================================================

/// 风险等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// 约束类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintType {
    /// 硬性约束（违反即 Blocking）
    Hard,
    /// 软性约束（违反即 Warning）
    Soft,
    /// 优化建议（不影响通过）
    Suggestion,
}

/// 专家约束：单个维度给出的约束条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub dimension: Dimension,
    pub constraint_type: ConstraintType,
    pub description: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

/// 专家建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub dimension: Dimension,
    pub title: String,
    pub description: String,
    pub priority: u8,
    pub expected_improvement: f64,
}

/// 专家意见：单个专家的完整输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertOpinion {
    pub expert_id: String,
    pub dimension: Dimension,
    pub score: f64,
    pub confidence: f64,
    pub constraints: Vec<Constraint>,
    pub suggestions: Vec<Suggestion>,
    pub risk_level: RiskLevel,
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn expert_meta_builder() {
        let m = ExpertMeta::new("sec", "安全专家", "gov")
            .with_capabilities(["pii".into(), "auth".into()])
            .with_dimension(Dimension::Security);
        assert_eq!(m.id, "sec");
        assert_eq!(m.capabilities.len(), 2);
        assert!(m.dimension.is_some());
    }

    #[test]
    fn consult_report_default_is_healthy() {
        let r = ConsultReport::default();
        assert!((r.score - 1.0).abs() < 1e-9);
        assert!(!r.vetoed);
    }
}

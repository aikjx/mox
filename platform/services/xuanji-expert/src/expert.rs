//! 专家 trait 与观点类型
//!
//! 每位专家无状态、只读分析，输出 `ExpertOpinion`，由裁决器归一合并。
//! 专家之间互不调用，保证可并行派发。

use crate::context::ExpertContext;
use crate::ir::{Dimension, ExpertId, PolicyId};
use flow_ai::model::Severity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 节点间有向引用（本地轻量结构，flow-ai 使用 FlowEdge 表达）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEdge {
    pub from: String,
    pub to: String,
}

/// 约束：归一化裁决的最小合并单元
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
    RouteModel(String, flow_ai::schedule::ModelTier),
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

/// 风险：发现的问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub severity: Severity,
    pub nodes: Vec<String>,
    pub dimension: Dimension,
    pub message: String,
    pub remediation: Option<String>,
    /// 是否为「否决级」风险：专家判定此风险不可自动修复、必须人工审批/禁止出码。
    /// 由 `xuanji_optimize` 汇总后并入算法验证否决（algo.vetoed），治理闸门不可覆盖。
    /// 默认 false = 仅作建议/可经约束自动修复。
    pub veto: bool,
}

/// 建议：非强制优化提议
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Suggestion {
    Parallelize,
    Cache,
    Split,
    Merge,
    Offload(flow_ai::schedule::ModelTier),
    Retry,
    Debounce,
}

/// 专家观点
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
    pub skipped: bool,
    pub skip_reason: Option<String>,
}

impl ExpertOpinion {
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

    /// 否决级风险：专家判定不可自动修复、必须禁止出码（并入 algo.vetoed）。
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

/// 专家 trait
pub trait Expert: Send + Sync {
    fn id(&self) -> ExpertId;
    fn dimension(&self) -> Dimension;
    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion;
}

/// 并行派发所有专家（rayon 真并行，利用多核；保持原序保证结果确定性）
pub fn dispatch(ctx: &ExpertContext, experts: &[Box<dyn Expert>]) -> Vec<ExpertOpinion> {
    use rayon::prelude::*;
    experts.par_iter().map(|e| e.analyze(ctx)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{GovernContext, Tenant};
    use flow_ai::model::FlowGraph;
    use flow_ai::schedule::ModelTier;

    struct Dummy;
    impl Expert for Dummy {
        fn id(&self) -> ExpertId {
            "dummy".into()
        }
        fn dimension(&self) -> Dimension {
            Dimension::Business
        }
        fn analyze(&self, _ctx: &ExpertContext) -> ExpertOpinion {
            let mut o = ExpertOpinion::empty("dummy", Dimension::Business);
            o.push_risk(Severity::Blocking, vec!["a".into()], "test", None);
            o
        }
    }

    #[test]
    fn blocking_risk_lowers_score() {
        let g = GovernContext::new(Tenant::new("t", "ns"), crate::context::Principal::new("u"));
        let fg = FlowGraph::new("x", "t");
        let ctx = ExpertContext::new(&fg, &g);
        let o = Dummy.analyze(&ctx);
        assert!(o.score < 1.0);
        assert!(o.risks.iter().any(|r| r.severity == Severity::Blocking));
    }

    #[test]
    fn suggestion_offload_carry_tier() {
        let s = Suggestion::Offload(ModelTier::Heavy);
        assert!(matches!(s, Suggestion::Offload(ModelTier::Heavy)));
    }
}

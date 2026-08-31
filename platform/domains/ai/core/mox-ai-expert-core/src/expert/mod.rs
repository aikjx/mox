// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 专家 trait 与并行派发
//!
//! 每位专家无状态、只读分析，输出 `ExpertOpinion`（来自 `mox-ai-expert-proto`，SSOT），
//! 由裁决器归一合并。专家之间互不调用，保证可并行派发。
//!
//! P2 架构解耦 · 阶段 4：
//! - 领域值类型（ExpertOpinion / Constraint / Risk / Suggestion / Dimension 等）
//!   统一使用 `mox-ai-expert-proto` 中的定义（SSOT 单一真相源）
//! - `Expert` trait 与 `dispatch` 函数属引擎核心逻辑，保留在 core 内部

use crate::context::ExpertContext;
use mox_ai_expert_proto::{Dimension, ExpertId, ExpertOpinion};

/// 专家 trait：引擎内部抽象
///
/// 每位专家实现此 trait，通过 `dispatch` 并行派发。
/// 注意：这是**内部**引擎 trait，与 proto 中的 `ExpertConsultant` / `ExpertRegistry`
/// 是不同层次的抽象——后者是对外的服务级 trait，前者是内部的单专家分析 trait。
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

// ─── 类型转换工具（proto Severity/ModelTier ↔ flow-svc Severity/ModelTier） ──
//
// 领域值类型的 SSOT 在 proto，但裁决器/管线需要与 flow-svc 的图模型交互，
// 因此需要一组零开销/低开销的转换函数。两者枚举变体一一对应，转换是无损的。

/// proto::Severity → flow_svc::model::Severity
pub fn to_flow_severity(s: mox_ai_expert_proto::Severity) -> mox_ai_flow_svc::model::Severity {
    match s {
        mox_ai_expert_proto::Severity::Info => mox_ai_flow_svc::model::Severity::Info,
        mox_ai_expert_proto::Severity::Warning => mox_ai_flow_svc::model::Severity::Warning,
        mox_ai_expert_proto::Severity::Blocking => mox_ai_flow_svc::model::Severity::Blocking,
    }
}

/// proto::ModelTier → flow_svc::schedule::ModelTier
pub fn to_flow_model_tier(t: mox_ai_expert_proto::ModelTier) -> mox_ai_flow_svc::schedule::ModelTier {
    match t {
        mox_ai_expert_proto::ModelTier::Light => mox_ai_flow_svc::schedule::ModelTier::Light,
        mox_ai_expert_proto::ModelTier::Standard => mox_ai_flow_svc::schedule::ModelTier::Standard,
        mox_ai_expert_proto::ModelTier::Heavy => mox_ai_flow_svc::schedule::ModelTier::Heavy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{GovernContext, Tenant, Principal};
    use mox_ai_flow_svc::model::FlowGraph;
    use mox_ai_expert_proto::{Severity, ModelTier, Suggestion};

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
        let g = GovernContext::new(Tenant::new("t", "ns"), Principal::new("u"));
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

    #[test]
    fn dispatch_returns_opinions() {
        let g = GovernContext::new(Tenant::new("t", "ns"), Principal::new("u"));
        let fg = FlowGraph::new("x", "t");
        let ctx = ExpertContext::new(&fg, &g);
        let experts: Vec<Box<dyn Expert>> = vec![Box::new(Dummy)];
        let opinions = dispatch(&ctx, &experts);
        assert_eq!(opinions.len(), 1);
        assert_eq!(opinions[0].expert, "dummy");
    }

    #[test]
    fn severity_conversion_roundtrip() {
        use mox_ai_expert_proto::Severity as PS;
        use mox_ai_flow_svc::model::Severity as FS;
        assert!(matches!(to_flow_severity(PS::Info), FS::Info));
        assert!(matches!(to_flow_severity(PS::Warning), FS::Warning));
        assert!(matches!(to_flow_severity(PS::Blocking), FS::Blocking));
    }

    #[test]
    fn model_tier_conversion_roundtrip() {
        use mox_ai_expert_proto::ModelTier as PM;
        use mox_ai_flow_svc::schedule::ModelTier as FM;
        assert!(matches!(to_flow_model_tier(PM::Light), FM::Light));
        assert!(matches!(to_flow_model_tier(PM::Standard), FM::Standard));
        assert!(matches!(to_flow_model_tier(PM::Heavy), FM::Heavy));
    }
}

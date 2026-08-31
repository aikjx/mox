// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox

//! 治理裁决器实现：实现 proto::GovernExpert trait
//!
//! 设计要点：
//! - 通过 `&dyn Any + Send + Sync` 接收 FlowGraph，内部 downcast
//! - 调用 core 引擎的 mox_optimize 进行完整治理
//! - 实现 DIP：下游通过 trait 访问，不依赖具体实现

use crate::context::{GovernContext, Principal, Tenant};
use crate::pipeline::mox_optimize;
use async_trait::async_trait;
use mox_ai_expert_proto::{GovernExpert, GovernLevel, GovernVerdict};
use mox_ai_flow_svc::model::FlowGraph;

/// 治理专家（实现 proto::GovernExpert trait）
///
/// 通过 `&dyn Any` 接收 FlowGraph，内部 downcast 为具体类型。
/// 注意：trait 要求 `Any + Send + Sync`，FlowGraph 已满足。
pub struct GovernExpertImpl;

impl Default for GovernExpertImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl GovernExpertImpl {
    pub fn new() -> Self {
        Self
    }

    /// 同步治理（核心引擎是同步的，直接调用）
    fn govern_sync(
        &self,
        graph: &(dyn std::any::Any + Send + Sync),
        ctx: &dyn mox_ai_expert_proto::domain::GovernContext,
    ) -> GovernVerdict {
        let flow = match graph.downcast_ref::<FlowGraph>() {
            Some(f) => f,
            None => {
                return GovernVerdict {
                    level: GovernLevel::Warn,
                    score: 0.5,
                    reasons: vec!["[GovernExpertImpl] 无法识别的图类型".into()],
                    gate_id: "type-error".into(),
                }
            }
        };

        let tenant = Tenant::new(ctx.tenant(), ctx.namespace()).regulated(ctx.is_regulated());
        let principal = Principal::new(ctx.principal()).with_roles(ctx.roles().to_vec());
        let gctx = GovernContext::new(tenant, principal);
        let rep = mox_optimize(flow, &gctx);

        let level = if rep.gate.approved {
            GovernLevel::Pass
        } else if rep.algo.vetoed {
            GovernLevel::Block
        } else {
            GovernLevel::Warn
        };

        let total: f64 = rep.expert_scores.iter().map(|(_, s)| *s).sum();
        let score = if rep.expert_scores.is_empty() {
            1.0
        } else {
            total / rep.expert_scores.len() as f64
        };

        GovernVerdict {
            level,
            score,
            reasons: if rep.gate.approved {
                vec![]
            } else {
                vec![rep.gate.reason.clone()]
            },
            gate_id: "govern-core".into(),
        }
    }
}

#[async_trait]
impl GovernExpert for GovernExpertImpl {
    async fn govern(
        &self,
        graph: &(dyn std::any::Any + Send + Sync),
        ctx: &dyn mox_ai_expert_proto::domain::GovernContext,
    ) -> GovernVerdict {
        // core 引擎本身是同步的，直接委托给同步实现
        // （同步实现不捕获 &dyn Any 到 future 中，避免 Send 问题）
        self.govern_sync(graph, ctx)
    }

    fn govern_blocking(
        &self,
        graph: &(dyn std::any::Any + Send + Sync),
        ctx: &dyn mox_ai_expert_proto::domain::GovernContext,
    ) -> GovernVerdict {
        self.govern_sync(graph, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_expert_proto::domain::{MinimalGovernContext, MockGovernExpert};

    #[test]
    fn govern_expert_impl_returns_verdict() {
        let expert = GovernExpertImpl::new();
        let flow = FlowGraph::new("f", "test");
        let ctx = MinimalGovernContext::default();
        let verdict = expert.govern_blocking(&flow, &ctx);
        assert!(verdict.score > 0.0);
        assert_eq!(verdict.gate_id, "govern-core");
    }

    #[test]
    fn unknown_graph_type_returns_warn() {
        let expert = GovernExpertImpl::new();
        let ctx = MinimalGovernContext::default();
        // 用 () 作为图（不是 FlowGraph 类型）
        let verdict = expert.govern_blocking(&(), &ctx);
        assert_eq!(verdict.level, GovernLevel::Warn);
        assert_eq!(verdict.gate_id, "type-error");
    }

    #[test]
    fn mock_vs_concrete_both_implement_trait() {
        // DIP 证据：MockGovernExpert 和 GovernExpertImpl 都实现 GovernExpert trait
        let mock = MockGovernExpert::default();
        let concrete = GovernExpertImpl::new();
        let ctx = MinimalGovernContext::default();

        let v_mock = mock.govern_blocking(&(), &ctx);
        let v_concrete = concrete.govern_blocking(&FlowGraph::new("x", "t"), &ctx);

        // 两者都返回合法的 GovernVerdict
        assert!(v_mock.score > 0.0);
        assert!(v_concrete.score > 0.0);
    }

    #[tokio::test]
    async fn async_govern_works() {
        let expert = GovernExpertImpl::new();
        let flow = FlowGraph::new("f", "test");
        let ctx = MinimalGovernContext::default();
        let verdict = expert.govern(&flow, &ctx).await;
        assert_eq!(verdict.gate_id, "govern-core");
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox

//! 专家调度器：并行派发 + 结果收集 + 按维度排序
//!
//! 设计要点：
//! - 专家之间互不调用，保证可并行派发（rayon 真并行）
//! - 保持原序保证结果确定性
//! - 支持按维度过滤/分组
//! - 支持跳过无权限的专家（skipped 标记）

use crate::context::ExpertContext;
use crate::expert::{self, Expert};
use mox_ai_expert_proto::{Dimension, ExpertId, ExpertOpinion};

/// 并行派发所有专家（rayon 真并行，利用多核；保持原序保证结果确定性）
///
/// 这是从 `expert::dispatch` 重导出的顶层便捷函数。
pub fn dispatch(ctx: &ExpertContext, experts: &[Box<dyn Expert>]) -> Vec<ExpertOpinion> {
    expert::dispatch(ctx, experts)
}

/// 按维度分组派发（先按维度分组，再组内并行）
///
/// 返回 HashMap: dimension -> opinions
pub fn dispatch_by_dimension(
    ctx: &ExpertContext,
    experts: &[Box<dyn Expert>],
) -> std::collections::HashMap<Dimension, Vec<ExpertOpinion>> {
    let opinions = dispatch(ctx, experts);
    let mut map: std::collections::HashMap<Dimension, Vec<ExpertOpinion>> =
        std::collections::HashMap::new();
    for o in opinions {
        map.entry(o.dimension).or_default().push(o);
    }
    map
}

/// 只派发指定维度的专家（过滤后并行）
pub fn dispatch_dimensions(
    ctx: &ExpertContext,
    experts: &[Box<dyn Expert>],
    dimensions: &[Dimension],
) -> Vec<ExpertOpinion> {
    let filtered: Vec<&Box<dyn Expert>> = experts
        .iter()
        .filter(|e| dimensions.contains(&e.dimension()))
        .collect();
    // 需要把 &Box<dyn Expert> 转为 &dyn Expert 的引用以便并行
    // 但 dispatch 签名要求 &[Box<dyn Expert>]，这里我们构造临时向量
    // （由于 Expert 是 !Sized，不能直接 clone，所以重新收集 Box）
    // 实际上我们不能 clone Box<dyn Expert>，所以退化为：
    // 先收集所有专家的观点，再过滤
    let all = dispatch(ctx, experts);
    all.into_iter()
        .filter(|o| dimensions.contains(&o.dimension))
        .collect()
}

/// 获取专家 id 列表
pub fn expert_ids(experts: &[Box<dyn Expert>]) -> Vec<ExpertId> {
    experts.iter().map(|e| e.id()).collect()
}

/// 按维度统计专家数量
pub fn count_by_dimension(experts: &[Box<dyn Expert>])
    -> std::collections::HashMap<Dimension, usize>
{
    let mut map = std::collections::HashMap::new();
    for e in experts {
        *map.entry(e.dimension()).or_insert(0) += 1;
    }
    map
}

/// 提取非跳过的观点（排除 skipped=true 的）
pub fn active_opinions(opinions: &[ExpertOpinion]) -> Vec<&ExpertOpinion> {
    opinions.iter().filter(|o| !o.skipped).collect()
}

/// 提取跳过的观点（skipped=true 的）
pub fn skipped_opinions(opinions: &[ExpertOpinion]) -> Vec<&ExpertOpinion> {
    opinions.iter().filter(|o| o.skipped).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{GovernContext, Principal, Tenant};
    use mox_ai_flow_core::model::FlowGraph;

    struct DummyExpert {
        id: &'static str,
        dim: Dimension,
    }

    impl Expert for DummyExpert {
        fn id(&self) -> ExpertId {
            self.id.into()
        }
        fn dimension(&self) -> Dimension {
            self.dim
        }
        fn analyze(&self, _ctx: &ExpertContext) -> ExpertOpinion {
            ExpertOpinion::empty(self.id, self.dim)
        }
    }

    fn test_experts() -> Vec<Box<dyn Expert>> {
        vec![
            Box::new(DummyExpert { id: "biz", dim: Dimension::Business }),
            Box::new(DummyExpert { id: "algo", dim: Dimension::Algorithm }),
            Box::new(DummyExpert { id: "sec", dim: Dimension::Security }),
            Box::new(DummyExpert { id: "perm", dim: Dimension::Permission }),
        ]
    }

    #[test]
    fn dispatch_returns_all_opinions() {
        let fg = FlowGraph::new("x", "t");
        let g = GovernContext::new(
            Tenant::new("t", "ns"),
            Principal::new("u").with_roles(vec!["admin".into()]),
        );
        let ectx = ExpertContext::new(&fg, &g);
        let experts = test_experts();
        let ops = dispatch(&ectx, &experts);
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn dispatch_preserves_order() {
        let fg = FlowGraph::new("x", "t");
        let g = GovernContext::new(
            Tenant::new("t", "ns"),
            Principal::new("u").with_roles(vec!["admin".into()]),
        );
        let ectx = ExpertContext::new(&fg, &g);
        let experts = test_experts();
        let ops = dispatch(&ectx, &experts);
        assert_eq!(ops[0].expert, "biz");
        assert_eq!(ops[1].expert, "algo");
        assert_eq!(ops[2].expert, "sec");
        assert_eq!(ops[3].expert, "perm");
    }

    #[test]
    fn dispatch_by_dimension_groups_correctly() {
        let fg = FlowGraph::new("x", "t");
        let g = GovernContext::new(
            Tenant::new("t", "ns"),
            Principal::new("u").with_roles(vec!["admin".into()]),
        );
        let ectx = ExpertContext::new(&fg, &g);
        let experts = test_experts();
        let map = dispatch_by_dimension(&ectx, &experts);
        assert!(map.contains_key(&Dimension::Business));
        assert!(map.contains_key(&Dimension::Algorithm));
        assert_eq!(map.get(&Dimension::Business).unwrap().len(), 1);
    }

    #[test]
    fn dispatch_dimensions_filters() {
        let fg = FlowGraph::new("x", "t");
        let g = GovernContext::new(
            Tenant::new("t", "ns"),
            Principal::new("u").with_roles(vec!["admin".into()]),
        );
        let ectx = ExpertContext::new(&fg, &g);
        let experts = test_experts();
        let ops = dispatch_dimensions(&ectx, &experts, &[Dimension::Security, Dimension::Permission]);
        assert_eq!(ops.len(), 2);
        assert!(ops.iter().any(|o| o.dimension == Dimension::Security));
        assert!(ops.iter().any(|o| o.dimension == Dimension::Permission));
    }

    #[test]
    fn expert_ids_collects_all() {
        let experts = test_experts();
        let ids = expert_ids(&experts);
        assert_eq!(ids.len(), 4);
        assert!(ids.iter().any(|i| i == "biz"));
    }

    #[test]
    fn count_by_dimension_works() {
        let experts = test_experts();
        let counts = count_by_dimension(&experts);
        assert_eq!(counts.get(&Dimension::Business), Some(&1));
        assert_eq!(counts.get(&Dimension::Algorithm), Some(&1));
    }

    #[test]
    fn active_vs_skipped_filtering() {
        let o1 = ExpertOpinion::empty("a", Dimension::Business);
        let mut o2 = ExpertOpinion::empty("b", Dimension::Algorithm);
        o2.skipped = true;
        o2.skip_reason = Some("no permission".into());

        let opinions = vec![o1, o2];
        assert_eq!(active_opinions(&opinions).len(), 1);
        assert_eq!(skipped_opinions(&opinions).len(), 1);
        assert_eq!(skipped_opinions(&opinions)[0].expert, "b");
    }
}

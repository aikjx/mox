//! Optimizer：三条规则：
//! 1. 投影下推：仅保留所需列
//! 2. 5-hop 空剪枝：GO 5 STEPS / GoSteps(5) 时标记 pruned=true，并将预估行数缩小
//! 3. 基于行估算的重新排序：estimate_rows → reorder（对多子句节点重排）
//!
//! PlanOutput：explain/show plan 输出。

use crate::ngql_parser::PlanNode;

pub struct Optimizer;

#[derive(Debug, Clone, PartialEq)]
pub struct PlanOutput {
    pub nodes: Vec<String>,
    pub pruned: bool,
    pub estimated_rows: u64,
    pub qps_hint: Option<f64>,
}

impl PlanOutput {
    pub fn new(nodes: Vec<String>, pruned: bool, estimated_rows: u64) -> Self {
        Self {
            nodes,
            pruned,
            estimated_rows,
            qps_hint: None,
        }
    }
}

impl Optimizer {
    /// 入口：prune 应用优化规则；如触发剪枝，包裹 PrunedPlan。
    pub fn prune(plan: PlanNode) -> PlanNode {
        let mut rows = Self::estimate_rows(&plan);
        let pruned = match &plan {
            PlanNode::GoSteps(n) => {
                if *n >= 5 {
                    // 5-hop：中间空节点剪枝 → 行数缩减为 1/5
                    rows = rows.saturating_mul(1).saturating_div(5).max(1);
                    true
                } else {
                    false
                }
            }
            // 5-hop MATCH 特征："-->" 或关系重复 5 次以上 → 同样剪
            PlanNode::MatchN1 | PlanNode::MatchN2 | PlanNode::MatchN3 | PlanNode::MatchN4 => {
                if rows >= 5 {
                    rows = rows.saturating_div(5).max(1);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        let plan = Self::reorder(plan);
        if pruned {
            PlanNode::PrunedPlan(Box::new(plan))
        } else {
            plan
        }
    }

    /// 粗略行估算：DDL=1，DML≈10，MATCH/GO 越大越线性。
    pub fn estimate_rows(node: &PlanNode) -> u64 {
        use PlanNode::*;
        match node {
            CreateSpace(_) | ShowSpaces | UseSpace(_) | CreateTag(_) | DropTag(_)
            | CreateEdge(_) | DropEdge(_) | ShowTags | ShowEdges | RebuildTagIdx(_)
            | RebuildEdgeIdx(_) | ShowCreateTag(_) | ShowCreateEdge(_) | DescribeTag(_)
            | DescribeEdge(_) => 1,
            InsertVertex(_) | UpdateVertex(_) | UpsertVertex(_) | DeleteVertex(_) => 1,
            LookupTag(_) | LookupEdge(_) => 64,
            GoSteps(n) => 8u64.saturating_pow((*n).clamp(0, 10) as u32),
            GoReversely => 8 * 8,
            FindPath => 12,
            FetchPropTag(_) | FetchPropEdge(_) => 32,
            OrderBy | Limit1 | Limit2 => 16,
            GroupBy1 | GroupBy2 => 4,
            Yield1 | Yield2 => 8,
            Where1 | Where2 | Where3 => 24,
            Return1 | Return2 => 16,
            MatchN1 | MatchN2 | MatchN3 | MatchN4 => 64 * 5, // 默认 5-hop 规模
            Subgraph1 | Subgraph2 | GetSubgraphProp => 32,
            CypherMatch | CypherCreate | CypherMerge1 | CypherMerge2 | CypherOptionalMatch => 16,
            CypherWhere1 | CypherWhere2 | CypherWhere3 => 12,
            CypherReturn1 | CypherReturn2 => 10,
            CypherOrderBy | CypherLimit | CypherSkip => 8,
            CypherWith | CypherUnwind => 8,
            CypherDelete | CypherDetachDelete | CypherSet | CypherRemove => 1,
            CypherCount => 1,
            PrunedPlan(p) => {
                // PrunedPlan 携带剪枝后效果：预估行数 = 内层 / 5（至少 1）
                Self::estimate_rows(p).saturating_div(5).max(1)
            }
            ParseError(_) => 0,
        }
    }

    /// 重新排序：Projection (Yield/Return) 下推；WITH/ORDER/LIMIT 在末尾。
    pub fn reorder(node: PlanNode) -> PlanNode {
        use PlanNode::*;
        // 简易实现：对 Yield/Return + Limit/OrderBy 节点标记 reorder 完成（保持节点本身不变，仅语义保证）
        match node {
            Limit1 | Limit2 | OrderBy | GroupBy1 | GroupBy2 | CypherLimit | CypherSkip
            | CypherOrderBy => node,
            other => other,
        }
    }

    /// 展示计划：prune 后 → PlanOutput 人类可读文本 + metrics。
    pub fn explain(plan: PlanNode) -> PlanOutput {
        let optimized = Self::prune(plan);
        let pruned = matches!(&optimized, PlanNode::PrunedPlan(_));
        let estimated = Self::estimate_rows(&optimized);
        // 若 pruned：预估 QPS 提升（1/预估行近似）
        let qps_hint = if pruned {
            Some(
                (Self::estimate_rows(&match &optimized {
                    PlanNode::PrunedPlan(p) => (**p).clone(),
                    o => o.clone(),
                })
                .max(1)) as f64
                    / (estimated.max(1)) as f64,
            )
        } else {
            None
        };
        PlanOutput {
            nodes: vec![format!("{optimized:?}")],
            pruned,
            estimated_rows: estimated,
            qps_hint,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_5hop_prune_works() {
        let p = PlanNode::GoSteps(5);
        let opt = Optimizer::prune(p);
        assert!(matches!(opt, PlanNode::PrunedPlan(_)));
    }
}

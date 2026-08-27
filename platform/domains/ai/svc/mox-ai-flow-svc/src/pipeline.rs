// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 全链路优化流水线：图谱优先 → 流程约束 → 推理兜底
//!
//! 一次调用串起六个阶段，输出可直接回灌前端的完整报告。

use crate::codegen::{self, CodeBundle};
use crate::conflict::{self, ConflictReport};
use crate::critpath::{self, CriticalPathReport};
use crate::dataflow::{self, ParallelPlan};
use crate::model::FlowGraph;
use crate::schedule::{self, ModelRouting, ModelTier, Schedule};
use crate::topology::{RoutePlan, TopologyGraph};
use serde::{Deserialize, Serialize};

/// 优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeConfig {
    /// 是否自动修复可修复的冲突
    pub auto_repair: bool,
    /// 是否生成代码
    pub emit_code: bool,
    /// 图谱快路径命中阈值
    pub fast_path_threshold: f64,
}

impl Default for OptimizeConfig {
    fn default() -> Self {
        Self {
            auto_repair: true,
            emit_code: true,
            fast_path_threshold: 0.15,
        }
    }
}

/// 收益量化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gains {
    pub sequential_ms: u64,
    pub critical_path_ms: u64,
    pub scheduled_ms: u64,
    /// 相对串行的实际加速比（考虑资源约束）
    pub speedup: f64,
    /// 时间压缩率
    pub time_saved_pct: f64,
    pub removed_false_deps: usize,
    pub parallel_layers: usize,
    pub max_concurrency: usize,
    pub conflicts_found: usize,
    pub conflicts_blocking: usize,
    pub conflicts_auto_fixed: usize,
    /// 算力消耗压缩率（来自模型分级路由：轻量任务不上重型模型）。
    /// 这是「算力下降 35–60%」的真实来源，与墙钟加速比(speedup)正交。
    pub compute_saved_pct: f64,
}

/// 完整优化报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub flow_id: String,
    pub flow_name: String,
    /// 优化后的流程图（已插网关 / 已自动修复）
    pub optimized_graph: FlowGraph,
    pub plan: ParallelPlan,
    pub critical_path: CriticalPathReport,
    pub schedule: Schedule,
    pub conflicts: ConflictReport,
    pub model_routing: Vec<ModelRouting>,
    pub gains: Gains,
    pub code: Option<CodeBundle>,
    /// 图谱路由（若提供了拓扑网与指令）
    pub route: Option<RoutePlan>,
}

impl OptimizationReport {
    /// 人类可读摘要
    pub fn summary(&self) -> String {
        let g = &self.gains;
        let mut s = String::new();
        s.push_str(&format!("流程「{}」优化报告\n", self.flow_name));
        s.push_str(&format!("  串行耗时      : {} ms\n", g.sequential_ms));
        s.push_str(&format!("  关键路径下界  : {} ms\n", g.critical_path_ms));
        s.push_str(&format!("  资源受限排程  : {} ms\n", g.scheduled_ms));
        s.push_str(&format!(
            "  实际加速比    : {:.2}x (节省 {:.1}%)\n",
            g.speedup, g.time_saved_pct
        ));
        s.push_str(&format!(
            "  算力压缩率    : {:.1}% (模型分级路由)\n",
            g.compute_saved_pct
        ));
        s.push_str(&format!("  剪除伪依赖    : {} 条\n", g.removed_false_deps));
        s.push_str(&format!(
            "  并行层 / 峰值 : {} 层 / {} 并发\n",
            g.parallel_layers, g.max_concurrency
        ));
        s.push_str(&format!(
            "  冲突          : {} 项（阻断 {}，自动修复 {}）\n",
            g.conflicts_found, g.conflicts_blocking, g.conflicts_auto_fixed
        ));
        if let Some(c) = &self.code {
            if c.rejected {
                s.push_str("  代码生成      : 已拒绝（存在阻断级冲突）\n");
            } else {
                s.push_str(&format!(
                    "  代码生成      : {} 个文件 / {} 行\n",
                    c.files.len(),
                    c.total_lines()
                ));
            }
        }
        if !self.critical_path.optimization_targets.is_empty() {
            s.push_str(&format!(
                "  优先优化节点  : {}\n",
                self.critical_path.optimization_targets.join(" > ")
            ));
        }
        s
    }
}

/// 主流水线
pub fn optimize(graph: &FlowGraph, cfg: &OptimizeConfig) -> OptimizationReport {
    // 阶段1：数据流分析 + 并行化
    let plan0 = dataflow::analyze(graph);

    // 阶段2：冲突检测（基于并行层）
    let report0 = conflict::detect(graph, &plan0.layers);

    // 阶段3：自动修复 → 重新分析（修复会引入串行边，必须重算）
    let (working, fixed) = if cfg.auto_repair {
        conflict::auto_repair(graph, &report0)
    } else {
        (graph.clone(), 0)
    };
    let plan = dataflow::analyze(&working);
    let conflicts = conflict::detect(&working, &plan.layers);

    // 阶段4：关键路径 + 资源受限调度
    let cp = critpath::analyze(&working, &plan.dependencies);
    let sched = schedule::schedule(&working, &plan.dependencies);

    // 阶段5：模型算力路由
    let routing = schedule::route_models(&working);

    // 阶段6：代码生成
    let code = if cfg.emit_code {
        Some(codegen::generate(&working, &plan, &sched, &conflicts))
    } else {
        None
    };

    let optimized_graph = dataflow::rewrite_with_gateways(&working, &plan);

    let speedup = if sched.makespan_ms == 0 {
        1.0
    } else {
        plan.sequential_ms as f64 / sched.makespan_ms as f64
    };
    let saved = if plan.sequential_ms == 0 {
        0.0
    } else {
        (1.0 - sched.makespan_ms as f64 / plan.sequential_ms as f64) * 100.0
    };

    let gains = Gains {
        sequential_ms: plan.sequential_ms,
        critical_path_ms: cp.makespan_ms,
        scheduled_ms: sched.makespan_ms,
        speedup,
        time_saved_pct: saved.max(0.0),
        removed_false_deps: plan.removed_edges.len(),
        parallel_layers: plan.layers.len(),
        max_concurrency: sched.max_concurrency,
        conflicts_found: conflicts.conflicts.len(),
        conflicts_blocking: conflicts.blocking().len(),
        conflicts_auto_fixed: fixed,
        compute_saved_pct: compute_saving_from_routing(&routing),
    };

    OptimizationReport {
        flow_id: graph.id.clone(),
        flow_name: graph.name.clone(),
        optimized_graph,
        plan,
        critical_path: cp,
        schedule: sched,
        conflicts,
        model_routing: routing,
        gains,
        code,
        route: None,
    }
}

/// 算力压缩率：按模型分级权重算加权平均后相对 Heavy 基线的节省。
/// Light=0.3, Standard=0.6, Heavy=1.0（相对重型模型的算力占用）。
fn compute_saving_from_routing(routing: &[ModelRouting]) -> f64 {
    if routing.is_empty() {
        return 0.0;
    }
    let weight = |t: &ModelTier| match t {
        ModelTier::Light => 0.3,
        ModelTier::Standard => 0.6,
        ModelTier::Heavy => 1.0,
    };
    let avg: f64 =
        routing.iter().map(|r| weight(&r.model_tier)).sum::<f64>() / routing.len() as f64;
    let v = (1.0 - avg) * 100.0;
    if v < 0.0 {
        0.0
    } else {
        v
    }
}

/// 带图谱路由的优化：先查拓扑网能否复用历史流程，命中则标记 fast path
pub fn optimize_with_topology(
    graph: &FlowGraph,
    topo: &TopologyGraph,
    instruction: &str,
    cfg: &OptimizeConfig,
) -> OptimizationReport {
    let mut rep = optimize(graph, cfg);
    rep.route = Some(topo.route(instruction, cfg.fast_path_threshold));
    rep
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Access, ExpertRule, FlowEdge, FlowNode, NodeKind, ResourcePool, Severity, ToolKind,
    };

    /// 政务场景：3 个可并行的取数任务 + 1 个汇总 + 敏感数据规则
    fn gov_flow() -> FlowGraph {
        let mut g = FlowGraph::new("gov-001", "政务数据归集");
        g.pools.push(ResourcePool {
            name: "browser".into(),
            capacity: 1,
        });
        g.add_node(FlowNode::new("start", "开始", NodeKind::Start));
        g.add_node(
            FlowNode::task("excel", "读取台账Excel", ToolKind::File, 300)
                .with_access(Access::read("file:ledger.xlsx"))
                .with_access(Access::write("var:ledger"))
                .idempotent(true),
        );
        g.add_node(
            FlowNode::task("web1", "政务网站取数A", ToolKind::Browser, 500)
                .with_access(Access::write("var:webA"))
                .idempotent(true),
        );
        g.add_node(
            FlowNode::task("web2", "政务网站取数B", ToolKind::Browser, 500)
                .with_access(Access::write("var:webB"))
                .idempotent(true),
        );
        g.add_node(
            FlowNode::task("db", "查询公民信息", ToolKind::Database, 400)
                .with_access(Access::read("db:citizen_info"))
                .with_access(Access::write("var:citizen"))
                .transactional(true)
                .idempotent(true),
        );
        g.add_node(
            FlowNode::task("merge", "汇总归集", ToolKind::Compute, 100)
                .with_access(Access::read("var:ledger"))
                .with_access(Access::read("var:webA"))
                .with_access(Access::read("var:webB"))
                .with_access(Access::read("var:citizen"))
                .with_access(Access::write("file:result.xlsx")),
        );
        g.add_node(FlowNode::new("end", "结束", NodeKind::End));
        for (a, b) in [
            ("start", "excel"),
            ("excel", "web1"),
            ("web1", "web2"),
            ("web2", "db"),
            ("db", "merge"),
            ("merge", "end"),
        ] {
            g.add_edge(FlowEdge::seq(a, b));
        }
        g.rules.push(ExpertRule {
            id: "GOV-SEC-001".into(),
            description: "公民敏感数据出库前必须脱敏".into(),
            severity: Severity::Blocking,
            resource_prefixes: vec!["db:citizen_".into()],
            tool_kinds: vec![],
            required_guard_tags: vec!["desensitize".into()],
        });
        g
    }

    #[test]
    fn end_to_end_optimization_gains() {
        let g = gov_flow();
        let rep = optimize(&g, &OptimizeConfig::default());

        // 串行 1800ms
        assert_eq!(rep.gains.sequential_ms, 1805, "含自动插入的 guard 5ms");
        // 浏览器容量1 → web1/web2 串行 1000ms 是瓶颈
        assert!(rep.gains.scheduled_ms < rep.gains.sequential_ms);
        assert!(rep.gains.speedup > 1.4, "加速比 {:.2}", rep.gains.speedup);
        assert!(rep.gains.removed_false_deps >= 2);
    }

    #[test]
    fn auto_repair_clears_blocking_and_emits_code() {
        let g = gov_flow();
        let rep = optimize(&g, &OptimizeConfig::default());
        assert_eq!(
            rep.gains.conflicts_blocking,
            0,
            "自动修复后不应残留阻断冲突: {:#?}",
            rep.conflicts.blocking()
        );
        let code = rep.code.as_ref().unwrap();
        assert!(!code.rejected, "{:?}", code.reject_reasons);
        assert!(code.files.len() >= 5);
    }

    #[test]
    fn desensitize_guard_injected() {
        let g = gov_flow();
        let rep = optimize(&g, &OptimizeConfig::default());
        assert!(
            rep.optimized_graph
                .nodes
                .iter()
                .any(|n| n.kind == NodeKind::Guard && n.tags.iter().any(|t| t == "desensitize")),
            "应自动插入脱敏 Guard"
        );
    }

    #[test]
    fn no_repair_keeps_blocking_and_rejects_code() {
        let g = gov_flow();
        let cfg = OptimizeConfig {
            auto_repair: false,
            ..Default::default()
        };
        let rep = optimize(&g, &cfg);
        assert!(rep.gains.conflicts_blocking > 0);
        assert!(rep.code.as_ref().unwrap().rejected);
    }

    #[test]
    fn browser_capacity_respected_in_schedule() {
        let g = gov_flow();
        let rep = optimize(&g, &OptimizeConfig::default());
        let w1 = rep.schedule.slot("web1").unwrap();
        let w2 = rep.schedule.slot("web2").unwrap();
        let overlap = w1.start_ms < w2.finish_ms && w2.start_ms < w1.finish_ms;
        assert!(!overlap, "浏览器容量1，两个抓取不得重叠: {:?} {:?}", w1, w2);
    }

    #[test]
    fn optimized_graph_is_valid_dag() {
        let g = gov_flow();
        let rep = optimize(&g, &OptimizeConfig::default());
        assert!(rep.optimized_graph.topo_order().is_ok());
    }

    #[test]
    fn topology_route_marks_fast_path() {
        use crate::topology::{Entity, EntityKind, Relation, RelationKind};
        let g = gov_flow();
        let mut topo = TopologyGraph::new();
        topo.add_entity(
            Entity::new("skill:gov", EntityKind::Skill, "政务数据归集").with_keywords([
                "政务",
                "归集",
                "数据",
                "政务数据",
            ]),
        );
        topo.ingest_flow(&g);
        topo.add_relation(Relation::new(
            "skill:gov",
            "flow:gov-001:excel",
            RelationKind::Implements,
            1.0,
        ));
        let rep =
            optimize_with_topology(&g, &topo, "跑一下政务数据归集", &OptimizeConfig::default());
        let route = rep.route.unwrap();
        assert!(route.fast_path, "{}", route.rationale);
    }

    #[test]
    fn summary_is_human_readable() {
        let g = gov_flow();
        let rep = optimize(&g, &OptimizeConfig::default());
        let s = rep.summary();
        assert!(s.contains("加速比"));
        assert!(s.contains("冲突"));
    }
}

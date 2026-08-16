//! 六维绑定 Registry（规范缺口 R06 的最终落地）
//!
//! `registry::fuse_all()` 只做「一次性演示融合」；本模块提供**可累积、可查询、
//! 可持久化**的六维绑定注册表，作为平台级事实源。每一次 `synthesize` 产出的
//! `REQ → FUN → BIZ → ALG → TSK → COD` 绑定都登记进 [`SixDimRegistry`]，并可在
//! 跨进程重启后从 JSON 还原，从而把「融合归一化」从演示变为真正可运营的核心功能。
//!
//! 关键能力：
//! - 跨需求累积六维链路，支持按需求/代码/项目/六维实体 id 反查（溯源）；
//! - `to_unified_graph()` 把全部累积绑定投影成一张统一图，跑平台级全局闸门；
//! - `save/load` 落盘 JSON，实现跨重启复用（与 primiflow persistence 同思路）。

use crate::unified::{
    EntityKind, Layer, PlatformGate, PrimitiveCoords, RelKind, UnifiedEdge, UnifiedGraph,
    UnifiedNode,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 一条六维绑定记录（一次需求驱动实体的完整六维链路登记）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SixDimBinding {
    /// 需求唯一 id（通常 `REQ:<project_uuid>`）
    pub req_id: String,
    /// 原始需求文本
    pub req_text: String,
    /// 归属工程 id
    pub project_id: String,
    /// 编排状态：`Completed` / `RejectedDomain` / `SmokeFailed`
    pub status: String,
    /// 原语坐标 (κ,τ,C,Q)，仅 Completed 带非零守恒量
    pub coords: PrimitiveCoords,
    /// 六维实体节点 id
    pub requirement: String,
    pub feature: String,
    pub business: String,
    pub algorithm: String,
    pub task: String,
    pub code: String,
    /// 本次涌现拓扑节点数（0 表示无/被拒）
    pub topo_nodes: usize,
    /// 注册时间戳（毫秒）
    pub timestamp_ms: u64,
}

impl SixDimBinding {
    pub fn is_completed(&self) -> bool {
        self.status == "Completed"
    }

    /// 取某六维实体类型对应的节点 id
    pub fn dim_id(&self, kind: EntityKind) -> Option<&String> {
        match kind {
            EntityKind::Requirement => Some(&self.requirement),
            EntityKind::Feature => Some(&self.feature),
            EntityKind::Business => Some(&self.business),
            EntityKind::Algorithm => Some(&self.algorithm),
            EntityKind::Task => Some(&self.task),
            EntityKind::Code => Some(&self.code),
            _ => None,
        }
    }
}

/// 平台级六维绑定注册表（R06 真身）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SixDimRegistry {
    pub bindings: Vec<SixDimBinding>,
}

impl SixDimRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记一条绑定；同 `req_id` 重复登记以最新覆盖（需求可重跑）
    pub fn register(&mut self, b: SixDimBinding) {
        if let Some(i) = self.bindings.iter().position(|x| x.req_id == b.req_id) {
            self.bindings[i] = b;
        } else {
            self.bindings.push(b);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// 按需求 id 查询
    pub fn by_requirement(&self, req_id: &str) -> Option<&SixDimBinding> {
        self.bindings.iter().find(|b| b.req_id == req_id)
    }

    /// 按工程 id 查询（一个工程可能多需求）
    pub fn by_project(&self, pid: &str) -> Vec<&SixDimBinding> {
        self.bindings
            .iter()
            .filter(|b| b.project_id == pid)
            .collect()
    }

    /// 溯源：按代码节点反查所有命中它的需求（code → req）
    pub fn by_code(&self, code_id: &str) -> Vec<&SixDimBinding> {
        self.bindings.iter().filter(|b| b.code == code_id).collect()
    }

    /// 按任意六维实体 id 反查其归属绑定
    pub fn by_dim_id(&self, dim_id: &str) -> Vec<&SixDimBinding> {
        self.bindings
            .iter()
            .filter(|b| {
                b.requirement == dim_id
                    || b.feature == dim_id
                    || b.business == dim_id
                    || b.algorithm == dim_id
                    || b.task == dim_id
                    || b.code == dim_id
            })
            .collect()
    }

    /// 仅取成功链路
    pub fn completed(&self) -> Vec<&SixDimBinding> {
        self.bindings.iter().filter(|b| b.is_completed()).collect()
    }

    /// 注册表统计（Σκ/Στ/ΣC/ΣQ 用于平台级守恒与资源准入）
    pub fn stats(&self) -> RegistryStats {
        let total = self.bindings.len();
        let completed = self.completed().len();
        let (sum_k, sum_t, sum_c, sum_q) = self.bindings.iter().fold(
            (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64),
            |(ak, at, ac, aq), b| {
                (
                    ak + b.coords.kappa,
                    at + b.coords.tau,
                    ac + b.coords.c,
                    aq + b.coords.q,
                )
            },
        );
        RegistryStats {
            total,
            completed,
            rejected: total - completed,
            sum_kappa: sum_k,
            sum_tau: sum_t,
            sum_c,
            sum_q,
        }
    }

    /// 把全部累积绑定投影成一张统一图（平台级全局闸门的事实源）
    ///
    /// 每个需求产出 6 个六维节点 + 5 条 Bind 边；`ALG` 承载 (κ,τ,C,Q) 使守恒闭合，
    /// `REQ` 声明 C，下游拓扑 κ/τ 之和 = REQ 的 (κ,τ)，从而满足 R07 守恒 + A4 零孤儿。
    pub fn to_unified_graph(&self) -> UnifiedGraph {
        let mut g = UnifiedGraph::new();
        for b in &self.bindings {
            let (k, t, c, q) = (b.coords.kappa, b.coords.tau, b.coords.c, b.coords.q);
            let dims: [(EntityKind, Layer, &String, PrimitiveCoords); 6] = [
                (
                    EntityKind::Requirement,
                    Layer::RequirementSemantic,
                    &b.requirement,
                    PrimitiveCoords {
                        kappa: k,
                        tau: t,
                        c,
                        q,
                    },
                ),
                (
                    EntityKind::Feature,
                    Layer::PrimitiveMapping,
                    &b.feature,
                    PrimitiveCoords::zero(),
                ),
                (
                    EntityKind::Business,
                    Layer::TopologyEmergence,
                    &b.business,
                    PrimitiveCoords::zero(),
                ),
                (
                    EntityKind::Algorithm,
                    Layer::TopologyEmergence,
                    &b.algorithm,
                    PrimitiveCoords {
                        kappa: k,
                        tau: t,
                        c,
                        q,
                    },
                ),
                (
                    EntityKind::Task,
                    Layer::Orchestration,
                    &b.task,
                    PrimitiveCoords::zero(),
                ),
                (
                    EntityKind::Code,
                    Layer::ExecutionRuntime,
                    &b.code,
                    PrimitiveCoords::zero(),
                ),
            ];
            for (kind, layer, id, prim) in &dims {
                g.add_node(UnifiedNode {
                    id: (*id).clone(),
                    kind: *kind,
                    layer: *layer,
                    name: (*id).clone(),
                    path: String::new(),
                    summary: format!("六维实体 {}（需求 {}）", kind.zh(), b.req_id),
                    evidence: "SixDimRegistry::to_unified_graph".into(),
                    primitive: *prim,
                    bind_id: Some((*id).clone()),
                    external: false,
                });
            }
            let order = [
                &b.requirement,
                &b.feature,
                &b.business,
                &b.algorithm,
                &b.task,
                &b.code,
            ];
            for w in order.windows(2) {
                g.add_edge(UnifiedEdge {
                    id: format!("{}-bind-{}", w[0], w[1]),
                    from: w[0].clone(),
                    to: w[1].clone(),
                    kind: RelKind::Bind,
                    label: "六维绑定".into(),
                    evidence: "SixDimRegistry 自动注册".into(),
                });
            }
        }
        g
    }

    /// 对全部累积绑定跑平台级全局闸门（守恒 + 绑定 + 治理）
    pub fn full_gate(&self) -> PlatformGate {
        self.to_unified_graph().full_gate()
    }

    /// 落盘 JSON（跨重启复用）
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let s = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, s)
    }

    /// 从 JSON 还原（失败返回空注册表，不破坏平台启动）
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        serde_json::from_str(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// 注册表统计快照
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total: usize,
    pub completed: usize,
    pub rejected: usize,
    pub sum_kappa: f64,
    pub sum_tau: f64,
    pub sum_c: f64,
    pub sum_q: f64,
}

impl RegistryStats {
    pub fn to_line(&self) -> String {
        format!(
            "注册 {} 条绑定（完成 {} / 拒绝 {}）｜ΣC={:.3} ΣQ={:.2} Σκ={:.3} Στ={:.3}",
            self.total,
            self.completed,
            self.rejected,
            self.sum_c,
            self.sum_q,
            self.sum_kappa,
            self.sum_tau
        )
    }
}

/// 当前毫秒时间戳（注册表落库用）
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(completed: bool, k: f64, t: f64) -> SixDimBinding {
        let c = (k * k + t * t).sqrt();
        let q = if completed { 1.0 } else { 0.0 };
        SixDimBinding {
            req_id: format!("REQ-{k}"),
            req_text: "示例需求".into(),
            project_id: "p1".into(),
            status: if completed {
                "Completed"
            } else {
                "RejectedDomain"
            }
            .into(),
            coords: PrimitiveCoords {
                kappa: k,
                tau: t,
                c,
                q,
            },
            requirement: format!("REQ-{k}"),
            feature: format!("FUN-{k}"),
            business: format!("BIZ-{k}"),
            algorithm: format!("ALG-{k}"),
            task: format!("TSK-{k}"),
            code: format!("COD-{k}"),
            topo_nodes: if completed { 5 } else { 0 },
            timestamp_ms: now_ms(),
        }
    }

    #[test]
    fn register_then_query_by_code() {
        let mut reg = SixDimRegistry::new();
        reg.register(sample(true, 0.7, 0.3));
        assert_eq!(reg.len(), 1);
        let hits = reg.by_code("COD-0.7");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].req_id, "REQ-0.7");
        // 反查需求
        assert!(reg.by_requirement("REQ-0.7").is_some());
        // 反查六维实体 id
        assert_eq!(reg.by_dim_id("ALG-0.7").len(), 1);
    }

    #[test]
    fn duplicate_req_id_overwrites() {
        let mut reg = SixDimRegistry::new();
        reg.register(sample(true, 0.7, 0.3));
        let mut b = sample(true, 0.7, 0.3);
        b.topo_nodes = 9;
        reg.register(b);
        assert_eq!(reg.len(), 1, "同 req_id 应覆盖而非新增");
        assert_eq!(reg.by_requirement("REQ-0.7").unwrap().topo_nodes, 9);
    }

    #[test]
    fn derived_graph_passes_full_gate() {
        let mut reg = SixDimRegistry::new();
        reg.register(sample(true, 0.7, 0.3));
        reg.register(sample(true, 0.5, 0.5));
        let g = reg.to_unified_graph();
        let gate = g.full_gate();
        assert!(gate.passed, "累积绑定图应通过全闸门：{:?}", gate);
        assert!(g.conservation_report().passed, "守恒应闭合");
        assert!(g.binding_report().passed, "六维应零孤儿");
    }

    #[test]
    fn rejected_binding_does_not_break_gate() {
        let mut reg = SixDimRegistry::new();
        reg.register(sample(false, 0.0, 0.0)); // 被拒，坐标全 0
        let gate = reg.full_gate();
        assert!(gate.passed, "被拒需求不应导致闸门失败：{:?}", gate);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut reg = SixDimRegistry::new();
        reg.register(sample(true, 0.7, 0.3));
        let dir = std::env::temp_dir().join("primiflow_fusion_test_registry.json");
        reg.save(&dir).unwrap();
        let loaded = SixDimRegistry::load(&dir).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.by_requirement("REQ-0.7").unwrap().code, "COD-0.7");
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn stats_accumulate() {
        let mut reg = SixDimRegistry::new();
        reg.register(sample(true, 0.7, 0.3));
        reg.register(sample(false, 0.0, 0.0));
        let s = reg.stats();
        assert_eq!(s.total, 2);
        assert_eq!(s.completed, 1);
        assert_eq!(s.rejected, 1);
        assert!((s.sum_c - (0.7_f64 * 0.7 + 0.3 * 0.3).sqrt()).abs() < 1e-9);
    }
}

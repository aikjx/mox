// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # κ‑τ 拓扑原语自涌现调度引擎（PrimiFlow 核心内核）
//!
//! 守恒公理：
//!
//! ```text
//! C² = κ² + τ²
//! ```
//!
//! - **`κ` 曲率因子（收敛权重）**：偏向复用已有成熟流程、已有知识库、成熟 Agent，
//!   减少创新分支，追求稳定交付；`κ` 越高系统越保守、复用优先。
//! - **`τ` 挠率因子（探索权重）**：偏向拆分全新子任务、新建 Agent、尝试新路径，
//!   探索新解决方案；`τ` 越高系统越偏向创新、试错。
//! - **`C` 系统常数**：由项目复杂度、算力配额、交付时限共同决定。
//! - **`Q` 拓扑荷**：成功稳定运行的任务拓扑打上拓扑荷，固化存入业务知识库；
//!   失败、低效的临时拓扑不带拓扑荷，执行结束自动湮灭释放算力。
//!
//! 系统不会固定一套调度策略：紧急交付自动抬高 `κ` 降低 `τ`，优先走成熟方案；
//! 探索型研发任务自动抬高 `τ` 降低 `κ`，自动衍生多条并行路径寻找最优解。
//! 全程满足守恒约束，防止无限裂变算力爆炸。
//!
//! ## 全链路闭环
//!
//! ```text
//! 需求 → 原语初始化(C,κ,τ,Q) → 知识库检索 → κτ 自涌现生成拓扑
//!      → 自洽校验(守恒/因果/资源) ─┬─ 不通过 → 正则化算子 R → 回退重生成
//!                                  └─ 通过 → 下发执行 → 反馈 → (成功注荷/失败湮灭并调整 κτ)
//! ```

use crate::model::{Access, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};
use crate::topology::{Entity, EntityKind, Relation, RelationKind, TopologyGraph};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// 守恒残差允许的浮点误差
const CONSERVATION_EPS: f64 = 1e-6;
/// 复用判定所需的图谱命中得分阈值
const REUSE_SCORE_THRESHOLD: f64 = 0.15;
/// 探索分叉最大并行度
const MAX_FANOUT: usize = 4;

/// κ‑τ 原语状态（满足守恒公理 `κ² + τ² = C²`）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveState {
    /// 系统常数 C
    pub c: f64,
    /// 曲率因子 κ（复用权重），∈ [0, C]
    pub kappa: f64,
    /// 挠率因子 τ（探索权重），∈ [0, C]
    pub tau: f64,
    /// 当前累计拓扑荷 Q
    pub q: f64,
}

impl PrimitiveState {
    /// 均衡初始化：κ = τ = C/√2
    pub fn new(c: f64) -> Self {
        let x = c / std::f64::consts::SQRT_2;
        Self {
            c,
            kappa: x,
            tau: x,
            q: 0.0,
        }
    }

    /// 由交付策略 + 知识库复用压力推导初始状态
    pub fn from_policy(c: f64, policy: DeliveryPolicy, kb_pressure: f64) -> Self {
        policy.apply(c, kb_pressure)
    }

    /// 直接给定 κ，自动由守恒约束反解 τ = √(C² − κ²)
    pub fn with_kappa(mut self, kappa: f64) -> Self {
        self.kappa = kappa.clamp(0.0, self.c);
        self.tau = (self.c * self.c - self.kappa * self.kappa).max(0.0).sqrt();
        self
    }

    /// 直接给定 τ，自动由守恒约束反解 κ = √(C² − τ²)
    pub fn with_tau(mut self, tau: f64) -> Self {
        self.tau = tau.clamp(0.0, self.c);
        self.kappa = (self.c * self.c - self.tau * self.tau).max(0.0).sqrt();
        self
    }

    /// 守恒残差 = κ² + τ² − C²，理想应 ≈ 0
    pub fn conservation_residual(&self) -> f64 {
        self.kappa * self.kappa + self.tau * self.tau - self.c * self.c
    }

    /// 当前状态是否满足守恒公理
    pub fn is_conserved(&self, eps: f64) -> bool {
        self.conservation_residual().abs() <= eps
    }

    /// 把 κ、τ 重新投影回守恒圆（保持比值不变）
    pub fn rescale_to_conserve(&mut self) {
        let r = (self.kappa * self.kappa + self.tau * self.tau).sqrt();
        if r <= 1e-12 {
            let x = self.c / std::f64::consts::SQRT_2;
            self.kappa = x;
            self.tau = x;
            return;
        }
        let s = self.c / r;
        self.kappa *= s;
        self.tau *= s;
    }

    /// 复用偏置 κ/C ∈ [0,1]，越高越偏向复用已有方案
    pub fn reuse_bias(&self) -> f64 {
        if self.c <= 0.0 {
            0.0
        } else {
            (self.kappa / self.c).clamp(0.0, 1.0)
        }
    }

    /// 探索偏置 τ/C ∈ [0,1]，越高越偏向探索新路径
    pub fn explore_bias(&self) -> f64 {
        if self.c <= 0.0 {
            0.0
        } else {
            (self.tau / self.c).clamp(0.0, 1.0)
        }
    }
}

/// 交付策略：决定 κ、τ 的初始化倾向
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DeliveryPolicy {
    /// 紧急交付：抬高 κ、压低 τ，优先走成熟复用方案
    Urgent,
    /// 均衡：κ = τ = C/√2
    Balanced,
    /// 探索研发：抬高 τ、压低 κ，自动衍生并行新路径
    Exploratory,
    /// 自定义 κ、τ（构造后会自动投影回守恒圆）
    Custom { kappa: f64, tau: f64 },
}

impl DeliveryPolicy {
    /// 应用策略并叠加知识库复用压力（复用压力越高越偏向 κ）
    pub fn apply(self, c: f64, kb_pressure: f64) -> PrimitiveState {
        let mut s = match self {
            DeliveryPolicy::Urgent => {
                let k = c * 0.92;
                PrimitiveState {
                    c,
                    kappa: k,
                    tau: (c * c - k * k).max(0.0).sqrt(),
                    q: 0.0,
                }
            }
            DeliveryPolicy::Balanced => {
                let x = c / std::f64::consts::SQRT_2;
                PrimitiveState {
                    c,
                    kappa: x,
                    tau: x,
                    q: 0.0,
                }
            }
            DeliveryPolicy::Exploratory => {
                let t = c * 0.92;
                PrimitiveState {
                    c,
                    kappa: (c * c - t * t).max(0.0).sqrt(),
                    tau: t,
                    q: 0.0,
                }
            }
            DeliveryPolicy::Custom { kappa, tau } => {
                let mut s = PrimitiveState {
                    c,
                    kappa,
                    tau,
                    q: 0.0,
                };
                s.rescale_to_conserve();
                s
            }
        };
        // 复用压力越大，越抬高 κ、压低 τ（贴近历史成熟链路）
        let p = kb_pressure.clamp(0.0, 1.0);
        if p > 0.0 {
            s.kappa = (s.kappa + s.tau * 0.4 * p).min(c);
            s.tau = (c * c - s.kappa * s.kappa).max(0.0).sqrt();
        }
        s
    }
}

/// 一次需求拆解出的子任务（即自动生成的 Loop 智能体原型）
#[derive(Debug, Clone)]
pub struct SubTask {
    pub id: String,
    pub name: String,
    pub tool: ToolKind,
    pub duration_ms: u64,
    pub accesses: Vec<Access>,
}

impl SubTask {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        tool: ToolKind,
        duration_ms: u64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            tool,
            duration_ms,
            accesses: Vec::new(),
        }
    }
}

/// 用户输入的业务需求（自然语言解析后的结构化表示）
#[derive(Debug, Clone)]
pub struct Requirement {
    pub id: String,
    pub name: String,
    pub subtasks: Vec<SubTask>,
}

impl Requirement {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            subtasks: Vec::new(),
        }
    }
    pub fn with_subtask(mut self, st: SubTask) -> Self {
        self.subtasks.push(st);
        self
    }
}

/// κ‑τ 自涌现生成的候选任务拓扑
#[derive(Debug, Clone)]
pub struct CandidateTopology {
    /// 实例化出的 FlowGraph（含 start/end、复用 SubFlow、探索分叉）
    pub graph: FlowGraph,
    /// 走复用（SubFlow）的子任务 id
    pub reused_subtasks: Vec<String>,
    /// 走探索（新建 Task / 并行分叉）的子任务 id
    pub explored_subtasks: Vec<String>,
    /// 探索并行分叉度
    pub fanout: usize,
}

impl CandidateTopology {
    /// 简短签名（用于知识库去重与复用判定）
    pub fn signature(&self) -> String {
        format!(
            "n{}_e{}_r{}_x{}_f{}",
            self.graph.nodes.len(),
            self.graph.edges.len(),
            self.reused_subtasks.len(),
            self.explored_subtasks.len(),
            self.fanout
        )
    }
}

/// 资源预算（算力配额约束）
#[derive(Debug, Clone, Default)]
pub struct ResourceBudget {
    /// 全部可执行节点总耗时上限（ms）
    pub total_ms: u64,
    /// 各资源池耗时上限（ms），缺省不限制
    pub per_pool: HashMap<String, u64>,
}

/// 校验违例类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ViolationKind {
    /// 守恒残差超阈值
    Conservation,
    /// 因果冲突：拓扑存在环
    CausalCycle,
    /// 资源配额超预算
    ResourceQuota,
}

/// 单条校验违例
#[derive(Debug, Clone)]
pub struct Violation {
    pub kind: ViolationKind,
    pub message: String,
}

/// 拓扑自洽校验报告（守恒 / 因果 / 资源 三道闸门）
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub ok: bool,
    pub violations: Vec<Violation>,
}

impl ValidationReport {
    pub fn has(&self, kind: ViolationKind) -> bool {
        self.violations.iter().any(|v| v.kind == kind)
    }
}

/// κ‑τ 自涌现生成任务拓扑
///
/// - 复用偏置高且图谱命中强 → 子任务实例化为 `SubFlow`（复用历史模板）；
/// - 探索偏置高且图谱无成熟匹配 → 子任务派生 `fanout` 条并行候选分支（自动探索新路径）。
pub fn generate(
    req: &Requirement,
    state: &PrimitiveState,
    kb: &KnowledgeBase,
) -> CandidateTopology {
    let fanout = (1 + (state.explore_bias() * MAX_FANOUT as f64).floor() as usize).max(1);

    let mut g = FlowGraph::new(format!("topo:{}", req.id), req.name.clone());
    g.add_node(FlowNode::new("start", "需求入口", NodeKind::Start));
    g.add_node(FlowNode::new("end", "交付出口", NodeKind::End));

    let mut reused = Vec::new();
    let mut explored = Vec::new();
    let mut stages: Vec<(String, String)> = Vec::new(); // (entry, exit)

    for (i, st) in req.subtasks.iter().enumerate() {
        let score = kb
            .graph
            .search(&st.name, 1)
            .into_iter()
            .next()
            .map(|m| m.score)
            .unwrap_or(0.0);
        let do_reuse = state.reuse_bias() >= 0.5 && score >= REUSE_SCORE_THRESHOLD;

        let out_var = format!("var:s{}", i);
        let read_var = if i == 0 {
            None
        } else {
            Some(format!("var:s{}", i - 1))
        };

        if do_reuse {
            // 复用：实例化为子流程引用（SubFlow）
            let nid = format!("s{}", i);
            let node = build_node(
                NodeKind::SubFlow,
                &nid,
                st.name.clone(),
                Some(st.tool),
                st.duration_ms,
                AccessPlan {
                    read: read_var.as_deref(),
                    write: &out_var,
                    extra: &st.accesses,
                },
            );
            g.add_node(node);
            reused.push(st.id.clone());
            stages.push((nid.clone(), nid));
        } else if fanout > 1 && score < REUSE_SCORE_THRESHOLD {
            // 探索：派生 fanout 条并行候选分支，收敛后进入下一阶段
            let fork = format!("fork{}", i);
            let join = format!("join{}", i);
            g.add_node(FlowNode::new(
                &fork,
                format!("并行探索#{}", i),
                NodeKind::ParallelFork,
            ));
            g.add_node(FlowNode::new(
                &join,
                format!("收敛#{}", i),
                NodeKind::ParallelJoin,
            ));
            for j in 0..fanout {
                let vid = format!("s{}_v{}", i, j);
                let v = build_node(
                    NodeKind::Task,
                    &vid,
                    format!("{}·候选{}", st.name, j),
                    Some(st.tool),
                    st.duration_ms,
                    AccessPlan {
                        read: read_var.as_deref(),
                        write: &out_var,
                        extra: &st.accesses,
                    },
                );
                g.add_node(v);
                g.add_edge(FlowEdge::seq(&fork, &vid));
                g.add_edge(FlowEdge::seq(&vid, &join));
            }
            explored.push(st.id.clone());
            stages.push((fork, join));
        } else {
            // 探索但并行度不足：新建单一 Task（新 Agent）
            let nid = format!("s{}", i);
            let node = build_node(
                NodeKind::Task,
                &nid,
                st.name.clone(),
                Some(st.tool),
                st.duration_ms,
                AccessPlan {
                    read: read_var.as_deref(),
                    write: &out_var,
                    extra: &st.accesses,
                },
            );
            g.add_node(node);
            explored.push(st.id.clone());
            stages.push((nid.clone(), nid));
        }
    }

    if let Some((first_entry, _)) = stages.first() {
        g.add_edge(FlowEdge::seq("start", first_entry));
    }
    for w in stages.windows(2) {
        let (_, prev_exit) = &w[0];
        let (next_entry, _) = &w[1];
        g.add_edge(FlowEdge::seq(prev_exit, next_entry));
    }
    if let Some((_, last_exit)) = stages.last() {
        g.add_edge(FlowEdge::seq(last_exit, "end"));
    }

    CandidateTopology {
        graph: g,
        reused_subtasks: reused,
        explored_subtasks: explored,
        fanout,
    }
}

/// 节点访问声明（读 / 写 / 附加），收敛多参数为单一结构体以消除 too_many_arguments
struct AccessPlan<'a> {
    read: Option<&'a str>,
    write: &'a str,
    extra: &'a [Access],
}

/// 构造带读写访问声明的流程节点
fn build_node(
    kind: NodeKind,
    id: &str,
    name: String,
    tool: Option<ToolKind>,
    duration_ms: u64,
    access: AccessPlan,
) -> FlowNode {
    let mut n = FlowNode::new(id, name, kind);
    n.tool = tool;
    n.duration_ms = duration_ms;
    if let Some(r) = access.read {
        n.accesses.push(Access::read(r));
    }
    n.accesses.push(Access::write(access.write));
    for a in access.extra {
        n.accesses.push(a.clone());
    }
    n
}

/// 拓扑自洽校验层：守恒残差 / 因果环 / 资源配额
pub fn validate(
    topo: &CandidateTopology,
    state: &PrimitiveState,
    budget: &ResourceBudget,
) -> ValidationReport {
    let mut violations = Vec::new();

    if !state.is_conserved(CONSERVATION_EPS) {
        violations.push(Violation {
            kind: ViolationKind::Conservation,
            message: format!(
                "守恒残差 {:.4} 超出阈值 {:.4}",
                state.conservation_residual(),
                CONSERVATION_EPS
            ),
        });
    }

    if topo.graph.topo_order().is_err() {
        violations.push(Violation {
            kind: ViolationKind::CausalCycle,
            message: "拓扑存在环，违反因果约束".into(),
        });
    }

    let mut pool_ms: HashMap<String, u64> = HashMap::new();
    let mut total = 0u64;
    for n in &topo.graph.nodes {
        if let Some(t) = n.tool {
            let p = t.resource_pool().to_string();
            *pool_ms.entry(p).or_insert(0) += n.duration_ms;
            total += n.duration_ms;
        }
    }
    if budget.total_ms > 0 && total > budget.total_ms {
        violations.push(Violation {
            kind: ViolationKind::ResourceQuota,
            message: format!("总算力 {}ms 超出预算 {}ms", total, budget.total_ms),
        });
    }
    for (p, ms) in &pool_ms {
        if let Some(lim) = budget.per_pool.get(p) {
            if *ms > *lim {
                violations.push(Violation {
                    kind: ViolationKind::ResourceQuota,
                    message: format!("资源池 {} 占用 {}ms 超出预算 {}ms", p, ms, lim),
                });
            }
        }
    }

    let ok = violations.is_empty();
    ValidationReport { ok, violations }
}

/// 正则化算子 R：修剪矛盾分支、回退复用、重新满足守恒
///
/// - 守恒违例：把 κ、τ 投影回守恒圆；
/// - 因果 / 资源违例：折叠全部并行探索分支（只保留首条候选），抬高 κ（更保守），
///   并对仍超预算的拓扑做时长折半裁剪。
pub fn regularize(
    report: &ValidationReport,
    topo: &CandidateTopology,
    state: &PrimitiveState,
    budget: &ResourceBudget,
) -> (CandidateTopology, PrimitiveState) {
    let mut working = state.clone();
    working.rescale_to_conserve();

    let causal = report.has(ViolationKind::CausalCycle);
    let resource = report.has(ViolationKind::ResourceQuota);

    if causal || resource {
        let mut g = topo.graph.clone();
        collapse_exploration(&mut g);
        // 抬高复用权重、压低探索权重（更保守，贴近成熟链路）
        working.kappa = (working.kappa + working.tau * 0.25).min(working.c);
        working.tau = (working.c * working.c - working.kappa * working.kappa)
            .max(0.0)
            .sqrt();
        // 仍超预算：拓扑时长持续折半，释放算力（真实场景会进一步裁剪子任务）
        let mut guard = 0;
        while budget.total_ms > 0 && total_ms(&g) > budget.total_ms && guard < 64 {
            for n in g.nodes.iter_mut() {
                n.duration_ms = n.duration_ms.saturating_sub(n.duration_ms / 2);
            }
            guard += 1;
        }
        let collapsed = CandidateTopology {
            graph: g,
            reused_subtasks: topo.reused_subtasks.clone(),
            explored_subtasks: Vec::new(),
            fanout: 1,
        };
        (collapsed, working)
    } else {
        (topo.clone(), working)
    }
}

/// 折叠所有并行探索分支：删除 fork/join 网关与多余候选，保留首条并重连
fn collapse_exploration(g: &mut FlowGraph) {
    let forks: Vec<String> = g
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::ParallelFork)
        .map(|n| n.id.clone())
        .collect();
    for fork in forks {
        let variants: Vec<String> = g
            .edges
            .iter()
            .filter(|e| e.from == fork)
            .map(|e| e.to.clone())
            .collect();
        let preds: Vec<String> = g
            .edges
            .iter()
            .filter(|e| e.to == fork)
            .map(|e| e.from.clone())
            .collect();
        let joins: HashSet<String> = g
            .edges
            .iter()
            .filter(|e| variants.contains(&e.from))
            .map(|e| e.to.clone())
            .collect();
        let join = joins.iter().next().cloned();
        let kept = variants.first().cloned();
        let mut succs = Vec::new();
        if let Some(j) = &join {
            succs = g
                .edges
                .iter()
                .filter(|e| e.from == *j)
                .map(|e| e.to.clone())
                .collect();
        }
        // 删除网关与多余候选
        g.nodes.retain(|n| {
            n.id != fork
                && join.as_deref() != Some(n.id.as_str())
                && !variants.iter().skip(1).any(|v| v == &n.id)
        });
        g.edges.retain(|e| {
            let bad_from = e.from == fork
                || join.as_deref() == Some(e.from.as_str())
                || variants.iter().skip(1).any(|v| v == &e.from);
            let bad_to = e.to == fork
                || join.as_deref() == Some(e.to.as_str())
                || variants.iter().skip(1).any(|v| v == &e.to);
            !(bad_from || bad_to)
        });
        // 重连：前驱 → 保留候选 → 后继
        if let Some(kept) = kept {
            for p in &preds {
                g.add_edge(FlowEdge::seq(p.clone(), kept.clone()));
            }
            for s in &succs {
                g.add_edge(FlowEdge::seq(kept.clone(), s.clone()));
            }
        }
    }
}

fn total_ms(g: &FlowGraph) -> u64 {
    g.nodes
        .iter()
        .filter_map(|n| n.tool.map(|_| n.duration_ms))
        .sum()
}

/// 沉淀入知识库的拓扑模板（带拓扑荷 Q）
#[derive(Debug, Clone)]
pub struct StoredTopology {
    pub id: String,
    pub signature: String,
    /// 拓扑荷 Q（成功稳定链路才拥有）
    pub charge: f64,
    pub reuse_count: u64,
}

/// 业务知识库：六维关系拓扑网 + 已固化拓扑模板
#[derive(Debug, Clone, Default)]
pub struct KnowledgeBase {
    pub graph: TopologyGraph,
    pub stored: Vec<StoredTopology>,
}

impl KnowledgeBase {
    pub fn new() -> Self {
        Self::default()
    }

    /// 复用压力 ∈ [0,1]：需求子任务在图谱中的最强命中得分
    pub fn reuse_pressure(&self, req: &Requirement) -> f64 {
        let mut best: f64 = 0.0;
        for st in &req.subtasks {
            let s = self
                .graph
                .search(&st.name, 1)
                .into_iter()
                .next()
                .map(|m| m.score)
                .unwrap_or(0.0);
            best = best.max(s);
        }
        best.min(1.0)
    }

    /// 成功交付：注入拓扑荷 Q，固化拓扑模板入图谱，提升复用权重
    pub fn commit_success(&mut self, topo: &CandidateTopology, quality: f64) -> f64 {
        let sig = topo.signature();
        let charge = (0.5 + quality.clamp(0.0, 1.0)).clamp(0.0, 5.0);
        if let Some(existing) = self.stored.iter_mut().find(|s| s.signature == sig) {
            existing.reuse_count += 1;
            existing.charge = (existing.charge + charge * 0.5).min(10.0);
        } else {
            // 把拓扑沉淀进六维关系网：作为 Skill 模板，绑定其流程节点
            let skill_id = format!("skill:{}", topo.graph.id);
            if self.graph.entity(&skill_id).is_none() {
                self.graph.add_entity(
                    Entity::new(&skill_id, EntityKind::Skill, topo.graph.name.clone())
                        .with_keywords([topo.graph.name.clone()]),
                );
            }
            for n in &topo.graph.nodes {
                if n.kind.is_executable() {
                    let fid = format!("flow:{}:{}", topo.graph.id, n.id);
                    if self.graph.entity(&fid).is_none() {
                        self.graph.add_entity(
                            Entity::new(&fid, EntityKind::FlowNode, n.name.clone())
                                .with_keywords([n.name.clone()]),
                        );
                        self.graph.add_relation(Relation::new(
                            &skill_id,
                            &fid,
                            RelationKind::Implements,
                            1.0,
                        ));
                    }
                }
            }
            self.stored.push(StoredTopology {
                id: format!("kb-{}", self.stored.len()),
                signature: sig,
                charge,
                reuse_count: 1,
            });
        }
        charge
    }

    /// 失败/低效：湮灭临时拓扑（不固化），并降低相关实体权重
    pub fn note_failure(&mut self, topo: &CandidateTopology) {
        for n in &topo.graph.nodes {
            let fid = format!("flow:{}:{}", topo.graph.id, n.id);
            if let Some(e) = self.graph.entity_mut(&fid) {
                e.weight *= 0.7;
            }
        }
    }
}

/// 执行反馈结果
#[derive(Debug, Clone)]
pub enum Outcome {
    /// 任务成功 + 质量评分(0..1)
    Success { quality: f64 },
    /// 任务失败/低效 + 严重度(0..1)
    Failure { severity: f64 },
}

/// 成功后的 κ、τ 自调整：巩固复用（κ↑），保留最低探索底线
pub fn adjust_after_success(state: &PrimitiveState, c: f64) -> PrimitiveState {
    let mut kappa = (state.kappa * 1.05).min(c);
    let mut tau = (c * c - kappa * kappa).max(0.0).sqrt();
    // 保留最低探索底线，避免彻底丧失创新能力
    let floor = c * 0.05;
    if tau < floor {
        tau = floor;
        kappa = (c * c - tau * tau).max(0.0).sqrt();
    }
    PrimitiveState {
        c,
        kappa,
        tau,
        q: state.q,
    }
}

/// 失败后的 κ、τ 自调整：提高探索（τ↑），降低该路径复用（κ↓）
pub fn adjust_after_failure(state: &PrimitiveState, c: f64, severity: f64) -> PrimitiveState {
    let sev = severity.clamp(0.0, 1.0);
    let mut tau = (state.tau * (1.0 + 0.3 * sev)).min(c * 0.98);
    let mut kappa = (c * c - tau * tau).max(0.0).sqrt();
    let floor = c * 0.05;
    if kappa < floor {
        kappa = floor;
        tau = (c * c - kappa * kappa).max(0.0).sqrt();
    }
    PrimitiveState {
        c,
        kappa,
        tau,
        q: state.q,
    }
}

/// 自涌现结果状态
#[derive(Debug, Clone, PartialEq)]
pub enum EmergeStatus {
    /// 通过校验（可能经过正则化）
    Validated { regularized: bool },
    /// 多次重试仍无法在预算内满足校验
    Failed,
}

/// κ‑τ 自涌现一次闭环的输出
#[derive(Debug, Clone)]
pub struct EmergenceResult {
    pub topology: CandidateTopology,
    pub state: PrimitiveState,
    pub validation: ValidationReport,
    /// 若成功将固化的拓扑荷预估
    pub charge_estimate: f64,
    pub attempts: usize,
    pub status: EmergeStatus,
}

impl EmergenceResult {
    pub fn summary(&self) -> String {
        let (tag, _reg) = match &self.status {
            EmergeStatus::Validated { regularized } => (
                "通过",
                if *regularized {
                    "（经正则化）"
                } else {
                    ""
                },
            ),
            EmergeStatus::Failed => ("未通过", "（超过最大重试）"),
        };
        format!(
            "κ‑τ 自涌现{}：κ={:.3} τ={:.3} 复用{}项 探索{}项 分叉{} 尝试{}次 预估拓扑荷{:.2}",
            tag,
            self.state.kappa,
            self.state.tau,
            self.topology.reused_subtasks.len(),
            self.topology.explored_subtasks.len(),
            self.topology.fanout,
            self.attempts,
            self.charge_estimate,
        )
    }
}

/// PrimiFlow 闭环引擎
pub struct PrimiEngine {
    pub state: PrimitiveState,
    pub kb: KnowledgeBase,
    pub budget: ResourceBudget,
    pub max_retries: usize,
}

impl PrimiEngine {
    pub fn new(c: f64, kb: KnowledgeBase, budget: ResourceBudget) -> Self {
        Self {
            state: PrimitiveState::new(c),
            kb,
            budget,
            max_retries: 3,
        }
    }

    /// κ‑τ 自涌现主流程：生成 → 校验 → （正则化重试）→ 输出
    ///
    /// `policy` 为 `None` 时沿用引擎内累积状态（闭环节点间状态自动继承）。
    pub fn emerge(&mut self, req: &Requirement, policy: Option<DeliveryPolicy>) -> EmergenceResult {
        let mut working = self.state.clone();
        working.rescale_to_conserve();
        if let Some(p) = policy {
            let target = p.apply(working.c, self.kb.reuse_pressure(req));
            // 向策略目标靠拢，但保留历史累积倾向
            working.kappa = working.kappa + (target.kappa - working.kappa) * 0.7;
            working.tau = (working.c * working.c - working.kappa * working.kappa)
                .max(0.0)
                .sqrt();
            working.rescale_to_conserve();
        }

        let mut attempts = 0;
        let mut last_topo = generate(req, &working, &self.kb);
        let mut last_report = validate(&last_topo, &working, &self.budget);

        while !last_report.ok && attempts < self.max_retries {
            attempts += 1;
            let (rt, rs) = regularize(&last_report, &last_topo, &working, &self.budget);
            last_topo = rt;
            working = rs;
            last_report = validate(&last_topo, &working, &self.budget);
            if last_report.ok {
                break;
            }
            // 仍不通过：加大探索，迫使下一轮生成不同拓扑
            working.tau = (working.tau * 1.15).min(working.c);
            working.kappa = (working.c * working.c - working.tau * working.tau)
                .max(0.0)
                .sqrt();
            last_topo = generate(req, &working, &self.kb);
            last_report = validate(&last_topo, &working, &self.budget);
        }

        let regularized = attempts > 0;
        let status = if last_report.ok {
            EmergeStatus::Validated { regularized }
        } else {
            EmergeStatus::Failed
        };
        let charge_estimate = 0.5
            + last_topo.reused_subtasks.len() as f64 * 0.2
            + last_topo.graph.nodes.len() as f64 * 0.05;

        EmergenceResult {
            topology: last_topo,
            state: working,
            validation: last_report,
            charge_estimate,
            attempts,
            status,
        }
    }

    /// 回灌执行反馈：成功注荷沉淀知识库、巩固复用；失败湮灭并抬高探索
    pub fn accept(&mut self, result: &EmergenceResult, outcome: Outcome) {
        match outcome {
            Outcome::Success { quality } => {
                let q = self.kb.commit_success(&result.topology, quality);
                let mut ns = adjust_after_success(&result.state, self.state.c);
                ns.q = self.state.q + q;
                self.state = ns;
            }
            Outcome::Failure { severity } => {
                self.kb.note_failure(&result.topology);
                self.state = adjust_after_failure(&result.state, self.state.c, severity);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ToolKind;

    fn sample_req() -> Requirement {
        Requirement::new("r1", "电商月度经营分析报告")
            .with_subtask(SubTask::new("fetch", "抓取销售数据", ToolKind::Http, 300))
            .with_subtask(SubTask::new("clean", "清洗对账", ToolKind::Compute, 200))
            .with_subtask(SubTask::new("report", "生成图表报告", ToolKind::Llm, 400))
    }

    fn rich_kb() -> KnowledgeBase {
        let mut kb = KnowledgeBase::new();
        kb.graph.add_entity(
            Entity::new("skill:report", EntityKind::Skill, "电商月度经营分析报告").with_keywords([
                "电商",
                "月度",
                "经营分析",
                "报告",
                "电商月度经营分析报告",
            ]),
        );
        kb.graph.add_entity(
            Entity::new("flow:demo:report", EntityKind::FlowNode, "生成图表报告")
                .with_keywords(["生成图表报告"]),
        );
        kb.graph.add_relation(Relation::new(
            "skill:report",
            "flow:demo:report",
            RelationKind::Implements,
            1.0,
        ));
        kb
    }

    // —— 守恒与策略层 ——

    #[test]
    fn conservation_holds_for_all_policies() {
        for p in [
            DeliveryPolicy::Urgent,
            DeliveryPolicy::Balanced,
            DeliveryPolicy::Exploratory,
        ] {
            let s = PrimitiveState::from_policy(10.0, p, 0.0);
            assert!(
                s.is_conserved(CONSERVATION_EPS),
                "{:?} 残差 {}",
                p,
                s.conservation_residual()
            );
        }
    }

    #[test]
    fn balanced_is_equal_kappa_tau() {
        let s = PrimitiveState::from_policy(10.0, DeliveryPolicy::Balanced, 0.0);
        assert!((s.kappa - s.tau).abs() < 1e-9);
    }

    #[test]
    fn urgent_favors_reuse() {
        let s = PrimitiveState::from_policy(10.0, DeliveryPolicy::Urgent, 0.0);
        assert!(s.reuse_bias() > s.explore_bias(), "紧急交付应复用优先");
    }

    #[test]
    fn exploratory_favors_explore() {
        let s = PrimitiveState::from_policy(10.0, DeliveryPolicy::Exploratory, 0.0);
        assert!(s.explore_bias() > s.reuse_bias(), "探索研发应探索优先");
    }

    #[test]
    fn kb_pressure_nudges_kappa_up() {
        let base = PrimitiveState::from_policy(10.0, DeliveryPolicy::Balanced, 0.0);
        let nudged = PrimitiveState::from_policy(10.0, DeliveryPolicy::Balanced, 0.9);
        assert!(nudged.reuse_bias() > base.reuse_bias(), "复用压力应抬高 κ");
    }

    #[test]
    fn rescale_recovers_conservation() {
        let mut s = PrimitiveState {
            c: 10.0,
            kappa: 6.0,
            tau: 6.0,
            q: 0.0,
        };
        assert!(!s.is_conserved(CONSERVATION_EPS));
        s.rescale_to_conserve();
        assert!(s.is_conserved(CONSERVATION_EPS));
        assert!((s.kappa * s.kappa + s.tau * s.tau - 100.0).abs() < 1e-9);
    }

    // —— 自涌现生成 ——

    #[test]
    fn exploratory_generates_parallel_forks() {
        let kb = KnowledgeBase::new();
        let s = PrimitiveState::from_policy(10.0, DeliveryPolicy::Exploratory, 0.0);
        let topo = generate(&sample_req(), &s, &kb);
        assert!(
            topo.graph
                .nodes
                .iter()
                .any(|n| n.kind == NodeKind::ParallelFork),
            "探索策略应出现并行分叉"
        );
        assert!(topo.fanout > 1, "探索策略分叉度应 > 1");
        assert_eq!(topo.reused_subtasks.len(), 0, "无知识库时不该复用");
    }

    #[test]
    fn rich_kb_triggers_reuse_subflow() {
        let kb = rich_kb();
        let s = PrimitiveState::from_policy(10.0, DeliveryPolicy::Urgent, 0.0);
        let topo = generate(&sample_req(), &s, &kb);
        assert!(
            topo.graph.nodes.iter().any(|n| n.kind == NodeKind::SubFlow),
            "强命中应出现 SubFlow 复用"
        );
        assert!(!topo.reused_subtasks.is_empty(), "应记录复用子任务");
    }

    #[test]
    fn generated_topology_is_acyclic() {
        let kb = KnowledgeBase::new();
        let s = PrimitiveState::from_policy(10.0, DeliveryPolicy::Exploratory, 0.0);
        let topo = generate(&sample_req(), &s, &kb);
        assert!(topo.graph.topo_order().is_ok(), "自涌现拓扑不得有环");
    }

    // —— 校验层 ——

    #[test]
    fn validate_passes_for_reasonable_budget() {
        let kb = KnowledgeBase::new();
        let s = PrimitiveState::from_policy(10.0, DeliveryPolicy::Balanced, 0.0);
        let topo = generate(&sample_req(), &s, &kb);
        let budget = ResourceBudget {
            total_ms: 10_000,
            per_pool: HashMap::new(),
        };
        let rep = validate(&topo, &s, &budget);
        assert!(rep.ok, "充裕预算应通过校验: {:?}", rep.violations);
    }

    #[test]
    fn validate_fails_resource_quota() {
        let kb = KnowledgeBase::new();
        let s = PrimitiveState::from_policy(10.0, DeliveryPolicy::Exploratory, 0.0);
        let topo = generate(&sample_req(), &s, &kb);
        let budget = ResourceBudget {
            total_ms: 100,
            per_pool: HashMap::new(),
        };
        let rep = validate(&topo, &s, &budget);
        assert!(rep.has(ViolationKind::ResourceQuota));
        assert!(!rep.ok);
    }

    #[test]
    fn regularize_fixes_resource_overrun() {
        let kb = KnowledgeBase::new();
        let s = PrimitiveState::from_policy(10.0, DeliveryPolicy::Exploratory, 0.0);
        let topo = generate(&sample_req(), &s, &kb);
        let budget = ResourceBudget {
            total_ms: 100,
            per_pool: HashMap::new(),
        };
        let rep = validate(&topo, &s, &budget);
        let (fixed, _) = regularize(&rep, &topo, &s, &budget);
        let rep2 = validate(&fixed, &s, &budget);
        assert!(rep2.ok, "正则化后应通过校验: {:?}", rep2.violations);
        assert!(
            fixed
                .graph
                .nodes
                .iter()
                .all(|n| n.kind != NodeKind::ParallelFork),
            "正则化应折叠探索分支"
        );
    }

    #[test]
    fn conservation_violation_is_reported() {
        let kb = KnowledgeBase::new();
        let topo = generate(&sample_req(), &PrimitiveState::new(10.0), &kb);
        let broken = PrimitiveState {
            c: 10.0,
            kappa: 9.0,
            tau: 9.0,
            q: 0.0,
        };
        let rep = validate(&topo, &broken, &ResourceBudget::default());
        assert!(rep.has(ViolationKind::Conservation));
    }

    // —— 知识库与闭环 ——

    #[test]
    fn success_commits_charge_and_stores_template() {
        let mut kb = KnowledgeBase::new();
        let topo = generate(
            &sample_req(),
            &PrimitiveState::from_policy(10.0, DeliveryPolicy::Balanced, 0.0),
            &kb,
        );
        let q = kb.commit_success(&topo, 0.9);
        assert!(q > 0.0);
        assert_eq!(kb.stored.len(), 1);
        assert!(kb.stored[0].charge > 0.0);
        // 图谱应沉淀 Skill 模板
        assert!(kb.graph.entity("skill:topo:r1").is_some());
    }

    #[test]
    fn repeated_success_accumulates_charge() {
        let mut kb = KnowledgeBase::new();
        let topo = generate(
            &sample_req(),
            &PrimitiveState::from_policy(10.0, DeliveryPolicy::Balanced, 0.0),
            &kb,
        );
        kb.commit_success(&topo, 0.9);
        let before = kb.stored[0].charge;
        kb.commit_success(&topo, 0.9);
        assert_eq!(kb.stored.len(), 1, "相同签名应去重");
        assert!(kb.stored[0].charge > before, "重复成功应累加拓扑荷");
        assert_eq!(kb.stored[0].reuse_count, 2);
    }

    #[test]
    fn closed_loop_consolidates_after_success() {
        let mut engine = PrimiEngine::new(
            10.0,
            KnowledgeBase::new(),
            ResourceBudget {
                total_ms: 10_000,
                per_pool: HashMap::new(),
            },
        );
        let k0 = engine.state.kappa;
        let res = engine.emerge(&sample_req(), Some(DeliveryPolicy::Urgent));
        assert_eq!(res.status, EmergeStatus::Validated { regularized: false });
        engine.accept(&res, Outcome::Success { quality: 0.95 });
        assert!(engine.state.kappa >= k0, "成功应巩固复用（κ 不降）");
    }

    #[test]
    fn closed_loop_explores_more_after_failure() {
        let mut engine = PrimiEngine::new(
            10.0,
            KnowledgeBase::new(),
            ResourceBudget {
                total_ms: 10_000,
                per_pool: HashMap::new(),
            },
        );
        let res = engine.emerge(&sample_req(), Some(DeliveryPolicy::Balanced));
        let tau_before = engine.state.tau;
        engine.accept(&res, Outcome::Failure { severity: 1.0 });
        assert!(engine.state.tau > tau_before, "失败应抬高探索（τ 上升）");
    }

    #[test]
    fn closed_loop_eventually_passes_under_tight_budget() {
        let mut engine = PrimiEngine::new(
            10.0,
            KnowledgeBase::new(),
            ResourceBudget {
                total_ms: 600,
                per_pool: HashMap::new(),
            },
        );
        let res = engine.emerge(&sample_req(), Some(DeliveryPolicy::Exploratory));
        assert_eq!(
            res.status,
            EmergeStatus::Validated { regularized: true },
            "紧预算下应经正则化通过"
        );
    }

    #[test]
    fn end_to_end_builds_knowledge_across_runs() {
        let mut engine = PrimiEngine::new(
            10.0,
            KnowledgeBase::new(),
            ResourceBudget {
                total_ms: 10_000,
                per_pool: HashMap::new(),
            },
        );
        // 第一轮探索研发
        let r1 = engine.emerge(&sample_req(), Some(DeliveryPolicy::Exploratory));
        engine.accept(&r1, Outcome::Success { quality: 0.85 });
        // 第二轮紧急交付，应命中首轮沉淀的知识库 → 复用
        let r2 = engine.emerge(&sample_req(), Some(DeliveryPolicy::Urgent));
        engine.accept(&r2, Outcome::Success { quality: 0.9 });
        assert!(!engine.kb.stored.is_empty(), "知识库应已沉淀拓扑模板");
        assert!(
            r2.topology.reused_subtasks.len() >= r1.topology.reused_subtasks.len(),
            "复用量应不降"
        );
    }
}

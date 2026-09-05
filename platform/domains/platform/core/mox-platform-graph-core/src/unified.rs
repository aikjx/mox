// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 归一化统一图模型（多维度融合 · 一体化底座）
//!
//! 把两份企业级规范熔铸为**一张**图：
//! - **GR-STD 关图规范**：「一切皆是信息」——12 类节点 / 7 类边；
//! - **PT-Primi 架构规范**：七层架构(L1-L7) + 六维绑定(REQ→FUN→BIZ→ALG→TSK→COD)
//!   + κ‑τ 守恒(C²=κ²+τ²) + `PTEnvelope` 跨层消息。
//!
//! 归一化思路：任意信息实体都被投影到三个正交维度——
//! 1. `Layer`    —— 它处在 PT-Primi 七层架构的哪一层；
//! 2. `EntityKind`——它的归一化实体类型（GR-STD 12 类 ∪ PT 六维 ∪ 数据落地 2 类 ∪ 拓扑 2 类）；
//! 3. `PrimitiveCoords`——它在 κ‑τ 几何空间的坐标 (κ,τ,C,Q)。
//!
//! 三者 + 六维绑定 ID 共同构成「归一化信息实体」，从而所有功能可融、可校验、可溯源。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// 守恒残差允许的浮点误差（对齐 PT-Primi §3.3 默认 1e-3 / flow-ai 内部 1e-6，
/// 此处用较宽松的全局图级阈值，便于跨大图聚合判定）
pub const GLOBAL_CONSERVATION_EPS: f64 = 1e-3;

/// 平台级守恒配额上界（资源准入，PT-Primi §11.1 规模膨胀缓解）
pub const PLATFORM_C_QUOTA: f64 = 1_000.0;

// ───────────────────────── 维度 A：架构层 ─────────────────────────

/// PT-Primi §4 七层技术架构（L1-L7）。治理层 L7 横切全部层。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    /// L1 需求语义层
    RequirementSemantic,
    /// L2 原语映射层
    PrimitiveMapping,
    /// L3 拓扑涌现层
    TopologyEmergence,
    /// L4 调度编排层
    Orchestration,
    /// L5 执行运行时层
    ExecutionRuntime,
    /// L6 资产沉淀层
    AssetPrecipitation,
    /// L7 治理合规层（横切）
    Governance,
}

impl Layer {
    pub fn code(&self) -> &'static str {
        match self {
            Layer::RequirementSemantic => "L1",
            Layer::PrimitiveMapping => "L2",
            Layer::TopologyEmergence => "L3",
            Layer::Orchestration => "L4",
            Layer::ExecutionRuntime => "L5",
            Layer::AssetPrecipitation => "L6",
            Layer::Governance => "L7",
        }
    }
    pub fn zh(&self) -> &'static str {
        match self {
            Layer::RequirementSemantic => "需求语义层",
            Layer::PrimitiveMapping => "原语映射层",
            Layer::TopologyEmergence => "拓扑涌现层",
            Layer::Orchestration => "调度编排层",
            Layer::ExecutionRuntime => "执行运行时层",
            Layer::AssetPrecipitation => "资产沉淀层",
            Layer::Governance => "治理合规层",
        }
    }
}

// ───────────────────── 维度 B：归一化实体类型 ─────────────────────

/// 归一化实体类型：GR-STD 12 类 ∪ PT 六维 ∪ assoc 数据落地 2 类 ∪ 拓扑节点 2 类。
/// 这是「一切皆是信息」在 PrimiFlow 内的统一分类法。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    // —— PT-Primi 六维绑定实体 ——
    Requirement,
    Feature,
    Business,
    Algorithm,
    Task,
    Code,
    // —— GR-STD 信息实体（除已并入上/下的类）——
    Data,
    Function,
    Interface,
    Script,
    ScheduleTask,
    Config,
    Dependency,
    ThirdParty,
    Doc,
    Runtime,
    // —— 数据落地（primiflow assoc 八层后两层）——
    DataSchema,
    DataStore,
    // —— 拓扑节点（PT-Primi §6 画布实体）——
    Loop,
    Graph,
}

impl EntityKind {
    pub fn zh(&self) -> &'static str {
        match self {
            EntityKind::Requirement => "需求",
            EntityKind::Feature => "功能",
            EntityKind::Business => "业务流程",
            EntityKind::Algorithm => "算法",
            EntityKind::Task => "任务",
            EntityKind::Code => "代码",
            EntityKind::Data => "数据",
            EntityKind::Function => "函数",
            EntityKind::Interface => "接口",
            EntityKind::Script => "脚本",
            EntityKind::ScheduleTask => "定时任务",
            EntityKind::Config => "配置",
            EntityKind::Dependency => "依赖库",
            EntityKind::ThirdParty => "第三方服务",
            EntityKind::Doc => "文档",
            EntityKind::Runtime => "运行时",
            EntityKind::DataSchema => "数据设计",
            EntityKind::DataStore => "数据存储",
            EntityKind::Loop => "闭环单元",
            EntityKind::Graph => "分支/Agent",
        }
    }

    /// Mermaid classDef 类名
    pub fn class(&self) -> &'static str {
        match self {
            EntityKind::Requirement => "req",
            EntityKind::Feature => "feat",
            EntityKind::Business => "biz",
            EntityKind::Algorithm => "algo",
            EntityKind::Task => "task",
            EntityKind::Code => "code",
            EntityKind::Data => "data",
            EntityKind::Function => "func",
            EntityKind::Interface => "iface",
            EntityKind::Script => "script",
            EntityKind::ScheduleTask => "sched",
            EntityKind::Config => "cfg",
            EntityKind::Dependency => "dep",
            EntityKind::ThirdParty => "third",
            EntityKind::Doc => "doc",
            EntityKind::Runtime => "rt",
            EntityKind::DataSchema => "ds",
            EntityKind::DataStore => "store",
            EntityKind::Loop => "loop",
            EntityKind::Graph => "graph",
        }
    }

    /// 是否属于 PT-Primi 六维绑定实体（A4 强制零孤儿）
    pub fn is_six_dim(&self) -> bool {
        matches!(
            self,
            EntityKind::Requirement
                | EntityKind::Feature
                | EntityKind::Business
                | EntityKind::Algorithm
                | EntityKind::Task
                | EntityKind::Code
        )
    }

    /// 是否为核心代码/数据/接口类（GR-STD 孤儿判定对象）
    pub fn is_core(&self) -> bool {
        matches!(
            self,
            EntityKind::Code
                | EntityKind::Function
                | EntityKind::Interface
                | EntityKind::Data
                | EntityKind::Algorithm
                | EntityKind::Task
                | EntityKind::Runtime
        )
    }
}

// ──────────────── 维度 C：原语坐标（κ,τ,C,Q）────────────────

/// 原语坐标：任意实体在 κ‑τ 几何空间的位置（PT-Primi §3.2）。
/// `Q` 为拓扑荷，成功链路带荷固化，失败链路不带荷湮灭。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PrimitiveCoords {
    pub kappa: f64,
    pub tau: f64,
    pub c: f64,
    pub q: f64,
}

impl PrimitiveCoords {
    pub fn zero() -> Self {
        Self {
            kappa: 0.0,
            tau: 0.0,
            c: 0.0,
            q: 0.0,
        }
    }
    /// 由 (κ,τ) 推出守恒总量 C = √(κ²+τ²)
    pub fn from_kt(kappa: f64, tau: f64) -> Self {
        let c = (kappa * kappa + tau * tau).sqrt();
        Self {
            kappa,
            tau,
            c,
            q: 0.0,
        }
    }
    /// 守恒残差 ε = |C − √(κ²+τ²)|（PT-Primi §3.1 A3）
    pub fn residual(&self) -> f64 {
        let implied = (self.kappa * self.kappa + self.tau * self.tau).sqrt();
        (self.c - implied).abs()
    }
    pub fn is_conserved(&self, eps: f64) -> bool {
        self.residual() <= eps
    }
}

// ─────────────────────── 边（融合关系类型）───────────────────────

/// 边类型：GR-STD 7 类 ∪ PT-Primi 5 类（绑定/数据/拓扑/触发）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelKind {
    // —— PT-Primi 跨层/拓扑关系 ——
    Bind,     // 六维一一绑定（最强约束）
    DataFlow, // 数据依赖
    LoopBack, // 闭环回流（κ）
    Branch,   // 分支/并行汇聚（τ）
    Trigger,  // 定时/事件触发
    // —— GR-STD 信息关系 ——
    Call,
    ReadWrite,
    Reference,
    Dependency,
    Inheritance,
    ConfigRef,
    Deploy,
}

impl RelKind {
    pub fn zh(&self) -> &'static str {
        match self {
            RelKind::Bind => "六维绑定",
            RelKind::DataFlow => "数据流",
            RelKind::LoopBack => "闭环回流",
            RelKind::Branch => "分支汇聚",
            RelKind::Trigger => "触发",
            RelKind::Call => "调用",
            RelKind::ReadWrite => "读写",
            RelKind::Reference => "引用",
            RelKind::Dependency => "依赖",
            RelKind::Inheritance => "继承/实现",
            RelKind::ConfigRef => "配置引用",
            RelKind::Deploy => "部署/承载",
        }
    }
    /// 是否计入六维绑定溯源链
    pub fn is_binding(&self) -> bool {
        matches!(self, RelKind::Bind)
    }
}

/// 归一化关联边（GR-STD §3.1：每条边必须带 evidence，否则视为未证实关系）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: RelKind,
    pub label: String,
    pub evidence: String,
}

/// 归一化信息实体（三维度 + 六维绑定 ID + 证据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedNode {
    pub id: String,
    pub kind: EntityKind,
    pub layer: Layer,
    pub name: String,
    pub path: String,
    pub summary: String,
    /// 可定位出处（GR-STD 强制）
    pub evidence: String,
    /// 原语坐标（κ,τ,C,Q）
    pub primitive: PrimitiveCoords,
    /// 六维绑定 ID（REQ-/FUN-/BIZ-/ALG-/TSK-/COD-）
    pub bind_id: Option<String>,
    /// 外部不可解析依赖（建模为 ThirdParty 时置 true）
    pub external: bool,
}

// ─────────────────────── 统一图 + 治理闸门 ────────────────────────

/// 多维度融合统一图：平台唯一事实源（Single Source of Truth）。
#[derive(Debug, Clone, Default)]
pub struct UnifiedGraph {
    pub nodes: HashMap<String, UnifiedNode>,
    pub edges: Vec<UnifiedEdge>,
    /// 重复节点 id 集合：add_node 插入已存在的 id 时记录（G3 重复 id 检测源）。
    /// 因 nodes 为 HashMap 会静默覆盖，必须单列记录才能被治理闸门识别。
    pub node_dups: HashSet<String>,
}

impl UnifiedGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, n: UnifiedNode) {
        if self.nodes.contains_key(&n.id) {
            self.node_dups.insert(n.id.clone());
        }
        self.nodes.insert(n.id.clone(), n);
    }

    pub fn add_edge(&mut self, e: UnifiedEdge) {
        self.edges.push(e);
    }

    pub fn node(&self, id: &str) -> Option<&UnifiedNode> {
        self.nodes.get(id)
    }

    pub fn edge_ends(&self, from: &str, to: &str) -> Vec<&UnifiedEdge> {
        self.edges
            .iter()
            .filter(|e| e.from == from && e.to == to)
            .collect()
    }

    /// 沿给定边类型做上游 BFS，返回全部可达祖先 id（含起点）
    pub fn upstream_ids(&self, start: &str, kinds: &[RelKind]) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut q = VecDeque::new();
        seen.insert(start.to_string());
        q.push_back(start.to_string());
        while let Some(cur) = q.pop_front() {
            for e in &self.edges {
                if e.to == cur && kinds.contains(&e.kind) && !seen.contains(&e.from) {
                    seen.insert(e.from.clone());
                    q.push_back(e.from.clone());
                }
            }
        }
        seen.into_iter().collect()
    }

    /// 沿给定边类型做下游 BFS
    pub fn downstream_ids(&self, start: &str, kinds: &[RelKind]) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut q = VecDeque::new();
        seen.insert(start.to_string());
        q.push_back(start.to_string());
        while let Some(cur) = q.pop_front() {
            for e in &self.edges {
                if e.from == cur && kinds.contains(&e.kind) && !seen.contains(&e.to) {
                    seen.insert(e.to.clone());
                    q.push_back(e.to.clone());
                }
            }
        }
        seen.into_iter().collect()
    }

    /// 无环判定（拓扑排序）
    pub fn is_acyclic(&self) -> bool {
        let mut indeg: HashMap<&str, usize> = self.nodes.keys().map(|k| (k.as_str(), 0)).collect();
        for e in &self.edges {
            if self.nodes.contains_key(&e.from) && self.nodes.contains_key(&e.to) {
                *indeg.get_mut(e.to.as_str()).unwrap() += 1;
            }
        }
        let mut q: VecDeque<&str> = indeg
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(k, _)| *k)
            .collect();
        let mut visited = 0;
        while let Some(n) = q.pop_front() {
            visited += 1;
            for e in &self.edges {
                if e.from == n && self.nodes.contains_key(&e.to) {
                    let d = indeg.get_mut(e.to.as_str()).unwrap();
                    *d -= 1;
                    if *d == 0 {
                        q.push_back(e.to.as_str());
                    }
                }
            }
        }
        visited == self.nodes.len()
    }

    // —— R07：守恒残差全局闸门（PT-Primi §3.1 A1/A3，规范缺口 R07）——
    // 【大白话】"账平不平"检查：每个节点都自己报了一个守恒量 C，以及它的两个分量 κ、τ。
    // 法律规定它们必须满足 C² = κ² + τ²。这道闸门就验算"你报的 C 和 κ/τ 算出来的是否对得上"。
    // 注意：C 是节点自报的，所以这只查"内部自洽"，查不出"整个图凭空编造了一套管账"。
    pub fn conservation_report(&self) -> ConservationReport {
        let mut per_node_violations = Vec::new();
        let mut sum_k = 0.0;
        let mut sum_t = 0.0;
        for n in self.nodes.values() {
            if n.primitive.c > 0.0 {
                sum_k += n.primitive.kappa;
                sum_t += n.primitive.tau;
                if !n.primitive.is_conserved(GLOBAL_CONSERVATION_EPS) {
                    per_node_violations.push(format!(
                        "节点 {} 守恒残差 ε={:.2e} 超阈（C={:.3}, κ={:.3}, τ={:.3}）",
                        n.id,
                        n.primitive.residual(),
                        n.primitive.c,
                        n.primitive.kappa,
                        n.primitive.tau
                    ));
                }
            }
        }

        // 每个需求根（Requirement）下游 Loop/Graph/Algorithm/Task 的 κ/τ 之和应等于其 C
        let mut topology_violations = Vec::new();
        let bind_like = [RelKind::Bind, RelKind::Reference, RelKind::DataFlow];
        for r in self.nodes.values() {
            if r.kind == EntityKind::Requirement && r.primitive.c > 0.0 {
                let down = self.downstream_ids(&r.id, &bind_like);
                let mut k = 0.0;
                let mut t = 0.0;
                let mut has_topo = false;
                for id in &down {
                    if let Some(n) = self.nodes.get(id) {
                        if matches!(
                            n.kind,
                            EntityKind::Loop
                                | EntityKind::Graph
                                | EntityKind::Algorithm
                                | EntityKind::Task
                        ) {
                            k += n.primitive.kappa;
                            t += n.primitive.tau;
                            has_topo = true;
                        }
                    }
                }
                // 仅当确实涌现了下游拓扑节点才校验（否则需求尚未展开，不构成违约）
                if has_topo {
                    let implied = (k * k + t * t).sqrt();
                    let res = (r.primitive.c - implied).abs();
                    if res > GLOBAL_CONSERVATION_EPS {
                        topology_violations.push(format!(
                            "需求 {} 下游拓扑 C 残差 ε={:.2e}（声明 C={:.3}, 实际 √(Σκ²+Στ²)={:.3}）",
                            r.id, res, r.primitive.c, implied
                        ));
                    }
                }
            }
        }

        let total_c = (sum_k * sum_k + sum_t * sum_t).sqrt();
        let over_quota = total_c > PLATFORM_C_QUOTA;

        let mut errors = per_node_violations;
        errors.extend(topology_violations);
        let warnings = if over_quota {
            vec![format!(
                "平台总守恒量 C={total_c:.1} 超出配额 {PLATFORM_C_QUOTA}",
            )]
        } else {
            vec![]
        };
        let passed = errors.is_empty();
        ConservationReport {
            errors,
            warnings,
            total_c,
            passed,
        }
    }

    // —— A4：六维绑定零孤儿（REQ→FUN→BIZ→ALG→TSK→COD 逐级非空）——
    // 【大白话】"不能只有一个环节、前面的环节断链"检查：一条需求(REQ)应该有功能(FUN)、
    // 功能对应业务(BIZ)、业务对应算法(ALG)、算法拆成任务(TSK)、任务落到代码(COD)。
    // 如果某个环节填了绑定 ID，却找不到上游环节，就成了"孤儿"——闸门会拦下。
    // 没填绑定 ID 的能力/资产节点算基础设施，不查（可被借机绕过，属已知薄弱点）。
    pub fn binding_report(&self) -> BindingReport {
        let bind_kinds = [RelKind::Bind];
        let upstream_of = |id: &str, want: EntityKind| -> bool {
            self.upstream_ids(id, &bind_kinds)
                .iter()
                .any(|u| self.nodes.get(u).map(|n| n.kind == want).unwrap_or(false))
        };

        let mut orphans: Vec<String> = Vec::new();
        for n in self.nodes.values() {
            // A4 零孤儿仅约束「声明了六维绑定 ID」的需求驱动实体；
            // 经 Reference/Deploy 挂接的能力/资产节点（无 bind_id）属基础设施，豁免。
            if !n.kind.is_six_dim() || n.bind_id.is_none() {
                continue;
            }
            let ok = match n.kind {
                EntityKind::Feature => upstream_of(&n.id, EntityKind::Requirement),
                EntityKind::Business => upstream_of(&n.id, EntityKind::Feature),
                EntityKind::Algorithm => upstream_of(&n.id, EntityKind::Business),
                EntityKind::Task => upstream_of(&n.id, EntityKind::Algorithm),
                EntityKind::Code => {
                    upstream_of(&n.id, EntityKind::Algorithm)
                        || upstream_of(&n.id, EntityKind::Task)
                }
                _ => true,
            };
            if !ok {
                orphans.push(format!("{} 维度孤儿：{}", n.kind.zh(), n.id));
            }
        }
        let counts = self
            .nodes
            .values()
            .filter(|n| n.kind.is_six_dim() && n.bind_id.is_some())
            .count();
        let passed = orphans.is_empty();
        BindingReport {
            orphans,
            six_dim_nodes: counts,
            passed,
        }
    }

    // —— GR-STD §9.4 强制合规清单（CI 门禁 8 项）——
    // 【大白话】"这图是不是一张合法、能交差的图"的基础体检：图非空、无悬空边、无重复
    // id、边带证据、核心无孤儿、无隐性依赖、文档引用有效、sync 漂移=0。
    // 其中 G8（sync 漂移）属跨快照比对，由 `full_gate_with_baseline` 注入 GovernanceReport；
    // 本函数负责 G1–G7 七项图内检查。
    pub fn governance_report(&self) -> GovernanceReport {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // G1：图非空（纳入版本管理 / 已生成）
        if self.nodes.is_empty() {
            errors.push("G1 图为空（未纳入版本管理或未生成）".into());
        }
        // G2：无悬空边（GR-E2）
        let dangling: Vec<_> = self
            .edges
            .iter()
            .filter(|e| !self.nodes.contains_key(&e.from) || !self.nodes.contains_key(&e.to))
            .map(|e| e.id.clone())
            .collect();
        if !dangling.is_empty() {
            errors.push(format!("G2 悬空边 {} 条（GR-E2）", dangling.len()));
        }
        // G3：无重复 id（GR-E4）——同 id 多定义属致命
        let mut node_seen: HashMap<&str, usize> = HashMap::new();
        for id in self.nodes.keys() {
            *node_seen.entry(id.as_str()).or_insert(0) += 1;
        }
        let dup_nodes: Vec<String> = node_seen
            .into_iter()
            .filter(|(_, c)| *c > 1)
            .map(|(id, _)| id.to_string())
            .collect();
        let mut edge_seen: HashMap<&str, usize> = HashMap::new();
        for e in &self.edges {
            *edge_seen.entry(e.id.as_str()).or_insert(0) += 1;
        }
        let dup_edges: Vec<String> = edge_seen
            .into_iter()
            .filter(|(_, c)| *c > 1)
            .map(|(id, _)| id.to_string())
            .collect();
        if !dup_nodes.is_empty() || !self.node_dups.is_empty() || !dup_edges.is_empty() {
            let dup_node_count = dup_nodes.len() + self.node_dups.len();
            errors.push(format!(
                "G3 重复 id {} 个（节点 {} / 边 {}）（GR-E4）",
                dup_node_count + dup_edges.len(),
                dup_node_count,
                dup_edges.len()
            ));
        }
        // G4：所有边带 evidence（GR-E3）
        let no_ev: usize = self
            .edges
            .iter()
            .filter(|e| e.evidence.trim().is_empty())
            .count();
        if no_ev > 0 {
            errors.push(format!("G4 缺 evidence 边 {} 条（GR-E3）", no_ev));
        }
        // G5：核心节点无孤儿（GR-E1）
        let orphan_core: Vec<_> = self
            .nodes
            .values()
            .filter(|n| n.kind.is_core() && !n.external)
            .filter(|n| !self.edges.iter().any(|e| e.from == n.id || e.to == n.id))
            .map(|n| n.id.clone())
            .collect();
        if !orphan_core.is_empty() {
            errors.push(format!("G5 核心孤儿节点 {} 个（GR-E1）", orphan_core.len()));
        }
        // G6：无未建模的隐性依赖（GR-E6）——外部依赖必须归类为 ThirdParty / Dependency
        let implicit: Vec<_> = self
            .nodes
            .values()
            .filter(|n| {
                n.external && !matches!(n.kind, EntityKind::ThirdParty | EntityKind::Dependency)
            })
            .map(|n| n.id.clone())
            .collect();
        if !implicit.is_empty() {
            errors.push(format!(
                "G6 未建模隐性依赖 {} 个（需归类 ThirdParty/Dependency）（GR-E6）",
                implicit.len()
            ));
        }
        // G7：文档引用全部有效（GR-E7）
        let mut doc_broken: Vec<String> = Vec::new();
        let mut doc_isolated: Vec<String> = Vec::new();
        for n in self.nodes.values().filter(|n| n.kind == EntityKind::Doc) {
            let refs: Vec<&UnifiedEdge> = self
                .edges
                .iter()
                .filter(|e| {
                    e.from == n.id
                        && matches!(
                            e.kind,
                            RelKind::Reference | RelKind::ConfigRef | RelKind::ReadWrite
                        )
                })
                .collect();
            if refs.is_empty() {
                doc_isolated.push(n.id.clone());
            } else {
                for e in refs {
                    if !self.nodes.contains_key(&e.to) {
                        doc_broken.push(format!("{}→{}", n.id, e.to));
                    }
                }
            }
        }
        if !doc_broken.is_empty() {
            errors.push(format!("G7 文档失效引用 {} 处（GR-E7）", doc_broken.len()));
        }
        if !doc_isolated.is_empty() {
            warnings.push(format!(
                "G7 文档孤岛 {} 篇（疑似未关联接口/数据）",
                doc_isolated.len()
            ));
        }

        let passed = errors.is_empty();
        GovernanceReport {
            errors,
            warnings,
            passed,
        }
    }

    /// 一次性聚合：守恒 + 绑定 + 治理（平台级发布闸门，无基线时不评估 sync）
    /// 【大白话】CI 里跑的"总闸门"：把上面几道检查(账平不平 / 环节断不断链 / 图合不合法)
    /// 一起跑，任何一道不通过，整张图就不准发布。等价于发布前的"多证合一"体检。
    pub fn full_gate(&self) -> PlatformGate {
        let conservation = self.conservation_report();
        let binding = self.binding_report();
        let governance = self.governance_report();
        let sync = SyncReport::none();
        let mut all_errors = conservation.errors.clone();
        all_errors.extend(binding.orphans.iter().cloned());
        all_errors.extend(governance.errors.clone());
        let passed = conservation.passed && binding.passed && governance.passed && sync.passed;
        PlatformGate {
            conservation,
            binding,
            governance,
            sync,
            passed,
            error_count: all_errors.len(),
        }
    }

    /// 带基线的平台级发布闸门（GR-E8 sync 漂移门禁生效）。
    /// 任何未授权删除（节点/边）都会令 G8 失败并阻断发布。
    pub fn full_gate_with_baseline(&self, baseline: &UnifiedGraph) -> PlatformGate {
        let conservation = self.conservation_report();
        let binding = self.binding_report();
        let mut governance = self.governance_report();
        let sync = self.sync_report(baseline);
        if !sync.passed {
            governance.errors.push(format!(
                "G8 sync 漂移 {} 处未授权删除（节点 {} / 边 {}）（GR-E8）",
                sync.drift,
                sync.removed_nodes.len(),
                sync.removed_edges.len()
            ));
            governance.passed = false;
        }
        let mut all_errors = conservation.errors.clone();
        all_errors.extend(binding.orphans.iter().cloned());
        all_errors.extend(governance.errors.clone());
        let passed = conservation.passed && binding.passed && governance.passed && sync.passed;
        PlatformGate {
            conservation,
            binding,
            governance,
            sync,
            passed,
            error_count: all_errors.len(),
        }
    }

    /// GR-E8 同步漂移比对：baseline 中存在但 self 中缺失的节点/边视为未授权删除。
    /// 漂移量 = 删除节点数 + 删除边数；正式发布门禁要求 drift == 0。
    pub fn sync_report(&self, baseline: &UnifiedGraph) -> SyncReport {
        let removed_nodes: Vec<String> = baseline
            .nodes
            .keys()
            .filter(|id| !self.nodes.contains_key(*id))
            .cloned()
            .collect();
        let removed_edges: Vec<String> = baseline
            .edges
            .iter()
            .filter(|e| !self.edges.iter().any(|x| x.id == e.id))
            .map(|e| e.id.clone())
            .collect();
        let added_nodes = self
            .nodes
            .keys()
            .filter(|id| !baseline.nodes.contains_key(*id))
            .count();
        let added_edges = self
            .edges
            .iter()
            .filter(|e| !baseline.edges.iter().any(|x| x.id == e.id))
            .count();
        let drift = removed_nodes.len() + removed_edges.len();
        SyncReport {
            baseline_nodes: baseline.nodes.len(),
            baseline_edges: baseline.edges.len(),
            added_nodes,
            added_edges,
            removed_nodes,
            removed_edges,
            drift,
            evaluated: true,
            passed: drift == 0,
        }
    }

    /// 六维溯源链（从任意 Code 节点沿 Bind 边回溯到 REQ）
    pub fn trace_binding(&self, code_id: &str) -> Vec<String> {
        let bind_kinds = [RelKind::Bind];
        let order = [
            EntityKind::Requirement,
            EntityKind::Feature,
            EntityKind::Business,
            EntityKind::Algorithm,
            EntityKind::Task,
            EntityKind::Code,
        ];
        let ups = self.upstream_ids(code_id, &bind_kinds);
        let mut chain = vec![code_id.to_string()];
        for k in order.iter().rev().skip(1) {
            if let Some(id) = ups
                .iter()
                .find(|id| self.nodes.get(*id).map(|n| n.kind == *k).unwrap_or(false))
            {
                chain.push(id.clone());
            }
        }
        chain.into_iter().rev().collect()
    }

    /// 导出 Mermaid（按实体类型着色，常驻显示守恒恒等式）
    pub fn to_mermaid(&self) -> String {
        let mut s = String::from("graph TD\n");
        for n in self.nodes.values() {
            let prim = if n.primitive.c > 0.0 {
                format!(" κ={:.2} τ={:.2}", n.primitive.kappa, n.primitive.tau)
            } else {
                String::new()
            };
            s.push_str(&format!(
                "  {}[\"{}({}): {}{}\"]:::{}\n",
                n.id,
                n.kind.zh(),
                n.layer.code(),
                n.name,
                prim,
                n.kind.class()
            ));
        }
        for e in &self.edges {
            s.push_str(&format!("  {} -->|{}| {}\n", e.from, e.kind.zh(), e.to));
        }
        // classDef
        let defs = [
            ("req", "#3b82f6"),
            ("feat", "#06b6d4"),
            ("biz", "#22c55e"),
            ("algo", "#f59e0b"),
            ("task", "#eab308"),
            ("code", "#9ca3af"),
            ("data", "#14b8a6"),
            ("func", "#84cc16"),
            ("iface", "#a855f7"),
            ("script", "#64748b"),
            ("sched", "#facc15"),
            ("cfg", "#78716c"),
            ("dep", "#475569"),
            ("third", "#ef4444"),
            ("doc", "#0ea5e9"),
            ("rt", "#ec4899"),
            ("ds", "#0d9488"),
            ("store", "#334155"),
            ("loop", "#fb923c"),
            ("graph", "#a855f7"),
        ];
        for (c, color) in defs {
            s.push_str(&format!("  classDef {c} fill:{color},color:#fff\n"));
        }
        s
    }
}

// ─────────────────────── 报告结构体 ────────────────────────

#[derive(Debug, Clone)]
pub struct ConservationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub total_c: f64,
    pub passed: bool,
}

#[derive(Debug, Clone)]
pub struct BindingReport {
    pub orphans: Vec<String>,
    pub six_dim_nodes: usize,
    pub passed: bool,
}

#[derive(Debug, Clone)]
pub struct GovernanceReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub passed: bool,
}

/// GR-E8 同步漂移报告：比对 baseline/new 两张快照，未声明的删除即漂移。
#[derive(Debug, Clone)]
pub struct SyncReport {
    pub baseline_nodes: usize,
    pub baseline_edges: usize,
    pub added_nodes: usize,
    pub added_edges: usize,
    pub removed_nodes: Vec<String>,
    pub removed_edges: Vec<String>,
    /// 漂移量 = 未授权删除总数（节点 + 边），GR-E8 要求 = 0
    pub drift: usize,
    /// 是否提供了基线快照；未提供视为未评估，不阻断发布
    pub evaluated: bool,
    pub passed: bool,
}

impl SyncReport {
    /// 未提供基线时的占位（不评估、不阻断）
    pub fn none() -> Self {
        Self {
            baseline_nodes: 0,
            baseline_edges: 0,
            added_nodes: 0,
            added_edges: 0,
            removed_nodes: Vec::new(),
            removed_edges: Vec::new(),
            drift: 0,
            evaluated: false,
            passed: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlatformGate {
    pub conservation: ConservationReport,
    pub binding: BindingReport,
    pub governance: GovernanceReport,
    /// GR-E8 同步漂移报告（仅带基线比对时评估）
    pub sync: SyncReport,
    pub passed: bool,
    pub error_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(id: &str, kind: EntityKind, layer: Layer, k: f64, t: f64, c: f64) -> UnifiedNode {
        n_bind(id, kind, layer, k, t, c, None)
    }

    fn n_bind(
        id: &str,
        kind: EntityKind,
        layer: Layer,
        k: f64,
        t: f64,
        c: f64,
        bind: Option<&str>,
    ) -> UnifiedNode {
        UnifiedNode {
            id: id.into(),
            kind,
            layer,
            name: id.into(),
            path: String::new(),
            summary: String::new(),
            evidence: "test".into(),
            primitive: PrimitiveCoords {
                kappa: k,
                tau: t,
                c,
                q: 0.0,
            },
            bind_id: bind.map(|s| s.into()),
            external: false,
        }
    }

    fn e(from: &str, to: &str, kind: RelKind) -> UnifiedEdge {
        UnifiedEdge {
            id: format!("{from}->{to}:{kind:?}"),
            from: from.into(),
            to: to.into(),
            kind,
            label: kind.zh().into(),
            evidence: "test".into(),
        }
    }

    #[test]
    fn conservation_gate_catches_residual() {
        let mut g = UnifiedGraph::new();
        // 声明 C=5 但 κ²+τ²=3²+4²=25 → C=5 守恒 ✓
        g.add_node(n(
            "R1",
            EntityKind::Requirement,
            Layer::RequirementSemantic,
            3.0,
            4.0,
            5.0,
        ));
        // 声明 C=5 但 κ²+τ²=1 → 残差大 ✗
        g.add_node(n(
            "R2",
            EntityKind::Requirement,
            Layer::RequirementSemantic,
            1.0,
            0.0,
            5.0,
        ));
        let rep = g.conservation_report();
        assert!(!rep.passed, "R2 应被判残差超限");
        assert_eq!(rep.errors.len(), 1);
    }

    #[test]
    fn binding_zero_orphan_ok() {
        let mut g = UnifiedGraph::new();
        g.add_node(n_bind(
            "REQ1",
            EntityKind::Requirement,
            Layer::RequirementSemantic,
            0.0,
            0.0,
            0.0,
            Some("REQ-1"),
        ));
        g.add_node(n_bind(
            "FUN1",
            EntityKind::Feature,
            Layer::PrimitiveMapping,
            0.0,
            0.0,
            0.0,
            Some("FUN-1"),
        ));
        g.add_node(n_bind(
            "BIZ1",
            EntityKind::Business,
            Layer::TopologyEmergence,
            0.0,
            0.0,
            0.0,
            Some("BIZ-1"),
        ));
        g.add_node(n_bind(
            "ALG1",
            EntityKind::Algorithm,
            Layer::TopologyEmergence,
            0.0,
            0.0,
            0.0,
            Some("ALG-1"),
        ));
        g.add_node(n_bind(
            "TSK1",
            EntityKind::Task,
            Layer::Orchestration,
            0.0,
            0.0,
            0.0,
            Some("TSK-1"),
        ));
        g.add_node(n_bind(
            "COD1",
            EntityKind::Code,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
            Some("COD-1"),
        ));
        for (a, b) in [
            ("REQ1", "FUN1"),
            ("FUN1", "BIZ1"),
            ("BIZ1", "ALG1"),
            ("ALG1", "TSK1"),
            ("ALG1", "COD1"),
        ] {
            g.add_edge(e(a, b, RelKind::Bind));
        }
        let rep = g.binding_report();
        assert!(rep.passed, "六维应零孤儿，实际：{:?}", rep.orphans);
        assert_eq!(rep.six_dim_nodes, 6);
    }

    #[test]
    fn binding_detects_orphan_feature() {
        let mut g = UnifiedGraph::new();
        g.add_node(n_bind(
            "REQ1",
            EntityKind::Requirement,
            Layer::RequirementSemantic,
            0.0,
            0.0,
            0.0,
            Some("REQ-1"),
        ));
        // FUN1 声明了绑定 ID 但无上游 REQ → 孤儿
        g.add_node(n_bind(
            "FUN1",
            EntityKind::Feature,
            Layer::PrimitiveMapping,
            0.0,
            0.0,
            0.0,
            Some("FUN-1"),
        ));
        assert!(!g.binding_report().passed);
    }

    #[test]
    fn governance_gates_dangling_and_evidence() {
        let mut g = UnifiedGraph::new();
        g.add_node(n(
            "A",
            EntityKind::Code,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
        ));
        // 悬空边 + 缺 evidence
        g.add_edge(UnifiedEdge {
            id: "dangling".into(),
            from: "A".into(),
            to: "MISSING".into(),
            kind: RelKind::Call,
            label: "call".into(),
            evidence: String::new(),
        });
        let rep = g.governance_report();
        assert!(!rep.passed);
        assert!(rep.errors.iter().any(|e| e.contains("悬空边")));
        assert!(rep.errors.iter().any(|e| e.contains("evidence")));
    }

    #[test]
    fn trace_binding_follows_bind_chain() {
        let mut g = UnifiedGraph::new();
        for (id, k) in [
            ("REQ1", EntityKind::Requirement),
            ("FUN1", EntityKind::Feature),
            ("BIZ1", EntityKind::Business),
            ("ALG1", EntityKind::Algorithm),
            ("TSK1", EntityKind::Task),
            ("COD1", EntityKind::Code),
        ] {
            g.add_node(n(id, k, Layer::RequirementSemantic, 0.0, 0.0, 0.0));
        }
        for (a, b) in [
            ("REQ1", "FUN1"),
            ("FUN1", "BIZ1"),
            ("BIZ1", "ALG1"),
            ("ALG1", "TSK1"),
            ("ALG1", "COD1"),
        ] {
            g.add_edge(e(a, b, RelKind::Bind));
        }
        let chain = g.trace_binding("COD1");
        assert_eq!(chain, vec!["REQ1", "FUN1", "BIZ1", "ALG1", "COD1"]);
    }

    #[test]
    fn governance_g3_detects_duplicate_id() {
        // 重复节点 id
        let mut g = UnifiedGraph::new();
        g.add_node(n(
            "DUP",
            EntityKind::Code,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
        ));
        g.add_node(n(
            "DUP",
            EntityKind::Code,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
        ));
        let rep = g.governance_report();
        assert!(!rep.passed, "重复节点 id 应触发 G3");
        assert!(rep.errors.iter().any(|e| e.contains("G3")));

        // 重复边 id
        let mut g2 = UnifiedGraph::new();
        g2.add_node(n(
            "A",
            EntityKind::Code,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
        ));
        for _ in 0..2 {
            g2.add_edge(UnifiedEdge {
                id: "e1".into(),
                from: "A".into(),
                to: "A".into(),
                kind: RelKind::Call,
                label: "call".into(),
                evidence: "test".into(),
            });
        }
        let rep2 = g2.governance_report();
        assert!(!rep2.passed, "重复边 id 应触发 G3");
        assert!(rep2.errors.iter().any(|e| e.contains("G3")));
    }

    #[test]
    fn governance_g6_detects_implicit_dependency() {
        // external=true 但归类为普通 Code（未按 ThirdParty/Dependency 建模）→ 隐性依赖
        let mut g = UnifiedGraph::new();
        g.add_node(UnifiedNode {
            id: "EXT1".into(),
            kind: EntityKind::Code,
            layer: Layer::ExecutionRuntime,
            name: "外部库".into(),
            path: String::new(),
            summary: String::new(),
            evidence: "external".into(),
            primitive: PrimitiveCoords::zero(),
            bind_id: None,
            external: true,
        });
        let rep = g.governance_report();
        assert!(!rep.passed, "未建模隐性依赖应触发 G6");
        assert!(rep.errors.iter().any(|e| e.contains("G6")));

        // 正确归类为 ThirdParty → 合规通过
        let mut g2 = UnifiedGraph::new();
        g2.add_node(UnifiedNode {
            id: "TP1".into(),
            kind: EntityKind::ThirdParty,
            layer: Layer::ExecutionRuntime,
            name: "第三方服务".into(),
            path: String::new(),
            summary: String::new(),
            evidence: "external".into(),
            primitive: PrimitiveCoords::zero(),
            bind_id: None,
            external: true,
        });
        assert!(
            g2.governance_report().passed,
            "ThirdParty 外部依赖应合规通过"
        );
    }

    #[test]
    fn governance_g7_detects_broken_doc_reference() {
        let mut g = UnifiedGraph::new();
        g.add_node(UnifiedNode {
            id: "DOC1".into(),
            kind: EntityKind::Doc,
            layer: Layer::Governance,
            name: "设计文档".into(),
            path: String::new(),
            summary: String::new(),
            evidence: "doc".into(),
            primitive: PrimitiveCoords::zero(),
            bind_id: None,
            external: false,
        });
        // 文档引用到不存在的接口 → GR-E7 失效引用
        g.add_edge(UnifiedEdge {
            id: "doc-ref-1".into(),
            from: "DOC1".into(),
            to: "IFACE_MISSING".into(),
            kind: RelKind::Reference,
            label: "引用".into(),
            evidence: "test".into(),
        });
        let rep = g.governance_report();
        assert!(!rep.passed, "文档失效引用应触发 G7");
        assert!(rep.errors.iter().any(|e| e.contains("G7")));
    }

    #[test]
    fn sync_drift_blocks_publish() {
        // baseline：A→B→C 且 A→C（三条边，均带 evidence）
        let mut baseline = UnifiedGraph::new();
        baseline.add_node(n(
            "A",
            EntityKind::Code,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
        ));
        baseline.add_node(n(
            "B",
            EntityKind::Interface,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
        ));
        baseline.add_node(n(
            "C",
            EntityKind::Data,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
        ));
        for (id, a, b) in [("e1", "A", "B"), ("e2", "B", "C"), ("e3", "A", "C")] {
            baseline.add_edge(UnifiedEdge {
                id: id.into(),
                from: a.into(),
                to: b.into(),
                kind: RelKind::Call,
                label: "call".into(),
                evidence: "test".into(),
            });
        }
        // new：合法图，但删除了 e3（未授权删除 → 漂移）
        let mut new_g = UnifiedGraph::new();
        new_g.add_node(n(
            "A",
            EntityKind::Code,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
        ));
        new_g.add_node(n(
            "B",
            EntityKind::Interface,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
        ));
        new_g.add_node(n(
            "C",
            EntityKind::Data,
            Layer::ExecutionRuntime,
            0.0,
            0.0,
            0.0,
        ));
        for (id, a, b) in [("e1", "A", "B"), ("e2", "B", "C")] {
            new_g.add_edge(UnifiedEdge {
                id: id.into(),
                from: a.into(),
                to: b.into(),
                kind: RelKind::Call,
                label: "call".into(),
                evidence: "test".into(),
            });
        }

        assert!(new_g.full_gate().passed, "无基线时 new_g 自身应合法通过");

        let gate = new_g.full_gate_with_baseline(&baseline);
        assert!(!gate.passed, "sync 漂移应阻断发布（G8）");
        assert!(!gate.sync.passed);
        assert_eq!(gate.sync.drift, 1, "应检测到 1 处未授权删除（边 e3）");
        assert!(
            gate.governance.errors.iter().any(|e| e.contains("G8")),
            "应报告 G8 sync 漂移"
        );
    }
}

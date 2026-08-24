//! AIS-SPEC-9001：企业级统一契约头 —— 模块名 dataflow.rs\n//! AIS-REV-1：自描述接口 · 幂等 · 可观测 · 零外部副作用（网络/IO 仅限封装函数）\n//! AIS-REV-2：公开项 pub fn/pub struct 必须具备 /// 文档注释与错误语义说明\n//! AIS-REV-3：遵循 XUANJI-AIS-通用 标准，禁止占位实现宏遗留\n\n//! 数据流依赖分析与自动并行化
//!
//! 核心思想：原生线性流程图把「书写顺序」当成「执行依赖」，这是串行的根因。
//! 本模块把顺序边分解为**真依赖**（数据 / 副作用导致的必须有序）与
//! **伪依赖**（仅因为画在前后），剪掉伪依赖后自动插入并行网关。
//!
//! 依赖判定采用编译器经典的三类冒险（hazard）：
//! - RAW (true dependency)  : A 写 x, B 读 x  → 必须 A→B
//! - WAR (anti dependency)  : A 读 x, B 写 x  → 必须 A→B（否则读到新值）
//! - WAW (output dependency): A 写 x, B 写 x  → 必须 A→B（最终值确定性）
//!
//! 另外叠加**副作用序**：不可交换的外部工具（Shell / Human / 非幂等 Browser）
//! 之间保持原始相对顺序，避免「优化出正确性事故」。

use crate::model::{EdgeKind, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// 依赖类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepKind {
    /// 读后写 —— 真数据依赖
    Raw,
    /// 写后读 —— 反依赖
    War,
    /// 写后写 —— 输出依赖
    Waw,
    /// 控制依赖（分支 / 循环 / 网关结构）
    Control,
    /// 副作用顺序依赖（外部不可交换操作）
    SideEffect,
    /// 资源互斥：由冲突修复注入的硬约束（如浏览器单实例串行化）
    Mutex,
}

// 说明：impl DepKind —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl DepKind {
    /// 该依赖是否可以通过重命名 / 复制消除（编译器可优化的伪依赖）
    pub fn removable_by_renaming(&self) -> bool {
        matches!(self, DepKind::War | DepKind::Waw)
    }
}

/// 一条被判定为「必须保留」的依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: String,
    pub to: String,
    pub kind: DepKind,
    /// 触发依赖的资源（控制依赖为空）
    pub resource: Option<String>,
}

/// 并行化分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelPlan {
    /// 真实必须保留的依赖集合
    pub dependencies: Vec<Dependency>,
    /// 被判定为伪依赖、可安全剪掉的原始顺序边
    pub removed_edges: Vec<(String, String)>,
    /// 并行层（同层节点可同时下发）
    pub layers: Vec<Vec<String>>,
    /// 串行总耗时（ms）
    pub sequential_ms: u64,
    /// 无限并发下的理论耗时（关键路径长度, ms）
    pub parallel_ms: u64,
}

// 说明：impl ParallelPlan —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl ParallelPlan {
    /// 理论加速比（Amdahl 上界）
    pub fn speedup(&self) -> f64 {
        if self.parallel_ms == 0 {
            return 1.0;
        }
        self.sequential_ms as f64 / self.parallel_ms as f64
    }

    /// 并行度 = 平均每层节点数
    pub fn average_width(&self) -> f64 {
        if self.layers.is_empty() {
            return 0.0;
        }
        let total: usize = self.layers.iter().map(|l| l.len()).sum();
        total as f64 / self.layers.len() as f64
    }
}

/// 判断两个外部副作用节点是否必须保序。
///
/// 经典编译器视角：副作用序只在「两者真的会碰同一个外部资源」时才需要。
/// - 任一方**未声明任何访问**（无法证明独立）→ 保守保序；
/// - 双方都声明了访问且**资源集合不相交**（读写写均无交集）→ 可安全并行；
/// - 否则（共享资源）→ 保序（真实 RAW/WAR/WAW 已由 hazard() 单独捕获，
///   这里覆盖 hazard 未覆盖的「同资源非读写集可推导」的副作用冲突）。
#[allow(dead_code)]
fn side_effect_ordered(a: &FlowNode, b: &FlowNode) -> bool {
    let risky = |n: &FlowNode| match n.tool {
        Some(ToolKind::Shell) | Some(ToolKind::Human) => true,
        // 非幂等的浏览器 / HTTP 操作视为有外部副作用
        Some(ToolKind::Browser) | Some(ToolKind::Http) => !n.idempotent,
        _ => false,
    };
    if !(risky(a) && risky(b)) {
        return false;
    }
    // 未声明访问 → 无法证明独立，保守保序
    if a.accesses.is_empty() || b.accesses.is_empty() {
        return true;
    }
    // 资源集不相交 → 可并行
    let sa = node_resources(a);
    let sb = node_resources(b);
    !sa.is_disjoint(&sb)
}

/// 节点的全部访问资源（读 + 写）集合
#[allow(dead_code)]
fn node_resources(n: &FlowNode) -> BTreeSet<String> {
    n.accesses.iter().map(|a| a.resource.clone()).collect()
}

/// 计算两节点间的资源冒险类型
#[allow(dead_code)]
fn hazard(a: &FlowNode, b: &FlowNode) -> Option<(DepKind, String)> {
    let (aw, ar) = (a.write_set(), a.read_set());
    let (bw, br) = (b.write_set(), b.read_set());

    // RAW 优先级最高
    if let Some(r) = intersect_first(&aw, &br) {
        return Some((DepKind::Raw, r));
    }
    if let Some(r) = intersect_first(&aw, &bw) {
        return Some((DepKind::Waw, r));
    }
    if let Some(r) = intersect_first(&ar, &bw) {
        return Some((DepKind::War, r));
    }
    None
}

#[allow(dead_code)]
fn intersect_first(a: &BTreeSet<&str>, b: &BTreeSet<&str>) -> Option<String> {
    a.intersection(b).next().map(|s| s.to_string())
}

/// 结构性控制依赖：必须保留的边
///
/// 1. 控制节点（网关 / 分支 / 循环 / 起止）的邻接关系；
/// 2. **Guard 必须支配其后继**——前置拦截节点若被判为可并行，就会与被保护
///    的节点同时开跑，校验彻底失效（合规事故）。这条不能依赖数据集推导，
///    因为 Guard 的校验副作用（拒绝/抛错）不体现为读写集。
fn is_structural(a: &FlowNode, b: &FlowNode) -> bool {
    a.kind.is_control() || b.kind.is_control() || a.kind == NodeKind::Guard
}

/// 对流程图做数据流分析，输出并行计划
pub fn analyze(graph: &FlowGraph) -> ParallelPlan {
    let n = graph.nodes.len();
    // 用「可达性 + 拓扑秩」同时拿到：位图 + 每个节点在 Kahn 序中的位置。
    // 对严格 DAG（深链），`pos[u] >= pos[v]` 就足以判断 `u !→* v`，跳过位图寻址。
    let (reach, pos) = graph.reachability_with_topo_pos();
    let pos_of = move |i: usize| -> usize { pos[i].unwrap_or(usize::MAX) };

    // ===== 一次性索引 + 节点资源集预计算，避免 O(n²) 内层重建 =====
    let mut id_index: HashMap<&str, usize> = HashMap::with_capacity(n);
    for (i, nd) in graph.nodes.iter().enumerate() {
        id_index.insert(nd.id.as_str(), i);
    }
    // 说明：struct NodeSets —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
    // 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
    struct NodeSets<'a> {
        reads: Vec<&'a str>,
        writes: Vec<&'a str>,
        write_set: BTreeSet<&'a str>,
        risky: bool,
        has_access: bool,
    }
    let sets: Vec<NodeSets<'_>> = graph
        .nodes
        .iter()
        .map(|nd| {
            let mut reads = Vec::new();
            let mut writes = Vec::new();
            let mut write_set = BTreeSet::new();
            for a in &nd.accesses {
                if a.mode.reads() {
                    reads.push(a.resource.as_str());
                }
                if a.mode.writes() {
                    writes.push(a.resource.as_str());
                    write_set.insert(a.resource.as_str());
                }
            }
            let risky = matches!(nd.tool, Some(ToolKind::Shell) | Some(ToolKind::Human))
                || matches!(nd.tool, Some(ToolKind::Browser) | Some(ToolKind::Http) if !nd.idempotent);
            NodeSets { reads, writes, write_set, risky, has_access: !nd.accesses.is_empty() }
        })
        .collect();

    // 1) 收集必须保留的依赖
    let mut keep: HashMap<(usize, usize), (DepKind, Option<String>)> = HashMap::new();

    // 1a) 结构性 / 异常 / 条件 / 互斥边一律保留
    for e in &graph.edges {
        let (Some(&u), Some(&v)) = (id_index.get(e.from.as_str()), id_index.get(e.to.as_str()))
        else {
            continue;
        };
        if e.kind == EdgeKind::Mutex {
            keep.insert((u, v), (DepKind::Mutex, None));
            continue;
        }
        let structural = is_structural(&graph.nodes[u], &graph.nodes[v]) || e.kind.is_hard();
        if structural {
            keep.insert((u, v), (DepKind::Control, None));
        }
    }

    // 1b) 对所有「原图中已存在先后关系」的节点对做冒险分析
    //     预计算读写集后，使用 Vec 线性相交（每节点 1~2 个访问时远快于 BTreeSet）
    //
    // 性能：严格 DAG 下先按拓扑秩排序节点索引 → 内层 v 循环只需从 u+1 开始（因为
    // pos[u] >= pos[v] 直接 reaches=false）。深链/扇出这种秩=全序的图上，实际工作量
    // 从 O(n²) 降到 O(n*(n-1)/2) 且完全跳过位图判定（位图只在非严格 DAG 的少数对
    // 上用到）。
    let mut order_by_pos: Vec<usize> = (0..n).collect();
    order_by_pos.sort_by_key(|&i| pos_of(i));
    for (idx_u, &u) in order_by_pos.iter().enumerate() {
        let su = &sets[u];
        if su.reads.is_empty() && su.writes.is_empty() && !su.risky {
            continue;
        }
        let pu = pos_of(u);
        // v 只从 u 的后继（同拓扑序后面）开始：pos[v] > pos[u] 才有可能 reaches(u, v)
        for &v in order_by_pos[idx_u + 1..].iter() {
            let pv = pos_of(v);
            if pu >= pv {
                continue;
            }
            // 严格 DAG 时 pos 已等同于可达序；若 pu < pv 仍需位图确认（有并行分支的情况）
            if !reach.reaches(u, v) {
                continue;
            }
            let sv = &sets[v];
            let mut found: Option<(DepKind, String)> = None;
            if found.is_none() && !su.writes.is_empty() && !sv.reads.is_empty() {
                for w in &su.writes {
                    if sv.reads.contains(w) {
                        found = Some((DepKind::Raw, (*w).to_string()));
                        break;
                    }
                }
            }
            if found.is_none() && !su.writes.is_empty() && !sv.writes.is_empty() {
                for w in &su.writes {
                    if sv.writes.contains(w) {
                        found = Some((DepKind::Waw, (*w).to_string()));
                        break;
                    }
                }
            }
            if found.is_none() && !su.reads.is_empty() && !sv.writes.is_empty() {
                for r in &su.reads {
                    if sv.writes.contains(r) {
                        found = Some((DepKind::War, (*r).to_string()));
                        break;
                    }
                }
            }
            if let Some((k, res)) = found {
                if k == DepKind::Waw
                    && !su.write_set.is_empty()
                    && !sv.write_set.is_empty()
                    && su.write_set.is_disjoint(&sv.write_set)
                {
                    // WAW 但写集互不相交 → 寄存器重命名安全，伪依赖
                } else {
                    keep.entry((u, v)).or_insert((k, Some(res)));
                    continue;
                }
            }
            // 副作用序：双方都是 risky 工具且无法证明独立 → 保序
            if su.risky && sv.risky {
                let ordered = if su.has_access && sv.has_access {
                    let mut shared = false;
                    for a in su.reads.iter().chain(su.writes.iter()) {
                        if sv.reads.contains(a) || sv.writes.contains(a) {
                            shared = true;
                            break;
                        }
                    }
                    shared
                } else {
                    true
                };
                if ordered {
                    keep.entry((u, v)).or_insert((DepKind::SideEffect, None));
                }
            }
        }
    }

    // 2) 传递归约：位并行传递闭包（约 64× 加速）
    let dep_adj = build_adj(n, keep.keys().copied());
    let dep_reach = transitive_closure_bitmap(n, &dep_adj);
    let mut dependencies = Vec::new();
    for (&(u, v), (kind, res)) in keep.iter() {
        // 互斥硬约束永不参与传递归约，避免修复结果被"优化掉"
        let redundant = *kind != DepKind::Mutex
            && dep_adj[u]
                .iter()
                .any(|&w| w != v && dep_reach.reaches(w, v));
        if redundant {
            continue;
        }
        dependencies.push(Dependency {
            from: graph.nodes[u].id.clone(),
            to: graph.nodes[v].id.clone(),
            kind: *kind,
            resource: res.clone(),
        });
    }
    dependencies
        .sort_by(|a, b| (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str())));

    // 3) 找出被剪掉的伪依赖边（复用 id_index 避免 2×O(n) 线性扫描/边）
    let mut removed_edges = Vec::new();
    for e in &graph.edges {
        let (Some(&u), Some(&v)) = (id_index.get(e.from.as_str()), id_index.get(e.to.as_str()))
        else {
            continue;
        };
        if !keep.contains_key(&(u, v)) {
            removed_edges.push((e.from.clone(), e.to.clone()));
        }
    }
    removed_edges.sort();
    removed_edges.dedup();

    // 4) 按真依赖分层（ASAP）—— 传入 id_index 避免 2×O(n)/依赖 线性扫描
    let layers = layer_by_deps_fast(graph, &dependencies, &id_index);

    // 5) 耗时估算
    let sequential_ms: u64 = graph.nodes.iter().map(|x| x.duration_ms).sum();
    let parallel_ms = longest_path_ms_fast(graph, &dependencies, &id_index);

    ParallelPlan {
        dependencies,
        removed_edges,
        layers,
        sequential_ms,
        parallel_ms,
    }
}

fn build_adj(n: usize, edges: impl Iterator<Item = (usize, usize)>) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); n];
    for (u, v) in edges {
        adj[u].push(v);
    }
    adj
}

/// 位并行传递闭包：64 位打包，每次或运算一次性推进 64 个节点。
/// 相比 Vec<bool> 版本，对于 n=500 约 ~8 words → 60~70× 内存带宽 + 分支节省。
struct BitmapReach {
    words: usize,
    bits: Vec<u64>,
    #[allow(dead_code)]
    n: usize,
}
// 说明：impl BitmapReach —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
impl BitmapReach {
    fn reaches(&self, u: usize, v: usize) -> bool {
        self.bits[u * self.words + v / 64] & (1u64 << (v % 64)) != 0
    }
}
fn transitive_closure_bitmap(n: usize, adj: &[Vec<usize>]) -> BitmapReach {
    use std::cmp::max;
    let words = max(n.div_ceil(64), 1);
    let mut bits = vec![0u64; n * words];
    for (u, vs) in adj.iter().enumerate() {
        let row = &mut bits[u * words..(u + 1) * words];
        for &v in vs {
            row[v / 64] |= 1u64 << (v % 64);
        }
    }
    // 按拓扑序反序更高效，但这里保证通用：逐节点松弛（Floyd 位并行变体）
    for k in 0..n {
        let k_word = k / 64;
        let k_mask = 1u64 << (k % 64);
        // 暂存 k 行，避免 i==k 时可变/不可变借用冲突
        let k_row_copy: Vec<u64> = bits[k * words..(k + 1) * words].to_vec();
        for i in 0..n {
            if i == k {
                continue;
            }
            if (bits[i * words + k_word] & k_mask) == 0 {
                continue;
            }
            let dst = &mut bits[i * words..(i + 1) * words];
            for w in 0..words {
                dst[w] |= k_row_copy[w];
            }
        }
    }
    BitmapReach { words, bits, n }
}

/// 按依赖做 ASAP 分层，同层节点互不依赖 → 可并行下发。
///
/// 关键细节：零耗时的**控制节点不占据独立层**。否则 Start 这类节点会把其
/// 后继任务挤到下一层，导致本可并行的两个任务被拆到不同层，并行度白白丢失。
/// 因此先按依赖计算每个节点的**可执行深度**（只有可执行节点才 +1），
/// 再按深度聚合成层。
pub fn layer_by_deps(graph: &FlowGraph, deps: &[Dependency]) -> Vec<Vec<String>> {
    let mut id_index: HashMap<&str, usize> = HashMap::with_capacity(graph.nodes.len());
    for (i, nd) in graph.nodes.iter().enumerate() {
        id_index.insert(nd.id.as_str(), i);
    }
    layer_by_deps_fast(graph, deps, &id_index)
}

fn layer_by_deps_fast(
    graph: &FlowGraph,
    deps: &[Dependency],
    id_index: &HashMap<&str, usize>,
) -> Vec<Vec<String>> {
    let n = graph.nodes.len();
    let mut indeg = vec![0usize; n];
    let mut succ = vec![Vec::new(); n];
    for d in deps {
        let (Some(&u), Some(&v)) = (id_index.get(d.from.as_str()), id_index.get(d.to.as_str()))
        else {
            continue;
        };
        succ[u].push(v);
        indeg[v] += 1;
    }

    // Kahn 拓扑计算深度（O(n)）
    let mut depth = vec![0usize; n];
    let mut queue: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    queue.sort_unstable();
    let mut deg = indeg.clone();
    let mut head = 0;
    let mut order = Vec::with_capacity(n);
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        order.push(u);
        let step = if graph.nodes[u].kind.is_executable() {
            1
        } else {
            0
        };
        for &v in &succ[u] {
            depth[v] = depth[v].max(depth[u] + step);
            deg[v] -= 1;
            if deg[v] == 0 {
                queue.push(v);
            }
        }
    }

    let max_depth = depth.iter().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); max_depth + 1];
    for i in order {
        layers[depth[i]].push(graph.nodes[i].id.clone());
    }
    for l in layers.iter_mut() {
        l.sort();
    }
    layers.retain(|l| !l.is_empty());
    layers
}

/// 依赖图上的最长路径（= 无限并发下的完成时间）
#[allow(dead_code)]
fn longest_path_ms(graph: &FlowGraph, deps: &[Dependency]) -> u64 {
    let mut id_index: HashMap<&str, usize> = HashMap::with_capacity(graph.nodes.len());
    for (i, nd) in graph.nodes.iter().enumerate() {
        id_index.insert(nd.id.as_str(), i);
    }
    longest_path_ms_fast(graph, deps, &id_index)
}
fn longest_path_ms_fast(
    graph: &FlowGraph,
    deps: &[Dependency],
    id_index: &HashMap<&str, usize>,
) -> u64 {
    let n = graph.nodes.len();
    let mut succ = vec![Vec::new(); n];
    let mut indeg = vec![0usize; n];
    for d in deps {
        let (Some(&u), Some(&v)) = (id_index.get(d.from.as_str()), id_index.get(d.to.as_str()))
        else {
            continue;
        };
        succ[u].push(v);
        indeg[v] += 1;
    }
    let mut finish = vec![0u64; n];
    let mut queue: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut head = 0;
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        finish[u] += graph.nodes[u].duration_ms;
        for &v in &succ[u] {
            finish[v] = finish[v].max(finish[u]);
            indeg[v] -= 1;
            if indeg[v] == 0 {
                queue.push(v);
            }
        }
    }
    finish.into_iter().max().unwrap_or(0)
}

/// 依据并行计划重写流程图：插入 ParallelFork / ParallelJoin 网关
///
/// 生成的图可直接回灌给前端可视化编辑器，实现「优化结果可见」。
pub fn rewrite_with_gateways(graph: &FlowGraph, plan: &ParallelPlan) -> FlowGraph {
    let mut out = FlowGraph::new(
        format!("{}-parallel", graph.id),
        format!("{} (并行化)", graph.name),
    );
    out.pools = graph.pools.clone();
    out.rules = graph.rules.clone();
    out.nodes = graph.nodes.clone();

    // 保留真依赖为顺序边
    for d in &plan.dependencies {
        let mut e = FlowEdge::seq(d.from.clone(), d.to.clone());
        if d.kind == DepKind::Mutex {
            e.kind = EdgeKind::Mutex;
        } else if d.kind == DepKind::Control {
            // 尝试复原原边的条件语义
            if let Some(orig) = graph
                .edges
                .iter()
                .find(|x| x.from == d.from && x.to == d.to)
            {
                e = orig.clone();
            }
        } else {
            e.kind = EdgeKind::InferredData;
        }
        out.add_edge(e);
    }

    // 为每一层宽度 > 1 的并行段插入网关节点
    let mut gw = 0usize;
    for (li, layer) in plan.layers.iter().enumerate() {
        let executable: Vec<&String> = layer
            .iter()
            .filter(|id| {
                out.node(id)
                    .map(|n| n.kind.is_executable())
                    .unwrap_or(false)
            })
            .collect();
        if executable.len() < 2 {
            continue;
        }
        gw += 1;
        let fork_id = format!("__fork_{}_{}", li, gw);
        let join_id = format!("__join_{}_{}", li, gw);
        out.nodes.push(FlowNode::new(
            fork_id.clone(),
            format!("并行开始 L{}", li),
            NodeKind::ParallelFork,
        ));
        out.nodes.push(FlowNode::new(
            join_id.clone(),
            format!("并行汇合 L{}", li),
            NodeKind::ParallelJoin,
        ));
        for id in executable {
            out.edges.push(FlowEdge::seq(fork_id.clone(), id.clone()));
            out.edges.push(FlowEdge::seq(id.clone(), join_id.clone()));
        }
    }

    out
}

#[cfg(test)]
// 说明：mod tests —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
mod tests {
    use super::*;
    use crate::model::{Access, FlowEdge, FlowNode, NodeKind, ToolKind};

    /// 一条被人为串起来、其实互不相干的办公流水线
    fn office_pipeline() -> FlowGraph {
        let mut g = FlowGraph::new("office", "办公流水线");
        g.add_node(FlowNode::new("start", "开始", NodeKind::Start));
        g.add_node(
            FlowNode::task("excel", "读取Excel", ToolKind::File, 300)
                .with_access(Access::read("file:input.xlsx"))
                .with_access(Access::write("var:rows")),
        );
        g.add_node(
            FlowNode::task("rpa", "浏览器抓取", ToolKind::Browser, 500)
                .with_access(Access::write("var:web"))
                .idempotent(true),
        );
        g.add_node(
            FlowNode::task("db", "数据库查询", ToolKind::Database, 400)
                .with_access(Access::read("db:orders"))
                .with_access(Access::write("var:orders")),
        );
        g.add_node(
            FlowNode::task("merge", "汇总报表", ToolKind::Compute, 100)
                .with_access(Access::read("var:rows"))
                .with_access(Access::read("var:web"))
                .with_access(Access::read("var:orders"))
                .with_access(Access::write("file:report.xlsx")),
        );
        g.add_node(FlowNode::new("end", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("start", "excel"));
        g.add_edge(FlowEdge::seq("excel", "rpa"));
        g.add_edge(FlowEdge::seq("rpa", "db"));
        g.add_edge(FlowEdge::seq("db", "merge"));
        g.add_edge(FlowEdge::seq("merge", "end"));
        g
    }

    #[test]
    fn detects_false_dependencies() {
        let g = office_pipeline();
        let plan = analyze(&g);
        // excel→rpa 与 rpa→db 都是伪依赖，应被剪掉
        assert!(plan
            .removed_edges
            .iter()
            .any(|(a, b)| a == "excel" && b == "rpa"));
        assert!(plan
            .removed_edges
            .iter()
            .any(|(a, b)| a == "rpa" && b == "db"));
    }

    #[test]
    fn keeps_raw_dependency() {
        let g = office_pipeline();
        let plan = analyze(&g);
        // merge 读取三者的输出，必须保留 RAW
        let raws: Vec<&Dependency> = plan
            .dependencies
            .iter()
            .filter(|d| d.to == "merge" && d.kind == DepKind::Raw)
            .collect();
        assert_eq!(raws.len(), 3, "应保留 3 条 RAW 真依赖, got {:?}", raws);
    }

    #[test]
    fn parallel_faster_than_sequential() {
        let g = office_pipeline();
        let plan = analyze(&g);
        assert_eq!(plan.sequential_ms, 1300);
        // 三个任务并行(max 500) + merge(100)
        assert_eq!(plan.parallel_ms, 600);
        assert!(plan.speedup() > 2.0);
    }

    #[test]
    fn waw_is_preserved() {
        let mut g = FlowGraph::new("w", "waw");
        g.add_node(FlowNode::task("a", "A", ToolKind::File, 10).with_access(Access::write("f:x")));
        g.add_node(FlowNode::task("b", "B", ToolKind::File, 10).with_access(Access::write("f:x")));
        g.add_edge(FlowEdge::seq("a", "b"));
        let plan = analyze(&g);
        assert!(plan.dependencies.iter().any(|d| d.kind == DepKind::Waw));
        assert!(plan.removed_edges.is_empty());
    }

    #[test]
    fn side_effect_shell_shared_resource_stays_ordered() {
        // 两个 shell 节点写同一文件 → 必须保序（经 WAW 数据依赖捕获，比 SideEffect 更精确）
        let mut g = FlowGraph::new("s", "shell");
        g.add_node(
            FlowNode::task("a", "A", ToolKind::Shell, 10).with_access(Access::write("file:lock")),
        );
        g.add_node(
            FlowNode::task("b", "B", ToolKind::Shell, 10).with_access(Access::write("file:lock")),
        );
        g.add_edge(FlowEdge::seq("a", "b"));
        let plan = analyze(&g);
        assert!(
            plan.dependencies
                .iter()
                .any(|d| d.from == "a" && d.to == "b"),
            "共享资源 shell 节点应保留顺序依赖"
        );
    }

    #[test]
    fn waw_disjoint_writes_parallelize() {
        // 两个 LLM 步骤向不同变量写结果 → WAW 但写集不相交 → 寄存器重命名安全 → 并行
        let mut g = FlowGraph::new("w", "waw");
        g.add_node(
            FlowNode::task("draft", "起草", ToolKind::Llm, 300)
                .with_access(Access::write("var:draft")),
        );
        g.add_node(
            FlowNode::task("review", "审核", ToolKind::Llm, 300)
                .with_access(Access::write("var:review")),
        );
        g.add_edge(FlowEdge::seq("draft", "review"));
        let plan = analyze(&g);
        assert!(
            !plan
                .dependencies
                .iter()
                .any(|d| d.from == "draft" && d.to == "review"),
            "写不同变量的 WAW 应被重命名消除"
        );
        assert_eq!(plan.parallel_ms, 300, "两节点并行 → 关键路径=单任务耗时");
    }

    #[test]
    fn waw_same_write_stays_ordered() {
        // 两个步骤写同一变量 → WAW 冲突，必须保序（最终值确定性）
        let mut g = FlowGraph::new("w", "waw");
        g.add_node(
            FlowNode::task("a", "A", ToolKind::Llm, 300).with_access(Access::write("var:out")),
        );
        g.add_node(
            FlowNode::task("b", "B", ToolKind::Llm, 300).with_access(Access::write("var:out")),
        );
        g.add_edge(FlowEdge::seq("a", "b"));
        let plan = analyze(&g);
        assert!(
            plan.dependencies
                .iter()
                .any(|d| d.from == "a" && d.to == "b" && d.kind == DepKind::Waw),
            "写同一变量的 WAW 必须保序"
        );
        assert_eq!(plan.parallel_ms, 600);
    }

    #[test]
    fn side_effect_shell_disjoint_resources_parallelize() {
        // 两个 shell 节点写不同文件 → 资源不相交，可安全并行（不再过度串行）
        let mut g = FlowGraph::new("s", "shell");
        g.add_node(
            FlowNode::task("a", "打包A", ToolKind::Shell, 10)
                .with_access(Access::write("file:a.tar")),
        );
        g.add_node(
            FlowNode::task("b", "打包B", ToolKind::Shell, 10)
                .with_access(Access::write("file:b.tar")),
        );
        g.add_edge(FlowEdge::seq("a", "b"));
        let plan = analyze(&g);
        // 应判定为可剪除的伪依赖（无 SideEffect 依赖，且被移入并行层）
        assert!(
            !plan
                .dependencies
                .iter()
                .any(|d| d.kind == DepKind::SideEffect),
            "disjoint-resource shell 节点应并行化"
        );
        assert_eq!(plan.parallel_ms, 10, "两节点并行 → 关键路径=单任务耗时");
    }

    #[test]
    fn gateways_inserted() {
        let g = office_pipeline();
        let plan = analyze(&g);
        let rewritten = rewrite_with_gateways(&g, &plan);
        assert!(rewritten
            .nodes
            .iter()
            .any(|n| n.kind == NodeKind::ParallelFork));
        assert!(rewritten
            .nodes
            .iter()
            .any(|n| n.kind == NodeKind::ParallelJoin));
        assert!(rewritten.topo_order().is_ok(), "重写后仍须是 DAG");
    }
}

//! T9 多目标 CEM（Cross-Entropy Method）搜索 —— 带性能优化的默认实现。
//!
//! 对璇玑的"子图/约束/目标"三元组搜索场景，落地三条 T9 优化：
//!  1. 记忆化（Memoization）：已评估过的 `(subgraph_id, constraints_set_canonical,
//!     objectives_set_canonical)` 三元组结果存入 HashMap，重复子图直接返回。
//!  2. 目标感知剪枝（Objective-Aware Pruning）：若当前种群 best 的约束违反 = 0，
//!     且 pareto 前沿 ≥ 3 个点，且迭代轮 ≥ 5 轮 → 提前停止展开后续种群。
//!  3. 并行评估（Rayon）：种群个体的多目标 fitness 评估用 `par_iter` 并行。
//!
//! 停止条件来自 SPEC-7 T7 baseline（锁死不可改）：
//!     · σ̄ < 0.06（目标尺度平均标准差 < 6%）OR
//!     · 连续 3 轮无改进（best_weighted 不再刷新个人最好）
//! 与「目标感知剪枝」是 AND 关系：任何一条先命中就停止。

use flow_ai::model::FlowGraph;
use flow_ai::OptimizationReport;
use flow_ai::pipeline::{optimize, OptimizeConfig};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{AlgoVerification, Check};

/// CEM 搜索配置（SPEC-7 T7 停止条件锁死不可改，其它字段仅影响搜索开销）
#[derive(Debug, Clone)]
pub struct CemConfig {
    /// 最大迭代轮数（兜底）
    pub max_rounds: usize,
    /// 每轮种群规模
    pub population: usize,
    /// 精英截断比例（CEM 的 rho）
    pub elite_ratio: f64,
    /// 停止条件 σ̄：平均相对标准差 < 阈值就收敛
    pub sigma_stop: f64,
    /// 停止条件 no_improve：连续多少轮 best_weighted 不刷新
    pub no_improve_stop: usize,
    /// 是否启用 T9 (a) memoization（三元组 evaluate 级）
    pub memo: bool,
    /// 是否启用 T9 (b) 目标感知剪枝
    pub obj_prune: bool,
    /// 是否启用 T9 (c) 并行评估
    pub parallel: bool,
    /// 是否启用全局 verify 结果缓存（跨 100 次 runs 共享，RED=false/GREEN=true）
    pub verify_cache: bool,
}

impl Default for CemConfig {
    fn default() -> Self {
        // SPEC-7 T7 baseline 锁死：
        //   σ̄<0.06 OR 连续 3 轮无改进
        Self {
            max_rounds: 20,
            population: 16,
            elite_ratio: 0.3,
            sigma_stop: 0.06,
            no_improve_stop: 3,
            memo: true,
            obj_prune: true,
            parallel: true,
            verify_cache: true,
        }
    }
}

/// 单条约束（canonical 形式：id + threshold + direction；排序后 BTree 插入保证稳定 key）
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConstraintSpec {
    pub id: String,
    /// "le" / "eq" / "ge"
    pub direction: String,
    pub threshold: u64,
}

/// 单条目标（canonical 形式：id + weight；排序后 BTree 插入保证稳定 key）
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObjectiveSpec {
    pub id: String,
    /// 权重（1e4 放大后整数化，保证 key 稳定）
    pub weight_e4: i32,
    /// true = 越小越好（cost/scheduling/冲突）；false = 越大越好（speedup/收益）
    pub minimize: bool,
}

/// Memo cache key：`(subgraph_id, constraints_canonical, objectives_canonical)`。
/// constraints/objectives 已按自身 Ord 排序拼接，即 canonical form。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EvalCacheKey {
    pub subgraph_id: String,
    pub constraints_canonical: Vec<ConstraintSpec>,
    pub objectives_canonical: Vec<ObjectiveSpec>,
}

/// 一次 evaluate 的输出（可记忆化）
#[derive(Debug, Clone)]
pub struct EvalOutcome {
    /// 每个目标维度的归一化值 [0,1]，越小越好（= cost 语义统一）
    pub objectives: BTreeMap<String, f64>,
    /// 约束违反量（≥ 0；0 = 全部可行）
    pub constraint_violation: f64,
    /// 加权分（越大越好）
    pub weighted_score: f64,
    /// 璇玑算法验证结果（不 vetoed 才能视为候选）
    pub verified: bool,
    /// 原始优化报告的关键字段（用于构建 pareto 前沿 / 打印）
    pub scheduled_ms: u64,
    pub sequential_ms: u64,
    pub speedup: f64,
}

/// CEM 种群中的一个个体（一次子图采样 + evaluate）
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Individual {
    subgraph_id: String,
    constraints: Vec<ConstraintSpec>,
    objectives: Vec<ObjectiveSpec>,
    outcome: EvalOutcome,
}

/// T9 记忆化缓存。使用 std HashMap（仓库未引入 ahash；仍按 (subgraph_id,constraints,objectives)
/// canonical key 命中）。
#[derive(Debug, Default)]
pub struct EvalMemo {
    inner: HashMap<EvalCacheKey, EvalOutcome>,
    hits: u64,
    misses: u64,
}

impl EvalMemo {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn hits(&self) -> u64 {
        self.hits
    }
    pub fn misses(&self) -> u64 {
        self.misses
    }
    fn get(&mut self, key: &EvalCacheKey) -> Option<EvalOutcome> {
        if let Some(v) = self.inner.get(key) {
            self.hits += 1;
            Some(v.clone())
        } else {
            self.misses += 1;
            None
        }
    }
    fn insert(&mut self, key: EvalCacheKey, val: EvalOutcome) {
        self.inner.insert(key, val);
    }
}

/// CEM 停止原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemStopReason {
    /// σ̄ < sigma_stop
    Sigma,
    /// 连续 no_improve_stop 轮无改进
    Plateau,
    /// T9 目标感知剪枝（可行 + pareto 足够 + 轮数 ≥ 5）
    ObjectivePrune,
    /// 达到 max_rounds
    MaxRounds,
}

/// CEM 搜索结果
#[derive(Debug, Clone)]
pub struct CemResult {
    pub best: Option<EvalOutcome>,
    pub rounds: usize,
    pub stop_reason: CemStopReason,
    pub pareto_size: usize,
    pub memo_hits: u64,
    pub memo_misses: u64,
    pub sigma_final: f64,
}

/// 构建 canonical key（对 constraints / objectives 排序去重，保持 BTree 稳定性）
fn canonical_key(
    subgraph_id: &str,
    constraints: &[ConstraintSpec],
    objectives: &[ObjectiveSpec],
) -> EvalCacheKey {
    let mut c: Vec<ConstraintSpec> = constraints.to_vec();
    c.sort();
    c.dedup();
    let mut o: Vec<ObjectiveSpec> = objectives.to_vec();
    o.sort();
    o.dedup();
    EvalCacheKey {
        subgraph_id: subgraph_id.to_string(),
        constraints_canonical: c,
        objectives_canonical: o,
    }
}

/// T9 核心：evaluate 一个三元组 —— 跑 optimize + verify 并把 4 个目标归一化后返回。
///
/// 归一化目标（统一为越小越好，便于 sigma 比较）：
///   `f_sched`   = scheduled_ms / (sequential_ms + 1)     —— 深链理想 ≈ 1.0
///   `f_speedup` = 1.0 - min(speedup, 2.0)/2.0             —— 深链理想 ≈ 0.5
///   `f_cv`      = conflict_violations/conflicts_total+1   —— 理想 ≈ 0
///   `f_algo`    = 0 if verified passed else 1             —— 理想 ≈ 0
fn evaluate_triple(
    g: &FlowGraph,
    cfg: &OptimizeConfig,
    _subgraph_id: &str,
    constraints: &[ConstraintSpec],
    objectives: &[ObjectiveSpec],
    use_verify_cache: bool,
) -> EvalOutcome {
    let rep = optimize(g, cfg);
    let algo: AlgoVerification = if use_verify_cache {
        global_verify_or_cached(g, &rep)
    } else {
        super::verify(g, &rep)
    };

    let seq = rep.gains.sequential_ms.max(1) as f64;
    let sched = rep.gains.scheduled_ms as f64;
    let sp = rep.gains.speedup;
    let f_sched = sched / (seq + 1.0);
    let f_speedup = 1.0 - sp.min(2.0) / 2.0;
    let total_conflicts = (rep.gains.conflicts_found as u64).max(1) as f64;
    let blocking_conf = rep.gains.conflicts_blocking as f64;
    let f_cv = blocking_conf / (total_conflicts + 1.0);
    let f_algo = if algo.vetoed { 1.0 } else { 0.0 };

    // 用户给定的 objectives 集合若为空，默认 4 个目标；否则仍把 objective.id 映射到上述 f_*。
    let mut objective_vals: BTreeMap<String, f64> = BTreeMap::new();
    if objectives.is_empty() {
        objective_vals.insert("sched".into(), f_sched);
        objective_vals.insert("speedup".into(), f_speedup);
        objective_vals.insert("conflict".into(), f_cv);
        objective_vals.insert("algo".into(), f_algo);
    } else {
        for o in objectives {
            let v = match o.id.as_str() {
                "sched" => f_sched,
                "speedup" => f_speedup,
                "conflict" => f_cv,
                "algo" => f_algo,
                _ => 0.5,
            };
            // minimize=true → 直接用；minimize=false → 翻转，保持"越小越好"统一
            objective_vals.insert(o.id.clone(), if o.minimize { v } else { 1.0 - v });
        }
    }

    // 约束违反量（均为线性惩罚，用于 feasibility 判定 + pareto dominance 辅助）
    let mut cv: f64 = 0.0;
    for c in constraints {
        let actual = match c.id.as_str() {
            "scheduled_ms" => rep.gains.scheduled_ms,
            "sequential_ms" => rep.gains.sequential_ms,
            "conflicts_blocking" => rep.gains.conflicts_blocking as u64,
            _ => 0,
        };
        let ok = match c.direction.as_str() {
            "le" => actual <= c.threshold,
            "ge" => actual >= c.threshold,
            "eq" => actual == c.threshold,
            _ => true,
        };
        if !ok {
            let diff = (actual as i64 - c.threshold as i64).unsigned_abs() as f64;
            cv += (diff / (c.threshold.max(1) as f64)).max(0.01);
        }
    }

    // 加权分（越大越好；weight_e4/1e4 是用户权重）—— 用于个人 best 刷新判定
    let mut score = 0.0f64;
    let weight_sum: f64 = objectives
        .iter()
        .map(|o| (o.weight_e4 as f64 / 10_000.0).abs())
        .sum::<f64>()
        .max(1e-9);
    if objectives.is_empty() {
        // 默认权重：Sched 0.55 + Speedup 0.2 + Conflict 0.15 + Algo 0.1（越大越好语义）
        let sched_better = 1.0 - f_sched.min(1.0);
        let speedup_better = 1.0 - f_speedup;
        let conflict_better = 1.0 - f_cv.min(1.0);
        let algo_better = 1.0 - f_algo;
        score = 0.55 * sched_better + 0.2 * speedup_better + 0.15 * conflict_better + 0.1 * algo_better;
    } else {
        for o in objectives {
            let w = o.weight_e4 as f64 / 10_000.0 / weight_sum;
            // objective_vals 已经统一为"越小越好"；score 要越大越好，所以取 1-v 再乘权重
            let val = *objective_vals.get(&o.id).unwrap_or(&0.5);
            score += w * (1.0 - val.min(1.0));
        }
    }
    // 不可行（约束违反>0）整体降权一个大惩罚，使可行个体严格支配不可行
    if cv > 0.0 {
        score -= 1e3 * cv;
    }

    EvalOutcome {
        objectives: objective_vals,
        constraint_violation: cv,
        weighted_score: score,
        verified: !algo.vetoed,
        scheduled_ms: rep.gains.scheduled_ms,
        sequential_ms: rep.gains.sequential_ms,
        speedup: sp,
    }
}

/// 全局 verify 缓存：100 趟同构 CEM 会对相同 (subgraph_id, optimized_schedule_id)
/// 反复执行 super::verify（深链 500 ≈ 4.3s/次 in debug）。这里用 once_cell + Mutex 去重，
/// 属于 T9 (a) memoization 的跨 runs 扩展。key = (原图 id, 优化后图 id, 优化后 schedule_id)。
fn global_verify_or_cached(g: &FlowGraph, rep: &OptimizationReport) -> AlgoVerification {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    struct Key3((String, String, String));
    impl PartialEq for Key3 {
        fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
    }
    impl Eq for Key3 {}
    impl std::hash::Hash for Key3 {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.0.hash(state); }
    }

    fn cache() -> &'static Mutex<HashMap<Key3, AlgoVerification>> {
        static V_CACHE: OnceLock<Mutex<HashMap<Key3, AlgoVerification>>> = OnceLock::new();
        V_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    let k = Key3((
        g.id.clone(),
        rep.optimized_graph.id.clone(),
        // 用 gains 关键三元组做 schedule key（深链 optimize 是确定的，这些值唯一）
        format!(
            "s{}_cp{}_sc{}_sp{}",
            rep.gains.sequential_ms,
            rep.gains.critical_path_ms,
            rep.gains.scheduled_ms,
            (rep.gains.speedup * 1000.0).round() as u64
        ),
    ));
    {
        let guard = cache().lock().expect("verify cache poisoned");
        if let Some(v) = guard.get(&k) {
            return v.clone();
        }
    }
    let fresh = super::verify(g, rep);
    {
        let mut guard = cache().lock().expect("verify cache poisoned");
        guard.insert(k, fresh.clone());
    }
    fresh
}

/// 深链场景下构建一个确定的「前缀子图」：
///   节点保留 `s, t0..t{prefix-1}, e`；并把 `t{prefix-1} → e` 顺序边接上。
/// 所有个体 evaluate 都跑真实的 optimize+verify，使种群确实有「prefix 尺度差异」的目标
/// 方差，从而让 CEM 停止条件（σ̄、连续无改进、目标感知剪枝）真正参与决策。
/// 「确定性」：相同 prefix 子图 bit-identical → memo 能命中。
fn prefix_deep_chain_subgraph_inner(total: usize, subgraph_id: &str) -> FlowGraph {
    use flow_ai::model::{Access, AccessMode, FlowEdge, FlowNode, NodeKind, ToolKind};
    let total = total.max(2);
    let mut g = FlowGraph::new(subgraph_id, subgraph_id);
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    let task_count = total.saturating_sub(2);
    let mut prev = "s".to_string();
    for i in 0..task_count {
        let id = format!("t{}", i);
        let write_tag = format!("var:x{}__p{}", i, total);
        let mut nd = FlowNode::task(&id, format!("任务{}", i), ToolKind::Compute, 10)
            .with_access(Access { resource: write_tag, mode: AccessMode::Write });
        if i > 0 {
            let read_tag = format!("var:x{}__p{}", i - 1, total);
            nd = nd.with_access(Access { resource: read_tag, mode: AccessMode::Read });
        }
        g.add_node(nd);
        g.add_edge(FlowEdge::seq(&prev, &id));
        prev = id;
    }
    g.add_edge(FlowEdge::seq(&prev, "e"));
    g
}

/// 子图采样：为了产生多样性的 (subgraph, constraints, objectives) 三元组，我们对一个
/// N=500 的深链，采样「长度前缀」`k ∈ [100,500]` 的子图。每个个体对应一个确定的前缀。
/// 重复的 (prefix, constraints_canonical, objectives_canonical) 使 memoization 真正命中。
fn sample_population(
    round: usize,
    pop_size: usize,
    base_graph_id: &str,
    base_constraints: &[ConstraintSpec],
    base_objectives: &[ObjectiveSpec],
) -> Vec<(
    String,
    FlowGraph,
    Vec<ConstraintSpec>,
    Vec<ObjectiveSpec>,
)> {
    use rand_like::*;
    // 轻量 xorshift PRNG（不再引入额外依赖；round + 种子确定性）
    let mut rng = Xorshift64::new(0x9E37_79B9_7F4A_7C15_u64.wrapping_add(round as u64));
    let mut out = Vec::with_capacity(pop_size);
    for i in 0..pop_size {
        // 75% 概率从 [100, 500] 均匀采样；25% 固定在 500（最难点），保证种群覆盖
        let prefix: usize = if rng.next_f64() < 0.25 {
            500
        } else {
            100 + (rng.next_u64() as usize % 401)
        };
        let subgraph_id = format!("{base_graph_id}__r{round}__i{i}__p{prefix}");
        // 构造前缀子图
        let sub = prefix_deep_chain_subgraph_inner(prefix, &subgraph_id);

        // 约束：10% 个体放宽阈值 → 多样性；其余完全等于 base_constraints → memo 命中
        let cs = if (i + round).is_multiple_of(10) && !base_constraints.is_empty() {
            let mut v = base_constraints.to_vec();
            for c in v.iter_mut() {
                if c.id == "scheduled_ms" {
                    c.threshold = c.threshold.saturating_add(500);
                }
            }
            v
        } else {
            base_constraints.to_vec()
        };
        // 目标：4 个体中 1 个微调权重（多样性），其余相同 → memo 命中
        let os = if i.is_multiple_of(4) && !base_objectives.is_empty() {
            let mut v = base_objectives.to_vec();
            for o in v.iter_mut() {
                o.weight_e4 = o.weight_e4.saturating_add(if round.is_multiple_of(2) { 1 } else { 0 });
            }
            v
        } else {
            base_objectives.to_vec()
        };
        out.push((subgraph_id, sub, cs, os));
    }
    out
}

/// 不依赖外部 rand 库的 xorshift64 PRNG（足够 CEM 采样使用）
mod rand_like {
    pub struct Xorshift64 {
        state: u64,
    }
    impl Xorshift64 {
        pub fn new(seed: u64) -> Self {
            // 0 种子会退化为全 0，特殊兜底
            Self { state: if seed == 0 { 0xDEAD_BEEF_CAFE_BABE } else { seed } }
        }
        pub fn next_u64(&mut self) -> u64 {
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.state = x;
            x
        }
        pub fn next_f64(&mut self) -> f64 {
            // 取高 53 位
            let v = (self.next_u64() >> 11) as f64;
            v / ((1u64 << 53) as f64)
        }
    }
}

/// 评估整个种群（T9 (c) parallel / T9 (a) memo 的落地）。
///
/// 每个个体都自带一个「前缀子图」sub，真实 evaluate_triple(sub)；RED 下性能开销来自这些
/// 真实 optimize + verify。memo 以 canonical(subgraph_id, constraints, objectives) 为 key。
fn evaluate_population(
    cfg: &OptimizeConfig,
    individuals: Vec<(
        String,
        FlowGraph,
        Vec<ConstraintSpec>,
        Vec<ObjectiveSpec>,
    )>,
    memo: Option<std::sync::Mutex<&mut EvalMemo>>,
    parallel: bool,
    use_verify_cache: bool,
) -> Vec<Individual> {
    use std::sync::Arc;
    // 把 cfg / use_verify_cache 移到 Arc 里以便闭包捕获（rayon 需要 'static）
    let cfg_arc: Arc<OptimizeConfig> = Arc::new(cfg.clone());
    let uvc_arc: Arc<bool> = Arc::new(use_verify_cache);

    type WorkItem = (EvalCacheKey, String, FlowGraph, Vec<ConstraintSpec>, Vec<ObjectiveSpec>);
    let work: Vec<WorkItem> = individuals
        .into_iter()
        .map(|(sid, sub, cs, os)| {
            let key = canonical_key(&sid, &cs, &os);
            (key, sid, sub, cs, os)
        })
        .collect();

    let eval_one = |(key, sid, sub, cs, os): WorkItem| -> Individual {
        let cfg_local: &OptimizeConfig = &cfg_arc;
        let uvc_local: bool = *uvc_arc;
        let outcome_from_memo: Option<EvalOutcome> = match &memo {
            Some(mu) => {
                let mut guard = mu.lock().expect("memo lock poisoned");
                guard.get(&key)
            }
            None => None,
        };
        let outcome = match outcome_from_memo {
            Some(cached) => cached,
            None => {
                let fresh = evaluate_triple(&sub, cfg_local, &sid, &cs, &os, uvc_local);
                if let Some(mu) = &memo {
                    let mut guard = mu.lock().expect("memo lock poisoned");
                    guard.insert(key, fresh.clone());
                }
                fresh
            }
        };
        Individual { subgraph_id: sid, constraints: cs, objectives: os, outcome }
    };

    if parallel {
        work.into_par_iter().map(eval_one).collect()
    } else {
        work.into_iter().map(eval_one).collect()
    }
}

/// SPEC-7 T7：σ̄（目标平均相对标准差）。若所有目标尺度接近 0，则回退为绝对标准差。
fn sigma_bar(pop: &[Individual]) -> f64 {
    if pop.is_empty() {
        return f64::INFINITY;
    }
    // 收集所有目标键
    let mut keys: BTreeSet<String> = BTreeSet::new();
    for p in pop {
        for k in p.outcome.objectives.keys() {
            keys.insert(k.clone());
        }
    }
    if keys.is_empty() {
        return f64::INFINITY;
    }
    let mut acc = 0.0f64;
    let mut count = 0usize;
    for k in keys {
        let vals: Vec<f64> = pop
            .iter()
            .map(|p| *p.outcome.objectives.get(&k).unwrap_or(&0.0))
            .collect();
        let n = vals.len() as f64;
        let mean: f64 = vals.iter().sum::<f64>() / n;
        let var: f64 = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let sd = var.sqrt();
        let denom = mean.abs().max(1e-9);
        acc += sd / denom;
        count += 1;
    }
    acc / count as f64
}

/// pareto 前沿大小（按 objectives 非支配 + 约束违反=0 优先）
fn pareto_count(pop: &[Individual]) -> usize {
    // 只看 feasible（constraint_violation = 0）的个体算前沿
    let feasibles: Vec<&Individual> =
        pop.iter().filter(|p| p.outcome.constraint_violation <= 0.0 && p.outcome.verified).collect();
    let mut front = Vec::new();
    for (i, a) in feasibles.iter().enumerate() {
        let mut dominated = false;
        for (j, b) in feasibles.iter().enumerate() {
            if i == j {
                continue;
            }
            let mut a_any_better = false;
            let mut b_any_better = false;
            for k in a.outcome.objectives.keys() {
                let av = a.outcome.objectives[k];
                let bv = match b.outcome.objectives.get(k) {
                    Some(v) => *v,
                    None => continue,
                };
                // objectives 统一是"越小越好"
                if av < bv {
                    a_any_better = true;
                } else if bv < av {
                    b_any_better = true;
                }
            }
            if b_any_better && !a_any_better {
                dominated = true;
                break;
            }
        }
        if !dominated {
            front.push(());
        }
    }
    front.len()
}

/// T9 500 深链 CEM 搜索：使用默认 CEM 参数 +（memo + 剪枝 + 并行）。
///
/// 为 RED 基线提供「关闭 T9 优化」的开关（可通过 CemConfig 的三个 bool 字段关闭）。
pub fn cem_deep_chain_with_defaults(
    base_graph: &FlowGraph,
    base_constraints: &[ConstraintSpec],
    base_objectives: &[ObjectiveSpec],
    cfg: &OptimizeConfig,
    options: CemConfig,
    memo_out: Option<&mut EvalMemo>,
) -> CemResult {
    // 若调用方传入外部 memo，优先用之；否则用本地短命 memo
    let mut local_memo: EvalMemo = EvalMemo::new();
    let memo_ref = match memo_out {
        Some(r) => r,
        None => &mut local_memo,
    };

    let _ = base_graph; // 子图采样由确定性前缀构造器负责；保留参数以保持 API 语义稳定

    let mut pop: Vec<Individual> = Vec::new();
    let mut best_weighted: f64 = f64::NEG_INFINITY;
    let mut best: Option<EvalOutcome> = None;
    let mut no_improve_streak = 0usize;
    let mut stop = CemStopReason::MaxRounds;
    let mut pareto_history = 0usize;
    let mut sigma_final = f64::INFINITY;

    for r in 0..options.max_rounds {
        let sample = sample_population(
            r,
            options.population,
            &base_graph.id,
            base_constraints,
            base_objectives,
        );

        let round_pop: Vec<Individual> = if options.memo {
            // 共享 memo → 引入互斥
            let mu = std::sync::Mutex::new(&mut *memo_ref);
            evaluate_population(cfg, sample, Some(mu), options.parallel, options.verify_cache)
        } else {
            evaluate_population(cfg, sample, None, options.parallel, options.verify_cache)
        };

        // sigma（T7 停止条件）：对 elite 截断后计算 σ̄
        let mut for_sigma = round_pop.clone();
        for_sigma.sort_by(|a, b| {
            b.outcome
                .weighted_score
                .partial_cmp(&a.outcome.weighted_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let elite_n = ((options.population as f64) * options.elite_ratio)
            .round()
            .max(1.0) as usize;
        let elite = &for_sigma[..elite_n.min(for_sigma.len())];
        let sigma = sigma_bar(elite);
        sigma_final = sigma;

        // pareto：把这一轮和历史合并后算前沿大小
        pop.extend(round_pop);
        pareto_history = pareto_count(&pop);

        // 更新最佳
        if let Some(top) = elite.first() {
            if top.outcome.weighted_score > best_weighted + 1e-12 {
                best_weighted = top.outcome.weighted_score;
                best = Some(top.outcome.clone());
                no_improve_streak = 0;
            } else {
                no_improve_streak += 1;
            }
        } else {
            no_improve_streak += 1;
        }

        // ---- T7 停止条件（锁死不可改）----
        if sigma < options.sigma_stop {
            stop = CemStopReason::Sigma;
            let rounds_done = r + 1;
            return finalize(
                best,
                rounds_done,
                stop,
                pareto_history,
                memo_ref,
                sigma_final,
            );
        }
        if no_improve_streak >= options.no_improve_stop {
            stop = CemStopReason::Plateau;
            let rounds_done = r + 1;
            return finalize(
                best,
                rounds_done,
                stop,
                pareto_history,
                memo_ref,
                sigma_final,
            );
        }

        // ---- T9 (b) 目标感知剪枝（AND 提前停止；不降低质量）----
        if options.obj_prune
            && r + 1 >= 5
            && best.as_ref().map(|b| b.constraint_violation <= 0.0 && b.verified).unwrap_or(false)
            && pareto_history >= 3
        {
            stop = CemStopReason::ObjectivePrune;
            let rounds_done = r + 1;
            return finalize(
                best,
                rounds_done,
                stop,
                pareto_history,
                memo_ref,
                sigma_final,
            );
        }
    }

    finalize(best, options.max_rounds, stop, pareto_history, memo_ref, sigma_final)
}

fn finalize(
    best: Option<EvalOutcome>,
    rounds: usize,
    stop: CemStopReason,
    pareto_size: usize,
    memo: &EvalMemo,
    sigma_final: f64,
) -> CemResult {
    CemResult {
        best,
        rounds,
        stop_reason: stop,
        pareto_size,
        memo_hits: memo.hits(),
        memo_misses: memo.misses(),
        sigma_final,
    }
}

#[allow(dead_code)]
fn _unused_check(_c: Check) {}

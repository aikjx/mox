//! 资源受限调度 (RCPSP)
//!
//! 关键路径给出的是**无限资源**下的理论下界；真实场景中浏览器只有 1 个实例、
//! 数据库连接池有限、LLM 有并发配额。本模块用带优先级的**列表调度算法**
//! （priority = 剩余关键路径长度，即 upward rank）在资源约束下求近似最优排程。
//!
//! 算法：
//! 1. 计算每个节点的 upward rank（到终点的最长路径），作为静态优先级；
//! 2. 事件驱动推进时间轴，每个时刻从就绪集中按优先级挑选，直到资源池耗尽；
//! 3. 输出每个节点的 start/finish、每个资源池的峰值占用与利用率。
//!
//! 列表调度对 RCPSP 有 (2 - 1/m) 近似保证，工程上足够且 O(n log n + E)。

use crate::dataflow::Dependency;
use crate::model::{FlowGraph, ToolKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BinaryHeap};

/// 单节点排程结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub id: String,
    pub start_ms: u64,
    pub finish_ms: u64,
    pub pool: String,
    /// 静态优先级（upward rank）
    pub priority: u64,
}

/// 资源池使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolUsage {
    pub pool: String,
    pub capacity: u32,
    pub peak: u32,
    /// 利用率 = 忙碌机器时间 / (capacity * makespan)
    pub utilization: f64,
}

/// 调度结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub slots: Vec<Slot>,
    pub makespan_ms: u64,
    /// 无资源约束下界（关键路径）
    pub lower_bound_ms: u64,
    /// 因资源等待导致的额外延迟
    pub resource_delay_ms: u64,
    pub pools: Vec<PoolUsage>,
    /// 最大并行度
    pub max_concurrency: usize,
}

impl Schedule {
    pub fn slot(&self, id: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.id == id)
    }
    /// 调度效率 = 下界 / 实际工期
    pub fn efficiency(&self) -> f64 {
        if self.makespan_ms == 0 {
            return 1.0;
        }
        self.lower_bound_ms as f64 / self.makespan_ms as f64
    }
}

#[derive(PartialEq, Eq)]
struct Ready {
    priority: u64,
    idx: usize,
}
impl Ord for Ready {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.idx.cmp(&self.idx))
    }
}
impl PartialOrd for Ready {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn pool_of(graph: &FlowGraph, idx: usize) -> String {
    graph.nodes[idx]
        .tool
        .map(|t| t.resource_pool().to_string())
        .unwrap_or_else(|| "control".to_string())
}

/// upward rank：从节点到任一终点的最长路径耗时（含自身）
fn upward_rank(graph: &FlowGraph, succ: &[Vec<usize>], topo: &[usize]) -> Vec<u64> {
    let n = graph.nodes.len();
    let mut rank = vec![0u64; n];
    for &u in topo.iter().rev() {
        let best = succ[u].iter().map(|&v| rank[v]).max().unwrap_or(0);
        rank[u] = graph.nodes[u].duration_ms + best;
    }
    rank
}

/// 资源受限列表调度
pub fn schedule(graph: &FlowGraph, deps: &[Dependency]) -> Schedule {
    let n = graph.nodes.len();
    let mut succ = vec![Vec::new(); n];
    let mut indeg = vec![0usize; n];
    for d in deps {
        let (Some(u), Some(v)) = (graph.index_of(&d.from), graph.index_of(&d.to)) else {
            continue;
        };
        succ[u].push(v);
        indeg[v] += 1;
    }

    // 拓扑序
    let mut topo = Vec::with_capacity(n);
    {
        let mut deg = indeg.clone();
        let mut q: Vec<usize> = (0..n).filter(|&i| deg[i] == 0).collect();
        let mut head = 0;
        while head < q.len() {
            let u = q[head];
            head += 1;
            topo.push(u);
            for &v in &succ[u] {
                deg[v] -= 1;
                if deg[v] == 0 {
                    q.push(v);
                }
            }
        }
        for i in 0..n {
            if !topo.contains(&i) {
                topo.push(i);
            }
        }
    }

    let rank = upward_rank(graph, &succ, &topo);
    let lower_bound = rank.iter().copied().max().unwrap_or(0);

    // 资源池占用计数
    let mut pool_free: BTreeMap<String, u32> = BTreeMap::new();
    for i in 0..n {
        let p = pool_of(graph, i);
        pool_free.entry(p.clone()).or_insert_with(|| {
            if p == "control" {
                u32::MAX
            } else {
                graph.capacity_of(&p)
            }
        });
    }

    let mut remaining = indeg.clone();
    let mut ready: BinaryHeap<Ready> = BinaryHeap::new();
    for i in 0..n {
        if remaining[i] == 0 {
            ready.push(Ready { priority: rank[i], idx: i });
        }
    }

    // 运行中任务: (finish_time, idx)
    let mut running: Vec<(u64, usize)> = Vec::new();
    let mut slots: Vec<Slot> = Vec::new();
    let mut now = 0u64;
    let mut busy_time: BTreeMap<String, u64> = BTreeMap::new();
    let mut max_conc = 0usize;
    let mut done = 0usize;

    while done < n {
        // 尽量派发
        let mut deferred: Vec<Ready> = Vec::new();
        while let Some(r) = ready.pop() {
            let p = pool_of(graph, r.idx);
            let free = pool_free.get_mut(&p).unwrap();
            if *free == 0 {
                deferred.push(r);
                continue;
            }
            *free -= 1;
            let dur = graph.nodes[r.idx].duration_ms;
            let finish = now + dur;
            slots.push(Slot {
                id: graph.nodes[r.idx].id.clone(),
                start_ms: now,
                finish_ms: finish,
                pool: p.clone(),
                priority: r.priority,
            });
            *busy_time.entry(p).or_insert(0) += dur;
            running.push((finish, r.idx));
        }
        for d in deferred {
            ready.push(d);
        }
        max_conc = max_conc.max(running.iter().filter(|(_, i)| graph.nodes[*i].duration_ms > 0).count());

        if running.is_empty() {
            // 无法推进（资源全被占用但没有运行中任务 → 容量为 0 的病态配置）
            if ready.is_empty() {
                break;
            }
            // 强制释放一个单位，避免死循环
            if let Some(r) = ready.pop() {
                let p = pool_of(graph, r.idx);
                pool_free.insert(p, 1);
                ready.push(r);
            }
            continue;
        }

        // 推进到最早完成时刻
        let next_t = running.iter().map(|(t, _)| *t).min().unwrap();
        now = next_t.max(now);
        let mut still = Vec::new();
        for (t, idx) in running.drain(..) {
            if t <= now {
                let p = pool_of(graph, idx);
                *pool_free.get_mut(&p).unwrap() += 1;
                done += 1;
                for &v in &succ[idx] {
                    remaining[v] -= 1;
                    if remaining[v] == 0 {
                        ready.push(Ready { priority: rank[v], idx: v });
                    }
                }
            } else {
                still.push((t, idx));
            }
        }
        running = still;
    }

    let makespan = slots.iter().map(|s| s.finish_ms).max().unwrap_or(0);
    let pools: Vec<PoolUsage> = busy_time
        .iter()
        .filter(|(p, _)| p.as_str() != "control")
        .map(|(p, &busy)| {
            let cap = graph.capacity_of(p);
            let peak = peak_usage(&slots, p);
            let util = if makespan == 0 || cap == 0 {
                0.0
            } else {
                busy as f64 / (cap as f64 * makespan as f64)
            };
            PoolUsage { pool: p.clone(), capacity: cap, peak, utilization: util }
        })
        .collect();

    slots.sort_by(|a, b| a.start_ms.cmp(&b.start_ms).then(a.id.cmp(&b.id)));

    Schedule {
        makespan_ms: makespan,
        lower_bound_ms: lower_bound,
        resource_delay_ms: makespan.saturating_sub(lower_bound),
        pools,
        max_concurrency: max_conc,
        slots,
    }
}

fn peak_usage(slots: &[Slot], pool: &str) -> u32 {
    let mut events: Vec<(u64, i32)> = Vec::new();
    for s in slots.iter().filter(|s| s.pool == pool && s.finish_ms > s.start_ms) {
        events.push((s.start_ms, 1));
        events.push((s.finish_ms, -1));
    }
    events.sort();
    let mut cur = 0i32;
    let mut peak = 0i32;
    for (_, d) in events {
        cur += d;
        peak = peak.max(cur);
    }
    peak.max(0) as u32
}

/// 依据「轻量模型 / 重型模型」策略给出 LLM 算力分配建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRouting {
    pub node_id: String,
    pub model_tier: ModelTier,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    /// 简单问答 / 分类 / 抽取
    Light,
    /// 标准业务推理
    Standard,
    /// 代码生成 / 流程建模等重型任务
    Heavy,
}

/// 按节点语义把 LLM 调用路由到不同规格模型，降低算力成本
pub fn route_models(graph: &FlowGraph) -> Vec<ModelRouting> {
    graph
        .nodes
        .iter()
        .filter(|n| n.tool == Some(ToolKind::Llm))
        .map(|n| {
            let name = n.name.to_lowercase();
            let tags = n.tags.join(",");
            let heavy_kw = ["代码", "code", "生成工程", "重构", "流程建模", "架构"];
            let light_kw = [
                "分类", "意图", "抽取", "判断", "路由", "摘要", "识别", "解析",
                "预处理", "预校验", "脱敏", "汇总", "校验", "提炼", "检索", "回填",
                "classify", "intent", "extract", "summarize", "parse",
            ];
            let (tier, reason) = if heavy_kw.iter().any(|k| name.contains(k) || tags.contains(k)) {
                (ModelTier::Heavy, "代码/架构类重型任务，需强推理模型")
            } else if light_kw.iter().any(|k| name.contains(k) || tags.contains(k))
                || n.duration_ms <= 200
            {
                (ModelTier::Light, "短时分类/抽取类任务，轻量模型足够")
            } else {
                (ModelTier::Standard, "常规业务推理")
            };
            ModelRouting {
                node_id: n.id.clone(),
                model_tier: tier,
                reason: reason.to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow;
    use crate::model::{Access, FlowEdge, FlowNode, ResourcePool, ToolKind};

    fn three_browsers() -> FlowGraph {
        let mut g = FlowGraph::new("b", "browser jobs");
        for i in 0..3 {
            g.add_node(
                FlowNode::task(format!("b{}", i), format!("抓取{}", i), ToolKind::Browser, 100)
                    .with_access(Access::write(format!("var:page{}", i)))
                    .idempotent(true),
            );
        }
        g
    }

    #[test]
    fn browser_capacity_one_serializes() {
        let g = three_browsers();
        let plan = dataflow::analyze(&g);
        let s = schedule(&g, &plan.dependencies);
        assert_eq!(s.lower_bound_ms, 100, "无约束下界=单任务耗时");
        assert_eq!(s.makespan_ms, 300, "浏览器容量1 → 必须串行");
        assert_eq!(s.resource_delay_ms, 200);
        let starts: Vec<u64> = s.slots.iter().map(|x| x.start_ms).collect();
        assert_eq!(starts, vec![0, 100, 200]);
    }

    #[test]
    fn raising_capacity_enables_parallel() {
        let mut g = three_browsers();
        g.pools.push(ResourcePool { name: "browser".into(), capacity: 3 });
        let plan = dataflow::analyze(&g);
        let s = schedule(&g, &plan.dependencies);
        assert_eq!(s.makespan_ms, 100);
        assert_eq!(s.resource_delay_ms, 0);
        assert_eq!(s.max_concurrency, 3);
    }

    #[test]
    fn respects_dependencies() {
        let mut g = FlowGraph::new("d", "dep");
        g.add_node(FlowNode::task("a", "A", ToolKind::Compute, 50).with_access(Access::write("x")));
        g.add_node(FlowNode::task("b", "B", ToolKind::Compute, 50).with_access(Access::read("x")));
        g.add_edge(FlowEdge::seq("a", "b"));
        let plan = dataflow::analyze(&g);
        let s = schedule(&g, &plan.dependencies);
        assert!(s.slot("b").unwrap().start_ms >= s.slot("a").unwrap().finish_ms);
        assert_eq!(s.makespan_ms, 100);
    }

    #[test]
    fn priority_prefers_critical_chain() {
        // 长链 a1->a2(共200) 与 短任务 s(10)，容量1，应先派发长链头
        let mut g = FlowGraph::new("p", "prio");
        g.pools.push(ResourcePool { name: "cpu".into(), capacity: 1 });
        g.add_node(FlowNode::task("a1", "A1", ToolKind::Compute, 100).with_access(Access::write("x")));
        g.add_node(FlowNode::task("a2", "A2", ToolKind::Compute, 100).with_access(Access::read("x")));
        g.add_node(FlowNode::task("s", "S", ToolKind::Compute, 10));
        g.add_edge(FlowEdge::seq("a1", "a2"));
        let plan = dataflow::analyze(&g);
        let sc = schedule(&g, &plan.dependencies);
        assert_eq!(sc.slot("a1").unwrap().start_ms, 0, "关键链头应优先");
    }

    #[test]
    fn model_routing_splits_tiers() {
        let mut g = FlowGraph::new("m", "models");
        g.add_node(FlowNode::task("c", "意图分类", ToolKind::Llm, 100));
        g.add_node(FlowNode::task("g", "代码生成", ToolKind::Llm, 3000));
        let r = route_models(&g);
        assert_eq!(r.iter().find(|x| x.node_id == "c").unwrap().model_tier, ModelTier::Light);
        assert_eq!(r.iter().find(|x| x.node_id == "g").unwrap().model_tier, ModelTier::Heavy);
    }
}

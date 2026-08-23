//! 业务流程图中间表示 (Flow IR)
//!
//! 这是「流程图 + 关系网」双载体的统一数据模型。
//! 所有优化算法（并行化、关键路径、冲突检测、资源调度、代码生成）
//! 都以本模块的 `FlowGraph` 作为唯一输入 / 输出，保证「一处修改，全链路联动」。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// 流程节点语义类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// 流程起点
    Start,
    /// 流程终点
    End,
    /// 普通任务（会绑定工具）
    Task,
    /// 排他判断分支（if / match）
    Decision,
    /// 并行网关：开
    ParallelFork,
    /// 并行网关：合
    ParallelJoin,
    /// 循环入口
    LoopStart,
    /// 循环出口
    LoopEnd,
    /// 异常捕获 / 校验节点（前置拦截器）
    Guard,
    /// 子流程引用（可复用 Skill 模板）
    SubFlow,
}

impl NodeKind {
    /// 是否为可执行的实体工作节点（参与调度与关键路径计算）
    pub fn is_executable(&self) -> bool {
        matches!(self, NodeKind::Task | NodeKind::SubFlow | NodeKind::Guard)
    }

    /// 是否为纯控制节点（零耗时，仅约束拓扑）
    pub fn is_control(&self) -> bool {
        !self.is_executable()
    }
}

/// 工具类别 —— 决定互斥资源与冲突规则
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// 纯计算 / LLM 推理，无外部副作用
    Compute,
    /// 大模型调用（计费、限流）
    Llm,
    /// 文件读写（Excel / CSV / 文档）
    File,
    /// 浏览器 RPA
    Browser,
    /// 数据库
    Database,
    /// HTTP / 三方接口
    Http,
    /// 桌面自动化 / 系统命令
    Shell,
    /// 人工审批
    Human,
}

impl ToolKind {
    /// 该工具默认独占的资源池名（用于资源受限调度）
    pub fn resource_pool(&self) -> &'static str {
        match self {
            ToolKind::Compute => "cpu",
            ToolKind::Llm => "llm",
            ToolKind::File => "io",
            ToolKind::Browser => "browser",
            ToolKind::Database => "db",
            ToolKind::Http => "net",
            ToolKind::Shell => "shell",
            ToolKind::Human => "human",
        }
    }
}

/// 数据访问模式 —— 数据流依赖推断的基础
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    Read,
    Write,
    /// 读改写（等价于 Read + Write，且要求事务原子性）
    ReadWrite,
}

impl AccessMode {
    pub fn reads(&self) -> bool {
        matches!(self, AccessMode::Read | AccessMode::ReadWrite)
    }
    pub fn writes(&self) -> bool {
        matches!(self, AccessMode::Write | AccessMode::ReadWrite)
    }
}

/// 一次具体的资源访问声明
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Access {
    /// 资源标识：表名 / 文件路径 / URL / 变量名
    pub resource: String,
    pub mode: AccessMode,
}

impl Access {
    pub fn read(r: impl Into<String>) -> Self {
        Self { resource: r.into(), mode: AccessMode::Read }
    }
    pub fn write(r: impl Into<String>) -> Self {
        Self { resource: r.into(), mode: AccessMode::Write }
    }
    pub fn rw(r: impl Into<String>) -> Self {
        Self { resource: r.into(), mode: AccessMode::ReadWrite }
    }
}

/// 合规 / 业务规则等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    /// 阻断级：必须在生成代码前修复
    Blocking,
}

/// 业务专家规则（政务 / 法院 / 等保等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertRule {
    pub id: String,
    pub description: String,
    pub severity: Severity,
    /// 命中该规则的资源前缀（如 "db.citizen_" 表示公民敏感表）
    #[serde(default)]
    pub resource_prefixes: Vec<String>,
    /// 命中该规则的工具类别
    #[serde(default)]
    pub tool_kinds: Vec<ToolKind>,
    /// 满足规则所必须存在的前置 Guard 标签（如 "desensitize"）
    #[serde(default)]
    pub required_guard_tags: Vec<String>,
}

/// 流程节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    /// 绑定的工具（控制节点可为 None）
    #[serde(default)]
    pub tool: Option<ToolKind>,
    /// 预估耗时（毫秒），用于关键路径与调度
    #[serde(default)]
    pub duration_ms: u64,
    /// 数据访问声明
    #[serde(default)]
    pub accesses: Vec<Access>,
    /// 语义标签（Guard 用 tag 声明自己校验了什么，如 "path_check"/"desensitize"）
    #[serde(default)]
    pub tags: Vec<String>,
    /// 是否事务性节点（数据库事务边界）
    #[serde(default)]
    pub transactional: bool,
    /// 是否可重试（幂等）
    #[serde(default)]
    pub idempotent: bool,
    /// 任意扩展属性
    #[serde(default)]
    pub props: BTreeMap<String, String>,
}

impl FlowNode {
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            tool: None,
            duration_ms: 0,
            accesses: Vec::new(),
            tags: Vec::new(),
            transactional: false,
            idempotent: false,
            props: BTreeMap::new(),
        }
    }

    pub fn task(id: impl Into<String>, name: impl Into<String>, tool: ToolKind, duration_ms: u64) -> Self {
        let mut n = Self::new(id, name, NodeKind::Task);
        n.tool = Some(tool);
        n.duration_ms = duration_ms;
        n
    }

    pub fn with_access(mut self, a: Access) -> Self {
        self.accesses.push(a);
        self
    }

    pub fn with_tag(mut self, t: impl Into<String>) -> Self {
        self.tags.push(t.into());
        self
    }

    pub fn transactional(mut self, v: bool) -> Self {
        self.transactional = v;
        self
    }

    pub fn idempotent(mut self, v: bool) -> Self {
        self.idempotent = v;
        self
    }

    /// 读集合
    pub fn read_set(&self) -> BTreeSet<&str> {
        self.accesses
            .iter()
            .filter(|a| a.mode.reads())
            .map(|a| a.resource.as_str())
            .collect()
    }

    /// 写集合
    pub fn write_set(&self) -> BTreeSet<&str> {
        self.accesses
            .iter()
            .filter(|a| a.mode.writes())
            .map(|a| a.resource.as_str())
            .collect()
    }
}

/// 边的语义
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// 顺序控制流
    Sequence,
    /// 条件分支（带条件表达式）
    Conditional,
    /// 异常流（catch 边）
    Exception,
    /// 由数据流分析自动推断出的隐式依赖
    InferredData,
    /// 资源互斥序：由冲突修复注入的硬约束，数据流分析**不得**剪除
    Mutex,
}

impl EdgeKind {
    /// 该边是否为不可剪除的硬约束
    pub fn is_hard(&self) -> bool {
        matches!(self, EdgeKind::Conditional | EdgeKind::Exception | EdgeKind::Mutex)
    }
}

/// 流程边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    #[serde(default = "default_edge_kind")]
    pub kind: EdgeKind,
    /// 条件表达式（Conditional 边）
    #[serde(default)]
    pub condition: Option<String>,
}

fn default_edge_kind() -> EdgeKind {
    EdgeKind::Sequence
}

impl FlowEdge {
    pub fn seq(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self { from: from.into(), to: to.into(), kind: EdgeKind::Sequence, condition: None }
    }
    pub fn cond(from: impl Into<String>, to: impl Into<String>, expr: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            kind: EdgeKind::Conditional,
            condition: Some(expr.into()),
        }
    }
    pub fn exception(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self { from: from.into(), to: to.into(), kind: EdgeKind::Exception, condition: None }
    }
    /// 资源互斥边：强制 from 先于 to，冲突修复专用
    pub fn mutex(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self { from: from.into(), to: to.into(), kind: EdgeKind::Mutex, condition: None }
    }
}

/// 资源池容量（并发上限）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePool {
    pub name: String,
    pub capacity: u32,
}

/// 完整业务流程图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowGraph {
    pub id: String,
    pub name: String,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    /// 资源池容量配置（缺省容量见 `capacity_of`）
    #[serde(default)]
    pub pools: Vec<ResourcePool>,
    /// 绑定的业务专家规则
    #[serde(default)]
    pub rules: Vec<ExpertRule>,
}

impl FlowGraph {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            pools: Vec::new(),
            rules: Vec::new(),
        }
    }

    pub fn add_node(&mut self, n: FlowNode) -> &mut Self {
        self.nodes.push(n);
        self
    }

    pub fn add_edge(&mut self, e: FlowEdge) -> &mut Self {
        self.edges.push(e);
        self
    }

    pub fn node(&self, id: &str) -> Option<&FlowNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: &str) -> Option<&mut FlowNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// 资源池容量，未显式配置时给出安全缺省
    pub fn capacity_of(&self, pool: &str) -> u32 {
        if let Some(p) = self.pools.iter().find(|p| p.name == pool) {
            return p.capacity.max(1);
        }
        match pool {
            // 浏览器实例默认单例，避免多实例抢占
            "browser" => 1,
            // 数据库连接池
            "db" => 4,
            "llm" => 2,
            "io" => 4,
            "net" => 8,
            "shell" => 1,
            "human" => 1,
            _ => 8,
        }
    }

    /// 邻接表（后继）
    pub fn successors(&self) -> Vec<Vec<usize>> {
        let n = self.nodes.len();
        let mut adj = vec![Vec::new(); n];
        // 一次 O(n) 构建索引，避免每条边 2×O(n) 的 position 扫描
        let mut idx: HashMap<&str, usize> = HashMap::with_capacity(n);
        for (i, nd) in self.nodes.iter().enumerate() {
            idx.insert(nd.id.as_str(), i);
        }
        for e in &self.edges {
            if let (Some(&a), Some(&b)) = (idx.get(e.from.as_str()), idx.get(e.to.as_str())) {
                adj[a].push(b);
            }
        }
        adj
    }

    /// 邻接表（前驱）
    pub fn predecessors(&self) -> Vec<Vec<usize>> {
        let n = self.nodes.len();
        let mut adj = vec![Vec::new(); n];
        let mut idx: HashMap<&str, usize> = HashMap::with_capacity(n);
        for (i, nd) in self.nodes.iter().enumerate() {
            idx.insert(nd.id.as_str(), i);
        }
        for e in &self.edges {
            if let (Some(&a), Some(&b)) = (idx.get(e.from.as_str()), idx.get(e.to.as_str())) {
                adj[b].push(a);
            }
        }
        adj
    }

    /// 拓扑排序（Kahn）。存在环时返回 Err(环上节点 id)
    pub fn topo_order(&self) -> Result<Vec<usize>, Vec<String>> {
        let n = self.nodes.len();
        let succ = self.successors();
        let mut indeg = vec![0usize; n];
        for succ_u in &succ {
            for &v in succ_u {
                indeg[v] += 1;
            }
        }
        let mut queue: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
        queue.sort_unstable();
        let mut out = Vec::with_capacity(n);
        let mut head = 0;
        while head < queue.len() {
            let u = queue[head];
            head += 1;
            out.push(u);
            for &v in &succ[u] {
                indeg[v] -= 1;
                if indeg[v] == 0 {
                    queue.push(v);
                }
            }
        }
        if out.len() != n {
            let cyc: Vec<String> = (0..n)
                .filter(|i| !out.contains(i))
                .map(|i| self.nodes[i].id.clone())
                .collect();
            return Err(cyc);
        }
        Ok(out)
    }

    /// 传递闭包（可达性矩阵），位图压缩。用于「是否已存在顺序约束」判定。
    pub fn reachability(&self) -> Reachability {
        let n = self.nodes.len();
        let words = n.div_ceil(64).max(1);
        let mut bits = vec![0u64; n * words];
        let succ = self.successors();
        let order = match self.topo_order() {
            Ok(o) => o,
            // 有环时退化为朴素顺序，可达性仍收敛（多轮松弛）
            Err(_) => (0..n).collect(),
        };
        for &u in order.iter().rev() {
            for &v in &succ[u] {
                // u 可达 v
                bits[u * words + v / 64] |= 1u64 << (v % 64);
                // 并入 v 的可达集
                for w in 0..words {
                    bits[u * words + w] |= bits[v * words + w];
                }
            }
        }
        Reachability { words, bits }
    }
}

/// 可达性位图
#[derive(Debug, Clone)]
pub struct Reachability {
    words: usize,
    bits: Vec<u64>,
}

impl Reachability {
    /// u 能否沿有向边到达 v
    pub fn reaches(&self, u: usize, v: usize) -> bool {
        self.bits[u * self.words + v / 64] & (1u64 << (v % 64)) != 0
    }

    /// 两节点是否「无序」（可并行的必要条件）
    pub fn concurrent(&self, u: usize, v: usize) -> bool {
        u != v && !self.reaches(u, v) && !self.reaches(v, u)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear() -> FlowGraph {
        let mut g = FlowGraph::new("g", "linear");
        g.add_node(FlowNode::new("s", "start", NodeKind::Start));
        g.add_node(FlowNode::task("a", "A", ToolKind::File, 100));
        g.add_node(FlowNode::task("b", "B", ToolKind::Database, 200));
        g.add_node(FlowNode::new("e", "end", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "a"));
        g.add_edge(FlowEdge::seq("a", "b"));
        g.add_edge(FlowEdge::seq("b", "e"));
        g
    }

    #[test]
    fn topo_is_linear() {
        let g = linear();
        let order = g.topo_order().unwrap();
        let ids: Vec<&str> = order.iter().map(|&i| g.nodes[i].id.as_str()).collect();
        assert_eq!(ids, vec!["s", "a", "b", "e"]);
    }

    #[test]
    fn cycle_detected() {
        let mut g = linear();
        g.add_edge(FlowEdge::seq("b", "a"));
        assert!(g.topo_order().is_err());
    }

    #[test]
    fn reachability_transitive() {
        let g = linear();
        let r = g.reachability();
        let s = g.index_of("s").unwrap();
        let e = g.index_of("e").unwrap();
        let a = g.index_of("a").unwrap();
        assert!(r.reaches(s, e));
        assert!(r.reaches(a, e));
        assert!(!r.reaches(e, s));
        assert!(!r.concurrent(s, e));
    }
}

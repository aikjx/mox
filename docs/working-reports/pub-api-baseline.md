# 璇玑 15 Rust Crate · Pub API 基线

> 版本：`Pub-API Baseline v2.0 (2026-09 企业级归一化 · 冻结态)`
>
> —— **冻结说明**：本基线列出的 pub 符号为外部契约，删除/重命名/签名变更**必须走架构评审**并同步：
> ① `rust-binding-contract.md` §3 映射；② `rust_crate_bindings_e2e.js` 断言；③ 15× README；④ 图谱 AtlasDomainNode 关系变更脚本。
>
> 本基线以「crate 目录名 → pub 符号清单」列出；每个 crate 选取代表性 pub 符号（struct / trait / fn / const），保证 `≥ 3` 条/ crate（对应 README `对外 API` 段）。

---

## 0. AIS 分层索引

```
L1 基础设施   ── mox-common-meta（AisLayer / CrateMeta · 所有 crate 只读依赖）
L2 平台网关   ── runtime（HTTP / sidecar / 服务组装）
L3 应用协同   ── primiflow-fusion（跨域流程组合）
L4 领域服务   ── mox-system, mox-expert, ai-agent, hermes-flow-bridge,
│                 business-catalog, template-market
L5 抽象适配   ── flow-ai（FlowGraph 抽象，跨领域复用）
L6 内核算子   ── operator-core, operator-wasm（纯算子，0 I/O）
L7 工作流     ── primiflow-core（状态机 / 节点 / 迁移）
L8 数据治理   ── kg-hub（图谱 CRUD + 索引）
L9 算法与优化 ── graph-algorithms（7 大算法）, optimizer（启发式 / 禁忌搜索）
L?            ── （L10 UX 暂由 Node.js frontend 承载，不属 Rust crate 层）
```

---

## 1. `mox-common-meta` (L1Infra · CRATE_ID: `a1c2b3d4-…`)
```rust
pub enum AisLayer { L1Infra, L2Platform, L3Application, L4Services, L5Abstraction, L6Kernel, L7Flow, L8Data, L9Algo, L10UX, Unknown }
pub struct CrateMeta { pub id: &'static str, pub name: &'static str, pub version: &'static str, pub layer: AisLayer, pub owner: &'static str }
```

## 2. `operator-core` (L6Kernel · CRATE_ID: `b8e1f2a3-…`)
```rust
pub mod resource;     pub mod topology;     pub mod plan;
pub mod schedule;     pub mod cost;         pub mod risk;
pub mod simulation;   pub mod dataflow;     pub mod visualization;
pub mod kernel_ext;   // enum F {...} + Kernel API

// 代表性 pub
pub struct OperatorCost    { pub cpu: u32, pub mem: u32, pub net: u32 }
pub struct TopologyEdge    { pub from: NodeId, pub to: NodeId, pub latency_ms: u64 }
pub fn optimize_placement(nodes: &[Node], deps: &[TopologyEdge]) -> Plan
```

## 3. `graph-algorithms` (L9Algo · CRATE_ID: `cf1a2b3d-…`)
```rust
// 7 大导出算法（T12 F1-F8 对应实现）
pub fn degree_centrality<N, E>(g: &Graph<N, E>) -> HashMap<NodeIndex, f64>;     // F2
pub fn pagerank<N, E>(g: &Graph<N, E>, d: f64, iter: usize) -> HashMap<NodeIndex, f64>; // F3
pub fn betweenness_centrality<N, E>(g: &Graph<N, E>) -> HashMap<NodeIndex, f64>;          // F4
pub fn closeness_centrality<N, E>(g: &Graph<N, E>) -> HashMap<NodeIndex, f64>;            // F5
pub fn community_detection_cnm<N, E>(g: &Graph<N, E>) -> Vec<Vec<NodeIndex>>;             // F6
pub fn conservation_law_check<N>(nodes: &[FlowNode<N>]) -> ConservationReport;            // F7
pub fn intent_detection_top1(query: &str, kb: &[IntentDef]) -> Option<&IntentDef>;        // F8
pub struct Graph<N, E> { /* stable petgraph-backed */ }
```

## 4. `primiflow-core` (L7Flow · CRATE_ID: `d0a89172-…`)
```rust
pub struct Workflow<K, V>    { /* DAG of Nodes */ }
pub struct Node<K, V>        { pub id: K, pub action: Action<K, V>, pub deps: HashSet<K> }
pub enum Action<K, V>        { Task(Box<dyn Task<K,V>>), Choice, Map(Box<...>), Parallel(Vec<K>) }
pub trait Task<K, V>         { fn run(&self, ctx: &FlowCtx<'_, K, V>) -> FlowResult<V>; }
pub fn execute_dag<K: Hash+Eq, V>(wf: &Workflow<K,V>, scope: &mut Scope<K,V>) -> FlowResult<()>
```

## 5. `kg-hub` (L8Data · CRATE_ID: `e92f1a3b-…`)
```rust
pub struct AtlasGraph { /* Domain ↔ Requirement ↔ Code ↔ Engine 图谱 */ }
pub struct AtlasNode  { pub id: String, pub kind: NodeKind, pub attrs: HashMap<String, String> }
pub enum   NodeKind   { Domain, Requirement, Code, Engine, Algo, TestCase, Persona }
impl AtlasGraph {
    pub fn new() -> Self
    pub fn add_node(&mut self, node: AtlasNode)
    pub fn add_edge(&mut self, from: &str, to: &str, rel: EdgeRel)
    pub fn domains(&self) -> impl Iterator<Item = &AtlasNode>
    pub fn outgoing(&self, id: &str) -> impl Iterator<Item = (&AtlasNode, EdgeRel)>
}
```

## 6. `flow-ai` (L5Abstraction · CRATE_ID: `f5e4d3c2-…`)
```rust
pub mod model;  pub mod routing;  pub mod template;  pub mod engine;
pub use model::{FlowGraph, FlowNode, FlowEdge, NodeKind, Criticality};
pub use template::{FlowTemplate, TemplateId, render_template};
pub use engine::FlowEngine;
```

## 7. `business-catalog` (L4Services · CRATE_ID: `1a2b3c4d-…`)
```rust
pub struct BusinessDomain { pub id: DomainId, pub name: String, pub capabilities: Vec<Capability> }
pub struct Capability     { pub id: CapId, pub requires: Vec<CapId> }
pub struct Catalog        { domains: HashMap<DomainId, BusinessDomain> }
impl Catalog {
    pub fn new() -> Self
    pub fn register(&mut self, d: BusinessDomain)
    pub fn resolve_requirements(&self, cap: &CapId) -> Vec<DomainId>  // 依赖解析
}
pub trait ExpertTrait { fn advice(&self, cap: &CapId) -> Option<String>; } // DIP 抽象（不依赖 concrete）
```

## 8. `mox-system` (L4Services · CRATE_ID: `2b3c4d5e-…`)
```rust
pub mod domain_traits;  pub mod business_rules;  pub mod sqlite_provider;  pub mod persistence_provider;
// DIP 三大 trait（ARC-02/03）
pub use domain_traits::{PermissionServiceTrait, TaskServiceTrait, ExpertServiceTrait};
pub struct EffectivePermissions { pub user_id: String, pub roles: Vec<String>, pub caps: Vec<String> }
```

## 9. `optimizer` (L9Algo · CRATE_ID: `3c4d5e6f-…`)
```rust
pub struct OptimizerConfig { pub max_iter: usize, pub tabu_len: usize, pub target_cost: f64 }
pub struct OptimizerResult { pub plan: Plan, pub cost: f64, pub iter_used: usize }
pub fn tabu_search(problem: &Problem, cfg: OptimizerConfig) -> OptimizerResult;
pub fn greedy_baseline(problem: &Problem) -> OptimizerResult;  // 与 T12 F1 对账
```

## 10. `mox-expert` (L4Services · CRATE_ID: `4d5e6f7a-…`)
```rust
pub mod expert_traits;  pub mod gov_engine;  pub mod viz;  pub mod http;  // feature=http 时
pub struct VizBundle { pub svg: String, pub highlights: Vec<(NodeId, HlKind)> }
pub enum HlKind { CriticalPath, Conflict, Reuse, AlgoVerified }
pub trait ExpertConsultant { fn consult(&self, graph: &FlowGraph) -> Vec<ExpertDecision>; }
pub struct ExpertDecision { pub target_node: NodeId, pub reason: Option<String>, pub veto: bool }
```

## 11. `primiflow-fusion` (L3Application · CRATE_ID: `5e6f7a8b-…`)
```rust
pub mod fusion;  pub mod cross_domain;
pub fn fuse_workflows<A, B>(a: &Workflow<A,A>, b: &Workflow<B,B>) -> Workflow<Either<A,B>, Either<A,A>>
pub struct CrossDomainCtx<'a> { pub left: &'a Catalog, pub right: &'a Catalog }
```

## 12. `ai-agent` (L4Services · CRATE_ID: `6f7a8b9c-…`)
```rust
pub mod engine;  pub mod tools;  pub mod planner;
pub use engine::AgentEngine;
pub use tools::{IntentRouterTool, DatabaseTool, AlgoInvokeTool, ReportGenTool};
pub struct AgentOutput { pub intent: Option<String>, pub trace: Vec<AgentStep> }
// DatabaseTool::new() 为三级降级入口（ENG-04）
```

## 13. `template-market` (L4Services · CRATE_ID: `7a8b9c0d-…`)
```rust
pub struct FlowTemplateItem { pub id: TemplateId, pub name: String, pub graph: FlowGraph }
pub struct TemplateMarket       { items: HashMap<TemplateId, FlowTemplateItem> }
impl TemplateMarket {
    pub fn install_defaults(&mut self)
    pub fn find_by_goal(&self, goal: &str) -> Vec<&FlowTemplateItem>  // F6 路由复用
}
```

## 14. `hermes-flow-bridge` (L4Services · CRATE_ID: `9bfaf43b-…`)
```rust
pub mod bridge;  pub mod hooks;  pub mod recorder;  pub mod router;  pub mod plugin;
pub use bridge::{optimize_session, spawn_optimizer};       // spawn_optimizer = catch_unwind+backoff（ENG-05）
pub use hooks::{on_tool_execution, on_tool_request};
pub use plugin::FlowBridgePlugin;
pub use state::{BridgeState, GateState};
// live: push_loop（feature=live；timeout+退避，ENG-06）
```

## 15. `operator-wasm` (L6Kernel · CRATE_ID: `9c0d1e2f-…`)
```rust
pub struct WasmOperator { pub bytes: Vec<u8> }
impl WasmOperator {
    pub fn new(bytes: Vec<u8>) -> Self
    pub fn run_cost(&self, input: &[u8]) -> Result<OperatorCost, WasmErr>
}
```

## 16. `runtime` (L2Platform · CRATE_ID: `ab1c2d3e-…`)
```rust
pub mod bootstrap;  pub mod sidecar;  pub mod routes;  pub mod graph_bridge;
// 企业级入口
pub async fn bootstrap(cfg: RuntimeConfig) -> std::io::Result<()>;
pub struct RuntimeConfig { pub bind: SocketAddr, pub sidecar_internal: bool }
// governance feature：权限 + 多租户治理子命令
#[cfg(feature = "governance")] pub mod governance;
```

---

## 附录 A · API 兼容性政策

| 变更性质            | 处理方式                                                               |
|---------------------|------------------------------------------------------------------------|
| 新增 pub struct/fn  | 向后兼容；README 同步；下一个 baseline 版本纳入                          |
| pub fn 签名参数新增（末尾 Default） | 向后兼容；不强制回归                                               |
| pub fn 重命名 / 删除 / 位置参数改序 | **Breaking**：必须提交架构评审 + 更新本基线 + README + E2E 测试 + 图谱迁移脚本 |
| CRATE_ID 变更       | **绝对禁止**（UUID 一次性冻结）；若确需变更 → 新建 crate + 弃用旧 UUID  |

## 附录 B · 非公开符号声明

以下内容**不**在基线覆盖范围，可自由 refactor（但仍需通过 Clippy / 测试）：
- `pub(crate)` 及更小可见性符号；
- `#[cfg(test)]` 下所有符号；
- `feature=hermes` / `feature=live` gated 模块（非默认）——以单独 Feature Contract 维护。

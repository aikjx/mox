# 璇玑 · mox 模块化系统架构处理工具流程图

> 企业级 · 归一化 · mox 模块化系统架构最优算法处理内核
> 定位：在 `flow-ai` 已验证的流程图优化引擎之上，叠加「多专家并行诊断 + 归一化裁决 + 企业治理」三层，
> 让业务流程图 / 算法流程图 / 权限流程图 / 资源流程图在**同一套归一化 IR** 上被mox 模块化系统架构维度联合优化。

---

## 0. 设计原则（不可妥协）

1. **归一化（Single Source of Truth）**
   四种流程图在内存里是**同一个 `FlowGraph`**，差异只体现在节点/边的 `Dimension` 着色与约束上。
   不存在"四套图各自优化再合并"——那会产生数据孤岛与冲突。mox 模块化系统架构视角 = 同一张图被七个镜头同时审视。

2. **mox 模块化系统架构（Full-Dimension）**
   每个节点同时可被业务、算法、权限、资源、安全、数据、可观测七个维度评估；优化结论必须**同时满足mox 模块化系统架构约束**，不做单维最优而牺牲其他维。

3. **最优算法处理（Optimal）**
   最终求解交给 `flow-ai` 已验证的算法栈（冒险分析并行化 + CPM 关键路径 + RCPSP 资源调度 + 冲突自动修复 + Dijkstra 关系网最短路）。本层不重造算法，只负责把mox 模块化系统架构约束**正确翻译**成引擎能吃的输入。

4. **可治理（Governable, Enterprise-grade）**
   多租户隔离、RBAC 权限、不可篡改审计、流程版本与审批、SLA/成本预算——任何"出码/执行"动作前必须过治理闸门。

---

## 1. 归一化 IR（在 flow-ai 上扩展，不另起炉灶）

复用 `flow-ai` 的 `FlowGraph / FlowNode / FlowEdge / ExpertRule / ResourcePool`，
仅增加**维度元数据**与**企业上下文**两层薄封装：

```rust
/// 七个优化维度（镜头）
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dimension {
    Business,       // 业务正确性 / 领域规则
    Algorithm,      // 计算复杂度 / 关键路径瓶颈
    Permission,     // RBAC / 合规 / 脱敏
    Resource,       // 算力 / 连接池 / 并发容量
    Security,       // 注入 / PII 泄露 / 越权
    Data,           // 数据血缘 / schema / 一致性
    Observability,  // 埋点 / 追踪 / 告警
}

/// 节点上的维度着色：同一物理节点可被多维度同时评估
#[derive(Clone, Serialize, Deserialize)]
pub struct DimensionTag {
    pub dimension: Dimension,
    pub owner_expert: ExpertId,
    pub policy_refs: Vec<PolicyId>,   // 关联的企业策略
    pub weight: f64,                  // 该维度在此节点的相对重要性
}

// FlowNode 增加字段：
//   pub dimensions: Vec<DimensionTag>
```

归一化关键：**物理节点唯一，维度只是标签**。这样"改一个节点，mox 模块化系统架构同步"天然成立，无需任何跨图同步逻辑。

---

## 2. 璇玑（Expert Council）

每位专家是一个无状态 trait 实现，输入归一化上下文，输出**观点（Opinion）**。
专家之间互不调用、互不依赖，由流水线并行派发。

```rust
pub trait Expert: Send + Sync {
    fn id(&self) -> ExpertId;
    fn dimensions(&self) -> &[Dimension];
    /// 只读分析，返回约束/风险/建议/评分
    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion;
}

pub struct ExpertContext<'a> {
    pub flow: &'a FlowGraph,
    pub tenant: &'a Tenant,
    pub principal: &'a Principal,
    pub policies: &'a [Policy],
    pub prior_opinions: &'a [ExpertOpinion], // 供专家参考其他维度（只读）
    pub quota: &'a ResourceQuota,
}

pub struct ExpertOpinion {
    pub expert: ExpertId,
    pub constraints: Vec<Constraint>,  // 必须落地的硬/软约束
    pub risks: Vec<Risk>,              // 发现的问题（含修复建议）
    pub suggestions: Vec<Suggestion>, // 优化提议（非强制）
    pub score: f64,                    // 本维度健康分 0..1
    pub metrics: HashMap<String, f64>,
}

/// 约束是归一化合并的最小单元
pub enum Constraint {
    MustOrder(EdgeRef),         // 强制顺序
    MustGuard(NodeRef, Vec<String>), // 前置拦截（如脱敏/鉴权）
    MustSerialize(EdgeRef),     // 互斥串行（资源/事务）
    MustIsolate(NodeRef),       // 沙箱/隔离执行
    MustAudit(NodeRef),         // 强制审计点
    ResourceCap(PoolName, u32), // 资源池上限（来自租户配额）
    Compliance(PolicyId),       // 合规策略绑定
}

pub struct Risk {
    pub severity: Severity,     // Info | Warning | Blocking
    pub nodes: Vec<NodeRef>,
    pub dimension: Dimension,
    pub message: String,
    pub remediation: Option<String>,
}

pub enum Suggestion {
    Parallelize, Cache, Split, Merge, Offload(ModelTier), Retry, Debounce,
}
```

### 2.1 七位专家职责

| 专家 | 维度 | 核心职责 | 典型产出 |
|------|------|---------|---------|
| 业务专家 | Business | 领域规则校验、分支完整性、失败兜底 | 缺 else 警告、必走审批分支 |
| 算法专家 | Algorithm | 关键路径瓶颈、复杂度、缓存命中 | 建议缓存/拆分、标记 O(n²) 节点 |
| 权限专家 | Permission | RBAC、越权、合规脱敏（政务等保） | `MustGuard(desensitize)`、Blocking 合规 |
| 资源专家 | Resource | 算力/连接池/并发，翻译租户配额 | `ResourceCap(browser,1)`、`MustSerialize` |
| 安全专家 | Security | 注入、PII 外发、提示词越狱 | `MustIsolate`、Blocking 泄露风险 |
| 数据专家 | Data | 血缘、schema 漂移、幂等 | `MustOrder`（血缘保序）、幂等建议 |
| 可观测专家 | Observability | 埋点/追踪/告警覆盖 | `MustAudit`、关键路径埋点 |

---

## 3. 归一化裁决（Reconcile）—— mox 模块化系统架构冲突的唯一仲裁者

多专家观点可能冲突（例：算法说"并行"，资源说"单实例必须串行"）。裁决器负责合并：

```
优先级（高 → 低）：Permission / Security > Resource > Data > Business > Observability > Algorithm
```

- **硬约束（Blocking 级）一律落地**，且优先于一切软约束（这保证了"权限/安全不可被性能优化绕过"）。
- **同优先级冲突**：按 `policy_refs` 权重 + 租户策略裁决；仍平手则升级为 `Risk(Blocking)` 交人工/审批。
- 软约束（建议类）写入 `suggestions`，由后续优化器按需采纳。
- 裁决产出 `ReconciledPlan`：原始 `FlowGraph` + 注入的 Guard/Mutex 边 + 绑定的 `ExpertRule` + 冲突日志。

> 设计要点：裁决器**只翻译、不求解**。它把mox 模块化系统架构约束物化为 flow-ai 能识别的边/规则，
> 真正的并行/关键路径/调度仍由 flow-ai 完成，保证算法正确性不被本层污染。

---

## 4. 企业级治理（Govern）

| 能力 | 说明 |
|------|------|
| 多租户 | `Tenant{ id, namespace, quota }`，租户配额 → `ResourcePool` 容量上限；关系网检索按租户隔离 |
| RBAC | `Principal{ subject, roles }` → 权限：`run-flow` / `edit-flow` / `approve-flow` / `view-audit` |
| 策略引擎 | 轻量策略（`PolicyId` + 谓词），如「政务租户公民字段出库必须 desensitize」；权限专家消费 |
| 审计 | 追加写事件流 `AuditEvent{ who, when, flow, decision, diff }`，不可篡改 |
| 版本 | `FlowVersion` 语义版本 + 状态机 `Draft → Review → Approved → Deprecated`，支持回滚 |
| SLA | 每流程延迟/成本预算；优化后若超时/超预算则 `Risk(Blocking)` 并拦截出码 |
| 审批闸门 | 仅 `Approved` 版本可 `emit`（生成代码/执行）；未批准只允许 dry-run |

---

## 5. mox 模块化系统架构处理流水线（Pipeline）

```rust
pub fn mox_optimize(raw: &RawFlow, ctx: &GovernContext) -> GovernanceReport {
    // 1. 归一化：解析 + 维度着色 + 租户配额翻译为 ResourcePool 上限
    let flow = normalize(raw, ctx.tenant);
    // 2. 并行派发七位专家（tokio::join! / rayon）
    let opinions = dispatch_experts(&flow, ctx);
    // 3. 归一化裁决 → ReconciledPlan（合并约束为边/规则）
    let plan = reconcile(opinions, &flow);
    // 4. 交给已验证的 flow-ai 引擎做最优求解
    let opt = flow_ai::optimize(&plan.graph, &OptimizeConfig::default());
    // 5. 治理：审计 + 版本 + SLA 校验 + 审批闸门
    let gate = govern(&opt, ctx);
    // 6. 出码/出图：代码工程 + 拓扑快路径 + 可视化 + 指标
    emit(gate)
}
```

`GovernanceReport` 包含：优化报告（来自 flow-ai）+ 各专家评分 + 裁决冲突日志 + 审计事件 + SLA 结论 + 审批状态。

---

## 6. 与 flow-ai / topology 的衔接（复用，不重写）

- `ReconciledPlan.graph: FlowGraph` 直接喂给 `flow_ai::optimize` —— 零改造。
- 注入的 `MustGuard` → `NodeKind::Guard` 节点；`MustSerialize` → `EdgeKind::Mutex`；
  `Compliance` → `ExpertRule`；`ResourceCap` → `ResourcePool.capacity`。
- 关系网复用 `flow_ai::topology::TopologyGraph`：跨租户做 Skill 复用快路径（命中则跳过完整推理），
  但检索结果按 `tenant.namespace` 过滤，保证隔离。
- 算力路由复用 `flow_ai::schedule::route_models`（轻量模型处理简单问答，Hermes3 处理流程图/代码任务）。

---

## 7. 模块结构（开发阶段落地）

```
crates/mox-expert/
├── Cargo.toml
├── src/
│   ├── lib.rs              # 汇总导出 + mox_optimize 入口
│   ├── ir.rs               # Dimension / DimensionTag 归一化扩展
│   ├── context.rs          # ExpertContext / Tenant / Principal / Policy / Quota
│   ├── expert.rs           # Expert trait + Opinion/Constraint/Risk/Suggestion
│   ├── experts/
│   │   ├── mod.rs
│   │   ├── business.rs     # 业务专家
│   │   ├── algorithm.rs    # 算法专家
│   │   ├── permission.rs   # 权限专家
│   │   ├── resource.rs     # 资源专家
│   │   ├── security.rs     # 安全专家
│   │   ├── data.rs         # 数据专家
│   │   └── observability.rs# 可观测专家
│   ├── reconcile.rs        # 归一化裁决器
│   ├── govern.rs           # 审计 / 版本 / 策略 / SLA / 审批
│   └── pipeline.rs         # mox 模块化系统架构流水线编排
└── tests/                  # 七专家 + 裁决 + 治理 集成测试
```

依赖：`flow-ai`（引擎）、`serde`、`tokio`（并行派发）、`thiserror`、`uuid`、`chrono`（审计时间）。

---

## 8. 验证与测试策略

- **单元**：每位专家对固定流程图产出确定性 Opinion；裁决器在「算法并行 vs 资源串行」冲突下正确采纳资源维度。
- **集成**：政务场景端到端——权限专家注入脱敏 Guard + 资源专家注入浏览器互斥 + 算法专家标记关键路径 → flow-ai 输出 1.79× 加速且 0 阻断冲突。
- **治理**：未批准版本 `emit` 被闸门拦截；越权 `Principal` 调用 `run-flow` 被 RBAC 拒绝；审计流不可篡改（追加写 + 哈希链）。
- **契约**：`mox_optimize` 对任意合法输入不产生 panic；`ReconciledPlan` 永远是可拓扑排序的 DAG（Mutex 边已保证不环）。

---

## 9. 开发路线图（文档先行，分阶段）

- **Phase 0（本次）**：设计 + 文档 ✅
- **Phase 1**：`ir` + `context` + `expert` trait/类型 + `reconcile` 核心 + 单元骨架
- **Phase 2**：七位专家实现（先 permission/resource/algorithm 三个高价值，再补其余）
- **Phase 3**：`govern`（租户 / 审计 / 版本 / SLA / 审批）
- **Phase 4**：`pipeline` 接 flow-ai + topology，跑通端到端
- **Phase 5**：HTTP/CLI 入口 + 前端 Three.js 力导向图联动（关键路径高亮 / 冲突标红 / 关系网复用路径点亮）

---

## 10. 一句话定位

**璇玑 = 七个领域专家在归一化 IR 上并行诊断，裁决器按「权限/安全优先」mox 模块化系统架构归一，
flow-ai 引擎做已验证的最优求解，治理层把关后出码——一张图，mox 模块化系统架构最优，企业可治理。**

---

## 11. 兼容性设计（MCP / Skills / Loops / 大模型）

> 本层不重写任何外部协议，而是把外部能力**归一化进同一张 FlowGraph**，让璇玑对它们一视同仁地做mox 模块化系统架构优化。

### 11.1 MCP（Model Context Protocol）
- **McpRegistry**：`HashMap<server, Vec<McpTool>>`，`McpTool{ server, name, schema }`。
- 任意 MCP 工具在归一化阶段被落成 `FlowNode(kind=Task, tool=ToolKind::Http/Shell, tags=["mcp:<server>"])`。
- MCP 调用的**入参/出参**映射为节点 `read_set`/`write_set`，从而参与数据流并行分析与冲突检测。
- 失败策略（retry/timeout）写入 `Suggestion::Retry` / 节点 `tags`，由调度层与 Loops 策略接管。
- 兼容任意 MCP server（stdio / SSE），与 OpenClaw 现有 mcporter 路径并存。

### 11.2 Skills（技能）
- **SkillRegistry**：`Vec<SkillRef>`，`SkillRef{ id, keywords, flow_template?: FlowGraph }`。
- 与 `flow-ai::topology` 关系网打通：`SkillRef.keywords` 即拓扑网中 Skill 实体的语义词，
  语音/自然语言指令先过拓扑最短路检索，命中即**跳过完整 ReAct 推理**（快路径），未命中才走mox 模块化系统架构推理兜底。
- 璇玑产出的 `ReconciledPlan` 可反向沉淀为 `SkillRef.flow_template` → 存入记忆图谱，实现「执行即提炼、复用即剪枝」。

### 11.3 Loops（循环 / 重试 / 自省）
- **LoopPolicy**：`Bounded(max_iter)` / `Unbounded` / `HumanInLoop`。
- 流程图中 `LoopStart`/`LoopEnd` 节点 + 循环回边，由 **数据专家**校验终止条件、**安全专家**约束单轮副作用、**可观测专家**埋点。
- `LoopGuard{ policy, max_iter }` 在出码时生成带上限的 `for / while` 与超时熔断；`HumanInLoop` 落为 `ToolKind::Human` 审批节点。
- 环检测复用 `flow-ai` 的 Kahn 拓扑（返 `Err` 即环），与 Loops 策略冲突时升级为 `Risk(Blocking)`。

### 11.4 大模型（LLM 路由 / 算力智能分配）
- 复用 `flow_ai::schedule::{ModelTier, route_models}`：
  - 轻量模型（如小参数）处理简单问答 / 分类节点；
  - `Hermes3` 处理流程图解析、代码生成、复杂推理节点；
  - 算力受限时按 `ModelTier` 降级，保证 SLA。
- `compatibility::llm::tier_for(node) -> ModelTier` 把节点语义映射到算力档位；
  **算法专家**的 `Suggestion::Offload(tier)` 与 **资源专家**的配额共同决定最终路由。
- 多模型结果经「璇玑」式融合（多数投票 / 置信度加权）写入节点 `write_set`，参与下游依赖。

### 11.5 归一化收口
| 外部能力 | 归一化落点 | 参与的mox 模块化系统架构优化 |
|---------|-----------|--------------|
| MCP 工具 | `FlowNode(Task, Http/Shell)` + tags | 并行 / 冲突 / 资源 / 安全 |
| Skill | `SkillRef` ↔ 拓扑网 | 快路径检索 / 复用剪枝 |
| Loop | `LoopStart/End` + 回边 | 终止性 / 副作用 / 埋点 |
| LLM | `ModelTier` 路由 | 算力分配 / SLA |

**结论**：MCP/Skills/Loops/LLM 不是「外接插件」，而是归一化 IR 的第一类公民——
璇玑对它们做与企业内部节点完全相同的mox 模块化系统架构约束与最优求解。

---

## 12. 开发状态

- Phase 0 设计 + 文档 ✅
- **Phase 1 基础设施 ✅**（已落地 `ir` / `context` / `expert` / 7 位专家 / `reconcile` / 最小 `govern` / `pipeline` + 编译通过的集成测试；MCP/Skills/Loops/LLM 兼容类型已就位）
- Phase 2 专家深化（权限/资源/算法规则库加厚）— 进行中
- Phase 3 `govern` 完整化（审计哈希链 / 版本状态机 / SLA 预算）
- Phase 4 `pipeline` 接 `topology` 快路径 + 跨租户隔离
- Phase 5 HTTP/CLI 入口 + 前端 Three.js 力导向图联动

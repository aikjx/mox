# 璇玑 RelGraph · 企业级架构文档（多视图 · 对齐六层金字塔）

> **文档类型**：企业架构（Enterprise Architecture，多视图 / TOGAF 风格切面 · 显式 L 层级标注）
> **文档版本**：v1.1 (ENT) · 最后更新 2026-08-23
> **权威链**：🟢 L0 第一级 → [`18-全域顶层总设计-三联盟模式-V1.0.md`](18-全域顶层总设计-三联盟模式-V1.0.md)（TOP-MASTER §二：六层金字塔 · §三：八层知识图谱建模）。本文为 L2 第三级（架构层），所有声明不得与 18 冲突。
> **主责联盟**：算法联盟（图算法/图谱） + 开发联盟（分层/工程落地） · 联合签署：产品联盟（需求一致性）
> **配套**：`01-requirements.md`（需求）、`03-design.md`（设计）、`04-business-processing.md`（业务处理）、`docs/architecture.md`（OUS 父总架构 · L2 Rust 自研底座视角）
>
> 本文以「璇玑 RelGraph」为切面，沿 **业务 / 信息 / 应用 / 技术 / 安全 / 集成 / 部署** 七视图展开，并附
> **架构锚点（与 18 TOP-MASTER 六层金字塔的 L 层级对应表）**、**架构决策记录（ADR）** 与 **跨视图 NFR 落地表**。

---

## 0. 架构一句话 + 六层金字塔锚点（对齐 18 TOP-MASTER §二）

璇玑 RelGraph 是 OUS（算子统一系统）之上的**全域归一化知识图谱协同子系统**：以「知识图谱内核 + 八大算法家族核 + Rust 全自研」为底座，
在**多租户璇玑**边界内，通过**统一 AI 入口（/ai/engine/{process,analyze,...}） + 三联盟协同闭环 + 四归三连铁律**，
把「产品联盟收口需求、算法联盟落核算法、开发联盟交付代码」三路组织决策，
归一收敛到「八层图谱 × 14 节点族 × 19 边族」的唯一事实基准上。

### §0.1 七视图 ↔ 六层金字塔 双向锚点表（强制对齐）

> 18 TOP-MASTER §二「六层金字塔」架构是跨文档、跨联盟的唯一锚点。本表把 TOGAF 七视图显式映射到六个 L 层级，任何七视图新增段落须在右侧标注其所属层级。

| TOGAF 七视图 | 对应 L 层级（六层金字塔） | 核心承载模块（路径零老化） | 三联盟责任 |
|--------------|--------------------------|----------------------------|:--:|
| ① 业务 Business | **L5 业务流程层**（协作/融合/判重/文档治理 10 BP） | `platform/services/mox-system` + `frontend-ui` 28 视图 | 产品联盟 R，开发/算法 C |
| ② 信息 Information | **L4 知识图谱核心层**（八层图谱 L0~L7 · 14 节点族 · 19 边族） | `platform/services/kg-hub` · `docs/graph/graph.enterprise.json`（372 节点 / 751 边） | 算法联盟 R，开发 C |
| ③ 应用 Application | **L3 算法推理层** + **L6 产品应用层**（前端视图） | L3: graph-algorithms / optimizer / flow-ai / mox-expert；L6: frontend-ui（28 views） | 算法联盟 + 产品联盟 |
| ④ 技术 Technology | **L2 Rust 自研工程底座**（15 crate · workspace 统一治理 · 零重型脚手架依赖） | `platform/services/*`（15） + `platform/gateway/runtime/`（聚合网关） | 开发联盟 R，算法 C |
| ⑤ 安全 Security | 横切 **L1 部署运维层** · L2 · L3 · L5 | `auth_middleware` + `rbac_audit_middleware` + ⛨璇玑验证网关（G2） | 开发联盟 · 安全组 R |
| ⑥ 集成 Integration | 横切 **L6 ↔ L2 ↔ 外部**（Rust Gateway AC-10 路由语义） | `platform/gateway/runtime/` 四端点 `/ai/engine/{process,analyze,capabilities,metrics}` | 开发联盟 R，算法 C |
| ⑦ 部署 Deployment | **L1 部署运维层**（9 里程碑 M0~M8 · 三级验收 L0/L1/L2） | 部署目录规划 + CI 模板（见 18 §八） | 开发联盟 · 运维 R |

### §0.2 八大算法家族（对齐 18 TOP-MASTER §五 · 全部禁止自研等价实现）

| # | 算法家族 | 关键论文/方法 | 代码落点（L3 算法推理层） | 适用业务场景 |
|---|----------|---------------|--------------------------|--------------|
| A1 | **CNM 社区检测**（模块度贪心凝聚） | Clauset-Newman-Moore 2004 | `graph-algorithms::community::cnm` | 社区归属、模块化拆分、业务聚类 |
| A2 | **Brandes 2001 介数中心性** | Brandes 2001（含向图双 BFS 版本） | `graph-algorithms::centrality::brandes_betweenness` | 关键节点识别、瓶颈分析、故障根因 |
| A3 | **Harmonic 紧密中心性** | Rochat 2009（调和平均，解不可达） | `graph-algorithms::centrality::harmonic_closeness` | 传播能力、信息可达性 |
| A4 | **PageRank（含转置图处理）** | Page 1999；质量沿出边方向正确传播（入边权重→转置图保证） | `graph-algorithms::centrality::pagerank` | 节点重要性、SEO 排序、图谱热度 |
| A5 | **激活扩散（Activation Spread · 个性化 PageRank）** | Haveliwala 2002；个性化 PageRank 特例，d=0.85，30 轮收敛 | `graph-algorithms::spread::activation_spread` | **意图识别**（统一 AI 路由）、影响面分析、推荐召回 |
| A6 | **RRF 结果融合（Reciprocal Rank Fusion）** | Cormack et al. 2009；k=60 | `kg-hub::fusion::rrf_rank_fuse` | 多路搜索融合、检索混合、跨域召回 |
| A7 | **CEM 交叉熵优化（Cross-Entropy Method）** | Rubinstein 1999；AI Engine 高维配置优化 | `ai-agent::optimizer::cem` · `mox-expert::pipeline` | AI 引擎参数、架构配置、多目标优化 |
| A8 | **CPM 关键路径 + RCPSP 资源约束调度** | Kelley-Walker CPM；RCPSP 贪心 | `optimizer::cpm` · `flow-ai::scheduling` | 任务排程、项目计划、并行调度 |

> 硬约束（来自 project_memory）：A1 社区检测禁止标签传播 LPA；A2 介数必须 Brandes 算法；A3 紧密中心性须 Harmonic；A4 PageRank 必须包含转置图处理；A5 激活扩散须用个性化 PR d=0.85 30 轮；公式库保留全精度禁止 toFixed；密度指标必须附带人读解读文案；RAW 边输入在库内展开（非用户传双份）以避免度中心性错误。

---

## 1. 业务架构视图（Business）

### 1.1 业务能力地图

```
                璇玑系统（协作治理）
   ┌──────────────┬──────────────┬──────────────┬──────────────┐
   │ 成员管理      │ 任务协作      │ 权限分配      │ 通信机制      │
   │ Member Mgmt  │ Task Collab  │ Permission   │ Communication │
   └──────┬───────┴──────┬───────┴──────┬───────┴──────┬───────┘
          │              │              │              │
          ▼              ▼              ▼              ▼
   [入璇玑/激活/停权]  [立项/派发/推进]  [RBAC/作用域]  [频道/消息/通知]
          └──────────────┬──────────────┘
                         ▼
                  [审计留痕 / 事件溯源]
                         │
                         ▼
                  [璇玑融合（璇玑治理）] → 算子市场上架
```

### 1.2 价值流

`组建璇玑 → 邀请专家 → 立项任务 → 分派协同 → 推进验收 → 融合优化 → 上架复用`。
全程横切**审计留痕**与**事件驱动的实时通信**。

### 1.3 组织角色（与 RBAC 对齐）

`MoxAdmin / Coordinator / Expert / Member / Auditor`（详见 `01-requirements` §2、`03-design` §RBAC）。

---

## 2. 信息/数据架构视图（Information）

### 2.1 领域模型（核心实体）

| 实体 | 关键属性 | 生命周期 |
|------|----------|----------|
| Mox（璇玑） | id, name, created_by, channels[] | 创建后常驻 |
| Member（成员） | id, mox_id, name, email, status, tier, expertise[] | Invited→Active→{Suspended\|Left} |
| Task（任务） | id, mox_id, title, status, assignees[], deps[], subtasks[], comments[] | Draft→…→Done/Cancelled |
| RoleBinding（角色绑定） | member_id, role, scope | 随成员/治理变更 |
| Channel（频道） | id, kind(Mox/Task/Direct), members[] | 惰性创建 |
| Message（消息） | id, channel_id, sender, body, kind | 追加不可变 |
| Notification（通知） | id, member_id, body, read | 推送+留存 |
| AuditRecord（审计） | id, member_id, action, permission, scope, reason, ts | 仅追加 |
| DomainEvent（领域事件） | 9 类（MemberInvited/TaskCreated/TaskAssigned/TaskStatusChanged/CommentAdded/…） | 事件溯源源 |

### 2.2 数据流（核心闭环）

```
写操作(成员/任务/权限/通信)
   │  produce
   ▼
DomainEvent ──▶ EventBus(broadcast)
                    │  subscribe
                    ▼
                Reactor(反应器)
                 ├─▶ CommService.send_message  → Channel(系统消息)
                 └─▶ CommService.notify        → Notification → WebSocket 推送
```

### 2.3 数据驻留与保留

- **持久化态（I-01/I-02 已落地）**：`Store` 通过 `trait Repository` 抽象，写透 + 启动重放，重启不丢且幂等。
- **多后端可移植（NFR-03）**：支持 `SQLite`（默认，单租户/单节点）/ `PostgreSQL` / `MySQL` 三种后端，由环境变量选择，业务层对后端无感知。方言差异统一在 `repo/schema.rs` 由 `sea-query` 按方言生成，详见 §7.4。
- **内存态**：`MOX_PERSIST=false` 时为纯内存 `RwLock<State>`（重启失忆），仅用于测试/演示，接口与持久化态完全一致。
- 审计数据：`AuditChain` 已落盘且不可变追加，支持重放（I-02）。

---

## 3. 应用/服务架构视图（Application）

### 3.1 分层映射（对齐 OUS 五层，见 `docs/architecture.md` §2）

```
┌─────────────────────────────────────────────────────────────┐
│ 接入层 Ingress   Vue3/Three.js · REST · WebSocket(SSE)        │
├─────────────────────────────────────────────────────────────┤
│ 运行时 Runtime   令牌↔成员解析 · RBAC 鉴权闸门(middleware)     │
├─────────────────────────────────────────────────────────────┤
│ 编排层 Orchestration  MoxSystem 门面                     │
│                  ├─ require() 统一鉴权                        │
│                  └─ Reactor 事件→通信 反应器                  │
├─────────────────────────────────────────────────────────────┤
│ 核心域 Core Domain   MemberService · TaskService             │
│                    PermissionService · CommService           │
├─────────────────────────────────────────────────────────────┤
│ 数据层 Data         Store(接口) · EventBus(broadcast)         │
└─────────────────────────────────────────────────────────────┘
        ▲ 融合治理旁路
        │
   mox-expert（双璇玑十四维 · 璇玑）
```

### §3.2 Rust 分层矩阵 · 16 Crate × 12 列（T2 真源 ↔ 三注册表 ↔ 文档 ↔ lib.rs 常量 四方对账 AC-22 基准）

> **权威等级**：L2 架构文档（第三级），与 TOP-MASTER `18-全域顶层总设计` §二六层金字塔严格对齐。
> **真源链**：`mox-common-meta::all_crate_metas()` (T2 表) → 各 crate `src/lib.rs` `pub const CRATE_ID/ENGINE_NAME` → 三注册表 `atlas_auto_registry.json` `domain-rust-*` → 本节表格 → `docs/standards/project-atlas.md` §7 SOP。五者不一致即触发 `GET /atlas/verify` W 系列破窗告警。
> **列顺序契约（12 列，测试 test-t10-arch-fourway-diff.js 依赖）**：`Crate 目录 · package.name · CRATE_ID · ENGINE_NAME · AIS Layer · Owner · 关键 Traits · 关键 Impl · 三注册 bind · README 链接 · 版本 · CI Status`。
> **行序契约（16 行）**：严格按 `mox_common_meta::all_crate_metas()` 返回顺序（T2 向量序），禁止擅自调整顺序（测试按名称集合比对，但人读顺序需与 T2 一致）。

| # | Crate 目录 | package.name | CRATE_ID (UUIDv5) | ENGINE_NAME | AIS Layer | Owner | 关键 Traits | 关键 Impl | 三注册 bind (domain;engine;code_graph) | README 链接 | 版本 | CI Status |
|---|-----------|--------------|-------------------|-------------|-----------|-------|-------------|-----------|---------------------------------------|-------------|------|-----------|
| 1 | platform/services/ai-agent | ai-agent | `00374bdd-cc60-55bf-8970-a879afbfe443` | `mox::ai_agent` | **L4Services** | mox-core | `trait LLMProvider`; `trait HttpProvider`; `trait Guard` (engine/guards.rs); `trait AgentTool` (engine/tools.rs) | `struct AIAgent`; `impl AIAgent { chat, configure_llm, compile_requirement, create_flow, run_engine_task, spawn_agent }`; `ConversationEngine`; `BrowserAutomationEngine`; `WorkflowEngine`; `FlowEngine`; `RequirementCompiler`; `MultiAgent`; `PluginBus` | `domain=domain-rust-ai-agent`;`engine=engine-rust-ai-agent`;`code_graph_unit=ai-agent` | [ai-agent/README.md](../../platform/services/ai-agent/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |
| 2 | platform/services/business-catalog | business-catalog | `62b2cca1-d98f-5e41-b26e-8d2a43966117` | `mox::business_catalog` | **L4Services** | mox-core | `trait CatalogProvider` | `struct Business`; `struct SpiralParams`; `struct SpiralKinematics`; `struct SpiralAnalysisReport`; `impl Catalog { list_topologies, list_flowgraphs, spiral_analysis }`; bin `catalog.rs` CLI | `domain=domain-rust-business-catalog`;`engine=module-rust-business-catalog`;`code_graph_unit=business-catalog` | [business-catalog/README.md](../../platform/services/business-catalog/README.md) | 0.1.0 | 🟢 enterprise-ci |
| 3 | platform/services/flow-ai | flow-ai | `2fcd3eac-e894-5876-b007-fb33c56c0d65` | `mox::flow_ai` | **L4Services** | mox-core | `trait Primitive`; `trait Scheduler` | `struct ConflictReport`; `struct CodeBundle`; `struct TopologyGraph`; `struct Schedule`; `struct Pipeline`; `impl Pipeline { build, validate, execute, schedule, detect_conflicts }`; bin `flowopt.rs` | `domain=domain-rust-flow-ai`;`engine=engine-rust-flow-ai`;`code_graph_unit=flow-ai` | [flow-ai/README.md](../../platform/services/flow-ai/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |
| 4 | platform/services/graph-algorithms | graph-algorithms | `fbd31c6a-41cd-5274-be2f-2a28066eaf0a` | `mox::graph_algorithms` | **L4Services** | mox-core | `trait GraphAlgorithm` | `struct KnowledgeGraph`; `struct KnowledgeNode`; `struct CentralityMetrics`; `struct Community`; `struct KnowledgeGraphBuilder`; `impl { pagerank, cnm_community, brandes_betweenness, harmonic_closeness, activation_spread, density, modularity, rrf_rank_fuse }` | `domain=domain-rust-graph-algorithms`;`engine=engine-rust-graph-algorithms`;`code_graph_unit=graph-algorithms` | [graph-algorithms/README.md](../../platform/services/graph-algorithms/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |
| 5 | platform/services/hermes-flow-bridge | hermes-flow-bridge | `9bfaf43b-385a-5a44-9fb2-65b4003ee80d` | `mox::hermes_flow_bridge` | **L4Services** | mox-core | `trait BridgePlugin`; `trait Hook`; `trait SessionRecorder` | `struct HermesBridge`; `struct Router`; `struct MiniHermes`; `struct Normalizer`; `struct PluginRegistry`; `impl Bridge { route, apply_plugins, record_session, normalize_input }`; bin `bridge_demo.rs` | `domain=domain-rust-hermes-flow-bridge`;`engine=module-rust-hermes-flow-bridge`;`code_graph_unit=hermes-flow-bridge` | [hermes-flow-bridge/README.md](../../platform/services/hermes-flow-bridge/README.md) | 0.1.0 | 🟢 enterprise-ci |
| 6 | platform/services/kg-hub | kg-hub | `cb909f06-c0df-55ec-b397-543623a8c349` | `mox::kg_hub` | **L4Services** | mox-core | `trait Connector`; `trait IngestPipeline`; `trait Reasoner` | `struct HybridIndex`; `struct Consolidator`; `struct URN`; `struct Ontology`; `struct LoopEngine`; `struct GovPolicy`; `impl { ingest, reason, govern, impact, hotspots, consolidate, loop_stage }`; 5 Connectors (SQLite/JSON/HTTP/CSV/API) | `domain=domain-rust-kg-hub`;`engine=engine-rust-kg-hub`;`code_graph_unit=kg-hub` | [kg-hub/README.md](../../platform/services/kg-hub/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |
| 7 | platform/services/operator-core | operator-core | `acf14283-3931-5528-adce-2c0cd3815363` | `mox::operator_core` | **L6Kernel** | mox-core | `trait Operator` (operator.rs); `trait Kernel` (kernel.rs); `trait ResourceContainer`; `trait ConservationLaw` (conservation.rs); `trait KernelExt` (kernel_ext.rs) | `struct OperatorError`; `struct State`; `struct Category`; `struct Registry`; `struct Resource`; `impl Monad for Result<T, OperatorError>`; `impl conservation::validate()`; `Registry::register/query/list()`; 4+ conservation 守恒律闸门 | `domain=domain-rust-operator-core`;`engine=engine-rust-operator-core`;`code_graph_unit=operator-core` | [operator-core/README.md](../../platform/services/operator-core/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |
| 8 | platform/services/operator-wasm | operator-wasm | `5a1df407-b217-5340-a5ae-5f4535d1e6de` | `mox::operator_wasm` | **L4Services** | mox-core | `trait WasmHost` | `struct WasmOperator`; `struct WasmModule`; `struct Instance`; `impl WasmOperator::call(wasmer::Instance) -> Result<Value>`; WASM Sandbox (wasmer + cranelift compiler) | `domain=domain-rust-operator-wasm`;`engine=module-rust-operator-wasm`;`code_graph_unit=operator-wasm` | [operator-wasm/README.md](../../platform/services/operator-wasm/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |
| 9 | platform/services/optimizer | optimizer | `e56676c7-ec1f-5415-9587-ba8249d0178a` | `mox::optimizer` | **L4Services** | mox-core | `trait Objective`; `trait Schedule` | `impl cpm_critical_path()`; `impl rcpsp_greedy()`; `impl multi_objective_eval_cem()`（CEM 交叉熵优化器，配置调参/多目标权重） | `domain=domain-rust-optimizer`;`engine=engine-rust-optimizer`;`code_graph_unit=optimizer` | [optimizer/README.md](../../platform/services/optimizer/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |
| 10 | platform/services/primiflow-core | primiflow-core | `8c8d2382-6f9f-5218-894e-a07a43aa9554` | `mox::primiflow_core` | **L4Services** | mox-core | `trait Executor`; `trait Store`; `trait Generator` | `struct Parse`; `struct Persistence`; `struct Runner`; `struct Server`; `mod gen { c1..c8 DDL 骨架模板 }`; `impl execute / persist / generate_code / parse_ddl` | `domain=domain-rust-primiflow-core`;`engine=engine-rust-primiflow-core`;`code_graph_unit=primiflow-core` | [primiflow-core/README.md](../../platform/services/primiflow-core/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |
| 11 | platform/services/primiflow-fusion | primiflow-fusion | `75238345-b48b-534b-818b-8d9abe083a41` | `mox::primiflow_fusion` | **L4Services** | mox-core | `trait Platform`; `trait Envelope`; `trait FusionRegistry` | `struct Sixdim` (六维度量); `struct Observability`; `struct PTDoc`; `struct Config`; `struct Server`; `struct Registry`; `impl fuse / register_service / sixdim_score / conservation_gate / version_migrate` | `domain=domain-rust-primiflow-fusion`;`engine=engine-rust-primiflow-fusion`;`code_graph_unit=primiflow-fusion` | [primiflow-fusion/README.md](../../platform/services/primiflow-fusion/README.md) | 0.1.0 | 🟢 enterprise-ci |
| 12 | platform/services/template-market | template-market | `4d2e50c1-9d64-525d-86cf-2d7d610a27b9` | `mox::template_market` | **L4Services** | mox-core | `trait MarketProvider` | `struct Template`; `struct MarketSeed`; `struct Rating`; `impl { list, publish, load, fork, sort_by_score }`; 2 商城种子（政务流程模板 + ETL 模板） | `domain=domain-rust-template-market`;`engine=module-rust-template-market`;`code_graph_unit=template-market` | [template-market/README.md](../../platform/services/template-market/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |
| 13 | platform/services/mox-expert | mox-expert | `50bb6200-04c5-5e4c-8354-4c6e1b230024` | `mox::mox_expert` | **L4Services** | mox-core | `trait Expert` (expert_traits.rs); `trait Verify` (verify/mod.rs); `trait AuditSink` (audit/sink.rs); `trait DomainRule` (domain/mod.rs); `trait ExpertHarness` (harness.rs) | 14 专家 `struct {Algorithm,Architecture,Business,CodeQuality,Data,Documentation,Maintainability,Observability,Performance,Permission,Resource,Security,SecurityCode,Testing}Expert`; `impl Expert::evaluate()`; `impl Audit::emit_to_s3_kafka_syslog()`; `Verify::{cem,topology,data_dep,conflict,gains,code_rt}`; RBAC `policy / check`; bin `mox.rs` | `domain=domain-rust-mox-expert`;`engine=engine-rust-mox-expert`;`code_graph_unit=mox-expert` | [mox-expert/README.md](../../platform/services/mox-expert/README.md) | 0.1.0 | 🟢 enterprise-ci |
| 14 | platform/services/mox-system | mox-system | `b81eec75-22ff-5155-ac49-19edf6f6b5ab` | `mox::mox_system` | **L7Infrastructure** | mox-core | `trait Repository` (repo/mod.rs); `trait PersistenceProvider` (persistence_provider.rs); `trait DomainService` (domain_traits.rs) | `struct Orchestrator`; 4 Services `{Member,Task,Permission,Comm}Service`; `struct Store`; `struct RBAC`; `struct Metrics`; `struct RateLimiter`; `struct Crypto`; `impl Repository for SqliteRepo + PostgresRepo + MysqlRepo`; `impl orchestrator.require()` 鉴权闸门 + 反应器编排 | `domain=domain-rust-mox-system`;`engine=engine-rust-mox-system`;`code_graph_unit=mox-system` | [mox-system/README.md](../../platform/services/mox-system/README.md) | 0.1.0 | 🟢 enterprise-ci |
| 15 | platform/gateway/runtime | runtime | `a6f7ad5c-dbc8-5c27-837f-d8332fd6f27b` | `mox::runtime` | **L3Orchestration** | mox-core | `trait Lifecycle` (cordis/lifecycle.rs); `trait CordisBundle` (cordis/bundle.rs); `trait AiRouter` (ai_router.rs); `trait RbacPolicy` (rbac_middleware.rs) | `struct RouterTable`; `struct CapabilityRouter`; `struct MarketDSL`; `struct MigrationEngine`; `struct Governance`; `struct Sidecar`; `impl axum routes/{agent,ai_engine,governance,market}`; Cordis5 生命周期 {startup/shutdown/before_handle/after_handle/profile}; OpenAPI spec; operator-server 二进制入口 | `domain=domain-rust-runtime`;`engine=engine-rust-runtime`;`code_graph_unit=runtime` | [runtime/README.md](../../platform/gateway/runtime/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |
| 16 | platform/services/mox-common-meta | mox-common-meta | `34a20231-1a80-5426-b392-40d7a2ddd9f7` | `mox::mox_common_meta` | **L5Domain** | mox-core | (纯数据元 crate，无对外 trait) | `pub enum AisLayer { L2Gateway, L3Orchestration, L4Services, L5Domain, L6Kernel, L6KernelExt, L7Infrastructure }`; `pub struct CrateMeta { id,name,version,layer,owner }`; `impl CrateMeta::engine_name()`; `pub fn all_crate_metas() -> Vec<CrateMeta>` (16 行硬编码真源); `pub fn lookup_meta_by_engine(name) -> Option<CrateMeta>` | `domain=domain-rust-mox-common-meta`;`engine=module-rust-mox-common-meta`;`code_graph_unit=mox-common-meta` | [mox-common-meta/README.md](../../platform/services/mox-common-meta/README.md) | 3.0.0-ai-powered | 🟢 enterprise-ci |

**行数量校验**：16/16（12 L4Services + 1 L3Orchestration runtime + 1 L6Kernel operator-core + 1 L7Infrastructure mox-system + 1 L5Domain mox-common-meta）。

> 🔍 **AC-22 四方对账自动化**：`node platform/backend-node/test/test-t10-arch-fourway-diff.js` 每次改动本表后必须运行；任何不一致都会 exit 1。
> 📌 **路径零老化约定**：所有 Crate 目录列均为相对仓库根的真实存在路径（`platform/services/*` 15 个 + `platform/gateway/runtime` 1 个）；不得再使用旧别名 `crates/`。

### 3.3 模块依赖

```
server.rs ──▶ orchestrator.rs(MoxSystem) ──▶ services.rs(Member/Task/Permission/Comm)
                                                    │
                                                    ▼
                                              store.rs + event.rs(EventBus)
mox-system ◀── POST /api/mox/* ── mox-expert(pipeline)
```

### 3.4 服务职责（单一职责）

| 服务 | 职责 | 不负责 |
|------|------|--------|
| MemberService | 成员生命周期、邀请幂等、状态机 | 任务逻辑 |
| TaskService | 任务 FSM、分派校验、DoD、DAG、评论 | 权限判定（交由 orchestrator.require） |
| PermissionService | 角色绑定增删、权限解析 | 请求上下文（交由 orchestrator） |
| CommService | 频道/消息/通知的读写 | 事件产生（由领域动作产生） |
| Orchestrator | 鉴权闸门 + 反应器编排 | 具体领域规则（下沉到 Service） |

---

## 4. 技术架构视图（Technology）

### 4.1 技术栈

| 层 | 技术 | 说明 |
|----|------|------|
| 语言 | Rust 2021 + Tokio | 异步运行时 |
| Web | Axum 0.7 + tower-http(CORS) | REST + WebSocket |
| 序列化 | serde / serde_json | 事件与 API 载荷 |
| 并发原语 | tokio::sync::{RwLock, broadcast} | Store 锁 + 事件总线 |
| 前端 | Vue3 + Three.js + ECharts | 融合工作台 / 监控台 |
| 插件沙箱 | WASM(wasmer) | 第三方算子隔离（OUS 内核） |

### 4.2 关键设计取舍

- **内存态优先**：降低首版复杂度，接口预留可替换（NFR-03）。
- **broadcast 事件总线**：多消费者（反应器、潜在的审计/指标消费者）解耦。
- **令牌即身份**：`X-Auth-Token` ↔ member_id 映射，缺失即 401（NFR-12）。

---

## 5. 安全架构视图（Security）

### 5.1 威胁模型（STRIDE 对齐 mox-expert `security.rs`）

| 威胁 | 缓解 |
|------|------|
| **S**poofing 伪造身份 | 令牌鉴权；bootstrap 唯一无鉴权入口 |
| **T**ampering 篡改 | 写操作统一 `require()`；事件不可变追加 |
| **R**epudiation 抵赖 | 领域事件 + 审计记录 + 鉴权拒绝留痕 |
| **I**nfo 泄露（跨租户） | 查询按 mox_id 过滤；分派三重校验（GAP-2） |
| **D**oS | 配额/限流（NFR-09，路线图中） |
| **E**levation 提权 | 最小权限 + 作用域 + `*Own` 所有权 + 试探式鉴权不落审计防探测 |

### 5.2 RBAC 模型

- **角色**：MoxAdmin / Coordinator / Expert / Member / Auditor，继承链 `Coordinator→Expert→Member`。
- **权限**：14 原子（task:create/assign/edit:all/edit:own/view:all/view:assigned/comment/transition:all/transition:own、member:invite/manage、comm:send:mox/send:task/send:direct、audit:view）。
- **作用域**：Global（仅 bootstrap 管理员）/ Mox（受邀默认）/ Task（临时授权）。
- **所有权**：`*Own` 类权限额外要求调用者在 `task.assignees` 中（前提：assignees 可信，见 GAP-2 修复）。

### 5.3 安全护栏（关键）

- **跨租户提权路径已闭环**：修复前 `assign()` 对 assignees 零校验 → 可写入他璇玑成员 ID 获得 own 权限；修复后 `validate_assignees` 三重校验（不存在/跨璇玑/非Active 均拒）。
- **审计不可被噪声淹没**：两段式鉴权，仅终局裁决失败留痕（见 `01-requirements` FR-PERM-06 / `br18`）。
- **AuthzDenied 事件不回推当事人**：避免实时通道成为权限探测反馈信道。

---

## 6. 集成架构视图（Integration）

### 6.1 对外接口

| 类别 | 端点 | 说明 |
|------|------|------|
| 健康检查 | `GET /api/health` | 探针 |
| 身份 | `POST /api/bootstrap`、`GET /api/me` | 建璇玑/令牌解析 |
| 成员 | `POST/GET /api/members` 等 | 邀请/列表/状态 |
| 任务 | `POST/GET /api/tasks`、`POST /api/tasks/:id/transition` 等 | 全生命周期 |
| 通信 | `GET /api/channels`、`POST /api/channels/:id/messages` | 频道消息 |
| 实时 | `WS /api/ws?token=` | 通知推送 |
| 融合 | `POST /api/mox/optimize`、`/publish` | 璇玑治理→上架 |

### 6.2 事件契约

`DomainEvent` 9 类，是审计、实时推送、潜在指标采集的**唯一事实源**（单一数据源原则）。

### 6.3 与外部系统

- `mox-expert`：归一化/治理/优化（旁路集成）。
- 算子市场：`/api/mox/publish` 上架优化产物（见 `docs/modules/business-process-flowcharts.md` §8）。
- LLM（可选）：企业流程 AiTask 真实执行，未配置 fail-closed。

---

## 7. 部署/运维视图（Deployment & Ops）

### 7.1 部署视图（运行形态）

- 单体进程：`cargo run -p mox-system` → `:3000`（REST+WS）；`--demo` 端到端演示。
- 作为 OUS 子系统：由 `runtime` 主服务聚合各 crate 端点。

#### 7.1.1 runtime crate 聚合内部架构（L1+L2 细项）

**L1 Ingress（路由+处理器薄层）：**
- `src/routes/mod.rs` 路由总入口：agent.rs (AI 智能体端点) + governance.rs（治理台 HITL/审批/指标） + market.rs（算子市场 DSL/版本化/迁移）
- `src/handlers/mod.rs` HTTP 处理器薄层：agent.rs / governance.rs / hitl.rs（纯 request→response 适配，不含业务算法）
- `main.rs` 二进制入口 `operator-server`（axum server 启动 + 生命周期 + 优雅停机）

**L2 Gateway（编排/中间件/聚合）：**
- `src/cordis/mod.rs`（OUS-Cordis 插件内核 5 子模块）：profile + bundle + seam(SeamRegistry fs 注册) + event_bus(事件瀑布) + lifecycle(Start/Stop/Pause)
- `src/rbac_middleware.rs`：RBAC 鉴权闸门（X-Auth-Token TokenRegistry → member_id + 角色校验）
- `src/subservers.rs`：聚合 16 crate 的子服务（ai-agent/mox-expert 等）挂载编排
- Feature gates: `market`（算子市场）/ `governance`（治理台）/ `openapi`（OpenAPI 生成） — 默认开启
- `src/automation.rs` + `api_standard.rs` + `openapi.rs`：API 标准化响应 / OpenAPI schema 生成

### 7.2 可观测性（设计态 → 路线图）

| 信号 | 现状 | 目标（NFR-08） |
|------|------|----------------|
| 日志 | 结构化（tracing） | 统一采集 |
| 指标 | 未采集 | 鉴权拒绝率/状态迁移分布/优化耗时（Prometheus） |
| 追踪 | trace_id 注入 | 跨 crate 链路 |

### 7.3 灾备

- **已落地**：Store 持久化写透 + 启动重放、审计链落盘重放（I-01/I-02）。SQLite 单文件可冷备；PostgreSQL / MySQL 可复用其原生主从与 PITR 备份体系。
- **待办**：WAL 快照与混沌演练（I-12，见 `05` 路线图）。对齐 `docs/architecture.md` §16。

### 7.4 持久化后端选型与配置矩阵（唯一权威落点）

> 本节是璇玑系统持久化配置的**唯一事实基准**。根 `README.md` 与其他文档仅可指向本节，不得重复定义，避免配置口径漂移。

**设计原则**：12-Factor 配置注入，零代码切换；默认 `SQLite` 保证开箱即用、无外部依赖。

| 环境变量 | 取值 | 默认 | 说明 |
|----------|------|------|------|
| `MOX_PERSIST` | `true`/`1` \| `false`/`0` | `false` | 是否落盘。`false` 为纯内存态（重启失忆，仅测试/演示） |
| `MOX_STRICT_PERSIST` | `true`/`1` \| `false`/`0` | `false` | **生产级 fail-fast**：打开后若连库或建表失败，**启动直接中止**，杜绝"连不上库却照常起服务、数据只进内存、重启即丢"的静默故障 |
| `MOX_BACKEND` | `sqlite` \| `postgres` \| `mysql` | `sqlite` | 后端方言。无法识别时安全回退 `sqlite` |
| `MOX_DB_URL` | 连接串 | `./data/mox.db` | SQLite 为文件路径；PG/MySQL 为标准 URL |

**推荐组合矩阵**：

| 场景 | `PERSIST` | `STRICT_PERSIST` | `BACKEND` | `DB_URL` |
|------|-----------|------------------|-----------|----------|
| 本地开发 / 单节点（默认） | `false` | `false` | `sqlite` | 默认 |
| SQLite 持久化 | `true` | `false` | `sqlite` | `./data/mox.db` |
| **PostgreSQL 生产** | `true` | **`true`** | `postgres` | `postgres://user:pass@host:5432/db` |
| **MySQL 生产** | `true` | **`true`** | `mysql` | `mysql://user:pass@host:3306/db` |

**方言归一化实现**：upsert 语义按后端生成——SQLite `INSERT OR REPLACE` + `?N`；PostgreSQL `ON CONFLICT DO UPDATE` + `$N`；MySQL `ON DUPLICATE KEY UPDATE` + `?`。落点 `crates/mox-system/src/repo/`（`schema.rs` 方言层 + `sqlite.rs`/`postgres.rs`/`mysql.rs` 驱动层）。

**启动可观测性**：启动日志如实回显后端与严格模式，便于运维核对实际生效配置：

```
持久化模式: 开启 (后端=Postgres, 严格模式=开(连库失败即中止))
```

**fail-fast 错误路径**：`Store::open` 内 `migrate()` 失败已 `?` 成 `Err`，故 `MOX_STRICT_PERSIST` **同时覆盖「连接失败」与「建表失败」**两条路径；错误经 `with_config` 上浮至 `main` 以规整致命信息 + 非零退出码终止（非 panic backtrace），契合容器编排重启探针语义。

---

## 8. 架构决策记录（ADR）

| ADR | 决策 | 背景 | 后果 |
|-----|------|------|------|
| ADR-01 | RBAC 作为统一鉴权闸门，所有写操作经 `require()` | 避免散落权限判断 | 单一入口易审计；需防绕过 |
| ADR-02 | 领域事件 + 反应器解耦通信 | 写逻辑与推送解耦 | 可加指标/审计消费者；需保证幂等 |
| ADR-03 | ~~内存 Store + 稳定接口，暂不实持久化~~（**已被 ADR-07 取代**） | 首版降复杂度 | 🟡 SUP：稳定接口设计使后续替换零成本 |
| ADR-07 | `trait Repository` 多后端持久化：SQLite（默认）/ PostgreSQL / MySQL，12-Factor 选择 + `STRICT_PERSIST` fail-fast | 兑现 NFR-03 可移植性；生产需外部库与主从备份，开发需零依赖 | 重启不丢；方言差异集中于 `repo/schema.rs`；严格模式下连库失败即中止启动（见 §7.4） |
| ADR-04 | `*Own` 所有权权限依赖可信 `assignees` | 精细授权 | 分派必须校验（GAP-2 修复） |
| ADR-05 | 两段式鉴权（试探不落审计） | 防审计噪声/权限探测 | 实现稍复杂；测试双向断言 |
| ADR-06 | 融合治理与协作治理分离为两域 | 关注点分离 | 双验收联动待补（FR-FUSE-05） |

---

## 9. 跨视图 NFR 落地表

| NFR | 业务视图 | 应用视图 | 技术视图 | 安全视图 |
|-----|----------|----------|----------|----------|
| NFR-01 多租户 | 璇玑边界 | 查询过滤 | Store 隔离 | 分派校验 |
| NFR-04 解耦 | 事件闭环 | Reactor | broadcast | 审计独立 |
| NFR-08 可观测 | 指标定义 | 指标埋点 | tracing | — |
| NFR-11 一致性 | 幂等 | Reactor 幂等 | broadcast | 事件不可变 |

---

## 10. 与父系统 OUS 的关系

- 本文是 `docs/architecture.md`（v7.0，79KB 总架构）的**璇玑子系统切面**。
- 能力对齐见 `docs/enterprise-architecture-analysis.md`（双璇玑十四维、能力覆盖矩阵）。
- 融合链路见 `docs/modules/mox-expert-alliance-fusion-flows.md`。

---

*本文七视图 + ADR 构成企业级架构骨架；详细模块设计见 `03-design.md`，业务处理见 `04-business-processing.md`。*

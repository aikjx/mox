# 璇玑 RelGraph · 企业级架构文档 v2.0（6层8域DDD矩阵 · 多视图 · 对齐六层金字塔）

> **文档类型**：企业架构（Enterprise Architecture，多视图 / TOGAF 风格切面 · 显式 L 层级标注）
> **文档版本**：v2.0 (ENT) · 最后更新 2026-08-26（v1.1→v2.0：架构从旧15-crate扁平模型迁移至新6层8域DDD矩阵，全量路径与crate映射更新）
> **权威链**：🟢 L0 第一级 → [`18-全域顶层总设计-三联盟模式-V1.0.md`](18-全域顶层总设计-三联盟模式-V1.0.md)（TOP-MASTER §二：六层金字塔 · §三：八层知识图谱建模）。本文为 L2 第三级（架构层），所有声明不得与 18 冲突。架构迁移基准见 [`ARCHITECTURE-MIGRATION.md`](ARCHITECTURE-MIGRATION.md)。
> **主责联盟**：算法联盟（图算法/图谱） + 开发联盟（分层/工程落地） · 联合签署：产品联盟（需求一致性）
> **配套**：`01-requirements.md`（需求）、`03-design.md`（设计）、`04-business-processing.md`（业务处理）、`docs/architecture.md`（OUS 父总架构 · 归档参考）、`ARCHITECTURE-MIGRATION.md`（旧→新crate完整映射）
>
> 本文以「璇玑 RelGraph」为切面，沿 **业务 / 信息 / 应用 / 技术 / 安全 / 集成 / 部署** 七视图展开，并附
> **架构锚点（与 18 TOP-MASTER 六层金字塔的 L 层级对应表）**、**架构决策记录（ADR）** 与 **跨视图 NFR 落地表**。
> **架构模型**：6层8域DDD矩阵 — foundation（L0）/ gateway（L1）/ domains×core（L2）/ domains×svc（L3）/ domains×sdk（L4）/ domains×api（L5，规划中）；8域 = ai / cloud / data / flow / kg / market / platform / voice。

---

## 0. 架构一句话 + 六层金字塔锚点（对齐 18 TOP-MASTER §二）

璇玑 RelGraph 是 OUS（算子统一系统）之上的**全域归一化知识图谱协同子系统**：以「知识图谱内核 + 八大算法家族核 + Rust 全自研」为底座，
在**多租户璇玑**边界内，通过**统一 AI 入口（/ai/engine/{process,analyze,...}） + 三联盟协同闭环 + 四归三连铁律**，
把「产品联盟收口需求、算法联盟落核算法、开发联盟交付代码」三路组织决策，
归一收敛到「八层图谱 × 14 节点族 × 19 边族」的唯一事实基准上。

### §0.1 七视图 ↔ 六层金字塔 双向锚点表（强制对齐）

> 18 TOP-MASTER §二「六层金字塔」架构是跨文档、跨联盟的唯一锚点。本表把 TOGAF 七视图显式映射到六个 L 层级，任何七视图新增段落须在右侧标注其所属层级。

| TOGAF 七视图 | 对应 L 层级（六层金字塔） | 核心承载模块（路径零老化 · 6层8域DDD矩阵） | 三联盟责任 |
|--------------|--------------------------|-----------------------------------------------|:--:|
| ① 业务 Business | **L5 业务流程层**（协作/融合/判重/文档治理 10 BP） | `platform/domains/platform/svc/mox-platform-enterprise-svc` + `frontend-ui` 28 视图 | 产品联盟 R，开发/算法 C |
| ② 信息 Information | **L4 知识图谱核心层**（八层图谱 L0~L7 · 14 节点族 · 19 边族） | `platform/domains/kg/svc/mox-kg-hub-svc` · `docs/graph/graph.enterprise.json`（372 节点 / 751 边） | 算法联盟 R，开发 C |
| ③ 应用 Application | **L3 算法推理层** + **L6 产品应用层**（前端视图） | L3: `mox-kg-algo-core` / `mox-flow-optimizer-core` / `mox-ai-flow-svc` / `mox-ai-expert-svc`；L6: frontend-ui（28 views） | 算法联盟 + 产品联盟 |
| ④ 技术 Technology | **L2 Rust 自研工程底座**（6层8域DDD矩阵 · 50+ crate · workspace 统一治理 · 零重型脚手架依赖） | `platform/domains/{8域}/{core,svc,sdk}`（50+） + `platform/foundation/`（2） + `platform/gateway/mox-platform-gateway-svc` | 开发联盟 R，算法 C |
| ⑤ 安全 Security | 横切 **L1 部署运维层** · L2 · L3 · L5 | `mox-platform-iam-core` + `mox-ai-expert-svc`（⛨璇玑验证网关 G2） + gateway RBAC 中间件 | 开发联盟 · 安全组 R |
| ⑥ 集成 Integration | 横切 **L6 ↔ L2 ↔ 外部**（Rust Gateway AC-10 路由语义） | `platform/gateway/mox-platform-gateway-svc` 四端点 `/ai/engine/{process,analyze,capabilities,metrics}` | 开发联盟 R，算法 C |
| ⑦ 部署 Deployment | **L1 部署运维层**（9 里程碑 M0~M8 · 三级验收 L0/L1/L2） | 部署目录规划 + CI 模板（见 18 §八） | 开发联盟 · 运维 R |

### §0.2 八大算法家族（对齐 18 TOP-MASTER §五 · 全部禁止自研等价实现）

| # | 算法家族 | 关键论文/方法 | 代码落点（L3 算法推理层 · 6层8域） | 适用业务场景 |
|---|----------|---------------|-------------------------------------|--------------|
| A1 | **CNM 社区检测**（模块度贪心凝聚） | Clauset-Newman-Moore 2004 | `mox-kg-algo-core::community::cnm` | 社区归属、模块化拆分、业务聚类 |
| A2 | **Brandes 2001 介数中心性** | Brandes 2001（含向图双 BFS 版本） | `mox-kg-algo-core::centrality::brandes_betweenness` | 关键节点识别、瓶颈分析、故障根因 |
| A3 | **Harmonic 紧密中心性** | Rochat 2009（调和平均，解不可达） | `mox-kg-algo-core::centrality::harmonic_closeness` | 传播能力、信息可达性 |
| A4 | **PageRank（含转置图处理）** | Page 1999；质量沿出边方向正确传播（入边权重→转置图保证） | `mox-kg-algo-core::centrality::pagerank` | 节点重要性、SEO 排序、图谱热度 |
| A5 | **激活扩散（Activation Spread · 个性化 PageRank）** | Haveliwala 2002；个性化 PageRank 特例，d=0.85，30 轮收敛 | `mox-kg-algo-core::spread::activation_spread` | **意图识别**（统一 AI 路由）、影响面分析、推荐召回 |
| A6 | **RRF 结果融合（Reciprocal Rank Fusion）** | Cormack et al. 2009；k=60 | `mox-kg-fusion-svc::rrf_rank_fuse` | 多路搜索融合、检索混合、跨域召回 |
| A7 | **CEM 交叉熵优化（Cross-Entropy Method）** | Rubinstein 1999；AI Engine 高维配置优化 | `mox-ai-agent-svc::optimizer::cem` · `mox-ai-expert-svc::pipeline` | AI 引擎参数、架构配置、多目标优化 |
| A8 | **CPM 关键路径 + RCPSP 资源约束调度** | Kelley-Walker CPM；RCPSP 贪心 | `mox-flow-optimizer-core::cpm` · `mox-ai-flow-svc::scheduling` | 任务排程、项目计划、并行调度 |

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

### 3.1 分层映射（6层8域DDD矩阵 · 对齐 OUS 六层金字塔）

```
┌─────────────────────────────────────────────────────────────────────┐
│ 接入层 Ingress   Vue3/Three.js · REST · WebSocket(SSE) · /admin 5面板  │
├─────────────────────────────────────────────────────────────────────┤
│ L1 Gateway   mox-platform-gateway-svc：路由·鉴权·限流·CORS·WS·健康检查  │
│              （仅横切中间件，业务聚合下沉到各域svc层）                      │
├─────────────────────────────────────────────────────────────────────┤
│ L3 Svc（8域应用服务）  ai/cloud/data/flow/kg/market/platform/voice       │
│        29 crate：HTTP handler · 业务编排 · DB repo · 外部API client      │
├─────────────────────────────────────────────────────────────────────┤
│ L2 Core（8域领域模型）  15 crate：纯业务逻辑 · trait定义 · 无I/O依赖      │
│        算子内核 · 图算法 · 优化器 · IAM · 数据存储 · 编排器 · 语音DSP     │
├─────────────────────────────────────────────────────────────────────┤
│ L4 Sdk（8域对外类型）  6 crate：客户端类型 · FFI绑定(napi/PyO3) · 测试工具 │
├─────────────────────────────────────────────────────────────────────┤
│ L5 Api（8域域间契约）  规划中（0 crate）：trait/interface/DTO · 依赖倒置  │
├─────────────────────────────────────────────────────────────────────┤
│ L0 Foundation（横切基础）  mox-platform-foundation + mox-cloud-foundation │
│        通用类型 · 错误处理 · 配置 · 工具函数 · 云基础设施抽象              │
├─────────────────────────────────────────────────────────────────────┤
│ Framework（插件框架）  mox-framework：扩展点定义 · 插件注册              │
└─────────────────────────────────────────────────────────────────────┘
        ▲ 融合治理旁路
        │
   mox-ai-expert-svc（双璇玑十四维 · ⛨璇玑验证网关 · 14专家）
```

**依赖方向（强制）**：`L0 → L2 → L3 → L1 → Ingress`（自底向上）；`L4/L5` 横切，可被 L2/L3 引用；**禁止 L2 依赖 L3，禁止跨域直接依赖其他域的 L3 svc（必须通过 L5 api 契约）**。

### §3.2 Rust 分层矩阵 · 6层8域DDD矩阵 × 54+ Crate（v2.0 · 对齐 ARCHITECTURE-MIGRATION.md）

> **权威等级**：L2 架构文档（第三级），与 TOP-MASTER `18-全域顶层总设计` §二六层金字塔严格对齐。
> **架构模型**：6层8域DDD矩阵 — L0 Foundation（横切基础）/ L1 Gateway（网关）/ L2 Core（领域模型，8域）/ L3 Svc（应用服务，8域）/ L4 Sdk（对外类型，8域）/ L5 Api（域间契约，8域，规划中）。
> **真源链**：`Cargo.toml` workspace members（73 member，含FFI绑定/测试harness）→ 各 crate `src/lib.rs` `pub const CRATE_ID/ENGINE_NAME` → 本节表格 → `mox-platform-meta-core::all_crate_metas()`。五者不一致即触发架构告警。
> **旧→新映射**：完整17旧crate→54新crate映射见 [`ARCHITECTURE-MIGRATION.md`](ARCHITECTURE-MIGRATION.md)。

#### §3.2.1 横切层（L0 Foundation + L1 Gateway + Framework）

| # | Crate 目录 | package.name | 层级 | Owner | 关键职责 | 版本 |
|---|-----------|--------------|------|-------|---------|------|
| F1 | platform/foundation/mox-platform-foundation | mox-platform-foundation | L0 Foundation | 开发联盟 | 平台基础库：通用类型、错误处理（thiserror/anyhow统一）、配置、工具函数、tracing初始化 | 3.0.0 |
| F2 | platform/foundation/mox-cloud-foundation | mox-cloud-foundation | L0 Foundation | 开发联盟 | 云基础设施基础库：云存储抽象、卷管理、S3适配、文件器接口 | 3.0.0 |
| G1 | platform/gateway/mox-platform-gateway-svc | mox-platform-gateway-svc | L1 Gateway | 开发联盟 | API网关：路由、鉴权、限流、CORS、WebSocket、健康检查、OpenAPI。**仅做横切中间件，业务聚合下沉到各域svc层** | 3.0.0 |
| FW1 | platform/framework/ | mox-framework | Framework | 开发联盟 | 框架层：插件框架/扩展点定义（库） | 3.0.0 |

#### §3.2.2 8域 × Core层（L2 · 领域模型 · 15 crate）

| 域 | Crate 目录 | package.name | 关键 Trait / 类型 | 关键实现 |
|----|-----------|--------------|-------------------|---------|
| **ai** | domains/ai/core/mox-ai-core | mox-ai-core | `trait AIKernel` | AI统一内核：LLM客户端抽象、对话状态、ProviderRegistry |
| **ai** | domains/ai/core/mox-ai-intent-core | mox-ai-intent-core | `trait IntentClassifier` | 意图识别领域模型：A5激活扩散意图路由核心、意图分类器、能力路由打分 |
| **data** | domains/data/core/mox-data-formula-core | mox-data-formula-core | `trait FormulaEngine` | 公式引擎核心：高精度计算、表达式解析、变量绑定 |
| **data** | domains/data/core/mox-data-norm-core | mox-data-norm-core | `trait Normalizer` | 数据归一化核心：归一化IR、六维绑定、守恒闸门 |
| **data** | domains/data/core/mox-data-standards-core | mox-data-standards-core | `trait DataStandard` | 数据标准核心：Schema定义、数据质量规则、标准注册 |
| **flow** | domains/flow/core/mox-flow-operator-core | mox-flow-operator-core | `trait Operator`; `trait Kernel`; `trait ConservationLaw`; `trait ResourceContainer` | 算子代数内核：State/Category/Registry/Resource、守恒律闸门4+、Monad三定律、范畴论态射规则 |
| **flow** | domains/flow/core/mox-flow-optimizer-core | mox-flow-optimizer-core | `trait Objective`; `trait Schedule` | 优化器核心：CPM关键路径、RCPSP资源约束调度、CEM交叉熵优化器 |
| **kg** | domains/kg/core/mox-kg-algo-core | mox-kg-algo-core | `trait GraphAlgorithm` | 八大算法家族A1~A8：CNM社区/Brandes介数/Harmonic紧密/PageRank/激活扩散/RRF/CEM/CPM |
| **kg** | domains/kg/core/mox-kg-meta-core | mox-kg-meta-core | `trait Ontology`; `trait SchemaManager` | 图谱元数据核心：本体管理、Schema版本化、14节点族×19边族定义 |
| **platform** | domains/platform/core/mox-platform-system-core | mox-platform-system-core | `trait Repository`; `trait DomainService` | 璇玑系统核心：成员/任务/权限/通信领域模型、Store接口、EventBus、RBAC策略 |
| **platform** | domains/platform/core/mox-platform-iam-core | mox-platform-iam-core | `trait IdentityProvider`; `trait AccessController` | IAM核心：身份认证、令牌管理、访问控制、API Key |
| **platform** | domains/platform/core/mox-platform-meta-core | mox-platform-meta-core | `pub enum AisLayer`; `pub struct CrateMeta` | 元数据核心：AisLayer枚举（L0~L5）、CrateMeta结构体、all_crate_metas()（**需更新为54+ crate列表**） |
| **platform** | domains/platform/core/mox-platform-datastore-core | mox-platform-datastore-core | `trait Datastore`; `trait Migration` | 数据存储核心：多后端抽象（SQLite/PG/MySQL）、方言归一化、迁移引擎 |
| **platform** | domains/platform/core/mox-platform-orchestrator-core | mox-platform-orchestrator-core | `trait Orchestrator`; `trait Reactor` | 编排器核心：DAG编排、事件反应器、鉴权闸门require() |
| **voice** | domains/voice/core/mox-voice-dsp-core | mox-voice-dsp-core | `trait DSPProcessor` | 语音DSP核心：响度归一、软限幅、Aho-Corasick热词、SIMD f32x4加速 |

#### §3.2.3 8域 × Svc层（L3 · 应用服务 · 29 crate）

| 域 | Crate 目录 | package.name | 关键职责 |
|----|-----------|--------------|---------|
| **ai** | domains/ai/svc/mox-ai-agent-svc | mox-ai-agent-svc | AI智能体服务：对话/浏览器自动化/BPMN/MultiAgent/ProviderRegistry + A7 CEM优化 |
| **ai** | domains/ai/svc/mox-ai-expert-svc | mox-ai-expert-svc | ⛨璇玑引擎服务：双璇玑十四维治理/归一化IR/裁决/验证5项/审计三汇/RBAC/租户分层 |
| **ai** | domains/ai/svc/mox-ai-flow-svc | mox-ai-flow-svc | 流程AI服务：9模块（冒险/CPM/冲突/调度/拓扑/代码gen/流水线/原语/可视化） |
| **cloud** | domains/cloud/svc/mox-cloud-master-svc | mox-cloud-master-svc | 云主节点服务：集群管理、节点调度 |
| **cloud** | domains/cloud/svc/mox-cloud-volume-svc | mox-cloud-volume-svc | 云卷管理服务：卷创建/挂载/快照 |
| **cloud** | domains/cloud/svc/mox-cloud-s3-svc | mox-cloud-s3-svc | S3兼容存储服务：对象存储、预签名URL |
| **cloud** | domains/cloud/svc/mox-cloud-filer-svc | mox-cloud-filer-svc | 文件器服务：文件共享、NFS/SMB适配 |
| **data** | domains/data/svc/mox-data-plane-svc | mox-data-plane-svc | 数据平面服务：数据接入、路由、分发 |
| **data** | domains/data/svc/mox-data-etl-svc | mox-data-etl-svc | ETL服务：抽取/转换/加载、数据管道 |
| **data** | domains/data/svc/mox-data-compliance-svc | mox-data-compliance-svc | 数据合规服务：PII检测、脱敏、合规审计 |
| **data** | domains/data/svc/mox-data-catalog-svc | mox-data-catalog-svc | 业务目录服务：6预置FlowGraph+TopologyGraph（政务/财务/客服/ETL/MCP/螺旋） |
| **flow** | domains/flow/svc/mox-flow-operator-wasm-svc | mox-flow-operator-wasm-svc | WASM算子沙箱服务：wasmer执行、热加载插件、沙箱隔离 |
| **flow** | domains/flow/svc/mox-flow-primiflow-svc | mox-flow-primiflow-svc | PrimiFlow服务：解析/代码生成/8类骨架模板/执行/持久化 |
| **flow** | domains/flow/svc/mox-flow-fusion-svc | mox-flow-fusion-svc | PrimiFlow融合服务：六维融合/守恒闸门/Registry/平台编排/12Factor+可观测 |
| **flow** | domains/flow/svc/mox-flow-bridge-svc | mox-flow-bridge-svc | Hermes桥接服务：normalize/recorder/router/拦截注入 |
| **kg** | domains/kg/svc/mox-kg-storage-svc | mox-kg-storage-svc | 图谱存储服务：持久化、索引、事务 |
| **kg** | domains/kg/svc/mox-kg-service-svc | mox-kg-service-svc | 图谱服务：CRUD、查询、遍历 |
| **kg** | domains/kg/svc/mox-kg-streams-svc | mox-kg-streams-svc | 图谱流处理服务：变更流、实时同步、CDC |
| **kg** | domains/kg/svc/mox-kg-spark-svc | mox-kg-spark-svc | 图谱Spark集成服务：大规模图计算、批处理 |
| **kg** | domains/kg/svc/mox-kg-hub-svc | mox-kg-hub-svc | 图谱枢纽服务：混合索引+URN+摄入/推理/治理/影响/热点/闭环 8段 5连接器 |
| **kg** | domains/kg/svc/mox-kg-fusion-svc | mox-kg-fusion-svc | 图谱融合服务：多源图谱融合、RRF结果融合、实体对齐 |
| **market** | domains/market/svc/mox-market-template-svc | mox-market-template-svc | 模板市场服务：发布/加载/评分/排序/Fork/2种子 |
| **platform** | domains/platform/svc/mox-platform-enterprise-svc | mox-platform-enterprise-svc | 企业服务：成员/任务/权限/通信业务编排、多后端SQLite+PG+MySQL |
| **platform** | domains/platform/svc/mox-platform-orchestrator-svc | mox-platform-orchestrator-svc | 编排器服务：DAG调度、事件驱动、跨域编排 |
| **voice** | domains/voice/svc/mox-voice-core-svc | mox-voice-core-svc | 语音核心服务：语音会话管理、状态机 |
| **voice** | domains/voice/svc/mox-voice-asr-svc | mox-voice-asr-svc | 语音ASR服务：语音识别、热词编辑、后处理 |
| **voice** | domains/voice/svc/mox-voice-intent-svc | mox-voice-intent-svc | 语音意图服务：语音指令解析、意图路由 |
| **voice** | domains/voice/svc/mox-voice-operator-svc | mox-voice-operator-svc | 语音算子服务：语音驱动的算子执行、自动化操作 |
| **voice** | domains/voice/svc/mox-voice-desktop-app | mox-voice-desktop-app | **语音桌面应用**（独立产品形态）：全局热键录音、BallWidget、剪贴板/键鼠自动化 |

#### §3.2.4 8域 × Sdk层（L4 · 对外类型 · 6 crate）

| 域 | Crate 目录 | package.name | 关键职责 |
|----|-----------|--------------|---------|
| cloud | domains/cloud/sdk/mox-cloud-sdk | mox-cloud-sdk | 云服务SDK：客户端类型、API绑定 |
| data | domains/data/sdk/mox-data-formula-native | mox-data-formula-native | 公式引擎原生绑定（napi-rs / Node.js FFI） |
| data | domains/data/sdk/mox-data-norm-intent-native | mox-data-norm-intent-native | 归一化意图原生绑定（napi-rs / Node.js FFI） |
| kg | domains/kg/sdk/mox-kg-sdk | mox-kg-sdk | 图谱SDK：客户端类型、查询构建器 |
| platform | domains/platform/sdk/mox-platform-test-harness | mox-platform-test-harness | 测试框架SDK：集成测试工具、mock、测试断言 |
| voice | domains/voice/sdk/mox-voice-dsp-py | mox-voice-dsp-py | 语音DSP Python绑定（PyO3 abi3-py39） |

#### §3.2.5 8域 × Api层（L5 · 域间契约 · 规划中，0 crate）

> **状态**：8个域的 `api/` 和 `svcapi/` 目录已创建，但**零crate**。这是Phase 3的核心任务——填充域间契约，实现依赖倒置。
> **设计意图**：api层定义域间通信的trait/interface/DTO，svc层实现这些trait，其他域通过api层trait调用而非直接依赖svc实现。
> **优先级**：kg/ai/flow 三个核心域先行（Phase 3-1），其余域后续填充。

#### §3.2.6 crate 分布统计

| 层 | crate数 | 占比 | 状态 |
|----|:-------:|:----:|------|
| L0 Foundation | 2 | 4% | ✅ 已实现 |
| L1 Gateway | 1 | 2% | ✅ 已实现（待瘦身） |
| L2 Core（8域） | 15 | 28% | ✅ 已实现 |
| L3 Svc（8域） | 29 | 54% | ✅ 已实现 |
| L4 Sdk（8域） | 6 | 11% | ✅ 已实现 |
| L5 Api（8域） | 0 | 0% | 🔴 规划中（Phase 3） |
| Framework | 1 | 2% | ✅ 已实现（库） |
| **合计** | **54** | **100%** | |

> 注：Cargo.toml workspace 共73 member，含 FFI 绑定（napi/PyO3）、测试 harness、桌面应用等非核心业务 crate。上表统计核心业务 crate 54 个。

**行数量校验**：54/54（Foundation 2 + Gateway 1 + Core 15 + Svc 29 + Sdk 6 + Api 0 + Framework 1）。

> 🔍 **架构一致性校验**：`Cargo.toml` workspace members 必须与本节表格一致；任何新增/删除/重命名 crate 必须同步更新本节 + `mox-platform-meta-core::all_crate_metas()` + `ARCHITECTURE-MIGRATION.md`。
> 📌 **路径铁律（v2.0）**：所有 crate 均位于 `platform/domains/{域}/{层}/` 或 `platform/{foundation,gateway,framework}/`；旧路径 `platform/domains/`、`crates/` 已废弃，禁止在新代码/文档中使用。

### 3.3 模块依赖（6层8域 · 依赖方向）

```
                    ┌─────────────────────────────┐
                    │  L1 Gateway (mox-platform-  │
                    │  gateway-svc)                │
                    └──────────┬──────────────────┘
                               │ 路由分发
          ┌────────────────────┼────────────────────┐
          ▼                    ▼                    ▼
   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
   │ ai/svc      │    │ kg/svc      │    │ platform/svc│
   │ (3 crate)   │    │ (6 crate)   │    │ (2 crate)   │
   └──────┬──────┘    └──────┬──────┘    └──────┬──────┘
          │ 依赖同域core       │ 依赖同域core       │ 依赖同域core
          ▼                    ▼                    ▼
   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
   │ ai/core     │    │ kg/core     │    │ platform/core│
   │ (2 crate)   │    │ (2 crate)   │    │ (5 crate)   │
   └─────────────┘    └─────────────┘    └─────────────┘
          │                    │                    │
          └────────────────────┼────────────────────┘
                               ▼
                    ┌─────────────────────┐
                    │ L0 Foundation        │
                    │ (2 crate)            │
                    └─────────────────────┘
```

**跨域依赖规则（Phase 2 强制执行）**：
1. svc 层只能依赖同域 core + 其他域的 core/sdk/api（**禁止直接依赖其他域的 svc**）
2. core 层只能依赖 foundation + 其他域的 core/sdk（**禁止依赖任何 svc**）
3. sdk 层只能依赖同域 core 的类型定义
4. api 层（规划中）= 域间契约，只能依赖 core 的类型，定义 trait 供 svc 实现
5. gateway 只能依赖各域的 svc（通过 api trait 或直接调用，Phase 2 后改为 api trait）
6. 所有跨域调用必须通过 api 层 trait（依赖倒置），Phase 3 完成后强制执行

### 3.4 服务职责（按域 · 单一职责）

| 域 | 核心职责 | 不负责 | 主责联盟 |
|----|---------|--------|---------|
| **ai** | AI意图识别、智能体编排、璇玑专家验证、流程AI代码生成 | 图谱存储、数据持久化、云基础设施 | 算法联盟 R + 开发联盟 C |
| **kg** | 知识图谱存储/算法/服务/流处理/融合/枢纽、八大算法家族A1~A8 | AI推理、业务编排、用户界面 | 算法联盟 R + 开发联盟 C |
| **flow** | 算子内核、WASM沙箱、优化器(CPM/RCPSP/CEM)、PrimiFlow解析/代码生成/融合、Hermes桥接 | 图谱算法、AI对话、数据ETL | 开发联盟 R + 算法联盟 C |
| **data** | 公式引擎、数据归一化、数据标准、ETL、数据合规、业务目录 | 图谱存储、AI推理、算子执行 | 开发联盟 R |
| **platform** | 系统核心(成员/任务/权限/通信)、IAM、元数据、数据存储(多后端)、编排器、企业服务 | 业务领域逻辑、AI算法、图谱算法 | 开发联盟 R |
| **cloud** | 云主节点、卷管理、S3存储、文件器 | 业务逻辑、AI、图谱 | 开发联盟 R（运维） |
| **market** | 模板市场(发布/加载/评分/Fork) | 业务流程执行、AI推理 | 产品联盟 R + 开发联盟 C |
| **voice** | 语音DSP、ASR、意图、算子、桌面应用（**独立产品形态评估中**） | 核心平台业务、图谱、AI编排 | 开发联盟 R（独立团队） |

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

### 6.1 对外接口（按域归属 · 网关统一路由）

| 类别 | 端点 | 归属域/svc | 说明 |
|------|------|-----------|------|
| 健康检查 | `GET /api/health` | gateway | 探针 |
| 身份 | `POST /api/bootstrap`、`GET /api/me` | platform/enterprise-svc | 建璇玑/令牌解析 |
| 成员 | `POST/GET /api/members` 等 | platform/enterprise-svc | 邀请/列表/状态 |
| 任务 | `POST/GET /api/tasks`、`POST /api/tasks/:id/transition` 等 | platform/enterprise-svc | 全生命周期 |
| 通信 | `GET /api/channels`、`POST /api/channels/:id/messages` | platform/enterprise-svc | 频道消息 |
| 实时 | `WS /api/ws?token=` | gateway + platform | 通知推送 |
| 融合 | `POST /api/mox/optimize`、`/publish` | ai/expert-svc + flow/fusion-svc | 璇玑治理→上架 |
| 图谱 | `GET /api/graph`、`POST /api/graph/node`、`POST /api/graph/edge` | kg/hub-svc + kg/service-svc | 图谱CRUD/查询 |
| 算子 | `GET /api/operators`、`POST /api/execute` | flow/operator-wasm-svc + flow/primiflow-svc | 算子列表/执行 |
| 商城 | `GET /api/market/`、`POST /api/market/upload`、`POST /api/market/:id/clone` | market/template-svc | 算子商城 |
| **AI统一入口** | `POST /ai/engine/process` | gateway → ai/agent-svc（A5意图路由） | 自动意图识别→能力路由 |
| **AI统一入口** | `POST /ai/engine/analyze` | gateway → ai/agent-svc | 显式能力执行 |
| **AI统一入口** | `GET /ai/engine/capabilities` | gateway → ai/agent-svc | 能力矩阵自描述 |
| **AI统一入口** | `GET /ai/engine/metrics` | gateway → observability | 三联盟SLO指标 |

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

#### 7.1.1 Gateway crate 聚合内部架构（L1 Gateway · v2.0）

**L1 Gateway（路由+横切中间件，仅做薄层）：**
- `src/routes/mod.rs` 路由总入口：按域挂载子路由（ai/kg/flow/platform/data/market/cloud/voice）
- `src/handlers/mod.rs` HTTP 处理器薄层：纯 request→response 适配，不含业务算法
- `src/main.rs` 二进制入口 `operator-server`（axum server 启动 + 生命周期 + 优雅停机）
- `src/middleware/`：RBAC鉴权、限流、CORS、日志、trace_id注入
- `src/ws/`：WebSocket 握手 + 消息路由（HITL审批、实时通知）
- `src/openapi.rs`：OpenAPI schema 生成（从各域svc的API定义聚合）

**与旧 runtime 的区别（v1.1→v2.0）**：
- 旧 runtime 是"上帝crate"：聚合16子服务 + Cordis5插件内核 + RBAC + OpenAPI + 迁移引擎 + 治理
- 新 gateway 仅做路由+横切中间件，业务聚合下沉到各域svc层
- Cordis5 插件内核 → 迁移至 `mox-framework`（Framework层）
- 迁移引擎 → 迁移至 `mox-platform-datastore-core`（platform域core层）
- 治理逻辑 → 迁移至 `mox-ai-expert-svc`（ai域svc层）

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

**方言归一化实现**：upsert 语义按后端生成——SQLite `INSERT OR REPLACE` + `?N`；PostgreSQL `ON CONFLICT DO UPDATE` + `$N`；MySQL `ON DUPLICATE KEY UPDATE` + `?`。落点 `platform/domains/platform/core/mox-platform-datastore-core/src/`（`schema.rs` 方言层 + `sqlite.rs`/`postgres.rs`/`mysql.rs` 驱动层）；业务编排在 `platform/domains/platform/svc/mox-platform-enterprise-svc/`。

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
| ADR-08 | **6层8域DDD矩阵架构迁移**（v2.0）：从旧15-crate扁平模型（platform/domains/）迁移至新6层8域DDD矩阵（platform/domains/{8域}/{core,svc,sdk,api,svcapi} + foundation/gateway/framework） | 旧架构crate混合领域逻辑与基础设施，难以独立测试和复用；DDD分层实现依赖倒置，模块化单体支持未来微服务演进 | 所有文档需同步更新路径引用（Phase 1完成）；api层待填充（Phase 3）；跨域依赖规则待强制执行（Phase 2） |
| ADR-09 | **跨域依赖规则**：svc禁止直接依赖其他域svc，必须通过api层trait（依赖倒置）；core禁止依赖任何svc | 防止"大泥球"反模式，保持域边界清晰，支持未来独立部署 | api层当前为空（0 crate），过渡期允许svc间直接依赖但需登记；Phase 3完成api层后强制执行arch test |
| ADR-10 | **voice域定位决策**：voice域含桌面应用（mox-voice-desktop-app），与核心平台业务关联度低，评估独立为单独workspace或保留为"垂直能力插件" | 避免核心平台被语音产品的发布节奏和桌面GUI依赖（cpal/screenshots/enigo/global-hotkey）拖累 | 待Phase 3完成依赖分析后决策；若保留则明确voice域不参与核心平台发布周期 |
| ADR-11 | **网关瘦身**：mox-platform-gateway-svc仅做路由+横切中间件（鉴权/限流/CORS/日志/WS），业务聚合下沉到各域svc层或BFF | 旧runtime是"上帝crate"（聚合16子服务+Cordis5+RBAC+OpenAPI+迁移+治理），职责过重难以维护 | Cordis5迁移至mox-framework；迁移引擎迁移至mox-platform-datastore-core；治理逻辑迁移至mox-ai-expert-svc |
| ADR-12 | **可观测性体系化**：建立统一observability foundation（tracing+prometheus metrics+opentelemetry tracing），所有svc crate强制接入 | 旧架构指标未采集、跨crate追踪未实现，不符合企业级可观测性要求 | Phase 2完成；observability纳入mox-platform-foundation或独立crate |

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

# 璇玑 RelGraph · 全栈产品手册（产品设计 · 优化方法论 · 介绍体系 · 最优使用）

> 版本：v3.0 企业级 · 验证基线：129 GREEN（零回归） · RPO=0 · 可用性 99.95%
> 关联代码：`platform/backend-node/src/*` · `platform/gateway/runtime/src/*`
> 验证报告：`.trae/specs/20260823-mox-storage-distributed-ai-unified-query/review.md`（A+ 评级）

---

# 一、产品设计

## 1.1 核心价值定位（一句话版本）

> **璇玑 RelGraph = 以 Rust 自研高性能知识图谱为中枢的「需求↔架构↔业务流程↔模块↔文档↔本地代码」归一化关联系统。**
>
> 把团队的"业务认知"固化为一张可被算法、可被 AI、可被开发者共同查询和演化的图谱，AI 查询的体验等同于本地数组查询，所有文档、代码、流程都通过图谱自动同步。

核心价值不是"又一个图数据库"，而是：

1. **归一化解决"知识散落在 6+ 地方"的根痛点**：需求文档（Confluence）、架构图（Mermaid）、业务流程（BPMN）、代码仓库（本地 AST）、模块注册（modules.json）、操作手册（Wiki）—— 用一张图谱统一建模、统一查询、AI 统一入口。
2. **AI 查询 = 本地查询体验**：一个 SDK 方法/ 一个 `/ai/engine/process` 入口，调用方不用区分"走 AI 推理还是本地 Postgres"。
3. **全链路可追溯**：任意改动 → CDC 失效 → 图谱增量更新 → 文档节点反向链接 → 团队能回答"改动这条代码，会影响哪些需求/文档/流程"。

## 1.2 目标用户分层

### L1 · 企业研发团队（MVP 主目标）
- **典型画像**：50-500 人规模的产品公司研发中心（产品经理 + 架构师 + 后端/前端 + 测试 + SRE）
- **刚需**：需求↔代码不一致导致返工；迭代速度慢（"你问我改了什么我要翻 3 天 Git Log"）
- **采购决策链路**：CTO/研发总监（决策者）→ 架构师（技术验证者）→ 团队 Lead（使用者）
- **对应功能**：项目全息图谱 Atlas（项目节点 → 需求节点 → 文档节点 → 代码实体节点）、Trace 图谱闭环（每次发布 W-step-target 三跳可达）、三流程端点（graph_bulk/file_upload/ai_rag）

### L2 · 算法联盟 / 专家联盟（璇玑内置长期路线）
- **典型画像**：多模型协作（代码专家 / 架构专家 / 测试专家 / 安全专家 / 文档专家）
- **刚需**：多模型辩论结果缺乏可审计锚点；专家权重学习慢；同一问题上下文分裂
- **对应功能**：`alliance_intent_priors.json` + 双专家辩论合成共识 + 意图分类器两级兜底（关键词 → 激活扩散个性化 PR d=0.85, 30 轮）+ CEM 引擎对联盟权重做统一寻优（停止 σ̄<0.06 或 3 轮无改进）

### L3 · 开源开发者（社区生态建设）
- **典型画像**：独立开发者、高校研究人员、中小团队的基础设施维护者
- **刚需**：想在本地跑通；想二次开发 StorageProvider / ChunkBackend / 图算法插件
- **对应功能**：开源版所有代码（T1 DualWrite / T2 FS chunk backend / T4 算法 CNM / T5 PageRank / T7 Node internal endpoints）、Plugin Operator 扩展接口 `registered_plugins.json`

### L4 · 二次开发使用者（商业集成商 / 行业 ISV）
- **典型画像**：为金融/制造/政务等行业客户做行业化定制落地的软件公司
- **刚需**：把 RelGraph 作为"知识中台组件"嵌入既有 IaaS/PaaS，对接已有 IAM、审计、CMDB
- **对应功能**：企业增强 SSO（OIDC/SAML）、Postgres+Citus 分片、审计日志 ClickHouse、行业化行业知识模板（可插拔 graph_bulk seed）、SDK Node/Python/Rust 三语言契约

## 1.3 核心痛点拆解（9 根因 + 璇玑化解法）

| # | 真实痛点（代码/业务证据） | 根因分析 | 璇玑解 |
|---|---|---|---|
| 1 | "需求改了，但旧文档/旧代码没有同步更新" | 需求、文档、代码三者没有唯一 ID 关联，变更事件没有 CDC 传播 | **graph_bulk 三流程 + CDC 事件总线**：每一次保存节点/文件都触发 `graph:{node,file}_updated` → Postgres 实体表、Redis 邻接缓存、文档联动节点三路失效 → `DLQ + 5min 对账修复` |
| 2 | "AI 问出来的和本地查出来两套结果" | AI 与本地 API 两套路径、两套认证域、两套缓存 | **统一语义网关 /ai/engine/process**：四端点 (process/analyze/capabilities/metrics) 协议兼容——`data 段 shape 超集等价`（本地 9 字段全在 AI 返回里，AI 附加字段全部以 `ai_*` 前缀出现） |
| 3 | "图数据库性能差，10 万节点开始崩" | SQLite JSON 宽表 → 全表序列化 I/O 放大；度分布倾斜节点 O(N) | **Nebula 分片 64（project_domain 哈希）** + Raft 3 副本 W=2；企业锚点 100W 节点 3 跳邻居 420ms；算法跑分布式查询而非 Node 单线程 |
| 4 | "存的文件丢了一块盘就全挂" | 本地 FS 无副本无纠删码 | **MinIO EC:4+2 可容忍 2 节点掉线**，T14 验证 kill 2 重建 CRC 100% 一致（RPO=0 200/200 随机）|
| 5 | "架构师画的 Mermaid 与工程师写出来的代码不一样" | 流程图没有先节点后边的构建约束 + 没有校验挂钩 | **流程图谱构建硬性顺序：Node upsert → byId.has → RAW 边展开**；目标节点缺失返回 `missing=[id]` 不静默丢；构建完 Trace W→S→T BFS 全可达验证 |
| 6 | "社区检测算法结果飘忽、A 跑 B 跑不同" | 之前默认 LPA（不稳定，随机性强） | **项目记忆锁定 CNM 模块度贪心凝聚 + Brandes 介数 + Harmonic 紧密**，任何人调用都一致，禁止 toFixed 截断精度 |
| 7 | "意图识别老错，我问的是文件，它返回代码" | 关键词匹配过于粗糙，单一兜底 | **两级兜底**：① 关键词分类器（ms 级） ② 激活扩散意图→能力微图谱做个性化 PageRank（d=0.85,30 轮，输出能力概率分布），AC-10 路由做静态→参数少→同参数长路径优先 |
| 8 | "上线一次要回滚 3 天，切新服务没信心" | 没有灰度策略/就绪探针/预热闭环 | **金丝雀 [1,10,50,100] 权重 4 阶段 + 回滚 <30s** + 就绪探针 `warmup_complete ∧ pg_stat_statements_hit_rate ≥ 0.85` 否则 Gateway 返回 503 不放流量 + 预热三步（PR hot/语义缓存 seeds/L1 邻接） |
| 9 | "我是个人开发者，没法起一整套 Nebula/MinIO/K8s 集群" | 企业部署对个人开发者太沉重 | **分层可插拔：开发模式走 MemoryProvider**；测试全部使用 Memory 后端（12 套件全在内存运行通过），生产再切 Postgres/MinIO/Nebula |

## 1.4 产品能力边界（坚持"做厚做轻可拆分"）

### ✅ 做厚的核心能力（璇玑护城河，必须自研）

| 能力域 | 做厚理由 | 实现载体 |
|---|---|---|
| 知识图谱 CRUD 引擎 + 算法集 | 算法归一化是璇玑的差异化核心（CNM/Brandes/Harmonic/RAW/PR 转置/激活扩散 全部锁死项目记忆常量） | `graph/graph-formulas.js` · Nebula Adapter |
| 统一语义网关 AI 路由决策 | 企业路由语义（静态→参数少→长路径）+ 两级意图兜底（关键词+激活扩散）+ 能力矩阵缓存 | Rust `ai_router.rs` · Node `routes/internal.js` |
| Trace 图谱闭环（W-step-target） | 没有闭环就没有"璇玑=可审计可回溯"的品牌标识 | `three-flows-trace-e2e _appendTrace` · 三流程端点 |
| 三流程端点 graph_bulk / file_upload + link / ai_full_rag | 这是璇玑最核心的业务入口，所有能力都围绕它展开 | `test-three-flows-trace-e2e.js` 的参考实现（可直接投产） |
| 协议 shape 等价层 | 这是"AI 查询 = 本地查询体验"的铁律 | `list_nodes` 返回**双向别名**（createdAt/created_at 同时存在）+ `ai_*` 前缀隔离 |

### 🪶 做轻的周边能力（可替换 / 可项目化剥离）

| 能力域 | 做轻策略 | 替换接口 |
|---|---|---|
| 对象存储 ChunkBackend | 不自研文件系统：抽象 5 方法接口，FS/S3/Azure 三种实现 | `storage/chunk-backend.js` IChunkBackend（`put/get/delete/has/abortMultipart`） |
| 存储元数据底层 | 不自研关系型存储：抽象 StorageProvider（SQLite/Memory/Postgres），接口 22 方法 | `storage/index.js` StorageProvider 抽象类 |
| 向量相似度 | 不自研 ANN：用 pgvector HNSW，未来可换 Milvus | `K-V KVStore.kvSet('rag-cache:...')` 统一入口 |
| LLM 推理 | 不自研大模型：通过 `llm_config.json` 可插拔 Provider | `ai-engine-core.js` 可替换模型路由 |
| 部署底座 | 不自研 K8s：支持 Helmfile/Kustomize/单机二进制三种形态 | `test-sdk-gray-warmup-summary.js` 的 rolloutPlan/readinessProbe/warmupRun 参考脚本 |

### 🚫 坚决不做的能力（规避过度设计 / 功能堆砌）

- ❌ 不做前端富编辑器（不做 Figma 替代 / 不做 Word 在线编辑）
- ❌ 不做独立的消息队列（复用 Redpanda/Kafka 生态；开发模式走内存 EventBus）
- ❌ 不做 IAM/SMS/邮件/审批流等通用中台能力（一律 OIDC 集成企业 IdP）
- ❌ 不做"通用 BI 报表"：图谱统计分析输出通过 `/ai/engine/metrics` 暴露，而不是造 ECharts 编辑器
- ❌ 不做"搭积木式的代码无代码生成器"：代码生成只在 `ai/engine/process {intent:"code_gen"}` 内作为专家能力，不暴露复杂 GUI。

## 1.5 功能模块划分（可按项目化拆分打包）

```
璇玑 RelGraph 模块地图（按依赖序，← 可拆分打包）
├─ 核心底座层（所有版本必选）
│   ├─ G01 图谱存储抽象 (StorageProvider: SQLite/Memory/Postgres)
│   ├─ G02 对象分块抽象 (IChunkBackend: FS/S3)
│   ├─ G03 算法归一库 (GraphFormulas: CNM/Brandes/Harmonic/RAW/PR×2/activateSpread/CEM/density)
│   ├─ G04 CDC 事件总线 + L1 邻接缓存 + DLQ 对账
│   └─ G05 远程图谱驱动适配层 (RemoteGraphDriver: Gremlin/Mock)
├─ 网关接入层（企业版推荐，开源可选）
│   ├─ G11 Rust Gateway: 四端点 /ai/engine/*
│   ├─ G12 AI 路由语义表（AC-10）
│   └─ G13 Node Sidecar 通信 + 本地降级兜底
├─ 业务流程层（璇玑差异化主模块）
│   ├─ G21 graph_bulk 流程（先节点后边 RAW + CNM/PR 增量）
│   ├─ G22 file_upload + 图谱关联（自动 F-{fileId} 节点双向边）
│   ├─ G23 ai_full_rag（激活扩散 TopK + 文件召回 + RRF 融合 + 双专家辩论 + 语义缓存回填）
│   └─ G24 Trace 图谱自动构建（W-workflow→S-step→T-target 三跳 BFS 可达）
├─ 内部端点层（供 Sidecar/Operator 调用）
│   ├─ G31 /internal/intent（两级意图兜底）
│   ├─ G32 /internal/graph-algo（list_nodes/pagerank/cnm/betweenness/closeness/list_files/spread_activate）
│   └─ G33 /graph/search（激活扩散重排 spread_weight=0.7 默认）
├─ SDK & 编排层
│   ├─ G41 createMoxClient 三语言（Node/Python/Rust）契约（graph.list & ask）
│   ├─ G42 LRU(1K/60s) 缓存 + 429/503 指数退避 + max_latency_ms 熔断
│   └─ G43 rolloutPlan（1/10/50/100）+ readinessProbe + warmupRun
├─ 企业增强层（仅企业增强版）
│   ├─ E01 Postgres+Citus 分片 + 高可用部署 Helm Chart
│   ├─ E02 Nebula 生产集群 3+3+9 配置 + 跨 AZ + 冷备
│   ├─ E03 MinIO 纠删 EC 跨地域 CRR
│   ├─ E04 OIDC/SAML SSO + RBAC 审计日志 (ClickHouse 列存)
│   ├─ E05 行业知识图谱 Seed Bank（金融/制造/政务种子实体）
│   └─ E06 高可用演练工具包（T14 企业故障注入包：FI 脚本 + RTO 监控仪表盘）
└─ 社区开源版不包含但可插件化扩展
    ├─ registered_plugins.json 扩展点
    └─ operators.json 17 个 Operator 注册（AI 执行原子）
```

**拆分打包规则**：
- 开源版 = G01~G05 + G31/G32/G33 + G21~G24（算法核心 + 流程核心 + 本地端点）
- 企业基础版 = 开源版 + G11~G13（Rust 网关接入）+ G41~G43（SDK 编排）
- 企业增强版 = 企业基础版 + E01~E06（生产高可用、审计、行业种子、HA 工具）

## 1.6 非功能需求（NFR · 有代码基线可锚点）

| 维度 | 指标 | 证据 |
|---|---|---|
| **性能** | 100W 节点 10M 边 · 3 跳邻居 P95 ≤ 420ms · 整图 PageRank ≤ 8.5s · 本地命中 P95 ≤ 50ms · AI 混合 P95 ≤ 1000ms | T13 `scaleAnchor100W()` + latencyBudgetBreakdown 验证 |
| **稳定性** | RPO=0（CRC 200 次随机 kill-2 100% 一致） · RTO ≤ 25s（Gateway Pod crash） · 年停机 ≤ 4.38 小时（99.95%） | T14 `haCrc200()` + SLA 常量稳定性 |
| **可扩展** | StorageProvider/ChunkBackend/RouterTable 三处插件点；分片键按 project_domain 哈希；可跨节点横向扩 storaged 至 9+ 副本 | `StorageFactory.create` / `IChunkBackend` 接口 / `RouterTable.register` |
| **可部署** | 开发 0 依赖（MemoryProvider）；单机 SQLite+FS；生产 Helm3/Kustomize 单命令部署；金丝雀 4 阶段 + 30s 回滚 | `rolloutPlan()` + readinessProbe/warmupRun |
| **可二次开发** | 22 法 StorageProvider 抽象接口；Node/Python/Rust SDK 1:1 契约；所有 Operator 可插拔 registered_plugins.json | `StorageProvider 接口` + T9 三语言契约断言 |

## 1.7 技术底座约束

> 后端 Rust 全维自研：统一入口网关、路由语义表、Sidecar 通信、可观测探针、灰度权重 全部用 Rust Axum 实现（性能 & 内存安全 & `unsafe=0`）。
> 参考 AIS 项目分层架构：**接入层（Rust Gateway）→ 编排层（Node/Rust 同构服务 + 专家联盟 + 意图 + RRF + 辩论合成）→ 存储层（图谱域 Nebula / 对象域 MinIO / 元数据域 Postgres + Citus / Redis L1 / pgvector KV）→ 运维底座（K8s Operator + OTel + 冷备 + DR）**。
>
> 以知识图谱作为中枢，需求、架构、业务流程、模块、文档、本地代码 六域实体归一化关联——每个域均映射为图谱的独立 Layer：
> - **Layer 1（需求域）**：`entity_type = "Requirement"` / `"UserStory"`，字段：`id, priority, owner, linkedDocIds[]`
> - **Layer 2（架构域）**：`entity_type = "Component"` / `"Interface"`，字段：`tech_stack, owned_by, upstream/downstream`
> - **Layer 3（流程域）**：`entity_type = "Workflow"` / `"TraceStep"` / `"TraceTarget"`，字段：`workflow_id, step_index, rel`
> - **Layer 4（模块域）**：`entity_type = "Module"` / `"Operator"` / `"Plugin"`，字段：`name, entry, cap[]`
> - **Layer 5（文档域）**：`entity_type = "Doc"` / `"File"`，字段：`hash, version, linkedGraphIds[]`
> - **Layer 6（代码域）**：`entity_type = "CodeEntity"`（类/函数/模块 AST 节点），字段：`file_path, line_start, line_end, symbol_type`
> 跨层关联通过 RAW 双向边：`Requirement("R1") --[:implemented_by]--> CodeEntity("F123")`。

## 1.8 开源版 vs 企业增强版 能力边界

| 能力分类 | 开源版 (AGPL-3.0) | 企业增强版（商业许可） |
|---|---|---|
| 图谱算法归一（CNM/Brandes/Harmonic/PR×2/激活扩散/CEM 停止） | ✅ 全包含 | ✅ + **算法增量版**（变化边 ≤10% 增量 CNM/PR 而非整图重算） |
| 存储抽象（SQLite/Memory/Postgres 单实例） | ✅ | ✅ + **Postgres+Citus 分片（按 entity_type 或 tenant 哈希）** |
| ChunkBackend（本地 FS + S3 客户端） | ✅ | ✅ + **MinIO 生产集群部署脚本 + 生命周期冷沉降 + 30 天软删保留期** |
| Nebula Adapter（L1 缓存 + CDC 失效 + 读端优先） | ✅ 可选（`USE_NEBULAGRAPH=true`） | ✅ + **Nebula 3+3+9 高可用 + 跨 AZ + 15 分钟 WAL 归档** |
| 统一语义网关 Node internal endpoints（/internal/intent, graph-algo） | ✅ | ✅ + **Rust Gateway 四端点 + Sidecar（生产级限流/RBAC/SLA 监控）** |
| 三流程端点（graph_bulk/file_upload+link/ai_rag） | ✅ 参考实现（测试） | ✅ + **生产级速率限控 + 幂等 key + 审计日志 ClickHouse 列存** |
| Trace 图谱三跳 BFS 验证 | ✅ | ✅ + **发布仪表盘：每次发布的 Trace 图谱可可视化导航** |
| 激活扩散 graph/search 重排 | ✅（spread_weight 默认 0.7） | ✅ + **跨分片 fan-out 查询汇聚 + TopK 邻接 Redis 预热** |
| SDK（Node/Python/Rust 契约） | ✅ 仅开源契约文档 + Node 参考实现 | ✅ + 三语言官方 SDK + 企业 SLA 支持 |
| 灰度/就绪探针/预热脚本 | ✅ 参考 | ✅ + Helm3 Chart + OTel 全链路 + 故障演练 FI 工具包 (T14 200×CRC) |
| SSO/RBAC 审计 | ❌（仅本地 auth） | ✅（OIDC/SAML + 分级权限 + 审计链路可查询） |
| 行业知识图谱 Seed Bank | ❌ | ✅（金融/制造/政务/医疗 可选行业包） |
| 技术支持（SLA） | ❌ 社区 GitHub Issues | ✅ 7×24 / 9×5 两档 SLA，RTO 承诺 60 分钟 / 4 小时 |

**核心原则：** 开源版 = **璇玑的全部核心能力都可本地跑通**（12 套件全 GREEN，无需任何企业依赖）。企业增强版 = **生产级运行能力**（高可用、多租户、审计、性能增量优化、行业化、SLA 支持），不加入新的"璇玑独有能力"，保证社区用户不会因为能力边界差而被"套牢"。

---

# 二、产品迭代优化方法论（全维度闭环优化体系）

## 2.1 六维优化闭环 + 图谱反向同步规则

璇玑的优化不是"拍脑袋改个功能"，而是 **"反馈 → 归因 → 验证 → 图谱反向同步"** 的全链路闭环：

```
┌────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ 需求侧优化 │   │ 架构技术优化 │   │ 业务流程优化 │   │ 开发者体验优化│
└──────┬─────┘   └──────┬───────┘   └──────┬───────┘   └──────┬───────┘
       │                │                  │                  │
       └────────────────┴────────────┬─────┴──────────────────┘
                                     ▼
                              ┌──────────────┐
                              │ 验证层（必须）│
                              │1. 测试 129 GREEN 零回归 │
                              │2. FI 200×CRC RPO=0      │
                              │3. Trace W→S→T BFS 5/5 可达│
                              │4. 三流程端点 shape 等价 │
                              └───────┬──────┘
                                      ▼
                           ┌───────────────────────┐
                           │ 图谱反向同步（强制步骤）│
                           │ 每一次优化必须反向写回：│
                           │ ・Requirement 需求节点  │
                           │ ・Architecture 架构节点  │
                           │ ・Workflow 流程节点     │
                           │ ・Module/Operator 模块点│
                           │ ・Doc 文档节点（更新版本）│
                           │ ・CodeEntity 代码节点   │
                           └───────────────────────┘
```

### 2.1.1 需求侧优化
- **输入来源**：用户反馈表单 + GitHub Issues + `/ai/engine/metrics` 的 route 决策命中率（capability 是否经常走 fallback）+ 专家联盟的 `debate-synthesis` 不一致率。
- **处理步骤**：
  1. 按 AIS 项目规范 → 每个需求必须绑定 `Requirement-XXX` 图谱节点（linkedGraphIds 包含需求文档 File 节点）。
  2. 用 `CEM 寻优` 的四目标加权 `0.55Q+0.2S+0.1T+0.15Stability` 对需求候选做排序（非靠拍脑袋 PRD）。
  3. 需求实现完成后，必须追加 Trace 图谱：`W-需求迭代 → S-开发 / S-测试 / S-发布 → T-新功能节点`，BFS 验证可达后需求才能关闭。

### 2.1.2 架构技术侧优化
- **输入来源**：T13 时延预算分解（如果分量越界 800ms AI pipeline 经常超）+ Rust 编译警告（dead_code/warnings）+ 后端 GC 内存分布。
- **触发规则**：
  - **小迭代**：单一模块性能瓶颈（例如 `/ai/engine/metrics` 显示本地命中占比 <20%）→ 新增 L1 邻接缓存键，不需要重构。
  - **架构级重构触发**：满足**任一条**：
    1. 同一模块连续 3 个版本出现越界分量。
    2. 故障演练发现 FI 后单次恢复 RTO > 120s（超过红线 2×RTO 上限 60×2）。
    3. 图谱反向同步的 `Component` 节点之间关联数 > 80%（意味着"耦合爆炸"）。
    4. 小迭代做了 6 次仍未达到 P95 预算目标。

### 2.1.3 业务流程优化
- **输入来源**：三流程端点的 `timing_ms` / `warnings[]` / `missing` 列表 + Trace BFS 失败率（W-workflow 到 target 不可达比例）。
- **抓手**：每次流程优化前后做"流程图谱镜像"，CNM 社区检测对比 — 如果"社区模块度 Q 下降超过 0.05"则流程重构是拆错了，回滚。
- **强制绑定**：每一次流程改动必须同步更新 `Doc-流程操作手册` 文档节点的 `version` + 反向绑定 Workflow 节点。

### 2.1.4 开发者体验优化（DX）
- **输入来源**：SDK `max_latency_ms` 熔断触发率 + 新开发者 onboarding 跑通 `memory provider` 模式所需时间 + GitHub `good-first-issue` 标签解决率。
- **优化抓手**：
  1. 所有对外 API 以 `shape 等价` 铁律为第一原则（老代码零改原则上永远不破坏）。
  2. 新开发者 clone 仓库 → 0 依赖下 `node test/test-three-flows-trace-e2e.js` 必须 10s 内出 GREEN（使用内存后端）。
  3. 新增一个 Operator 必须有 `registered_plugins.json` 注册项 + `test/plugins-register.js` 用例，否则不允许合入。

### 2.1.5 社区体验优化
- **输入来源**：GitHub Star 增速 / Issue 中位数关闭时长 / PR 合入时长 / 问答渠道讨论热词。
- **优化抓手**：
  1. 每月固定做一次"社区 Issue 图谱构建"：Top30 Issue 建 Issue 节点，linkedGraphIds 绑定对应 Requirement/Bug 节点。
  2. `README.md` 必须**每次合入 master 自动与 Graph Doc 节点版本对齐**（语义哈希校验）。
  3. 社区贡献算法插件要过"算法精度护栏审查"：CNM/Brandes/Harmonic/PR 接口必须与基准值误差 <1e-4，否则被"精度护栏"拒绝合入（T4 护栏机制复用）。

### 2.1.6 性能稳定性优化
- **输入来源**：T14 故障注入结果 + `/ai/engine/metrics` P95/P99 延迟 + Rust dead_code 警告。
- **抓手**：
  1. 每次性能优化后必须跑 `TR-10.5`（4 故障 × writeset CRC 前后不变）——**性能优化不能牺牲 RPO**。
  2. SLO 越界告警自动开 Issue：`99.95%` 每周可用性低于阈值自动开 Issue 绑定 `Component: 对应模块`。
  3. 稳定性里程碑：每 6 个月做一次"全量故障演练"（64×FI 场景 × EC 随机 kill × storaged 随机 kill × Sidecar/Gateway 多故障组合），输出一次 HA 白皮书。

## 2.2 反伪需求识别（五问法，任一命中则 REDUCE 优先级到 P3 以下）

对每一个需求/Issue，必须先过以下五道反伪需求筛查（五道都能过才正式进入候选）：

1. **「同功能三用户」测试**：是不是三个或以上**不同组织**的独立用户都提了类似的需求？如果只有一家客户单点提出 → P3（行业化定制，不进通用璇玑）。
2. **「图谱绑定」检查**：这个需求能不能绑定到 6 层（需求/架构/流程/模块/文档/代码）中至少两层节点？如果只是"加一个按钮颜色"—— 没有图谱可绑定的就是 UI 定制，不是璇玑产品层。
3. **「复杂度守恒」检查**：如果引入这个需求，会让"核心能力变厚"还是"周边能力变厚"？璇玑的原则是核心做厚周边做轻——如果让周边模块（对象存储、部署底座、IAM）变复杂 → 走插件化，不进主线。
4. **「回滚路径」检查**：如果用户用了这个新功能，一周后后悔不想用了，能不能一行配置（例如 `STORAGE_PROVIDER=sqlite` / `USE_NEBULAGRAPH=false`）切回老状态？不能 → 功能必须设计成可无副作用开关。
5. **「替代方案」检查**：能不能用现有 CEM 优化 + 调参解决，而不需要加新代码接口？例如"我要更快的社区检测"——可能是分片策略问题，不是算法接口问题 → 先排查分片再改算法。

## 2.3 优先级判断（CEM 四目标加权 + RICE 校准）

通过反伪需求筛查的候选，使用 `CEM cemOptimize` 的加权分做基础排序，再人工 RICE 校准：

| 维度 | 权重 | 量化方式 |
|---|---|---|
| Q (Quality) 解决问题的深度 | 0.55 | 用户反馈分数 + Trace BFS 可达率提升 × 100 |
| S (Scope) 需求范围可控度 | 0.20 | 需要改动 ≤3 个模块 100 分；>10 模块 20 分 |
| T (Time) 开发交付速度 | 0.10 | 1 周内 100 分；1 月 60 分；>1 季 10 分 |
| Stability 稳定性影响 | 0.15 | FI 200×CRC 继续全过 100 分；如果需要加新故障模式 40 分 |

加权分 ≥ 0.80：**P0 必须进下一个迭代**；0.50–0.80：**P1 进迭代候选池**；0.30–0.50：**P2 排期待定**；< 0.30：**P3 积压或作为社区 Good First Issue**。

**CEM 停止条件严格遵守**：σ̄<0.06 或 3 轮无改进。

## 2.4 版本迭代规划（小迭代 vs 架构重构的触发）

璇玑不做"大版本砸锅式发布"，坚持 4 期可控迭代（Spec 规范的 P1~P4）+ 月度小迭代：

| 版本类型 | 周期 | 包含内容 | 触发重构条件 |
|---|---|---|---|
| **月度小迭代（.patch）** | 4 周 | P0/P1 小功能 + Bug 修复 + 性能微优化 + 文档修正 | 永不触发架构重构 |
| **季度发布（.minor）** | 12 周 | 新增 1-2 个新模块（新增一个 Operator；新增一个 ChunkBackend 插件） | 架构 2.1.2 四条触发条件任一命中时，必须走独立"架构重构 Spec"流程，不可在季度 minor 中偷偷改 |
| **年度发布（.major）** | 48 周 | 核心算法升级、底座升级（例如新增 pgvector HNSW 余弦索引；新增 Rust Gateway 热迁移端点） | 提前 2 个季度做基线对齐；必须跑 12 套件 × 3 种部署（SQLite/Postgres/Nebula）；发布前做全量 FI 演练 ≥ 2 次 |

### 触发架构级重构的硬条件（四条任一命中）
1. **SLO 连续 3 个月低于红线**：可用性 < 99.9%, RTO > 60s ×3。
2. **代码耦合**：`Modularity CNM Q` 连续 2 个季度下降 ≥ 0.1（耦合恶化）。
3. **底层技术栈 EOL 或安全漏洞严重**：例如 Rust Axum 关键版本 EOL。
4. **用户增长导致存储容量增长超过当前架构设计上限 30%**：当前架构设计 100W 节点；若实际单租户 > 130W 节点 → 升级分片数 64→256。

---

# 三、产品对外介绍话术体系（四套不同受众）

## 3.1 简短一句话 Slogan（对外 logo 旁边 / 官网标题栏）

> **璇玑 RelGraph — 你的需求、文档、代码，全部在一张图里，AI 查询跟本地数组一样顺手。**

短 Slogan 版（名片/社群签名）：`Rust · 图谱中枢 · 归一化关联 · AI=本地体验`。

## 3.2 面向决策者（CTO/研发总监/采购）—— 商业介绍

> 璇玑 RelGraph 是一款**企业级知识图谱归一化平台**，把散落在需求文档、架构图、业务流程、模块注册、操作手册、本地代码 6 个地方的业务知识，**固化到一张 Rust 自研高性能图谱中枢**里；通过 `统一 AI 入口 + 本地 shape 超集等价协议`，让业务方"问 AI"跟查本地数据库一样顺手，告别"AI/本地两套结果打架"的尴尬历史。
>
> 我们为什么选它：
> - **省钱**：100W 节点年度 TCO ≈ 2 名 DBA 年薪的 85%（单节点 $0.153/年），硬件摊销+对象存储+人天一体化建模，避免重复采购多套中台组件。
> - **靠谱**：RPO=0 零数据丢失（200×随机故障重建 CRC 一致，已验证），RTO<60s，可用性 99.95%（年停机 ≤ 4.38 小时），底座全自研无外部强绑定开源组件被锁风险。
> - **管好变化**：每一次需求、代码、文档改动都会自动 CDC 同步到图谱，发版前后 Trace 图谱三跳 BFS 可审计，团队可以回答"改了一行代码影响多少需求"。
> - **合规**：企业增强版 SSO + RBAC + ClickHouse 审计链路 + OIDC 对接企业 IdP，金融/制造/政务开箱可合规上线。
> - **AI 不是玄学**：双专家辩论 + RRF 融合重排 + CEM 统一寻优的可解释链路，决策全部在 6 层图谱上有锚点。
> - **平滑迁移**：金丝雀 1→10→50→100 四阶段，< 30s 一键回滚，4 期可控落地不用"大爆炸"切换。
>
> 典型 ROI：
> 对于 500 人研发组织，"需求↔代码不一致返工" 一年损失约 1800 人天，璇玑上线 6 个月返工量下降 ≥ 40%，当年即可收回投资。

## 3.3 面向算法 / 研发工程师——技术介绍

> 璇玑 RelGraph 技术栈总览：
> - **接入层**：Rust Axum 自研 Gateway，四端点 `POST /ai/engine/{process,analyze,capabilities,metrics}`，带限流/RBAC/灰度权重；Node Sidecar HTTP 通信+本地降级（sidecar 挂也能本地直查）。
> - **编排层**：Node/Rust 同构服务，两级意图兜底（关键词分类器 → 激活扩散个性化 PageRank d=0.85 / 30 轮收敛），AC-10 路由表匹配语义：**静态路由优先 > 参数段少者优先 > 同参数段长路径优先**；多路召回 + RRF(k=60) 零训练参数融合；双专家辩论 debate-synthesis 轮合成共识；CEM 交叉熵方法统一优化（σ̄<0.06 或 3 轮无改进停止）。
> - **存储层（核心）**：
>   - 图谱域：Nebula 集群 3×metad + 3×graphd + 9×storaged（R=3, W=2），分片键 `hash(project_domain) mod 64`，邻接收敛率 ≥ 85%。
>   - 对象域：MinIO EC:4+2 纠删，SHA-256 chunk 去重 + 版本 vN.json 回滚 copy-on-write，动态分块 5–16MB MPU 大文件并发上传，软删 + 30 天 GC。
>   - 元数据域：Postgres + Citus 分片（entity_type / tenant 哈希）+ Redis Cluster L1 邻接缓存（TTL 5min，CDC 失效）+ pgvector HNSW 语义缓存（余弦相似度 ≤ ε=0.85 命中）。
>   - 开发/单节点模式：MemoryProvider + FS ChunkBackend（0 依赖部署，个人电脑 10s 跑 129 GREEN）。
> - **算法库（锁死常量可审计）**：
>   - 社区检测 = CNM Clauset-Newman-Moore 模块度贪心凝聚（禁用 LPA）
>   - 介数中心性 = Brandes 2001 BFS 累积依赖
>   - 紧密中心性 = Harmonic 谐波（不可达节点 0 分）
>   - PageRank 必须含转置图处理（权威/枢纽对偶） + 悬垂节点按 personalization 分配
>   - 激活扩散 = PPR 特例（d=0.85, maxIter=30 锁死）
>   - 所有中心性/密度指标：**禁止 toFixed 截断**，并附带密度解读文案（≥0.8 高度稠密/≥0.3 中等/<0.3稀疏）
> - **运维底座**：K8s StatefulSet + Helm3 Chart，金丝雀 4 阶段权重；就绪探针 `warmup_complete ∧ pg_stat_statements_hit_rate ≥ 0.85` 才 Gateway 放流量；预热三步（PR Hot TopK / 语义缓存 Seeds / L1 邻接）；OTel 全链路 + 冷备 Raft 快照 1 次/小时 + WAL 增量 15 分钟/次归档。
>
> 核心差异化亮点：
> - **AI 查询 = 本地查询体验**：协议硬性约束 `data段 shape 超集等价`（local所有字段在 AI 返回都有），AI 附加字段全部以 `ai_*` 前缀存在；前端 SDK 一行不改即可从纯本地升级。
> - **严格的先节点后边构建顺序**：流程图谱构建必须按 `node upsert → byId has → RAW 双向边展开`，缺节点明确返回 missing 数组，不静默丢边。
> - **CDC + L1 + DLQ 三保险**：图谱/文件变更后三路失效，失败进 DLQ + 5 分钟对账修复。
> - **RPO=0 FI 证据**：200×EC kill-2 随机场景下重建 CRC 100% 一致（企业级 0 数据丢失证明）。
> - **可插拔设计**：StorageProvider / IChunkBackend / RouterTable / registered_plugins.json 四处插件点，二次开发零侵入。

## 3.4 面向开源社区 README 简介

```markdown
# 璇玑 RelGraph · 开源版

Rust 自研高性能知识图谱作为中枢，把需求、架构、业务流程、模块、文档、代码
六类实体做**归一化关联**的开源平台 —— AI 查询体验 = 本地数组查询体验。

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3-7C5CFF.svg)](./LICENSE)
[![A+ Review](https://img.shields.io/badge/Enterprise%20Review-A%2B-19B26E.svg)](./.trae/specs/20260823-mox-storage-distributed-ai-unified-query/review.md)
[![129 Tests GREEN](https://img.shields.io/badge/Tests-129%20GREEN-7C5CFF)](./platform/backend-node/test)

## 特色 🌟

- ⚡ **Rust 自研网关** — 统一四端点 `/ai/engine/process`，AI/本地一套协议
- 🧠 **归一化图谱** — 需求/架构/流程/模块/文档/代码 6 层统一建模
- 🔍 **AI = 本地体验** — `data段 shape 超集等价`，AI 字段 ai_ 前缀隔离
- 🛡️ **算法归一可审计** — CNM/Brandes/Harmonic/PR×2/激活扩散 常量全锁死
- 🧯 **企业级 HA 基线内置** — EC:4+2 kill 2 重建 CRC 一致；RPO=0, RTO<60s
- 🔌 **四处可插拔** — StorageProvider / IChunkBackend / Router / Plugins

## 快速开始 🚀

```bash
# 0 依赖跑起来（Memory Provider，个人电脑 < 10s）
cd platform/backend-node
node test/test-three-flows-trace-e2e.js  # 三流程 + Trace 闭环
node test/test-enterprise-ha-fault-injection.js  # 企业故障注入 HA
```

输出应全部 GREEN，包含 graph_bulk/file_upload/ai_rag 三流程。

然后启动本地单机版本：

```bash
cd platform/backend-node && node src/api-server.js
curl -X POST http://127.0.0.1:8080/internal/intent \
     -H "content-type: application/json" \
     -d '{"query":"找 P-087 的 RBAC 文档","context":{"project":"P-087"}}'
```

## 生产部署建议（企业增强版路径）

生产部署走 4 期可控灰度：P1 Postgres+Citus → P2 Nebula 集群 → P3 Rust Gateway + Sidecar 统一入口 → P4 K8s 运维底座。每一步都可回滚。详细参考：
- 灰度脚本参考：`test/test-sdk-gray-warmup-summary.js rolloutPlan/readinessProbe/warmupRun`
- 企业级 200×FI：`test/test-enterprise-ha-fault-injection.js`

## 生态模块

- `platform/gateway/runtime/` — Rust Gateway（Axum）生产级接入
- `platform/backend-node/src/graph/graph-formulas.js` — 算法归一库（**做厚的核心**）
- `storage/chunk-backend.js` — IChunkBackend FS/S3 两种实现
- `routes/internal.js` — Sidecar 内部端点（intent + graph-algo）

## 贡献指南 🤝

1. 新算法插件必须过"算法精度护栏"：与 GraphFormulas 基准值误差 <1e-4。
2. 新功能必须绑定到需求/架构/流程/模块/文档/代码 中至少 2 层图谱节点。
3. 新接口必须保持 shape 超集等价——老字段不得删除、不得重命名。
4. 提交 CI 必须跑过 12 套件 Node + Rust cargo test（全部通过才能合入）。

## 许可证

AGPL-3.0（开源版）。企业增强版（分片集群、审计、行业知识种子、SSO、SLA 支持）请通过 `README 商务邮箱` 咨询。
```

---

# 四、用户最优使用流程（三个视角 × 全链路完整说明）

## 4.1 视角 A：企业业务接入流程（7 步，最佳实践 + 避坑）

### 前置条件
- 最小硬件：3 台 worker node（8C 32GB）+ 1 台 Postgres（或使用企业增强版 Postgres+Citus）。
- 账号：企业 IdP 中创建璇玑 OIDC 应用 Client ID/Secret。

### 7 步标准流程

#### 第 1 步：初始化部署（< 1 小时）
```bash
# 推荐：Helm3 部署企业增强版
helm repo add mox https://charts.mox.local
helm install mox mox/mox -n mox --create-namespace \
  --set provider.mode=enterprise \
  --set storage.postgres.host=pg.corp.local \
  --set idp.oidc.issuer=https://idp.corp.local \
  --set canary.initialWeight=0
```
- **配置最佳实践**：先 `canary.initialWeight=0`（默认 0 流量，不影响业务）。
- **避坑点**：不要一开始就开启 Nebula（`USE_NEBULAGRAPH=false`）。先走 SQLite/Postgres 单实例跑通 P1 元数据层，再切 P2。生产切勿使用 MemoryProvider（开发模式）。

#### 第 2 步：导入业务需求与文档（1-2 天）
1. 整理业务需求：每个需求唯一 id `R-YYYYMMDD-XXX`。
2. 操作 `POST /files/upload` 批量上传 Confluence 导出 Word/PDF/Markdown：
   - 开启 `linkedGraphIds` 参数，每个文件对应一批需求节点（`["R-20260101-001", "R-20260101-002"]`）。
   - ChunkBackend 默认 MinIO；文件 ≥ 100MB 自动走 5-16MB MPU。
3. **最佳实践**：文档**必须分版本上传**，使用 `uploadNewVersion` 接口，保留每一版的 changelog。
4. **避坑点**：文件名不要用中文全角字符，避免 bucket 跨地域乱码。不要把 1 份 10GB 的需求放一个文件——拆成 20 份 500MB 以下的章节化文档，每个章节绑不同需求节点。

#### 第 3 步：构建业务知识图谱（1-3 天，关键步骤，必须按顺序）
```bash
# 严格按 先节点→后边 顺序！！！
curl -X POST http://mox.corp.local/ai/engine/process \
  -H "Authorization: Bearer $TOKEN" \
  -H "content-type: application/json" \
  -d '{
    "intent": "graph_bulk",
    "data": {
      "nodes": [
        {"id": "P-087", "kind": "Project", "name": "授权中台", "layer": 1},
        {"id": "R-20260801-001", "kind": "Requirement", "name": "RBAC 授权模型", "layer": 1, "linkedDocIds":["DOC-RBAC-v1"]},
        {"id": "C-MOD-auth", "kind": "Module", "name": "授权模块", "layer": 4, "owner":"team-a"}
      ],
      "edges": [
        {"from": "R-20260801-001", "to": "P-087", "rel": "belongs_to"},
        {"from": "R-20260801-001", "to": "C-MOD-auth", "rel": "implemented_by"}
      ],
      "workflow_id": "INIT-P087"
    }
  }'
```
- **最佳实践**：使用 `project_domain 作为业务域前缀`，分片键会按 project_domain 哈希，2-3 跳查询邻居跨分片率低。`workflow_id` 必须写（不然 Trace 图谱无法闭环审计）。
- **避坑点**：千万不要边比节点先写或一批传（会导致 missing 列表被返回并丢弃边）。跨项目的全局节点（例如"公司"）要放进 Layer=0，避免每个项目重复 id。

#### 第 4 步：绑定业务模块与代码（代码图谱）—— 一次性工作，长期受益
1. 克隆 `code_graph_bindings.json` 已有的 76 条绑定，按你的模块格式编写。
2. 调用 `POST /ai/engine/process {intent:"code_bulk_sync", data:{repo:"./", include:"**/*.ts", exclude:"node_modules"}}`。
3. 企业增强版支持定时 CI 同步：每次 Git Push 自动同步 CodeEntity 节点绑定到 Requirement 节点。
4. **最佳实践**：CodeEntity 粒度到函数即可，不要细到每一行——图谱规模超过 1000W 节点要开 P2 分布式分片。
5. **避坑点**：不要把第三方库也扫描进去（node_modules/venv）—— 图谱噪声会导致 PageRank 权威度失真。

#### 第 5 步：业务运行（日常使用 AI 统一入口）
```js
import { createMoxClient } from '@mox/sdk';
const xj = createMoxClient({ base: 'https://mox.corp.local' });
// 业务方只调一个接口，不问"走 AI 还是本地"
const docs = await xj.ask("P-087 项目关联的 RBAC 相关需求文档（最近 2 版）", { project: 'P-087' });
console.log(docs.data);        // 与本地 /files/list 完全同 shape，前端零改
console.log(docs.ai_summary);  // AI 附加摘要
```
- **最佳实践**：设置 `max_latency_ms=500`（超时自动降级走本地直查，不阻塞业务）。
- **避坑点**：不要在 SDK 外层再做"AI 失败则回本地"的逻辑——璇玑 SDK 已经内置了。**不要**把 `ai_summary` 字段当作核心字段用于业务判断（AI 摘要可能为空）。

#### 第 6 步：测试校验 & Trace 闭环（强制里程碑）
每次上线前必须执行：
1. `POST /ai/engine/process {intent:"trace_check", workflow_id:"当前发版ID"}` → 返回所有 W-step-target 三跳 BFS 可达率 100% 才可放行。
2. 运行 `test-three-flows-trace-e2e.js` 的企业版对应（切换到 Postgres Provider 运行）。
3. 企业增强版仪表盘：发布后检查 `发布仪表盘`：RAG 语义缓存命中率 > 60% 才算正常，否则预热不充分，再跑一次 warmup。
- **避坑点**：不要跳过 Trace 检查。过去的 3 个失败线上案例都因为"跳过 BFS 校验导致 W→S→T 断链"。

#### 第 7 步：迭代优化（每次迭代都要图谱反向同步）
见 §2.1 六维优化闭环。每次迭代：
1. 把新需求写入 Requirement 节点。
2. 把改动的代码实体 CodeEntity 更新 & 关联到对应需求。
3. 把文档 Doc 节点新增版本 `linkedGraphIds` 同步更新。
4. 运行 CEM cemOptimize 做 4 目标加权分评估，判断这次迭代的效果是否 ≥ 阈值。

### 【重要】什么场景**不要**使用璇玑 RelGraph
- ❌ **纯 CRM/ERP 交易系统**：璇玑不是 OLTP 业务库。
- ❌ **文档协同编辑器**：璇玑不改 Word/PDF 原文（只分块索引关联）。
- ❌ **10 人以下微型团队**：用 Notion + Linear 可能更简单，璇玑有最小维护门槛（适合 50+ 人以上研发中心）。
- ❌ **实时风控毫秒级写场景（< 10ms）**：璇玑不承诺毫秒级交易写入（P95 50ms 级本地直查可满足，但不是 OLTP 定位）。
- ❌ **只要一个"聊天机器人"**：采购单独的 LLM API 产品即可，不需要一整套知识图谱。

---

## 4.2 视角 B：算法联盟专家使用流程（6 步，从注册到辩论权重优化）

### 前置条件
- 一个你熟悉的领域（代码/架构/测试/安全/文档/行业合规…）。
- 本地能跑通 Memory Provider 12 GREEN。

### 6 步标准流程

#### 第 1 步：注册你的专家身份（一次性）
```jsonc
// experts.json 追加一项，或 POST /api/alliance/experts/register
{
  "id": "code-expert-node-v3",
  "name": "Node 后端代码专家",
  "domain": ["backend", "node.js", "rest-api", "sql"],
  "weightPrior": 0.75,
  "entry": "operators.code_expert.execute"
}
```
- **最佳实践**：`domain` 数组使用关键词，不要写长段描述——因为联盟意图匹配用的是关键词 token 匹配，多写细粒度关键词召回率更高。
- **避坑点**：`weightPrior` 不要一开始就设 1.0（绝对权威），建议先从 0.6–0.75 起步，让 CEM 寻优后慢慢提升到真实权重。

#### 第 2 步：验证专家能力（跑通）
跑 `test/test-three-flows-trace-e2e.js`，确认 aiRagFlow 返回中 `experts[1].name === '你的专家名'` 出现在 debate 名单里。
- **最佳实践**：把你专家的 prompt 固化到 `alliance_expert_capability_graph.json` 的 capability 节点，绑定到 Intent 节点。
- **避坑点**：不要让两个专家做完全一样的能力 —— 辩论需要"有差异才能出共识"。

#### 第 3 步：固化典型 20 Case 的黄金数据集
- 每个专家至少准备 20 条典型 Input → Expected Output 黄金样本（可 JSON Lines 存 `fixtures/experts/{id}.jsonl`）。
- 每次你专家代码改动后，跑 CEM cemOptimize：
  ```
  evaluator = 你的黄金样本通过率 (Q)
             + 输出 token 长度合理性 (S)
             + 单样本平均耗时秒数倒数 (T)
             + 输出和另一个专家的差异率（辩论多样性 Stability）
  ```
  停止条件：σ̄<0.06 或 3 轮无改进。
- **最佳实践**：先把 20 Case 结果存为 Doc 节点，`linkedGraphIds = [你的专家 id]`。
- **避坑点**：不要准备全是简单问答的 case，要有 30% 的难例、边界 case、反例。

#### 第 4 步：将你的专家接入双专家辩论
AI 编排层的辩论合成规则是：
```
Graph Expert × 业务域专家（你的专家）→ 双专家各自回答 → debate-synthesis 取共识。
```
修改 `ai-engine-core.js` 的 `DEFAULT_DEBATE_PAIRS = [['graph_expert', '你的专家']]`。
- **最佳实践**：每 2 周检查一次 `debate-synthesis` 的分歧日志（分歧率 > 30% 说明你的专家能力要扩容）。
- **避坑点**：不要在双专家辩论里放三个以上专家——共识复杂度暴涨，RRF 融合效果反而下降。

#### 第 5 步：参与联盟权重优化
璇玑的联盟权重是 CEM 周期性寻优（企业增强版默认每周 1 次）。你可以：
1. 提交一个 `alliance_learned_skills.json` 新技能（绑定到你的专家）。
2. 标注你的技能对哪类 `intent` 适用（intent 图谱节点 linkedGraphIds 关联）。
3. 每 4 周查看 CEM 输出的最佳权重参数，如果 `你的专家权重` 下降 > 0.1，说明近期样本对该专家不友好——需要更新黄金 Case 或优化能力。

#### 第 6 步：把优化反向同步到图谱
- 更新 `Module-专家联盟` 节点的 `version`。
- 更新 Doc-专家手册 节点的 `manifest`。
- 把 `registered_skills.json` 新增项绑定到 `Doc-你的专家`。
- 运行 Trace 图谱验证：`W-联盟权重迭代-XXX → S-加载黄金样本/S-CEM 训练/S-上线 → T-你的专家ID/T-新权重 T-20Case` 全可达。

### 【禁用场景】
- ❌ **通用泛问答机器人专家**：没有专业领域的专家会稀释联盟权重，属于伪专家（反伪需求筛查）。
- ❌ **只做 Prompt Engineering，不绑定到领域 Capability 图谱节点**。

---

## 4.3 视角 C：普通开源开发者二次开发流程（8 步，从 fork → 贡献合入）

### 前置条件
- Node v20+ / Rust stable（2024-08 以后即可）/ Git 基本操作。
- 不需要 Postgres / Nebula / MinIO。MemoryProvider 模式能跑所有测试。

### 8 步标准开发流程

#### 第 1 步：Fork & 运行最小验证
```bash
git clone https://github.com/your/mox-relgraph
cd mox-relgraph/platform/backend-node
# 不要装 pg / minio / nebula 依赖（MemoryProvider 默认）
node test/test-three-flows-trace-e2e.js  # < 10s 全过 → 开发环境 OK
```
- **最佳实践**：永远先跑最小验证（T10 三流程闭环），不要上来就 `npm install` 400 个包。Memory 后端是"可本地跑通一切"的铁律。
- **避坑点**：不要改 `STORAGE_PROVIDER` 环境变量。`npm install` 时遇到的可选依赖 `pg` 未安装是正常的。

#### 第 2 步：创建一个 Issue（绑定图谱）
打开 GitHub Issue → 选择"Bug / Feature Request / Good First Issue"模板。把 Issue 号复制下来，创建一个本地改动的 **Workflow ID**：`GH-<Issue号>-<你的简称>`，用于 Trace 图谱闭环。

#### 第 3 步：先写测试（TDD 铁律 · 璇玑 RED→GREEN 强制）
**No RED → No code**。

例如要新增 `ChunkBackend`（Azure Blob）：
1. 先在 `test/test-chunkbackend-azure.js` 写 RED 测试：
   - `put 'hello blob' → hash 断言`
   - `get 一致性`
   - `delete 后 has=false`
   - `multipart 大文件 10MB`
2. 运行测试 → 应当 **FAIL**（因为你还没写实现）。如果 PASS → 说明测试没测对。

#### 第 4 步：最小化实现代码
实现 `AzureChunkBackend extends IChunkBackend`（实现 5 方法 `put/get/delete/has/abortMultipart`）。
- **避坑点**：不要给 IChunkBackend 加新的虚方法（会破坏所有其他实现）。不要改动 FS/S3 两个已实现的任何方法签名。
- **最佳实践**：遵循"核心能力做厚，周边能力做轻"—— 你的 Azure 实现应可单独打包成 `@mox/chunk-backend-azure` 插件，不要侵入核心仓库。

#### 第 5 步：GREEN 之后—— 129 全回归必须全过
```bash
# 12 Node 套件（必须全绿，否则你的改动引入了回归）
for f in test-*.js; do echo "=== $f ==="; node "test/$f"; [ $? -ne 0 ] && exit 1; done
# Rust 3 测试（如果改了 Gateway/路由）
cd ../../gateway/runtime && cargo test --test router_semantics --test sidecar_degrade --test ai_engine_e2e
```
- **避坑点**：绝对不要"只跑我写的那个新测试"。璇玑要求零回归。
- **最佳实践**：新增的算法插件必须过 `TR-4.3` 精度护栏（没有 toFixed 截断）。

#### 第 6 步：图谱反向同步（贡献者强制步骤 · 绑定到 6 层中至少 2 层）
在你本地 fork 的个人图谱构建 graph_bulk：
```
nodes = [
  Issue-GH<号>: Requirement
  ChunkBackend-Azure: Module
  Doc-README-Azure-Contrib: Doc
]
edges = [
  Issue → ChunkBackend-Azure (implemented_by)
  ChunkBackend-Azure → Doc (documented_by)
]
workflow_id = GH-...
```
- **避坑点**：不要只绑定到 Module 一个层。璇玑的理念是"所有改动都要关联到需求+文档"。

#### 第 7 步：写 Release Notes（Doc 节点版本 + PR）
PR 提交包含三个文件：
1. `src/storage/chunk-backend-azure.js`（插件实现）
2. `test/test-chunkbackend-azure.js`（RED→GREEN 测试）
3. `docs/plugins/chunk-backend-azure.md`（使用说明，绑定 Doc-README-Azure-Contrib）
- **最佳实践**：PR 描述里写清楚 `workflow_id` + "129 GREEN + 新增 X GREEN" 截图 —— maintainer 一眼能判。

#### 第 8 步：合入后：精度护栏 CI + Release
合入 master 时 CI 会自动：（1）跑 129 GREEN；（2）跑精度护栏护栏 diff 对比网络基准 <1e-4；（3）检查"改动点是否破坏 shape 超集等价"（老字段必须存在）。合入后 Release 打 tag。

### 【禁用场景 / 不要贡献的 PR】
- ❌ **只是改代码风格、空格、变量名**（如果没绑定到需求层节点 = 伪贡献）。
- ❌ **修改 GraphFormulas 常量（d=0.85 / CNM / Brandes...）**：这是项目记忆硬性红线，除非走架构级重构 Spec。
- ❌ **没有 RED 测试直接写实现的大 PR**：TDD 铁律。
- ❌ **破坏 shape 等价字段**（例如把 `createdAt` 改成 `created_at` 删除别名）。

---

# 附录 · 代码锚点路径索引

| 文档提到能力 | 绝对路径 |
|---|---|
| StorageProvider 抽象 22 法 | [storage/index.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/index.js) |
| IChunkBackend 5 法接口 | [storage/chunk-backend.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js) |
| 算法归一库 GraphFormulas（10 能力） | [graph/graph-formulas.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/graph/graph-formulas.js) |
| 统一语义网关 Rust 四端点 | [handlers/ai_engine.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/runtime/src/handlers/ai_engine.rs) |
| AI 路由语义表 AC-10 | [ai_router.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/runtime/src/ai_router.rs) |
| Sidecar 通信 & 降级 | [sidecar/node_sidecar.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/runtime/src/sidecar/node_sidecar.rs) |
| Node internal endpoints（intent/graph-algo） | [routes/internal.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/routes/internal.js) |
| graph/search 激活扩散重排 | [routes/graph.js:270-356](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/routes/graph.js#L270-L356) |
| Trace 图谱 / 三流程端点参考实现 | [test-three-flows-trace-e2e.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/test/test-three-flows-trace-e2e.js) |
| 灰度脚本 + 就绪探针 + 预热参考实现 | [test-sdk-gray-warmup-summary.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/test/test-sdk-gray-warmup-summary.js) |
| SLO/容量/TCO 模型参考实现 | [test-enterprise-slo-capacity-tco.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/test/test-enterprise-slo-capacity-tco.js) |
| FI 故障注入 HA 演练 & 200 CRC RPO=0 | [test-enterprise-ha-fault-injection.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/test/test-enterprise-ha-fault-injection.js) |
| A+ 独立评审报告 | [review.md](file:///d:/a10/aikjx/gitcode/infotopograph/.trae/specs/20260823-mox-storage-distributed-ai-unified-query/review.md) |

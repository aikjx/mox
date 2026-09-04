# 璇玑 分布式存储 + AI 统一查询 产品需求规格（PRD）

## Overview
- **Summary**：将璇玑系统当前"单机 SQLite + 本地文件"的知识图谱与云盘存储升级为"分片分布式存储 + 对象存储 + 统一语义网关"的企业级底座，并使 AI 查询入口与本地查询入口归一为同一会话化协议，业务方零感知。同时落地核心算法、端到端业务流程、可灰度发布底座、mox 模块化系统架构测试与独立验收。
- **Purpose**：突破当前单机 I/O、扩展性、可靠性与"AI/本地双入口割裂"的瓶颈，满足"一切皆是项目 · 8 项目 × 24 域 × 17 引擎"目标规模（100 万节点 / 1000 万边 / PB 级文件 / 亚秒级 AI 混合查询）的稳定承载，并使 AI 查询对业务与用户"与本地查询一样方便"。
- **Target Users**：平台调用方（frontend-ui、外部系统）、开发专家联盟与算法联盟开发者、平台运维 SRE、终端业务用户（通过 Chat / GraphView / DocsView / KnowledgeBaseView 等界面使用）。

## Goals
- G-1：在 `platform/backend-node/src/storage/` 抽象内新增真正可用的 Postgres（Citus 分片友好）Provider，使实体/KV/日志/文件元数据能一键切换到分布式关系底座，并支持开发环境下 SQLite 热切换（零业务代码改动）。
- G-2：将 `FileStore` 的本地磁盘 chunk IO 抽象为 `IChunkBackend` 接口，至少落地 `fs`（现状）与 `s3-compatible`（MinIO/OSS/S3）两种实现；保留 SHA-256 内容寻址去重与版本化模型；配置化切换。
- G-3：将 `NebulaGraphAdapter` 从"本地单 JSON 整图 + 可选异步双写"改造为"远程分布式图优先（Gremlin/nGQL）+ 本地 L1 邻接缓存 + CDC 失效"的读端，并确保图写端按"节点先建、边后加"顺序执行，防止引用丢失。
- G-4：在 Rust Gateway 新增统一 AI 引擎路由模块，落地项目记忆硬性要求的 4 个端点：`POST /ai/engine/process`、`POST /ai/engine/analyze`、`GET /ai/engine/capabilities`、`GET /ai/engine/metrics`；并提供 Node 侧 sidecar 共享同一套意图识别与图公式库。
- G-5：统一 AI 查询协议 = 本地查询返回体超集：`data` 字段与现有 `/graph/*`、`/files` 完全同 shape，AI 附加值字段仅以 `ai_*` 前缀出现；调用方可一行不改平滑升级；发布三语言同构 SDK（Node/Python/Rust）。
- G-6：业务流程全部在统一流程图谱中承载，提供可运行的业务处理流程定义（graph_bulk 写、文件上传 + 自动关联图谱、AI RAG 查），并以端到端接口冒烟 + 流程图校验验证闭环。
- G-7：核心算法严格遵守项目记忆硬性约束：激活扩散=个性化 PageRank 特例（method=spread, d=0.85, 30 轮收敛）；社区检测=CNM 模块度贪心凝聚；介数中心性=Brandes；紧密中心性=harmonic；RAW 边输入在库内双向展开；公式库保留全精度（禁用 toFixed 截断）；密度指标附带人读解读文案。
- G-8：发布底座支持"优雅切换：新服务启动成功 → 流量切换 → 下线旧服务"，在 Gateway 侧提供 canary 权重 + 双写双读回源 + 就绪预热探针 + 版本化 helm/kustomize 切换脚本（至少可运行的最小可验证形态，无需真实 K8s 集群强依赖）。
- G-9：mox 模块化系统架构自动测试覆盖：单元（算法/存储/路由）+ 集成（接口）+ 流程（业务流程图断言）+ 回归（AI 混合查询协议）；生成交付级测试分析修复优化报告。

## Non-Goals
- NG-1：不在本次 Spec 范围中生产部署真实物理集群（Nebula/MinIO/Postgres+Citus 的真实多节点运维），只要求代码级适配、配置化切换、容器化 manifest、本地可运行 compose/standalone 形态验证通过。
- NG-2：不重写现有 Rust AI Agent 的算子系统、不新增 Raft/一致性协议的自研实现，优先使用成熟组件（Nebula 的 Raft、Postgres 的两阶段提交）。
- NG-3：不改变当前前端路由结构、不重写 frontend-ui 的页面组件；仅新增 SDK 接入与可选的 AI 增强展示字段渲染。
- NG-4：不引入外部 SaaS（如云端图数据库 SaaS / 商业对象存储）作为硬依赖，本地 MinIO / Postgres / 嵌入式 Nebula（或 mock）即可完成全部验收。
- NG-5：不实现"真实跨地域多活"的底层网络复制方案，仅在接口与配置上预留多地域桶与复制策略，可切换。

## Background & Context
- 现状代码（截至 2026-08-23）已审计：
  - 知识图谱读写：`platform/backend-node/src/modules/graph.js` 使用 `graph_nodes/graph_edges` 两张 entity_type 列表，每次 `saveList` 做"全删 + 全量重写"，不可扩展。
  - 企业图谱适配：`platform/backend-node/src/nebulagraph-adapter.js` 以单大 JSON `knowledge_graph:main` 持久化；仅当 `USE_NEBULAGRAPH=true` 时才向 Gremlin 端做异步双写，读端完全读本地。
  - 存储抽象：`platform/backend-node/src/storage/index.js` 提供 `StorageProvider` 基类 + `SQLiteProvider` + `MemoryProvider`；`StorageFactory` 里 `mysql/postgresql` 仍为 throw 占位。
  - 云盘分块与版本：`platform/backend-node/src/file-store.js` 本地 fs + SHA-256 chunk 去重 + `vN.json` 版本；与图谱通过 `linkedGraphIds` 双向绑定。
  - 图算法单源：`platform/backend-node/src/ai-flow-graph.js` 已实装度中心性、Brandes 介数、harmonic 紧密中心性、PageRank、密度公式（含人读解读）；`graph-algos.js` 仍为旧实现含 LPA 社区检测，需统一收敛到 ai-flow-graph 并升级为 CNM。
  - 意图识别：`platform/backend-node/src/expert-alliance/domain/intent-classifier.js` 关键词加权命中；激活扩散在算法层未被接入 `/graph/search` 做路由重排。
  - AI 路由：`routes/ai-engine.js` 有算子/工作流/MCP 等端点，但项目记忆硬性要求的 4 个统一端点 `/ai/engine/{process,analyze,capabilities,metrics}` 未在 Rust Gateway 实装。
- 项目记忆硬性约束（来自项目域历史决策，本 Spec 必须满足，不做谈判）：
  - melody2score 打包窗口化 stderr/stdout 兜底（与本次存储/AI 无直接关系，但仓库测试矩阵必须仍绿）。
  - AI engine 优化使用 CEM（交叉熵方法）；评估多目标加权 `0.55Q + 0.20S + 0.10T + 0.15Stability`；停止条件 `σ̄<0.06` 或连续 3 轮无改进。
  - 统一编排核心 4 个端点必须提供：`POST /ai/engine/process`、`POST /ai/engine/analyze`、`GET /ai/engine/capabilities`、`GET /ai/engine/metrics`。
  - 业务流程与算法流程统一承载于图谱引擎。
  - 激活扩散意图识别 = 个性化 PageRank 特例（method=spread, d=0.85, 30 轮收敛）。
  - 社区检测必须用 CNM，禁止 LPA。
  - 介数中心性 Brandes；紧密中心性 harmonic。
  - 无向图边 = RAW 输入，库内双向展开，避免度中心性误算。
  - 公式库保留全精度，禁用 toFixed。
  - 所有中心性输出附带人读公式；密度指标附带解读文案（高度稠密/中等密度/稀疏）。
  - 流程图谱构建按"节点 → 边"顺序。
  - PageRank 必须含转置图处理。
  - 路由匹配必须遵循"静态路由优先于参数化路由；参数段少者优先，同参数数时保留长路径优先"的企业级语义。
- 设计美学：所有可视化输出遵循项目用户偏好：极简高级、1:1.618 黄金比布局、低饱和深空色、柔散光影、渐变圆角。本 Spec 新增页面/流程图遵循。

## Functional Requirements

### FR-1：可切换 Postgres 分布式 Provider（Storage 抽象落地）
- StorageFactory 中 `postgres` 不再抛错，实现真实 `pg` 驱动连接，支持与 SQLite 同 API（insert/upsert/update/delete/get/list/count/search/kv*/log*/migrateFromJSON）。
- 提供 `storage/switchDatabase('postgres')` 运行时切换（与现有 SQLite/Memory 等价），并保留自动迁移 JSON 数据的幂等护栏。
- 写双写开关 `DB_DUAL_WRITE=true`：在切换过渡期同时写 SQLite 与 Postgres；读优先 Postgres，空读回源 SQLite 并回填。

### FR-2：云盘 `IChunkBackend` 双实现（fs + s3-compatible）
- 抽象 `IChunkBackend`（readChunk / writeChunk / hasChunk / deleteChunk / listManifests），在 FileStore 构造时注入；默认 fs，`FILE_BACKEND=s3` 时用 AWS S3 兼容客户端（MinIO/OSS/S3）。
- 大文件 ≥ 100MB 启用动态分块 + MPU（Multipart Upload）；并发上传分片数受 `FILE_MPU_CONCURRENCY` 环境变量控制（默认 4）。
- 删除为软删除 + 保留期 + GC：软删标记写 Postgres `files.status='soft_deleted'`；超过 `FILE_GRACE_DAYS`（默认 30）后台作业清理；chunk 引用计数到 0 即物理删。
- 上传后的文件若触发 KB pipeline，应按"节点先建 → 边后加"顺序写入知识图谱，并把 `file.linkedGraphIds` 双向绑定。

### FR-3：Nebula 适配器读端 + CDC 失效
- `USE_NEBULAGRAPH=true` 时，`getNode / listNodes / neighbors / multiHopTraversal / shortestPath / semanticSearch` 优先查询 Gremlin/nGQL（可配置）；失败回退本地 L1 缓存（LRU，TTL 300s）。
- 图谱写路径：createNode/createEdge/bulkUpsert 完成后 emit CDC 事件到内部事件总线（支持 Redis Stream 或内存事件总线两种实现，可配置）；CDC 消费端失效 L1 缓存并触发属性/向量索引更新。
- bulkUpsert 必须按"先节点、后边"原子阶段执行；若边指向不存在的节点，必须返回 `warnings: [{missingTargets: [...]}]` 并保证节点侧不丢失。

### FR-4：CNM 社区检测替换 LPA（算法归一）
- 在 `ai-flow-graph.js` 新增单源 `communityDetectionCNM(nodes, edges)` 实现：模块度 Q 贪心凝聚；严禁再调用 LPA 的任何对外接口。
- `/graph/*` 与 `NebulaGraphAdapter.getStats()` 的 `communities` 字段来源统一改为 CNM，且不改变响应字段 shape（向下兼容）。
- 测试：CNM 与图学术基准（Zachary Karate Club 期望社区数 ∈ {2,4}，依赖选择）的模块度分 ≥ 参考值，误差在全精度阈值内通过。

### FR-5：Rust Gateway `/ai/engine/*` 四端点 + Node sidecar
- Gateway `handlers/ai_engine.rs` 挂载 4 个端点，返回体协议与"现状分析 §3.2"一致：`{ok,route,data,ai_summary?,metrics?}`；数据 `data` 与旧 `/graph/*`、`/files` 同 shape。
- 路由决策：`intent-classifier`（关键词）→ 激活扩散 PR（d=0.85, 30 轮）→ 语义缓存命中探测（pgvector 余弦 ε=0.85，可配置）→ 能力路由（静态优先 → 参数段少优先 → 同参数数路径长优先）。
- 因意图识别/图算法当前在 Node 端实装，Gateway 侧通过本地 sidecar（http 到 127.0.0.1:3010）完成图计算；同时将 `intent-classifier.js` + 激活扩散导出为可选 WASM，允许单进程形态。
- `GET /ai/engine/capabilities` 返回能力矩阵的自描述（local-only / ai-augmented / ai-only + P95 latency）。`GET /ai/engine/metrics` 返回近 5 分钟窗口的成功率、降级率、本地命中/AI 命中/混合执行 SLA 计数与分位延迟。

### FR-6：三语言同构 SDK（Node / Python / Rust）
- `sdk/mox-sdk-node/`、`sdk/mox-sdk-py/`、`sdk/mox-sdk-rs/`：每个 SDK 提供 `graph.list() / file.* / ask(自然语言)` 三组 API；`ask` 返回与 `POST /ai/engine/process` 同结构。
- SDK 内建 `max_latency_ms` 熔断（默认 500ms，超时走本地查询分支）、本地 LRU 1K/60s、429/503 指数退避（最大 2 次重试）；流式走 SSE。

### FR-7：业务流程图谱承载 + 流程接口
- 三条核心流程以图谱 step 节点落地（step/capability/keyword/engine 四类节点与边），并在 Node 端提供 `POST /ai/engine/flow/{graph_bulk_write, file_upload_auto_link, ai_rag_query}` 三端点执行流程：
  - 写图谱：节点预验 → 节点批量 upsert → CDC → 等待提交点 → 边批量 upsert（RAW 双向展开）→ 增量公式重算 → 返回结果/警告。
  - 文件上传：对象存储 MPU → 版本 manifest → KB 文档 pipeline（解析/实体抽取/差异）→ 三元组写图谱 → linkedGraphIds 绑定。
  - AI RAG：缓存命中 → 意图拆分 → 多路检索（graph+file+code+logs）→ RRF 融合 → 专家联盟辩论合成 → 回填缓存。
- 每条流程的执行记录以 trace 节点写回图谱，形成闭环可追溯。

### FR-8：灰度发布与优雅切换底座
- Gateway 新增 `GET/POST /system/traffic`：可配置 `canary.weight`（默认 0.01 起步，1.0=全切）、`dualWrite=true`、`readPref=postgres|sqlite|auto`、`warmup.targets=pg_stat_statements`。
- 就绪探针：`/ready` 在预热完成（本地查询基准数据集命中率 >= 0.85 或预热次数已达上限）后才通过；启动未就绪时 Gateway 返回 503 让上游 LB 不摘流量。
- 提供最小可运行切换脚本：`/platform/scripts/graceful-switch.ps1`（PowerShell）与 `graceful-switch.sh` 两个平台等价脚本，完成"新健康探活 → 设置权重 1 → 确认健康 → 停旧进程"的最小闭环。

## Non-Functional Requirements

### NFR-1：性能与规模
- 本地图谱查询：1 跳邻居（度 ≤ 1K 节点）P95 ≤ 10ms；3 跳邻居（单机 10 万节点）P95 ≤ 120ms；目标规模 100 万节点/1000 万边 P95 3 跳 ≤ 500ms。
- AI 混合查询（hybrid）：语义缓存命中时 P95 ≤ 30ms；冷路径 P95 ≤ 1000ms，其中 `max_latency_ms=500` 阈值可强制降级本地。
- 对象存储上传：≥100MB 文件（MPU 并发 4）吞吐 ≥ 80MB/s；读取 P95 延迟 ≤ 首字节 150ms（本地 MinIO）。

### NFR-2：可靠性与一致性
- 图谱写入：Raft 仲裁（远程图）或 Postgres 事务（本地过渡），失败返回可诊断错误与跳过列表，不得产生引用悬空但无任何告警。
- 对象存储：后端≥MinIO EC 4+2（可配置）；即使 2 台 chunk server 掉线仍可读；软删除保留期配置错误不得物理删除未过期文件（必须通过测试）。
- 双写过渡期：写入失败主路径仍保证一致性；次路径失败入 DLQ（内存/Redis），每 5 分钟对账补偿。

### NFR-3：安全与合规
- 统一 RBAC 中间件沿用 Gateway 现有 `rbac_middleware.rs`；4 个 AI 端点必须过鉴权与审计，写操作写入审计日志。
- 文件元数据与图节点属性中的 PII 字段（email/phone/身份证等）以 `_pii_` 前缀命名，并在日志输出、CDC 事件、AI 上下文中自动脱敏（Mask: `***` 或按策略）。
- SDK 不会在本地明文缓存完整响应；LRU 只缓存 query→响应摘要或引用；可通过 `cache.disable=true` 全关。

### NFR-4：可观测性
- OTel 兼容埋点：请求 trace_id 贯穿 Gateway → sidecar → 存储；关键 span：router.intent_detect / router.cache_check / executor.local / executor.ai / cdc.emit。
- `GET /ai/engine/metrics` 输出 OpenMetrics 兼容格式（同时保留 JSON），可被 Prometheus 抓取。

### NFR-5：可维护性与兼容性
- API 返回字段为现状超集，不得删除/重命名现状字段；新增 `ai_*` 字段为可选；老客户端不感知。
- 所有新增配置项通过环境变量控制，默认值保证现状行为不变（即零配置启动=当前单机形态，**向后兼容**）。
- 代码模块边界清晰：`storage/` 只做存储；`file-store` 只做文件；`ai-engine-core` 只做统一编排；`handlers/ai_engine` 只做协议与鉴权；无环依赖。

## Constraints

### Technical
- CT-1：算法约束必须 100% 对齐本 Spec "Background & Context" 中项目记忆硬性约束列表（可由独立 Review 逐条检查）。
- CT-2：`backend-node` 仍采用 CommonJS `require/module.exports`（与仓库现状一致）；禁止引入 ESM-only 模块导致破坏性升级。
- CT-3：Rust Gateway（`platform/gateway/runtime/`）保持 `axum` + `tokio` 现有栈，不引入框架切换。
- CT-4：不得引入真实外部付费/闭源组件作为硬依赖；所有新组件至少有一个本地可运行（compose 或 standalone）的自由开源替代默认值。
- CT-5：公式库所有值在内部流程保留全精度；仅在最后展示层允许按配置做格式化（默认不格式化）。

### Business
- CB-1：开发专家联盟与算法联盟可并行协作，SDK 与算法层可分别独立发布、独立回滚；版本号遵循 SemVer。
- CB-2：璇玑"8 项目 × 24 域 × 17 引擎 = 一切皆是项目"的归属层级不得被打破；图分片哈希必须以 project_domain 为主键位。
- CB-3：新功能默认不改变现有业务流程行为，启用需要显式配置（Opt-in）。

### Dependencies
- CD-1：Postgres 15+（pgvector 可选扩展，0.7+）、Redis 7、MinIO RELEASE.2023+；全部为开源版本。
- CD-2：Node.js 18+（仓库现状使用 npm package.json 声明版本）；Rust MSRV 与 `gateway/runtime/Cargo.toml` 保持一致。
- CD-3：npm/pnpm 依赖新增需在 `backend-node/package.json` 中显式声明并通过现有 `npm ci`/`pnpm install` 可安装（国内镜像可访问）。

## Assumptions
- A-1：独立验收阶段，测试环境允许以 Docker（或等价 Windows 容器方案）启动 Postgres 单节点、Redis 单节点、MinIO 单节点即可完成绝大多数 rule 类验收；分布式多节点形态以配置正确 + 单元覆盖即可。
- A-2：NebulaGraph 在 CI 不可用时，允许用 `USE_NEBULAGRAPH=false` + 本地增强 L1 缓存形态完成读端验收，但配置与 CDC 代码路径不得省略。
- A-3：LLM 供应商在测试中可被 Mock（现有 llm-gateway 的 deterministic 分支），不阻塞统一协议与路由的企业级验收。

## Acceptance Criteria

### AC-1：PostgresProvider 真实可用且与 SQLite 行为一致
- **Type**: `rule`
- **Given**: 仓库本地安装 `pg` 驱动并指向可用 Postgres（单节点即可），环境变量 `DB_PROVIDER=postgres`
- **When**: 调用 `switchDatabase('postgres')` → 依次执行 `upsertEntity`/`getEntity`/`listEntities`/`kvSet`/`kvGet`/`migrateFromJSON`/`saveList/getList`/`searchEntities`/`addLog/getLogs`
- **Then**: 每一项操作返回与 SQLiteProvider 相同的结构化结果；JSON 字段与时间戳字段在等价语义下一致；`migrateFromJSON` 幂等（执行两次数量相同）
- **Pass Condition**: 与 SQLite 对照测试（同 seed 数据集）逐字段等价对比全部通过
- **Evidence**: `backend-node/test/test-storage-postgres.js` 运行全部断言通过；控制台 diff 为 0 行

### AC-2：DB_DUAL_WRITE 双写+回源验收
- **Type**: `rule`
- **Given**: 空 Postgres + 已有数据 SQLite；`DB_DUAL_WRITE=true`、`DB_READ_PREF=postgres`
- **When**: 写 100 条实体，再手动 DELETE Postgres 中 5 条后执行对应 `getEntity`
- **Then**: 写入后两边数量一致；被删 5 条通过"读 Postgres 空 → 回源 SQLite → 回填 Postgres"路径，最终读数仍为 100 且数据无损
- **Pass Condition**: 回填后双端计数=100，回源计数=5
- **Evidence**: 测试 `test-dual-write.js` 日志 + 计数断言通过

### AC-3：IChunkBackend s3 实现与 fs 行为完全等价（SHA-256 去重）
- **Type**: `rule`
- **Given**: 本地 MinIO（或内存 mock S3）可达；`FILE_BACKEND=s3`；提供 2 份字节完全相同的 2MB 文件 A 和 副本 A'，再提供 1 份 2MB 不同内容 B
- **When**: `uploadFile(A)` → `uploadFile(A')` → `uploadFile(B)`
- **Then**: chunk 桶中实际独立 chunk 数 = A 的 chunk 数 + B 的 chunk 数（A 与 A' 共享）；`getFileContent(id)` 读出 A、A'、B 的字节与原始完全一致
- **Pass Condition**: 后端对象数 = `ceil(2MB/1MB) + ceil(2MB/1MB)` 且 A/A' 内容读取 sha256 相同
- **Evidence**: `test-file-store-s3.js` 运行通过；控制台列出桶对象数量与哈希分布报告

### AC-4：大文件 MPU 流式上传 128MB 成功且 hash 校验
- **Type**: `rule`
- **Given**: `FILE_BACKEND=s3`；`FILE_MPU_CONCURRENCY=4`；生成 128MB 伪随机文件 seed=42
- **When**: `uploadFile(buffer, '128m.bin')` 成功返回，再 `getFileContent(id)` 读回
- **Then**: 上传过程无阻塞错误；读回字节 sha256 = 原始文件 sha256
- **Pass Condition**: 两端 hash 字符串相等；接口总耗时在本地 ≤ 20s（参考，不作为硬阈值失败，只作为 rubric）
- **Evidence**: 生成的哈希日志 + 测试断言通过

### AC-5：软删除 + 保留期 + GC 行为正确
- **Type**: `rule`
- **Given**: `FILE_GRACE_DAYS=0`（便于测试立即 GC）；上传文件 A
- **When**: 调 `deleteFile` 后，直接列出后端 chunk；在 GC 作业运行前后分别检查 `status` 和 `getFileContent` 可用性
- **Then**: 删除后 status=soft_deleted 且 chunk 仍存在；GC 完成后 chunk 数减到 0 且 getFileContent 抛 `not found`
- **Pass Condition**: 两次状态与 chunk 计数均满足
- **Evidence**: `test-file-store-gc.js` 断言通过

### AC-6：Nebula 适配器远程读端优先 + L1 缓存 + CDC 失效
- **Type**: `rule`
- **Given**: 远端 Gremlin 端点可通过 mock（或真实），`USE_NEBULAGRAPH=true`；先插入节点 N1、N2、边 E1(N1→N2)
- **When**: 第一次 `getNode(N1)` 记录后端调用计数；第二次 `getNode(N1)`；第三次修改 N1 后 emit CDC，再第四次 `getNode(N1)`
- **Then**: 一调后端=1；二调后端=0（L1 命中）；三调 CDC 失效 → 四调后端=1（再次拉取最新）
- **Pass Condition**: mock 后端计数器为 [1, 0, 1]
- **Evidence**: `test-nebula-read-l1-cdc.js` 通过，mock 打印调用序列与期望一致

### AC-7：图谱 bulk 写严格"先节点后边"，引用缺失返回警告不静默丢边
- **Type**: `rule`
- **Given**: 空图谱；提交 nodes=[N1] + edges=[(N1→MissingN, "r1"), (MissingX→N1, "r2")]
- **When**: 调 `POST /ai/engine/flow/graph_bulk_write`
- **Then**: 返回 `added.nodes=1`；`added.edges=0`；`warnings[].missingTargets = ["MissingN", "MissingX"]` 且两条失败边完整出现在 warnings 的 `skippedEdges` 字段中
- **Pass Condition**: warnings 结构可判定，不产生孤立成功假象
- **Evidence**: E2E 请求/响应 JSON 快照断言通过

### AC-8：社区检测算法 = CNM；LPA 任何对外出口关闭（回归护栏）
- **Type**: `rule`
- **Given**: Zachary Karate Club 数据集（34 节点，78 边）作为输入；输入采用 RAW 无向边（项目约束 RAW 输入）
- **When**: 调 `communityDetectionCNM()` 并记录模块度 Q；同时尝试 require 旧 `labelPropagation` 公开接口
- **Then**: Q ≥ 基准（0.37~0.41 常见区间，取 ≥ 0.35 通过）；旧 `labelPropagation` 入口在公开 API（路由 + 适配器 stats）上不再被调用且调用即抛 DeprecationError
- **Pass Condition**: Q≥阈值 + 旧 LPA 对外不可用（抛错）
- **Evidence**: `test-graph-cnm-vs-lpa.js` 通过；包含 DeprecationError 触发断言

### AC-9：Rust Gateway 4 个统一端点齐全且协议符合 §3.2
- **Type**: `rule`
- **Given**: Gateway 以开发模式启动，Node sidecar 3010 也启动
- **When**: 分别请求 `POST /ai/engine/process`、`POST /ai/engine/analyze`、`GET /ai/engine/capabilities`、`GET /ai/engine/metrics`
- **Then**: 四个端点 HTTP 状态码=200；响应 JSON schema 与 spec 中定义一致；`process` 的 `data` 字段 shape 与现状本地同 API 返回可 diff（仅多 ai_* 字段）
- **Pass Condition**: OpenAPI schema 校验全部通过 + `data` 段 shape 对比通过
- **Evidence**: `gateway/runtime/tests/ai_engine_e2e.rs` 全部断言通过；schema 校验日志无错误

### AC-10：路由匹配语义严格遵守"静态优先 → 参数少优先 → 同参数时长路径优先"
- **Type**: `rule`
- **Given**: 路由表注册 6 条：`/a/b/c`（静态3）、`/a/b/:x`（1参）、`/a/:y/c`（1参2段少）、`/a/:y/:z`（2参）、`/a/:y/:z/:w`（3参）、`/x/y/z/w`（静态4）
- **When**: 请求 `/a/b/c`、`/a/b/hello`、`/a/foo/bar`、`/x/y/z/w`
- **Then**: 命中目标分别为：静态3；1参 `/a/b/:x`（非 `/a/:y/c`，同 1 参但路径段更长的 `/a/b/:x` 优？不对，按企业级语义：参数段少者优先；同参数数时保留长路径优先 → 同 1 参时 `/a/b/:x` 的"实际匹配前缀更长" = 其等价静态段数更多，故优先）；2 参；静态4
- **Pass Condition**: 全部 4 个请求命中的 handler id 与预期数组完全一致
- **Evidence**: `test-router-semantics.js` 断言通过，打印命中顺序矩阵

### AC-11：AI 统一查询 data 段与现有本地接口返回 shape 完全兼容（老客户端零改）
- **Type**: `rule`
- **Given**: 老客户端仅消费 `data` 字段（忽略 `ai_*`）；分别用本地接口 `/graph/nodes` 与新接口 `/ai/engine/process{intent:"graph_list"}` 拿到结果
- **When**: 对两端返回的 `data[i].{id,label,type,attributes,created_at}` 字段集合与值做 JSON 等价比较
- **Then**: 同数据集下 100% 字段相等
- **Pass Condition**: 逐字段 deepEqual diff 大小=0
- **Evidence**: `test-ai-local-shape-compat.js` 通过

### AC-12：三语言 SDK 的 `ask` 与 `graph.list` 行为一致、协议统一
- **Type**: `rule`
- **Given**: Node/Python/Rust SDK 各一份，指向同一 Gateway；测试 query = "list Project nodes" 与自然语言 ask
- **When**: 三个 SDK 同时发起 `graph.list({kind:"Project"})` 与 `ask("列出所有 Project 节点")`
- **Then**: 两组调用返回的 `data` 字段按 id 排序后 deepEqual；`ask` 响应中多出 `route` 与 `metrics`，老字段无损
- **Pass Condition**: 三语言两组调用等价断言通过
- **Evidence**: 各自测试脚本 `sdk/*/tests/compat.{js,py,rs}` 输出全部绿色

### AC-13：三条核心业务流程在图谱中可追溯 + 接口冒烟一次通过
- **Type**: `rule`
- **Given**: 全新环境、空数据；启动全部组件（Gateway+backend-node+Postgres+MinIO+Redis mock）
- **When**: 顺序执行：(1) 图谱 bulk 写 (2) 文件上传 10MB + 触发自动图谱关联 (3) AI RAG 查询"刚上传的文档涉及哪些项目？"；每次完成后记录 traceId
- **Then**: 在图谱中以 traceId 搜索可得对应 step 节点与执行边；3 条流程全部返回 ok=true；最后 RAG 查询的 `ai_summary` 非空且包含刚上传文件名
- **Pass Condition**: 3 个流程 trace 节点存在 + RAG 结果包含文件名
- **Evidence**: E2E 日志 `tests/test-three-flows-traceable.js` 通过 + trace 图导出 JSON 快照

### AC-14：灰度优雅切换流程脚本闭环（新健康 → 切流量 → 停旧）
- **Type**: `rule`
- **Given**: 同时运行 "旧版 backend-node 3010" 与 "新版 backend-node 3011"，Gateway 指向 v1=3010；`canary.weight=0`
- **When**: 运行 `graceful-switch.ps1`（或 sh 等效）完成"探活 v2 → 权重 1 → 稳定 → 停 v1"整套流程
- **Then**: 脚本退出码=0；结束后 Gateway 内部 active_upstream=v2；旧进程 PID 不存在；过程中随机请求 100 次无 5xx
- **Pass Condition**: 脚本 exit=0 + active_upstream=v2 + 100 请求 0 5xx
- **Evidence**: 脚本执行日志 + 健康检查与 active_upstream 接口返回 JSON

### AC-15：算法公式全精度保留 + 密度指标人读解读齐全
- **Type**: `rule`
- **Given**: 同一 100 节点测试图，内部计算 `(betweenness/closeness/density/pagerank)` 后直接序列化 JSON 再 parse（不改数字）
- **When**: 检查其中 3 个非零指标的字符串是否出现 `.toFixed` 导致统一末位 00（或任意截断痕迹）；检查 density 返回对象
- **Then**: 公式对象字符串不包含任何 toFixed 样式统一截断（允许浮点自然不同长度）；density 返回含 `{value, formula, interpretation}` 三字段且 interpretation ∈ {"高度稠密图…", "中等密度…", "稀疏图…", "节点数不足 2…"}
- **Pass Condition**: 无截断痕迹；字段齐全、解读文案合法枚举
- **Evidence**: `test-graph-formula-precision.js` 断言通过 + 采样 JSON 输出快照含三字段

### AC-16：激活扩散意图识别 = 个性化 PageRank 特例 (d=0.85, 30 轮)
- **Type**: `rule`
- **Given**: 构造"关键词→能力"小图：10 个关键词节点、5 个能力节点、边权 1；种子节点 = 其中两个关键词
- **When**: 跑 `activateSpread` 3 次：(a) 默认参数 (b) 传 d=0.85, maxIter=30 (c) 故意传 d=0.5 再改回 (b)
- **Then**: (a) 与 (b) 输出完全一致（参数默认值即为 0.85/30）；(b) 种子相邻能力的能量排序稳定；(c) 与 (b) 不同（参数生效）
- **Pass Condition**: deepEqual(a,b)===true；!deepEqual(b,c)
- **Evidence**: `test-activate-spread-params.js` 断言通过，附带能量排序日志

### AC-17：PageRank 含转置图处理，传播方向正确
- **Type**: `rule`
- **Given**: 有向图：A→B, A→C, B→C（期望 C 最高，B 次，A 最低；权威沿出边传）
- **When**: 调用 PageRank，damping=0.85
- **Then**: 排序 pr(C) > pr(B) > pr(A)；与参考结果（Python networkx.pagerank 同图）差异的绝对值 < 1e-4（全精度）
- **Pass Condition**: 排序成立 + 与 networkx 差异 < 1e-4
- **Evidence**: `test-pagerank-transpose.js` 断言通过，含参考值快照

### AC-18：无向图 RAW 输入在库内双向展开，度中心性不被误算
- **Type**: `rule`
- **Given**: 输入 RAW 无向边 `[{S,T}]`（只给一条）；节点集 {S,T}
- **When**: 计算 degreeCentrality + 检查内部邻接表 `adj[S].includes(T) && adj[T].includes(S)`
- **Then**: adj 双向皆含；degree 结果 S.degree=T.degree=1；归一化值=1
- **Pass Condition**: 两个布尔为真 + degree 数值断言
- **Evidence**: `test-raw-edge-bilateral-expand.js` 通过

### AC-19：多目标评估（CEM 停止条件）正确实现
- **Type**: `rule`
- **Given**: 评估器配置 `weights={quality:0.55, speed:0.20, token:0.10, stability:0.15}`，停止条件 `sigmaBar < 0.06` 或 3 轮无改进
- **When**: 构造两条模拟序列：(a) σ 连续下降并在第 6 轮跌破 0.06 (b) 第 3 轮起 3 轮无改进
- **Then**: (a) 停止原因="sigmaBar<0.06"；(b) 停止原因="no_improvement_3_rounds"；每轮综合分 = 权重点积严格按规格
- **Pass Condition**: 停止原因与轮次完全符合；每轮打分与点积计算手工验算一致
- **Evidence**: `test-multi-objective-eval-cem.js` 通过

### AC-20：架构与流程图可通过真实调用断言闭环（"所有功能接口明确，通过业务流程图验证"）
- **Type**: `rubric`
- **Dimension**: 业务流程与接口一致性（图谱定义 vs 真实请求 vs 产物）
- **Scale**: 1-5
- **Anchors**: 1 = 流程只有文档，无真实执行入口；3 = 有入口但不能产出可解析 trace；5 = 三条核心流程皆可端到端调用、trace 图节点可被独立解析、每条流程至少一个可回归接口快照
- **Pass Threshold**: >= 4
- **Evidence**: AC-13 的测试输出 + 流程图 trace 节点 JSON 导出 + 独立 Reviewer 手工重放任一流程脚本均成功

### AC-21：系统模块边界清晰度与代码可维护性
- **Type**: `rubric`
- **Dimension**: 模块化与架构归一度
- **Scale**: 1-5
- **Anchors**: 1 = 大量跨模块直接 require，环依赖；3 = 有边界但偶有渗漏；5 = storage/file/graph/ai-engine/gateway/sdk 各自单向依赖，不循环，抽象接口处不混入业务逻辑
- **Pass Threshold**: >= 4
- **Evidence**: 依赖静态扫描报告（madge/类似工具）显示无环、方向与架构图一致

### AC-22：企业级稳定性（大规模数据与失败注入下无崩溃）
- **Type**: `rubric`
- **Dimension**: 稳定性与降级稳健性
- **Scale**: 1-5
- **Anchors**: 1 = 10 倍预期规模立即 OOM/超时；3 = 可运行但有明显性能悬崖与内存泄漏；5 = 50 万节点 bulk 写入、断网降级、GC 回收、CDC 重放 1000 次，内存变化线性无泄漏、错误全部可诊断
- **Pass Threshold**: >= 4
- **Evidence**: 压力脚本结果 + 故障注入（kill 某个 chunk server 或让 sidecar 503）下仍满足 AC-2/AC-3/AC-6 降级路径

## Open Questions
- [ ] Q1：实际生产 Nebula 还是 ArangoDB/Neo4j 作为最终分布式图选型？（Spec 默认先按 Nebula 接口落 Gremlin/nGQL 双适配结构；可在 P2 阶段 Review 前确定。本 Spec 不阻塞，代码上保留 graphd 驱动可注入抽象即可。）
- [ ] Q2：pgvector 的向量维度与 Embedding 模型？本 Spec 默认维度=1536（OpenAI 常见），但可在实现阶段改为 `EMBED_DIM` 配置，不强绑定特定模型。（不阻塞，允许后续切换。）
- [ ] Q3：SDK 三语言是否同时"随仓库发布"？若资源紧张，可允许 Node SDK 先行，Python/Rust 最小可用（2 周内）；本 Spec 验收以三语言最小可用均达到为 rule 门槛，Rubric 以质量打分。（需要用户确认。）

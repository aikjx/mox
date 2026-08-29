# 璇玑 分布式存储 + AI 统一查询 实施计划（tasks.md）

> 对应 spec：`.trae/specs/20260823-mox-storage-distributed-ai-unified-query/spec.md`
> 原则：每个任务是一个垂直切片，可独立自验证；依赖关系明确；每条 AC 至少被一个任务覆盖。

---

## Task 1：Storage 抽象落地 PostgresProvider + 双写回源
- **Status**: `pending`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - 在 `platform/backend-node/src/storage/index.js` 的 `StorageFactory` 中实现真实 `PostgresProvider`（基于 `pg` 驱动），与 SQLiteProvider 同构同参同返回；
  - 扩展 `config.storage.providers.postgresql` 的读取；
  - 增加 `DB_DUAL_WRITE` / `DB_READ_PREF` 环境变量支持，在 `getStorage()` 调用链中实现双写与"读优先→空读回源→回填"；
  - 保留自动 JSON 迁移幂等护栏（与 SQLite 同逻辑）。
- **Acceptance Criteria Addressed**: AC-1, AC-2, NFR-5（兼容）, CT-2（CommonJS）
- **Test Requirements**:
  - `rule` TR-1.1：在测试 Postgres 上跑与 SQLite 同数据集的行为等价测试（upsert/get/list/kv/log/search/migrate 全 API 覆盖率）。证据：`backend-node/test/test-storage-postgres.js` 全部断言通过 + diff 报告 0 行。
  - `rule` TR-1.2：`DB_DUAL_WRITE=true` + `DB_READ_PREF=postgres` 下删除 Postgres 5 行后，回源回填计数与总数正确。证据：`test-dual-write.js` 输出日志 + 计数断言通过。
  - `rule` TR-1.3：StorageFactory 的 `getStorage()` 不抛错，`switchDatabase('postgres')` 后再切回 `sqlite` 不泄漏连接、不影响后续读。证据：`test-storage-switch.js` 通过。

## Task 2：FileStore 抽象 IChunkBackend 双实现 + GC
- **Status**: `pending`
- **Priority**: high
- **Depends On**: T1（Postgres 可用，便于存元数据）
- **Description**:
  - 新建 `platform/backend-node/src/storage/chunk-backend.js`，定义 `IChunkBackend` 接口与 2 个实现：`FSChunkBackend`（现状 fs 逻辑抽离）、`S3ChunkBackend`（基于 `@aws-sdk/client-s3` 兼容 MinIO/OSS/S3）。
  - 改造 `file-store.js`：构造注入 backend；`FILE_BACKEND=fs|s3` 切换；`FILE_MPU_CONCURRENCY`；`FILE_GRACE_DAYS`。
  - MPU：≥100MB 自动走 MPU；其余按 1MB 固定分块（保留 SHA-256 去重）。
  - GC：`soft_deleted` 作业；chunk 引用计数在删除 vN 时更新，到 0 且文件所有 v 过期才删。
- **Acceptance Criteria Addressed**: AC-3, AC-4, AC-5, FR-2
- **Test Requirements**:
  - `rule` TR-2.1：S3 backend 上传同内容 A/A' 去重，B 不同内容不分摊。chunk 数断言通过。证据：`test-file-store-s3.js`（本地 MinIO + 内存 mock 双模式）。
  - `rule` TR-2.2：128MB 伪随机文件 MPU 上传下载 hash 一致。证据：`test-file-mpu-128m.js` 输出前后 hash 日志。
  - `rule` TR-2.3：软删→GC→彻底删三段状态机正确。证据：`test-file-store-gc.js` 通过。

## Task 3：Nebula 适配器读端优先 + L1 缓存 + CDC 事件总线
- **Status**: `pending`
- **Priority**: high
- **Depends On**: None（可并行）
- **Description**:
  - 抽离 `platform/backend-node/src/graph/remote-graph-driver.js`：支持 Gremlin/nGQL 两种调用协议（HTTP Transport，可注入 mock）。
  - 改造 `NebulaGraphAdapter`：`USE_NEBULAGRAPH=true` 读端先调远程；失败→L1（LRU-ttl）→再失败→抛错；CDC 事件总线：内存实现 + Redis Stream 实现（配置化）。
  - 写端 createNode/createEdge/bulkUpsert：成功写后 emit CDC；消费端失效 L1 + 调用索引更新钩子（给后续 pgvector 留接口）。
- **Acceptance Criteria Addressed**: AC-6, FR-3
- **Test Requirements**:
  - `rule` TR-3.1：mock Gremlin 驱动的调用序列 = [1,0,1]（首读→命中→改+失效→再读）。证据：`test-nebula-read-l1-cdc.js`。
  - `rule` TR-3.2：远程 503 时 L1 仍能读且返回降级标记。证据：`test-nebula-degrade.js` 通过。

## Task 4：图算法归一：CNM 社区检测 + RAW 边展开 + 精度兜底
- **Status**: `pending`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - 在 `ai-flow-graph.js` 新增 `communityDetectionCNM(nodes, edges)`：模块度贪心凝聚（CNM），严格与项目约束一致。禁用旧对外 LPA：在 `graph-algos.js` 导出打上 DeprecationError（但仍保留内部函数引用供对比测试）。
  - 统一对外接口（适配器 stats、`/graph/analytics`）全部切换为 CNM 结果，字段 shape 不破坏。
  - RAW 边展开：在 `ai-flow-graph.js` 提供 `_expandRawEdges(edges)`，度/介数/社区算法全部走它；PageRank 仍按有向边（与项目约束一致，不二次扩展到无向，除非调用方传方向参数）。
  - 检查所有算法公式：禁用 toFixed 截断；density 返回三字段（value/formula/interpretation）。
- **Acceptance Criteria Addressed**: AC-8, AC-15, AC-18, CT-1, CT-5
- **Test Requirements**:
  - `rule` TR-4.1：Zachary Karate Club CNM Q ≥ 0.35；调用旧 LPA 公开 API 抛 DeprecationError。证据：`test-graph-cnm-vs-lpa.js`。
  - `rule` TR-4.2：RAW 边双向展开断言；度中心性正确。证据：`test-raw-edge-bilateral-expand.js`。
  - `rule` TR-4.3：公式不包含 toFixed；density 三字段与解读文案枚举合法。证据：`test-graph-formula-precision.js`。

## Task 5：PageRank 转置图 + 激活扩散参数锁死（d=0.85/30 轮）
- **Status**: `pending`
- **Priority**: high
- **Depends On**: T4
- **Description**:
  - 强化 PageRank 实现：严格按出边方向传播（转置图处理方式保留，单源实现已接近，需加对照测试）；
  - `activateSpread` 的默认参数锁死为 `decay=0.85`, `maxDepth=30`（项目约束 method=spread 的等价实现）；对外参数覆盖仍允许，但默认值必须合规。
- **Acceptance Criteria Addressed**: AC-16, AC-17
- **Test Requirements**:
  - `rule` TR-5.1：A→B, A→C, B→C 三节点有向图 PageRank 排序 C>B>A，并与 networkx 参考结果差 <1e-4。证据：`test-pagerank-transpose.js` + 参考值快照。
  - `rule` TR-5.2：activateSpread 默认传参 vs 显式 d=0.85/30 结果 deepEqual；改 d=0.5 时不同。证据：`test-activate-spread-params.js`。

## Task 6：Rust Gateway `/ai/engine/*` 四端点 + sidecar 协议
- **Status**: `pending`
- **Priority**: high
- **Depends On**: T4, T5（意图识别依赖激活扩散算法）
- **Description**:
  - 新增 `platform/gateway/runtime/src/handlers/ai_engine.rs`：4 个端点 + schema 校验；`routes/ai_engine.rs` 挂载。
  - 新增 `platform/gateway/runtime/src/ai_router.rs`：路由匹配语义实现（静态优先 → 参数少优先 → 同参数长路径优先）。
  - 新增 sidecar 调用层：`platform/gateway/runtime/src/sidecar/node_sidecar.rs`，通过本地 HTTP 到 127.0.0.1:3010 的内部 endpoints `/internal/intent` 与 `/internal/graph-algo`（由 backend-node 在 FR 中新增）。
  - 路由决策 pipeline：意图分类 → 激活扩散 → 缓存探测 → 能力路由 → 执行 → 回填。
  - `capabilities` 自描述 + `metrics`（5 分钟滑动窗口）。
- **Acceptance Criteria Addressed**: AC-9, AC-10, NFR-4
- **Test Requirements**:
  - `rule` TR-6.1：四端点 HTTP=200 且 schema 校验通过。证据：`runtime/tests/ai_engine_e2e.rs`。
  - `rule` TR-6.2：路由语义 6 条表对 4 个请求的命中顺序完全符合 AC-10。证据：`runtime/tests/router_semantics.rs`。
  - `rule` TR-6.3：sidecar 不可用时返回可诊断错误（5xx 不吞栈），metrics 计数增加 `sidecar_fail`。证据：`runtime/tests/sidecar_degrade.rs`。

## Task 7：Node 侧内部 endpoints + 意图识别统一接入 graph search 重排
- **Status**: `pending`
- **Priority**: high
- **Depends On**: T5, T6
- **Description**:
  - 在 `backend-node/src/routes/internal.js` 新增 `/internal/intent` 与 `/internal/graph-algo`，复用 intent-classifier 与 ai-flow-graph；
  - `GET /graph/search?q=` 新增"关键词匹配 + 激活扩散重排"组合（默认激活扩散权重可调，默认 0.7）；保持老字段 shape 不变；
  - 多目标评估（CEM）在 `ai-engine-core.js` 落地：停止条件 σ̄<0.06 或 3 轮无改进，严格按 `0.55Q+0.20S+0.10T+0.15Stability` 公式。
- **Acceptance Criteria Addressed**: AC-10（Node 侧也做路由矩阵，供 sidecar 调）, AC-19, CT-1（CEM 停止条件）
- **Test Requirements**:
  - `rule` TR-7.1：CEM 模拟两条序列停止原因与轮次正确、加权分与手工验算一致。证据：`test-multi-objective-eval-cem.js`。
  - `rule` TR-7.2：internal endpoints 响应 schema 通过；graph search 返回的前 3 条相较旧版纯 LIKE 至少包含一条激活扩散带来的新命中（基于测试 fixture）。证据：`test-graph-search-rerank.js`。

## Task 8：协议统一：/ai/engine/process data 段与本地接口同 shape（兼容老客户端）
- **Status**: `pending`
- **Priority**: high
- **Depends On**: T6, T7
- **Description**:
  - 在 Gateway 的 process 处理中，对"graph_list / graph_node_get / file_list / file_get"等本地等价能力，执行器直接调用 sidecar → backend-node 原生接口拿 data 段原样填回响应；
  - AI 附加值字段仅增加 `ai_summary`、`ai_evidence`、`route`、`metrics`，不改变 `data[i]` 任何键；
  - 新增 `/ai/engine/process?compat=true` 默认开；若未来有破坏性变更必须 bump 版本字段但保持 compat 可用。
- **Acceptance Criteria Addressed**: AC-11
- **Test Requirements**:
  - `rule` TR-8.1：同数据集下 deepEqual(本地data, AI.data) === true。证据：`test-ai-local-shape-compat.js`。

## Task 9：三语言 SDK（Node/Python/Rust）最小可用 + 兼容测试
- **Status**: `pending`
- **Priority**: medium
- **Depends On**: T8
- **Description**:
  - 新建 `sdk/mox-sdk-node/`、`sdk/mox-sdk-py/`、`sdk/mox-sdk-rs/`；
  - 公共能力：client 配置（base/token/timeout/max_latency_ms）；`graph.list()` 等本地直接代理；`ask(query, ctx, opts)` 走统一 endpoint；SDK 内置 LRU 1K/60s；5xx/429 退避重试（默认 2 次）；SSE 流式 `on('data')`。
- **Acceptance Criteria Addressed**: AC-12
- **Test Requirements**:
  - `rule` TR-9.1：三语言 `graph.list` 返回 deepEqual。证据：三测试脚本各自测试通过。
  - `rule` TR-9.2：三语言 `ask` 响应包含 `route` 与老字段无损。证据：compat 测试脚本。
  - `rubric` TR-9.3：SDK 代码风格与惯用（Node 用 Promise/ESM OK 但需 CommonJS 兼容；Python 用 dataclass；Rust 用 typed builder）。维度：SDK 质量；scale 1-5；anchors：1=不可编译；3=可用但无文档测试；5=三语言均有测试 + README + 零误用陷阱；阈值>=4。证据：各目录测试 + README 走读。

## Task 10：业务流程图谱承载 + 三流程端点（graph_bulk/file_upload/ai_rag）
- **Status**: `pending`
- **Priority**: high
- **Depends On**: T3, T7, T8
- **Description**:
  - 定义三张流程的 step/capability/keyword/engine 图谱骨架（作为内置 fixture 随系统启动 upsert 到图谱）。
  - 新增 `POST /ai/engine/flow/graph_bulk_write`：节点预验 → 节点 upsert → CDC → 全局提交点等待 → 边 upsert（RAW 双向展开，缺失目标返回 warnings，不静默丢失）→ 增量公式重算（变化边 ≤ 10% 走增量，否则整图公式）。
  - 新增 `POST /ai/engine/flow/file_upload_auto_link`：uploadFile 后 KB pipeline（解析 → 实体抽取 → diff 建议）→ 写图谱三元组（通过 graph_bulk_write）→ linkedGraphIds 绑定。
  - 新增 `POST /ai/engine/flow/ai_rag_query`：pgvector 语义缓存命中探测 → 意图拆分 → fan-out（graph/file/code/logs 四路并行，代码用 project-atlas 桥接）→ RRF 融合 → 专家联盟辩论合成 → 回填缓存 + trace 图谱节点。
  - 三个流程各自把 step 执行记录写回 trace 图谱，形成"请求→流程步骤→产物"可追溯链。
- **Acceptance Criteria Addressed**: AC-7, AC-13, AC-20
- **Test Requirements**:
  - `rule` TR-10.1：graph_bulk_write 的 warnings/skippedEdges 返回正确（缺失节点边不静默丢失）。证据：`test-flow-graph-bulk-warnings.js`。
  - `rule` TR-10.2：文件上传自动链接后 linkedGraphIds 非空、KB 三元组节点存在于图谱中。证据：`test-flow-file-upload-auto-link.js`。
  - `rule` TR-10.3：RAG 查询能命中刚上传的文件名，ai_summary 包含该名。证据：`test-flow-ai-rag.js`。
  - `rule` TR-10.4：三次调用后，按各自 traceId 搜索图谱分别可得≥3 step 的 trace 子图。证据：E2E 快照 `three-flows-trace.json`。

## Task 11：灰度发布优雅切换脚本 + 就绪探针 + 预热判定
- **Status**: `pending`
- **Priority**: medium
- **Depends On**: T6, T10（组件都有健康探活）
- **Description**:
  - Gateway `/system/traffic`：canary.weight、dualWrite、readPref、warmup.targets 配置；
  - 新增 `/ready`：预热（跑基准 SQL 查询 pg_stat_statements 命中率，或本地 LRU 命中率）完成后才 200；否则 503；
  - `platform/scripts/graceful-switch.ps1` 与 `graceful-switch.sh`：探活 v2 → 权重 0→1 → 稳定 → 停 v1。Windows 脚本用 PowerShell，Linux 用 bash，步骤一致，日志可审计。
- **Acceptance Criteria Addressed**: AC-14, 部署需求（用户偏好：优雅切换新服务→停旧）
- **Test Requirements**:
  - `rule` TR-11.1：本地启动双版本，脚本退出 0，active_upstream=v2，100 随机请求 0 5xx。证据：脚本运行日志 + curl 统计。
  - `rubric` TR-11.2：部署脚本可读性。维度：部署可维护性；scale 1-5；anchors：1=步骤混乱；3=可用但无审计；5=幂等、有日志、有 rollback 按钮、跨平台；阈值>=4。证据：脚本源码走读 + dry-run 输出。

## Task 12：测试基础设施、交付矩阵、报告
- **Status**: `pending`
- **Priority**: medium
- **Depends On**: T1-T11（最后汇）
- **Description**:
  - 在 `backend-node/test/` 与 `gateway/runtime/tests/` 与 `sdk/*/tests/` 分别新增对应测试文件；
  - 新增 `platform/scripts/run-full-regression.ps1` 跑完所有测试并生成：覆盖率报告、性能报告、算法精度报告、降级回归报告、三流程 trace JSON；
  - 输出 08-企业级交付清单风格的验收矩阵（Markdown 报告，写入 `docs/enterprise/18-mox-storage-ai-unified-query-report.md`）。
- **Acceptance Criteria Addressed**: AC-20, AC-21, AC-22
- **Test Requirements**:
  - `rubric` TR-12.1：模块边界清晰度。维度：模块化；scale 1-5；anchors 1/3/5 与 AC-21 一致；阈值>=4。证据：madge 依赖图 + 无环报告。
  - `rubric` TR-12.2：稳定性。维度：大规模/降级；scale 1-5；与 AC-22 对齐；阈值>=4。证据：50 万节点 bulk 写入脚本、sidecar 故障注入、MinIO 单节点断电、CDC 重放 1000 次。
  - `rule` TR-12.3：回归脚本 exit=0 且全部 AC 的 TR 在报告中有对应证据链接。证据：脚本输出 + 报告存在。

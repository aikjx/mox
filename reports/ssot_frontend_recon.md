# SSOT 契约层与前端就绪度侦察报告

- 仓库：`D:\a10\aikjx\gitcode\infotopograph`
- 侦察方式：只读（未修改任何源文件）
- 日期：2026-09-27
- 范围：三个 SSOT crate 暴露面 / `platform/domains/alliance` 对齐缺口 / `frontend-ui` 现状与可做边界

---

## 一、三个 SSOT crate 暴露面清单

### 1. `platform/shared/mox-unified-contract`（v3.0.0-ai-powered）

定位：跨 Rust / Python / 前端三端的统一类型契约。零业务依赖，仅 serde/chrono/uuid/thiserror/tracing/rand。`CONTRACT_VERSION = "1.0.0"`。

| 模块 | 对外暴露的核心原语 |
|---|---|
| `error.rs` | `ErrorLevel`(Info/Warning/Error/Critical)；`ErrorDomain` 13 域枚举（PL/AI/AL/KG/CL/FL/DT/PJ/RS/US/MK/VC/OP，含 `code()`/`name()`/`from_code()`）；`ErrorCode{domain,module,seq}` → 7 位字符串如 `AL05123`；`MoxError{code,message,detail,level,http_status,trace_id,timestamp}` + 快捷构造器 `bad_request/unauthorized/forbidden/not_found/conflict/unprocessable/too_many_requests/internal/unavailable/unknown`；`MoxResult<T>` |
| `event.rs` | `EventPhase` 七阶段枚举（intent/team/debate/synthesize/gate/learn/done，带 index/name/label/icon/color）；常量 `PHASE_NAMES[7]`、`AUDIT_EVENTS_7[7]`（`ALLIANCE_START…ALLIANCE_DONE`）；`EventType`(PhaseStarted/PhaseData/Progress/Complete/Error/Audit)；`MoxEvent{event_id,event_type,phase,trace_id,payload,latency_ms,timestamp,degraded,degrade_reason}` + `to_sse()`；`StreamEvent`（SSE 轻量投影）；`ProgressEvent{phase,current,total,message,trace_id}` |
| `trace.rs` | `TraceId`(UUIDv4)；`SpanId`(u64 hex)；`TraceContext{trace_id,span_id,parent_span_id,service,operation,start_time_ms,sampled}` + `new_root/child/to_headers/from_headers`（HTTP 头 `X-Trace-Id`/`X-Span-Id`/`X-Parent-Span-Id`）；`current_trace_id()`/`with_trace_context()` |
| `response.rs` | `ApiResponse<T>{code,msg,data,trace_id,latency_ms,timestamp}`（code=0 成功）；`ApiSuccess<T>`/`ApiError`；`PagedResponse<T>`+`PaginationInfo{page,page_size,total,total_pages,has_next,has_prev}` |
| `pagination.rs` | `PaginationRequest{page,page_size}`（offset/limit/validate，page_size clamp 1–100）；`SortOrder`(asc/desc)；`SortCondition`；`FilterOperator` 13 算子（eq/ne/gt/gte/lt/lte/contains/starts_with/ends_with/in/not_in/is_null/is_not_null/between，带 `as_sql()`）；`FilterCondition`；`QueryRequest{pagination,sorts,filters,keyword}` |
| `quality.rs` | `QualityGrade`(A/B/C/D，passed/retryable/blocked/color)；`GateThresholds{a:0.90,b:0.80,c:0.70}` 常量 `GATE_THRESHOLDS`（HC-8 硬阈值锁值，测试守护）；`QualityScore(f64)` newtype；`GateResult{score,grade,passed,dimensions,block_reason,suggestions,latency_ms}` |
| `normalize.rs` | 纯函数：`clamp_score`、`normalize_min_max`、`z_score`、`sigmoid`、`softmax`、`normalize_weights`、`compute_consensus`(0.7·std+0.3·conf)、`weighted_average`、`synthesis_weight`(0.5/0.3/0.2)、`delivery_gate_score`(0.55/0.25/0.20)；可配置权重结构 `NormalizationConfig` + `validate()` |

### 2. `platform/shared/mox-unified-algo-core`

定位：跨域算法归一化（KG / 专家联盟 / 云盘检索三域共享）。依赖 `mox-platform-foundation`/petgraph/nalgebra。features：默认 `graph-algo/similarity/ranking/clustering`，`full` 追加 activation/embedding/stats。

- **types.rs**：`AlgoResult<T>{algo_id,algo_version,status,data,error,metrics}`；`AlgoStatus`(Success/Partial/Failed/Running)；`AlgoMetrics`(duration_ms/peak_memory/iterations/convergence_error/nodes_processed/edges_processed)；`ScoredItem<K>{key,score,rank,confidence,score_breakdown}`；`RankingResult<K>`；`SimilarityMethod`(Cosine/Jaccard/Euclidean/Manhattan/Pearson/Levenshtein/Hamming)；`SimilarityResult<K>`；`ClusteringMethod`(KMeans/Hierarchical/Dbscan/Spectral/Community)；`Cluster<K>`/`ClusteringResult<K>`；`GraphAlgoType`；`CentralityType`(Degree/Betweenness/Closeness/Eigenvector/Katz/PageRank)；`PathResult<K>`；`AlgoConfig`；`DenseVector`(norm/normalize)。
- **traits.rs**：根 `Algorithm`(id/name/version/description)；`SyncAlgorithm<I,O>`、`AsyncAlgorithm<I,O>`；`RankingAlgo<K>`/`IncrementalRanking<K>`；`SimilarityAlgo<T>`（带默认 `top_k_similar`）；`VectorSimilarity`（默认 cosine/euclidean）；`ClusteringAlgo<K>`；`GraphAlgorithm` 及子 trait `CentralityAlgo`/`PageRankAlgo`/`CommunityDetectionAlgo`/`ShortestPathAlgo`；联盟专用 `FusionAlgo<T>` + `ExpertOutput<T>` + `FusionResult<T>`。
- **registry.rs**：`AlgoInfo`、`AlgoCategory`(8)、`AlgoRegistry`（RwLock 注册表，register/get/list/list_by_category/search_by_tag）；14 条 `BUILTIN_ALGORITHMS` 静态描述；`global_algo_registry()` OnceLock 单例。
- **algorithms/**：`similarity.rs`、`ranking.rs` 为真实实现；`clustering/activation/embedding/stats` 均为空占位模块（README 宣称但未落地）。
- 跨域参数常量：`PPR_DAMPING=0.85`、`PPR_MAX_ITER=30`、`LOUVAIN_MAX_ITER=100`、`DEFAULT_EMBEDDING_DIM=384` 等。

### 3. `platform/domains/data/core/mox-data-norm-core`

定位：归一化流水线权威单源（去重 / 规则求解 / 冲突融合）。crate-type 含 `cdylib`/`staticlib`（FFI 预留）。

- **`NormRecord`**：归一化记录载体 `{id, attributes: HashMap<String, Value, ahash::RandomState>, source, updated_at_ms, confidence:f32=0.5}`。
- **`dedup.rs`**：`dedup_records(&[NormRecord]) -> (Vec<NormRecord>, DedupReport)`。指纹 = ahash64(id) XOR ahash(sorted 属性 keys)；同桶内全属性比对；胜者 = 高置信度优先，平局取新鲜度。`DedupReport{input,unique,duplicates_removed,kept_by_confidence,kept_by_freshness}`。
- **`rules.rs`**：`Rule{id,conditions,actions,priority}`；`Condition{field,op,value}`（op: ==/!=/>/>=/</<=/contains/in/regex[实际=contains]）；`Action` 枚举（Set/Rename/Map/Delete，serde tag=`"kind"`）；`RuleEngine::new/apply`；批处理入口 `resolve_rules`。
- **`merge.rs`**：`MergeStrategy` 6 变体（HighestConfidenceFirst / UnionAttributes / SourceAuthority{src_order} / LastWriteWins / Majority / UnionFields）；`merge_records(&[NormRecord], &MergeStrategy) -> MergeResult`；自定义冲突回调 `merge_conflicts` + `ConflictMergeFn`；`MergeResult{merged,merged_groups,conflicts_resolved}`。

---

## 二、alliance 域与 SSOT 的对齐缺口

### 2.1 依赖事实（Cargo.toml grep）

全仓显依赖这三个 SSOT crate 的业务 crate 共 9 个：

- `platform/shared`：`mox-observability-core`、`mox-auth-core`、`mox-config-core`
- `platform/domains/ai/core/mox-ai-alliance-engine`（`mox-unified-contract = { workspace = true }`，Cargo.toml:55）
- `platform/domains/ai/svc/mox-ai-expert-svc`
- `platform/domains/data/sdk/mox-data-norm-intent-native`
- `platform/domains/cloud/core/mox-cloud-kb-core`
- `platform/domains/flow/svc/mox-flow-ea-workspace-svc`

**关键事实：整个 `platform/domains/alliance/` 目录树（16 个 crate）没有任何一个依赖这三个 SSOT crate。** 它们的共同依赖只有 `mox-platform-foundation` + 自身 proto crate。

### 2.2 结构真相：存在两套"联盟"后端

| 后端族 | 路径 | 角色 | 是否接 SSOT |
|---|---|---|---|
| 7 阶段辩论引擎 | `platform/domains/ai/core/mox-ai-alliance-engine` | intent→team→debate→synthesize→gate→learn→done 管线 | **是**（接 `mox-unified-contract`） |
| DAG 任务编排服务 | `platform/domains/alliance/{core,svc,proto,sdk,api}` | scheduler/executor/registry 任务编排、NATS 风格事件 | **否** |

### 2.3 重叠清单（domains/alliance 各自重复定义）

| SSOT 契约 | domains/alliance 平行定义 | 位置 | 差异 |
|---|---|---|---|
| `MoxError`/`ErrorCode`("AL05123")/`MoxResult` | `AllianceErrorCode`(u32 枚举 1000–7999) + `AllianceError`(thiserror) + `AllianceResult<T>` | `proto/mox-alliance-common-proto/src/error.rs` | 错误码空间不同（u32 vs 2 字母+5 数字），错误模型不同 |
| `MoxEvent`/`StreamEvent`/`ProgressEvent`/`EventPhase`(7 阶段) | `AllianceEvent`{Task,Node,Expert} + `TaskEvent`/`NodeEvent`/`ExpertEvent` + `TaskAction`/`NodeAction`/`ExpertAction` | `proto/mox-alliance-common-proto/src/events.rs` | 事件分类是任务/节点/专家生命周期，不是 7 阶段辩论管线——抽象层不同，非纯重复 |
| `ApiResponse<T>`(code/msg/data/trace_id) | `SuccessResponse{success,message}` / `ErrorResponse{success,error_code:u32,message}` | `api/src/dto.rs:159-190` | 信封不同 |
| （foundation 层） | `mox_api_protocol::{ApiResponse, api_ok, api_error}` | `platform/foundation/mox-api-protocol`，被 `sdk/mox-alliance-http-sdk` 使用 | **第三套信封**，网关/orchestrator/kg 也用它 |
| `PagedResponse<T>`/`PaginationInfo` | `TaskListResponse{tasks,total,page,page_size}` | `api/src/dto.rs:58-64` | 手写分页 |
| `TraceId`/`TraceContext` | 无（事件内直接用裸 `Uuid`，无 trace 透传模型） | — | 缺口 |
| `QualityGrade`/`GateResult` | 无（质量门禁在 ai-alliance-engine 内，已接 SSOT） | — | 无重叠 |
| SSOT 无任务模型 | `Task`/`Node`/`Expert`/`TaskStatus`/`NodeStatus`/`ExpertStatus`/`FusionStrategy`/`MatchingWeights` | `proto/mox-alliance-common-proto/src/types.rs`(41KB) | 这是领域原生 DAG 模型，SSOT 从未声称拥有，不算缺口 |

### 2.4 缺口结论

1. **domains/alliance 未接 SSOT 是事实**：它有自己完整的 error/event/DTO 栈，HTTP SDK 走 foundation 的 `mox_api_protocol` 信封，与 SSOT `ApiResponse<T>` 是两套 JSON。
2. **当前线上同时存在至少三种响应信封**：SSOT `ApiResponse`(code/msg/data/trace_id)、foundation `mox_api_protocol`、alliance DTO `{success, error_code}`。前端 `contract/endpoints.js` 已被迫用 `nesting: 'flat' | 'nested'` 逐端点标注信封层数，并在 UI 注释里明确"两套 handler 族，信封不同，不共用 store"。
3. **真正接了 SSOT 的是 7 阶段引擎**（`mox-ai-alliance-engine`），前端 `contract/phases.js` 的 7 阶段常量就是它的投影。
4. **对齐建议（后端侧，非前端任务）**：domains/alliance 至少应在 error/trace/response 信封上向 SSOT 收敛，或显式 ADR 声明 foundation 信封为该域权威，避免三信封长期并存。

---

## 三、前端现状（frontend-ui/src）

### 3.1 模块结构

存在一个**完全成形**的专家联盟自包含模块：`src/modules/expert-alliance/`。

```
modules/expert-alliance/
├── index.js                 # defineModule 声明式注册（路由+导航单源）
├── module.test.js           # 模块声明门禁测试
├── contract/                # 端点/阶段/枚举/请求体契约 + 逐文件 .test.js
│   ├── endpoints.js         # 80+ 端点，每条带 registry ID + nesting(flat|nested)
│   ├── phases.js           # 7 阶段 SSOT 投影（直接引用 Rust 源，drift 测试守护）
│   ├── enums.js / collab.js / dispatcher.js / graph.js
│   ├── mode.js / orchestration.js / registry.js / sessions.js
├── api/alliance.api.js      # createAllianceApi(httpClient) 工厂
├── model/normalize.js      # 43KB 出参归一化器（按端点分支 flat/nested）
├── store/                   # 6 个 pinia store（collab/console/experts/graph/orch/sessions）
├── components/              # 15 个 .vue 组件
└── views/                   # 6 个 .vue 页面
```

- **路由**：`router/index.js` 通过 `collectModuleRoutes()` 从模块注册表派生；`router/modules/alliance.js` 现在只保留 `/expert`、`/alliance` 等 redirect 别名。已注册 11 条路由：`/expert-workspace`、`/expert-center`(overview/enterprise/orchestrator/tasks 4 子路由)、`/expert-config`、`/expert-plaza`、`/alliance/{console,collab,graph,sessions,orchestration,experts}`。
- **legacy 视图并存**：`src/views/expert/` 下还有 `ExpertCenterView.vue`、`ExpertConfigView.vue`(192KB)、`ExpertPlazaView.vue`、`AllianceTaskView.vue`(31KB)、`panels/` 三个面板。
- **根级 legacy API client**：`src/api/alliance.api.js`、`allianceTaskModel.api.js`、`experts.api.js`、`allianceTaskModel.api.test.js`、`src/stores/alliance.store.js`、`src/composables/useAllianceTasks.js`、`src/components/expert/RegisterExpertDialog.vue`。

### 3.2 测试范式

- **Vitest 2.x** + happy-dom + globals，配置 `vitest.config.js`：include `src/**/*.{test,spec}.{js,ts}`，别名 `@` → `./src`。
- **脚本**（package.json）：`test` = `vitest run`；另有 `dev/build/preview/lint/storybook`。**没有 playwright npm script**。
- **Playwright 1.48** 已安装且 `playwright.config.js` 存在（testDir `./tests`，chromium 项目只跑 `*.@P0.spec.js`，webServer `vite preview :4173`），但需 `npx playwright test` 手动触发，未接入 `pnpm test`。
- **用例写法范式**（`module.test.js`）：`import { describe, it, expect } from 'vitest'`，与源文件同目录 `*.test.js` 并列；断言路由 meta 完备性、EP 图标命名、注册表不变量。
- **契约 drift 守护**：`contract/contract.test.js`(87KB) 直接读 Rust 源文件断言 JS 投影一致；`endpoints.js` 每条 path 必须在 `docs/API-REGISTRY.md` 有同名字面量，由 contract.test.js 守护。这是仓库已建立的跨端 SSOT 对齐机制。

### 3.3 最小 .vue 页面范式（`views/AllianceCollabView.vue`，4.4KB）

- `<script setup>` Composition API；
- 从 `@/modules/expert-alliance/store` 引 pinia store，从 `@/modules/expert-alliance/components` 引面板组件；
- 页面 = 薄外壳（header + grid 布局），重逻辑下沉到 panel 组件 + store；
- scoped 样式用 CSS 变量（`--text-primary`/`--border`/`--bg-card`/`--radius-md`）；
- Element Plus 组件（el-tag/el-button/el-alert）+ `@element-plus/icons-vue`。

### 3.4 后端就绪度判断

**前端不是在等后端。** `contract/endpoints.js` 登记了 80+ 个端点，每条都带 handler 源码行号引用（如 `alliance.rs:657-681`、`experts_registry.rs:247-254`）和信封层数标注；`model/normalize.js` 已按端点分支处理 flat/nested 两种信封；视图与 store 均有配套测试。

真正的问题不是"后端接口未就绪"，而是**后端存在两套信封、domains/alliance 未接 SSOT**——前端已用 `nesting` 标注和分 store 绕开，但这是后端契约分歧，不是前端能收口的。

---

## 四、建议：前端能安全做什么、不能做什么

### 4.1 前端现在就能安全做的

1. **新增最小页面**：按既有范式——
   - 建 `modules/expert-alliance/views/XxxView.vue`（薄外壳，参考 AllianceCollabView）；
   - 在 `modules/expert-alliance/index.js` 的 `routes` + `nav` 各加一条（meta 必须含 `title/module/layout`，icon 用 EP 图标名）；
   - 新端点在 `contract/endpoints.js` 登记（registry ID 对齐 `docs/API-REGISTRY.md`，标注 nesting）；
   - 出参在 `model/normalize.js` 加归一化器，在 `api/alliance.api.js` 的 `call()` 体系里包一层；
   - 同目录补 `*.test.js`。
2. **扩既有视图 / 组件**：store、组件、测试范式齐全，直接加。
3. **加 vitest 单测**：沿用 colocated `*.test.js` 约定，跑 `pnpm test`。
4. **加 API wrapper**：`call()` helper 已统一处理 flat/nested 解包与错误冒泡。
5. **（可选）补 Playwright P0 用例**：放到 `tests/` 目录、文件名带 `@P0.spec.js`，配置已就绪。

### 4.2 前端现在不能 / 不该做的

1. **不要试图在前端统一 flat 与 nested 信封**——`model/normalize.js` 已按端点分支；强行合并会掩盖后端真实的信封分歧（这需要后端先收敛）。
2. **不要在 JS 里手写新的错误码 / 阶段名 / 质量等级常量**——只能改 `contract/phases.js`、`contract/enums.js` 这两个投影点，且 contract.test.js 会反向读 Rust 源做 drift 断言。
3. **不要假设 `/api/alliance/*` 返回 SSOT `ApiResponse<T>`(code/msg/data/trace_id)**——这些路由返回的是 foundation `mox_api_protocol` 的 nested 信封；后端不先动，前端不能换解包逻辑。
4. **新增联盟任务/事件模型前先确认服务于哪套后端**：7 阶段引擎（ai-alliance-engine，SSOT 信封）vs DAG 编排服务（domains/alliance，foundation 信封），两者前端 store 已刻意隔离。
5. **不要改 `router/modules/alliance.js` 加业务路由**——业务路由必须走 `defineModule` 声明式注册，该文件只允许 redirect 别名。

### 4.3 后端侧待办（不在本次前端任务范围，仅记录）

- `platform/domains/alliance/` 16 个 crate 在 error / trace / response 信封上向 `mox-unified-contract` 收敛，或显式 ADR 声明 `mox_api_protocol` 为该域权威信封；
- 消除"SSOT `ApiResponse` / foundation `mox_api_protocol` / alliance DTO `{success,error_code}`"三信封并存；
- 给 domains/alliance 补 trace 透传模型（当前事件内裸 Uuid，无 X-Trace-Id 传播）。

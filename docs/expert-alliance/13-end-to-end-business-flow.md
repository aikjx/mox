# 13 专家联盟端到端全业务流程

> **权威优先级**：现状事实以 [CURRENT-ARCHITECTURE.md V1.1](CURRENT-ARCHITECTURE.md) 为最终裁决；
> 端口以 [docs/api/PORT-REGISTRY.md](../api/PORT-REGISTRY.md) V1.2 为唯一权威；
> 枚举字典以 [08-normalized-architecture.md §八](08-normalized-architecture.md) 为统一版。
> 本文是 CURRENT V1.1 事实下的端到端业务流程拆解，**不沿用 03-business-flow.md（V1.0）已过期结论**，差异见 §七。
>
> 三服务 + 网关：gateway **:3080**（统一入口）→ scheduler **:3100**（匹配/计划/排队）→ executor **:3200**（DAG 执行/融合落盘）；旁挂 registry **:3400**（专家登记/心跳聚合）。服务间为 **HTTP 短调用**，非 gRPC（gRPC :50051 联盟未使用）。

> **【2026-10-01 收官全链路更新】** 本轮把多轮新增的真实能力并入本图，**不改动既有主流程结论、不新增运行时逻辑**：
> 图谱数据维护面 **N4**（节点/边增量 CRUD，RBAC `graph.mutate`）、检索护城河 **T2**（`POST /api/expert-graph/rag/expand` 邻域多跳扩展）、
> 组队可解释 **U2**（`ModularWeightMatcher` 五维加权 + 逐维 `scores` 透明透出）、画布可视化 **U1**（前端拖拽/编辑/邻域展开）、
> 外部消费出口 **T3**（独立 stdio MCP 服务器，`expert_search`/`optimal_team`/`graph_expand` 三工具）。
> 嵌入位置见 §1.3 总图、§2.3（U2）、§4.3（N4 已述 + T2 新增）、§4.6（U1）、§4.7（T3）；原文一律保留，新增段落以「2026-10-01 新增」标注。

---

## 一、流程总览

### 1.1 端到端叙事（远程形态，生产）

用户在前端提交一次"多专家协作任务"，请求先到网关 :3080。网关在 `RemoteAllianceClient` 判断形态：
配了 `MOX_ALLIANCE_SCHEDULER_URL` 且 `MOX_ALLIANCE_REMOTE_MODE=auto` 时走远程链路，否则走进程内本地预览。

远程链路下：网关把任务创建转发给 scheduler :3100；scheduler 先做中文分词与领域推断，再经
模块化权重匹配器选出 Top N 专家（健康权重可配置，默认0.05，不健康得分0.2；另有租户/Active/领域等过滤）；随后 SimplePlanGenerator
按 7 种协作模式之一生成 DAG 计划。scheduler 通过 ExecutorBridge 把任务与计划桥接到 executor :3200，
DAG 引擎按拓扑逐节点派发（就绪节点并行、失败节点按 3 次指数退避重试）。全部节点成功后，引擎在
DAG 尾部按融合策略产出 FusionOutput；任一节点终态失败则任务整体置 failed。结果与节点状态由网关直连
executor :3200 读回（调度器 :3100 不做执行态读代理），实时日志走 SSE。整条链路的写路径在
scheduler/executor，读路径（节点/融合/DAG 形状）由网关直连 executor。

### 1.2 文本流程图（远程形态）

```
用户/前端
   │  POST /api/alliance/tasks  {title,description,task_type,priority,mode,fusion_strategy}
   │  Header: X-Tenant-Id / X-User-Id / x-request-id / Authorization
   ▼
┌──────────────────────── gateway :3080 ────────────────────────┐
│  JWT 鉴权 → RemoteAllianceClient(alliance_remote.rs)          │
│   • REMOTE_MODE=off 或未配 URL → 本地预览(进程内,重启即失)      │
│   • REMOTE_MODE=auto + 配 URL  → 远程(下方链路)                │
└───────────────┬──────────────────────────────┬───────────────┘
                │ Authorization: Bearer $MOX_INTERNAL_TOKEN
                ▼                                ▲  读路径(节点/融合/DAG)
┌──────────────── scheduler :3100 ───────────────┤
│ POST /tasks            任务创建 → TaskStatus=pending/planning│
│ POST /experts/search   模块化权重匹配 Top N                  │
│ planner.rs             SimplePlanGenerator → DAG(7 模式)      │
│ ExecutorBridge ────────写路径─────────────────► executor:3200│
│ POST /tasks/:id        pause/resume/cancel/complete          │
└──────────────────────────────────────────────────────────────┘
                                                 │
                                                 ▼
                          ┌────── executor :3200 ──────┐
                          │ DagEngine: 拓扑调度/并行执行 │
                          │ expert_executor: 超时60s/重试3│
                          │ fusion: DAG尾部融合→FusionOut│
                          │ state_sink: SQLite(WAL)落盘 │
                          └─────────────────────────────┘
                                                 │
   实时日志: GET /api/alliance/tasks/:id/logs/stream (SSE, 非 WebSocket)
   结果读回: 网关 → executor GET /tasks/:id/result、/tasks/:id/status、/tasks/:id/nodes

旁挂: registry :3400 专家登记 + 10:1:1 心跳聚合(主动探活默认关)
```

### 1.3 全链路总图（2026-10-01 收官更新）

> 实线 = 既有自动化主链（§1.2）；虚线框 = 本轮并入的新增能力面，区分「已接线进主链」与「独立读/写面，供人/MCP 消费，未强耦合进 scheduler 匹配」。

```
 ┌─ 业务发起 ─────────────────────────────────────────────────────────────┐
 │  建任务 POST /api/alliance/tasks（gateway:3080 → scheduler:3100）          │
 │  选专家 POST /api/alliance/experts/search（U2：五维加权 + scores 透出）   │
 └───────┬───────────────────────────────────────────────────▲────────────┘
         │                                                   │ 读面
         ▼                                                   │
 ┌─ 图谱检索 T2（护城河，2026-10-01 新增）───────────────────┼────────────┐
 │  POST /api/expert-graph/rag/expand（gateway:3080，读面）   │            │
 │  seeds→BFS 多跳邻域扩展(max_depth≤4)→按路径边权乘积排序    │            │
 │  ⚠ 现状为独立读面：供 U1 画布/MCP/planner 候选契约消费，    │            │
 │    尚未强耦合进 scheduler 的 ModularWeightMatcher 主匹配链 │            │
 └───────┬───────────────────────────────────────────────────┘            │
         │ 候选专家(node_type=expert)契约                                  │
         ▼                                                                 │
 ┌─ 最优组队 U2 ───────────────────────────────────────────────────────────┤
 │  主路径 ModularWeightMatcher 五维加权(domain.35/cap.30/rating.20/        │
 │  perf.10/health.05)；输出 ExpertSummary 增 match_score/match_reason/    │
 │  scores{每维 value,weight}。另网关图上贪心集合覆盖                      │
 │  POST /api/expert-graph/optimal-team → team_members/coverage/team_score │
 └───────┬─────────────────────────────────────────────────────────────────┘
         ▼
 ┌─ DAG 编排（§2.4 SimplePlanGenerator 七模式）→ executor:3200 执行         │
 │   → 融合 FusionOutput → 结果交付（§2.7 读 executor:3200 / SSE）          │
 └───────▲─────────────────────────────────────────────────────────────────┘
         │ 写回图谱（人工/增量）
 ┌───────┴─────────────────────────────────────────────────────────────────┐
 │ 画布可视化 U1（前端 :3020，2026-10-01 新增）：拖拽/节点边编辑/邻域展开    │
 │  · 拖拽坐标仅活在前端 store 视觉层，不落后端                              │
 │  · 编辑/删除/连线 → N4 增量 CRUD（RBAC graph.mutate，按钮级 v-role-any） │
 │  · 「展开邻域」读面 → 调 T2 rag/expand，幂等并入画布                     │
 └─────────────────────────────────────────────────────────────────────────┘

 ┌─ MCP 外部消费 T3（2026-10-01 新增）─────────────────────────────────────┐
 │ 独立 binary mox-alliance-mcp-server（stdio JSON-RPC 2.0，不占端口）      │
 │ MOX_MCP_GATEWAY_URL 默认 http://127.0.0.1:3080，MOX_INTERNAL_TOKEN 直通  │
 │ 三工具薄适配网关读面：expert_search / optimal_team / graph_expand         │
 └─────────────────────────────────────────────────────────────────────────┘
```

> **嵌入纪律**：T2/U1/T3 均为**读面或独立面**，不改变 §2.5 DAG 执行、§2.6 融合、§六异常表的既有语义；
> N4 写面已在 §4.3 落地。远程传输失败仍一律 503 保持数据源（§六），不因新增 MCP/读面而切本地兜底。

---

## 二、主流程逐步详解

> 每步固定六栏：输入 / 输出 / 涉及服务与端口 / 状态迁移 / 异常处理 / 本地 vs 远程差异。
> 状态枚举引自 08 §八：TaskStatus 7 态（pending/planning/running/paused/completed/failed/cancelled）、
> NodeStatus（ready/running/success/failed/cancelled；代码存储另见 pending/skipped，见 §五注）。

### 2.1 任务创建

- **输入**：`POST /api/alliance/tasks`，请求体关键字段 `title`、`description`、`task_type`、
  `priority`、`mode`（协作模式）、`fusion_strategy`；请求头 `X-Tenant-Id`/`X-User-Id`/`x-request-id`。
  前置状态：无（任意已认证用户）。
- **输出**：`task_id`（UUID）、`title`、初始 `status`、`created_at`。远程形态对应 scheduler
  `POST /tasks`（routes.rs:26,170），scheduler 把请求头租户/用户贯通进 `TaskSubmitRequest`。
- **服务与端口**：gateway :3080 →（远程）scheduler :3100。
- **状态迁移**：任务落库初始为 `pending`，进入规划后转 `planning`（两态均在 7 态字典内）。
- **异常**：参数非法 → `InvalidArgument` 400；租户不符 → `TenantMismatch` 403；
  远程传输失败（连接拒绝/超时，10s 上限）→ **503 且不切本地数据源**（alliance_remote.rs `transport_fallback`）。
- **本地 vs 远程**：`REMOTE_MODE=off` 时写入网关 SDK 内嵌 `InMemoryTaskRepository`（CURRENT §4.1），
  **进程内、重启即失**；远程形态才落 scheduler 任务库。

### 2.2 意图识别

- **输入**：任务 `description` 文本。
- **输出**：领域候选集合 +（Dynamic 模式下）推荐协作模式与决策理由。
- **服务与端口**：scheduler :3100 内 `scheduler-core/matching.rs`——`tokenize` 中文分词 →
  `infer_domains` 领域推断；`planner.rs decide_dynamic_mode` 做关键词意图识别（含"迭代/优化/refine"→Iterative、
  "评审/审核/review"→Sequential 等，见 03 §3.3，属 planner 内部）。
- **状态迁移**：不改变任务状态（仍 pending/planning）。
- **异常**：无独立意图服务；分词为空时走"0 匹配兜底"（planner 既有设计），不报错。
- **本地 vs 远程**：纯算法，两形态一致。
- **注意**：仓库另有 `ai-intent-svc`（:8765），与联盟任务链路**无关**，不得画进本流程图。

### 2.3 专家匹配

- **输入**：`POST /api/alliance/experts/search`，字段 `query`（任务描述）、`domains`、`limit`；
  空租户查询时回退 `"system"`（scheduler 内置领域专家租户，routes.rs:301）。
- **输出**：`experts[]`（expert_id/name/description/domains/status）+ `total`；
  生产主路径 `ModularWeightMatcher`，`matcher.rs` 为 fallback。
- **服务与端口**：gateway :3080 →（远程）scheduler :3100 `POST /experts/search`（routes.rs:32,294）。
- **加权与过滤口径（2026-10-01 复核）**：主 ModularWeightMatcher 使用 `is_healthy ? 1.0 : 0.2`，健康权重来自每专家配置，未覆盖时默认0.05（modular_matcher.rs:170–171/197–207；common-proto/types.rs:1213–1217）。备用matcher才是0.3/0.15。主路径还过滤租户、非Active、低优先级、不匹配领域及总分低于0.2的候选；健康不是独立硬过滤，但改变健康分可能影响总分门槛。
- **异常**：匹配失败记录指标 `record_match(...,false)` 并透传错误体；远程不可用 → 503 保持数据源。
- **本地 vs 远程**：本地形态匹配在网关进程内完成（RuleBasedExpertMatcher）；远程形态打到 scheduler:3100。
- **输出可解释（2026-10-01 新增 U2）**：`ModularWeightMatcher` 本就为每候选算 `MatchScoreBreakdown`，此前 HTTP 边界压成瘦 `ExpertSummary` 只回总分；本轮把逐维明细透出：
  - 响应 `experts[]` 每项增可选 `match_score`（总分）、`match_reason`（决策理由）、`scores`——
    `scores.{domain,capability,health,priority,performance}` 各为 `{value, weight}`，外加 `total`。
  - **权重真相**（以代码为准）：domain 0.35 / capability 0.30 / priority(rating) 0.20 / performance 0.10 / **health 0.05**；
    health 分 = `is_healthy ? 1.0 : 0.2`（非硬过滤）。前端「为什么匹配」面板（`MatchExplainPanel`）逐维条形图 + 权重标注。
  - 透传路径：scheduler-svc `/experts/search` 组装 → http-sdk `alliance_remote.rs`（远程）透传 / `alliance.rs`（本地降级）按 `MatchingWeights::default()` 组装；新字段均 `#[serde(default)]`，旧调用方不受影响。
  - **边界**：本轮只透出、不改算法/权重值；`optimal-team`（网关图贪心集合覆盖）公式独立，其输出 `team_members/coverage/team_score(weighted_set_cover_greedy)` 不随 `/experts/search` 的 scores 面板展示。

### 2.4 计划生成（DAG）

- **输入**：匹配到的专家列表 + 用户指定/默认 `mode` + `fusion_strategy`。
- **输出**：`CollaborationPlan`（节点 nodes、依赖 edges、fusion_strategy、expert_weights、
  dynamic_routes）。实现为 scheduler-core `SimplePlanGenerator`（planner.rs:81-92 七臂 match 全覆盖）。
- **服务与端口**：scheduler :3100 规划期内完成（不跨服务）。
- **协作模式 AllianceMode（7 种，planner.rs:81-91）**：
  Sequential / Parallel / Voting / Hierarchical / Debate / Iterative / Dynamic。
- **融合策略 FusionStrategy（6 种）**：confidence_weighted / debate / iterative / map_reduce /
  stacking / weighted_vote。
- **状态迁移**：`planning` →（计划交付 executor 启动）`running`。
- **异常**：计划校验失败 `plan.validate()` → `InvalidPlan` 错误（dag_engine.rs:793）。
- **本地 vs 远程**：本地形态计划生成在网关进程内（CURRENT §4.1 第 4 步）；远程形态在 scheduler:3100，
  且计划随任务一并落库（`persist_plan`，dag_engine.rs:324），供崩溃恢复重建 DAG。

### 2.5 执行（DAG 引擎）

- **输入**：`CollaborationPlan` + 任务 + `ExecutionOptions`（max_retries/node_timeout_ms/fail_fast）。
- **输出**：逐节点状态与 `outputs`；任务进度 `progress`（终态节点数/总节点数）。
- **服务与端口**：scheduler ExecutorBridge（HttpExecutorBridge，写路径）→ executor :3200
  `start_execution`；读路径由网关直连 executor（/tasks/:id/status、/nodes、/dag）。
- **调度机制**（dag_engine.rs）：启动命令把任务置 `Running`、节点全部 `Pending`；调度循环按
  `find_ready_nodes`（入度=0）取就绪节点，置 `Running` 后 `tokio::spawn` **并行**派发（无信号量上限，
  靠 tokio 自然调度，08 缺口 N8）；节点成功 → `Completed`，失败 → `Failed`。
  Dynamic 模式：决策节点完成后按 `dynamic_routes`（eq/neq/gt/gte/lt/lte）选真/假分支，未选分支置 `Skipped`。
- **节点重试**：`MOX_EXECUTOR_MAX_RETRIES=3`、单节点超时 `MOX_EXECUTOR_NODE_TIMEOUT_MS=60000`、
  指数退避（初始 1000ms、因子 2.0、上限 30000ms）。重试耗尽 → 节点 `Failed`。
- **状态迁移**：TaskStatus `running`；NodeStatus `pending/ready → running → completed|failed|skipped|cancelled`。
- **异常**：任一节点终态失败且全部节点终态 → 任务 `failed`（见 2.6）；`SkipNode` 命令可把未终态节点
  主动置 `Skipped`；租户不符 → `TenantMismatch` 403；任务不存在 → `TaskNotFound` 404。
- **本地 vs 远程**：本地形态节点推进与融合在网关同一进程；远程形态在 executor:3200，且
  sqlite 模式下任务/计划/节点增量落盘（dag_engine.rs 场景②③出口）。

### 2.6 结果融合

- **输入**：全部成功节点的 `outputs` + `plan.expert_weights` + `plan.fusion_strategy` + 任务描述。
- **输出**：`FusionOutput`（content、confidence、expert_count、strategy、contributions、summary、
  participating_nodes）。引擎为 executor-core `FusionEngine`（mox-alliance-core/fusion/strategies，6 策略）。
- **触发时机**：仅当**所有节点到达终态且无失败**时，由 `check_task_completion` 在 DAG 尾部调用
  `run_fusion`（dag_engine.rs:694,721）。有任何 Failed 节点则任务直接 `failed`，**不融合**。
- **服务与端口**：executor :3200 内部；融合输出以保留行 `__fusion_output__` 持久化（sqlite 模式）。
- **状态迁移**：融合成功 → TaskStatus `completed`、`progress=1.0`、写 `completed_at`/`duration_ms`。
- **异常**：融合计算失败仅 `warn!`（dag_engine.rs:751），任务仍 `completed`；
  已取消任务**不得融合**（终态优先，晚到节点完成不覆盖 Cancelled，dag_engine.rs:675）。
- **本地 vs 远程**：逻辑相同；sqlite 模式重启后 `/result`、`/fusion-result` 仍可读回融合结论
  （`read_fusion_output`），file/memory 模式无此能力。

### 2.7 交付

- **输入**：用户查询任务结果/进度/日志。
- **输出**：
  - 任务详情/状态：网关聚合 scheduler `/tasks/:id` 与 executor `/tasks/:id/status`（`effective_task_status`：
    scheduler 终态 paused/cancelled/failed/completed 优先，否则回退执行器态）。
  - 节点/DAG：网关 → executor `/tasks/:id/nodes`，DAG 形状由依赖推导边、位置按序生成。
  - 融合结果：网关 → executor `GET /tasks/:id/result`；**404（尚无融合产出）归一化为 pending 形状**
    而非报错（alliance_remote.rs:964）。
  - 实时日志：SSE `GET /api/alliance/tasks/:id/logs/stream`（**无 WebSocket**，全 crate `WebSocketUpgrade` 零命中）。
- **服务与端口**：网关 :3080 对外；读 executor:3200、查 scheduler:3100。
- **状态迁移**：交付发生在 `completed`/`failed`/`cancelled` 终态或 `running` 轮询中。
- **异常**：任务/节点不存在 → 404；远程传输失败 → 503 保持数据源，**不伪造本地日志**
  （`remote_task_logs` 明确"report real snapshots instead of manufacturing local task logs"）。
- **本地 vs 远程**：本地形态读的是网关进程内状态（重启即失）；远程形态读 executor:3200 实时快照，
  内存 miss 时回退存储层 `read_back`（多实例/重启后可用）。

### 2.8 反馈闭环

> **诚实边界**：03 §1 第 6 步写"归档案例 + 更新图谱边权重 + 会话记忆"。按 CURRENT V1.1 逐条核对——
> 案例库（baseline `ea_case`）是**目标态、现状无表**；图谱虽已有节点/边增量 CRUD 端点（2026-09-30，§4.3），
> 但反馈闭环**不会自动**更新图谱边权，边权维护仍靠管理员手动 rebuild 或增量端点。
> 现状真正落地的自动反馈闭环只有以下三条，不得虚构"自动更新图谱边权重"或"案例沉淀"。

- **审计闭环**：每次写操作（专家 CRUD/会话/任务动作/分发）经 `emit_audit` 落 NDJSON 哈希链（见 §4.5）。
- **会话闭环**：多轮对话上下文随会话沉淀到网关 SQLite `sessions`/`session_messages`（见 §4.2）。
- **登记与收藏闭环**：专家 `availability.status`（online/busy/offline/away）登记、收藏 favorites（进程内 HashSet，
  重启即失）、预约 bookings（SQLite 落盘）。图谱关系既可手动 `POST /api/expert-graph/rebuild` 全量重建，
  也可经 2026-09-30 新增的节点/边增量 CRUD 端点（RBAC `graph.mutate`）单条维护——但专家 CRUD 仍不自动回调图谱边权。

---

## 三、本地形态 vs 远程形态差异

由 `MOX_ALLIANCE_REMOTE_MODE`（默认 `auto`）与是否配置 `MOX_ALLIANCE_SCHEDULER_URL`/`..._EXECUTOR_URL` 决定
（alliance_remote.rs:100-109）：`off` 或未配 URL → 全进程内；`auto` 且配 URL → 跨服务。

| 维度 | 本地形态（REMOTE_MODE=off / 未配 URL） | 远程形态（auto + URL） |
|------|----------------------------------------|------------------------|
| 链路 | 网关 :3080 进程内一条龙（匹配/计划/执行/融合同进程） | gateway:3080 → scheduler:3100 → executor:3200 |
| 任务真源 | SDK 内嵌 `InMemoryTaskRepository`，**重启即失** | scheduler/executor 任务库（STORAGE_MODE 决定） |
| 持久化 | 进程内结构；/dag、/logs/stream 读同一份内存态 | `MOX_ALLIANCE_STORAGE_MODE`：memory=纯内存 / file=JSON 快照无恢复 / **sqlite=WAL 完整持久化+重启恢复** |
| 重启恢复 | 无（未完成任务丢失） | sqlite 模式：未完成任务重注入、running 节点标记 interrupted→pending、已完成节点跳过不重跑、融合结论读回 |
| 执行就绪探针 | `runtime_readiness` 报告 `execution_ready=false`、`mode=local_preview`，**任务工作台据此禁止执行** | 同时探 scheduler/executor `/health`，executor 还需 `execution_ready=true`（或 mock） |
| 传输失败行为 | 不涉及远程 | 连接失败/超时（10s）→ **503，保持远程数据源，绝不切本地兜底**（防重复任务/脏写） |
| 鉴权 | 本地 dev_mode 直通 | 出站自动带 `Authorization: Bearer $MOX_INTERNAL_TOKEN`，下游 internal_auth_layer 校验 |
| 生产建议 | 仅单机联调/CI | 生产必 `auto` + `STORAGE_MODE=sqlite`（见 09 §2.7） |

> **关键纪律**：界面/运维判断必须先看 `MOX_ALLIANCE_REMOTE_MODE`。"重启后任务全没了"在本地/file 形态是
> 预期行为，不是后端丢数据；生产必须 sqlite。远程不可用时返回 503 是**有意保持数据源**，不要在文档或前端
> 描述成"已自动切换本地执行"。

---

## 四、附加流程

### 4.1 专家入驻与可用性维护

- **注册**：网关 `/api/experts[/:id]` CRUD → 网关 SQLite 表 `experts`（experts_db.rs:91）；
  启动 `load_registry`、变更 `save_registry`。注册表为空时 `seed_builtin_experts()` 播种 10 位具名专家
  exp-*-001（CURRENT §7.2）。
- **状态登记 ≠ 探活**：`availability.status` 取值 `online/busy/offline/away`，是**注册时写入的登记值**，
  不是健康探测结果。前端不得表述为"在线检测通过"。
- **收藏 favorites**：`experts_common.rs` 进程内 `HashSet`，**无表、重启即失**（前端已标"仅本次会话"）。
- **预约 booking**：表 `bookings`（experts_db.rs:158，experts_ext.rs），SQLite 落盘。
- **注册中心（独立服务）**：registry-svc **:3400** + registry-proto 契约层；分级心跳聚合
  node→rack→cell **10:1:1**（`POST /api/registry/aggregated-heartbeat`）。
  **主动探活默认关闭**（`health_probe_enabled:false`），生产设 `MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=1`。
  网关 `registry_client.rs` 的 `health()` 是 `GET {registry}/health`，探的是**注册中心进程**而非某个专家。

### 4.2 专家咨询/协作会话

- 表 `sessions`（experts_db.rs:108）+ `session_messages`（:124），网关 SQLite 落盘（load/save_sessions）。
- 会话创建 → 多轮上下文追加 → 归档均走网关 `experts_session.rs`（959 行）。
- **边界**：会话是**单进程内恢复**（08 缺口 N11），网关未做多活，网关扩容需 sticky session。

### 4.3 图谱构建与查询

- **存储定位**：联盟图谱落在网关**关系层 SQLite**（experts.db 的 `graph_nodes`/`graph_edges`/`graph_meta`，
  experts_db.rs:136/143/152），无独立图数据库；对应全局数据库架构的"关系层（SQLite WAL）+ 图邻接表"分层，
  不是专用图引擎。
- **查询**：`/api/expert-graph/*` 共 8 条**只读**路由（列表/详情/optimal-team 等）。
- **构建**：`POST /rebuild` **全量重建**；此外 2026-09-30 起新增 **6 个增量写端点**
  POST/PUT/DELETE `/nodes[/:id]`、`/edges[/:seq]`（RBAC `graph.mutate` 强制 super_admin/tenant_admin，
  未认证 401 / 非管理 403），可对单节点/单边增删改，删节点联动删其全部关联边，增量写后 `version+=1`。
- **存储**：增量写经 experts_db.rs 的 `upsert_graph_node`/`delete_graph_node_cascade`/`upsert_graph_edge`/
  `replace_graph_edges` 落 SQLite（不动全量 `save_graph_conn`）；`seq` 即内存 edges Vec 下标，删/增边后按序重排。
- **与 rebuild 的关系**：rebuild 全量重算会覆盖增量结果，属预期语义。专家 CRUD 仍不会自动增量维护图谱边权
  （增量 CRUD 是管理员手动维护面，非自动回调）。
- **邻域检索 T2（2026-10-01 新增，读面）**：`POST /api/expert-graph/rag/expand`（gateway:3080，与 get_graph 同读面，登录即可、无需角色）。
  - **输入**：`{ seeds: [节点id], max_depth?: 1..4(默认2), top_k?: >0(默认20), node_types?: [String], min_weight?: 0..1(默认0) }`。
  - **处理**：从每个 seed 在**内存态图** `state.graph`（与 N4 SQLite 双向同步）BFS 沿无向边多跳扩展；
    召回排序 = 路径边权重**乘积**（w∈[0,1]，随深度自然衰减），同节点多路径取最优（乘积大者、同乘积取更浅深度）；单路径内防回环。
  - **输出**：`{ query, results: [{ node{id,label,node_type}, depth, aggregate_weight, path, first_hops }], stats:{searched_nodes,returned,elapsed_ms}, rerank:"graph_only（向量融合待 #27）" }`。
  - **异常**：seeds 空 / max_depth 越界 / top_k=0 / min_weight 越界 → 400；任一种子节点不存在 → 404；空图 → 空 results（非错误）。
  - **定位（护城河）**：结果中 `node_type=="expert"` 即候选专家排序，可直接供 `optimal-team`/planner/U1 画布/MCP 消费；
    **现状未强耦合进 scheduler `ModularWeightMatcher` 主匹配链**，`hybrid_rerank` 当前透传（pgvector 融合留待 #27），不冒充已接线。
  - **路由计数更新**：本图路由现 = 8 只读(含 GET 系) + `POST /optimal-team` + `POST /rebuild` + N4 6 写端点 + T2 1 读端点；
    §4.3 原述「共 8 条只读路由」为 T2 落地前口径，以本节实测路由清单为准。

### 4.4 多活与故障转移（HA）

- **开关**：`MOX_ALLIANCE_HA_MODE=on`（默认 off＝单副本即 leader）。硬约束：开 HA 必须
  `STORAGE_MODE=sqlite`（file/memory 下启动即失败，server.rs:108-109）。
- **选主**：scheduler-core `leadership.rs` 租约选主（`LeaseStore`/`LeaderElector`/`SqliteLeaseStore`），
  租约表 `alliance_leader_lease` 与任务表**同库**（SQLite）。`MOX_ALLIANCE_HA_LEASE_MS=10000`，
  `MOX_ALLIANCE_HA_STALL_MS=300000`（孤儿任务静默窗口）。
- **fencing**：租约带 epoch 任期，旧 leader 续租失败后其写操作被任期拒绝。
- **职责切分**：请求路径（建任务/查任务）无状态可任意 LB；**扫全表并逐条求证执行器的对账/孤儿接管是唯一单点职责**，
  用租约限定在 leader，且写成幂等（租约只保证互斥，不保证 exactly-once）。
- **观测**：`GET /leadership`——HA 未开时如实返回 `ha_enabled=false, is_leader=true`；开后返回 holder/是否 leader/
  租约任期与到期时刻。

### 4.5 审计与合规

- **落盘**：NDJSON 文件 `MOX_AUDIT_LOG_PATH`（默认 `data/audit/experts-audit.ndjson`，experts_common.rs:563）。
- **防篡改**：SHA-256 **哈希链**（逐行链上前一行哈希，experts_common.rs:521-574）+ **HMAC 签名**
  （密钥 `MOX_AUDIT_HMAC_SECRET`，默认 `mox-experts-alliance-audit`，**生产必改**）。
- **缺口**：审计事件 Actor 当前硬编码 `AuditActor::system()`（08 缺口 N2），哈希链防篡改但暂不能追溯"谁干的"；
  下游三 svc（:3100/:3200/:3400）无 JWT 中间件（N1），靠网络隔离 + `MOX_INTERNAL_TOKEN` 兜底。
  > **【2026-10-01 收官复核】** 上两行属 N1/N2 修复前口径，已被后端修复报告取代：下游三 svc 现已挂 `internal_auth_layer`（`MOX_INTERNAL_TOKEN`/双值 `..._ALT` 校验，未配置则放行）；
  > 审计 Actor 已从 `system()` 改为经 `OptionalAuthUser` 取真实 user_id+roles（未认证降级 system）。详见 §七第 9–10 行。

### 4.6 画布可视化 U1（2026-10-01 新增，前端编排可视化面）

- **定位**：前端 `AllianceGraphView` + `GraphCanvas`（手写确定性 SVG，复用既有组件，不引 VueFlow/LogicFlow/echarts force，遵守 G9 三栈归一）；
  dev 端口 vite **:3020**，数据面经网关 **:3080**。是**图谱的人读/人写可视化面**，不在 §2.5 DAG 自动化执行链上。
- **输入 / 处理 / 输出**：
  - **拖拽移动**：原生 pointer events，client→SVG 坐标换算；**坐标只活在前端 store `dragPositions` 视觉覆盖层，不落后端、不入图谱数据模型**，`loadGraph` 后即弃（刷新复位，有意为之）。
  - **节点/边编辑（写面）**：Inspector「编辑/删除节点」、视图「新增节点」对话框、编辑模式下点源→点目标节点选 edge_type 连线 → 调 N4 增量 CRUD（§4.3），删节点级联删边由后端做。
  - **邻域展开（读面）**：Inspector「展开邻域」以选中节点为 seed、maxDepth=2 调 T2 `rag/expand`（§4.3），结果**幂等并入**画布（按 id 与 source|target|edgeType 去重），原节点不动，**不触发 loadGraph/rebuild**。
- **服务端口**：前端 :3020 → 网关 :3080（读面 `/api/expert-graph/*`、写面 N4 六端点）；不直连 scheduler/executor。
- **状态 / 异常**：复用 N4 错误语义——重复 id 409、node_type/weight 非法 400、节点/seq 不存在 404；邻域展开 404（seed 不存在）透传。
- **权限**：管理写面按钮用按钮级指令 `v-role-any="['super_admin','tenant_admin']"`（无角色即从 DOM 移除），与后端 RBAC `graph.mutate`（未认证 401 / 非管理 403）前后端双重守卫；邻域展开/查看详情为读面，登录即可。
- **未做（诚实标注）**：DAG 导出 planner 可执行 JSON + 预演校验、逐节点实时回显 T5、minimap、虚拟滚动、拖拽坐标持久化。

### 4.7 MCP 外部消费出口 T3（2026-10-01 新增）

- **定位**：独立 binary `mox-alliance-mcp-server`（crate `platform/domains/alliance/mcp/`），以 **stdio JSON-RPC 2.0**（`Content-Length` 帧）对外部 MCP Client 暴露联盟读面；**不监听 HTTP 端口**，独立进程、不并入 axum 网关。
- **配置**：`MOX_MCP_GATEWAY_URL`（默认 `http://127.0.0.1:3080`）；`MOX_INTERNAL_TOKEN` 可选，设置后出站注入 `Authorization: Bearer <token>`。
- **输入 / 处理 / 输出（三工具，薄适配网关读面，零本地 mock）**：

  | MCP 工具 | 上游真实端点（gateway:3080） | 输入形状 | 输出 |
  |---|---|---|---|
  | `expert_search` | `POST /api/alliance/experts/search` | `{query, domains[], limit}` | 专家列表 + U2 逐维 `scores` + `match_score` |
  | `optimal_team` | `POST /api/expert-graph/optimal-team` | `{required_skills[], required_domains[], max_members, min_rating, goal}` | `team_members` + `coverage{coverage_ratio}` + `team_score`（weighted_set_cover_greedy） |
  | `graph_expand` | `POST /api/expert-graph/rag/expand` | `{seeds[], max_depth, top_k, node_types[], min_weight}` | 邻域节点 + `aggregate_weight` + `path` + `first_hops` |

- **状态 / 异常**：纯转发网关读面响应；网关 4xx/5xx（含远程 503 保持数据源）原样透传为 MCP 错误，不在 stdio 层伪造数据。
- **权限**：MCP 自身无 OAuth/企业 IdP（留待），当前以 `MOX_INTERNAL_TOKEN` 静态 Bearer 直通网关读面；外部 Client 接入与凭证托管未做，如实标注。

---

## 五、全链路状态机总图（TaskStatus × NodeStatus）

> TaskStatus 7 态与 NodeStatus 取值引自 08 §八统一字典。
> **口径注**：dag_engine.rs 实际写入存储的节点字符串为 pending/running/completed/failed/skipped/cancelled
> （`node_status_str`），其中 completed 即字典"success/已完成"、ready 在网关归一化为 pending；本文以 08 §八
> 展示名为准，括号给代码别名。

### 5.1 TaskStatus 迁移（7 态）

```
                 提交任务
                    │
                    ▼
              ┌── pending ──┐
              │             │ 进入规划
              ▼             │
          planning ─────────┘
              │ 计划交付 executor 启动
              ▼
          running ──────pause────► paused
              │  ▲                    │
              │  └────────resume──────┘（仅 paused 可 resume，否则 409）
              │
   ┌──────────┼───────────────────┐
   │ 全部节点成功(DAG尾部融合)      │ 任一节点终态失败  │ 用户/系统 cancel
   ▼                              ▼                 ▼
 completed                    failed            cancelled
   └──── 三者均为终态，不可离开：晚到节点完成不覆盖终态 ────┘
```

| 迁移 | 触发 | 代码位置 |
|------|------|---------|
| → pending | 创建任务 | scheduler submit_task |
| pending → planning | 进入计划生成 | scheduler-core/planner |
| planning → running | ExecutorBridge 启动执行 | dag_engine.rs:299（Start 置 Running） |
| running → paused | POST action=pause | dag_engine.rs:398 |
| paused → running | POST action=resume（校验必须 paused） | dag_engine.rs:406,846 |
| running → completed | 全部节点成功 + run_fusion 完成 | dag_engine.rs:696 |
| running → failed | 全部终态且任一节点 Failed | dag_engine.rs:690 |
| running → cancelled | POST action=cancel | dag_engine.rs:414 |

### 5.2 NodeStatus 迁移

```
pending(ready) ──入度=0被调度──► running ──成功──► completed(success)
     │                             │
     │                             └──失败且重试耗尽──► failed
     │
     ├── Dynamic 未选分支 / SkipNode 命令 ──► skipped
     └── 任务 cancel 时所有未终态节点 ──► cancelled
```

| 迁移 | 触发 |
|------|------|
| pending/ready → running | 调度循环选中就绪节点（dag_engine.rs:486） |
| running → completed | 节点执行 success（重试内成功） |
| running → failed | 重试耗尽仍失败 / execute_node 返回 Err |
| * → skipped | SkipNode 命令 / Dynamic 未选分支（dag_engine.rs:431,660） |
| * → cancelled | 任务取消，所有未终态节点一并置 cancelled（dag_engine.rs:418） |

---

## 六、异常处理总表

| 异常 | 触发条件 | 系统行为 | 用户可见反馈 | 文档/代码依据 |
|------|---------|---------|--------------|----------------|
| 远程传输失败 | scheduler/executor 连接拒绝/超时（>10s） | 返回 503，**保持远程数据源，不切本地、不建重复任务** | 503「任务服务暂时不可用；未切换数据源」 | alliance_remote.rs transport_fallback:423 |
| 远程业务错误 | 远程返回 4xx/5xx 错误体 | 归一化为网关错误响应，不降级 | 透传远程 message | alliance_remote.rs http_err:429 |
| 任务不存在 | 查/改未登记 task_id | `TaskNotFound`/`NotFound` | **404** | routes.rs:381；dag_engine.rs:910 |
| 节点不存在 | 查/跳未登记 node_id | `NodeNotFound` | **404** | routes.rs:383 |
| 专家不存在 | 匹配/路由命中空 | `ExpertNotFound` | 404（0 匹配走兜底） | routes.rs:382 |
| 租户不匹配 | 任务/节点属其他租户 | `TenantMismatch` | **403** | dag_engine.rs:818,894 |
| 非法参数 | 请求体校验失败 | `InvalidArgument` | 400 | routes.rs:378 |
| 终态再操作 / 状态非法 | 对已 completed 任务 toggle-done、对非 paused 任务 resume | `TaskAlreadyTerminal`/`InvalidTaskStatus` | **409** | routes.rs:384-385；alliance_remote.rs:591 |
| 调度器/执行器/专家不可用 | 容量满 / 桥接失败 / 专家不可用 | `SchedulerFull`/`ExecutorUnavailable`/`ExpertUnavailable` | **503** | routes.rs:386-388 |
| 节点执行失败 | 超时（60s）/报错 | 按 3 次指数退避（1s→2s→4s，上限 30s）重试；耗尽→节点 failed | 节点 ERROR 日志；全终态后任务 failed | expert_executor.rs；dag_engine.rs:530 |
| 取消任务 | POST action=cancel | 任务 cancelled，未终态节点全 cancelled，终态立即落库 | 任务"已取消"，晚到节点不翻盘 | dag_engine.rs:411,675 |
| 暂停/恢复 | pause/resume | running↔paused；resume 仅 paused 合法 | 已暂停/已恢复执行 | dag_engine.rs:395,846 |
| 融合尚无产出 | 任务未完成即查 /result | 执行器 404 → 归一化为 pending 形状（非报错） | fusion_status=pending「尚未产生可融合输出」 | alliance_remote.rs:964 |
| 未配远程但调执行 | REMOTE_MODE=off | readiness 报 execution_ready=false | 前端禁止执行，提示连接调度器/执行器 | alliance_remote.rs:235 |
| 下游未带内部令牌 | 直连 :3100/:3200 且配了 MOX_INTERNAL_TOKEN | internal_auth_layer 拒绝 | **401**（/health//metrics//leadership 白名单除外） | routes.rs:69-93 |

---

## 七、与 03-business-flow.md（V1.0）的差异说明

> 03 是 V1.0 目标态。下列结论已被 CURRENT V1.1 / 代码事实取代，引用 03 时须对照本表。

| # | 03 V1.0 写法 | CURRENT V1.1 / 代码事实 | 处置 |
|---|-------------|------------------------|------|
| 1 | §4「WebSocket/SSE 推送节点状态」 | 全 crate `WebSocketUpgrade` 零命中；实时性仅 SSE `/tasks/:id/logs/stream` | 以 SSE 为准，WebSocket 是幻影 |
| 2 | §2「状态过滤：仅 Active 专家」「5维评分健康15%」 | 主路径健康权重可配置（默认0.05），不健康得分0.2；另有Active等过滤；`availability.status`=online/busy/offline/away（登记值） | 写"加权"不写"过滤/排除" |
| 3 | §1 入口写作 POST /api/experts/tasks | 真实入口是 `POST /api/alliance/tasks`（actuator.rs:519） | 以 /api/alliance/tasks 为准 |
| 4 | §6 异常"专家不可用→找替代专家""参数错误→标记 Skipped 继续" | 代码无运行时替代专家切换；Skipped 来自 SkipNode 命令或 Dynamic 未选分支，非自动降级 | 不虚构自动换专家 |
| 5 | §1 第6步"归档案例 + 更新图谱边权重" | 案例库无表（目标态）；图谱已有节点/边增量 CRUD（2026-09-30），但专家 CRUD/反馈仍不自动回调边权 | 反馈闭环写审计/会话/登记/收藏；图谱边权靠管理员手动 rebuild 或增量 CRUD |
| 6 | §5 融合 6 策略名（MajorityVote/BestOf/Concatenation…） | 实际策略名为 confidence_weighted/debate/iterative/map_reduce/stacking/weighted_vote；网关展示名另有归一化映射 | 以 08 §八 FusionStrategy 为准 |
| 7 | 隐含"默认就跨三进程" | **默认形态是全本地进程内**（REMOTE_MODE=auto 但未配 URL 即本地），重启即失；远程才谈 sqlite 落盘恢复 | 先判 REMOTE_MODE 再下结论 |
| 8 | §7 注册后"心跳超时→Offline" | 注册中心主动探活**默认关闭**；网专家 online/busy 是登记值 | 探活语义见 §4.1，勿当在线检测 |
| 9 | §4.5「下游三 svc 无 JWT 中间件(N1)，靠网络隔离+令牌兜底」 | 三 svc 已挂 `internal_auth_layer`（`MOX_INTERNAL_TOKEN` 主+备双值校验，未配置放行），网关出站 `RemoteAllianceClient`/`RegistryClient` 构造点注入 Bearer | 鉴权链路端到端打通；未配令牌仍放行（开发默认） |
| 10 | §4.5「审计 Actor 硬编码 `AuditActor::system()`(N2)」 | 11 个写 handler 经 `OptionalAuthUser` 记录真实 user_id+roles，未认证降级 system | 拒绝/成功均审计，`rbac.denied` 记 Blocked |
| 11 | §4.3「图谱仅 8 条只读路由」 | 2026-09-30 起 N4 增 6 写端点 + 2026-10-01 T2 增 `rag/expand` 1 读端点；写面 RBAC `graph.mutate` | 路由清单以 §4.3 末行实测为准；rebuild 仍会覆盖增量结果 |

---

*本文为 CURRENT-ARCHITECTURE V1.1 事实下的端到端流程拆解。状态/端口/服务名均可对照
routes.rs、dag_engine.rs、alliance_remote.rs 与 PORT-REGISTRY V1.2 复核；与 03 V1.0 冲突处以本文 §七与 CURRENT V1.1 为准。*

*【2026-10-01 收官全链路更新】并入 N4 图谱增量 CRUD / T2 邻域检索 / U2 组队可解释 scores / U1 画布可视化 / T3 MCP 外部消费；
原文主流程结论保留，新增段落均以「2026-10-01 新增」标注，被取代旧结论见 §七第 9–11 行。本轮只整合核验，不新增功能、不改核心代码。*

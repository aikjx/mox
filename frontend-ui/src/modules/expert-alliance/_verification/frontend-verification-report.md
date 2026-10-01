# 专家联盟前端 · 代码事实核验报告

> 核验范围：`frontend-ui/src/modules/expert-alliance/**` + `src/views/expert/*` + `src/views/workspace/ExpertWorkspaceView.vue` + 路由注册。
> 核验方式：只读，未改动任何业务代码。后端权威口径取 `docs/API-REGISTRY.md`（由 `gateway/.../actuator.rs ROUTES` 生成）与 `docs/expert-alliance/CURRENT-ARCHITECTURE.md` §6.1。
> 每条结论均给 `文件:行号`；无法证实的标 ❓未核。
> 生成时间：2026-09-27。

---

## 0. 一句话结论

**模块自身（`modules/expert-alliance`）是一份契约纪律极强的实现**：`contract/endpoints.js` 里 68 个端点 key 全部能在后端 `API-REGISTRY.md` 找到同 registry id，无假端点；孤儿端点全部在 `UNMOUNTED_ROUTES` 里被显式定性为 rejected 并给了后端源码证据；三处权威文档点名的文案红线（在线检测通过 / 尚无编排记录 / 进程内空结果）在模块内**均未踩线**。

**真正的缺口全部在模块之外的 legacy 视图层**：`src/views/expert/*`、`src/views/workspace/ExpertWorkspaceView.vue` 绕过模块 `allianceApi`，另起一套 `src/api/*.js` 旧客户端，其中**仍在运行**地调用了模块契约明令禁止的 `/api/ai/engine/alliance/full`、`/ai/engine/alliance/capabilities`，并埋了一条后端注册表里根本不存在的 `POST /api/alliance/tasks/:id/qa`。

---

## 1. 前端能力全景表

### 1.1 Store（Pinia，6 个）

| 名称 | 路径 | 行数 | 依赖 API（方法名） | 状态管理（state/getter/action 摘要） | 测试 |
|---|---|---|---|---|---|
| allianceCollab | `store/alliance-collab.store.js` | 143 | `collaborate()`（内部路由到 route/consult/multi/debate/intelligent/algorithm 6 端点） | state: mode/input/controls/result/history(≤20)/loading.run/error.run；getter: current/textField/needsExperts/runnable/blocked/resultKind；action: setMode/toggleExpert/run/reset | ✅ `alliance-collab.store.test.js`(214) + `collab-panel.test.js`(351) |
| allianceConsole | `store/alliance-console.store.js` | 356 | getRuntime, listTasks, createTask, getTask, controlTask, toggleTaskDone, getNodes, getDag, getExecutionStatus, getFusion, getPlan, getLogs, getDispatcherConfig, updateDispatcherConfig, listExperts(status:'online'), runDispatch, dispatcherStatus, resetDispatcherLoad, resetAllDispatcherLoads | state: runtime/tasks/total/selectedId/detail{task,nodes,dag,fusion,execution,plan}/logs(≤500)/dispatcherConfig/configDraft/dispatchResult/dispatchCandidates/dispatcherStatus/resetReceipt；action: selectTask/createTask/controlTask/pushLog/replaceLogs/loadDispatcherConfig/saveDispatcherConfig/runDispatch/loadDispatcherStatus/resetExpertLoad/resetAllLoads/toggleTaskDone | ❌ 无独立 `.test.js` |
| allianceExperts | `store/alliance-experts.store.js` | 344 | listExperts, getExpertMetrics, expertsStats, listExpertCapabilities, searchExperts, listMyBookings, toggleFavorite, createBooking, cancelBooking, consultNow, consultRoom, joinTeam, registerExpert, updateExpert, deleteExpert | state: experts/total/page/pageSize=24/filters/favorites(Set,会话级)/bookings/stats/capabilities/expertMetrics/expertMatches/notice；action: loadExperts/loadBookings/loadStats/loadCapabilities/loadExpertMetrics(带 token 防抖)/searchExpertMatches(带 token)/run/toggleFavorite/createBooking/consultNow/registerExpert/saveExpert/removeExpert | ✅ `alliance-experts.store.test.js`(523) |
| allianceGraph | `store/alliance-graph.store.js` | 217 | graphOverview, graphStats, graphCommunities, graphNeighbors, graphCollaborators, graphPath, rebuildGraph, optimalTeam | state: graph/metrics/communities/selectedId/neighbors/collaborators/path/team/rebuildResult/pathDraft/teamDraft/collaboratorLimit；action: loadGraph/loadMetrics/loadCommunities/selectNode/changeCollaboratorLimit/findPath/rebuild/formTeam | ✅ `alliance-graph.store.test.js`(204) |
| allianceOrch | `store/alliance-orch.store.js` | 219 | orchestrate, generateOrchPlan, executeOrchPlan, getOrchStats, getOrchHistory | state: form/orchestration/plan/execution/stats/history{records,total,page,pageSize=20}/filters；getter: wire/validation/outcome/disclaimer/volatility/statCells/historyAnomaly/statusSplit；action: runOrchestrate/generatePlan/executePlan/loadStats/loadHistory | ❌ 无独立 `.test.js`（规则在 `contract/orchestration.test.js` 528 行覆盖） |
| allianceSessions | `store/alliance-sessions.store.js` | 391 | listSessions, sessionStats, getSession, createSession, appendSessionMessage, sessionSimilarSearch, semanticSearch, updateSession, archiveSession, exportSession, deleteSession | state: filters/list/stats/selectedId/detail/similar/semantic/createDraft/composer/editDraft/metaRows/notice；action: loadList/selectSession/appendMessage(并线)/saveEdit(差分)/archiveSession/deleteSession/exportSession/runSimilarSearch/runSemanticSearch/openSemanticResult | ✅ `alliance-sessions.store.test.js`(413) |

### 1.2 View（模块内 6 个 + 外部 5 个）

| 名称 | 路径 | 行数 | 装配的 store | 关键职责 | 测试 |
|---|---|---|---|---|---|
| AllianceConsoleView | `views/AllianceConsoleView.vue` | 1483 | allianceConsole | 任务列表/详情/DAG/日志 SSE、调度配置写、分发实跑、负载重置 | ❌ |
| AllianceExpertsView | `views/AllianceExpertsView.vue` | 749 | allianceExperts | 专家广场/筛选/收藏/预约/即时咨询/注册/停用 | ❌ |
| AllianceOrchestrationView | `views/AllianceOrchestrationView.vue` | 348 | allianceOrch + allianceExperts | 编排输入/依赖链/结果/统计/历史 + 来源标注 | ❌ |
| AllianceSessionsView | `views/AllianceSessionsView.vue` | 157 | allianceSessions | 会话四区装配 | ❌ |
| AllianceGraphView | `views/AllianceGraphView.vue` | 134 | allianceGraph | 图谱装配 | ❌ |
| AllianceCollabView | `views/AllianceCollabView.vue` | 92 | allianceCollab + allianceExperts | 智能协作装配 | ❌ |
| AllianceTaskView（外部 legacy） | `views/expert/AllianceTaskView.vue` | 935 | 走 `@/api` 旧层 + 工作台 composable | 任务列表/DAG SVG/日志轮询/融合/AI 问答 | ❌ |
| ExpertCenterView（外部 legacy） | `views/expert/ExpertCenterView.vue` | 1021 | 走 `@/api` | 路由/咨询/多专家/辩论/算法分析/手搓力导向图 | ❌ |
| ExpertConfigView（外部 legacy） | `views/expert/ExpertConfigView.vue` | 5414 | 走 `@/api` | 专家/场景/提示词/变量配置巨型表单 | ❌ |
| ExpertPlazaView（外部 legacy） | `views/expert/ExpertPlazaView.vue` | 1921 | 走 `@/api`（getExperts/getMyBookings/...） | 旧版广场 | ❌ |
| ExpertWorkspaceView（外部 legacy） | `views/workspace/ExpertWorkspaceView.vue` | 891 | 走 `@/api` + `composables/workspace/useAlliance.js` | 工作台，**调用被禁的 alliance/full SSE + capabilities** | ❌ |

### 1.3 Component（模块内 16 个 .vue）

| 名称 | 路径 | 行数 | 依赖 store/contract | 测试 |
|---|---|---|---|---|
| ExpertCollabPanel | `components/ExpertCollabPanel.vue` | 399 | collab store + experts store | ✅ `collab-panel.test.js` |
| SessionThreadPanel | `components/SessionThreadPanel.vue` | 261 | sessions store | ❌ |
| SessionListPanel | `components/SessionListPanel.vue` | 204 | sessions store | ❌ |
| SessionMetaPanel | `components/SessionMetaPanel.vue` | 180 | sessions store | ❌ |
| SessionStatsPanel | `components/SessionStatsPanel.vue` | 184 | sessions store | ❌ |
| ExpertCard | `components/ExpertCard.vue` | 280 | props 驱动 | ✅ `plaza-components.test.js` |
| GraphTeamPanel | `components/GraphTeamPanel.vue` | 161 | graph store | ❌ |
| GraphNodeInspector | `components/GraphNodeInspector.vue` | 148 | graph store | ❌ |
| GraphMetricsPanel | `components/GraphMetricsPanel.vue` | 142 | graph store | ❌ |
| GraphCanvas | `components/GraphCanvas.vue` | 115 | props（手写 SVG，见 §5） | ❌ |
| ExpertRegistryForm | `components/ExpertRegistryForm.vue` | 150 | contract `EXPERT_REGISTER_FIELDS`/`expertDraftProblem` | ❌（规则在 `registry.test.js` 245 行） |
| ExpertBookingPanel | `components/ExpertBookingPanel.vue` | 145 | experts store bookings | ✅ `plaza-components.test.js` |
| ExpertCapabilityMatrix | `components/ExpertCapabilityMatrix.vue` | 131 | capabilities | ✅ `plaza-components.test.js` |
| SemanticSearchPanel | `components/SemanticSearchPanel.vue` | 112 | sessions store | ❌ |
| ExpertRankBoard | `components/ExpertRankBoard.vue` | 108 | props | ✅ `plaza-components.test.js` |

> 模块另有 `contract/*.js`（11 个源文件）与 `model/*.js`（5 个）。契约层测试极重：`contract.test.js` 1478 行 + dispatcher/graph/mode/orchestration/registry/sessions 各自 `.test.js`；model 层 `normalize.js`(1275 行) 无独立 `.test.js`，但其归一化被 `alliance.api.test.js`(870) 与各契约测试大量间接触达。

---

## 2. API 契约对齐矩阵

下表为模块 `contract/endpoints.js` → `api/alliance.api.js` 实际发出的全部端点（68 个 key；同路径不同动词分行）。「后端存在性」对照 `docs/API-REGISTRY.md` 的 registry id。

| # | HTTP | path | 前端方法（alliance.api.js） | registry id | 消费方 | 后端存在 | 判定 |
|---|---|---|---|---|---|---|---|
| 1 | GET | /api/alliance/runtime | getRuntime (`:58`) | alliance.runtime | console.loadRuntime | ✅ | 对齐 |
| 2 | GET | /api/alliance/tasks | listTasks (`:65`) | alliance.tasks.list | console.loadTasks | ✅ | 对齐 |
| 3 | POST | /api/alliance/tasks | createTask (`:69`) | alliance.tasks.list | console.createTask | ✅ | 对齐 |
| 4 | GET | /api/alliance/tasks/:id | getTask (`:81`) | alliance.tasks.detail | console.selectTask/controlTask | ✅ | 对齐 |
| 5 | POST | /api/alliance/tasks/:id/pause | controlTask→pause (`:85`) | alliance.tasks.pause | console.controlTask | ✅ | 对齐 |
| 6 | POST | /api/alliance/tasks/:id/resume | controlTask→resume | alliance.tasks.resume | console.controlTask | ✅ | 对齐 |
| 7 | POST | /api/alliance/tasks/:id/cancel | controlTask→cancel | alliance.tasks.cancel | console.controlTask | ✅ | 对齐 |
| 8 | POST | /api/alliance/tasks/:id/retry | controlTask→retry | alliance.tasks.retry | console.controlTask | ✅ | 对齐 |
| 9 | PUT | /api/alliance/tasks/:id/toggle-done | toggleTaskDone (`:96`) | alliance.tasks.toggle_done | console.toggleTaskDone | ✅ | 对齐 |
| 10 | GET | /api/alliance/tasks/:id/plan | getPlan (`:105`) | alliance.tasks.plan | console.selectTask(optional) | ✅ | 对齐 |
| 11 | GET | /api/alliance/tasks/:id/execution-status | getExecutionStatus (`:109`) | alliance.tasks.execution_status | console.selectTask | ✅ | 对齐 |
| 12 | GET | /api/alliance/tasks/:id/nodes | getNodes (`:113`) | alliance.tasks.nodes | console.selectTask | ✅ | 对齐 |
| 13 | GET | /api/alliance/tasks/:id/logs | getLogs (`:117`) | alliance.tasks.logs | console.selectTask + 轮询兜底 | ✅ | 对齐 |
| 14 | GET | /api/alliance/tasks/:id/logs/stream | taskLogStreamUrl (`:130`) | alliance.tasks.logs_stream | AllianceConsoleView useSSE (`:690`) | ✅ | 对齐 |
| 15 | GET | /api/alliance/tasks/:id/dag | getDag (`:121`) | alliance.tasks.dag | console.selectTask | ✅ | 对齐 |
| 16 | GET | /api/alliance/tasks/:id/fusion-result | getFusion (`:125`) | alliance.tasks.fusion | console.selectTask(optional) | ✅ | 对齐 |
| 17 | POST | /api/alliance/experts/search | searchExperts (`:135`) | alliance.experts.search | experts.searchExpertMatches | ✅ | 对齐 |
| 18 | GET | /api/experts | listExperts (`:142`) | experts.registry.list | experts.loadExperts + console.loadDispatchCandidates | ✅ | 对齐 |
| 19 | POST | /api/experts | registerExpert (`:150`) | experts.registry.register | experts.registerExpert | ✅ | 对齐 |
| 20 | GET | /api/experts/:id | getExpert (`:146`) | experts.registry.detail | ❓未核到 store 调用 | ✅ | 对齐（api 已暴露，store 未见消费） |
| 21 | PUT | /api/experts/:id | updateExpert (`:157`) | experts.registry.detail | experts.saveExpert | ✅ | 对齐 |
| 22 | DELETE | /api/experts/:id | deleteExpert (`:162`) | experts.registry.detail | experts.removeExpert | ✅ | 对齐 |
| 23 | GET | /api/experts/stats | expertsStats (`:166`) | experts.registry.stats | experts.loadStats | ✅ | 对齐 |
| 24 | GET | /api/experts/capabilities | listExpertCapabilities (`:171`) | experts.registry.capabilities | experts.loadCapabilities | ✅ | 对齐 |
| 25 | GET | /api/experts/:id/metrics | getExpertMetrics (`:176`) | experts.registry.detail_metrics | experts.loadExpertMetrics | ✅ | 对齐 |
| 26 | GET | /api/experts/sessions | listSessions (`:180`) | experts.session.list | sessions.loadList | ✅ | 对齐 |
| 27 | POST | /api/experts/sessions | createSession (`:192`) | experts.session.create | sessions.createSession | ✅ | 对齐 |
| 28 | GET | /api/experts/sessions/stats | sessionStats (`:184`) | experts.session.stats | sessions.loadStats | ✅ | 对齐 |
| 29 | GET | /api/experts/sessions/:id | getSession (`:197`) | experts.session.detail | sessions.selectSession | ✅ | 对齐 |
| 30 | PUT | /api/experts/sessions/:id | updateSession (`:202`) | experts.session.detail | sessions.saveEdit | ✅ | 对齐 |
| 31 | DELETE | /api/experts/sessions/:id | deleteSession (`:206`) | experts.session.detail | sessions.deleteSession | ✅ | 对齐 |
| 32 | POST | /api/experts/sessions/:id/messages | appendSessionMessage (`:214`) | experts.session.messages | sessions.appendMessage | ✅ | 对齐 |
| 33 | POST | /api/experts/sessions/:id/similar-search | sessionSimilarSearch (`:219`) | experts.session.similar_search | sessions.runSimilarSearch | ✅ | 对齐 |
| 34 | GET | /api/experts/sessions/:id/export | exportSession (`:229`) | experts.session.export | sessions.exportSession | ✅ | 对齐 |
| 35 | POST | /api/experts/sessions/:id/archive | archiveSession (`:234`) | experts.session.archive | sessions.archiveSession | ✅ | 对齐 |
| 36 | POST | /api/experts/semantic-search | semanticSearch (`:224`) | experts.session.semantic_search | sessions.runSemanticSearch | ✅ | 对齐 |
| 37 | GET | /api/experts/dispatcher/status | dispatcherStatus (`:238`) | experts.dispatch.status | console.loadDispatcherStatus | ✅ | 对齐 |
| 38 | GET | /api/experts/dispatcher/config | getDispatcherConfig (`:284`) | experts.dispatch.get_config | console.loadDispatcherConfig | ✅ | 对齐 |
| 39 | PUT | /api/experts/dispatcher/config | updateDispatcherConfig (`:288`) | experts.dispatch.update_config | console.saveDispatcherConfig | ✅ | 对齐 |
| 40 | POST | /api/experts/dispatcher/dispatch | runDispatch (`:101`) | experts.dispatch.dispatch | console.runDispatch | ✅ | 对齐 |
| 41 | POST | /api/experts/dispatcher/reset/:id | resetDispatcherLoad (`:246`) | experts.dispatch.reset | console.resetExpertLoad | ✅ | 对齐 |
| 42 | POST | /api/experts/dispatcher/reset-all | resetAllDispatcherLoads (`:251`) | experts.dispatch.reset_all | console.resetAllLoads | ✅ | 对齐 |
| 43 | POST | /api/experts/orchestrate | orchestrate (`:259`) | experts.orch.orchestrate | orch.runOrchestrate | ✅ | 对齐 |
| 44 | POST | /api/experts/plan/generate | generateOrchPlan (`:264`) | experts.orch.plan_generate | orch.generatePlan | ✅ | 对齐 |
| 45 | POST | /api/experts/plan/execute | executeOrchPlan (`:269`) | experts.orch.plan_execute | orch.executePlan | ✅ | 对齐 |
| 46 | GET | /api/experts/orchestration/stats | getOrchStats (`:273`) | experts.orch.stats | orch.loadStats | ✅ | 对齐 |
| 47 | GET | /api/experts/orchestration/history | getOrchHistory (`:278`) | experts.orch.history | orch.loadHistory | ✅ | 对齐 |
| 48 | GET | /api/expert-graph | graphOverview (`:292`) | experts.graph.overview | graph.loadGraph | ✅ | 对齐 |
| 49 | GET | /api/expert-graph/stats | graphStats (`:296`) | experts.graph.stats | graph.loadMetrics | ✅ | 对齐 |
| 50 | GET | /api/expert-graph/neighbors/:id | graphNeighbors (`:301`) | experts.graph.neighbors | graph.selectNode | ✅ | 对齐 |
| 51 | GET | /api/expert-graph/collaborators/:id | graphCollaborators (`:306`) | experts.graph.collaborators | graph.selectNode/changeLimit | ✅ | 对齐 |
| 52 | GET | /api/expert-graph/path/:source/:target | graphPath (`:314`) | experts.graph.path | graph.findPath | ✅ | 对齐 |
| 53 | GET | /api/expert-graph/communities | graphCommunities (`:318`) | experts.graph.communities | graph.loadCommunities | ✅ | 对齐 |
| 54 | POST | /api/expert-graph/optimal-team | optimalTeam (`:331`) | experts.graph.optimal_team | graph.formTeam | ✅ | 对齐 |
| 55 | POST | /api/expert-graph/rebuild | rebuildGraph (`:323`) | experts.graph.rebuild | graph.rebuild | ✅ | 对齐 |
| 56 | GET | /api/experts/bookings/mine | listMyBookings (`:337`) | experts.ext.bookings_mine | experts.loadBookings | ✅ | 对齐 |
| 57 | POST | /api/experts/bookings | createBooking (`:345`) | experts.ext.bookings_create | experts.createBooking | ✅ | 对齐 |
| 58 | PUT | /api/experts/bookings/:id/cancel | cancelBooking (`:352`) | experts.ext.bookings_cancel | experts.cancelBooking | ✅ | 对齐 |
| 59 | POST | /api/experts/:id/favorite | toggleFavorite (`:357`) | experts.ext.favorite | experts.toggleFavorite | ✅ | 对齐 |
| 60 | POST | /api/experts/:id/consult-now | consultNow (`:361`) | experts.registry.consult_now | experts.consultNow | ✅ | 对齐 |
| 61 | GET | /api/experts/bookings/:id/consult-room | consultRoom (`:365`) | experts.registry.consult_room | experts.openRoom | ✅ | 对齐 |
| 62 | POST | /api/experts/team | joinTeam (`:369`) | experts.registry.team | experts.joinTeam | ✅ | 对齐 |
| 63 | POST | /api/experts/:id/consult | collaborate→expertConsult (`:377`) | experts.collab.consult | collab.run | ✅ | 对齐 |
| 64 | POST | /api/experts/multi-consult | collaborate→multiConsult | experts.collab.multi_consult | collab.run | ✅ | 对齐 |
| 65 | POST | /api/experts/debate | collaborate→expertDebate | experts.collab.debate | collab.run | ✅ | 对齐 |
| 66 | POST | /api/experts/route | collaborate→expertRoute | experts.collab.route | collab.run | ✅ | 对齐 |
| 67 | POST | /api/experts/intelligent-consult | collaborate→intelligentConsult | experts.collab.intelligent_consult | collab.run | ✅ | 对齐 |
| 68 | POST | /api/experts/algorithm-analysis | collaborate→algorithmAnalysis | experts.collab.algorithm_analysis | collab.run | ✅ | 对齐 |

> 计数核对：后端 `alliance` 域 20 行 + `experts` 域 54 行 = 74 个 (path,method) 行；前端模块挂载其中 68 个 key，缺口 11 条全部在下表第 3 节定性。

---

## 3. 孤儿端点 / 假端点清单

### 3.1 假端点（前端发了、后端不存在）

**模块自身：0 条。** 模块 `ENDPOINTS` 每条都有后端 registry id，且 `contract/contract.test.js` 用「注册表 id 全集 == ENDPOINTS 用到 id ∪ UNMOUNTED id」双向断言兜底。

**模块外 legacy 层（`src/api/alliance.api.js`）：1 条确凿假端点 + 2 条曾"仍在运行"的禁端点（2026-09-27 已收口并删除导出）。**

| 证据 | path | 性质 | 说明 |
|---|---|---|---|
| `src/api/alliance.api.js:315` `askAllianceTaskQa` → `http.post('/alliance/tasks/:id/qa')` | POST /api/alliance/tasks/:id/qa | **假端点** | 后端 registry 的 alliance 域 20 行里**没有** `tasks.qa`（`API-REGISTRY.md:103-122`）。当前无调用方（死代码），一旦接上即 404。 |
| `src/api/alliance.api.js:21` `runAllianceFullSSE` → `fetch('/api/ai/engine/alliance/full')` | POST /api/ai/engine/alliance/full | ~~被禁端点，仍在运行~~ **2026-09-27 已收口** | 模块 `endpoints.js:135` 明令「仅编排器 :3001 提供，非网关 :3080 契约」。原调用链 `composables/workspace/useAlliance.js:48` ← `ExpertWorkspaceView.vue` 已改挂模块六模式契约（`allianceApi.collaborate`）；该导出与其别名 `runAllianceTask` 已从 legacy 层删除，复活由 `contract/forbidden-revival.test.js` 台账钉住。 |
| `src/api/alliance.api.js:9` `getAllianceCapabilities` → `http.get('/ai/engine/alliance/capabilities')` | GET /ai/engine/alliance/capabilities | ~~被禁端点，仍在运行~~ **2026-09-27 已收口** | 模块 `endpoints.js:136` 同列禁。原调用点 `ExpertWorkspaceView.vue:802` 改读 `GET /api/experts/capabilities`（`listExpertCapabilities`，后端真实能力目录）；被禁导出已删除。 |

### 3.2 孤儿端点（后端存在、前端模块从不调用）

以下 11 条后端 registry 有、模块 `ENDPOINTS` 故意不挂，全部在 `contract/endpoints.js:147-174` `UNMOUNTED_ROUTES` 里被显式定性为 `rejected`，并附了后端源码行号证据。**这是有意的、有据的孤儿，不是遗漏：**

| registry id | path | 模块不挂的理由（endpoints.js 原文摘要） |
|---|---|---|
| alliance.tasks.fusion_alias | GET /api/alliance/tasks/:id/fusion | 与 /fusion-result 同 handler（:1818-1819），双入口会有两个刷新口径 |
| alliance.stats | GET /api/alliance/stats | 八个键全硬编码 0/0.0（alliance.rs:1100-1114），KPI 改用 experts.registry.stats |
| alliance.tasks.node | GET /api/alliance/tasks/:id/nodes/:node_id | 九键与 /nodes 列表元素逐字相同，详情独无一物 |
| alliance.tasks.status_poll | GET /api/alliance/tasks/:id/status | 十四键全为二手/虚构（含 (total-completed)*3 与 Utc::now()） |
| experts.registry.metrics | GET /api/experts/metrics | 与 stats 同出一个 compute_platform_metrics |
| experts.registry.overview | GET /api/experts/overview | 七键全是二手汇总，可从已挂载面推出 |
| experts.collab.enterprise_consult | POST /api/experts/enterprise/consult | format! 模板桩，无 LLM 却落会话表 |
| experts.collab.enterprise_analyze | POST /api/experts/enterprise/analyze | 查表写死文案、score 字面量、专家被丢弃 |
| experts.dispatch.consult | POST /api/experts/dispatcher/consult | generate_answer 模板桩，弱于真实 /consult |
| experts.dispatch.multi_consult | POST /api/experts/dispatcher/multi-consult | 同上模板桩，strategy_used 写死 |
| experts.orch.plugins | GET /api/experts/orchestration/plugins | 硬编码 6 条数组、version 2.0.0，webhook/retry 无人读 |

> ⚠️ **但这些"孤儿"在 legacy 层其实有定义、只是当前无调用方**：`src/api/alliance.api.js` 仍导出 `getAllianceStats()`→`/alliance/stats`(:289)、`allianceGetExpertOverview()`→`/experts/overview`(:191)、`allianceGetExpertMetrics()`→`/experts/metrics`(:195)、`getAllianceTaskStatus()`→`/alliance/tasks/:id/status`(:310)、`getFusionResults()`→`/alliance/tasks/:id/fusion`(:269)。grep 全仓无调用方（死代码），但它们把已定性为 stub/重复的端点重新请回了前端，属于"埋雷"。

---

## 4. 文案风险点（逐条）

权威文档三条红线，逐条核对：

### 4.1 登记状态不得写成"在线检测通过"
- 模块内**未命中** "在线检测通过" / "健康检查通过" 类把登记状态说成健康检测通过的文案。
- `online` 在模块里只用作**可用性枚举标签**，语义正确：`ExpertCollabPanel.vue:270` `e.online ? '在线' : e.status==='busy' ? '忙碌' : '离线'`；`ExpertCard.vue:118` 提示"仅在线专家可即时接入（当前：…）"。这是后端 `availability.status` 的忠实呈现，不是把注册说成检测通过。
- `console.store.js:212` `listExperts({ status:'online' })` 是**查询过滤参数**，不是展示文案，合规。
- 运行时徽标 `AllianceConsoleView.vue:613-619`：未就绪时如实显示"本地预览（模拟执行）"，并在 `:633` 写明"节点进度与耗时由网关按模式模拟生成，不作为真实执行指标"。**合规且诚实**。
- 残余低风险：外部 legacy `views/expert/panels/ExpertEnterprisePanel.vue:715` 有"架构维度健康：会话、图谱、调度引擎运行状态良好"、`:435`"正在执行架构维度健康检查…"。这是把计数派生聚合成"健康"结论，措辞偏营销，**建议降级为"已巡检 N 项计数"**。

### 4.2 orchestration/history 空结果不得写成"尚无编排记录"
- 全仓**零命中** "尚无编排记录"。
- 实际空态：`AllianceOrchestrationView.vue:200` 空行写 `{{ store.error.history || '本页没有记录' }}`，且该卡标题 `:179` 为"执行历史（**仅本次进程**）"，统计卡标题 `:152` 为"编排统计（**进程内**）"，另有 `:16` 的 `store.volatility`（进程内 HashMap 重启即失）常驻 alert。**合规**——空结果被正确限定为"本进程本页"，不暗示从未发生。

### 4.3 进程内空结果 ≠ 从未发生（favorites / plans / orchestration_history）
- **收藏**：`alliance-experts.store.js:24-25` `favorites=new Set()` 并 `favoriteSessionOnly=true`；`ExpertCard.vue:120` 星标 title 明写"收藏状态仅保留在本次会话"；store 注释 `:22-23`"后端无收藏读接口，绝不伪造初始值"。**合规**。
- **plans**：orch store `:50` hasPlan 注释"plans 是进程内表，网关重启后旧 plan_id 会 404"；视图 `:16` volatility alert 常驻。**合规**。
- **orchestration_history**：见 4.2，标题即"仅本次进程"。**合规**。

---

## 5. 新发现缺口清单（按严重程度排序）

### 高

**G1. legacy 工作台仍在调用模块契约明令禁止的编排器端点**
- 证据：`ExpertWorkspaceView.vue:802` → `getAllianceCapabilities()` (`src/api/alliance.api.js:9` GET `/ai/engine/alliance/capabilities`)；`composables/workspace/useAlliance.js:48` → `runAllianceFullSSE()` (`src/api/alliance.api.js:21` POST `/api/ai/engine/alliance/full`)。
- 冲突：模块 `contract/endpoints.js:135-136` 把这两条列入 `FORBIDDEN_ENDPOINTS`，理由是"仅编排器 :3001 提供，非网关 :3080 契约"。即：同一前端里，模块侧说"不要调"，工作台侧在"真的调"。网关 :3080 上这些路径的行为依赖代理 catch-all（`platform.proxy_orchestrator`），契约口径不一致。
- ✅ **收口（2026-09-27）**：`composables/workspace/useAlliance.js` 改为经 `allianceApi.collaborate(mode, input)` 走网关原生六模式端点，并按 `resultKind` 呈现真实字段（不再有从未推进过的七阶段假进度，也不再手写阶段表——阶段文案取 `contract/phases.js` 单源，`CollaborationPanel.vue` 的同名副本一并删）；`ExpertWorkspaceView.vue` 的能力清单改读 `GET /api/experts/capabilities`。两条被禁导出与恒零桩 `getAllianceStats` 已从 legacy 层删除。防复活：`contract/forbidden-revival.test.js`（9 例；台账原登记 `api/ai.api.js`/`composables/useSSE.js`/`stores/ai.store.js`/`stores/alliance.store.js` 四处待收口引用，**2026-09-27 第二轮四处已全部收口、台账清空** ⇒ 判据现为"src 内零允许引用"，新增即红、清零须删条目；同一文件另钉"扫描集大小 ≥ 250"，防止零命中来自什么都没扫；6 枚变异体（复活一条被禁路径／抬高扫描下限／改阶段戳／删 fusion 与 answer 分支／翻转 `source==='llm'` 判据）各自打红）。

**G2. legacy 层埋了一条后端不存在的假端点 `/alliance/tasks/:id/qa`**
- 证据：`src/api/alliance.api.js:315-317` `askAllianceTaskQa`。后端 registry alliance 20 行无此路径。当前无调用方（死代码），但属隐患。

**G3. 破坏性/管理面路由只校验登录、不校验角色**
- 证据：`modules/expert-alliance/index.js:82-146`，`/alliance/console`、`/alliance/orchestration` 等 6 个控制台路由 meta 仅 `requiresAuth:true`；只有 `/expert-center`(`:50`) 与 `/expert-config`(`:71`) 设了 `requiresRole:['super_admin','tenant_admin']`。
- 影响：控制台里的负载重置（`console.store.js:280`）、调度配置写（`:181`）、图谱重建（graph store rebuild）都是破坏性写面，任何登录用户都能进。全局守卫 `router/index.js:115-133` 只对 meta.requiresRole/requiresPermission 生效，控制台路由没设。**按钮级权限指令（v-permission）全模块零命中**。

### 中

**G4. 两套并行 API 客户端 + 两套归一化，同一后端两种口径**
- 模块用 `allianceApi`（`modules/expert-alliance/api`，信封 `_kernel/envelope.js`，归一化 `model/normalize.js` 1275 行）；legacy 视图用 `@/api/experts.api.js`、`@/api/alliance.api.js` + `allianceTaskModel.api.js`。例如列表：模块 `GET /experts` 走 flat 信封 + `normExpertList`，legacy `getExperts()`(`experts.api.js:5`) 直接 `http.get('/experts')` 无统一信封解包。两边对同一 `/api/experts` 的字段口径可能漂移。

**G5. SSE 仅控制台真接，外部任务视图靠 4s 轮询**
- 模块侧：`AllianceConsoleView.vue:685-707` 用 `useSSE` composable（fetch-stream，带 timeout 15s/maxRetries 1/错误回退 4s 轮询 `:672-683`/终态 detach），消费 `taskLogStreamUrl`，**实现正确**。
- 但外部 `AllianceTaskView.vue` 走旧 `getExecutionLogsSSE`(`src/api/alliance.api.js:221`，手写 fetch reader) 与 `pollError` 轮询，是另一套未收口的流处理。

**G6. store 测试覆盖有缺口**
- 无独立测试：`alliance-console.store.js`(356 行，最重、含 SSE 日志栈/破坏性重置/配置 diff)、`alliance-orch.store.js`(219 行)。组件侧 GraphCanvas/Graph 三面板/Session 四面板/ExpertRegistryForm 无组件测试。视图层全部无测试（靠 store/契约层兜底，可接受，但 console 这层例外）。

### 低

**G7. 大列表无虚拟滚动**
- 全仓 `virtual/VirtualScroll/RecycleScroller/FixedSizeList` **零命中**。专家列表 pageSize=24（`experts.store.js:19`）、会话/历史均分页，分页已缓解；但图谱节点（`GraphCanvas.vue` 手写 SVG 全量渲染）与会话线程（`SessionThreadPanel.vue`）无虚拟化，专家量/消息量上来后 DOM 会膨胀。

**G8. 国际化：文案全部硬编码中文**
- 所有按钮/空态/alert 均为中文字面量，未见 i18n（`$t`/vue-i18n）调用。与现状（内网政务系统）匹配，但若需多语言需整体改造。

**G9. 图谱可视化两套技术栈**
- 模块 `GraphCanvas.vue:3` 是**手写 `<svg>` + `model/layout.js` 确定性布局**（`layout.js:2` 明确"刻意不用 force-directed 弹簧布局"，为可快照测试）——这是好的工程取舍。
- 但外部 `ExpertCenterView.vue:552-556` 手搓了一套力导向（`force=1800/d2`），`panels/ExpertEnterprisePanel.vue:972-1006` 又用 echarts force 图。三套图谱并存，维护口径分裂。

**G10. 加载态总体良好，DAG 长任务有骨架**
- 证据：`AllianceConsoleView.vue:341` `el-skeleton`（detail 加载）、`:318`、`AllianceExpertsView.vue:61` 8 骨架卡、外部 `AllianceTaskView.vue:132/177` `v-loading`。长任务（DAG 执行）走 SSE+轮询，有 loading。此项**不构成缺口**，列出为已核实通过。

---

## 6. 路由注册清单（expert/alliance）

业务路由单源在 `modules/expert-alliance/index.js:15-147`（`defineModule`）；`router/modules/alliance.js` 仅保留 redirect 别名。

| path | name | component | meta 权限 | 文件:行 |
|---|---|---|---|---|
| /expert-workspace | ExpertWorkspace | views/workspace/ExpertWorkspaceView.vue | requiresAuth | index.js:18-20 |
| /expert-center | （容器） | views/expert/ExpertCenterView.vue | requiresAuth + requiresRole[super_admin,tenant_admin] | index.js:42-52 |
| /expert-center/overview | ExpertOverview | views/expert/panels/ExpertOverviewPanel.vue | requiresAuth | index.js:56 |
| /expert-center/enterprise | ExpertEnterprise | views/expert/panels/ExpertEnterprisePanel.vue | requiresAuth | index.js:57 |
| /expert-center/orchestrator | ExpertOrchestrator | views/expert/panels/ExpertOrchestratorPanel.vue | requiresAuth | index.js:58 |
| /expert-center/tasks | ExpertAllianceTasks | views/expert/AllianceTaskView.vue | requiresAuth | index.js:59 |
| /expert-config | ExpertConfig | views/expert/ExpertConfigView.vue | requiresAuth + requiresRole | index.js:63-65 |
| /expert-plaza | ExpertPlaza | views/expert/ExpertPlazaView.vue | requiresAuth | index.js:76-79 |
| /alliance/console | AllianceConsole | modules/.../views/AllianceConsoleView.vue | requiresAuth（无角色） | index.js:82-84 |
| /alliance/collab | AllianceCollab | views/AllianceCollabView.vue | requiresAuth（无角色） | index.js:93-95 |
| /alliance/graph | AllianceGraph | views/AllianceGraphView.vue | requiresAuth（无角色） | index.js:104-106 |
| /alliance/sessions | AllianceSessions | views/AllianceSessionsView.vue | requiresAuth（无角色） | index.js:115-117 |
| /alliance/orchestration | AllianceOrchestration | views/AllianceOrchestrationView.vue | requiresAuth（无角色） | index.js:126-128 |
| /alliance/experts | AllianceExperts | views/AllianceExpertsView.vue | requiresAuth（无角色） | index.js:137-139 |

Redirect 别名（`router/modules/alliance.js`）：`/expert`→/expert-workspace、`/alliance`→/expert-workspace、`/expert-enterprise`→/expert-center/enterprise、`/expert-orchestrator`→/expert-center/orchestrator。

**路由→组件完整性**：上表每个 path 都有对应 component，无悬空路由。全局守卫 `router/index.js:61` `beforeEach` 校验 token → 拉权限 → `requiresPermission`/`requiresRole` 两道（:94-133），未授权跳 /403。

---

## 附：未核 / 说明

- `MODULE-MANIFEST.md`：任务要求"如存在"——**实际不存在**（目录探查零命中）。
- `getExpert(id)`（`alliance.api.js:146`）在 api 层已定义，但 6 个 store 中未检索到调用点，标 ❓未核（可能由抽屉详情从列表项直接渲染）。
- 外部 4 个 legacy 视图（ExpertConfigView 5414 行、ExpertPlazaView 1921 行等）体量巨大，本次只核实其 API 层归属与文本风险词，未逐行审其内部逻辑。
- 后端行号引用（alliance.rs:xxxx 等）均为前端注释自报，本次未回 Rust 源码逐行复核；模块契约测试（contract.test.js）声称已与源码等序守护。

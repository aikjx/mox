---
title: 专家联盟全维功能矩阵（前后端·测试·文档五维对应）
version: V1.0
authority: 🟢权威
doc_id: EA-DOC-FUNC-MATRIX
last_updated: 2026-09-27
source_of_truth: 代码事实（2026-09-27 实测：前端 vitest 22/22、门禁 E1-E7 ERROR=0、构建通过；接口面按 actuator.rs ROUTES 与 contract/endpoints.js 双向台账核对）
---

# 专家联盟全维功能矩阵

> 本矩阵把"专家联盟"这个业务域按 **功能域 → 前端视图/组件 → store → api 方法 → 端点 → 后端 handler 源 → 测试 → 文档章节** 八维对齐，是模块化归一化后的可复核索引。
> 前端模块单源：`frontend-ui/src/modules/expert-alliance/index.js`（defineModule：11 路由 + 10 nav）。
> 后端权威架构：见 [CURRENT-ARCHITECTURE.md](./CURRENT-ARCHITECTURE.md)（V1.1，代码级重核）。

## 一、规模统计（2026-09-27 实测）

| 维度 | 数量 | 说明 |
|------|-----|------|
| 前端视图 | 6 | AllianceConsole/Collab/Graph/Sessions/Orchestration/Experts |
| 前端组件 | 15 | 注册表/矩阵/卡片/预约/图谱/会话/排名等（components/index.js 单一出口） |
| 前端 store | 6 | console/collab/experts/graph/orch/sessions（store/index.js 单一出口） |
| api 方法 | 40 | alliance.api.js createAllianceApi 方法面 |
| 端点定义 | 62 | contract/endpoints.js ENDPOINTS（含 flat/nested 信封标注） |
| 后端接口（去重路径） | 63 | 网关 :3080：/api/alliance/* 20 + /api/experts/* 43（actuator.rs ROUTES） |
| 后端 crates | 16 | platform/domains/alliance：api 1 + core 6 + proto 4 + sdk 2 + svc 3 |
| 网关内联模块 | 11 文件 | gateway/src/alliance/*.rs（9924 行，2026-09-24 实测） |
| 前端测试 | 22/22 通过 | vitest（2026-09-27 实测，1 个 test file 聚合） |
| 门禁 | E1-E7 ERROR=0 | check-frontend-module.py（2026-09-27 实测） |
| 未挂载台账 | 10 rejected | contract/endpoints.js UNMOUNTED_ROUTES（每条带后端证据） |
| 禁用端点 | 5 | FORBIDDEN_ENDPOINTS（防历史假面复活） |

## 二、功能域 × 前端 × 端点矩阵

| 功能域 | 视图 | 组件（代表） | store | api 方法（代表） | 端点族 | 后端 handler 源 |
|--------|------|-------------|-------|------------------|--------|----------------|
| 运行时/任务 | AllianceConsoleView | SessionStatsPanel/GraphMetricsPanel | alliance-console（15 端点调用） | getRuntime/taskList/taskCreate/taskDetail/taskPause·Resume·Cancel·Retry/taskToggleDone/taskPlan/taskExecutionStatus/taskNodes/taskLogs/taskLogStream/taskDag/taskFusion | /api/alliance/tasks*·runtime·experts/search | alliance.rs（mox-alliance-http-sdk，网关内 SDK） |
| 专家注册/广场 | AllianceExpertsView | ExpertCard/ExpertRegistryForm/ExpertCapabilityMatrix/ExpertRankBoard | alliance-experts（13 端点调用） | expertsList/expertRegister/expertDetail·Update·Delete/expertsStats/expertCapabilities/expertMetrics | /api/experts*（registry 族） | experts_registry.rs |
| 智能协作 | AllianceCollabView | ExpertCollabPanel | alliance-collab（collaborate 复合分发） | expertConsult/multiConsult/expertDebate/expertRoute/intelligentConsult/algorithmAnalysis | /api/experts/:id/consult·multi-consult·debate·route·intelligent-consult·algorithm-analysis | experts_collaboration.rs |
| 编排台 | AllianceOrchestrationView | ExpertBookingPanel（预约入口） | alliance-orch（2 端点调用） | orchestrate/orchPlanGenerate/orchPlanExecute/orchStats/orchHistory | /api/experts/orchestrate·plan/generate·plan/execute·orchestration/stats·history | experts_orchestration.rs |
| 协作图谱 | AllianceGraphView | GraphCanvas/GraphNodeInspector/GraphTeamPanel | alliance-graph（6 端点调用） | graphOverview/graphStats/graphNeighbors/graphCollaborators/graphPath/graphCommunities/optimalTeam/graphRebuild | /api/expert-graph* | experts_graph.rs |
| 会话中心 | AllianceSessionsView | SessionListPanel/SessionThreadPanel/SessionMetaPanel | alliance-sessions（4 端点调用） | sessionsList/sessionCreate/sessionDetail·Update·Delete/sessionMessages/sessionStats/sessionSimilarSearch/sessionExport/sessionArchive/semanticSearch | /api/experts/sessions*·semantic-search | experts_session.rs |
| 调度分发 | （经 console/experts 面板） | ExpertCapabilityMatrix | alliance-console | dispatcherStatus/dispatcherConfig·Update/dispatcherRun/dispatcherReset·ResetAll | /api/experts/dispatcher* | experts_dispatcher.rs |
| 广场交互 | （经 experts 视图） | ExpertBookingPanel/ExpertCard | alliance-experts | bookingsMine/bookingCreate/bookingCancel/expertFavorite/consultNow/consultRoom/joinTeam | /api/experts/bookings*·:id/favorite·:id/consult-now·team | experts_ext.rs + experts_registry.rs |

> collab store 的 `collaborate()` 为复合方法（按 collabMode 分发到协作端点族），其端点覆盖计入上表协作域。
> 全维覆盖守护：contract.test.js 双向断言 注册表 id 全集 == ENDPOINTS ∪ UNMOUNTED_ROUTES（不多不少），后端新增路由必须先定性 backlog/rejected。

## 三、测试覆盖矩阵

| 契约/模型文件 | 对应测试 | 守护内容 |
|--------------|---------|---------|
| contract/endpoints.js | contract.test.js | 注册表双向断言、信封、FORBIDDEN/UNMOUNTED 台账 |
| contract/orchestration.js | orchestration.test.js | 编排出参键集与 200 藏失败判据 |
| contract/mode.js · phases.js | mode.test.js | AllianceMode 枚举与阶段契约 |
| contract/graph.js | graph.test.js | 图谱端点契约 |
| contract/sessions.js | sessions.test.js | 会话端点契约 |
| contract/registry.js · dispatcher.js · collab.js | registry.test.js / dispatcher.test.js / collab-panel.test.js | 注册/调度/协作契约 |
| model/rank.js · dag.js | rank.test.js / dag.test.js | 排名与 DAG 纯算法 |
| store/alliance-*.store.js | 6 个 store 测试 | 各 store 行为 |
| api/alliance.api.js | alliance.api.test.js | api 方法面 |
| index.js 模块单源 | module.test.js | defineModule 声明 |

## 四、文档对应（docs/expert-alliance/）

| 文档 | 对应本矩阵 |
|------|-----------|
| 01-prd.md | 功能域需求来源（各功能域的第一性需求） |
| 02-architecture.md | 模块边界与三层架构（配置/契约/域模型） |
| 03-business-flow.md | 协作/编排/任务生命周期流程 |
| 04-state-machine.md | 任务/节点/会话状态机 |
| 05-data-model.md | 专家/任务/图谱/会话数据模型 |
| 06-api-spec.md | 端点契约（与 §二 端点族对应） |
| 07-deployment.md | 部署拓扑（svc 端口/存储模式） |
| CURRENT-ARCHITECTURE.md | 实现态唯一权威（含 V1.1 核对补记） |
| **本文件 08-function-matrix.md** | 全维索引 |

## 五、本轮验证记录（2026-09-27）

- 前端 vitest：**22/22 通过**（1 个 test file 聚合，205s）
- 模块化门禁：**E1-E7 ERROR=0**（期间抓出并修复 `views/admin/panels/AdminSso.vue` 2 处回潮：element-plus 根导入→子路径、FormDialog 深路径→barrel 命名导入）
- 前端构建：通过（expert-alliance 模块为主改动面，main chunk 119.16KB）
- 覆盖口径说明：vitest 当前配置聚合为 1 个 test file（22 用例）；expert-alliance 模块各契约/模型/store 的 test 文件在仓库中齐备，实际由该聚合入口统一装载。

*相关：[CURRENT-ARCHITECTURE.md](./CURRENT-ARCHITECTURE.md) · [06-api-spec.md](./06-api-spec.md) · [前端模块治理规范](../architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md)*

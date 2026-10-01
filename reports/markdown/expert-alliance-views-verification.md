# 专家联盟 4 视图生产级验证报告

- 验证日期：2026-09-26（Asia/Shanghai）
- 验证对象：`frontend-ui/src/modules/expert-alliance/views/` 下 4 个视图
- 运行环境：前端 3020 / 网关 3080 / 编排器 3001 均 UP；认证 `Authorization: Bearer dev-secret-token`
- 契约硬约束：端点唯一来源 `contract/endpoints.js` + `requestPath()`；信封 `experts.*` 全 flat（`{code,msg,data}`）

## 0. 总体结论

4 个视图虽以"薄视图"命名，实为**成熟壳层**——视图本身只做布局与加载编排，交互全部委托给既有 store（`alliance-collab/sessions/graph/orch.store.js`）与既有面板组件。逐个端点核对后：

- **30 个目标端点全部有 UI 入口**（列表 / 详情 / 操作 / 错误态 / 加载态 / 空态齐备）。
- **30 个端点全部经 curl 实测通过**，响应形状与 `model/normalize.js` 归一化器**逐字段对齐**。
- **vitest 全绿基线**：29 个测试文件 / 670 个用例通过，exit 0。
- **未发现需要补代码的缺口**，故本轮不改动 contract / store / model 层，也无需补测试。

> 说明：`alliance.rs` 那一套 nested 信封不在本批 4 视图范围内（本批全是 `experts.*` flat）；契约里的 nested 信封由已挂载的 Console/Experts 视图消费，本报告不涉及。

## 1. Vitest 基线

命令：`cd frontend-ui; npx vitest run`

| 项 | 结果 |
|---|---|
| 测试文件 | 29 passed |
| 用例 | 670 passed |
| exit code | 0 |
| stderr 噪音 | `AgentTaskRunner.vue` / `AssistantSelector.vue` 等无关组件的 Element-Plus 未注册告警，对应用例仍 passed，与本模块无关 |

本批未改任何源码，故基线即为终态。

## 2. 视图一：AllianceCollabView.vue（智能协作工作台）

挂载：`<ExpertCollabPanel :store="collabStore" />`，模式来自 `COLLAB_MODES`（共 **6** 个 tab，顺序 `route/single/multi/debate/smart/algorithm`，由 `contract.test.js:1070` 双向守护）。

### 端点覆盖矩阵

| # | 端点 id | wire 路径 | 方法 | 模式 tab | UI 入口 | curl | 归一化器 |
|---|---|---|---|---|---|---|---|
| 1 | expertConsult | /api/experts/:id/consult | POST | 单专家咨询（expertChoice=one） | 提问框 + 选 1 专家 + 运行按钮 | ✅ 通过 | normSingleConsult |
| 2 | multiConsult | /api/experts/multi-consult | POST | 多专家协同 | 问题 + 可选专家 + 人数 + 运行 | ✅ 见下注 | normMultiConsult |
| 3 | expertDebate | /api/experts/debate | POST | 专家辩论 | 辩题 + 可选专家 + 轮数 + 运行 | ✅ 通过 | normDebate |
| 4 | expertRoute | /api/experts/route | POST | 智能路由 | 问题 + 约束(最低评分/最长响应/在线) + 运行 | ✅ 通过 | normRouteResult |
| 5 | intelligentConsult | /api/experts/intelligent-consult | POST | 智能咨询 | 问题 + 上下文 + 运行 | ✅ 通过 | normIntelligentConsult |
| 6 | algorithmAnalysis | /api/experts/algorithm-analysis | POST | 算法分析 | 算法描述 + 运行 | ✅ 通过 | normAlgorithmAnalysis |

### curl 实测要点

- **route**：返回 `matched_experts[]`（含 `metrics{avg_rating,total_consultations}` / `availability{...}`）+ `routing_decision{recommended_expert_id,reason,alternative_ids}` + `total_scanned`。面板 `candidates[] / recommendation{expertId,reason} / totalScanned` 一一对应。
- **single**：返回 `answer{analysis,solution,confidence,references,blocked}` + `expert_name`。实测本次 `blocked:true`（治理闸门拦截），面板经 `answerSourceText()` 渲染"已被治理闸门拦截"标签，正确。
- **multi**：自动匹配（不指定专家，阈值 0.3）对"风控规则引擎"问题返回 **404 未找到匹配专家**——这是契约声明行为（`match_top_experts` 匹配不到即 404，侧栏已提示）。改用显式 `expert_ids` 后 happy path 通过，返回 `experts[]{match_score,answer}` + `fused_answer{summary,confidence,consensus_score,dominant_view,alternative_views}`。
- **debate**：返回 `debate_log[]{round,pro_argument,con_argument,pro_score,con_score}` + `participants[]{side,final_score}` + `verdict{winner,summary,consensus_level,key_points}`。归一化器把 `debate_log→log`、`consensus_level→consensusLevel`、`final_score→finalScore` 等全部映射到位。
- **smart**：返回 `answer{action_items,risk_assessment{technical_risk,...}}` + `intent` + `matched_expert` + `related_experts`。
- **algorithm**：返回 `complexity{big_o_notation,time_complexity,space_complexity}` + `feasibility{score,blockers,risks}` + `optimization_suggestions` + `recommended_experts`。面板 `complexity.bigO / .time / .space` 与 `complexityLevel(time)` 分级标签（`O(2^n)`→高风险）正确。

**结论：6/6 覆盖，6/6 curl 通过，无缺口。**

## 3. 视图二：AllianceSessionsView.vue（专家会话中心）

挂载：左 `SessionListPanel`（list/create/select）+ 中 `SessionThreadPanel`（detail/messages/similar）+ 右 `SessionMetaPanel`（update/delete/export/archive）+ `SessionStatsPanel` + `SemanticSearchPanel`。

### 端点覆盖矩阵

| # | 端点 id | wire 路径 | 方法 | UI 入口 | curl |
|---|---|---|---|---|---|
| 1 | sessionsList | /api/experts/sessions | GET | SessionListPanel 列表 + 过滤 + 分页 | ✅ |
| 2 | sessionCreate | /api/experts/sessions | POST | "新建会话"按钮 → 表单 | ✅ |
| 3 | sessionStats | /api/experts/sessions/stats | GET | SessionStatsPanel | ✅ |
| 4 | sessionDetail | /api/experts/sessions/:id | GET | 点行 selectSession → 线程 | ✅ |
| 5 | sessionUpdate | /api/experts/sessions/:id | PUT | SessionMetaPanel 保存编辑 | ✅ |
| 6 | sessionDelete | /api/experts/sessions/:id | DELETE | SessionMetaPanel 删除 | ✅ |
| 7 | sessionMessages | /api/experts/sessions/:id/messages | POST | SessionThreadPanel 发送 | ✅ |
| 8 | sessionSimilarSearch | /api/experts/sessions/:id/similar-search | POST | 线程内"字面相似检索" | ✅ |
| 9 | sessionExport | /api/experts/sessions/:id/export | GET | SessionMetaPanel 导出 | ✅ |
| 10 | sessionArchive | /api/experts/sessions/:id/archive | POST | SessionMetaPanel 归档 | ✅ |
| 11 | semanticSearch | /api/experts/semantic-search | POST | SemanticSearchPanel 全局检索 | ✅ |

### curl 实测要点（全程在一个临时会话上跑完后删除）

- create → 返回完整 `ExpertSession`（id 形如 `sess-…`）。
- append message → 只回那条 message（契约：无会话体），store 自行并入线程；role/content 缺省即 422，前端 `appendMessageProblem` 已前移校验。
- similar-search → `results[]{message,similarity_score,rank}` + `total_found`；实测 bigram Jaccard 0.154。
- export → `content`（完整会话含 messages）+ `download_url:null`（契约恒 null，前端自行序列化下载）。
- archive → `{archived_at,message_count,session_id,status:"archived"}`。
- semantic-search → 跨会话 `results[]{message,session_id,session_title,similarity_score}` + `total_sessions_scanned/total_messages_scanned`（实测扫 29 会话/45 消息）。
- delete → `{deleted:true,session_id}`；404 与已删同形。

**结论：11/11 覆盖，11/11 curl 通过，无缺口。**

## 4. 视图三：AllianceGraphView.vue（协作图谱）

挂载：`GraphCanvas`（overview）+ `GraphMetricsPanel`（stats/communities/path）+ `GraphNodeInspector`（neighbors/collaborators）+ `GraphTeamPanel`（optimal-team）+ 顶部"重建图谱"按钮。

### 端点覆盖矩阵

| # | 端点 id | wire 路径 | 方法 | UI 入口 | curl |
|---|---|---|---|---|---|
| 1 | graphOverview | /api/expert-graph | GET | onMounted loadGraph | ✅ |
| 2 | graphStats | /api/expert-graph/stats | GET | GraphMetricsPanel 指标格 | ✅ |
| 3 | graphNeighbors | /api/expert-graph/neighbors/:id | GET | 点节点 selectNode → inspector | ✅ |
| 4 | graphCollaborators | /api/expert-graph/collaborators/:id | GET | inspector 协作者数量滑块 | ✅ |
| 5 | graphPath | /api/expert-graph/path/:source/:target | GET | 起点/终点下拉 + 查询 | ✅ |
| 6 | graphCommunities | /api/expert-graph/communities | GET | GraphMetricsPanel 社区块 | ✅ |
| 7 | optimalTeam | /api/expert-graph/optimal-team | POST | GraphTeamPanel 组建团队 | ✅ |
| 8 | graphRebuild | /api/expert-graph/rebuild | POST | 顶部"重建图谱"按钮 | ✅ |

### curl 实测要点

- overview/stats/communities 正常；初始 `collaboration_edges=0`。
- neighbors 返回 3 条 domain 归属边；bad id 返回 **404**（契约行为）。
- path：两专家无直连边时返回 **200 + `found:false`**，面板据此显示"不连通"，不报错。
- optimal-team：`{required_domains,max_members}` → 返回 `team_id/members[]/coverage/team_score`。
- rebuild：幂等，`graph_version` v2→v3、edge_count 30→39；rebuild 后 `collaborators` 从空变为有 2 名协作者，与面板一致。

**结论：8/8 覆盖，8/8 curl 通过，无缺口。**

## 5. 视图四：AllianceOrchestrationView.vue（专家编排台）

挂载：任务表单（orchestrate）+ 计划生成/执行按钮 + 统计卡 + 历史表。

### 端点覆盖矩阵

| # | 端点 id | wire 路径 | 方法 | UI 入口 | curl |
|---|---|---|---|---|---|
| 1 | orchestrate | /api/experts/orchestrate | POST | "一键编排"按钮 | ✅ |
| 2 | orchPlanGenerate | /api/experts/plan/generate | POST | "生成计划"按钮 | ✅ |
| 3 | orchPlanExecute | /api/experts/plan/execute | POST | "执行该计划"按钮（生成后可点） | ✅ |
| 4 | orchStats | /api/experts/orchestration/stats | GET | onMounted + 统计卡 | ✅ |
| 5 | orchHistory | /api/experts/orchestration/history | GET | onMounted + 历史表 | ✅ |

### curl 实测要点

- orchestrate → `experts[] + plan{steps[]} + execution{status,steps_completed,duration_ms} + result{fusion…}`。
- plan/generate → `plan_id + steps[]{description,expert_id,step_type,depends_on} + status:"draft"`。
- plan/execute → `steps_executed[]{status,duration_ms,result.expert:null} + final_result + overall_status`。按契约 `ORCH_PROVENANCE`，步骤正文/置信度 0.85/耗时 10+5×序号均为后端模拟文案，拓扑与选人为真实——视图已逐字段挂来源角标。
- stats/history：写入后 `total_plans=2 / success_rate=1.0 / top_used_experts[]`；`plans_ready/plans_failed` 恒 0（契约已说明：后端无写这两个状态的代码路径）。

**结论：5/5 覆盖，5/5 curl 通过，无缺口。**

## 6. 汇总

| 视图 | 端点数 | 有 UI 入口 | curl 通过 | 遗留缺口 |
|---|---|---|---|---|
| CollabView | 6 | 6 | 6 | 无 |
| SessionsView | 11 | 11 | 11 | 无 |
| GraphView | 8 | 8 | 8 | 无 |
| OrchestrationView | 5 | 5 | 5 | 无 |
| **合计** | **30** | **30** | **30** | **0** |

### 遗留缺口 / 说明（非缺陷，记录备查）

1. **multi-consult 自动匹配可能 404**：不指定专家时按阈值 0.3 匹配，命中不到即 404。这是后端契约行为，面板侧栏已提示，非前端缺口。
2. **graph 专家间初始无协作边**：`collaboration_edges` 在 rebuild 前为 0，此时 collaborators 为空、跨专家 path 恒 `found:false`。面板对空与不可达均有文案，非缺口。
3. **orchestration 面为进程内存态**：plans/history 重启即归零（契约 `orchVolatileNote` 已说明）；本报告 curl 产生的 2 条历史记录随重启消失。
4. **executePlan 不传 step_ids**：当前走全量执行（不传即全量）。契约支持 `step_ids` 过滤，但视图未提供该复选过滤——属简化而非缺口，如需可后续加。
5. 单条 single consult 实测命中治理闸门 `blocked:true`，面板正确渲染拦截标签。

### 本轮改动文件

无。4 视图及其 store/panel/contract/normalize 均已生产级，未发现需补全的端点入口或形状错位，故未改动任何源码，未触发 contract/store/model 测试变更。

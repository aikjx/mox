# 专家联盟前端模块 · 实现态唯一权威

> **编号**：EA-DOC-FE-MODULE | **版本**：V1.4 | **权威等级**：🟢权威（前端实现态）
> **文档层级**：L3（模块实现规格）
> **最后更新**：2026-09-25（V1.4：模式→拓扑契约挂载 `contract/mode.js`，控制台 DAG 页签补"这是模板不是进度"的显式声明，X 系列 17 个变异体跑满）
> **单源声明**：本文档是 `frontend-ui/src/modules/expert-alliance/` 前端联盟模块的唯一权威说明。端点表、枚举值、阶段序列的**数据权威在代码里**（`contract/*.js` 与其跨语言测试），本文只描述结构、裁决规则与已知后端事实，不复制数值清单。
> **后端侧对偶文档**：`docs/expert-alliance/CURRENT-ARCHITECTURE.md`（Rust 实现态）。二者冲突时以后端为准，前端必须改。

---

## 1. 为什么存在这个模块

联盟前端此前没有单一实现层：`frontend-ui/src/stores/alliance.store.js`（628 行）零消费者且指向不存在的端点；辩论/多专家咨询/智能路由三套流程在 `views/workspace/` 与 `views/expert/` 各抄一份；阶段表在 3 处重复；`views/expert/ExpertConfigView.vue` 单文件 5414 行。

本模块给出**可验证的替代实现**：契约来自 Rust 权威源并由测试强制，UI 只消费归一化后的模型。存量页面按 §7 迁移后退役。

## 2. 分层与文件事实

```
frontend-ui/src/modules/
├─ index.js                        # 模块登记入口（新模块在此追加一行）
├─ _kernel/                        # 与业务无关的内核
│  ├─ envelope.js                  # 响应信封归一（83 行）
│  └─ module-registry.js           # defineModule / collectRoutes / collectNav（78 行）
└─ expert-alliance/
   ├─ contract/{phases,enums,endpoints,collab,dispatcher,graph,sessions,registry,orchestration,mode}.js  # 阶段、枚举、端点＋全维覆盖台账、协作、调度配置与实跑、图谱、会话、注册写面、编排（步骤表 / 常量清单 / 逐字段来源）、协作模式→展示态拓扑契约（61 / 358 / 194 / 307 / 308 / 189 / 445 / 265 / 404 / 92 行）
   ├─ model/normalize.js                     # 后端字段 → 前端模型（1285 行，含图谱 8 个与会话 9 个 handler 的出参、两个动作面的两种响应形态、写面三条响应的三种不同形状、编排五种出参的摊平）
   ├─ model/display.js                       # 时间与数值的展示口径（41 行，面板共用，勿在 .vue 里各写一份 toLocaleString）
   ├─ model/layout.js                        # 图谱确定性布局：环簇坐标/度数半径（123 行，无第三方依赖）
   ├─ model/rank.js                          # 榜单口径：只排真实指标（102 行）
   ├─ model/dag.js                           # DAG 分层与依赖记账：drawn/dangling/backEdges/edgeDelta 四笔账同源（69 行）
   ├─ api/alliance.api.js                    # 唯一取数入口：collaborate() 单入口 + 调度配置读写与实跑 + 图谱 8 法 + 会话 11 法 + 注册写 3 法 + 编排 5 法（410 行）
   ├─ store/alliance-console.store.js        # 控制台状态机：运行时/任务/详情/日志 + 调度配置草稿与差异 + 实跑结果与候选 + 标记完成 + 调度状态与负载重置（355 行）
   ├─ store/alliance-experts.store.js        # 广场状态机：筛选/收藏/预约/咨询/平台统计/能力目录/派生指标 + 注册/编辑/停用三条写面（303 行）
   ├─ store/alliance-collab.store.js         # 六模式协作状态机：模式/输入/控件/约束/结果/历史（142 行）
   ├─ store/alliance-graph.store.js          # 图谱状态机：图/统计/社区/选中节点邻域/路径/组队/重建（216 行）
   ├─ store/alliance-sessions.store.js       # 会话状态机：列表/详情/统计/检索四区 + 草稿与差分（390 行）
   ├─ store/alliance-orch.store.js           # 编排台状态机：表单 + 三个动作各自的最近结果 + 统计/历史两读数；来源角标与恒 0 说明一律转自契约，不在此重算（218 行）
   ├─ components/ExpertCard.vue              # 专家卡片（279 行）
   ├─ components/ExpertBookingPanel.vue      # 我的预约面板（144 行）
   ├─ components/ExpertRankBoard.vue         # 专家排行榜（107 行）
   ├─ components/ExpertCapabilityMatrix.vue  # 能力目录页签：只沿用后端 id 升序，点一行落到领域过滤（130 行）
   ├─ components/ExpertRegistryForm.vue      # 注册 / 编辑表单：控件由 EXPERT_REGISTER_FIELDS 生成，草稿与校验理由自持（149 行）
   ├─ components/ExpertCollabPanel.vue       # 协作工作台面板，六模式唯一渲染体（398 行）
   ├─ components/GraphCanvas.vue             # 图谱画布：SVG + viewBox，节点可点选（114 行）
   ├─ components/GraphNodeInspector.vue      # 节点详情：属性 + 邻居 + 协作者（147 行）
   ├─ components/GraphMetricsPanel.vue       # 统计 / 社区 / 路径查询（141 行）
   ├─ components/GraphTeamPanel.vue          # 最优团队表单，控件由契约字段生成（160 行）
   ├─ components/SessionListPanel.vue        # 会话列表：过滤 / 分页 / 新建（203 行）
   ├─ components/SessionThreadPanel.vue      # 会话线程：消息渲染 + 追加 + 会话内字面检索（260 行）
   ├─ components/SessionMetaPanel.vue        # 标题/状态/标签/metadata 差分编辑 + 归档/删除/导出（179 行）
   ├─ components/SessionStatsPanel.vue       # 会话统计 KPI 与 top_experts（183 行）
   ├─ components/SemanticSearchPanel.vue     # 全域字面检索：命中行可跳到所属会话（111 行）
   ├─ views/AllianceConsoleView.vue          # 联盟控制台（1482 行，含调度配置表单、分发实跑、标记完成与调度状态/负载重置，DAG 页签顶部挂模式拓扑说明条）
   ├─ views/AllianceCollabView.vue           # 智能协作工作台页（91 行，只供外壳与候选名单）
   ├─ views/AllianceExpertsView.vue          # 联盟专家广场（656 行，含注册 / 编辑 / 停用三条写面与其后果清单）
   ├─ views/AllianceGraphView.vue            # 专家协作图谱页（133 行，画布 + 检视 + 统计 + 组队）
   ├─ views/AllianceSessionsView.vue         # 专家会话页（156 行，列表 + 线程 + 元信息 + 统计 + 全域检索）
   ├─ views/AllianceOrchestrationView.vue    # 专家编排台（347 行：表单 + 依赖链 + 编排结果 + 计划执行 + 进程内统计与历史 + 常量图例，逐字段带来源角标）
   ├─ index.js                               # defineModule 声明路由与导航（6 条路由 / 6 个导航项，93 行）
   └─ *.test.js（+ src/modules/wiring.test.js）# 模块内 533 例 / 18 文件：跨语言契约 99 / 会话契约 37 / 调度配置与状态契约 45 / 编排契约 48
                                               # / 图谱契约 27 / 注册写面契约 32 / 模式拓扑契约 18 / API 58 / 广场 store 34 / 会话 store 30
                                               # / 广场组件 21 / 协作 store 16 / 协作面板 17 / 图谱 store 15 / 榜单 10 / DAG 布局 11 / 样式契约 9 / 登记 6
                                               # （src/modules 共 539 例 / 19 文件，全应用 616 例 / 26 文件；装配顺序守卫 6 例即 wiring.test.js）
```

依赖方向单向向下：`views → store → api → model/contract → _kernel`。`contract/` 不 import 任何 UI；`api/` 不做 UI 决策、不吞错误。

## 3. 归一化（本模块的核心约束）

"归一化"在本模块不是口号，而是 `contract/contract.test.js`、`contract/dispatcher.test.js`、`contract/graph.test.js`、`contract/sessions.test.js`、`contract/mode.test.js` 这一批**读 Rust 源码做断言**的测试（例数与文件数只以 §8 上方代码块为口径，本段不复述）：

| 契约 | 前端位置 | 后端权威源 |
|------|----------|-----------|
| 7 阶段管线序列 | `contract/phases.js` `PHASE_IDS` | `platform/shared/mox-unified-contract` 的 `PHASE_NAMES` |
| 7 类审计事件 | `contract/phases.js` `AUDIT_EVENTS_7` | 同上（`ALLIANCE_START` 起始，**无** `SYNTHESIZE_DONE`） |
| 任务/节点状态、优先级、融合策略、等级 | `contract/enums.js` | `platform/domains/alliance/svc/.../alliance.rs`、`mox-alliance-common-proto/src/types.rs` |
| 模式双名（传输名 / 展示名） | `MODE_WIRE` / `MODE_DISPLAY` | `mox-alliance-common-proto/src/naming.rs` 的 `mode_serde` / `mode_display` 分支，按声明顺序全量比对 |
| 协作模式 → 展示态拓扑 | `contract/mode.js` `MODE_TOPOLOGY` / `MODE_CONTRACT_NODES` / `modePendingCount` / `modeTopologyNote` | SDK `alliance.rs` 的 `build_dag_for_task`（`:441-515`）：按大括号计数取七个 `AllianceMode::X =>` 分支、按序取每分支的 `n("名", "expert-…", NodeExecStatus::…)` 调用，**逐模式**比对节点数、按序节点名、Running 个数，并要求首节点恒 `需求分析/completed`、末节点恒 `融合输出/expert-fusion`。表里的键是 `MODE_WIRE` 值而 Rust 分支是 PascalCase，所以查表用 `modeWireOf()` 兼容两条线（出参给 `mode_display`，请求发 `mode_serde`）。引用坐标不做装饰性检查：`alliance.rs:439-440/:441-515`、`planner.rs:332-411`、`types.rs:133-148` 都要按大括号配对重算结束行后相等——写下 `planner.rs:332-401` 时正是这条把它抓成 411 |
| dynamic 的分支缺席 | `contract/mode.js` `DYNAMIC_BRANCH`（`onWire: false`） | 三处正面对手 + 一处零命中：`planner.rs:332` 起确有 `fn generate_dynamic_plan` 与 `routes.push(PlanDynamicRoute {`、`types.rs:355` 的 struct 确有 `true_branch/false_branch`、`dag_engine.rs` 的 `build_dynamic_routes(…) -> Option<…>` 在 `plan.dynamic_routes` 为空时直接 `return None`，而 SDK `alliance.rs` 全文对 `dynamic_routes` **零命中** ⇒ 网关从不把分支发出去，界面只能声明"这条模式允许分支"，答不了"这一次走了哪条" |
| 质量门限 A/B/C/D | `GATE_THRESHOLDS`、`gradeOf` | `mox-unified-contract/src/quality.rs`（阈值从源码解析，含边界值） |
| 端点路径 | `contract/endpoints.js` `ENDPOINTS` | `docs/API-REGISTRY.md` 中的反引号字面量 |
| 专家域取值（在线态/类型/计费/认证/预约态） | `contract/enums.js` `EXPERT_AVAILABILITY` 等 | `alliance/experts_common.rs` 字段文档注释、`experts_ext.rs` 的 `status` 字面量 |
| 列表查询参数与排序值 | `EXPERT_QUERY_KEYS` / `EXPERT_SORT` / `EXPERT_SORT_LABELS` | `experts_registry.rs` 的 `list_experts`（`params.get(...)` 与 `"rating" =>` 分支），文案键集合与后端可识别值**双向等集** |
| 可取消 / 可即时咨询判据 | `canCancelBooking` / `isConsultable` | `experts_ext.rs` 取消前置判断、`experts_registry.rs` 仅 `online` 建会话的判断 |
| 计费单位 | `pricingText()` | `experts_common.rs` 的 `hourly_rate_cents: u32`（注释「每小时费率（分）」）与 `default_pricing() = "free"` |
| 榜单可排字段 | `model/rank.js` `RANK_BOARDS` / `buildBoard` | `experts_common.rs` `ExpertMetrics` 字段集（与 `normExpert().metrics` 键**双向等集**）+ `ExpertDescriptor.created_at`；同时断言 `good_rate`/`month_growth`/`response_time` 等历史假指标在后端查无此名 |
| 平台统计 KPI | `normExpertStats()` + `expertStatsCells()` | `experts_registry.rs` `experts_stats_real` 的 `ok(json!({...}))` 响应键集（等集比对）；`expert_count`/`consult_count`/`good_rate`/`avg_response` 这类历史假键被断言**不存在** |
| 协作模式 → 端点 | `contract/collab.js` `COLLAB_MODES[].path` | `experts_collaboration.rs` 的 6 条 `route(...)` 注册字面量，逐模式比对 |
| 协作请求体 | `COLLAB_MODES[].wires` + `collabBody()` | 同文件各 `*Body` 结构体字段集 **∩** handler 里真正出现的 `body.<field>`；且 `Object.keys(collabBody(...))` 与该模式声明的 wires **等集**（既不多发死键，也不少发必填键） |
| 协作数值边界 | `COLLAB_MODES[].controls`（`max_experts` 1–20 / 1–10、`rounds` 1–10） | handler 里的 `clamp` / `min(.., max(..))` 字面量，逐值比对；越界值由前端夹到边界后再发 |
| 智能路由的嵌套约束 | `COLLAB_MODES[route].constraintFields`（`min_rating` 0–5、`max_response_time` 1–240、`require_online`） | `route_query` 里逐名比对手写读取 `c.get("min_rating")` / `c.get("max_response_time")` / `c.get("require_online")`；`neutral` 取后端 `unwrap_or` 缺省（0.0 / 无上限 / false），**等于缺省的子键不进请求体** |
| 调度器配置 | `contract/dispatcher.js` `DISPATCH_STRATEGY` / `DISPATCH_CONFIG_FIELDS`（含 `checked` 与 `default`） | `experts_dispatcher.rs` 的 `let valid = [...]`、`UpdateConfigBody` 字段集、`if let Some(..) = body.x` 合并分支与 `"<键> must be A-B"` 的 400 文案；默认值取 `experts_common.rs` 里 `#[serde(default = "default_x")]` 指向的函数体（按行回看属性，不跨字段匹配） |
| 后端接受但本模块**故意不发**的键 | `COLLAB_LAZY_FIELDS`（`ConsultBody.context`/`priority`、`DebateBody.stance`、`IntelligentConsultBody.history`） | 断言这些键在对应 handler 里没有 `body.x` 读取，并逐条写明不发的理由 |
| 协作出参 | `model/normalize.js` 的 `normRoute`/`normMultiConsult`/`normDebate`/`normAlgorithmAnalysis` 等 | handler 末尾 `ok(json!(...))` 的**顶层**键集（等集），并断言前端归一化读取的键全在其中 |
| 图谱端点面（8 个） | `contract/endpoints.js` 的 `graph*` / `optimalTeam` | `alliance/experts_graph.rs` 的 `.route("…", get|post(handler))` 注册集合 **∩** `actuator.rs` 的 `experts.graph.*` id **∩** `docs/API-REGISTRY.md` 登记行，三方等集；`/api/expert-graph/overview`（未路由）留在 `FORBIDDEN_ENDPOINTS` |
| 图的真实构成 | `GRAPH_NODE_TYPE` / `GRAPH_EDGE_TYPES`（权重口径写进 `weightHint`） | `experts_common.rs` 的 `build_graph_from_registry`：节点只有 `expert` / `domain` 两类、域节点 id 为 `format!("domain-{}", …)`、协作边**仅** `if similarity > 0.1` 才建、节点属性只有 `title/domains/avg_rating/status`（**没有 skills**） |
| 组队请求体 | `OPTIMAL_TEAM_FIELDS`（`mounted`）+ `optimalTeamBody()` / `optimalTeamProblem()` | `OptimalTeamBody` 字段集与 handler 里真正出现的 `body.<key>` **交集**决定 `mounted`；`constraints` 被 struct 收下却从不读取 → `mounted: false` 且永不发送；数值等于 `unwrap_or` 缺省（5 / 4.0）时不进请求体；`goal` 与显式需求互斥（后端只在两者皆空时读它） |
| 图谱出参 | `normGraph` / `normGraphStats` / `normNeighbors` / `normCollaborators` / `normGraphPath` / `normCommunities` / `normGraphRebuild` / `normOptimalTeam` | 每个 handler 里**所有** `json!({...})` 的 depth-1 键并集（`jsonFaces`：只取第一个 `json!` 会把内层元素对象当顶层，`compute_graph_stats` 等四个 handler 都会误判）；**双向**核对：后端给的键必须被读到（能力不被吞），前端读的键必须在 handler 或 builder 里以字符串字面量出现过（不臆造字段） |
| 协作者分页 | `COLLABORATOR_LIMIT_DEFAULT` / `COLLABORATOR_LIMITS` / `collaboratorQuery()` | handler 的 `params.get("limit").and_then(\|v\| v.parse().ok()).unwrap_or(10)`：非正整数与等于缺省一律不发；截断事实取响应的 `total_collaborators` |
| DAG 分层与计数 | `model/dag.js` `layoutDag()`（`layers` / `drawn` / `dangling` / `backEdges` / `edgeDelta` / `tally`） | 分层用的节点集与页脚报的依赖数**必须同一次算出来**，所以算法不许住在 SFC 里（`contract.test.js` 断言视图不出现 `const buckets = new Map()` 且 `dag.edges.length` 不进界面）。后端可核对的部分逐字解析：`get_task_dag` 三处 `json!` 的键集（节点 12 键、边 `source/target/label`、stats 六键）与 `normNode`/`normDag` 的读取集合对齐，`node_status_str` 的六个出口值要求前端标签与节点样式**逐个都在**，`node_stats` 的第 6 格必须是 `skipped + cancelled`（后端折叠，前端就不显示那个桶） |
| 图谱布局 | `model/layout.js`（环簇坐标、度数半径、`pathChainText`） | 后端**不返回坐标**，故此处无对齐对象，改为自证：两次 `graphLayout` 坐标逐点相同、`nodeRadius` 随度数单调且封顶、无 id 的条目不进画布。刻意不引 `3d-force-graph`（存量构建产物里那个 1.3 MB chunk），也不引入随机初值 —— 那会让「同一份图同一张画布」无法快照测试 |
| 会话端点面（11 条 (path, method)） | `contract/endpoints.js` 的 `session*` / `semanticSearch` | `alliance/experts_session.rs` 的 `build_experts_session_router`（8 条 `.route()` 注册 = 11 个动词面）**∩** `actuator.rs` 的 `experts.session.*` id **∩** `docs/API-REGISTRY.md` 登记行，三方等集；`ANY` 行只作动词通配，故 detail/update/delete 共用 `experts.session.detail`；另断言 `sessions/stats` 注册在 `sessions/:id` **之前**（否则 "stats" 被当成会话 id） |
| 会话请求体 | `createSessionBody` / `sessionUpdatePatch` / `appendMessageBody` / `similarSearchBody` / `semanticSearchBody` | `CreateSessionBody` 七字段全 `Option + #[serde(default)]`（空 body 合法）；`UpdateSessionBody` **只有** 5 个键——换专家阵容/换类型/换发起人没有端点，发给它们只被丢弃；`AppendMessageBody` 的 `role`/`content` **无** `serde(default)`，缺失即 axum 422；与后端 `unwrap_or` 同值的键不进请求体（`session_type=single`、`msg_type=text`、`top_k=5/10`、`min_score=0.1`）；全域检索 body 断言**只有 3 个键**，`min_score` 那里根本不存在 |
| 会话列表查询与分页 | `SESSION_QUERY_KEYS` / `sessionListQuery` / `sessionPageCount` / `SESSION_PAGE` | `list_sessions` 的 `params.get(...)` 名集与前端键表**双向等集**（`limit` 只是 `page_size` 的别名，故刻意不在表内）；`parse_pagination`（`experts_common.rs:875`）的 `.max(1)` / `unwrap_or(20)` / `.clamp(1,200)` 字面量；`search` 只命中 `title`/`topic` 的小写包含、**不含消息正文**；排序固定 `created_at` 降序，源码里没有 `sort` 参数，界面也就不摆排序控件 |
| 会话枚举「后端不校验」 | `SESSION_TYPES` / `SESSION_STATUSES` / `SESSION_STATUS_COUNTED` / `MESSAGE_ROLES` / `MSG_TYPES` | 取值清单来自 `ExpertSession` / `SessionMessage` 的**字段文档注释**（`会话类型：single / multi / debate / enterprise` 等），不是前端挑的；三个 `default_*` 函数体与 handler 的 `unwrap_or_else` 共同指向前端缺省；`session_stats` 的 `match s.status` 只有三档且 `_ => {}`——写出第四个值不报错，但**在统计里隐形**，界面必须这样说明 |
| metadata 只能合并、不能删 | `sessionUpdatePatch()` 的 `unremovableMetadataKeys` + `metaAsObject` / `sameMetaValue` | `update_session:390-394` 只有 `insert`，没有删除键的语义；库里是数字/布尔而表单只能收集文本，`3` 与 `'3'` 在输入框里长得一样，故按文本等价判定"未改动"，否则每次保存都会把 `3` 悄悄改写成 `"3"` |
| 「相似检索」不是语义模型 | `similarSearchBody` / `SIMILAR_SEARCH_DEFAULTS` / `similarTruncated` | `text_similarity:827` 是**字符 bigram Jaccard**（空串得 0，单字相等才得 1）；会话内按 `score >= min_score` 收录、全域只要求 `score > 0.0`；`rank` 从 1 起编号，`total_found` 是截断前命中数——两者不等时界面要说出"还有没展示的" |
| 会话出参的两种形态 | `normSession` / `normSessionList` / `normSessionStats` / `normSimilarSearch` / `normSemanticSearch` / `normSessionExport` / `normSessionArchive` / `normSessionDelete` / `normSessionMessage` | `session_to_list_view:113` 的**投影 13 键**（含 `message_count`、**不含** `messages`）vs `get_session` 的 `ok(json!(session))`（整份结构体，带 `messages` 而无 `message_count`）——同一份归一化器吃下两种形态，`messageCount` 恒有值；`rating` 是 `Option<u8>`，缺失/null 归 `null` 才能区分"没评分"与"0 分"；`session_type_distribution` 后端是 map，归一成 `[{type,count}]` 才不会漏类型；`export` 的 `download_url` **恒 null**（`:586` 写死），归一化保留 `null` 就是不让界面画假链接；`delete` 只回 `{deleted, session_id}`，与 404 同形 |
| 长会话的截断是前端的责任 | `THREAD_RENDER_LIMIT` / `threadWindow()` | `get_session:347` 一次给全量 `messages`，后端没有分页，故"只渲染最近 N 条"这件事必须写在一处并向用户说明，而不是散落在视图里的 magic 数 |
| 同一端点两条分支的两种形态 | `normToggleDone()` 的 `branch` / `direction` | 本地 `alliance.rs:1646-1657` 给 `{task_id, previous_status, current_status, toggled, completed_at, message}`，远程 `alliance_remote.rs:543-552` 只给 `{success, message}` 且 `task_id` 落在与 `data` 同级的 `params` 里。断言归一化 `s.<key>` 读取集合**恰等于两分支键集之并**（L3：多读一个键即红），方向只由 `toggled`/`success` 判、缺证据落 `unknown`（L1/L2） |
| 分发实跑请求体与结论口径 | `dispatchRunBody` / `dispatchRunProblem` / `dispatchRunFindings` | `DispatchBody` 四字段 **∩** handler 的 `body.<key>` 读取集合：`constraints` 收了从不读 → 永不发送（L6/L12）；`task_type`/`input` 无 `#[serde(default)]` 故恒发（缺一个键整段 JSON 被 axum 拒收，且拒收体不是信封）；`strategy_used` 取值集合取自 `dispatch_task` 的 6 处 `"…".into()` 字面量、`best_match(fallback)` 不在可配置枚举里（L9）；`intelligent_matching=false` 时硬编码的 `0.5` 从源码解析并与前端判据的数字**比对而非复制**（L8）；空 `input` 必拦的依据是 `compute_match_score` 里 `!q.is_empty()` 全文**只出现一次**（L7） |
| 注册请求体的键与形态 | `contract/registry.js` `EXPERT_REGISTER_FIELDS` / `EXPERT_UNMOUNTED_FIELDS` / `EXPERT_ALIAS_FIELDS` / `registerBody()` / `expertFormDraft()` | `experts_registry.rs` 的 `merge_expert_from_value`：认识的键由 `body.get("…")` / `av.get` / `mt.get` 现场解析，挂载 / 不挂载 / 别名三类逐项表态。**双向核对**——前端发的键后端必须认得（否则静默丢弃），后端 merge 新增的键前端必须先表态（否则本模块判红） |
| 写面数值边界与静默截断 | `EXPERT_U32_MAX` / `EXPERT_PROFICIENCY_MAX` / `expertFieldProblem()` / `expertDraftProblem()` | `name` 空 → 400 `expert name is required`；`hourly_rate_cents` 按 `u64` 读再 `as u32` 存，超界**截成另一个数**而不是报错；`proficiency` 是 `u8`，越界的整条 capability 被静默丢弃。三条都不是前端偏好，是把源码里的类型抄成边界值 |
| 合并式 patch | `expertPatch()` | `update_expert` 走同一个 merge，只覆盖请求里出现的键 ⇒ 前端只发改动过的键；`Some(exp) if exp.enabled` ⇒ 停用过的专家 PUT 必 404；零改动不发 PUT，否则白盖一次 `updated_at` |
| 软删的后果 | `deleteConsequences()` / `deleteResultText()` | `delete_expert` 写 `exp.enabled = false` + `metadata.deleted_at` + `save_registry`；**全 alliance 目录没有任何一处把 `enabled` 写回 `true`**（唯一的 `enabled: true` 是构造默认值，即重新注册一位新专家）⇒ 停用单向不可逆。`enabled` 的过滤点散在 registry / dispatcher / collaboration / orchestration / graph / DB 十余处 ⇒ "消失"是全域事件，文案必须逐面点出来 |
| 写面身份 | `EXPERT_WRITE_IDENTITY` | `AuthConfig::default()` 的 `enabled: true`（`config.rs:37`）+ `modules.rs:186` 给业务路由统一挂 `auth_middleware` ⇒ 写请求必带身份；而联盟域 handler 签名里没有任何身份抽取器 ⇒ **只认证、不授权**。登录与否归外壳 `router/index.js:74` 的守卫，模块页不再造一套置灰（那等于替后端编造角色判定） |
| 三条写响应的三种形状 | `model/normalize.js` `normExpertWrite()` | POST 回 `{expert,id,created}`、PUT 回 `{expert,updated}`（**不回 id**）、DELETE 回 `{id,deleted,soft_delete,message}`（**不回 expert**）——一份归一化器吃三种形态，缺字段就落 `null` 让界面说"后端未确认"，而不是假装成功 |

**新增/改动契约必须同时改代码与后端**，否则 `npx vitest run src/modules` 直接失败。`FORBIDDEN_ENDPOINTS` 另存一组"曾在旧代码里出现但后端根本不存在"的路径，测试保证它们不再被任何模块源码引用。

### 3.1 样式契约（`style.test.js`，9 例）

视觉层同样按单源处理，权威源是 `src/styles/global.css` 的 `:root` 与 `src/styles/themes/*.css`：

- 模块内每个 `.vue` 的 `<style>` 里出现的 `var(--x)`，**必须**在主题源里有 `--x:` 定义。存量代码里的 `var(--transition)` 从未被定义，浏览器静默丢弃整条声明，本模块改用 `var(--dur-2) var(--ease)`。
- 不出现裸 `#hex` / `rgb()` / `hsl()`（含 `var(--token, #fallback)` 这种写法），圆角不留 `px` 字面值。
- 每个 `.vue` 的**根类名在本模块内唯一**，且任意两个 `.vue` 不共享块级类名。这条守卫来自一次真实故障：控制台与协作面板都用根类名 `ac`，页面内 `querySelector('.ac')` 命中了控制台的根节点，把协作结果读成了控制器的告警。修饰符类（`.a.is-x` 里的 `.is-x`）允许重名，因为必须与自有块级类同现才命中。

### 3.2 全维覆盖台账（`UNMOUNTED_ROUTES`，"没做"也要可审计）

"全维重新开发"若只写在文档里就是一句口号：后端两域（`experts.*` + `alliance.*`）在 `docs/API-REGISTRY.md` 里共有 **74 行 ready 路由**，模块挂了 **66 行**（71 个端点条目落在 66 个行 id 上：`experts.session.detail` 一行担 detail/update/delete 三个动词面，`experts.registry.detail` 的 `ANY` 行同时是 PUT/DELETE 的溯源行，`alliance.tasks.list` 同时是创建任务的溯源行），剩下 **8 行全部是 `rejected`**（backlog 已随编排台挂载归零），逐条写进 `contract/endpoints.js` 的 `UNMOUNTED_ROUTES`，每条给出定性与**指向后端源码行号**的理由。`contract.test.js` 把这张表锁住：

| 断言 | 红的情形 |
|------|---------|
| 两域注册表行非空且台账 id 唯一 | 解析正则空转（`ROWS.size > 60` 是最防空转的哨兵）、台账写重 id |
| 每个 ready 行要么被挂载、要么在台账里定性 | 后端新增路由没人登记；或从台账里抹掉一条假装它不存在（变异体 C5） |
| 挂载集必须落在两域注册表内 | 端点挂到一个查无此行的 id（变异体 C6） |
| 台账只收真实行，理由须指向后端源码位置 | 定性写成第三种值（C1）、理由删成两个字（C2）、理由漂亮但没有 `.rs:` 行号（C3） |
| 已挂载端点的方法与路径逐字对齐注册表行 | 注册端点的方法漂成 GET（C4）。`ANY` 行放行任意方法，其余必须逐字相同 |
| 待办面数量只减不增 | `backlog` 长出任何一条（编排台挂载后它必须是空集：要么先挂载，要么举证它不该挂）；覆盖率跌破 **0.89**（当前 66/74 = 0.8919） |

搬出台账的是六条行、五个面（两个 reset 行合起来是一面），它们都已挂载，守卫与台账一样指名变异体（K / L / D 系列，见 §8）：

- **`experts.registry.capabilities`** → 广场第三个页签「能力目录」（`components/ExpertCapabilityMatrix.vue`）。后端 `list_capabilities`（`experts_registry.rs:436-478`）只统计 `enabled` 专家、按 `capability id` 升序、同名能力合并后给 `expert_count` 与 `avg_proficiency`；前端**保持后端次序**（K1：本地按人数重排即判红）。挂载时撞出的第一条真实约束是：`list_experts` 只读 `domain`/`skill`/`status`/`expert_type`/`search`/`sort`（`experts_registry.rs:249-254`），**不读 `capability_id`**，所以点一行只能落到「领域」这一层服务端过滤，面板的说明文字就把这句话写死在界面上（K5：改口称支持精筛即判红）。
- **`experts.registry.detail_metrics`** → 专家详情抽屉的「派生指标（后端计算）」三格。`expert_metrics`（`:559-597`）给 `rank_percentile`/`load_ratio`/`efficiency_score`，权重写死 `40/30/30`（`:583-585`）；前端只做单位换算，**不重算**（K2：把 `* 0.4` 搬进 `enums.js` 即被源码守卫判红）。两个易错点也各自钉住：`Some(e) if e.enabled` 意味着**停用专家也返回 404**，store 不把它折叠成零值（K3）；抽屉连点两位专家时按序号丢弃迟到响应（K4）。
- **`alliance.tasks.toggle_done`** → 控制台任务动作行的「标记完成 / 重新打开」（`views/AllianceConsoleView.vue`）。这一面的分量全在两条分支的形状差上：本地分支（`alliance.rs:1596-1665`）把 `Completed ⇄ Running` 的结论写在 `toggled` 里，顺带将该任务**非 failed / 非 cancelled 的节点整批置 `Completed`**、`progress = 1.0` 并 `tasks.save` 落盘（`:1620-1642`）——所以 store 不能只刷任务，节点不重取界面就停在旧节点色上（L4）；远程分支（`alliance_remote.rs:543-552`）只回 `{success, message}`，`task_id` 还在与 `data` 同级的 `params` 里，剥完信封即不存在，于是 `taskId` 允许为空串、方向只能落 `unknown`（L1/L2/L3）。两条分支的可逆性也不对称：本地 `toggled:false` 就是重开，而远程已完成的任务经网关重开必回 **409**（`alliance_remote.rs:577-582`），所以"远程 + 已完成"时按钮直接 disabled 并把原因写进 `title`（L11），不是等一次红弹窗；请求在途时用户切了任务就不回填详情（L5）。
- **`experts.dispatch.dispatch`** → 控制台调度配置卡里的「分发实跑」。配置面此前只能证明"我 PUT 了什么"，实跑是唯一能证明"策略真的生效"的面：后端各分支自己写回 `strategy_used`（`experts_dispatcher.rs:245,262,284,301,309,320`），前端不改写它、也不把它"归一"成配置值，两边同值才敢说一致（L9 专测有人把 `best_match(fallback)` 当成策略生效）。三条硬约束各自钉住一处后端事实：`constraints` 被 `DispatchBody` 收下却从不读取（`:85-92` 对照 `:559-560`）→ 界面无此控件、请求体无此键（L6/L12）；`input` 为空必拦，因为 `compute_match_score` 切词后**不过滤空 token**（`experts_common.rs:774,781,789`，`!q.is_empty()` 全文只在 bio 分支 `:797` 出现一次），空串会让「领域匹配」对每位候选判满分，那份分数与需求无关（L7）；全员 `match_score` 恰为 `0.5` 是 `intelligent_matching=false` 的指纹而非巧合（`experts_dispatcher.rs:171-175`），判据里的数字与源码**比对而非复制**（L8）。另两点关乎诚实：实跑不碰任何专家的 `current_load`（`dispatch_task` 全程只读注册表，改这个字段的只有两个 reset handler `:788`、`:821`），一次失败也不清空上一次结果——`dispatch_id` 与 `created_at` 让它能自证是哪一次的证据（L10）。
- **`experts.dispatch.reset` + `experts.dispatch.reset_all`** → 控制台的「调度状态与负载重置」卡（`views/AllianceConsoleView.vue` 的 `.acks-card`）。这两行是台账里唯一的破坏性面，接法必须和"读数侧"同批：`experts.dispatch.status` 此前只到 api 层（`store/views` 零消费），没有它「重置成功」就只剩后端自报的一个布尔，界面无法自证——所以卡片先摆真实读数（五个 KPI 格 + 熔断列表 + `expert_loads` 表），两个动作只挂在读数之上，且**没拿到读数时两个入口都 disabled**。五处后端事实各自钉住一条界面措辞并由 **D1–D19** 守（§8）：① `FAILURE_COUNTS`（`experts_dispatcher.rs:42`）全仓只有读侧与 reset 自己的 remove/clear，没有任何生产 insert ⇒ `circuit_breakers[]` 恒空，空列表由 `breakerEmptyNote` 说成"没有数据"而不是"所有专家服务正常"（D5）；② 两个 reset **都不调 `save_registry`** 而注册表是 SQLite 支撑（`experts_db.rs:225-264`）⇒ 归零只活在内存里，重启按最后一次落库快照恢复，而重置后的第一次咨询又会把"已归零"顺带写进库（`experts_registry.rs:806-810`）——二次确认清单五条把这条时序逐条点出，每条带 `:行号`（D16 专测抹掉坐标）；③ 两者请求体要求相反：`reset_expert` 签名带 `Json<ResetBody>` ⇒ 即使 `reason` 留空也必发 `{}`（空体被 axum 拒成非信封体，页面拿不到可读原因），`reset_all` 没有 body 提取器 ⇒ 不发体（D7/D11/D12）；④ `reset/:id` 对查无此 id 同样回 `reset:true`（无 404，`previous_load` 走 `unwrap_or(0)`）⇒ 回执文案不得长成存在性断言（D17），被重置人数只能引后端的 `reset_count`（D18）；⑤ 重置之后一律**重取状态**，绝不在本地把 `currentLoad` 抹成 0——那会让"我点了"看起来像"后端改了"（D14），失败时上一次读数与回执原地保留，各自带着自己的 `ts`/`reset_at` 自证是哪一次的结果（D13）。全量重置另加一道手动闸：`后端只认证不授权`（§2 身份行），没有角色判定可依赖，故必须输入 `RESET-ALL` 才放行（D8）。

- **`experts.orch.orchestrate` + `plan_generate` + `plan_execute` + `stats` + `history`** → 新一页「专家编排台」（`views/AllianceOrchestrationView.vue`，路由 `/alliance/orchestration`）。台账里最后五条 backlog 一次结清，换来的不是"多一个页面"而是**一面对"真与假同屏"的逐字段记账**：这一面拓扑与统计是真的（`topological_sort` 是带环检测的 Kahn、`compute_match_score` 读真实注册表），步骤内容却全部出自查表与字面量，所以既不能整面贴"模拟"（低估真实部分）也不能不贴（拿样板文冒充专家结论）。落法是 `contract/orchestration.js` 的 `ORCH_PROVENANCE` 四档（real / simulated / literal / wallclock）+ 约 25 条字段路径 + `ORCH_SIMULATED` 九条常量（每条带 `:行号` 与它在 Rust 里的原文字面量，测试逐条命中该行才绿——后端换成真实实现时先红，界面文案随之收）；界面每个带角标的字段路径必须在来源清单里登记过，未登记即判红（Q14：角标会静默降级成"真实计算"）。另外四条硬事实各自钉住一处措辞：**恒为 0 的两把统计**（`plans_ready`/`plans_failed` 没有任何写入路径，`plan.status` 全文只被赋三次值 :203/:469/:511，这条 0 不是"没有失败"而是"没有能记录失败的代码"）；**成环失败是 HTTP 200 + body 里的 `status:"failed"`**（:452-463 由 :766 原样 `ok(result)`，按状态码判成败会把一次失败计划读成成功，且该分支不写回 plan.status，所以 stats 永远看不见它）；**历史分页是全域唯一不走 `parse_pagination` 的一处**（:936-937 直接 parse、无 1..=200 夹取，`page_size=0` 会得到"本页空而 total 非零"的空页陷阱，前端自设上限并把 200/20 两个数**比对** `experts_common.rs:880` 与 handler 的 `unwrap_or` 而非复制）；**步骤表只有四张专用表 + 一张 6 步兜底表**（:101-139，后端默认 `general` 就落兜底，所以界面不得写"已按 X 类型定制流程"，兜底提示的步数取自契约常量，界面自己写死数字即判红 Q1）。**依赖恒为单链**（:174-177 每步只依赖上一步）⇒ 本面正常输入下走不到成环分支，能走到说明计划来自别处，这句话写在页面而不是藏在注释里。`orchestration/plugins` 那条仍是 `rejected`（六条硬编码数组、`webhook_url`/`retry_count` 无人读取 :839-920）。Q1–Q16 十六个变异体守这一面（§8）。界面消费的键集不靠点名而靠结构核对——界面对 status 伸的每一次手都必须落在归一化产出的键集里（D10 正是这样被抓出来的：先前那版逐名点字面的守卫会被 `circuit_breaker?.states` 这种可选链写法绕过，同一写法就在存量面板里）。

**第六个面不在台账里**，因为它的端点行一直是"已挂载"状态——`POST /api/experts` 与 `experts.registry.detail`（`ANY` 行，同时是 PUT/DELETE 的溯源行）此前只到 api 层，`store/views` 零消费。补齐时它带来的不是"多一个按钮"，而是三处后端静默陷阱的界面留痕，各自钉住一条源码事实并由 **R1–R8** 八个变异体守（见 §8）：`hourly_rate_cents` 按 `u64` 读再 `as u32` 存，超界**截成另一个数**而不是报错（R2）；`proficiency` 是 `u8`，越界的整条 capability **静默丢弃**（R1）；`delete_expert` 只写 `enabled = false`，而全 alliance 目录没有任何一处把 `enabled` 写回 `true`（R5 之外另有源码守卫直接 `not.toMatch(/\.enabled\s*=\s*true/)`），所以停用是单向门，弹窗必须把"消失是全域事件"逐面点出来。写请求的三条响应形状互不相同（POST 回 `expert+id+created`、PUT 回 `expert+updated` 而**不回 id**、DELETE 回 `id+deleted+soft_delete` 而**不回 expert**），归一化只有一处兜不住就假报成功，故 R4 专测"后端没回 created/expert 也当成功"。最后 R6/R7/R8 守的是这一面自己踩出来的界面缺陷：错误留在页面级横幅上，而弹窗盖住它，用户只看得到一条会消失的 toast。

`rejected` 的判据都来自后端实现本身，不是"没排上期"：`/api/alliance/stats` 的 handler 连 `State` 参数都没有、八个键写死 0（`alliance.rs:1100-1114`）；`/fusion` 与 `/fusion-result` 挂的是同一个 `get_fusion_result`（`alliance.rs:1818-1819`），挂两条就多出两个取数源；`experts/enterprise/analyze` 的 SWOT 与建议是按类型查表的硬编码文案、`overall_score` 是 `0.72/0.68/0.75/0.70` 字面量且匹配到的专家被 `let _experts` 丢弃（`experts_collaboration.rs:1548-1596`）；`/api/experts/dispatcher/consult` 的答案出自 `generate_answer` 模板桩（`confidence = 0.7 + rating/5*0.25`），而 `/api/experts/:id/consult` 走真实 LLM 路径——并列两个咨询面等于把一份假结果摆到用户面前；`dispatcher/multi-consult` 更把 `strategy_used` 写死成 `"best_match_multi"`（`experts_dispatcher.rs:741`），界面显示"按你所选策略"就是撒谎；`orchestration/plugins` 是 6 条硬编码数组、声明的 `webhook_url`/`retry_count` 无人读取（`experts_orchestration.rs:839-920`）。

动作面 2 条（`alliance.tasks.toggle_done`、`experts.dispatch.dispatch`）与破坏性面 2 条（`experts.dispatch.reset` / `reset_all`）都已挂载，易失／模拟档（`experts.orch.*` 五条）也随编排台一次结清（§3.2 最后一条），台账里因此**只剩 8 条，且全部是 `rejected`**——`backlog` 已归零，门禁改为「长出任何一条 backlog 即判红」。这一档的判据没有因为挂载而消失，只是从"待办的理由"变成了"挂载时必须照搬的措辞"：编排计划与历史读的是进程内 `HashMap`/`Vec`（`experts_common.rs:468-470`，无 `save_*`），重启即归零；`orchestrate` 的步骤结果出自 `simulate_step_execution` 硬编码文案、`confidence` 恒 0.85，界面必须自陈"模拟执行"；拓扑失败是以 200 + `status:"failed"` 返回，不能按 HTTP 码判成败。破坏性面那两条接走之前必须先解决的五点（熔断无写侧、重置不落库、两体要求相反、`reset:true` 不证存在、效果要有读数侧）逐条落在上面那个 bullet 与 §5 表里，此处不再重述；只补一条判据：`current_load` 归零本身是真的，因为全仓唯一的生产写入者是咨询落库时的 `+= 1`（`experts_registry.rs:806-810`），界面说"归零"不是空话。这些面的判据全部写在对应用例的注释里，接口一旦补齐语义（持久化、404、真实执行），挡路理由就自动失效，届时对应的契约常量与图例先用例转红、再由人决定收哪句文案。

## 4. 信封裁决规则（易错点）

后端两条 handler 族嵌套不同，前端 `src/api/http.js` 的响应拦截器又已经剥掉一层 `{code,msg,data}`：

| handler 族 | 原始响应 | 拦截器之后 |
|-----------|----------|-----------|
| `alliance.*`（nested） | `{code,msg,data:{elapsed_ms,params,data:{…}}}` | `{elapsed_ms,params,data:{…}}` |
| `experts.*` / `sessions`（flat） | `{code,msg,data:{…}}` | `{…}` |

`_kernel/envelope.js` 的 `unwrap()` 因此按**壳特征**（同时存在 `data` 与 `params`/`elapsed_ms`）判定是否再剥一层，而不是按声明层数硬剥 —— 对"已剥"和"未剥"两种输入幂等。回归防护见 `alliance.api.test.js`「信封已被 http 拦截器解过一层时不多剥」。

`ENDPOINTS[*].nesting` 保留为契约文档值：`flat` 显式禁止二次剥离，`nested` 仅在特征匹配时剥离。错误一律以 `ApiError` 冒泡，`api/` 层不发 `ElMessage`；`call()` 带 `silent: true`，避免与 `http.js` 的全局提示重复。

## 5. 已知后端事实与前端处置

| 事实 | 证据 | 前端处置 |
|------|------|---------|
| `/api/alliance/stats` 是恒零桩 | `alliance.rs` 约 1100-1115 | 列入 `FORBIDDEN_ENDPOINTS`，统计取 `/api/experts/stats` |
| 不存在 `/api/alliance/stream` | `docs/API-REGISTRY.md` 无此路径 | 日志流用 `GET /api/alliance/tasks/:id/logs/stream`（仅默认 `message` 事件，负载为裸 LogEntry JSON，15s keepalive） |
| 图谱概览是 `GET /api/expert-graph` | 同上 | `/api/expert-graph/overview` 列为禁用 |
| 任务列表忽略 `page/page_size` | `alliance.rs` 列表 handler | `listTasks()` 不传分页参数，前端本地关键字过滤 |
| 执行状态 `skipped_nodes` 恒 0 | 后端桩 | `normExecutionStatus` 原样透出并在 UI 不当作真实指标 |
| DAG 出参的 `stats.skipped` 是**折叠桶**，不是"跳过"人数 | `node_stats` 返回元组第 6 位写作 `skipped + cancelled`（`alliance.rs:141-158`），handler 把它取名 `other` 再以 `"skipped": other` 发出（`:1575`、`:1587`），而节点级状态区分两者（`NodeExecStatus` 六态 `:54-61`，`node_status_str :390-399`） | `normDag` 只按后端键名透出、不改写成 `cancelled`；页面上的「跳过 / 取消」由 `model/dag.js` 的 `layoutDag` 从 `nodes[].status` 分开计数，视图不读 `stats.skipped`（P5、P8 两个变异体各守一处） |
| DAG 节点的 `progress` 是状态映射出来的整数，不是真实进度 | `alliance.rs:1535-1539` 的 `match n.status { Completed => 100, Running => 50, _ => 0 }`；同名任务级 `progress()` 才是加权真实值 `(completed + running*0.5)/total`（`:161-167`）——**一个字段名两种口径** | `normNode` 保留该键（后端确实给），但 DAG 页签不渲染节点进度条；要显示必须先标注"由状态推导" |
| DAG 侧没有拓扑校验，悬空依赖与成环都可能到界面 | 边由 `n.dependencies` `flat_map` 生成（`alliance.rs:1560-1573`），不校验 dep 是否存在；Kahn 检查在计划执行路径（`experts_orchestration.rs:443` 起），不是这份出参 | `layoutDag` 三笔账分开：`drawn`（参与分层的依赖）、`dangling`（指向不存在节点）、`backEdges`（回边，遇到即截断但不丢节点、不抛错），页脚各自披露；`edgeDelta` 专测"edges 与 dependencies 恒 1:1"这条后端不变量有没有破 |
| DAG 页签那张图是**模式模板**，不是执行读数 | SDK `alliance.rs:441-515` 的 `build_dag_for_task` 在建任务时按模式查表**写死**字面量节点（每个节点由 名称 / expert_id / 状态 / 依赖 / 输出 / 坐标 六项定死）：七条分支的节点数 4/5/6/7/6/5/4，其中恒 1 个 `Running`（voting 3 个、debate 2 个），首节点 `需求分析` 恒 `Completed`、末节点恒 `expert-fusion 融合输出`（`:439-440` 的注释自己就写明"契约保持"） | 控制台 DAG 页签顶部一行说明由 `contract/mode.js` 的 `modeTopologyNote()` 生成：报出该模式的节点数、恒 Running / 恒 Pending 的个数、首末契约位，并指名后端坐标；措辞判据是双向的——既要点出「展示态拓扑」，也要点出「不是执行读数」（X6/X7 各守一处，此前是一条 `|` 析取式，改成两条独立断言才被抓出可绕过）。视图只取值，算法与文案住契约层。**真机边界**：:3080 停机时取不到任务详情，DAG 分支整块不渲染，所以这句说明条的**可见性**还没有浏览器级证据（X14/X17 只证明它在模板里、且读的是任务的 `mode`），待网关恢复后按任务 #11 补一次渲染核验 |
| dynamic 在网关侧**没有分支可展示** | 同一函数里 Dynamic 分支只有 4 个节点，其中「动态路由」是一个普通 expert 节点（`expert-router`，`:509`）；真实的动态选型在调度器规划期（`planner.rs:332-411` 的 `generate_dynamic_plan`，产物 `PlanDynamicRoute{decision_node, true_branch, false_branch}`，`types.rs:355-370`），只被执行器的 `build_dynamic_routes`（`dag_engine.rs`）在内存里消费，SDK `alliance.rs` 全文对 `dynamic_routes` **零命中** | 说明条在 dynamic 那句里追加"图上没有分支…不在任何响应里，所以界面答不了「这一次实际走了哪条分支」"（X16/X11/X12/X13 守这条线，包括把引用行号当事实重算）。**不做**的事：不画假分支、不从 `plan.nodes` 猜一条"当前走的"路径 |
| `build_dag_for_task` 的文档注释只列了**六种**模式拓扑 | `alliance.rs:431-437` 写「六种模式对应六种专家拓扑」并枚举 6 条 `/// - Xxx`，而下面的 `match` 有 7 条分支（Dynamic 是后加的，注释没跟上） | 前端不说"六"：`mode.test.js` 解析注释里的 `/// - ` 条目数（必须仍为 6）并断言它小于分支数，同时要求七个模式的说明文案里都不出现 `六[种个]`。注释补齐那天这条断言先红，界面文案跟着重看 |
| `estimated_remaining_minutes` 是编造的估算 | `alliance.rs:1702`（`get_task_status_poll`）写死 `(total - completed) * 3` | 控制台不显示该字段，KPI 只取 `execution.counts`；设计稿里「预计剩余 2.5s」在 Rust 侧无对应实现 |
| 非 remote 模式下进度/耗时为模拟 | `runtime.mode = local_preview` | `normRuntime` 置 `simulated: true`，控制台顶部黄条显式声明 |
| 收藏是 toggle，响应回读真实态 | `experts_ext.rs` `toggle_expert_favorite` 返回 `{favorite, action}` | `store.toggleFavorite()` 以响应的 `favorite` 增删本地 Set，绝不按点击方向猜测 |
| 收藏与预约都**不按登录用户隔离** | `experts_ext.rs`：favorites 是进程内全局 HashSet；`create_booking` 写死 `user_id: "admin-user"`，`my_bookings` 无过滤返回全量 | 面板显式标注该后端现状（「我的预约」实为全量预约），前端不假装已按用户切分 |
| `consult_now` 双分支都 200 | 专家不在线时 `session_id: null` | `normConsultNow` 保留 null，UI 以 `sessionId` 判空措辞，不把 `status` 当成功 |
| 预约的 `scheduled_at` 可缺省 | `experts_ext.rs` `CreateBookingBody` 为 `Option`，缺省取 now+24h；`duration_minutes` 缺省 60 | 表单留空即不发送该键（`api.createBooking` 只带非可选空值项），计划时间以响应为准 |
| 咨询室凭据独立于会话 | `GET /api/experts/bookings/:id/consult-room` 返回 room_id/token/ICE | 弹窗只呈现凭据并注明「前端尚未挂载实时房间页」，不伪装成已进入房间 |
| 存量页排行榜的依据后端根本不存在 | `ExpertPlazaView.vue` `processExperts()` 把 `goodRate ?? '0.0'`、`consultCount ?? 0`、`monthGrowth ?? 0`、`responseTime ?? '-'` 补成假数据后排序；`experts_common.rs` 无 `good_rate`/`month_growth`/`consult_count`/`response_time` 字段 | 榜单重建于 `ExpertMetrics`（`total_consultations` / `avg_rating`+`rating_count`）与 `created_at`；无真实样本的专家**排除**而非补零，弹窗顶部标出「样本仅当前页 N 位，其中 M 位可排名」 |
| 存量页顶部 KPI 读的键在 stats 响应里一个都没有 | `ExpertPlazaView.vue` `loadStats()` 取 `expert_count`/`consult_count`/`good_rate`/`avg_response`；`experts_stats_real` 实际返回 `total_experts`/`total_consultations`/`avg_rating`/`avg_response_minutes`/`satisfaction_rate`/`domains`/`ts` → 四个 `if (s.x != null)` 分支永不命中，卡片停在占位值 | `normExpertStats` 逐键对齐真实响应，广场顶部 `expertStatsCells` 呈现 6 张卡；计数 0 照实显示（如「今日 0 次」），评分/响应为默认零值时显示「—」，浮点统一 1 位小数 |
| 「加入专家团」在存量页必失败 | `ExpertPlazaView.vue` `addToTeam()` 只发 `{expert_id}`，而 `join_team_real`（`experts_registry.rs:694-700`）要求 `team_id`，缺失即 `err(400, "team_id is required")` | 模块 `api.joinTeam({teamId, expertId, role})` 与 `store.joinTeam` 按真实签名就绪并有单测；广场页**未挂载**该入口，因为后端没有团队读接口可给出 `team_id`（`/api/experts/team` 与 `/api/expert-graph/optimal-team` 都是 POST），待有团队来源后再上 UI |
| 协作请求体只有两个 serde 别名 | `RouteBody.question` 带 `alias = "query"`、`max_experts` 带 `alias = "top_n"`，其余字段无别名 | 前端只发 snake_case wire 名（`COLLAB_WIRE`）；camelCase 键会被 serde 静默丢弃而不是报错，这类"看着发了其实没发"的偏差由 §3 等集断言拦住 |
| 后端注释与 serde 实际接受的键名相互矛盾 | `RouteBody.max_experts` 的文档注释写「前端统一传 `maxExperts`」，但字段别名只有 `top_n`，`maxExperts` 反序列化时落不进任何字段 | 以**代码行为**为权威而非注释：只发 `max_experts`。存量两页正是照注释写的（`views/expert/ExpertCenterView.vue:270`、`views/workspace/ExpertWorkspaceView.vue:600` 都传 `maxExperts`），它们的「参与数量」选择器因此从未生效 —— 见 §7.2 |
| 智能路由还有第二层输入：`constraints` 嵌套对象 | `RouteBody.constraints: Option<serde_json::Value>`，`route_query` 用 `c.get("min_rating")/get("max_response_time")/get("require_online")` 逐项取值，缺项 `unwrap_or` 到 0.0 / `f64::MAX` / false | 三个子键进契约为 `constraintFields`；面板按「0 表示不限」「留空表示不限」如实措辞，清空数值项回落 `null` 后该子键不进请求体，越界先夹到 5 / 240。存量两页完全没发过这个对象，路由能力被当作只有 `question` 用 |
| 缺 `topic` 的辩论请求是 422 而不是 400 | `DebateBody.topic: String` 无 `#[serde(default)]`，反序列化失败由 axum 直接拒绝 | 辩论模式的输入框绑 `topic`（`textField`），校验文案与 wire 名分开措辞，避免用户以为"问题"能当议题用 |
| 多专家自动匹配可能 404 | `multi_consult` 走 `match_top_experts(.., 0.3, ..)`，无专家过线时 `err(404, ...)` 而不是返回空列表（辩论的自动匹配阈值是 0.2，见 `experts_collaboration.rs:1002`） | 面板把「不选专家」解释为"由后端自动匹配，可能 404"，并在候选少于 2 位时提示辩论不可运行，而不是发一个必失败的空 `expert_ids` |
| 辩论只取前 4 位上场 | 显式传 `expert_ids` 时 `.filter(enabled).take(4)`；不传时按 `topic` 自动匹配 4 位 | 勾选第 5 位时 `capacityNote` 直接写明「后端仅取前 4 位上场」，不假装参与；`rounds` 则由 `clamp(1, 10)` 兜底，前端同样只发边界内的值 |
| 治理闸门否决后正文被替换 | 各协作 handler 走同一否决/后验治理，`answer` 里带 `veto_reason` 等字段 | 面板用 `blockedNote` 说明"下方是替换后的拦截说明，不是专家原文"并列出 `vetoReason`，不再声称被拦截正文从未出网 |
| 咨询类回答有模板兜底 | `answer.source` 缺省即模板生成（真实单专家咨询响应的 `answer` 里没有 `source` 键） | `answerSourceText()` 把两种来源分开标注为「模板兜底作答」/真实来源，置信度取后端数值 |
| `algorithm-analysis` 的 `input_constraints`/`requirements` 只回显 | handler 读取后原样放进响应，不参与计算 | 归一化保留为 `echoed`，UI 不把它当作分析依据；`context`/`history`/`stance`/`priority` 这四个后端不消费的键由 `COLLAB_LAZY_FIELDS` 明确拒发 |
| 智能咨询的风险评估与行动项是模板 | `experts_collaboration.rs:1210-1214` 的 `risk_assessment` 里 `overall_level` 写死 `"medium"`，三条 `*_risk` 与四条 `action_items` 都是格式化模板串 | 面板标题直接写「风险评估（后端固定为 medium 档）」，行动项不与真实模型输出混同呈现 |
| 调度配置是网关进程内的内存态 | `update_config` 只改 `state.dispatcher_config.lock()`，无持久化写入 | 控制台配置卡的措辞是「网关内存态」，保存后以响应回读作新基线，不声称已落库、不假装重启后仍在 |
| 调度配置的 PUT 是合并式而非整份回写 | `update_config` 逐键 `if let Some(v) = body.x` | 前端 `dispatchPatch` 只发差异键；无差异时「保存」是空操作、不发请求（变异体 M13 专测这一点） |
| 调度配置此前**只有 api 没有界面** | `views/**` 里 `dispatcher/config` 零命中，仅存量 `api/experts.api.js:37-38` 两个导出无人调用 | 本模块把它挂成控制台第一块可写配置面；`weights`（`Map<专家ID, 权重>`）刻意不挂载，理由登记在 `DISPATCH_UNMOUNTED_FIELDS`，避免造出"能填但没人会用"的控件 |
| 后端只对 4 个配置项执法 | `update_config` 为 `strategy`（valid 数组）、`match_threshold`（0.0..=1.0）、`max_retries`（>10）、`timeout_seconds`（1..=3600）返回 400；`circuit_breaker_threshold` 与两个布尔开关照收不验 | 契约用 `checked` 标记二者之差，`checked: false` 项的提示文案必须含「后端不校验」，前端不冒充执法者；四条 400 规则在 UI 侧前移拦截，越界改动根本不发请求 |
| 图谱是从专家注册表**派生**的内存态，新专家要重建才上图 | `experts_common.rs` `build_graph_from_registry`；`POST /api/expert-graph/rebuild` 返回 `{rebuilt, previous_version, new_version: previous_version + 1, duration_ms, …}` | 页面顶部常显 `version` 与 `built_at`；「重建图谱」成功后重取 图 / 统计 / 社区 三份，失败时**保留旧图**并只在原地处报错，不出现半新半旧的画布 |
| 图里**没有技能清单** | expert 节点 `properties` 恰好是 `title / domains / avg_rating / status` | 检视面板不给技能，页面侧栏「这份图谱不告诉你什么」列在第一条；契约断言 `normGraphNode` 里不存在 `.skills` 读取 |
| 协作边是共享能力域的 Jaccard 相似度，不是历史共事次数 | `collaborates_with` 建边条件 `if similarity > 0.1`，`weight: similarity`，properties 带 `shared_domains` | 边例说明与协作者表都按「相似度」措辞，并把 `shared_domains` 作为唯一关联证据展示 |
| 域节点没有协作者维度 | `get_collaborators` 只遍历 `collaborates_with` 边 | `store.selectNode` 只对 `nodeType === 'expert'` 发协作者请求；域节点显示「该维度不适用」，不空跑一次请求 |
| 路径不可达不是错误 | `get_path` 从不 404，不可达时 200 + `found: false` + 空 `path` | `normGraphPath` 保留 `found`，界面走 `pathChainText()` 的「两节点间不存在连通路径」，不弹错误 |
| 邻居与协作者对未知 id 是 404 | `node not found: {id}` / `expert not found` | 失败只落 `error.node`，**不清空** `selectedId`，用户可换节点重试 |
| `max_members` 只有「0 当缺省 5」这一条兜底，`min_rating` 完全不校验区间 | `params…unwrap_or(5)` / `min_rating.unwrap_or(4.0)` / `if max_members == 0 { 5 }` | 表单按后端真话提示（「后端无上限」「后端不校验区间：>5 只会筛出空团队」），越界由 `optimalTeamProblem` 拦在请求之前；等于缺省的数值**不进请求体** |
| `constraints` 是收了却从不读的字段 | `OptimalTeamBody.constraints: Option<Value>` 带 `#[serde(default)]`，handler 里无 `body.constraints` | 契约标 `mounted: false` 且 `optimalTeamBody` 永不发送，页面写明「后端收下即丢，故不提供该控件」；变异体 G1 专测有人把它挂回界面 |
| 只有详情带正文，列表是投影 | `session_to_list_view:113` 给 `message_count` 且剥掉 `messages`；`get_session:347` 才 `ok(json!(session))` | 「打开会话」必须补一次 `GET /sessions/:id`（`store.selectSession`），列表行内不渲染正文也不假装能渲染 |
| 追加消息的响应只有那条消息 | `append_message:448` `ok(json!(message))`，不回会话快照 | store 自己把它并进线程、推出 `messageCount` 与 `lastActiveAt`，随后重取列表让两侧的条数同数；变异体 S12 专测"不并线" |
| PUT 是合并式更新，metadata 只能逐键写入 | `update_session:390-394` 只有 `insert`，没有删除键的分支 | 改动一律先经 `sessionUpdatePatch` 差分，未改的键不进请求体（无差异即不发请求，变异体 S14）；用户移除的键在通知里点名"后端删不掉"，不给"删除成功"的假象 |
| 换专家阵容 / 换类型 / 换发起人没有端点 | `UpdateSessionBody` 字段面恰好 `title/status/topic/tags/metadata` 五个 | 编辑面板不给这三个控件，只在创建表单里给；`expert_ids` 的提示写明"之后无法修改" |
| 会话的 `status`、消息的 `role`/`msg_type`、`session_type` 一律不校验 | `create_session:154`、`update_session:361`、`append_message:422` 全部 `unwrap_or_default` 或直接落库 | 枚举只用于选项与提示，UI 文案不得声称"后端会拒绝越界值"；真实代价写在 `sessionStatusProblem`：第四个状态值会**在统计里隐形**（`session_stats` 的 `match` 只有三档 + `_ => {}`） |
| `?status=` 空串是有效过滤 | `if s.status != st` 用 `Some("")` 参与比较 | `sessionListQuery` 对空白值一律不发（变异体 S1），否则一次误点就把列表清成空表 |
| 列表没有 `sort` 参数 | `sort_by(\|a, b\| b.created_at.cmp(&a.created_at))` 写死 | 界面不摆排序控件；`search` 只命中 `title`/`topic` 的小写包含，**不含消息正文**，占位文案照此写 |
| 「相似检索」是字符 bigram Jaccard，全域检索不过滤状态 | `text_similarity:827`（空串 0、单字相等才 1）；`semantic_search:522-532` 只按 `session_type`/`expert_id` 过滤，收录门槛 `score > 0.0` | 界面一律说「字面相似」，不借用 AI 语义的话头；全域命中含已归档会话，作用域文案由 `semanticScopeText()` 把扫描量与本次过滤条件一起说出（后端不回显过滤条件，故只能由发起方交代，`store.semanticFilter` 记的就是这个） |
| `download_url` 恒为 null | `export_session:586` 写死 `"download_url": null` | 归一化保留 `null`（变异体 S10 专测有人把它归成空串），页面只给"复制/存为本地文件"，落地由 `contract/exportText` + `exportFileName` 在前端完成 |
| 归档没有反端点，删除与"不存在"同形 | `archive_session:605-607` 强制写 `status=archived` + `archived_at`；`delete_session:413` 只回 `{deleted, session_id}`；7 处 `err(404, "session not found: {id}")` | 归档后通知里说明"恢复只能把 status 改回 active 保存"；删除/未知 id 都按 404 文案判，`isNotFound(e)` 命中才清选中态（变异体 S13 反着判也要红） |
| 协作写路径会把会话落库，`user_id` 是后端写死的 | `experts_collaboration.rs:815 / 925 / 1023 / 1245` 用 `"anonymous"`，`:1469` 用 `"enterprise-user"` | 会话面是智能协作的**服务器侧真相**：列表 `user_id` 过滤框与创建表单的 `user_id` 共用一根线，页面明说留空即永远命中不到任何 `user_id` 过滤，避免用户把匿名会话当成丢了 |
| 三条路由**早已挂载却漏登记** | `experts_dispatcher.rs:850-852` 挂了 `GET/PUT /api/experts/dispatcher/config`、`experts_registry.rs:839` 挂了 `POST /api/experts`（`create_expert`），`actuator.rs` `ROUTES` 里都没有对应行 | 由 §3 的三方等集测试与 §3.2 台账暴露，本次补登记三行（ROUTES 225 → 228，重跑 `scripts/doc/gen-api-registry.py`）；`expertRegister` 此前只能借用 GET 的 `experts.registry.list` 行做溯源。这正是治理规则「新增路由必须先登记 ROUTES」的现存缺口，不是前端能绕的事 |
| 写路径**只认证、不授权**，`name` 是唯一硬必填 | 认证在：`config.rs:37` 的 `AuthConfig::default()` 是 `enabled: true`，`modules.rs:186` 给业务路由统一挂 `auth_middleware` ⇒ 匿名请求进不到 handler。授权不在：`src/alliance/*.rs` 全域没有一处 `ApiAuth` / `Extension<UserInfo>` / `current_user` ⇒ handler 根本不知道调用方是谁。必填只有 `name`（`create_expert:348-364`：空即 400 `expert name is required`，`id` 撞库即 400 `expert id already exists`，其余字段经 `merge_expert_from_value` 挑着读） | 已挂载（注册 / 编辑 / 停用，见 §7.5 A 档第一行）。界面措辞只能说"没有角色判定"，不能写"仅管理员可操作"——后端没有这件事，写了就是替后端编造授权模型；模块页也不再自造 `canWrite` 置灰（外壳路由守卫已要求身份，那行代码在页面上不可达）。两条事实由 `EXPERT_WRITE_IDENTITY` 钉成跨语言测试 |
| 单个专家的指标对**停用**者也返回 404 | `expert_metrics:565-566` 是 `Some(e) if e.enabled`，`derived` 三项为 `rank_percentile` / `load_ratio` / `efficiency_score`（40/30/30 加权，`:583`） | 已挂载（派生指标三格）：404 不折叠成零值也不写成"该专家不存在"，停用与不存在在后端同形（变异体 K3 守这一点，K2 守前端不重算 40/30/30） |
| 会话列表**认 `limit`**，但只当 `page_size` 的别名 | `parse_pagination:877` 是 `params.get("page_size").or_else(\|\| params.get("limit"))`，再 `.clamp(1,200)` | `SESSION_QUERY_KEYS` 刻意不含 `limit`（同值两条线会造成"到底谁生效"的歧义），只发 `page_size`；而 `project_id` 后端从不读，任何"按项目过滤"的开关都是假作用域（§7.4） |
| 编排历史与统计是**进程内 `Vec`**，无落盘 | `state.orchestration_history`（`experts_common.rs:470` 初始为 `Vec::new()`），对比 sessions 有 `save_sessions`、registry 有 `experts_db::save_registry` | **已挂载**（任务 #22）且带硬约束：文案不得写"累计/历史全部"，`plan/execute` 对重启前的 `plan_id` 恒 404，拓扑失败是以 200 + `status:"failed"` 返回（`experts_orchestration.rs:457`），判成败不能看 HTTP 码——这条口径由 `orchExecuteOutcome` 单源产出，视图不自读 HTTP 码 |
| 编排面的"诚实"来自逐字段来源标注，不来自文案 | `ORCH_PROVENANCE` 三档（`computed`/`simulated`/`zero`）× `ORCH_FIELD_PROVENANCE` 十行登记，视图每个带角标的字段必须能在清单里查到（`orchestration.test.js` 反向取界面正则再等集） | **已挂载**：界面只能给已登记的十行加角标（Q14 把一条路径改错拼即判红），未登记的路径显示"真实计算"是最危险的默认，所以清单缺行等同于测试失败 |
| 步骤内容是查表不是生成 | `simulate_step_execution:230-263` 七类 `step_type` 各配一段固定中文文案＋兜底分支，`confidence` 是 `:271` 的字面量 `0.85`，`duration_ms` 是 `:493` 的 `10 + completed_count*5`，`status` 是 `:497` 的字面量 `completed`，`result.expert` 因 `:488` 传 `&[]` 恒 null | **已挂载**：五处字面量以 `ORCH_SIMULATED` 九条枚举入契约，`at:` 逐条按行号反查 Rust 源码（Q3 改一个行号坐标即判红），图例由 store 原样转发整张表（Q16 删一条、Q15 让界面渲染空数组都判红），界面不得把 `confidence` 说成置信度模型 |
| `plans_ready` / `plans_failed` 恒为 0，且不是"没有失败" | `plan.status` 全文只有三处赋值：`draft`（`:203`）、`running`（`:469`）、`completed`（`:511`）；环检测失败分支（`:452-463`）直接 `return` 而不回写 status | **已挂载**：`ORCH_ZERO_COUNTERS` 两条各自带一句"这条 0 不是没有失败，是没有能记录失败的代码"（Q4 从清单里删一条即判红）。统计区照常渲染 0，但补一句成因，避免把"没有代码"读成"没有事故" |
| 计划拓扑是单链，不是 DAG | `topological_sort:39-92` 是**真** Kahn + 环检测，但 `generate_plan:174-177` 生成的步骤 `depends_on` 永远只指向前一步 ⇒ 检测环的代码没有输入能触发它 | **已挂载**：笔记区分"算法真实"与"输入使环不可达"两件事，不写"不支持并行"这种把后端事实说反的话；步骤清单同样由查表决定（`research/consulting/development/analysis` = 5/5/6/5 步，兜底表 `:101-139` 为 **6** 步），`ORCH_STEP_COUNT_BY_TASK_TYPE` 是唯一数字源（Q1 改兜底步数即判红），界面那句"兜底表（N 步通用文案）"由 `orchFallbackNote()` 生成，写死数字的分支由断言禁掉 |
| `orchestration/history` 是全域唯一**不走** `parse_pagination` 的分页 | `:936-937` 直接 `params.get("page").unwrap_or(1)` / `page_size.unwrap_or(20)`，没有 `.clamp(1,200)` ⇒ `page_size=0` 会得到"总数非 0 的空页" | **已挂载**：前端在 `orchHistoryQuery` 里自行 clamp 到 `ORCH_HISTORY_PAGE_SIZE_MAX`，该常量从 `experts_common.rs:880` 的正则反查得到（Q5 期待常量自比、Q6 拆掉 clamp 都判红）；`orchHistoryAnomaly` 负责把"空页却有 total"这条后端事实说给用户，而不是静默翻页 |
| 调度器分发面读的是同一份内存配置 | `dispatch_task:229` 与 `dispatch_n_experts:331` 都锁 `state.dispatcher_config`，与 `update_config:420-444` 同一互斥量；`multi_consult:741` 却把 `strategy_used` 写死 `"best_match_multi"` | 配置面与控制台的「分发实跑」因此可以互相印证——实跑响应的 `strategy_used` 是唯一能证明"改过的策略真的被走到"的字段，界面把它和当前配置并排显示、不一致就报不一致（§3.2 的 L9）；但 `dispatcher/multi-consult` 因那个写死值被定为 `rejected`：界面若显示"按你所选策略"就是撒谎 |
| 调度状态端点**早已挂载，界面却取不到** | `api/alliance.api.js:249` 有 `dispatcherStatus()`、`model/normalize.js:625-648` 的 `normDispatcherStatus` 已与 `dispatcher_status`（`experts_dispatcher.rs:452-545`）的扁平出参逐键对齐，但 `store/**` 与 `views/**` 全域零命中 ⇒ 它计入 §3.2 的 66 行挂载集（该集合随挂载增长，此处按当前口径），界面上却一个字段都看不见 | **已挂载**（任务 #15）：状态面是重置动作的**读数侧**，二者同批接。`.acks-card` 五格 KPI + 熔断列表 + `expert_loads` 表全部来自这份出参，`store.loadDispatcherStatus()` 是它在全模块唯一的取数入口；两个重置入口没拿到读数时一律 disabled（D 系列 19 个变异体守这条线） |
| 熔断计数**只有读侧、没有写侧** | `FAILURE_COUNTS`（`experts_dispatcher.rs:42` 的 `static Mutex<Option<HashMap<String,u32>>>`）在全 gateway 只有四处读（`:47/:132/:494/:776`）与两处清（`:794/:828`，都在 reset 里）＋一处测试（`:1029`），**没有任何生产路径 insert** ⇒ `circuit_breakers[]`（`:494-508` 只列 `count>0`）恒为 `[]`、`is_expert_available` 的熔断分支（`:131-138`）永不成立、配置面的 `circuit_breaker_threshold` 是"读侧存在、写侧没有"的死旋钮、reset 的 `previous_failures` 恒 0 | **已按这条口径挂载**（任务 #15）：界面不得写"所有专家服务正常"——那是把"没有数据"说成"数据良好"；要按"后端当前没有累计失败计数的代码路径，故该列表恒为空"措辞，与 §5 上面对 `skipped_nodes` 恒 0 的处置同一标准。落点：`contract/dispatcher.js` 的 `breakerEmptyNote`（唯一措辞源），非空时才渲染列表（D5 专测有人把空列表静默成"没有异常"） |
| `current_load` 的来源与"重置不落库"的不对称 | 会写的只有三处：咨询落库时 `+= 1` 且 `save_registry`（`experts_registry.rs:806-810`）、注册/编辑直写 `availability.current_load`（`:136-138`，u64 读入后 `as u32` 截断）、两个 reset handler 写 0（`experts_dispatcher.rs:786-790`、`:813-841`）。而**两个 reset 都不调 `save_registry`**（`experts_dispatcher.rs` 全文零 `save`），registry 本身是 SQLite 支撑（`experts_db.rs:225-264`）⇒ 归零只活在内存里，重启按最后一次 `save` 的快照恢复；重置之后的第一次咨询又会把"已归零"顺带落库 | **已挂载**（任务 #15）：`dispatchResetLines` 五条后果清单逐条带 `:行号`，第三条原话就是"两处都只改内存、不落库…重置之后的第一次咨询又会把『已归零』顺带写进库"；清单里任何一条被抹掉坐标即判红（D16） |
| 两个 reset 端点的请求体要求相反 | `reset_expert`（`:763-808`）签名带 `Json<ResetBody>` ⇒ **必须发一个 JSON 对象**（`reason` 可选，`:114-117` `#[serde(default)]`；空 body 被 axum 拒成非 `{code,msg,data}` 信封体，页面拿不到可读原因）；`reset_all`（`:813-841`）**没有 body 提取器**，响应也只有 `reset_count/reset_expert_ids/reset_at`，无逐项旧值 | **已挂载**（任务 #15）：单专家重置即使 `reason` 留空也发 `{}`、全量重置不发体，两式都在 `contract/dispatcher.js` + `api/alliance.api.js` 收口并由请求形状用例逐字对表（D11 让单专家不发体、D12 让全量发体，各自判红）。`reset:true` 也不当"该专家存在"的证据：回执文案由 `dispatchResetNotice` 单源产出，改口称它证明存在即判红（D17） |
| 存量企业面板消费三个**后端从不返回**的键 | `views/expert/panels/ExpertEnterprisePanel.vue:106,107,117,118` 读 `dispatcherStatus.circuit_breaker.states`（真键是扁平 `circuit_breakers`）、`dispatcher.recent_dispatches`（`dispatcher_status` 出参里根本没有这个键）、`cb.status`（真键是 `state`）；`:605` 的 KPI 虽写了 `?.circuit_breakers || ?.circuit_breaker?.states` 兜底，过滤条件仍是 `s.status === 'open'` | 该面板的「熔断器状态」区块因此恒显示"所有专家服务正常"、「最近调度」恒空、顶部「熔断器触发」KPI 恒 0——三重假象。列作 §7.5 差集的输入：模块侧接管状态面后，这块存量界面属于"看起来能用、读数永远为空"的一类 |

## 6. 装配单源（路由 / 导航 / 模块归属）

- `src/modules/index.js` 追加一行即完成登记；路由由 `collectRoutes()` 在 `src/router/index.js` 展开，**必须位于 `...fallbackRoutes` 之前**（通配兜底会吞掉模块页），由 `wiring.test.js` 顺序断言守卫。
- 侧栏导航由 `src/constants/nav.config.js` 末尾按 `collectNav()` 自动挂载到 `meta.module` 对应模块，业务模块页不再手抄导航项。
- 模块归属判定收敛到 `src/composables/useActiveModule.js`：优先 `route.meta.module`，旧路由回退路径前缀表。此前 App.vue / TheSidebar.vue 等 5 处各抄一份且已漂移。
- 侧栏条目只有带 `path` 才可跳转（`.navigable`），并禁止编造 `count`/`badge`（`module.test.js` + `wiring.test.js` 双守卫）。
- `defineModule` 强制每条路由 `meta.{title,module,layout}` 完整，`layout ∈ {default, blank}`。
- `defineModule(descriptor, import.meta.url)` 记录**声明源**：同一源文件重复声明按 Vite 热更新替换（否则改一次 `index.js` 就抛「模块重复注册」并把路由打挂），异源抢占同名仍抛错。`module.test.js` 两个方向都守。

## 7. 存量迁移顺序

模块覆盖到等价能力后才允许删除存量，逐步收敛：

1. `ExpertPlazaView.vue`(1944) → **已落地** `views/AllianceExpertsView.vue`(492)：列表/服务端搜索/状态与类型筛选/排序/收藏/详情抽屉/预约下单/取消/咨询室/即时咨询/平台统计 KPI/排行榜，全部走本模块 `store → api → contract`。存量页暂留 `views/expert/` 未删，逐屏核对见 §7.1。
   - 排行榜与 KPI 是两处**不 1:1 移植**的能力：存量页按 `goodRate`/`consultCount`/`monthGrowth` 与 `expert_count`/`good_rate` 排名和计数，这些字段后端不存在（见 §5），新页改为 `ExpertMetrics` + `created_at` + `/api/experts/stats` 真实响应键，并显式声明样本只覆盖当前页、KPI 为平台级口径。
2. 六模式智能协作（存量抄在 `views/expert/ExpertCenterView.vue` 与 `views/workspace/ExpertWorkspaceView.vue` 两处）→ **已落地**（2026-09-23 第 2 批）：`views/AllianceCollabView.vue`(91) + `components/ExpertCollabPanel.vue`(398) + `store/alliance-collab.store.js`(142) 成为 smart / route / single / multi / debate / algorithm 六模式的唯一实现，控制台里的协作弹窗已删除并改为 `router.push('/alliance/collab')`。逐屏证据见 §7.2。
   - **为什么从弹窗升为独立页**：协作一次要同时呈现模式说明、候选专家、控件与结果四类信息，860px 弹窗里互相挤压且结果区无法与「请求形状」对照；更硬的理由是一次真实故障——弹窗根类名与控制台的 `ac` 相同，页面内 `querySelector('.ac')` 命中控制台节点，把协作结果读成了控制器的告警。独立页 + §3.1 类名守卫同时消除两者。
   - 剩余部分：`views/workspace/` 14 面板中真正独有的**任务创建工作流**尚未合并进控制台，属第 3 批。
3. `ExpertCenterView.vue`(1044) 与 `panels/`(总约 4.1k) → 模块内管理面子页。其中 `panels/ExpertEnterprisePanel.vue`(1290) 的图谱 / 统计 / 调度三块能力已被本模块的 `/alliance/graph` 与 `AllianceConsoleView` 覆盖，逐字段差异见 §7.3，即其退役证据。
4. `ExpertConfigView.vue`(5414) → 拆分为配置表单 + 预览 + 历史三段。
5. `stores/alliance.store.js`(628) → 由 `store/alliance-console.store.js` 取代后删除。
6. **第 3 批（2026-09-23）不是迁移而是补面**：`/alliance/graph` 把 `experts.graph.*` 八个已登记端点第一次做成企业级界面（`views/AllianceGraphView.vue` 133 行 + 四个组件 + `store/alliance-graph.store.js` 216 行 + `model/layout.js` 123 行）。它同时是存量两处图谱消费方（`panels/ExpertEnterprisePanel.vue`、`useGraphCanvas.js`）的替代实现，逐字段证据见 §7.3。

`frontend-ui/src/MODULE-MANIFEST.md` 记录每步的落地与存量删除。

### 7.1 存量广场页能力核对表（退役前置证据，2026-09-23）

| 存量能力 | 后端是否支撑 | 新页处置 |
|---------|-------------|---------|
| 列表 / 服务端搜索 / 状态与类型筛选 / 排序 / 分页 | ✅ | 等价覆盖并加契约守卫 |
| 详情抽屉、收藏、预约下单、取消、咨询室凭据、即时咨询 | ✅ | 等价覆盖，全部走 store 单点错误 |
| 顶部 KPI 四卡 | ❌ 读的是 stats 响应里不存在的键 | 以 `expertStatsCells(normExpertStats(...))` 重建为 6 卡 |
| 排行榜（月度咨询榜 / 好评榜 / 新星榜 + 综合分加权） | ❌ 排名依据全为前端补的假字段 | 以 `ExpertMetrics` 真实字段重建三榜，见 §5 |
| 加入专家团按钮 | ⚠️ 后端要求 `team_id`，存量只发 `expert_id` → 恒 400 | 暂不挂载 UI；`api/store.joinTeam` 已按真实签名就绪并单测 |
| 评价列表 / 好评率 / 联系方式（phone、email、joinDate、department）/ 热度·推荐·新晋徽标 / 按类型配色头像 | ❌ `ExpertDescriptor` 无这些字段，且无专家评价端点 | 不移植；头像配色改由主题 token（`--brand` / `--brand-accent`）驱动，徽标只保留后端可证的认证态与在线态 |

结论：新页已覆盖存量页**全部有后端支撑**的能力，剩余差异都是「后端不存在的假数据」或「必然失败的调用」。删除 `views/expert/ExpertPlazaView.vue` 与 `/expert-plaza` 路由仍属破坏性操作，需用户确认后再执行。

### 7.2 存量协作实现核对表（第 2 批退役前置证据，2026-09-23）

六模式在存量里抄了两份（`views/expert/ExpertCenterView.vue`、`views/workspace/ExpertWorkspaceView.vue`）。逐处请求与响应键同 Rust 源码比对：

| 模式 | 存量出处 | 存量发出 / 读取 | 后端真实行为 | 新页处置 |
|------|---------|----------------|-------------|---------|
| 智能路由 | `ExpertCenterView.vue:269-270`、`ExpertWorkspaceView.vue:600` | 发 `maxExperts` | 无此别名，`max_experts` 落 `unwrap_or` 缺省 → 「参与数量」选择器**从未生效** | 发 `max_experts`（夹 1–20），并补发存量从未用到的 `constraints` |
| 智能咨询 | `ExpertCenterView.vue:288-292` | 发 `mode:'auto'`、读 `result.routing` | `mode` 不在 `IntelligentConsultBody` → 丢弃；响应顶层只有 `consultation_id/question/intent/matched_expert/answer/…`（`:1286-1295`），**没有 `routing`** → `routingResult` 恒 undefined | 按真实顶层键归一化，`intent` 走 `INTENT_LABELS` 出中文 |
| 单专家咨询 | `ExpertCenterView.vue:346` | 发 `question` | 与后端一致 | 等价覆盖（专家走路径参数，不落 body） |
| 多专家咨询 | `ExpertCenterView.vue:374-377`、`ExpertWorkspaceView.vue:575` | 发 `question/expert_ids`、读 `result.results.filter(r => r.success)` | 请求正确；但 `multi_consult` 顶层是 `session_id/question/experts/fused_answer/created_at`（`:972-978`），**没有 `results`** → 200 之后立刻 TypeError 被外层 catch 吞成"请求失败" | 以 `normMultiConsult` 读 `experts` + `fused_answer`；实跑 200 渲染 2 位专家与共识度 0.57 |
| 辩论 | `ExpertCenterView.vue:402-403`、`ExpertWorkspaceView.vue:520` | 发 `question/expert_ids/rounds/mode` | `DebateBody.topic: String` 无 `#[serde(default)]`，缺 `topic` → axum **422**，存量辩论从未成功 | 输入框绑 `topic`；`rounds` 夹 1–10；第 5 位起标注「后端仅取前 4 位上场」 |
| 算法分析 | `ExpertCenterView.vue:449-450` | 发 `question/graphData/options` | `AlgorithmAnalysisBody.algorithm_description` 必填 → **422**；`graphData`/`options` 结构体里没有该字段 | 绑 `algorithm_description`；`input_constraints`/`requirements` 归一化为 `echoed` 只作回显 |

不移植清单（有意收敛，不是遗漏）：`TaskOrchestrationPanel.vue` + `useTaskOrchestration.js`（`setTimeout`/`Math.random` 造的假进度）、`/api/ai/engine/alliance/full` 与 `/capabilities`（仅编排器模式可用）、四套布局状态机、以及两份互相漂移的颜色/emoji 映射表。

### 7.3 存量图谱实现核对表（第 3 批退役前置证据，2026-09-23）

图谱 8 个端点在存量里有两处消费方：`views/expert/panels/ExpertEnterprisePanel.vue`（走 `src/api/experts.api.js:47-54`）与 `composables/workspace/useGraphCanvas.js`（被 `ExpertWorkspaceView.vue:302` 消费）。逐处请求与响应键同 `experts_graph.rs` / `experts_common.rs` 比对：

| 存量做法 | 出处 | 后端真实契约 | 后果 | 模块处置 |
|---------|------|-------------|------|---------|
| `findOptimalTeam({ question, size: 3 })` | `ExpertEnterprisePanel.vue:869-872` | `OptimalTeamBody` 只有 `required_skills / required_domains / max_members / min_rating / constraints / goal`，两个键都**无别名** | serde 静默丢弃 → 后端收到空体，恒返回 0/0 覆盖的空团队且**不报错**：「最优团队」从未真正组过队 | `optimalTeamBody()` 只发真名，`constraints` 刻意不发；`GraphTeamPanel` 表单由 `OPTIMAL_TEAM_FIELDS.filter(mounted)` 生成 |
| 统计卡读 `graphStats.communities` 与 `graphStats.type_distribution` | 同上 `:59,65,82` | `/stats` 顶层无这两个键（社区在 `/communities`，类型分布根本不存在） | 社区数恒 0、类型分布条恒空；且面板从未调用 `getExpertGraphCommunities` | `normGraphStats` / `normCommunities` 分别对齐各自 handler，社区卡显式标 `algorithm: label_propagation` |
| `graphStats.density * 100` 无空值判定 | 同上 `:55` | 首次加载前 `density` 为 undefined | 首帧渲染出 `NaN%` | `statCells` 统一 `num()` 归零，且契约断言「后端给的键必须被读、读的键必须后端给过」 |
| 节点取 `node.type` 上色 | 同上 `:199` + `getTypeColor()` 回退 `#64748b` | 节点键是 `node_type`，取值只有 `expert` / `domain` | 类型永远落回默认灰，域/专家视觉上不可分；裸 hex 违反 §3.1 | `normGraphNode.nodeType` + `GRAPH_NODE_TYPE_META`，配色全走主题令牌 |
| `getNodePosition()`：`idx % 3` 三环 + 百分比定位 | 同上 `:846-853` | 后端不返回坐标 | 布局与图结构无关，同图每次刷新不变但换个顺序就全变，无法快照 | `model/layout.js` 确定性环簇布局，两次运行坐标逐点相同（变异体 G9/G10/G11 守） |
| `graphData.nodes?.slice(0, 30)` + 「仅展示前 30 位」 | 同上 `:197,204` | — | 截断而不给总数，用户无从知道少了谁 | 画布画全量（SVG 无第三方依赖），协作者列表按 `total_collaborators` 显式标注被截断 |
| `rebuildExpertGraph()` 后只 `loadGraphStats()` | 同上 `:790-798` | rebuild 返回 `new_version` | 图与社区停在旧数据，版本号与画布不一致 | `store.rebuild()` 重取 图 / 统计 / 社区 三份；失败保留旧图（G14 守） |
| `getExpertGraphCollaborators(node.id, 5)` 对任意节点调用 | 同上 `:859` | 域节点无 `collaborates_with` 边 → 200 空表 | 白跑一次请求，界面显示「无协作者」误导为「该域没协作关系」 | 仅 expert 节点请求（G12 守），域节点显示「该维度不适用」 |
| `x: n.x \|\| 200 + Math.random() * 400`、`y: n.y \|\| 100 + Math.random() * 300` | `useGraphCanvas.js:51` | 后端从不发 `x/y` | 每次加载全图重排，无法测试也无法对照；默认 `currentLayout = 'force'` 那条分支还是空的（`{ /* force layout uses real graph data */ }`），所谓力导向并不存在 | 模块图谱页不引 `3d-force-graph`（构建产物里那个 1.3 MB chunk 属 `views/graph/GraphView.vue` 的知识图谱页，与本模块无关），坐标一律由 `model/layout.js` 算出 |

不移植清单（有意收敛，不是遗漏）：`useGraphCanvas.js` 的 `switchLayout/applyLayout` 四布局状态机（force/radial/hierarchical/circular，其中 force 为空分支）与 `selectNode` 里靠坐标相等反查边的 O(n) 写法；`ExpertEnterprisePanel.vue` 的 `type_distribution` 与 `graphStats.communities` 两处假字段消费。

该面板此前三块**后端真实存在但未挂载**的能力现已全部有了落点：会话面由 `/alliance/sessions` 完整承接（逐字段证据见 §7.4），调度器只读状态由控制台的「调度状态与负载重置」卡承接（§3.2，且新卡是按真实出参键集渲染的，不像这块面板一样读三个不存在的键），编排面由 `/alliance/orchestration` 承接（§3.2 末条 + §7.5 A 档第 3 行）。也就是说它**已无"模块侧缺能力"这一类障碍**，剩下的只是删除需用户点名（§7.5 第 3 条）——并且要注意方向：存量面板把模拟步骤当结论显示，模块版把它们标了出来，所以"能显示"不等于"覆盖等价"，退役时以模块版为准。

### 7.4 存量会话实现核对表（第 4 批退役前置证据，2026-09-23）

会话 11 个端点在存量里有三处消费方：`views/expert/panels/ExpertEnterprisePanel.vue` 的「会话中心」（主用户）、`views/project/Workbench.vue:61-93`、`views/workspace/ExpertWorkspaceView.vue:419-442`，全部走 `src/api/experts.api.js:20-34`。逐处请求与响应键同 `experts_session.rs` / `experts_common.rs` 比对：

| 存量做法 | 出处 | 后端真实契约 | 后果 | 模块处置 |
|---------|------|-------------|------|---------|
| 11 个端点只挂了 2 个（列表 + 创建，共 5 处调用） | `ExpertEnterprisePanel.vue:669, 743, 803`、`Workbench.vue:63, 86`、`ExpertWorkspaceView.vue:422`；`experts.api.js:20-34` 另外 9 个导出（detail / stats / update / delete / messages / similar-search / semantic-search / export / archive，含 `listExpertSessions` 这个 `@deprecated` 别名）在 `src/**` 全库**零调用** | 11 个 (path, method) 全部 `ready` 且已登记 | 会话只有"看一眼列表 + 建一条"：读不到正文、发不出消息、改不了状态，也没有全域检索 | `/alliance/sessions` 五块面板覆盖全部 11 个端点，列表 / 详情 / 统计 / 检索四区独立记账 |
| 类型标签读 `s.mode`，映射表写 `smart / multi_expert / algorithm` | 同上 `:156` + `:566-568` | 字段名是 `session_type`，取值只有 `single / multi / debate / enterprise`（`ExpertSession` 字段文档注释）—— `mode` 这个键从不存在，`smart` / `multi_expert` / `algorithm` 也都不是合法值 | 标签恒落回原始值；新建发的 `{ title, mode: 'smart' }` 里 `mode` 被 serde **静默丢弃**，入库实为 `single`，界面却显示"智能路由" | `SESSION_TYPES` 与后端文档注释等集守卫；创建只发 `session_type`，等于缺省时不发（变异体 S2） |
| 更新时间读 `session.updated_at` | 同上 `:157, 813` | 时间戳是 `created_at` / `last_active_at` / `archived_at`，没有 `updated_at` | `formatTime(undefined)` 渲染成空串或 Invalid Date | `normSession` 只读后端给的三个键；格式化收敛到 `model/display.js` |
| 消息数与"最后一条提问"取 `session.messages` | 同上 `:813, 820-822` | `session_to_list_view:113` 剥掉 `messages`，只给 `message_count` | 列表里恒为 `0` / 「暂无消息」，弹出的"会话详情"也只有空值——点开的每条会话都看起来是空的 | 「打开会话」补一次 `GET /sessions/:id`（`store.selectSession`），列表用 `messageCount`；契约断言列表视图 13 键**不含** `messages` |
| 会话详情用 `ElMessageBox.alert({ dangerouslyUseHTMLString: true })` 手拼 HTML | 同上 `:812-816` | `create_session` 对 `title` 不做任何校验或转义，直接落库 | 标题里的 `<img src=x onerror=…>` 会被当 HTML 执行——这是**存储型 XSS 面**，属"不移植"而不是"待补" | 模块一律 Vue 文本插值，全模块无 `v-html`；渲染不依赖 innerHTML，样式契约也不放宽 |
| 顶部两张卡用本地数组算 `sessions.length` 与 `status === 'active'` | 同上 `:581, 587` | `GET /sessions/stats` 返回 11 个键（含 archived / closed / `session_type_distribution` / `top_experts_by_sessions`） | 卡片数字实为"当前页条数"，归档数、关闭数、类型分布、专家排行全看不到 | `SessionStatsPanel` 走 `sessionStats()`，`normSessionStats` 与 handler 键集双向等集，后端是 map 的 `session_type_distribution` 归一成数组才不会漏类型 |
| 请求失败即 `sessions.value = []` | 同上 `:749-750` | — | 一次网络抖动把已展示的列表清空，且说不出错在哪一区 | 失败只清本区并写 `error.list`，统计与已打开详情原地不动（store 单测守） |
| 列表响应形态三处各猜一遍，没有一处与真实键集相同 | `ExpertEnterprisePanel.vue:673, 748`（兼容裸数组）、`Workbench.vue:64`（`data?.list \|\| data?.items \|\| data?.sessions`）、`ExpertWorkspaceView.vue:423-425`（只认 `res.data` 是数组，或 `res` 本身是数组） | `list_sessions:251-256` 只返回 `{sessions, total, page, page_size}`，经 `http.js:97` 拆信封后拿到的就是这个对象 | 前两处靠猜中某个别名侥幸出数（`Workbench` 命中第三个别名），`ExpertWorkspaceView` 两个分支都不成立 ⇒ 协作会话面板**恒为空列表**，且成功路径不抛错所以连错误提示都没有；三处都不读 `total`/`page_size`，翻页能力从入口处就丢了 | 归一化只认 `flat` 信封，层数由 §4 裁决表 + `contract` 断言守卫；键集与 `json!` 字面双向等集，`total`/`page`/`page_size` 进归一结果，分页控件按真实总数渲染 |
| 请求参数按"后端应该会懂"来发 | `ExpertWorkspaceView.vue:422` 的 `project_id`、`Workbench.vue:86` 的 `type: 'workbench'`、`ExpertEnterprisePanel.vue:803` 的 `mode: 'smart'` | `list_sessions:197-201` 只读 `status`/`session_type`/`expert_id`/`user_id`/`search`（外加 `parse_pagination` 的 `page`/`page_size`/`limit`），**没有** `project_id`；`CreateSessionBody:38-53` 只有 `title`/`expert_ids`/`user_id`/`session_type`/`topic`/`tags`/`metadata` | `project_id` 被 `Query<HashMap>` 静默收下再无人使用 ⇒ 页面写"本项目会话"，实为全库会话（`http.js:190` 还会给每个请求自动注入 `project_id`，让这层假作用域看起来更像真的）；`type`/`mode` 不在 body 结构里 ⇒ serde **静默丢弃**，入库落回缺省 `single` | 查询键与 body 键都从后端源码提取并双向等集（不在名集里的键不发）；会话页在筛选区明示可过滤维度只有这五个，不展示"按项目过滤"这种后端不支持的开关 |
| 失败或空列表时伪造本地会话顶上 | `Workbench.vue:58, 78-93`（`sess-<ts>-<seq>` 假 id，创建失败仍用假 id 继续）、`ExpertWorkspaceView.vue:435-441`（`newCollaboration` 直接 `unshift` 一条纯本地会话并提示"已创建"） | `create_session` 成功才返回带 `id` 的会话；没有落库的 id 在任何后续端点上都是 404 | 用户看到的"会话"没有一条在后端存在：刷新即消失、无法检索、无法导出，界面却已提示创建成功 | 创建只走 `createSession()` 并等待返回，失败写 `error.create` 且不往列表插任何东西；行 id 一律取后端返回值 |
| 搜索框只在前端过滤已加载页 | `ExpertEnterprisePanel.vue:618` 本地 `filter` | `list_sessions` 支持 `search`（`title`/`topic` 小写包含）与 `status` / `session_type` / `expert_id` / `user_id` 五个过滤参数 | 用户以为在搜全库，实际只搜了当前 20 条 | 过滤条件进 `SESSION_QUERY_KEYS` 与后端 `params.get(...)` 名集双向等集；空串一律不发（S1），页面明说搜索范围不含消息正文 |

不移植清单（有意收敛，不是遗漏）：`dangerouslyUseHTMLString` 的详情弹窗、`mode` / `type` / `project_id` 三个后端不收的假字段与 `smart / multi_expert / algorithm` 三个不存在的取值、`list || items || sessions || 裸数组` 的响应形态猜测、刷新即消失的本地伪造会话、本地"只搜当前页"的假搜索。

结论：新页覆盖存量「会话中心」**全部有后端支撑**的能力，并且把 9 个从未被调用的端点导出第一次做成界面。三处消费方里，`ExpertWorkspaceView.vue` 的协作会话面板与 `Workbench.vue` 的会话侧栏都属于"看着能用、实际读不到真数据"的那一类，`/alliance/sessions` 是它们唯一的行为超集。删除 `ExpertEnterprisePanel.vue` 的会话块仍属破坏性操作，需用户确认后执行（与 §7.3 的调度状态面一并处理）。

### 7.5 存量联盟面独有能力差集（退役决策输入，2026-09-24）

用户裁定「先做能力差集再定」，所以本节只列差集与判定，不含任何删除。逐条按**是否阻塞退役**分三档，全部经源码复核（`grep` 到行），不是按页面名字猜的。

**A 档｜联盟域内、模块确实没有**

| 能力 | 存量唯一出处 | 后端 | 台账 | 阻塞谁 |
|------|-------------|------|------|--------|
| 专家注册表单（`POST /api/experts`）| `views/expert/ExpertConfigView.vue`(5414) + `constants/expert.constants.js` 的 `RegisterExpertDialog` | 真实 | ✅ **已补齐（2026-09-24）**：字段清单不再是照抄存量页，而由 `contract/registry.js` 从 `merge_expert_from_value` 现场解析生成（17 项挂载 + 6 项刻意不挂载 + 别名各按类声明）；`store.registerExpert` → 广场「注册专家」弹窗 | 曾阻塞 `/expert-config`，现无阻塞 |
| 专家改 / 删（`PUT`、`DELETE /api/experts/:id`）| 同上 | 真实（**删除是软删且单向不可逆**） | ✅ **已补齐（2026-09-24）**：`saveExpert` 走合并式 patch（只发改动过的键，零改动不发请求），`removeExpert` 前摆 `deleteConsequences()` 后果清单 | 曾阻塞同上，现无阻塞 |
| 编排器面 `/experts/orchestrate` + `/plan/generate` + `/plan/execute` + `/orchestration/{stats,history}` | `views/expert/panels/ExpertOrchestratorPanel.vue:259-262` | 真实但**半假**：拓扑是真 Kahn，步骤结果出自 `simulate_step_execution` 硬编码文案、`confidence` 恒 0.85；`plans` 与 `orchestration_history` 都是进程内结构，重启即失 | ✅ **已补齐（2026-09-25，任务 #22）**：五条进 `ENDPOINTS`（`contract/endpoints.js:69-73`），假的部分由 `contract/orchestration.js` 逐字段记账并在界面带角标呈现；`plugins` 一条仍是 `rejected`（:158） | 曾阻塞 `/expert-center/orchestrator`，现无阻塞 |
| 调度器复位 `experts.dispatch.reset` / `reset_all` | 存量无界面 | 真实、**破坏性**（`current_load` 归零并删熔断计数；未知 id 也回 `reset:true`，无 404） | ✅ **已补齐（2026-09-24，任务 #15）**：控制台 `.acks-card` 一次接进读数侧（`GET /api/experts/dispatcher/status`）与两个复位入口；破坏性由界面上的 `RESET-ALL` 打字确认闸 + 五条带 `:行号` 的后果清单承担（后端只认证不授权，不会有第二道闸） | 不阻塞存量页；曾阻塞控制台成为完整调度面，现已解除 |

A 档的四种「后端有」不是同一种可信度：注册与改删是真能力，编排器面有一半是模拟（所以它挂载时带的不是"又一个页面"而是逐字段来源角标与常量图例——假的那部分被接进来，但没有被藏起来），复位面是真能力但**破坏性**——接进来时按要求带了二次确认，且它的回执被如实写成不证明任何东西：未知 id 也回 `reset:true`，所以界面只说「后端确认已重置」不说「该专家存在」。台账 `reason` 字段与契约常量里逐条写明了这些边界，界面文案不得越过它。

**B 档｜住在联盟命名的页面里，但不属联盟域**（退役时要**搬家**而不是删掉，也不构成本模块的欠账）：白板（`useWhiteboard.js:109` + `POST /whiteboard/:id/save`，模块内 `whiteboard` 零命中）、项目文件上传 / 预览 / blob 下载、`/workspace/kpi` 与 `/workspace/history`、`/notifications/unread-count`、`/kb/*` 五件套（同一能力在 `/resources/knowledge` 已有权威页面，`router/modules/alliance.js:14` 只是复用了那条路由）、知识图谱分析族 `/graph/{centrality,pagerank,communities,activate,auto-sync,export,import,ai-insights}`（`api/graph.api.js`，与专家协作图 `/api/expert-graph` 是两个域，模块只承接后者）。

**C 档｜前端自造或死线**（直接支撑退役，不是欠账）：`views/workspace/ExpertWorkspaceView.vue:882-885` 监听的四个快捷动作事件 `mox:open-register-expert` / `mox:open-expert-debate` / `mox:open-multi-consult` / `mox:smart-route-expert`，**全库零发射方**（`dispatchEvent(new CustomEvent('mox:open-…'))` 只出现在 `App.vue:161,168,171` 的 market-upload 与 create-task 上）——也就是说存量工作台的四条招牌动作从任何地方都触发不了；四布局模式与 `localStorage 'expert_workspace_mode'`、Ctrl+K 全局搜索、投票 / 轮询 UI 同为纯前端偏好件；`GET /api/ai/engine/alliance/full` 与 `/capabilities` 走编排器 :3001 而非网关 :3080，已钉在 `FORBIDDEN_ENDPOINTS:124-125`；`stores/alliance.store.js`(628) 复核为**零 importers**（仅 `contract.test.js` 把它当源码事实读），随时可删。

**已确认等价、无需差集**：联盟任务台账面（`AllianceTaskView.vue` 934 行 → 控制台的 list / detail / nodes / plan / logs + 两个动作面），含 SSE 日志流——`AllianceConsoleView.vue:537` 用的正是 `allianceApi.taskLogStreamUrl(id)`，与存量 `/alliance/tasks/:id/logs/stream` 同源；六模式协作、`/expert-graph` 全家、会话 11 端点分别见 §7.2 / §7.3 / §7.4。

**退役判定（供确认，未执行）**：

1. 可立刻退役且无能力损失：`stores/alliance.store.js`、`ExpertWorkspaceView.vue:882-885` 那四条死监听、以及 `/expert-plaza`（其全部有后端支撑的能力已由 `/alliance/experts` 覆盖，见 §7.1）。
2. 须先搬 B 档再退役：`/expert-workspace` —— 它仍是白板、项目文件、KPI 与历史四条能力的唯一宿主。
3. A 档第一行（注册 + 改删面）**已于 2026-09-24 补齐** ⇒ `/expert-config`(5414) 的退役前提成立，只等用户点名；`/expert-center` 的三个块中 `orchestrator` 也已于 2026-09-25 由 `/alliance/orchestration` 承接（A 档第 3 行），`overview` / `enterprise` 两块已由 §7.3 / §7.4 覆盖，`tasks` 已等价——**该页四块能力全部有了模块侧落点，退役障碍从"能力缺"变成"只等点名"**（删除仍是破坏性操作，未经用户逐条指名不动，尤其因为模块版把模拟部分显式标注了，而存量面板没有）。
4. 另有一处**硬引用**必须先处理：`views/project/Workbench.vue:43` 直接 `import FlowGraph from '@/views/graph/FlowGraph.vue'`，与本模块无关但会被图谱侧的任何目录清理打断。

删除任一路径或组件都属破坏性操作，等用户点名到具体条目再动。

## 8. 验证

```bash
cd frontend-ui
npx vitest run src/modules/expert-alliance   # 533 例 / 18 文件：跨语言契约 99 / 注册写面契约 32 / 图谱契约 27 / 会话契约 37 / API 58
                                             # / 调度配置与状态契约 45 / 编排面契约 48 / 模式拓扑契约 18 / 广场 store 34 / 广场组件 21 / 协作 store 16
                                             # / 协作面板 17 / 图谱 store 15 / 会话 store 30 / 榜单 10 / DAG 布局 11 / 样式契约 9 / 登记 6
npx vitest run src/modules                   # 539 例 = 模块内 533 例 + 装配顺序守卫 6 例
npx vitest run                               # 全量回归（616 例 / 26 文件）
npx vite build                               # 生产构建（最近一次 2026-09-25 通过，built in 29.08s；时长随缓存浮动，不作为门禁）
python scripts/check-collab-mutants.py       # 变异电池：138 个指名变异体（可分片：`... M1,M2` 或 `... X1,X6`）
```

**断言的可靠性由变异电池证明**（`frontend-ui/scripts/check-collab-mutants.py`，串行执行、逐字节还原并校验哈希）：全库 **138 个指名变异体**覆盖协作契约、调度配置契约、样式契约、图谱契约、会话契约、覆盖台账、能力目录、两个动作面、**注册写面**、**调度状态/负载重置面**、**DAG 面**、**编排面**与**模式拓扑面**，逐个都被捕获，无存活、无 SKIP。2026-09-24 完成**一次跑满九系列的整跑**：**M13 / G17 / S14 / C6 / K6 / L14 / R8 / D19 / P8 = 105 个，捕获 105/105、异常 0、无 SKIP，每行都带 `restored`**（还原后哈希逐字节一致），整跑结束后模块树 56 个文件与跑前备份逐字节相同。编排面（Q 系列 16 个）于 2026-09-25 以分片跑法跑完：首跑 13/16 捕获、**3 个存活**，存活本身就是这一轮的产出——Q5 暴露了一条"常量与常量自比"的自指断言（已改为从 `experts_common.rs:880` 正则反查网关上界），Q9 暴露了零值夹具掩盖的错键读取（已改为 13 个键逐一要求取自自己的 wire 键，并把夹具改成互不相同的非零值），Q10 暴露了 api 层对五个新方法零覆盖（已补 URL/动词/请求体三对照用例）。三处修好后复跑这三分片为 3/3 捕获、异常 0，与首跑 13 个合起来记 **16/16**。**因此十系列合并整跑（121 个）从这一批改完起重新欠账**，与 §8 上表并列记账。模式拓扑面（X 系列 17 个）于同日一次跑满 **17/17 捕获、异常 0、每行 `restored`**，无存活、无 SKIP；但这一轮最值钱的三个判据是在写下变异体**之前**发现的：**① 析取式措辞断言**——`展示态拓扑|不是执行读数` 与 `不在任何响应里|不在响应里` 各自允许删掉另一半而仍绿（X6 把"不是执行读数"改成肯定式、X7 把"展示态"改成"执行态"、X16 把"分支不在任何响应里"念成"已在响应里"），三条都要求先拆成独立断言才红得起来；**② 装饰性引用**——行号原本只检查"字符串里含 `alliance.rs:4xx`"，改成**按大括号配对重算结束行后要求相等**，`planner.rs:332-401` 当场红：真实结束行是 411（X12 就是这个漂移的形状），这类错靠形状检查会带着一直绿；**③ 接线断言的位置**——`modeTopologyNote(` 出现在 SFC 里不能证明它被渲染，所以判据切到 `<script setup>` **之前**那段模板找 `dagModeNote`（X14 摘掉挂点即红，而脚本侧的 import 原样留着）；**同一条判据还差半步**——"渲染了"也不等于"读对了字段"，把 `modeTopologyNote(store.detail.task?.mode)` 换成 `store.detail.runtime?.mode`（那是本地预览开关，永远落到"未知模式"分支）此前不会有任何断言变红，X17 就是这一刀，判据现在直接钉到实参写法。**十一系列合并整跑（138 个一次跑满：M13 / G17 / S14 / C6 / K6 / L14 / R8 / D19 / P8 / Q16 / X17）自本批改完起重新欠账**。运行期间按本节末段的并发纪律不开第二个 `vitest`、不读写模块源、不动 Rust 与外壳样式；电池日志是块缓冲的（进程退出前一直是 0 字节），所以"跑完"只以 MUTATION_PENDING 标记消失 + 驱动进程退出为准，后台命令包装器提前报出的 exit 0 不能当作证据——本轮它就曾把退出码 1 的一次运行报成 exit 0。整跑之前的分片运行（七系列 78 个、D 系列 19 个、P 系列 8 个，均为 2026-09-24）已被那次 105 整跑取代。例数与文件数以上方代码块为唯一口径（本段不复述数字，避免两处计数各自腐烂）。

| 变异 | 断言 | 结果 |
|------|------|------|
| `constraintFields` 子键改成后端不读的名字 | 每个子键都被 `route_query` 的 `c.get()` 读取 | 捕获（5 例转红） |
| 约束无条件发出 | 等于后端默认值的子键不进请求体 | 捕获（2 例） |
| 去掉数值夹取 | 越界先夹到边界（99→5 / 9999→240） | 捕获（2 例） |
| 面板不再渲染约束行 | 路由模式呈现 3 条 `constraints.*` 说明 | 捕获（1 例） |
| 重新引入裸 `#hex` | 模块内无裸 hex/rgb/hsl | 捕获（1 例） |
| 重新引用未定义的 `--transition` | 每个 `var(--x)` 都有主题源定义 | 捕获（1 例） |
| 跨文件抢注他人块级类名 | 块级类名跨文件不共享 | 捕获（1 例） |
| `match_threshold` 上界放宽到 2 | 边界取后端 400 文案里的区间 | 捕获（3 例） |
| 策略项的 `checked` 改成 false | checked 与后端 400 分支一致 | 捕获（1 例） |
| `max_retries` 上界漂到 11 | 区间数值逐条核到后端文案 | 捕获（3 例） |
| 熔断默认值凭记忆改成 6 | 默认值取后端 `default_*` 函数体 | 捕获（1 例） |
| 策略清单塞进后端不认的值 | 策略清单与后端 valid 数组等集且同序 | 捕获（1 例） |
| 丢掉合并式更新的差异判定 | 与后端同值的键不进请求体 | 捕获（3 例） |
| 给 `constraints` 挂上界面 | `mounted` 集合等于 handler 里 `body.<key>` 的读取集合 | 捕获（1 例） |
| `max_members` 后端缺省 5→6 | 缺省值逐字取自 handler 的 `unwrap_or` | 捕获（2 例） |
| `goal` 与显式需求同时发出 | 二者互斥：同时填时不发 `goal` | 捕获（2 例） |
| 等于缺省的 `min_rating` 也发 | 等于后端缺省的值不进请求体 | 捕获（1 例） |
| 协作者 limit 缺省 10→12 | `limit` 缺省取 handler 的 `unwrap_or(10)` | 捕获（1 例） |
| 图版本号归一成字符串 | `version` 是数值而不是字符串 | 捕获（1 例） |
| 归一化读一个 handler 未产出的键 | 归一化不得读后端未返回的键 | 捕获（1 例） |
| 吞掉统计面里的 `density` | 后端返回的每个键都被读到 | 捕获（1 例） |
| 无归属专家落到中环 | 孤儿节点落在外环，不与簇重叠 | 捕获（1 例） |
| 节点半径去掉度数封顶 | 度数与半径单调且半径封顶 | 捕获（1 例） |
| 把没有 id 的条目也画进画布 | 后端塞进无 id 条目时不画它 | 捕获（1 例，见下段） |
| 域节点也去查协作者 | `collaborators` 只对专家节点请求 | 捕获（1 例） |
| 换 `limit` 顺带重取邻居 | 改 `limit` 只重取协作者 | 捕获（1 例） |
| 重建失败时清空已有图 | 重建失败不动已有图 | 捕获（1 例） |
| 协作者路径参数名写错 | 八个图谱方法的路径与动词精确对齐路由表 | 捕获（2 例） |
| `limit` 不经契约直接塞查询串 | `limit` 只有非缺省正整数才进查询串 | 捕获（1 例） |
| 组队请求体绕过契约 | 请求体由契约生成，camelCase 不漏进请求 | 捕获（1 例） |
| 空白过滤条件照发（S1） | 空串过滤一律不发：后端 `Some("")` 是真比较 | 捕获（2 例） |
| 与后端同值的 `session_type` 也占位（S2） | 等于后端缺省值的 `session_type` 不发 | 捕获（2 例） |
| 丢掉 `tags` 的数组判定（S3） | 合并式更新只发改动过的键 —— **这条就是当初的真实 bug 回归** | 捕获（1 例） |
| metadata 数字与文本不再等价（S4） | 与当前同值的键不发 | 捕获（1 例） |
| metadata 表单行被当成对象（S5） | metadata 行按字符串上送，不偷偷 `JSON.parse` | 捕获（4 例） |
| 统计三档判定反了（S6） | 写出第四个状态会在统计里隐形 | 捕获（7 例） |
| `rating` 丢掉 0–5 区间（S7） | `rating` 是 `Option<u8>`，区间来自字段文档 | 捕获（2 例） |
| 缺省 `top_k` 当自定义值发（S8） | 两个检索的 `unwrap_or` 字面量即前端缺省 | 捕获（2 例） |
| 全域检索发明 `min_score`（S9） | 两个检索 body 各自只有 3 个键 | 捕获（2 例） |
| 导出的 `null` 被归成空串（S10） | 不发明下载链接，能给的只有文本与文件名 | 捕获（1 例） |
| 详情假装有 `message_count`（S11） | 列表视图 13 键含 `message_count`、不含 `messages` | 捕获（3 例） |
| 追加成功后不并线（S12） | 后端只回那条消息，并线与计数由 store 负责 | 捕获（1 例） |
| 404 判定反了（S13） | `session not found` 清选中态，其他错误保留 | 捕获（1 例） |
| 无改动也发 PUT（S14） | 没有任何改动时一个请求都不发 | 捕获（1 例） |
| 台账里给一条定性换成非法值（C1） | 台账只认 `backlog`/`rejected` 两种定性 | 捕获（2 例） |
| 把一条理由删成两个字（C2） | 每条理由都要够长 | 捕获（1 例） |
| 理由不指后端源码位置（C3） | 理由必须带 `.rs:` 行号，可回溯 | 捕获（1 例） |
| 注册端点的方法漂成 GET（C4） | 已挂载端点的方法与路径逐字对齐注册表行 | 捕获（1 例） |
| 从台账里抹掉一条未挂载面（C5） | 每个 ready 行要么被挂载，要么在台账里定性 | 捕获（1 例） |
| 端点挂到查无此行的 id（C6） | 挂载集必须落在两域注册表内 | 捕获（3 例） |
| 能力目录在本地按人数重排（K1） | 后端按 `capabilities` 数组原序给出，乱序输入必须原样输出 | 捕获（1 例） |
| 把 40/30/30 权重搬到前端重算（K2） | 三个派生值由后端算，前端只做单位换算 | 捕获（2 例） |
| 派生指标的 404 被吞成「没有错误」（K3） | 后端 404 不折叠成零值 | 捕获（1 例） |
| 去掉派生指标的竞态守卫（K4） | 先请求的慢响应不得覆盖后请求的结果 | 捕获（1 例） |
| 目录面板改口称支持按 `capability_id` 精筛（K5） | 说明文字承认后端不接收 `capability_id` | 捕获（1 例） |
| 分组把后端给的 `domains` 换成本地字母序（K6） | 分组次序沿用后端 | 捕获（2 例） |
| 把任何响应都当成远程分支（L1） | 方向只由 `toggled`/`success` 判定，缺证据就是 `unknown` | 捕获（3 例） |
| `toggled` 的两个方向对调（L2） | `toggled=false` 对应后端把状态改回 `Running` | 捕获（3 例） |
| 归一化读两条分支都不产出的键（L3） | 前端读的恰是两条分支 `data` 键集的并集 | 捕获（1 例） |
| 标记完成后只刷任务不刷节点（L4） | 本地整批置完成会改执行表并落盘，节点必须一起重取 | 捕获（2 例） |
| 丢掉「切了任务就不写详情」的守卫（L5） | 等待期间切了任务就不把上一个的状态写进详情 | 捕获（2 例） |
| 替后端臆造一个 `constraints` 消费方（L6） | 实跑只发 handler 读取的键，照收不读的键不发 | 捕获（3 例） |
| 不再拦空需求描述（L7） | 空串会让后端把领域匹配对全员判满分 | 捕获（3 例） |
| 智能匹配关闭的指纹漂 0.5→0.6（L8） | 判据跟着后端字面量走，不跟记忆走 | 捕获（1 例） |
| 把后端回退当成「按所选策略执行」（L9） | `specified` 分支不算策略生效，`fallback` 单独定性 | 捕获（1 例） |
| 一次失败把已有实跑结论擦成空白（L10） | 无可用专家是 503 而非空结果，失败留着上次结果 | 捕获（2 例） |
| 远程重开按钮不再挡（L11） | 远程已完成任务经网关重开回 409，界面先挡住而不是等报错 | 捕获（1 例） |
| 实跑请求体绕过契约（L12） | 请求体由 `dispatchRunBody` 生成，camelCase 不漏进请求 | 捕获（2 例） |
| 界面改口称实跑会累加负载（L13） | 分发只选人，不改动任何专家的 `current_load` | 捕获（1 例） |
| 界面改口称 `task_type` 可省略（L14） | 该键无 serde 缺省，不发会被整段 JSON 拒绝 | 捕获（1 例） |
| 熟练度上界放宽到 999（R1） | `proficiency` 在 Rust 侧是 `u8`，越界整条静默丢弃 | 捕获（1 例） |
| `EXPERT_U32_MAX` 写成 `0x7fff…`（R2） | 时薪超 `2³²−1` 必拒：后端 `as u32` 截成另一个数 | 捕获（1 例） |
| patch 丢掉差异判定（R3） | 只发改动过的键，未动的数组不发（后端整值替换会清空） | 捕获（8 例） |
| 后端没回 `created/expert` 也当成功（R4） | 注册结果不可信就不乐观收单：行不增、列表不刷 | 捕获（1 例） |
| 身份提示改口成「仅管理员可操作」（R5） | 只认证不授权是源码事实，界面不得替后端编造角色判定 | 捕获（2 例） |
| 注册弹窗不再留痕（R6） | 写失败必须留在发起它的弹窗里，toast 会自己消失 | 捕获（1 例） |
| 关闭错误横幅清的是 `notice`（R7） | 关掉的必须是这条错误本身，成功提示不该被顺手抹掉 | 捕获（1 例） |
| 开面清旧错写成 `reg && dis`（R8） | 两个写弹窗各自开面都要清旧错，否则串台 | 捕获（1 例） |
| 状态键集里丢掉 `avg_dispatch_ms`（D1） | status 出参键集与后端 `json!` 十键等集 | 捕获（1 例） |
| 归一化改读后端不返回的 `timestamp`（D2） | 同上：归一化不许读没有的键 | 捕获（1 例） |
| 平均耗时被写成常量 0（D3） | 同上：后端给的每个键都要落到界面上 | 捕获（1 例） |
| 全量回执的被重置名单归成空数组（D4） | 两种回执互不混形 | 捕获（1 例） |
| 熔断空列表不再解释（D5） | 空列表要说成「没有数据」，不能说成健康 | 捕获（1 例） |
| 关掉「无样本 ⇒ 1.0 是默认值」的分支（D6） | 无终态样本时的 1.0 不是 100% 成功 | 捕获（1 例） |
| `reason` 不再 trim（D7） | 两个 handler 的请求体要求相反 | 捕获（1 例） |
| 全量重置的确认词门槛恒真（D8） | 后端只认证不授权，输入 `RESET-ALL` 才放行 | 捕获（1 例） |
| `engine_status` 被说成探活结论（D9） | 它是后端字面量，界面必须自陈 | 捕获（1 例） |
| 界面重新读 `circuit_breaker?.states`（D10） | 界面对 status 伸的每次手都要落在归一化产出的键集里（结构核，非点名） | 捕获（1 例，第一版曾存活） |
| 单专家重置不发体（D11） | 必发 `{}`：空体被 axum 拒成非信封错误 | 捕获（1 例） |
| 全量重置开始塞 body（D12） | 不发体：handler 没有 body 提取器 | 捕获（1 例） |
| 重置失败时抹掉上一次读数（D13） | 失败只写 `error.reset`，读数与回执原地保留 | 捕获（1 例） |
| 本地替后端把负载归零（D14） | 重置后必须重取，不许先替后端归零 | 捕获（1 例） |
| 全量回执打成单专家 scope（D15） | 回执要能分清是哪一次动作 | 捕获（1 例） |
| 后果清单抹掉源码坐标（D16） | 五条齐全且每条带 `:行号` | 捕获（1 例） |
| 回执改口称它证明该专家存在（D17） | `reset:true` 不是存在性证据（未知 id 也回 true，无 404） | 捕获（1 例） |
| 被重置人数不再引 `reset_count`（D18） | 人数来自后端计数，前端不估 | 捕获（1 例） |
| 单专家重置路径漂成 `reset-all/:id`（D19） | 路径与注册表行逐字对齐 | 捕获（3 例） |
| 页脚退回拿后端 `edges` 数组当依赖数（P1） | 分层与依赖计数同源，算法住 model 层 | 捕获（1 例） |
| 悬空依赖不再单独记账（P2） | 悬空依赖参与声明数但不参与分层 | 捕获（1 例） |
| 回边不再计数（P3） | 环不抛错、不丢节点，并计入 `backEdges` | 捕获（2 例） |
| 后端 edges 与依赖清单不再 1:1 时不露差值（P4） | 差值 `edgeDelta` 必须算得出 | 捕获（1 例） |
| 取消态并进跳过一格（P5） | 跳过与取消分开计数，不并入一个桶 | 捕获（1 例） |
| 取消态节点样式漂成错误拼写（P6） | `NodeExecStatus` 每变体都有 wire 出口、中文标签与 DAG 节点样式 | 捕获（1 例） |
| `normDag` 读后端 DAG 出参没有的 `stats.cancelled`（P7） | DAG 出参三处键集与前端读取一一对上 | 捕获（1 例） |
| 界面把折叠的 `stats.skipped` 当"跳过"人数（P8） | `node_stats` 把 skipped 与 cancelled 折叠成第 6 格，界面按节点级分开计数 | 捕获（1 例） |
| 兜底表步数凭记忆写成 5（Q1） | 各表步数与契约记法一致（development 与兜底各 6 步） | 捕获（1 例） |
| 常量清单里 `confidence` 字面量与源码脱钩（Q2） | 字面量逐条命中所声明的行号 | 捕获（1 例） |
| 执行器传空专家表的坐标漂走（Q3） | 同上：行号漂了说明这一面又变了 | 捕获（1 例） |
| 恒 0 计数器从契约里少报一把（Q4） | `plan.status` 只有三处赋值，ready/failed 两个计数没有写入路径 | 捕获（2 例） |
| 历史 `page_size` 上限从网关口径 200 漂成 500（Q5） | 上限取 `experts_common.rs:880` 的 clamp 值，不是取契约常量自比 | 捕获（修复自指断言后复跑） |
| 取消请求侧夹取（Q6） | 请求侧夹到 1..=200，缺省回落后端默认 20 | 捕获（1 例） |
| `step_ids` 空数组改成"空则省略"（Q7） | `Some(空集)` 与 `None` 是两种语义，不许像 `expert_ids` 那样省略 | 捕获（1 例） |
| `max_experts` 用真值判断（Q8） | `max_experts=0` 必须发出去（0 与"不发"是两种结果） | 捕获（1 例） |
| 归一化把 `plans_ready` 读成 camel 键（Q9） | 13 个直译键逐一要求取自自己的 wire 键 | 捕获（夹具改非零互异值后复跑） |
| 历史请求丢掉分页参数（Q10） | api 层把 `orchHistoryQuery` 的结果真的发出去 | 捕获（补 api 覆盖用例后复跑） |
| 历史路径漂成 `/orch/history`（Q11） | 五条注册表行的路径与方法逐字对齐 docs/API-REGISTRY.md | 捕获（4 例） |
| 把已挂载的编排行塞回台账 `backlog`（Q12） | 本面 5 行已移出，backlog 归零 | 捕获（1 例） |
| 编排台重新引入裸 hex 颜色（Q13） | 模块内无裸 hex/rgb/hsl | 捕获（1 例） |
| 角标挂到未登记的字段路径（Q14） | 界面每个带角标的路径都在来源清单里登记过 | 捕获（1 例） |
| 常量与模拟字段图例在界面上不再展开（Q15） | 图例转发整张 `ORCH_SIMULATED`，不筛不减 | 捕获（1 例） |
| store 把图例筛掉一条（Q16） | 同上：simulated 不能从那本账上消失 | 捕获（1 例） |
| dynamic 节点数按"看起来该多一个"写成 5（X1） | 每模式的节点数、Running 个数与按序节点名逐字对齐 `build_dag_for_task` 解析出的分支 | 捕获（2 例） |
| sequential 的节点名抄成 parallel 的名字（X2） | 同上：两个模式在界面上不得长成同一张图 | 捕获（1 例） |
| voting 的三路并发记成一路（X3） | 同上：那句"N 个恒记 Running"是照本表说的 | 捕获（1 例） |
| Pending 计数漏减恒 Completed 的首节点（X4） | Pending = 节点数 − 1 − Running，与 Running+Completed 加起来等于节点数 | 捕获（1 例） |
| 说明里的 `alliance.rs` 坐标整体漂走（X5） | 两截坐标逐行命中契约注释与函数本体，结束行按大括号配对重算 | 捕获（2 例） |
| "不是执行读数"改成肯定式（X6） | 措辞必须点出这不是进度——原为析取式，拆独立断言后才红 | 捕获（1 例） |
| "展示态拓扑"写成"执行态拓扑"（X7） | 措辞必须点出这是模板——析取式的另一半 | 捕获（1 例） |
| 未知模式不再指名那个值（X8） | 表里没有的模式要把后端原值印出来，不说"见后端日志" | 捕获（1 例） |
| 末节点契约名凭印象写成"融合结果"（X9） | 首末契约位比对的是 Rust 解析出的节点名，不是常量自比 | 捕获（1 例） |
| `modeWireOf` 不认 `mode_display` 线名（X10） | 出参给的那条线（`expert_alliance` 等）也必须查得到拓扑 | 捕获（1 例） |
| 动态分支节点的 expert_id 记成 `expert-routing`（X11） | 网关侧「动态路由」确是一个普通 expert 节点 | 捕获（1 例） |
| `plannerAt` 的结束行按记忆写成 401（X12） | 引用坐标按大括号配对重算——真实结束行是 411 | 捕获（1 例） |
| `types.rs` 的枚举段尾写短，漏掉后两个变体（X13） | 那句"两根轴"引用的段必须框住全部七个变体 | 捕获（1 例） |
| 控制台把说明从 DAG 页签摘掉（X14） | 接线判据看 `<script setup>` 之前的模板段，不看脚本里的 import | 捕获（1 例） |
| 说明条重新引入裸 hex（X15） | 模块内无裸 hex/rgb/hsl，颜色一律取令牌 | 捕获（1 例） |
| "分支不在任何响应里"念成"已在响应里"（X16） | dynamic 的分支缺席必须被明说，界面才答不了"走了哪条" | 捕获（1 例） |
| 说明条改读 `runtime.mode`（X17） | 接线判据钉到实参：读的是任务的 `mode`，不是本地预览开关 | 捕获（1 例） |

覆盖台账这一批还顺带把后端的一处登记缺口补了：`POST /api/experts`（`create_expert`，`experts_registry.rs:839` 早已挂载）在 `actuator.rs` 里没有行，于是 `expertRegister` 只能借用 `experts.registry.list` 这个 GET 行做溯源。补登记 `experts.registry.register` 后 `ROUTES` 为 **228** 条，`docs/API-REGISTRY.md` 重新生成，`cargo check -p mox-platform-gateway-svc` 与 actuator 单测（7 例）通过。这与 §5 记下的调度配置面漏登记是同一类病：**路由挂在树上，注册表里查不到，前端就只能凭记忆写路径**——现在这种写法会被 C6 直接判红。

图谱批次里 G11 第一轮**存活**：`nodes.filter((n) => n?.id && positions[n.id])` 删掉后 26 例全绿。原因是放置对每个有 id 的节点都是全覆盖的（域在中环、专家在簇上或外环），`positions[n.id]` 永不落空 —— 那是一个不可能触发的防御分支，而当时唯一相关的断言 `graphLayout([], []).nodes == []` 对空数组恒真，属夹具自身不触发缺陷的那一类。做法是先删死条款、再把用例换成真正踩得到的形状（`[{ nodeType: 'expert' }, …]` 一条无 id 条目 + 断言 `junk.nodes` 只含有 id 的两个），随后 G11 改为「整个 filter 去掉」即转红。重跑 30/30。

调度配置一轮也复现了「夹具自己踩坑」这一类：解析 `#[serde(default = "...")]` 的属性回看最初写成跨字段正则（Rust 字段以逗号结尾，`[^;]*?` 会一路匹到结构体里第一个属性），于是七个字段全拿到 `default_strategy` 的值，两条用例同红而产品代码无误。改成按行回看后两条转绿，变异电池重跑通过。同一形状在图谱批次又出现一次（见上段 G11），两次的教训一致：**先问「什么输入能让这条断言变红」，再决定断言写在哪**。

调度状态这一批抓到的不是夹具、是**一条假守卫**（D10 第一次跑存活）。那条断言的本意是"控制台不许重蹈存量面板覆辙"，写法却是指名点姓地禁止三个字段名：`circuit_breaker\.states`、`recent_dispatches`、`cb\.status`。变异体把界面改成读 `store.dispatcherStatus.circuit_breaker?.states?.length` —— 45 例全绿。原因很直白：可选链在字段名中间插了一个 `?`，正则里那个 `\.` 就再也匹不上；而存量面板恰恰就是用 `?.` 写的（`ExpertEnterprisePanel.vue:106,605`），也就是说这条守卫从写下那天起就拦不住它声称要拦的东西。**否定式点名断言是脆的**：它只挡住了"照抄某个写法"，挡不住"换一种写法犯同一个错"。改法是把守卫换成结构式——从界面源码里解析出「对 status 读数伸的每一次手」（`dispatcherStatus\??\.\s*([A-Za-z_]\w*)`），逐个要求它落在归一化产出的键集内，键集本身再从 `normDispatcherStatus` 的返回对象里解析，不写死清单；三个字段名的点名断言保留，但补上 `\??` 以覆盖可选链。改完后 D10 转红（1 例），且凡改动 status 键集的变异体现在有两处独立判据（后端 `json!` 侧与界面侧）。同一教训回头检视 §7.5：旧广场页那份差集表用的是"组件名 + 行为"描述，不是结构核对，退役任何一条前都要按这条标准重写一遍守卫。

会话批次还暴露了电池自身的一个洞：一次运行在「写盘 → 跑测试」之间被外部杀掉，`contract/sessions.js` 就停在变异态——被改掉的恰好是 `tags` 的数组守卫（本轮刚修的真实 bug）。下一次运行把脏内容当基线，S3 报 `SKIP-锚点未命中`，而这副面孔很容易被误读成「这个变异体不适用」并就此放过。现在脚本在写盘前先立一张 `collab-mutants-pending.txt`（记文件名），还原并核对哈希后才撤；开局见到残留即中止并指名待核对的文件（实测拦下了第二次中断，且因 `dispatcher.js` 已被还原而只报警不脏写）。**这属于"工具被中断也会腐蚀证据"那一类**：还原核对只能证明"本轮没留下脏"，证明不了"上一轮死在哪一步"。

`SKIP-锚点未命中` 在这一轮还以第二种形式出现，且与脏写无关：C1 的锚点原本是 `experts.dispatch.dispatch` 那条 `backlog` 台账行，而该面本轮被挂载后这行合法消失，锚点自然未命中。教训是 **SKIP 从不表示「这条断言不适用」，只表示「靶子已经搬走了」**——挂载一个 backlog 面时必须同时重指或删掉以它为靶的变异体，否则台账上留的是一枚不会再响的哑弹，而"68/68 全部捕获"这类汇总数字会把它掩盖成一次通过。C1 现已重指 `alliance.tasks.fusion_alias` 的 `rejected` 行（该行是永久定性，不随挂载消失）。

同一次教训的另一半：**电池运行期间整棵模块树处于间歇变异态，任何并行的 `npx vitest run` 读到的都是被改过的源码**。本轮一次并行回归因此报了 3 例红，其中 `match_threshold` 上界那条正是 M8 的靶子——单跑同一文件 15/15 全绿，红的是并发读取的假象。判别方法不是"看哪条红了"，而是看 `%TEMP%\collab-mutants-pending.txt` 的 mtime 与大小是否还在增长：在长就说明电池还活着，此刻的一切测试结果都不能当证据；要么等它跑完（marker 自动清除），要么根本别开第二个 vitest。

页面：`/#/alliance/console`（控制台：任务与节点 / DAG / 融合 / 日志 + 调度配置卡内的「分发实跑」 + 任务动作行的「标记完成 / 重新打开」 + 「调度状态与负载重置」卡（真实读数 + 单专家/全量两个二次确认动作））、`/#/alliance/collab`（智能协作工作台）、`/#/alliance/experts`（专家广场，含头部「排行榜」弹窗与「能力目录」页签）、`/#/alliance/graph`（专家协作图谱：画布 + 节点检视 + 统计与社区 + 组队）、`/#/alliance/sessions`（专家会话：列表 + 线程与追加 + 元信息差分编辑 + 统计 + 全域字面检索）、`/#/alliance/orchestration`（专家编排台：三个动作 + 进程内统计与历史 + 常量图例，逐字段带来源角标）；**六项**在侧栏「专家联盟」下并列。后端不可达时按面板降级，不白屏（这一句现在有我自己的证据，见下段——但下段的浏览器核验发生在编排台挂载之前，**第六页尚无同等的真机降级证据**，记在任务 #11）。

联调基线（2026-09-23，网关 :3080 实跑）：广场以真实账号登录后，`GET /api/experts?search=…&page=1&page_size=24`、`POST /api/experts/bookings`、`PUT /api/experts/bookings/:id/cancel`、`GET /api/experts/bookings/:id/consult-room`、`POST /api/experts/:id/favorite`、`POST /api/experts/:id/consult-now` 全部 200 且渲染真实数据；服务端搜索「前端」得 1 位专家、`sort=name` 改变服务端次序。

榜单与 KPI 基线（2026-09-23 同一登录态）：顶部 6 卡取 `/api/experts/stats` 真实响应 —— `11 位 / 11·0·0 / 3561 次 / 1 次 / 4.4 / 5 分钟`；本页 11 位专家中 10 位有 `total_consultations`（榜首 357 次，其余 356 次按名称 zh-CN 定序），1 位因 0 次被排除；评分榜 10 位（榜首 4.8 / 128 人评）；新晋榜 11 位（榜首 2026-09-12）。三榜均按真实字段排名，控制台无新增报错（仅存量 `/api/health` 502 轮询与一次令牌刷新导致的 stats 401→刷新后 200）。

协作实跑基线（2026-09-23 同一登录态，`POST /api/experts/multi-consult`）：工作台上勾选 2 位专家发起多专家咨询，返回 200 并渲染真实融合结论 —— 共识度 `0.57`、2 位专家分别作答、会话号 `sess-multi-5aa0a89fcc06…` 同时进入本次运行历史。这是本模块第一次有真实数据的端到端协作证据；存量两份实现同一请求在 200 后崩于 `result.results.filter`（见 §7.2）。

调度配置、协作图谱、会话与两个新挂载的动作面四块的**真数据**验证仍止于契约与测试：落地期间网关 :3080 已停（`curl` 三处均为 `000` 连接被拒，既非 401 也非 404，无法据此判断路由存活），因此四块都**没有**实跑证据，会话页也从未在浏览器里对着真数据渲染过。下次联通后应核对：`GET /api/experts/dispatcher/config` 的真实字段面与 `PUT` 的合并语义；`GET /api/expert-graph` 的节点/边规模与 `stats.version` 递增；`GET /api/expert-graph/communities` 的 `algorithm`/`converged`；`GET /api/expert-graph/path/:s/:t` 在不可达时确为 `found: false`；`POST /api/expert-graph/optimal-team` 的 `coverage` 与 `selection_strategy`；会话面则按 `experts_session.rs` 的十条事实逐条对：`GET /sessions` 的 13 键投影与 `total/page` 反算、`GET /sessions/:id` 是否真的带全量 `messages`、`POST /sessions` 空 body 是否 200（`session_type=single`）、`PUT` 的 metadata 是否只能改不能删、`POST /sessions/:id/messages` 响应是否只有那条消息、`GET /sessions/stats` 的三档计数是否等于按 status 手数的结果、`similar-search` 与 `semantic-search` 在同一个词上的分数差别（前者是字面 bigram，后者也是——两者都不是语义模型）、`export` 的 `download_url` 确为 `null`、`archive` 之后 `status/archived_at/message_count` 三键、`DELETE` 二次调用是否为 404 文案；并把基线值登记在此。图谱与会话各另需人眼确认一处测试覆盖不到的东西：SVG 环簇布局在真实规模（数十节点）下的可读性与 1180px 断点下的单列降级；会话页五块面板在长线程（数百条消息）与窄栏下的排版。两个动作面另有一组只能实跑才能拿到的证据：`PUT /api/alliance/tasks/:id/toggle-done` 本地分支是否真回 `toggled` 并把非 failed/cancelled 节点整批置 `completed`（重取 `/nodes` 对比前后状态，这是 L4 那条"必须连节点一起重取"的唯一实证）、同一请求在远程模式对已完成任务是否确为 **409**、`POST /api/experts/dispatcher/dispatch` 的 `strategy_used` 是否随 `PUT /dispatcher/config` 的改动而变（这是"配置真的生效"的唯一实证，界面上的"与当前配置一致"全赖它）、全员离线或并发满时是否确为 503 而不是空 `assigned_experts`、以及 `intelligent_matching=false` 时 `match_score` 是否恰为 `0.5`。

**注册 / 编辑 / 停用面的降级态核验（2026-09-24 实做，同样 :3080 停机）**：注入 `mox_access_token` 后外壳守卫放行（这条本身就是 §5 那条「登录归路由守卫」的现场印证——不注入时 `/#/alliance/experts` 直接弹到 `/#/login?redirect=/alliance/experts`），逐项目视确认：

1. 注册弹窗的 17 项控件、四条整值替换警告（语言 / 领域 / 技能 / 标签）、u32 截断与 `proficiency` 手填警告、「本表单不录入：metrics · availability.current_load · …」这段与 `EXPERT_UNMOUNTED_FIELDS` 同源的清单，全部按契约渲染出来了；页脚那句身份提示与 `EXPERT_WRITE_IDENTITY.statement` 逐字一致。
2. 提交门是契约驱动的而不是 `el-form` 规则：`name` 为空时提交键 `disabled`，页面写「还不能提交 / 专家名称不能为空」；填入一位后同一 tick 内即转绿。
3. 发 `POST /api/experts`（探针记到 **恰好一条**，无重复提交）→ 网关 500 → 弹窗**不关**、用户已输入的草稿**不丢**，失败原因由新加的弹窗内错误条留痕（「专家未注册 / 服务端内部错误，请稍后重试（500）」）。这一条是本轮才补的：错误此前只落在页面级 `error.action` 横幅上，而弹窗盖在它上面，用户能看到的只有一条几秒后消失的 toast。
4. 同一轮把两处留痕缺陷也改了：页面级错误横幅的 `@close` 原本清的是 `store.notice`（把还没显示出来的成功提示顺手抹掉，而那条错误仍然压着它），现改为清 `store.error.action`；并且**开面即清旧错**——否则上一次收藏失败会在下一次打开注册弹窗时被读成「这次注册被后端拒了」。以上三条行为合起来由 `contract.test.js` 的一条结构断言钉住（含「失败即留在弹窗、成功才关窗」）。

**这一面的真数据核对清单**（网关恢复后补，全部是只能实跑才能拿到的证据）：`POST` 成功响应是否确为 `{expert,id,created}` 且新建专家立刻出现在 `GET /api/experts`；显式带 `id` 撞库是否回 400 `expert id already exists`（含撞上**已停用**的那位，因为 `contains_key` 扫的是整表）；`PUT` 是否真的只覆盖发出去的键（改 `title` 后 `domains` 应原样保留）且**不回 id**；对已停用专家 `PUT` 是否确为 404；`DELETE` 之后同一 `id` 是否在读接口全部消失、`GET /api/experts/:id` 是否 404、重新 `POST` 同一 `id` 是否被 400 挡下（这是「不可逆」唯一的实证）；以及 `hourly_rate_cents` 送 `2**32` 时后端是否真的截成 `0`（`as u32` 那条警告的实证）。

**调度状态与负载重置面的浏览器核验（2026-09-24 实做，同样 :3080 停机）**：分两步做，两步的证据各自只说自己能说的部分。

1. **真降级态**（网关 500，未注入任何数据）：`.acks-card` 照常渲染，卡头写端点与数据源（`GET /api/experts/dispatcher/status` + "记录数读进程内 dispatch_records（重启归零），负载读注册表内存态"），卡体只有一条 `服务端内部错误，请稍后重试（500）` 与「重取状态」按钮——**两个重置入口这时根本不存在**，"没有读数就不给按"落实成了"没有读数就没有按钮"，不是灰着一半。
2. **注入读数**（`pinia._s.get('allianceConsole').dispatcherStatus = {…}`，数据是我手写的、**不是后端返回的**，所以这一条只算渲染证据）：五格 KPI 按真实键位出数（`running` 那格旁边就是"这不是探活结果"的自陈、成功率格挂 `不含进行中的记录` 那条注、最近调度为空时写"本次进程内还没有"并单列读数时间），熔断区走上文那条"没有数据"的原话而不是"服务正常"，负载表两行分别渲染 `5 / 3`（超并发）与 `0 / 2`，目录里查不到的 id 显示"（目录里没有名字）"而不是留空。卡片文本里没有任何一个字段是这十个键之外的（这正是 D10 之后那条结构守卫的现场版）。
3. **二次确认闸与失败路径**（同一份注入读数下）：点「全量重置」开弹窗 → 五条后果清单全部带 `:行号`（`everyLineHasCoord: true`）、确认按钮初始 `disabled`、`placeholder` 就是 `RESET-ALL`，把 `RESET-ALL` 填进输入框后按钮才转可用；此时点确认（网关仍停着）→ **弹窗不关**、`store.error.reset` 被填、弹窗内出现 `重置未生效 / 服务端内部错误，请稍后重试（500）` 的错误横幅、`resetReceipt` 仍为 `null`、负载读数原样保留 `[5, 0]`（本地没替后端归零）。这一条同时是 §3.2 ⑤ 与 D13/D14 两条断言的现场印证。
4. **对比度抽查**（承接 #13 的令牌口径）：`.acks-title` / `.acks-cell-value` 12.14:1、`.acks-cell-label` / `.acks-honest` / 表格单元 5.62:1、`.acks-cell-hint` / `.acks-note` 5.07:1，全部压在卡面 `rgb(36,40,56)` 上过 AA；模块样式里没有任何裸 hex（`style.test.js` 九例 + M5 守）。跑完 `location.reload()` 复验：注入读数消失、回到第 1 条的降级形态，没在页面上留下假数据。

**这一面的真数据核对清单**（网关恢复后补，全部是只能实跑才能拿到的证据）：`GET /api/experts/dispatcher/status` 的 `engine_status` 是否恒为 `running`（那条字面量唯一的实证）、`success_rate` 在有终态记录后是否真等于 完成/(完成+失败)、无终态记录时是否确为 `1.0`（那个缺省值本身就是界面上「成功率 100%」的来源）、`avg_dispatch_ms` 是否只统计带 `completed_at` 的记录、`load_ratio` 在 `max_concurrent=0` 时给什么（那个除零分支走的是 `0.0` 还是 `NaN`）、`expert_loads` 是否真把停用者滤掉（对照 `GET /api/experts?status=disabled` 的人数）、`circuit_breakers` 在一次真实失败后是否仍为 `[]`（这是"没有写侧"这条判据唯一的实证；一旦出现非空，§5 那条与 `breakerEmptyNote` 的措辞必须立刻改）；`POST /api/experts/dispatcher/reset/:id` 对一个**不存在的 id** 是否确为 200 + `reset:true` + `previous_load:0`（界面上那句"不代表该专家存在"的实证），带 `reason` 时是否原样回填，不带体的空 `POST` 又是什么形状——应为 axum 的解码 400 且**不是**我们的 `{code,msg}` 信封（界面对这两型错误的解析路径不同）；`reset-all` 的 `reset_count` 是否等于注册表当前键数（含停用者，故应 **大于** 表内行数），`reset_expert_ids` 是否真把 disabled 专家列进去（`values_mut` 不过滤的实证）；重置后立刻 `GET status` 是否确实归零，再**重启网关**后是否又按最后一次落库快照恢复（"不落库"那条时序判据唯一的实证，也是二次确认清单第三条的真伪所在。**这条要重启网关，得先经用户点头，不在自主动作范围内**）。

**降级态的浏览器核验（2026-09-24 实做，:3080 停机 + :3020 dev 在跑）**：登录取不到令牌，就按网关真停着的样子过——往 `localStorage` 注入明文 `mox-token` 骗过路由守卫（`router/index.js:84-91` 在权限拉不到时 `console.warn` 后仍放行），逐页取 `main` 的文本与计算样式，跑完清除注入。结论两条：

1. **五页都渲染出了真实面孔，无一白屏**。控制台 81 条 warn/error 逐条看过，除网络 500 之外只剩三条刻意的降级日志（`[NotificationCenter] 通知服务不可用，已降级为空状态`、`[permissionStore] 加载权限失败`、`[Router] 权限加载失败，继续访问`），**没有一条 Vue 运行时告警**——未知组件、重复 key、未声明的 emit 都会在这里现形，一条都没有。
2. **降级文案就是设计要的那一份**：控制台显示「后端未返回，显示契约默认值」并按字段列出后端那句 400 文案；图谱把统计 / 社区 / 路径三块各自单独报错而不是一片黑；会话页写明「打开会话会补发一次详情请求——列表接口刻意剥掉了 `messages`」；广场是「平台统计未加载：服务端内部错误（500）」。两个新动作面的耦合文案（「但不会改动任何专家的 `current_load`」「该键无 serde 缺省，不发会被整段 JSON 拒绝」）确实在页面上读得到，L13 / L14 两条断言的靶子不是纸面上的。

**这次核验的边界要说清**：in-app 浏览器给不出可见表面对象（视口 `0×0`，`take_screenshot` 报 `NATIVE_BROWSER_VIEWPORT_UNAVAILABLE`），所以以上全是 **DOM + 计算样式级**证据，不是像素级——换行、遮挡、滚动条、SVG 在真实宽度下的排布仍然没看过，等能出图时补。

**顺带查出一个根因在外壳的对比度缺陷**（探针逐条算可见文本的 WCAG 比值，背景按祖先链合成）：仅广场一页就有 26 条低于 AA 正文的 4.5:1，分两类根因——

- `--text-muted` 只在 `src/styles/global.css:13` 定义为 `#6b7280`，三套主题（dark / cyberpunk / sky）都只覆写 `--text-secondary`，`--bg-card` 更是无人覆写。于是次级标签在 `#242838` 卡面上只有 **3.03:1**、在 `#2a2f45` 悬停面上 **2.73:1**。模块里 `var(--text-muted)` 用了 69 处 / 17 个文件（`AllianceConsoleView.vue` 一个就占 18 处），**这是令牌取值的问题，不是模块用错了令牌**。
- Element Plus 的亮色态组件：`.el-alert--error.is-light` 实测 `#f56c6c` on `#fef0f0` = **2.61:1**（一块浅粉底坐在深空卡面上），`el-tag` 同理 `#60a5fa` on `#f4f4f5` = 2.54:1。外壳 `global.css:114-149` 明明写着「Element Plus 深色主题变量」，却只覆写到基色（`--el-color-danger: #f87171` 确实生效），`light-N` 序列没跟上。而**补变量修不好**：现场把 `--el-color-danger-light-9` 改成 `#2a1b1f`，`.el-alert` 的 background 仍是 `rgb(254,240,240)` —— EP 的 `dist/index.css` 把这些亮色值烤死在规则里，只有显式覆写 `.el-alert--*.is-light`，或引入 EP 官方 `theme-chalk/dark/css-vars.css` + `html.dark`，才治得到。（此处把 `el-tag` 归到同一类是错的——tag 读的是变量，病因是外壳漏覆写 `--el-fill-color-blank`，见下一段的表。）

**已动手（2026-09-24，定位由用户裁定：外壳 `global.css` 统一修）**。四类病因全改在 `src/styles/global.css` 的令牌源，模块与存量页一行未动：

| 病因 | 改动（`global.css`） | 实测 WCAG 前 → 后 |
|---|---|---|
| EP 的 `light-N` 语义是「往白里混」，深色外壳下读它的组件带近白底 | `:157-166` 把 primary/success/warning/danger/info 的 `light-8/9` 换成同色相暗色淡染 | `el-tag` 底 `#f4f4f5` → `rgba(96,165,250,.12)`，字 `#60a5fa` 2.54 → 5.75 |
| `.el-alert--*.is-light` 在 EP dist 里烤成字面量，补变量无效 | `:184-190` 显式覆写四类底色，文字取 `--text-primary/-secondary` | error 横幅标题 2.61 → **13.39**，描述 → 6.19 |
| **`--el-fill-color-blank` 外壳从未覆写**：EP dist 烤 `#fff`，theme-chalk 有 **28 个组件样式表**读它（plain 态 tag、默认与禁用态 button、card、popper、input、select、table、pagination…）；三套主题各自覆写了 `--el-fill-color` 四连，同样漏掉它 | `:171` `--el-fill-color-blank: var(--bg-raised)` | tag 与头部按钮由 `#9aa0b4 on #fff` = 2.54 / 2.61 → 底变 `#242838` 后 5.62。**上一行那条 `light-N` 治不到它**，这才是 tag 的真病因 |
| 灰阶与语义色被直接当正文用 | `:70-71` `--text-tertiary/--text-quaternary` 由 `#6b7280`/`#4b5563` 同为 `#9098ad`，`:13` `--text-muted` 改引用 tertiary；`:133-135` EP 的 regular/secondary/placeholder 全改引灰阶令牌；`:23` `--danger` `#ef4444` → `#f87171`，并让 `--el-color-danger` 反引 `var(--danger)` | muted 3.03（卡面）/ 2.73（悬停面）→ **4.58**（最亮面上，卡面 5.07）；占位符 1.94 → 5.07；`var(--danger)` 作正文 3.89 → 5.29（全仓 31 处 `var(--danger)` 里 23 处是 `color:`，而 `theme-dark.css:27` 与 EP 块本来就各写了一份 `#f87171`，此处归一） |

四级灰阶在这条外壳上收敛为三级可用：**最亮卡面 `#2a2f45` 上 AA 要求前景亮度 ≥ .308，而 `--text-secondary #9aa0b4` 只到 .353**——余量装不下两级更暗的灰，故 tertiary/quaternary 取同值，层级差交给字号与字重。这条约束已写进 `global.css:68-71` 的注释，免得下一个人「把它改回更淡的灰」。

**同一探针下的 A/B**（现场注入 `:root` 旧值再撤，不是跨会话比较）：模块五页全部 **0 条 <4.5:1**（广场 11 → 0，那 11 条的构成为 9 条灰阶 + 2 条白底）；存量三页同向变好（首页 6→4、旧广场 3→2、联盟管理总览 16→13）。主按钮是单独一处：EP 的 `.el-button--primary` 把禁用态烤成 `light-5` 浅紫底压白字，实跑 **1.49:1**（控制台「保存改动」在网关停机时正是这个态），外壳原 hover 用 `--accent-light` 也是白字压浅靛（2.98:1）——`:389-399` 把整条链改走 `--brand-600/700` 与禁用态 `--bg-hover`，白字分别到 6.29 / 7.90，禁用态 4.58。**回归**：`npx vitest run` 452/452 绿、`npx vite build` 通过（31.40s）。并且这一类缺陷已钉进门禁——`style.test.js` 的令牌约束由 6 条增至 9 条，新增三条分别守「模块里当作正文色的每个令牌在四块实心面上都 ≥4.5:1」「灰阶五级连悬停面都不低于 AA」「EP 的 `light-8/9` 序列不得留成近白 literal，且 `--el-fill-color-blank` 必须被覆写」（第三条带 `checked > 6` 反空转自守）。**门禁反向验证过**：把 `--text-quaternary` 改回 `#4b5563` 并删掉空白底那一行，两条用如期变红（`--text-quaternary=#4b5563 压在 --bg-primary 上 2.50:1`、`expected undefined to be defined`），随后 `cmp` 确认文件按字节复原。

**两处明确没修**。其一是主题接线而非对比度：`html[data-theme="sky"]` 下同一探针报 16 条，根因是 `theme-sky.css` 只覆写 `--bg-surface/-2/--bg-raised/--bg-input`，从不覆写 `--bg-primary/--bg-secondary/--bg-card`，而外壳的 `html,body,#app` 与卡片读的恰是后者，于是「浅色主题」渲染成深底配浅色主题的文字（白底 tag 坐在 `#0f1117` 页面上）。其二是 dark / cyberpunk 各剩 3 条占位符，都是各主题文件自写的 `--text-quaternary`（`theme-dark.css:47` `#64748b` 压在 `#1e293b` 上 3.07:1）——同一病因的三份拷贝，要修得先回答「三套主题各自覆写谁」，与布局方案 Q1–Q6 是同一个问题，**未动**。存量页自烤的 hex（如 `components/PhasePipeline.vue:144` 步骤圈 `#cbd5e1` 压白字 1.48:1）同样留在原地：它们是「外壳没有可用令牌」的产物，不是原因。

**DAG 面核对这一批先推翻了我自己的两个前提**（记下来，因为两个都是"照着设计稿推代码"推出来的）：其一是以为拓扑要从 `plan/generate` 的 `depends_on` 推，其二是以为 `GET /api/alliance/tasks/:id/dag` 需要新挂载——它其实**早已挂载**（`api/alliance.api.js:128-131` 的 `getDag` → `store/alliance-console.store.js:78-82` → `views/AllianceConsoleView.vue:412-426` 的 DAG 页签）。设计稿 `DAG-VISUALIZATION-DESIGN.md` 写的是 `/api/v1/alliance/tasks/:task_id/dag` 外加一条 SSE 事件流（`.../events`，六种事件类型），前者路径不存在（真实的那行是 `alliance.rs:1820` 的 `/api/alliance/tasks/:id/dag`，网关无 `/api/v1` 前缀的联盟路由），后者从未落地（SDK 侧唯一的流是 `:1817` 的 `/logs/stream`，其 handler `task_logs_stream`（`:1720` 起）只回放 `LogEntry` 并广播日志行，没有任何节点生命周期事件）。所以这一批不是新增面，是把已挂载的面按后端源码逐条核对，结果三处不诚实：

1. **页脚拿的是另一个数组**。`AllianceConsoleView.vue:424` 原写 `{{ store.detail.dag.edges.length }} 条依赖`，而分层（原来 `:592` 的 `dagLayers`）从 `nodes[].dependencies` 算，`parents.filter(Boolean)` 会把悬空依赖静默丢掉。后端两处同源于 `alliance.rs:1560-1573`，因此 `edges.length ≡ Σ dependencies.length` 恒等——数字看着"没错"，但它统计的是含悬空项的声明量，画出来的却是剔除后的结果。改法是让分层与计数同源：算法从视图挪进 `model/dag.js` 的 `layoutDag`（按 §2 的分层规则它本就不该住在 SFC 里，视图只剩一行 `layoutDag(...)`），页脚分列 `drawn` / `dangling` / `backEdges` / `edgeDelta` 四笔账。**环在这里不抛错也不丢节点**：`visiting` 集合遇到回边即截断并计数——DAG 侧没有 Kahn 校验（拓扑检查在 `experts_orchestration.rs:443` 起的计划执行路径，不是这份出参），所以成环是可能到达界面的形状，截断就得说明是截断。
2. **`stats.skipped` 是折叠桶**：`node_stats` 返回元组第 6 格写作 `skipped + cancelled`（`alliance.rs:158`），handler `:1575` 把它取名 `other`、`:1587` 以 `"skipped"` 发出，而节点级状态区分两者（`NodeExecStatus :54-61` 六态含 `Cancelled`）。界面因此按 `nodes[].status` 分开计，`normDag` 仍按后端键名原样透出、**不**改写成 `cancelled`——这一处我差点反着做：把出参改名会让前端"看起来更准"，那是在伪造后端没有的键。
3. **节点 `progress` 是状态映射出来的 100/50/0**（`alliance.rs:1535-1539`），而同名的任务级 `progress()` 是 `(completed + running * 0.5) / total`（`:161-167`）——一个字段名两种口径，前者从未测量过任何东西。当前 DAG 页签不渲染节点进度，所以这一条不改界面，只钉进 §5 表：**将来要显示必须先标注"由状态推导"**。同一类还有一处：`estimated_remaining_minutes = (total - completed) * 3`（`:1702`），界面上没有它，设计稿里那个「预计剩余 2.5s」因此在 Rust 侧无对应实现。

门禁：`contract.test.js` 的跨语言块 94→99（新增五例：六个变体 ↔ `node_status_str` 出口 ↔ 中文标签 ↔ DAG 节点样式四处对齐、`node_stats` 折叠桶解析、DAG 三处 `json!` 键集与 `normNode`/`normDag` 读取集合对齐、视图对 `dag.stats` 的每次取数必须落在产出键集内、页脚同源且算法不在 SFC 里）、新增 `model/dag.test.js` 11 例（链式递进 / 取最深父依赖 / 纯函数确定性 / 悬空记账 / 环不抛错且不丢节点 / 自依赖 / `edgeDelta` 漂移 / 跳过与取消分开 / 直接吃 `normDag` 产物 / 缺状态落 `pending` / 畸形入参）。模块 449→**465 例（16 文件）**、`src/modules` **471**、全量 532→**548 例 / 24 文件**全绿。这五例都是**结构式**判据（先从 Rust 源码解析出集合，再要求界面覆盖它或落在其中），承本节 D10 那条教训：不写"禁止读某个字段名"式的否定点名。真数据边界：悬空依赖与环在真实任务里到底出不出现，:3080 停机期间无从验证——这一批的证据是源码级 + 夹具级，实跑要等网关恢复后按本节那两份「真数据核对清单」的口径补。

## 9. 关联文档

- `docs/architecture/frontend/FRONTEND-LAYOUT-REFACTOR-PLAN-v1.0.md` — 外壳分层与布局重构方案（本模块遵守其路由 meta 契约）
- `docs/expert-alliance/CURRENT-ARCHITECTURE.md` — 联盟后端实现态
- `docs/expert-alliance/DAG-VISUALIZATION-DESIGN.md` — DAG 可视化设计稿（🟡参考）。**其上半部分的 API 路径、响应形状与 SSE 设想与代码不符**，落地事实以该文档文末「设计与实现对账」与本文 §5 为准
- `docs/expert-alliance/DAG-VISUALIZATION-DESIGN.md` — 🟡 DAG 可视化设计稿（尚未落地，控制台 DAG 页为其第一块消费方）
- `docs/API-REGISTRY.md` — 端点权威登记

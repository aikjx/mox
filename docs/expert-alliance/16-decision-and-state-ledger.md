# 16 专家联盟决策与状态总账

> 总账定位：本文件是专家联盟全主线的「决策与状态总账」——把三方核验（BVR/FVR/DVR）、两份修复报告（BFR/FFR）、归一化架构 08 §九、创新规划 12、索引 INDEX 中的所有缺口 / 设计取舍 / 规划项逐条登记成可回溯的账。
> 记账口径：每条必带证据（代码 `文件:行号` 或文档章节或测试）；无法核实标「待核」；纯文档产出，不改任何代码。
> 现状争议最终裁决：以 `CURRENT-ARCHITECTURE.md` V1.1（2026-09-24）为准（见 INDEX §三）。
> 记账日期：2026-09-30。

> 证据简写约定（全文通用）：
> - **BVR** = `platform/domains/alliance/_verification/backend-verification-report.md`（2026-09-27）
> - **FVR** = `frontend-ui/src/modules/expert-alliance/_verification/frontend-verification-report.md`（2026-09-27）
> - **DVR** = `docs/expert-alliance/_verification/docs-verification-report.md`（2026-09-27，§9 为 2026-09-29 第三轮补记）
> - **BFR** = `platform/domains/alliance/_verification/backend-fix-report.md`（2026-09-27，含 2026-09-28/29 追加节）
> - **FFR** = `frontend-ui/src/modules/expert-alliance/_verification/frontend-fix-report.md`（2026-09-27，含 2026-09-29 追加节）
> - **08** = `docs/expert-alliance/08-normalized-architecture.md`；**12** = `docs/expert-alliance/12-innovation-roadmap.md`；**INDEX** = `docs/expert-alliance/INDEX.md`

---

## 一、总账总表（核心交付）

> 状态取值：`已闭环` / `设计取舍` / `留待未来` / `规划中(Pn)` / `待核`。
> 闭环轮次：R1=2026-09-27 首轮修复；R2=2026-09-27 第二轮中低严重度收尾；R3=2026-09-28/29 第三轮。

### 1.1 后端缺口 N1–N11（源自 BVR §三，08 §九登记）

| 编号 | 缺口/决策项 | 类型 | 状态 | 证据 | 闭环轮次/日期 |
|---|---|---|---|---|---|
| N1 | 下游三 svc（scheduler:3100/executor:3200/registry:3400）无鉴权中间件 | 缺口 | 已闭环（代码）；生产需配 `MOX_INTERNAL_TOKEN` 才真正生效 | BVR §三 N1（routes.rs 仅 tracing+crypto）；BFR §一 新增 `internal_auth_layer`（scheduler/routes.rs:62-94 等三处）；BFR §六 网关出站注入 Bearer（registry_client.rs:22-41、alliance_remote.rs:116-130）；测试 cargo test 85→79 通过 | R1 2026-09-27（+R3 双值见 P0-C） |
| N2 | 审计事件 Actor 硬编码 `AuditActor::system()`，无真实用户 | 缺口 | 已闭环 | BVR §三 N2（experts_common.rs:593）；BFR §二 新增 `OptionalAuthUser` 提取器（experts_common.rs:587-626），11 个写 handler 传真实 actor，未认证降级 system；cargo test 85 通过 | R1 2026-09-27 |
| N3 | SQLite 无 schema 版本表 / 无增量迁移 | 缺口（=12-D1/P0-A） | 已闭环 | BVR §三 N3（grep `user_version` 零命中）；BFR §三 三个存储点接入 `PRAGMA user_version=1`（experts_db.rs、scheduler-core/storage.rs、registry-svc/storage.rs）；cargo test 85 通过 | R1 2026-09-27 |
| N4 | 图谱 graph_nodes/edges 无节点级 CRUD，只能全量 rebuild | 缺口（=12-D7/10-#11） | 已闭环（2026-09-30，增量 CRUD） | BVR §三 N4（experts_graph.rs 仅只读+rebuild）；BFR §N4：新增 6 写端点 POST/PUT/DELETE `/api/expert-graph/nodes[/:id]` 与 `/edges[/:seq]`，RBAC `RbacAction::MutateGraph`（code `graph.mutate`）强制 super_admin/tenant_admin，experts_db.rs 新增 upsert_node/cascade_delete_node/upsert_edge/replace_edges/set_meta 增量落库（不动全量 save_graph_conn），内存态 version+=1；前端补 6 个 api/store 薄方法（不接 UI，U1 留待）；cargo test alliance 86 通过（73 基线+13 新增） | R4 2026-09-30 |
| N5 | registry-svc 主动健康探测默认关闭 | 缺口→设计取舍 | 设计取舍（生产设 `MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=true` 即开） | BVR §三 N5（app_state.rs:56 `health_probe_enabled:false`）；BFR §八 不改默认值、文档标注；08 §十一 排障表 | 设计取舍，2026-09-27 登记 |
| N6 | registry_client.rs:26 生产路径 `.expect()` 恐慌点 | 缺口 | 已闭环 | BVR §三 N6；BFR §七 改为 `.build().unwrap_or_else(降级 Client::new())`（registry_client.rs:37-41）；cargo test alliance 64 通过 | R2 2026-09-27 |
| N7 | scheduler/executor `/metrics` 返回 JSON 快照，非 Prometheus 文本 | 缺口（=12-D6） | 已闭环（2026-09-30） | BVR §三 N7（scheduler/routes.rs:112 原 `Json(...)`）；本轮三 svc `/metrics` 按 Accept 头协商：scheduler routes.rs:168/184/201、executor routes.rs:176/197、registry routes.rs:37/83-99 新增端点，Prometheus 文本 `mox_alliance_<svc>_*`；三 svc 测试 66 通过，既有无 Accept→JSON 集成测试仍绿 | R4 2026-09-30 |
| N8 | DAG 执行器并行度无信号量上限 | 缺口（=12-D5/10-#15） | 已闭环（T1a 信号量，2026-09-30；重放随 A3） | BVR §三 N8（原 grep `max_parallel/Semaphore` 零命中）；本轮 `executor-core/src/dag_engine.rs`：ENV_DAG_MAX_PARALLEL:43、node_semaphore:105、构造:152-158、acquire_owned:587；接线既有死字段 `ExecutorConfig.max_concurrent_nodes`（默认 50），env 优先、0/负=无界逃生门；executor-core lib 41 + e2e 5 + bench 1 全绿，gateway alliance 73 基线不变；两级配额（per-expert/provider）留待 P1.5、前端滑块留待 | R4 2026-09-30 |
| N9 | 调度器动词面数文档口径错误（实测 8，文档写 9） | 缺口（文档） | 已闭环 | BVR 摘要#8 + §三 N9（routes.rs:23-32 实计 8）；BFR §七 CURRENT-ARCHITECTURE.md:248「9→8」 | R2 2026-09-27 |
| N10 | 网关侧与 scheduler 侧两套熔断器状态不共享 | 缺口→设计取舍 | 设计取舍（进程边界隔离，文档说明） | BVR §三 N10（llm_router.rs:477 vs dispatcher.rs:494 独立内存 map）；BFR §八 设计取舍；08 §九 低严重度 | 设计取舍，2026-09-27 登记 |
| N11 | 会话跨进程恢复：单进程内可恢复，多副本不共享 | 缺口→设计取舍 | 设计取舍（已知架构边界；A2 memory 独立时解） | BVR §三 N11（sessions 落网关本地 SQLite，网关未多活）；BFR §八 留待未来；12 §4.2 A2（网关无状态化解决 sticky） | 设计取舍，规划中 P2 |
| N12 | 后端联盟管理写面不做角色强制（14 号 G-2） | 缺口（=14 §六 G-2） | 已闭环 | BFR §R1（新增 `alliance/experts_rbac.rs`：`ADMIN_ROLES`/`RbacAction`/`enforce_admin_or_respond`/`RbacDenied`；7 个管理写面 handler 入口强制 super_admin/tenant_admin，未认证 401 / 非管理 403 + `rbac.denied` 审计）；cargo test alliance 73 通过（基线 64 + 新增 9 RBAC 用例） | R4 2026-09-30 |

### 1.2 前端缺口 G1–G10（源自 FVR §五，08 §九登记）

> 编号口径备注：FVR 的 G1–G10 为原始缺口编号；08 §九 与 BFR §八 在转录时发生一次重排——把「虚拟滚动」记作 G8、把「硬编码中文」记作 G10，并略去 FVR 的 G7/G10 原义。本账以 **FVR 原始编号**为准，右列注明 08/BFR 的异号。

| 编号 | 缺口/决策项 | 类型 | 状态 | 证据 | 闭环轮次/日期 |
|---|---|---|---|---|---|
| G1 | legacy 工作台仍调模块契约明令禁止的编排器端点（`/ai/engine/alliance/full`、`/ai/engine/alliance/capabilities`） | 缺口 | 已闭环 | FVR §0/§3.1/§5 G1（endpoints.js:135-136 FORBIDDEN）；FFR 任务1（useAlliance.js 改走 `allianceApi.collaborate`、ExpertWorkspaceView 改读 `/api/experts/capabilities`）；防复活 `contract/forbidden-revival.test.js`（9 例）；vitest 646 通过 | R1 2026-09-27（进场前一轮已收口，本轮复核） |
| G2 | legacy 埋假端点 `POST /alliance/tasks/:id/qa`（后端 registry 无此路径） | 缺口 | 已闭环 | FVR §3.1（alliance.api.js:315 askAllianceTaskQa）；FFR 任务2 删定义 + 唯一活调用点 AllianceTaskView.vue:357 改走 `api.aiChat`，另清 3 个零调用方桩；vitest 646 通过 | R1 2026-09-27 |
| G3 | 破坏性/管理面路由只校验登录、不校验角色 | 缺口（=12-D2/10-#19+#38） | 已闭环 | FVR §5 G3（index.js:82-146 仅 requiresAuth，v-permission 零命中）；FFR 任务3 给 console/graph/orchestration/experts 4 路由补 `requiresRole:['super_admin','tenant_admin']`，collab/sessions 保持登录即可；未引入不存在的 `operator` 角色码 | R1 2026-09-27 |
| G4 | 两套并行 API 客户端 + 两套归一化，同一后端两种口径 | 缺口 | 后端侧**已归一化**；前端侧 legacy 调用面已收敛（留 9 处合法例外），完整删 legacy 桶为 P1 | BFR §九（gateway/src/alliance_remote.rs 仅 2 行 re-export SDK；RegistryClient→3400 与 RemoteAllianceClient→3100/3200 职责不同）；FFR §2026-09-29（30+ 调用点 adapter 收敛到 allianceApi，grep 零残留）；FVR §3.2 11 条 UNMOUNTED 孤儿有意拒绝 | 后端 R3 2026-09-28 归一化；前端 R3 2026-09-29 收敛 |
| G5 | SSE 仅控制台真接，外部任务视图靠 4s 轮询 | 缺口（=12-D9） | 留待未来 P2 | FVR §5 G5（AllianceConsoleView.vue:685-707 useSSE vs AllianceTaskView 手写 reader）；BFR §八 P2 | 未修，规划中 P2 |
| G6 | console/orch 两个 store 无独立测试 | 缺口（=12-D9） | 留待未来 P2 | FVR §5 G6（alliance-console.store.js 356 行 / alliance-orch.store.js 219 行无 .test.js）；BFR §八 P2 | 未修，规划中 P2 |
| G7 | 大列表无虚拟滚动（GraphCanvas 手写 SVG 全量渲染） | 缺口（08/BFR 误标 G8） | 留待未来 P2 | FVR §5 G7（全仓 virtual/VirtualScroll 零命中；pageSize=24 已缓解）；BFR §八「G8 虚拟滚动 P2」即本项 | 未修，规划中 P2 |
| G8 | 国际化：文案全部硬编码中文（08/BFR 误标 G10） | 缺口→设计取舍 | 设计取舍（内网政务定位；多语言需求出现时整体 i18n） | FVR §5 G8（$t/vue-i18n 零命中）；BFR §八「G10 文案硬编码中文」设计取舍 | 设计取舍，2026-09-27 登记 |
| G9 | 图谱可视化三套技术栈并存（手写 SVG / 手搓力导向 / echarts force） | 缺口（=12-D10/10-#40） | 留待未来 P2（U1 画布前置：先归一栈） | FVR §5 G9（GraphCanvas.vue:3 手写确定性 SVG vs ExpertCenterView.vue:552 力导向 vs ExpertEnterprisePanel.vue:972 echarts）；12 §3.1/§3.3 U1/U3 依赖 G9 先收口 | 未修，规划中 P2 |
| G10 | 加载态良好，DAG 长任务有骨架（非缺口） | 核验通过项 | 已核实通过（不构成缺口） | FVR §5 G10（el-skeleton/v-loading 证据：AllianceConsoleView.vue:341 等） | 核验通过 2026-09-27 |

### 1.3 D 系列差距（源自 12 §1.1，由 10 评审 P0/差距归纳）

| 编号 | 差距项 | 对应缺口 | 类型 | 状态 | 证据 | 闭环轮次/日期 |
|---|---|---|---|---|---|---|
| D1 | SQLite 无 schema 版本迁移 | N3 | 前置底线 P0 | 已闭环（=P0-A） | 12 §1.1；BFR §三 | R1 2026-09-27 |
| D2 | 审计 Actor 硬编码 system + 前端控制台无角色按钮级权限 | N2+G3 | 前置底线 P0 | 已闭环（=P0-B） | 12 §1.1；BFR §二；FFR 任务3 | R1 2026-09-27 |
| D3 | WS 文档幻觉未修订（代码无 WS，06 文档宣称有） | #25 | 前置底线 P0 | 已闭环（=P0-D，纯文档） | 12 §1.1；DVR §9（01/02/03/06 共 5 处补记+4 处横幅，原文不改） | R3 2026-09-29 |
| D4 | 进程内三项（favorites/plans/orchestration_history）重启即失 | 已知缺口 | P1 落盘 | **已闭环（2026-10-02，schema v3 三表落盘 + 启动读回，两租户隔离 E2E 已证）** | experts_db.rs 新增 collaboration_plans/orchestration_history/favorites（tenant_id 复合主键）；写后 upsert/insert，new() 启动 load_all_*；证据 _verification/d4-e2e-evidence.txt；tests/d4_crash_recovery.rs；lib 180 / 集成全绿 | R? 2026-10-02 |
| D5 | DAG 并行无护栏（信号量/重放） | N8 | P1 差距 | 信号量已闭环（T1a 2026-09-30）；重放随 A3 规划中 P1 | 12 §1.1；12 §2.1 T1；本账 §一.1 N8 | 信号量 R4 2026-09-30 |
| D6 | 指标三进程格式不统一 | N7 | P1 差距 | 已闭环（2026-09-30，Accept 协商 Prometheus 文本） | 12 §1.1；本账 §一.1 N7 | R4 2026-09-30 |
| D7 | 图谱节点级无 CRUD，只能全量 rebuild | N4/#11 | 🔴 硬缺口 | 已闭环（2026-09-30，增量 CRUD；图RAG T2 的前置已就绪） | 12 §1.1；BVR §三 N4；本账 §一.1 N4、BFR §N4 | R4 2026-09-30 |
| D8 | registry 主动探活默认关闭 | N5 | P1 差距 | **已闭环（2026-10-02，默认改开；env `MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=0/false/no/off` 可关回退被动租约）** | app_state.rs:61 `health_probe_enabled:false→true`；health_probe.rs 注释+断言同步；registry-svc 28 全绿（probe_default_config_enables_task_d8） | R? 2026-10-02 |
| D9 | SSE 未统一 + store 无测试 | G5+G6 | P1 差距 | 留待未来 P2 | 12 §1.1；FVR §5 G5/G6 | 规划中 P2 |
| D10 | 图谱可视化三套栈未归一 | G9 | P2 | 规划中 P2（=U1/U3 前置） | 12 §1.1；FVR §5 G9 | 规划中 P2 |

### 1.4 P0 前置底线四项（12 §1.3 三行 P0 拆为 A/B/C/D）

| 编号 | P0 项 | 类型 | 状态 | 证据 | 闭环轮次/日期 |
|---|---|---|---|---|---|
| P0-A | SQLite schema 版本迁移（N3/D1） | 前置底线 | 已闭环 | BFR §三；cargo test 85 通过 | R1 2026-09-27（前轮） |
| P0-B | 审计真实用户身份（N2）+ 前端控制台角色权限（G3）（D2） | 前置底线 | 已闭环 | BFR §二；FFR 任务3；cargo test 85 / vitest 646 | R1 2026-09-27（前轮） |
| P0-C | 下游 svc 内部令牌双值滚动（`MOX_INTERNAL_TOKEN_ALT`，零停机换令牌） | 前置底线 | 已闭环 | BFR §P0-C（三 svc routes.rs 同构加 ALT 校验：scheduler:80-104/executor:137-161/registry:89-113）；cargo test 三 svc 64 + gateway 64 通过；网关出站仍只读主值 | R3 2026-09-29（本轮） |
| P0-D | WS 文档幻影修订（01/02/03/06 补记，代码侧无需实现） | 前置底线 | 已闭环 | DVR §9（`p0d_patch_docs.py` 幂等插入 5 处补记+4 处横幅，原文一字未改） | R3 2026-09-29（本轮） |

### 1.5 规划项 T1–T5 / U1–U4 / A1–A4 / M1–M4（源自 12 §1.3，均未动工）

| 编号 | 创新项 | 板块 | 优先级 | 状态 | 证据（依据） |
|---|---|---|---|---|---|
| T1 | DAG 并行度信号量与重试/重放 | 新技术 | P1 | 信号量（T1a）已闭环 2026-09-30；重放随 A3 规划中 P1 | 12 §2.1；依据 10-D5/N8/#15、11-弱⑦；本账 §一.1 N8 |
| T2 | 图 RAG（知识图谱增强检索） | 新技术 | P1 | **已闭环（部分）**：2026-10-01 图谱纯检索落地（内存态加权多跳扩展，`POST /api/expert-graph/rag/expand`，权重乘积聚合，向量融合待 #27）；实现选择由规划期「SQLite 递归 CTE」改为内存态（state 无连接句柄，语义等价，A4 图库迁入时换查询实现） | 12 §2.2；依据 11-强①、10-#10/#11/N4/D7；本轮见 backend-fix-report.md「T2 图 RAG（2026-10-01）」 |
| T3 | MCP 协议接入（Server 先行、Client 跟进；由 P3 提前） | 新技术 | P1 | **部分闭环**（2026-10-01 MCP Server 已落地：stdio 自实现 + 3 真实工具；Client/凭证托管留待） | 12 §2.3；依据 11-弱⑧；依赖 P0-N1/N2 |
| T4 | 事件驱动架构（任务状态事件流，公共骨干） | 新技术 | P1 | 🟡 **部分闭环（进程内总线，2026-10-02）**：`tokio::sync::broadcast` 进程内事件总线 + 事件模型（PlanCreated/PlanStatusChanged/ExpertRegistered/ExpertDisabled，带 tenant），真实 handler emit、消费者落 `alliance_event_log`（schema v4），E2E 真实验证通过（t4-e2e-evidence.txt）。**未做（后续）**：SSE 升级「日志帧+事件帧」双通道、外部系统 webhook/SSE 订阅端点、跨进程/多副本广播。**勘误**：任务书曾把「事件驱动」误挂 M1；实际 M1=私有化交付产品化（见下行），事件驱动即本行 T4 | 12 §2.4；依据 10-#24/D9、11-弱②；backend-fix-report T4 节 |
| T5 | 流式编排（边执行边交付） | 新技术 | P2 | 规划中 P2（依赖 T4） | 12 §2.5；依据 10-#6/#9 |
| U1 | 画布式可视化编排（拖拽 DAG） | 新UI | P1 | **已闭环（MVP：能力图谱画布编辑，2026-10-01）**——在既有手写 SVG `GraphCanvas.vue` 上增强（不引 VueFlow/LogicFlow，守 G9 三栈归一、不做第四套渲染栈）：节点拖拽（视觉坐标只活前端 dragPositions，不入库）、Inspector 节点改/删（v-role-any 管理写面）、新增节点、点两节点连线（选 edge_type）、选中节点「展开邻域」走 T2 `expandGraphNeighborhood` 幂等并入。**未做（诚实标注）**：DAG 编排导出/预演、实时多人回显、虚拟滚动（G7 仍 P2）、拖拽坐标持久化。G9 三套栈归一与 U3 力导向/分层仍留待 | 12 §3.1；FFR「U1 画布 MVP（2026-10-01）」；本账 §五 2026-10-01 行；vitest 1044 全绿 |
| U2 | 专家画像与匹配透明化（逐维打分可视化） | 新UI | P1 | **已闭环（2026-10-01）**。代码证据：scheduler-proto/matcher.rs `MatchedExpert.weights`；scheduler-core/modular_matcher.rs:286 带出实际权重；api/dto.rs `ExpertScoreView{domain,capability,health,priority,performance 各 {value,weight},total}`；scheduler-svc/routes.rs `/experts/search` 透出；http-sdk alliance.rs（本地降级）+ alliance_remote.rs（远程优先）两路径均透传。前端 `MatchExplainPanel.vue` 逐维条形图+权重标注+总分演算。**权重表（主路径默认）**：domain 0.35 / capability 0.30 / priority(=rating 权重) 0.20 / performance 0.10 / **health 0.05**；健康分 is_healthy?1.0:0.2，非硬过滤。**口径纠错**：旧账「健康度 0.15」系 bio/备用 matcher 权重误植，主路径实为 0.05。测试：scheduler-core 115 + scheduler-svc/http-sdk 38 全绿。 | 12 §3.2；15 U2；依据 10-#12、11-弱② |
| U3 | 图谱可视化增强（力导向/分层/虚拟渲染） | 新UI | P2 | 规划中 P2（在 U1 选栈后） | 12 §3.3；依据 10-#40/G7/G8/G9/D10 |
| U4 | 实时协作（多人围观/评论标注） | 新UI | P2 | 规划中 P2（依赖 A1/T4） | 12 §3.4；依据 11-Coze 借鉴 |
| A1 | 多租户隔离（数据/配额/密钥 + SSO） | 新架构 | P1 | 🟢 **阶段二续（2026-10-02）**：阶段一数据/内存态按租户隔离、TenantId 提取器、审计带 tenant、两租户 E2E 已证（a1-e2e-evidence.txt）。**配额子项已闭环**：单租户专家数上限 `MOX_ALLIANCE_QUOTA_EXPERTS_PER_TENANT`（默认 1000，env 覆盖），`create_expert` 超限真实 **409** + 结构化 `quota/used`，按租户独立计数，低配额 E2E 已证（a1-quota-e2e-evidence.txt）。**SSO 子项=方案稿/待真实 IdP**：OAuth2/OIDC 授权码交换已真实实现（reqwest 直连 token_endpoint），但本机无真实 IdP 凭据/无本地 Keycloak，端到端真实验证缺 IdP，不做假对接；SAML/CAS/LDAP 仍 501。**密钥托管 / 会话-任务-执行器分区 / 租户内 RBAC 细化 / 租户级配额配置表 = 阶段三（方案稿）**。依赖 P0-N2/G3/N1 均已闭环 | 12 §4.1；依据 11-弱③；backend-fix-report A1 阶段二节 |
| A2 | 模块化微服务深化（fusion / memory 独立，网关无状态化） | 新架构 | P2 | 🟡 **部分闭环（阶段一：冷数据外移为 SQLite 唯一真相，2026-10-02）**：D4 已落盘的三项冷数据（collaboration_plans / orchestration_history / favorites）读路径由「内存为主」改为**按租户实时查 SQLite**（写穿 + busy 重试），两个活实例共享同一文件时 A 写穿、B 不重启即读到，跨实例一致 + 租户隔离 E2E 已证（a2-e2e-evidence.txt）。**registry/graph 高频态外移、执行器 task 状态、分布式通知/失效广播、多副本写冲突策略、memory/fusion 独立 = 阶段二（方案稿）** | 12 §4.2；依据 08 §十 P2、10-#9/#26/N11；backend-fix-report A2 节 |
| A3 | 事件溯源（任务/审计可重放，审计送 SIEM） | 新架构 | P2 | 规划中 P2（依赖 T4 事件模型） | 12 §4.3；依据 10-#19/#28/#29/D4 |
| A4 | 图数据库引入（关系层从 SQLite 升级） | 新架构 | P2 | 规划中 P2（关键约束：须可气隙/嵌入式，守住国密气隙卖点） | 12 §4.4；依据 10-#11/N4/D7 |
| M1 | 私有化交付产品化（信创/气隙离线包 + 一键自检） | 新模式 | P1 | 规划中 P1（投入产出比最高） | 12 §5.1；依据 11-强②③、10-#21、09 生产模板 |
| M2 | 生态开放（MCP Server / 插件 SDK / 开放 API webhook） | 新模式 | P2 | 规划中 P2（T3 落地后顺势） | 12 §5.2；依据 11-弱④⑧ |
| M3 | 专家市场（先做组织内模板库，不做公开 C 端） | 新模式 | P2 | 规划中 P2（依赖 M2/A1） | 12 §5.3；依据 11-弱④ |
| M4 | SaaS 订阅 | 新模式 | P2/P3 | 规划中 P2/P3（远期；政务气隙客户永远走 M1） | 12 §5.4；依据历史 39 号（已退役快照） |

---

## 二、设计取舍汇总（不是缺陷、是选择）

> 每条给「取舍内容 + 理由 + 何时升级 / 文档出处」。

1. **进程内三项（favorites / plans / orchestration_history）重启即失** ✅ 已闭环（2026-10-02，D4）
   - 历史现状：三者曾为网关进程内 `HashSet/HashMap/Vec`，无表、无持久化。证据：BVR 摘要#13/#14/#15。
   - **已落地**：schema 升 v3（PRAGMA user_version 2→3），新增 `collaboration_plans` / `orchestration_history` / `favorites` 三表（均带 `tenant_id` 复合主键，与 A1 多租户行级隔离一致）；写后立即 upsert/insert（best-effort，与既有 sessions/graph 同约定），`ExpertsSharedState::new()` 启动 `load_all_*` 读回。favorites 内存态随之按租户分区（`HashMap<tenant, HashSet<expert_id>>`）。
   - 真实验证：tests/d4_crash_recovery.rs 走「写入→模拟崩溃重启→读回逐字段一致」闭环，并证 tenant-a 看不到 tenant-b 的 plan/收藏；证据 _verification/d4-e2e-evidence.txt。单租户(default)行为零回归。
   - 遗留（诚实标注）：sessions 此前仅「单进程内可恢复」（N11，多副本不共享）；三项落盘同样是网关本地 SQLite，多副本/A2 memory 独立前不跨进程共享。远期随 A3 事件溯源化为不可变事件流。
   - **更新（A2 阶段一，2026-10-02）**：三项冷数据「多副本不跨进程共享」遗留**已解**——读路径改为按租户实时查 SQLite（唯一真相），两活实例共享同一文件时 A 写穿、B 不重启即一致（a2-e2e-evidence.txt）。sessions（N11）与 registry/graph 高频态仍留阶段二。

2. **专家健康字段是登记值，非探活结果** ✅ 探活默认开已闭环（2026-10-02，D8；登记值语义不变）
   - 现状：`availability.status` 从注册请求 body 写入（online/busy/offline/away）。证据：BVR 摘要#27（experts_registry.rs:127-128）。
   - **已落地（D8）**：registry `Config::default().health_probe_enabled` 由 `false` 改 **`true`**（app_state.rs:61）——默认即启动后台主动探测，提前标记「仍心跳但业务端点不可用」的实例为 Unhealthy 并在恢复时自动回册。env 覆盖保留：`MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=0/false/no/off` 可关闭，回退「仅被动心跳租约」。
   - 语义与前端约束：前端仍不得把 `online` 写成「健康检测通过」，`online` 仅作可用性枚举标签；真正健康判断走 scheduler matcher 健康度加权（非硬过滤）。探活端点公开白名单（/health、/metrics、/leadership、/api/registry/health）不鉴权，本次改默认值不触及路由。

3. **两套熔断器状态不共享（网关 dispatcher vs 下游 scheduler）**
   - 现状：scheduler-core/llm_router.rs:477 是真实熔断（threshold=5/60s）；网关 dispatcher.rs:494/794/828 是另一独立内存 map。证据：BVR §三 N10。
   - 理由：二者分处不同进程边界（网关本地预览态 vs 远程 scheduler 态），各自熔断是合理的进程内隔离，状态互通会引入分布式一致性成本。
   - 何时升级：文档说明边界即可（08 §九 已登记）；无跨进程熔断需求前不动。

4. **中文硬编码文案（暂不国际化）**
   - 现状：全仓 `$t`/vue-i18n 零命中。证据：FVR §5 G8。
   - 理由：当前定位内网政务系统，用户群体固定中文，与现状匹配。
   - 何时升级：出现多语言需求时整体引入 i18n（BFR §八 设计取舍）。

5. **G4 两套 Rust 客户端：已归一化，不硬做收敛**
   - 事实：网关 `gateway/src/alliance_remote.rs` 仅 2 行 `pub use mox_alliance_http_sdk::alliance_remote::*;`（re-export SDK），RemoteAllianceClient 真源在 `mox-alliance-http-sdk/src/alliance_remote.rs`（1282 行，单一实现）。证据：BFR §九。
   - 另一客户端 RegistryClient（registry_client.rs，~110 行）调 registry-svc:3400（注册中心：health/register/heartbeat/aggregated-heartbeat），与 RemoteAllianceClient 调 scheduler:3100+executor:3200（任务编排）**职责完全不同**，不是重复。
   - 结论：后端 G4 **已归一化、无需收敛**；唯一小重复（各自在 builder 注入 `MOX_INTERNAL_TOKEN` ~8 行）因指向不同 base_url 抽公共函数收益极小。前端「模块 allianceApi vs legacy @/api」是另一问题，已在 FFR §2026-09-29 把 30+ 调用点收敛到模块契约。

6. **39 号文档角色继承链非现行（历史快照定位）**
   - 事实：`docs/enterprise/39-*.md` 引用已删除的 `platform/backend-node/`、`mox-expert` crate 与已迁移网关端口 :8080。证据：DVR §7；INDEX §四。
   - 定位：带「历史快照说明（2026-09-13）」的 Node 时代诊断，与现行 Rust 实现是**被取代关系**，禁止并列引用。现行架构见 CURRENT-ARCHITECTURE V1.1。

7. **JWT vs 内部令牌的鉴权边界（内网设计取舍）**
   - 事实：前端→网关 :3080 走 JWT（gateway auth_middleware，auth.rs:143）；网关→下游三 svc 走内部共享令牌 `MOX_INTERNAL_TOKEN`（BFR §一）。
   - 理由：gateway 的 JWT auth_middleware 依赖其内部 `AuthConfig`/`token_blacklist`，不在公共 crate，下游直接复用会产生循环依赖；内部令牌是最小侵入方案。未配置令牌时放行（向后兼容开发默认），`MOX_DEV_MODE=1` 强制跳过。
   - 边界：下游 svc 不直接对前端暴露，生产靠网络隔离 + 令牌双窗口滚动（P0-C）防绕过。

8. **engine_status 恒 "running" / circuit_breakers 初始空**
   - 事实：experts_dispatcher.rs:536 硬编码 `"engine_status":"running"`（BVR 摘要#29）；circuit_breakers 初始为空 Vec，失败经 ensure_failure_map 写入后才填充（BVR 摘要#30，⚠️ 部分一致——非「恒 []」）。
   - 理由：本地预览/模拟执行语义，运行时徽标已如实标注「本地预览（模拟执行）…不作为真实执行指标」（FVR §4.1，AllianceConsoleView.vue:613-633）。

9. **不引入 `operator` 角色；按钮级已改走 v-role-any（2026-09-30 演进）**
   - 事实：工程 `ROLE_TEMPLATES` 真实角色码仅 `super_admin/tenant_admin/dept_manager/normal_user/readonly_auditor`，无 `operator`。证据：FFR 任务3。
   - 理由（不引入 operator）：按工程既有 ADMIN_GUARD 口径统一用 `['super_admin','tenant_admin']`，避免向后端编造授权模型。
   - **演进（G-1 闭环）**：早前「破坏性按钮所在路由已整体限管理员，按钮级指令在路由 meta 层收口即可」的判断，
     于 2026-09-30 傍晚补到元素级——7 个管理写面按钮挂 `v-role-any="['super_admin','tenant_admin']"`
     （与路由 requiresRole、后端 experts_rbac.rs ADMIN_ROLES 三端同源）。**注意：用的是角色码 v-role-any，
     不是 v-permission 权限码**（联盟无权限码登记）。详见 FFR「2026-09-30 按钮级权限接入」节、14 号 §六 G-1。

10. **11 条孤儿端点有意不挂（UNMOUNTED_ROUTES rejected）**
    - 事实：`alliance.tasks.status_poll`（十四键二手/虚构）、`alliance.stats`（八键硬编码 0）、`experts.orch.plugins`（硬编码 6 条假数据）、enterprise/consult、dispatcher/consult 等 11 条后端存在、模块故意不挂，已在 `contract/endpoints.js:147-174` 附后端行号证据。证据：FVR §3.2。
    - 理由：是有据的拒绝，不是遗漏；防止把 stub/重复端点请回前端。

11. **图谱手写 SVG，刻意不用 force-directed 弹簧布局**
    - 事实：模块 GraphCanvas.vue 用 `<svg>` + `model/layout.js` 确定性布局（layout.js:2 自述「刻意不用 force-directed」）。证据：FVR §5 G9。
    - 理由：可快照测试，工程上是正向取舍；外部 legacy 的手搓力导向/echarts 才是待归一的第二、三套（G9）。

12. **gRPC :50051 联盟未使用（措辞待统一，非缺陷）**
    - 事实：CURRENT §2.2 写「gRPC :50051 在 framework 层保留但专家联盟未使用」，PORT-REGISTRY §3 写「专家联盟内部 gRPC」。证据：DVR §2。
    - 状态：⚠️ 措辞差异待统一（本账标「待核/待统一」），代码事实以「联盟未使用 gRPC、实时性仅 HTTP+SSE」为准。

13. **N7 子代理误在错误工程落盘 → 已完整回退并在正确工程重做（账实同步教训）**
    - 事实：N7 指标 Prometheus 文本化子代理曾误在 `C:\Users\mo\xuanji`（另一工程）落地改动，发现后已完整回退（grep 0 残留），并在正确工程 `D:\a10\aikjx\gitcode\infotopograph` 重做。
    - 教训：开工前先 `Test-Path` 目标工程关键文件确认所在工程，避免跨工程污染。

---

## 三、冲突裁决链确认（最终权威源表）

> 一个事实一个权威源（INDEX §三）。每条注明生效时点与例外。

| 争议点 | 唯一裁决源 | 何时生效 / 例外 |
|---|---|---|
| 现状长什么样（进程、16 crates、9924 行、7 模式、存储、枚举） | `CURRENT-ARCHITECTURE.md` **V1.1**（2026-09-24，🟢权威） | 自 2026-09-24 起为最终裁决；01–07 均 V1.0 目标态，文首已补「被取代」横幅（DVR §9），不得直接当现状 |
| 端口号（生产 3080/3100/3200/3400；旁挂 3300/3210） | `docs/api/PORT-REGISTRY.md` **V1.2**（2026-09-14） | 自 2026-09-14 起唯一端口权威；例外：CURRENT §2.1 进程表曾漏登 3400（DVR §2），以 PORT-REGISTRY 3400 在册为准；gRPC 50051 措辞差异见 §二.12 |
| 部署 / 运行开关（MOX_* 取值） | `09-deployment-templates.md`（语义再落到代码 `文件:行号`） | 09 核对 52 个 MOX_* 读取点最全；07-deployment 漏列 env、08 仅 12 变量，均以 09 + 代码为准 |
| 数据库 / 持久化 | `docs/database/DATABASE-ARCHITECTURE.md`（as-built）+ `08 §六` 三栏对账 | 生效自 2026-09-17 as-built；**关键例外**：`mox-v3.0-baseline.sql` 是 **MySQL 8.3 目标模板（ea_* 11 表），不是 PG 现状**（DVR §3.3，任务书称「PG 基线」实为 MySQL）；现状是嵌入式 SQLite 7 表 |
| 代码事实 / 缺口证据 | 三处 `_verification/` 报告（BVR/FVR/DVR，文件:行号） | 文档结论可争议，代码行号不争议；2026-09-27 快照行号 |
| 传输加密语义（`MOX_API_CRYPTO=sm4`） | `docs/api/API-CRYPTO-TRANSPORT.md` | 6 处挂载点已落地（BVR §2.1(6)）；算法本体在外部 `mox-api-crypto` crate，本仓库 alliance/ 内零 SM4/gzip 实现 |
| 文档历史 / 被取代文档 | `_archive/v1~v3/` + enterprise 39 号 | 均为历史快照，勿作现行引用；39 号见 §二.6 |

**整合视图例外**：13 号 `13-end-to-end-business-flow.md`（INDEX 编号 21）虽为 🟢权威定位，但其「流程整合视图」仍**回落 CURRENT V1.1 + PORT-REGISTRY**（INDEX 表 21 末列），不另立现状事实。同理 08（导航总图）现状回落 CURRENT；10/11/12（评审/对标/规划）不改变现状裁决链。

---

## 四、文档体系最终索引锚点（INDEX 编号 00–21 全表）

> 口径：编号取自 INDEX.md §一（逻辑阅读序号，非文件数字前缀）；「回溯锚点」= 读它解决什么问题。

| INDEX 编号 | 文档（文件） | 权威等级 | 一句话定位 | 读完得到什么 / 回溯锚点 |
|---|---|---|---|---|
| 00 | `CURRENT-ARCHITECTURE.md` | 🟢 权威 | 当前实现架构（唯一现状裁决） | 现状争议的最终答案；第 0 读裁决基准 |
| 01 | `08-normalized-architecture.md` | 🟢 权威 | 三目录对齐归一化总图（40 项矩阵/缺口/路线/排障） | 一图看全 + 缺口在哪（§九）+ 怎么演进（§十） |
| 02 | `09-deployment-templates.md` | 🟢 权威 | 企业级生产部署配置模板（两档/compose/k8s/52 env/检查清单） | 上生产怎么配、怎么自检；MOX_* 开关权威 |
| 03 | `01-prd.md` | 🟡 目标态 | PRD（业务目标/五角色/阈值，含 F-04 WS 等过期写法） | 当初要做什么；勿当现状 |
| 04 | `02-architecture.md` | 🟡 目标态 | 总体架构 V1.0（分层依赖/技术选型，接入层写 REST/WS） | 设计意图；冲突回落 CURRENT |
| 05 | `03-business-flow.md` | 🟡 目标态 | 业务流程 V1.0（六步主流程/注册探活闭环） | 业务怎么走；跨服务/状态过滤写法已过期 |
| 06 | `04-state-machine.md` | 🟡 目标态 | 状态机/业务规则（枚举值已过期） | 状态流转设计；枚举以 08 §八统一字典为准 |
| 07 | `05-data-model.md` | 🟡 目标态 | 数据模型/字典（含 plans/tasks 表幻觉） | 字段字典；存储对账以 08 §六为准 |
| 08 | `06-api-spec.md` | 🟡 目标态 | API 规范 V1.0（含 /ws/v1 幻影、漏 /api/alliance/*） | 接口设计意图；真端点以 CURRENT §6.1 为准 |
| 09 | `07-deployment.md` | 📄 参考 | 早期启动步骤 + 极简 compose（漏 MOX_* env） | 怎么先跑起来；生产以 02(=09-deployment-templates) 为准 |
| 10 | `index.html` | 📄 参考 | 旧版可视化图形导航 | 浏览器快速浏览全貌 |
| 11 | `expert-alliance-whitepaper.html` | 📄 参考 | 自包含白皮书单文件（四层/矩阵/时间线/开关/排障） | 5 分钟建立全貌，数字回落源文档 |
| 12 | `_verification/docs-verification-report.md` | 📁 核验记录 | docs↔code↔architecture 三方对账 | 为什么有冲突；三类硬冲突证据 |
| 13 | `platform/.../_verification/backend-verification-report.md` | 📁 核验记录 | 后端 30 项摘要核验（28✅/1❌/1⚠️）+ N1–N11 | 后端代码事实溯源（文件:行号） |
| 14 | `frontend-ui/.../_verification/frontend-verification-report.md` | 📁 核验记录 | 前端契约纪律 + G1–G10 | 前端代码事实溯源 |
| 15 | `platform/.../_verification/backend-fix-report.md` | 📁 核验记录 | N1/N2/N3/N6/N9 修复 + 鉴权链路 + G4 收敛 + P0-C | 缺口怎么被修掉（含测试数） |
| 16 | `frontend-ui/.../_verification/frontend-fix-report.md` | 📁 核验记录 | G1/G2/G3 + legacy 30+ 调用点收敛 | 前端缺口怎么被修掉（vitest 646→980/981） |
| 17 | `scripts/`（sh/ps1/README） | 🛠️ 工具 | 部署后一致性一键校验（四进程/health、200-401、SM4、HA、审计） | 上生产/CI 跑自检；逻辑以 09 §六为权威 |
| 18 | `10-enterprise-maturity-review.md` | 📄 评审结论 | 40 项成熟度 7 维评审（✅22/🟡14/🔴4） | 够不够企业级、P0 阻断项来源 |
| 19 | `11-competitive-benchmark.md` | 📄 外部研究 | 四类 10 竞品对标（135 处来源 URL） | 对标最好用、强弱清单 |
| 20 | `12-innovation-roadmap.md` | 📄 规划 | 17 张创新卡片 + P0 前置 + 三阶段里程碑 | 下一步排期；不改变现状 |
| 21 | `13-end-to-end-business-flow.md` | 🟢 权威 | 端到端全业务流程 8 步详解 + 状态机总图 | 任务怎么流转；细节回落 CURRENT/PORT-REGISTRY |
| 22 | `14-enterprise-permission-model.md` | 🟢 现状核证 | 企业级权限/鉴权/审计链路逐层对账到代码（先读码后落笔） | 谁能干什么（角色/鉴权/审计 Actor）；与 39 历史角色、12 的 A1/G3 对齐 |
| 23 | `15-product-spec-standard.md` | 🟡 验收基线 | 把成熟度差距翻译成「可验收规范条目 + QA 可执行检查点」 | 「企业级最好用」达标长什么样、QA 怎么验；不改变现状事实 |
| 24 | `16-decision-and-state-ledger.md`（本账） | 📁 记录权威 | 全主线决策与状态总账（缺口/取舍/规划逐条可回溯） | 查任意 N/G/D/P0/T/U/A/M 项的状态与证据 |

> 本账（16）已作为编号 24 登记在上表；14/15/16 三份新文档均已落盘（2026-09-30）。本账不另立现状事实，与 CURRENT/08/12/INDEX 无冲突。

---

## 五、主线条目（时间线，每步一行）

| 日期 | 产物 / 动作 | 状态 |
|---|---|---|
| 2026-09-24 | `CURRENT-ARCHITECTURE.md` V1.1 现状基线落地 | 🟢 权威现状基准 |
| 2026-09-25 | 01–07 V1.0 目标态文档成文（后补「被取代」横幅） | 🟡 目标态 |
| 2026-09-27 | 三方代码事实核验（BVR/FVR/DVR）：30 项摘要 + N1–N11/G1–G10/三类硬冲突 | 📁 核验完成 |
| 2026-09-27 R1 | 首轮修复：N1/N2/N3（后端）+ G1/G2/G3（前端）+ 网关出站令牌注入 | ✅ 闭环（cargo 85 / vitest 646） |
| 2026-09-27 R2 | 中低严重度收尾：N6（.expect 降级）+ N9（动词面 9→8） | ✅ 闭环 |
| 2026-09-28 | G4 后端客户端归一化结论（re-export SDK，职责不重叠） | ✅ 归一化结论 |
| 2026-09-29 R3 | P0-C 内部令牌双值滚动 + P0-D WS 文档幻影补记 + 前端 legacy 30+ 调用点收敛 | ✅ 本轮闭环（cargo 128 / vitest 980） |
| 2026-09-29 | INDEX 最近核对、12 规划 last_updated（17 卡片 + 三阶段） | 📄 规划定稿 |
| 2026-09-30 | **本账 16-decision-and-state-ledger.md 成文**（全主线决策与状态总账） | 📁 本次交付 |
| 2026-09-30 R4 | 后端管理写面 RBAC 闭环（experts_rbac.rs，7 handler 强制 super_admin/tenant_admin） | ✅ G-2 闭环（cargo alliance 73） |
| 2026-09-30 R4+ | 前端按钮级权限接入（G-1）：7 个管理写面按钮挂 v-role-any，三端同源 | ✅ G-1 闭环（vitest 649/650，唯一失败为后端 .rs 行号漂移预存在） |
| 2026-09-30 R4++ | 企业级增量优化：T1a DAG 并行信号量（N8）+ 三 svc 指标 Prometheus 文本化（N7）闭环 | ✅ 闭环（executor-core 47 / 三 svc 66 全绿，gateway alliance 73 基线不变） |
| 2026-09-30 R4+++ | **N4 图谱节点级 CRUD 闭环**：6 写端点（节点/边增删改）+ RBAC `graph.mutate` + SQLite 增量落库 + 内存态 version+=1 | ✅ 闭环（cargo alliance 86 = 73+13；前端 graph/store/api 108 例全绿，唯一 vitest 失败为 dispatcher.rs 行号既有漂移） |
| 2026-10-01 U1 | **U1 能力图谱画布 MVP 落地**：增强既有 GraphCanvas.vue（手写确定性 SVG，不引新引擎），节点拖拽（视觉态不入库）+ Inspector 改/删/新增节点 + 点两节点连线 + 选中节点展开 T2 邻域幂等并入；管理写面按钮 v-role-any 三端同源 | ✅ MVP 闭环（vitest 78 文件/1044 全绿；DAG 导出/实时回显/虚拟滚动留待） |
| 2026-10-01 T2 | **图 RAG 落地**：POST /api/expert-graph/rag/expand（experts_graph.rs:1612），内存态加权多跳扩展（权重乘积聚合、hybrid_rerank 扩展点）；实现由规划期「SQLite 递归 CTE」改为内存态，A4 图库迁入时换查询实现 | 🟡 部分闭环（向量融合待 #27；gateway alliance 97 = 86+11 RAG） |
| 2026-10-01 T3 | **MCP Server 落地**：platform/domains/alliance/mcp/mox-alliance-mcp-server/src/main.rs stdio 自实现 JSON-RPC 2.0（Content-Length 帧），3 工具 expert_search(:69)/optimal_team(:82)/graph_expand(:96)，非 HTTP 路由、不进 actuator ROUTES | 🟡 部分闭环（cargo test mox-alliance-mcp-server 5；Client/凭证托管留待 M2） |
| 2026-10-01 U2 | **匹配透明化落地**：api/dto.rs:169 ScoreDim{value,weight}/:181 ExpertScoreView{domain,capability,health,priority,performance,total}，scheduler-svc /experts/search 透出，http-sdk+alliance_remote 双路径透传；前端 MatchExplainPanel.vue 逐维条形图（AllianceExpertsView 消费）；健康权重口径纠错 0.15→0.05 | ✅ 闭环（scheduler-core 115 + scheduler-svc/http-sdk 38 全绿） |

> 账实同步（2026-09-30）：14 权限模型、15 产品规范、16 总账三份文档均已落盘，见 §四 索引锚点编号 22/23/24。

## 六、2026-10-01 文档治理增量

EA-DOC-01–07 的整理与证据边界见 `docs/expert-alliance/17-docs-architecture-and-flow-atlas.md#conflicts`；目标实施任务 EA-W01–10 及质量场景 EA-Q01–16 见 `docs/expert-alliance/18-modular-product-design.md#roadmap`。本轮仅文档与资料盘点工具交付，不新增任何运行时能力完成记录。N4/N7/N8 的后端实现与产品运行验收分别判定；15 已校正近期口径。

## 七、逐目录低代码文档增量（2026-10-01）

LC-DIR-01、LC-STD-001、控制面/契约/存储/模块/前端/联盟配方文档及24个主题入口设计卡已编制。来源[目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#progress)；目标语义[统一规范](docs/standards/lowcode-dynamic-configuration.md#scope)。这是document_ready，不新增运行时配置中心、热发布或业务E2E完成记录。

## 八、业务计划与执行归一化增量（2026-10-01）

[20实施边界](docs/expert-alliance/20-normalized-plan-execution.md#acceptance)：统一计划值类型校验，执行/恢复共用任务绑定检查，通用DAG排序可复现；七种模式经测试执行器验证节点与融合结果。EA-W03/W05仅此部分已实施；release快照、真实模型与生产恢复不标完成。验证来源reports/data/20261001-flow-implementation-checks.json。

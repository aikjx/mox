---
title: 专家联盟企业级模块化归一化架构（导航+现状+目标合一）
version: V1.0
authority: 🟢权威（与 CURRENT-ARCHITECTURE.md 并列；现状事实以 CURRENT V1.1 为最终裁决）
doc_id: EA-ARCH-NORM-001
last_updated: 2026-09-27
source_of_truth: 2026-09-27 三方代码事实核验（platform 后端 / frontend-ui 前端 / docs 文档体系）；核验报告位于各工程 `_verification/` 目录
scope: 打通 platform / frontend-ui / docs 三目录的归一化架构视图，产出全维功能矩阵、缺口清单与目标态路线
---

# 专家联盟企业级模块化归一化架构

> **本文档定位**：不重复 CURRENT-ARCHITECTURE.md 的代码事实，而是做三件事——
> ① 把三目录（platform / frontend-ui / docs）的事实在一张矩阵上对齐；
> ② 收录 2026-09-27 三方核验新发现的缺口（含 §8/§9 增量核验）；
> ③ 给出归一化演进路线与故障排查手册。
>
> **权威优先级**：现状事实 → [CURRENT-ARCHITECTURE.md V1.1](CURRENT-ARCHITECTURE.md)；端口 → [PORT-REGISTRY](../api/PORT-REGISTRY.md)；数据库 → [DATABASE-ARCHITECTURE](../database/DATABASE-ARCHITECTURE.md)。本文与上述任一冲突时，以它们为准。

---

## 一、文档导航与权威链

### 1.1 专家联盟文档全景（2026-09-27 实测）

```
docs/expert-alliance/
├── CURRENT-ARCHITECTURE.md     🟢 现状唯一权威（V1.1, 2026-09-24）
├── 08-normalized-architecture.md  🟢 本文（归一化导航+矩阵+缺口+路线）
├── 01-prd.md                   🟡 V1.0 目标态（部分与 V1.1 冲突，见 §1.2）
├── 02-architecture.md          🟡 V1.0 目标态
├── 03-business-flow.md          🟡 V1.0 目标态
├── 04-state-machine.md         🟡 V1.0 目标态
├── 05-data-model.md            🟡 V1.0 目标态（含 plans 表幻觉）
├── 06-api-spec.md              🟡 V1.0 目标态（含 WS 幻觉、漏 /api/alliance/*）
├── 07-deployment.md            🟡 V1.0 参考（漏 MOX_* env 表）
├── index.html                  可视化导航
└── _archive/                   v1/v2/v3 历史版本（已归档，勿引用）

关联权威：
├── docs/api/PORT-REGISTRY.md           端口唯一权威
├── docs/api/API-CRYPTO-TRANSPORT.md    MOX_API_CRYPTO 权威
├── docs/database/DATABASE-ARCHITECTURE.md  as-built 持久化权威
└── docs/enterprise/39-*.md              历史诊断（已退役快照，勿作现行引用）
```

### 1.2 01-07 与 CURRENT V1.1 的三类硬冲突（已核实）

| 冲突类型 | 01-07 写法 | V1.1 代码实测 | 处置 |
|---------|-----------|--------------|------|
| **WebSocket 幻影** | 06 §4、02 §1.2、01 F-04、03 §4 均写 `/ws/v1/*` 进度推送 | 全 crate `WebSocketUpgrade` **零命中**；实时性仅 SSE `GET /api/alliance/tasks/:id/logs/stream` | 本文以 V1.1 为准；06 文档待修订 |
| **plans/tasks 表幻影** | 05 §1/§9 称 Task/Node/Plan 落 SQLite `experts_db.rs` | 实读 experts_db.rs:91-158 仅 7 表（experts/sessions/session_messages/graph_nodes/graph_edges/graph_meta/bookings），**无 plans/tasks/nodes 表** | 本文 §六 给三栏对账 |
| **专家状态枚举三套** | 01/03/04 用 `active/inactive`；05 用 `active/busy/offline/inactive` | V1.1 实测网关 `availability.status` = `online/busy/offline/away`（experts_registry.rs:127-128） | 本文 §八 统一枚举字典 |

> **结论**：01-07 全部为 V1.0（2026-09-25）目标态文档，文首均未标注"已被 V1.1 部分取代"。**引用 01-07 时必须对照 CURRENT V1.1，不得直接当现状引用。**

---

## 二、模块清单与物理代码分布（三目录打通）

### 2.1 后端 platform（16 crates + 网关 11 文件）

| 层 | 路径 | 模块数 | 关键文件 |
|----|------|--------|---------|
| api | `domains/alliance/api/` | 1 crate | HTTP DTO |
| core | `domains/alliance/core/` | 6 crates | mox-alliance-core（dag/fusion/utils）、scheduler-core（13 模块）、executor-core（6 模块）、config-core、boot-config、registry-core |
| proto | `domains/alliance/proto/` | 4 crates | common/scheduler/executor/registry-proto（协议先行） |
| sdk | `domains/alliance/sdk/` | 2 crates | mox-alliance-sdk、mox-alliance-http-sdk（含 alliance_remote.rs 远程模式切换） |
| svc | `domains/alliance/svc/` | 3 crates | scheduler-svc(:3100) / executor-svc(:3200) / registry-svc(:3400) |
| 网关内联 | `gateway/mox-platform-gateway-svc/src/alliance/` | 11 .rs（9924 行） | experts_collaboration(2227)/orchestration(1236)/dispatcher(1173)/graph(1099)/common(1014)/registry(1009)/session(959)/db(721)/ext(366)/registry_client(92)/mod(28) |

### 2.2 前端 frontend-ui（模块 + 视图两层）

| 层 | 路径 | 模块数 | 关键文件 |
|----|------|--------|---------|
| 模块内 api | `src/modules/expert-alliance/api/` | 1 文件（19KB） | alliance.api.js（68 端点 key） |
| 模块内 contract | `contract/` | 11 源文件 + 对应 .test.js | endpoints.js（68 key + 11 UNMOUNTED_ROUTES）、collab/dispatcher/graph/mode/orchestration/registry/sessions |
| 模块内 store | `store/` | 6 Pinia stores | collab/console/experts/graph/orch/sessions |
| 模块内 views | `views/` | 6 .vue | Console(1483行)/Experts(749)/Orchestration(348)/Sessions(157)/Graph(134)/Collab(92) |
| 模块内 components | `components/` | 16 .vue | ExpertCollabPanel/Session×4/Graph×4/ExpertCard/RankBoard/Booking/RegistryForm/CapabilityMatrix/SemanticSearch |
| 外部 legacy 视图 | `src/views/expert/` + `src/views/workspace/` | 5 .vue | AllianceTaskView(935)/ExpertCenterView(1021)/ExpertConfigView(5414)/ExpertPlazaView(1921)/ExpertWorkspaceView(891) |
| legacy API 客户端 | `src/api/` | 2 文件 | alliance.api.js（旧层，含假端点 /qa 与禁端点 /ai/engine/alliance/*）、experts.api.js |

> **关键发现**：前端存在**两套并行 API 客户端**——模块内 `allianceApi`（契约纪律极强，68 key 全部对齐后端 registry id）与 legacy `@/api/*.js`（绕过模块契约，含 1 条假端点 + 2 条运行中的禁端点）。归一化必须收敛到一套。

### 2.3 文档 docs（01-08 + 关联权威）

见 §1.1 全景图。本文为 08 号，与 01-07 数字序连续。

---

## 三、全维功能矩阵（三方对齐）

> **判定口径**：✅已实现（代码+前端+文档三方对齐）｜🟡部分（后端有但前端未消费 / 进程内即失 / 默认关闭）｜🔴缺口（应有没有 / 假端点 / 冲突）｜❓未核

| # | 功能维度 | 后端能力（代码位置） | 前端模块/视图 | 权威文档章节 | 状态 |
|---|---------|---------------------|--------------|-------------|------|
| 1 | **专家注册 CRUD** | experts_registry.rs（网关 SQLite 表 experts，:91）；POST/GET/PUT/DELETE /api/experts[/:id] | allianceExperts store + AllianceExpertsView + ExpertRegistryForm | CURRENT §5；05 §5 | ✅ |
| 2 | **专家可用性状态** | availability.status = online/busy/offline/away（登记值，非探活），experts_registry.rs:127-128 | ExpertCard.vue:118 忠实呈现为标签 | CURRENT §1.2 补记；01 F-02 误写 active | ✅（但文案须守纪律） |
| 3 | **专家收藏 favorites** | experts_common.rs:472 `Arc<Mutex<HashSet<String>>>`，**进程内即失** | allianceExperts store:24 `favoriteSessionOnly=true`，ExpertCard.vue:120 明写"仅本次会话" | CURRENT §5 | 🟡 进程内 |
| 4 | **专家预约 booking** | experts_ext.rs + 表 bookings（experts_db.rs:158），SQLite 落盘 | allianceExperts store + ExpertBookingPanel | CURRENT §1.2 | ✅ |
| 5 | **专家指标/排行** | experts_registry.rs:180-192 compute_platform_metrics | allianceExperts store + ExpertRankBoard | 01 §3 | ✅ |
| 6 | **多轮协作辩论** | experts_collaboration.rs:496 run_debate（rounds 真循环，2227 行） | allianceCollab store + ExpertCollabPanel（POST /api/experts/debate） | 03 §3；CURRENT §1.2 | ✅ |
| 7 | **任务分解** | experts_orchestration.rs:99-140 按 task_type 选固定步骤表 | allianceOrch store + AllianceOrchestrationView（POST /api/experts/orchestrate） | 03 §1；CURRENT §1.2 补记 | ✅ |
| 8 | **7 种协作模式（DAG）** | planner.rs:81-91 AllianceMode 7 臂：Sequential/Parallel/Voting/Hierarchical/Debate/Iterative/Dynamic | allianceCollab store mode 枚举 + contract/mode.js | CURRENT §3.7；05 §7 FusionStrategy 误写 9 值 | ✅（7 种，非 6 非 9） |
| 9 | **结果融合（6 策略）** | mox-alliance-core/src/fusion/strategies/（confidence_weighted/debate/iterative/map_reduce/stacking/weighted_vote） | 前端通过 /api/alliance/tasks/:id/fusion-result 消费 | CURRENT §1.1；05 §7 | ✅ |
| 10 | **图谱关联查询** | experts_graph.rs（1099 行）：8 条只读路由 + rebuild POST | allianceGraph store + AllianceGraphView + GraphCanvas/NodeInspector/TeamPanel/MetricsPanel | 05 §6；CURRENT §1.2 | ✅ |
| 11 | **图谱节点级 CRUD** | **grep `INSERT/UPDATE/DELETE graph` 零命中**；唯一写是 POST /rebuild（全量重建） | 前端 graph store rebuildGraph 对应 | 05 §6 未提 | 🔴 缺口（N4） |
| 12 | **调度匹配（模块化权重）** | scheduler-core/modular_matcher.rs（生产主路径）；matcher.rs 是 fallback；健康度 0.15 加权非过滤（matcher.rs:166,122） | allianceExperts store searchExpertMatches（POST /api/alliance/experts/search） | 01 F-02 误写 RuleBased 主路径 + "状态过滤" | ✅（但 01 文档口径错） |
| 13 | **计划生成器** | scheduler-core/planner.rs SimplePlanGenerator；网关侧 experts_orchestration.rs 做固定步骤表+Kahn+预演文案 | allianceOrch store generateOrchPlan | CURRENT §3.7 补记 | ✅ |
| 14 | **执行器 DAG 引擎** | executor-core/dag_engine.rs（拓扑调度/并行执行）；expert_executor.rs（超时 300s + 重试 3 + 指数退避） | AllianceConsoleView 消费 /api/alliance/tasks/:id/dag + /nodes | 03 §5；CURRENT §3.2 | ✅ |
| 15 | **DAG 并行度上限** | **grep `max_parallel/concurrency/Semaphore` 零命中**；靠 tokio 自然调度 | 前端无配置入口 | — | 🔴 缺口（N8） |
| 16 | **注册中心（独立服务）** | registry-svc:3400 + registry-proto 契约层 + 10:1:1 分级心跳聚合（aggregation.rs:11,489） | 前端无直接消费（网关代理） | CURRENT §8 | ✅ |
| 17 | **注册中心主动探活** | health_probe.rs 代码真实发 HTTP GET，但 `health_probe_enabled: false`（app_state.rs:56） | 前端无开关 | — | 🟡 默认关闭（N5） |
| 18 | **调度器多活（HA）** | leadership.rs:110 LeaseStore trait / :194 LeaderElector / storage.rs:890 SqliteLeaseStore；fencing epoch（:57,100,186）；leader 专属对账（ha.rs:200） | 前端无 HA 状态展示 | CURRENT §8；07-massive-scale:86 | ✅（需 MOX_ALLIANCE_HA_MODE=on） |
| 19 | **权限审计** | 网关 auth_middleware（auth.rs:143 JWT 校验）+ 审计 SHA-256 哈希链（experts_common.rs:521-574，NDJSON 落盘） | 路由 requiresAuth（6 控制台路由无 requiresRole） | 01 §3 五角色 | 🟡 审计无用户身份（N2）；前端按钮级权限零命中（G3） |
| 20 | **下游 svc 鉴权** | scheduler/executor/registry 路由仅挂 tracing+crypto，**无 JWT 中间件** | — | — | 🔴 缺口（N1） |
| 21 | **传输加密（SM4）** | MOX_API_CRYPTO=sm4 一键开关；6 处挂载点（gateway lib.rs:332 / scheduler routes.rs:36 / executor routes.rs:93 / registry routes.rs:60 / executor_bridge.rs:112 / alliance_remote.rs:156） | 前端无协商（信封层透明） | API-CRYPTO-TRANSPORT:29；CURRENT §8 | ✅ |
| 22 | **存储迁移（JSON→SQLite）** | experts_db.rs:604 migrate_json_to_sqlite()，导入后 rename 归档 | — | 05 §9 | ✅（一次性，无版本迁移机制） |
| 23 | **SQLite schema 版本管理** | **grep `PRAGMA user_version/schema_version` 零命中** | — | — | 🔴 缺口（N3） |
| 24 | **SSE 日志流** | GET /api/alliance/tasks/:id/logs/stream（actuator.rs:530） | AllianceConsoleView.vue:690 useSSE composable（fetch-stream + 4s 轮询回退） | CURRENT §6.1 补记 | ✅ |
| 25 | **WebSocket** | **全 crate WebSocketUpgrade 零命中** | 前端 useSSE 而非 WS | 06 §4 幻影 | 🔴 文档幻觉（代码无） |
| 26 | **会话管理** | 表 sessions/session_messages（experts_db.rs:108,124），SQLite 落盘 | allianceSessions store + SessionsView + Session×4 Panel | 03 §2 | ✅（单进程内恢复） |
| 27 | **语义搜索** | POST /api/experts/semantic-search | allianceSessions store + SemanticSearchPanel | 01 F-05 | ✅ |
| 28 | **编排计划 plans** | experts_common.rs:468 `Arc<Mutex<HashMap<String, CollaborationPlan>>>`，**进程内即失**，无 GET 列表端点 | allianceOrch store hasPlan 注释明写"重启后旧 plan_id 会 404" | 05 §1 误写 SQLite plans 表 | 🟡 进程内 |
| 29 | **编排历史 orchestration_history** | experts_common.rs:470 `Arc<Mutex<Vec<OrchestrationRecord>>>`，**进程内即失** | AllianceOrchestrationView:200 空态写"本页没有记录"+"仅本次进程"角标 | 03 §7 | 🟡 进程内（前端文案合规） |
| 30 | **熔断（网关侧）** | experts_dispatcher.rs:494 circuit_breakers（独立内存 map，初始空但失败后填充） | AllianceConsoleView dispatcherStatus 消费 | — | 🟡 与 scheduler 侧熔断不共享（N10） |
| 31 | **熔断（scheduler 侧）** | scheduler-core/llm_router.rs:477 circuit_break_until（threshold=5, duration=60s） | — | — | ✅（与网关侧两套独立） |
| 32 | **LLM 多 Provider 路由** | scheduler-core/llm_router.rs（多 Provider 智能路由） | — | 02 §3.2 | ✅ |
| 33 | **指标 /metrics** | scheduler/executor 返回 JSON 快照（非 Prometheus 文本）；gateway 侧是 Prometheus 文本 | 前端无指标面板 | 07 §5.1 误写"gateway 侧" | 🟡 格式不统一（N7） |
| 34 | **内置专家（模块目录 11 个）** | config-core/examples/domain_experts.rs:108-132 build_domain_experts() 返回 11 条 | 前端无直接消费（scheduler 装载） | CURRENT §7.1 | ✅ |
| 35 | **内置专家（网关种子 10 位）** | experts_common.rs:645-660 seed_builtin_experts()，exp-*-001 具名专家 | allianceExperts store listExperts 读到 | CURRENT §7.2 | ✅ |
| 36 | **legacy 工作台禁端点** | — | ExpertWorkspaceView.vue:802 调 /ai/engine/alliance/capabilities；useAlliance.js:48 调 /ai/engine/alliance/full（POST SSE） | 模块 contract/endpoints.js:135-136 明令禁止 | 🔴 缺口（G1） |
| 37 | **legacy 假端点 /qa** | 后端 registry 无此路径 | src/api/alliance.api.js:315 askAllianceTaskQa（死代码） | — | 🔴 缺口（G2） |
| 38 | **前端路由权限** | — | 6 个 /alliance/* 控制台路由仅 requiresAuth，无 requiresRole；按钮级 v-permission 零命中 | 01 §3 五角色 | 🟡 缺口（G3） |
| 39 | **前端 store 测试覆盖** | — | console.store(356行) 与 orch.store(219行) 无独立 .test.js | — | 🟡 缺口（G6） |
| 40 | **大列表虚拟滚动** | — | 全仓 virtual/VirtualScroll 零命中；图谱节点手写 SVG 全量渲染 | — | 🟡 缺口（G7） |

> **矩阵统计**：✅已实现 22 项｜🟡部分/进程内/默认关闭 11 项｜🔴缺口/幻觉 7 项。所有 🔴 项的代码证据见 §九 缺口清单。

---

## 四、服务拓扑与端口单表

> 端口权威：[docs/api/PORT-REGISTRY.md](../api/PORT-REGISTRY.md) V1.2。下表补齐 CURRENT §2.1 漏登项。

| 进程 | 端口 | 职责 | 生效代码 | PORT-REGISTRY |
|------|------|------|---------|---------------|
| platform-gateway-svc | 3080 | 统一入口：REST + 内联专家逻辑（11 文件 9924 行） | gateway/src/main.rs:27, config.rs:181 | ✅:31,50 |
| alliance-scheduler-svc | 3100 | 任务调度/匹配/计划/HA 选主 | boot-config/src/lib.rs:65,184 | ✅:37,64 |
| alliance-executor-svc | 3200 | DAG 执行/节点调度/融合落盘 | boot-config/src/lib.rs:247 | ✅:38,65 |
| alliance-registry-svc | 3400 | 专家注册中心/10:1:1 聚合 | registry-svc/app_state.rs:50 | ✅:67 |
| AI 专家服务桥接 | 3300 | scheduler→专家服务 base_url | expert_service.base_url | ✅:39,66（CURRENT 漏登） |
| codeengine-svc | 3210 | 自研 AI 代码引擎 | MOX_CODEENGINE_PORT | ✅:40,68（CURRENT §2 漏登） |
| gRPC | 50051 | framework 保留，**联盟未使用** | — | ⚠️ PORT-REGISTRY 写"联盟内部 gRPC"，CURRENT 写"未使用"，需统一措辞 |
| 本地开发 | 33080/33100/33200 | start-alliance-local.ps1 默认 | — | 🟡本地 |
| 前端 dev | 33020 | npm run dev --port 33020 | — | 🟡本地 |

---

## 五、运行开关（MOX_* 环境变量）单一参考

> 此前散落在 5+ 份文档。本表为归一化后的单一参考。

| 变量 | 默认值 | 取值 | 作用 | 生产建议 | 权威出处 |
|------|--------|------|------|---------|---------|
| `MOX_API_CRYPTO` | 未设置（关闭） | `sm4` | data gzip+SM4-GCM 全链路加密（6 处挂载点） | **生产必开** | API-CRYPTO-TRANSPORT:29 |
| `MOX_ALLIANCE_REMOTE_MODE` | `auto` | `off` / `auto` | off=强制全进程内；auto=配了 URL 就跨服务 | 本地开发 off；生产 auto | CURRENT:172（仅此一处，无独立专页） |
| `MOX_ALLIANCE_STORAGE_MODE` | `file` | `memory` / `file` / `sqlite` | memory=纯内存；file=JSON 快照（无恢复）；sqlite=WAL 完整持久化+重启恢复 | **生产必设 sqlite** | port-norm:171；CURRENT:219 |
| `MOX_ALLIANCE_HA_MODE` | off | `on` | on=开启租约选主+fencing+leader 对账 | 多副本时 on（必须配合 sqlite） | 07-massive-scale:86；CURRENT §3.1:128 |
| `MOX_ALLIANCE_HA_LEASE_MS` | 10000 | ms | 租约时长 | — | ha.rs:78 |
| `MOX_ALLIANCE_HA_STALL_MS` | 300000 | ms | 孤儿任务判定静默窗口 | — | ha.rs:83 |
| `MOX_ALLIANCE_SQLITE_BUSY_MS` | 5000 | ms | SQLite busy_timeout | — | scheduler-core/storage.rs:412 |
| `MOX_ALLIANCE_REGISTRY_ADDR` | 0.0.0.0:3400 | host:port | registry-svc 监听地址 | — | registry-svc/app_state.rs:75 |
| `MOX_ALLIANCE_REGISTRY_PROBE_ENABLED` | false | bool | 主动健康探测开关 | **生产建议开启** | app_state.rs:95 |
| `MOX_EXECUTOR_NODE_TIMEOUT_MS` | 60000 | ms | 单节点执行超时 | — | executor-core/expert_executor.rs:93 |
| `MOX_EXECUTOR_MAX_RETRIES` | 3 | int | 节点失败重试次数 | — | expert_executor.rs:98 |
| `MOX_AUDIT_LOG_PATH` | data/audit/experts-audit.ndjson | path | 审计哈希链日志路径 | — | experts_common.rs:563 |
| `MOX_AUDIT_HMAC_SECRET` | mox-experts-alliance-audit | string | 审计 HMAC 密钥 | **生产必改** | experts_common.rs:571 |

---

## 六、存储与持久化三栏对账

> 三栏：①网关嵌入式 SQLite（experts_db.rs）②scheduler+executor 任务库 ③目标态 baseline.sql。消除 05-data-model 与现状的口径冲突。

| 数据域 | ① 网关 SQLite（现状） | ② scheduler/executor（现状） | ③ baseline.sql（目标态 MySQL 8.3） | 重启即失？ |
|--------|----------------------|----------------------------|----------------------------------|-----------|
| 专家注册 | 表 experts（experts_db.rs:91） | — | ea_expert（:223） | 否（SQLite 落盘） |
| 专家能力 | —（在 experts JSON 字段内） | — | ea_capability + ea_expert_capability（:224-225） | — |
| 专家领域 | — | — | ea_domain + ea_expert_domain（:226-227） | — |
| 会话 | 表 sessions（:108）+ session_messages（:124） | — | **baseline 未覆盖** | 否（网关 SQLite） |
| 知识图谱 | 表 graph_nodes（:136）/ graph_edges（:143）/ graph_meta（:152） | — | 通用 kg_entity/kg_relation（非联盟专用） | 否（全量 rebuild） |
| 预约 booking | 表 bookings（:158） | — | **baseline 未覆盖** | 否 |
| 协作任务 | **无表** | scheduler-core/storage（SQLite WAL 或 JSON）+ executor state_sink（data/alliance_tasks.db） | ea_task + ea_task_node + ea_task_edge（:229-231） | sqlite 模式否；file 模式是 |
| 融合结果 | **无表** | executor state_sink 保留行 `__fusion_output__` | ea_task_result（:232） | sqlite 模式否 |
| 专家收藏 favorites | **无表**（进程内 HashSet，experts_common.rs:472） | — | 无 | **是** |
| 编排计划 plans | **无表**（进程内 HashMap，:468） | — | 并入 ea_task_node | **是** |
| 编排历史 | **无表**（进程内 Vec，:470） | — | 无 | **是** |
| 审计日志 | **无表**（NDJSON 文件 data/audit/experts-audit.ndjson，:563） | — | 无 | 否（文件落盘） |
| 租约（HA） | — | scheduler-core/storage.rs:939 `alliance_leader_lease` 表（SQLite） | 无 | 否 |
| 案例库 | 无 | 无 | ea_case（:228） | —（目标态领先） |

> **关键结论**：
> 1. 05-data-model.md:18,206,207 称 plans/tasks 在 experts_db.rs SQLite——**代码不属实**；任务在 scheduler/executor 侧，plans 进程内。
> 2. baseline.sql 是 **MySQL 8.3 目标模板**（ENGINE=InnoDB/utf8mb4），不是 PostgreSQL 现状；as-built 是嵌入式 SQLite。
> 3. **"进程内即失"三项**（favorites/plans/orchestration_history）的前端文案已合规（"仅本次会话"/"仅本次进程"角标），但能力上确属缺口，P1 应落盘。

---

## 七、API 清单与路由前缀

> 口径权威：CURRENT §6.1（去重路径）。前端 74 是 (path,method) 行数，两者不可互换。

| 前缀 | 去重路径数 | (path,method) 行数 | 前端模块消费 | 说明 |
|------|-----------|-------------------|-------------|------|
| `/api/experts/*` | 43 | 46 | 68 key 中 51 个走此前缀 | 专家 CRUD/会话/协作/图谱/分发/编排/预约 |
| `/api/alliance/*` | 20 | 28 | 68 key 中 17 个走此前缀（+1 SSE） | 任务全生命周期 + DAG/节点/融合/日志流 |
| `/api/expert-graph/*` | 8 | 8 | graph store 8 个方法 | 图谱只读查询 + rebuild + optimal-team |
| `/ws/v1/*` | **0** | 0 | 无 | **代码零命中**（06 §4 幻影） |
| `/api/ai/engine/alliance/*` | — | — | legacy 工作台 2 个调用点 | 模块契约 **明令禁止**（endpoints.js:135-136），归一化须移除 |

### 7.1 前端模块契约纪律（已核实）

- **68 个端点 key** 全部能在后端 API-REGISTRY 找到同 registry id，**模块内 0 假端点**。
- **11 条孤儿端点** 全部在 `contract/endpoints.js:147-174` `UNMOUNTED_ROUTES` 中被显式定性 `rejected`，并附后端源码行号证据（非遗漏，是有意拒绝）。
- **3 处文案红线全部未踩**：①无"在线检测通过"（online 仅作可用性标签）；②无"尚无编排记录"（写"本页没有记录"+"仅本次进程"角标）；③favorites/plans/history 都如实标注进程内。

### 7.2 legacy 层必须清理的 3 个点

| # | 证据 | 问题 | 归一化动作 |
|---|------|------|-----------|
| G1 | `ExpertWorkspaceView.vue:802` + `useAlliance.js:48` | 调模块明令禁止的 `/ai/engine/alliance/full` 与 `/capabilities` | 迁移到模块 allianceApi 或标记废弃 |
| G2 | `src/api/alliance.api.js:315` | 假端点 `POST /alliance/tasks/:id/qa`（后端无） | 删除死代码 |
| G3 | `src/api/alliance.api.js:289,191,195,310,269` | 把已定性为 stub/重复的 5 条端点重新请回（死代码但埋雷） | 删除或标记 @deprecated |

---

## 八、状态机与枚举字典（统一版）

> 消除 01/03/04/05 之间的枚举矛盾。**以代码为准**。

| 枚举 | 权威位置 | 取值 | 01-07 旧写法（已过期） |
|------|---------|------|----------------------|
| **专家状态 availability.status** | experts_registry.rs:127-128 | `online` / `busy` / `offline` / `away` | 01/03/04 写 `active/inactive`；05 写 `active/busy/offline/inactive` |
| **任务状态 TaskStatus** | scheduler-core/storage.rs | `pending` / `planning` / `running` / `paused` / `completed` / `failed` / `cancelled` | 04 §1.1 写 `pending→planning→executing→fusing`（无 executing/fusing，多 paused） |
| **节点状态 NodeStatus** | executor-core/dag_engine.rs | 含 `ready` / `running` / `success` / `failed` / `cancelled` | 04 §1.2 缺 ready/cancelled |
| **协作模式 AllianceMode** | planner.rs:81-91 | `Sequential` / `Parallel` / `Voting` / `Hierarchical` / `Debate` / `Iterative` / `Dynamic`（**7 种**） | 05 §7 FusionStrategy 写 9 值（混淆了模式与策略） |
| **融合策略 FusionStrategy** | mox-alliance-core/fusion/strategies/ | `confidence_weighted` / `debate` / `iterative` / `map_reduce` / `stacking` / `weighted_vote`（**6 种**） | 05 §7 列 9 值 |
| **健康度** | matcher.rs:166 | `is_healthy ? 1.0 : 0.3`，按 **0.15 权重**参与总分（**加权非过滤**） | 01 F-02 / 03 §2 误写"状态过滤" |

> **统一原则**：`availability.status` 是**登记值**（注册时写入），不是探活结果。前端文案不得把它表述为"在线检测通过"。主动探活仅在 registry-svc 侧（默认关闭，N5）。

---

## 九、已核实缺口清单（按严重度排序）

> 以下为 2026-09-27 三方核验新发现的缺口，均附代码证据。已知缺口（favorites/plans/history 进程内、无 WS、availability 登记值、engine_status 恒 running、experts_db 零 fs::write）已在 CURRENT V1.1 记录，不重复列出。

### 🔴 高严重度

| # | 缺口 | 代码证据 | 影响 | 归一化建议 |
|---|------|---------|------|-----------|
| N1 | **下游三 svc 无鉴权中间件** | scheduler/routes.rs:34-36、executor/routes.rs:91-93、registry/routes.rs:59-60 仅挂 tracing+crypto，无 JWT | 任何能直连 :3100/:3200/:3400 的客户端可绕过网关鉴权 | 下游 svc 增加 JWT 校验或 X-Tenant-Id 签名校验 |
| N2 | **审计事件 Actor 硬编码 system** | experts_common.rs:593 `AuditActor::system()`；dispatcher/registry/session 所有 emit_audit 调用均不传用户 ID | 审计哈希链虽防篡改（NDJSON 落盘），但无法追溯"谁干的" | 从 ApiAuth extractor 取 UserInfo 注入 AuditActor |
| N3 | **SQLite 无 schema 版本迁移** | grep `PRAGMA user_version/schema_version` 全 alliance/ 零命中；experts_db.rs:604 仅一次性 JSON→SQLite 导入 | 未来加列时老库不会自动 ALTER，新代码读旧库报错 | 引入 `PRAGMA user_version` + 版本号迁移脚本 |
| G1 | **legacy 工作台调禁端点** | ExpertWorkspaceView.vue:802 / useAlliance.js:48 调 `/ai/engine/alliance/full` 与 `/capabilities`；模块 endpoints.js:135-136 明令禁止 | 契约口径分裂，网关 :3080 行为依赖 catch-all 代理 | 迁移到模块 allianceApi 或标记废弃 |
| G2 | **legacy 假端点 /qa** | src/api/alliance.api.js:315 `askAllianceTaskQa` → POST `/alliance/tasks/:id/qa`（后端 registry 无此路径） | 死代码，接上即 404 | 删除 |
| G3 | **前端控制台路由无角色校验** | modules/expert-alliance/index.js:82-146，6 个 /alliance/* 控制台路由仅 requiresAuth；按钮级 v-permission 零命中 | 负载重置/调度配置写/图谱重建等破坏性写面，任何登录用户可操作 | 补 requiresRole + 按钮级权限指令 |

### 🟡 中严重度

| # | 缺口 | 代码证据 | 归一化建议 |
|---|------|---------|-----------|
| N4 | 图谱无节点级 CRUD，只能全量 rebuild | experts_graph.rs:909-916 仅 8 条只读路由 + rebuild；grep `INSERT/UPDATE/DELETE graph` 零命中 | 补 POST /nodes、POST /edges，或专家 CRUD 时自动增量维护图谱 |
| N5 | registry 主动健康探测默认关闭 | registry-svc/app_state.rs:56 `health_probe_enabled: false` | 生产默认开启或文档明确标注需显式设 true |
| N6 | registry_client.rs:26 生产 `.expect()` 恐慌点 | `gateway/src/alliance/registry_client.rs:26` `.build().expect(...)` | 改为返回 Result 并降级为 None |
| N7 | scheduler/executor /metrics 是 JSON 非 Prometheus 文本 | scheduler/routes.rs:112 `Json(state.metrics.snapshot())`；gateway 侧是 Prometheus 文本（o11y.rs:21） | 补 Prometheus 文本格式端点 |
| N8 | DAG 执行器并行度无信号量上限 | grep `max_parallel/concurrency/Semaphore` 在 dag_engine.rs 零命中；靠 tokio 自然调度 | dag_engine 加 Semaphore 限并发，防打爆 LLM 配额 |
| G4 | 两套并行 API 客户端+归一化 | 模块 allianceApi（信封+normalize.js 1275 行）vs legacy @/api/*.js（无统一信封） | 收敛到模块 allianceApi，删除 legacy |
| G5 | SSE 仅控制台真接，外部任务视图靠轮询 | AllianceConsoleView.vue:690 useSSE；AllianceTaskView.vue 用旧 getExecutionLogsSSE 手写 reader | 统一到 useSSE composable |
| G6 | console/orch 两个 store 无测试 | alliance-console.store.js(356行) / alliance-orch.store.js(219行) 无独立 .test.js | 补 store 测试 |
| G8 | 大列表无虚拟滚动 | 全仓 virtual/VirtualScroll 零命中；GraphCanvas 手写 SVG 全量渲染 | 专家量大后引入虚拟滚动 |

### 🟢 低严重度 / 观察项

| # | 项 | 证据 | 说明 |
|---|---|------|------|
| N9 | scheduler 动词面数文档口径 | CURRENT §6.2 写"9 个动词面"，实测 8（routes.rs:23-32） | 文档改"9"为"8" |
| N10 | 两套熔断状态不共享 | scheduler-core/llm_router.rs:477 真实熔断 vs 网关 dispatcher.rs:494 独立内存 map | 文档说明边界 |
| N11 | 会话单进程内恢复 | sessions 落网关本地 SQLite，网关未做多活 | 网关扩容时需 sticky session |
| G9 | 图谱可视化三套技术栈 | 模块 GraphCanvas.vue 手写 SVG（确定性布局）vs ExpertCenterView.vue:552 手搓力导向 vs ExpertEnterprisePanel.vue:972 echarts force | 归一化到一套 |
| G10 | 文案硬编码中文 | 全仓 $t/vue-i18n 零命中 | 内网政务场景可接受，多语言需求时整体改造 |

---

## 十、目标态演进路线（归一化）

> 对齐 CURRENT §8。已完成项标 ✅，进行中/待办标 P0-P3。

| 维度 | 现状（2026-09-27 核实） | 目标态 | 优先级 | 归一化动作 |
|------|----------------------|--------|--------|-----------|
| **传输加密** | ✅ MOX_API_CRYPTO=sm4 一键开关，6 处挂载点已落地 | 生产密钥注入 + 前端协商 | P0（已完成代码，待生产化） | 文档明确生产必改 `MOX_AUDIT_HMAC_SECRET` |
| **注册中心独立** | ✅ registry-svc:3400 + proto 契约层 + 10:1:1 聚合 | — | ✅ 已完成 | — |
| **调度器多活** | ✅ 租约选主+fencing+leader 对账（需 HA_MODE=on） | 跨机仲裁后端（换 PG/etcd LeaseStore） | P1 | 保持现状，文档标注开关 |
| **存储** | 🟡 嵌入式 SQLite WAL（生产应设 STORAGE_MODE=sqlite）；favorites/plans/history 进程内 | PostgreSQL + Redis + pgvector；进程内三项落盘 | P1 | ①生产默认 sqlite；②补 favorites/plans/history 落盘；③引入 schema 版本迁移（N3） |
| **下游鉴权** | 🔴 三 svc 无 JWT | 统一 JWT/签名校验 | P0 | 高优先补（N1） |
| **审计用户身份** | 🔴 Actor 硬编码 system | 从 ApiAuth 注入真实用户 | P1 | 补（N2） |
| **前端 API 收敛** | 🔴 两套客户端并行 | 收敛到模块 allianceApi，删除 legacy | P1 | 清理 G1/G2/G3/G4 |
| **前端权限** | 🟡 控制台路由无角色校验 | requiresRole + 按钮级 v-permission | P1 | 补（G3） |
| **图谱节点级 CRUD** | 🔴 只能全量 rebuild | 增量维护 | P2 | 补（N4） |
| **DAG 并行度** | 🟡 无信号量上限 | Semaphore 限流 | P2 | 补（N8） |
| **/metrics 格式统一** | 🟡 scheduler/executor 是 JSON | Prometheus 文本 | P2 | 补（N7） |
| **服务间通信** | 🟡 HTTP 短调用 | gRPC :50051 | P3 | 保持现状 |
| **fusion 独立** | 🟡 executor-core 适配层 | 独立 fusion-svc | P2 | 保持现状 |
| **memory 独立** | 🟡 网关内联 session/db | 独立 memory-svc | P2 | 保持现状 |
| **协议扩展** | 🟡 REST + SSE | + JSON-RPC + MCP | P3 | 保持现状 |
| **大规模编排** | 🟡 10:1:1 聚合 + 多活 | ShardFanout 分片感知 DAG | P2→P3 | 见 07-massive-scale 方案 |

---

## 十一、故障排查与常见误读

> 收录 CURRENT V1.1 已踩坑 + 本次核验新发现。

| 现象 | 根因 | 排查动作 |
|------|------|---------|
| **重启后任务全没了** | 默认 `MOX_ALLIANCE_REMOTE_MODE=auto` + `STORAGE_MODE=file`，任务状态在进程内/JSON 快照，无恢复 | ①查 env REMOTE_MODE 是否 off；②查 STORAGE_MODE 是否 sqlite；③生产必设 sqlite |
| **编排历史空了** | orchestration_history 是进程内 Vec（experts_common.rs:470），重启即失；空结果只证明"本进程没有"，不证明"从未发生" | 界面文案已合规（"仅本次进程"）；如需持久化走 P1 落盘 |
| **收藏全没了** | favorites 是进程内 HashSet（:472） | 同上 |
| **plan_id 404** | plans 是进程内 HashMap（:468），网关重启后旧 plan_id 失效 | orch store 已注释提示；重新生成 plan |
| **专家显示"在线"但实际不通** | `availability.status` 是注册时写入的登记值，不是探活结果；registry 主动探活默认关闭（N5） | ①不要把登记值当健康检测；②生产开 REGISTRY_PROBE_ENABLED；③真正健康判断看 scheduler matcher 的 health_score（0.15 加权） |
| **直连 :3100 创建任务成功，绕过了登录** | 下游 svc 无 JWT 中间件（N1） | 生产环境网络隔离，不暴露 :3100/:3200/:3400 到公网；代码层补 JWT |
| **Prometheus 抓 scheduler /metrics 解析失败** | 返回的是 JSON 快照，非 Prometheus 文本（N7） | 写 JSON→Prometheus 适配器，或补文本格式端点 |
| **前端工作台调 /ai/engine/alliance/full 404** | 该端点仅编排器 :3001 提供，网关 :3080 不保证（G1） | 迁移到模块 allianceApi 的 /api/alliance/* 族 |
| **/ws/v1 连不上** | 代码里根本没有 WebSocket（全 crate WebSocketUpgrade 零命中）；06 §4 是 V1.0 幻影 | 实时性走 SSE `GET /api/alliance/tasks/:id/logs/stream` |
| **HA 开启后启动失败** | `HA_MODE=on` 要求 `STORAGE_MODE=sqlite`（server.rs:272-276 anyhow::ensure） | 检查 env 是否同时设了 sqlite |

---

## 十二、附录：核验报告索引

| 报告 | 路径 | 核心结论 |
|------|------|---------|
| 后端代码事实核验 | `platform/domains/alliance/_verification/backend-verification-report.md` | 30 项摘要核验（28 ✅ / 1 ❌ / 1 ⚠️）；§8/§9 增量全部属实；11 个新缺口（N1-N11） |
| 前端代码事实核验 | `frontend-ui/src/modules/expert-alliance/_verification/frontend-verification-report.md` | 模块契约纪律极强（0 假端点、11 孤儿有意拒绝、3 文案红线未踩）；缺口全在 legacy 层（G1-G10） |
| 文档体系一致性核验 | `docs/expert-alliance/_verification/docs-verification-report.md` | 01-07 三类硬冲突（WS 幻影/plans 表幻影/枚举三套）；端口漏登 3400/3300/3210；baseline.sql 是 MySQL 目标非 PG 现状 |

---

*本文档为 2026-09-27 三方代码事实核验后的归一化产物。所有"已实现"结论均可在上述三份核验报告中找到 文件:行号 证据。与 CURRENT-ARCHITECTURE.md V1.1 冲突时以 CURRENT 为准。*

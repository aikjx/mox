# 开发专家联盟（MOX Expert Alliance）可开发设计文档

> **版本**：v1.0
> **日期**：2026-09-02
> **状态**：待评审 → 按 M0–M4 里程碑开发
> **前置依据**：
> - 代码库现状梳理（2026-09-02，见对话）、《api-backend-gap-audit.md》、《EXPERT_SVC_VERIFICATION_REPORT.md》
> - 生产引擎 `platform/domains/ai/svc/mox-ai-expert-svc`（v3.0.0-ai-powered）
> - 前端契约 `frontend-ui/src/api/experts.api.js`（52 个导出函数）

---

## 1. 文档定位与范围

本设计文档把"开发专家联盟"从**现状（前端契约 + 生产引擎代码存在、但未装配运行）**推进到**可上线运行的正式服务**。范围限定在"专家联盟"本身，不扩张到平台其它域。

设计原则（沿用仓库既有约定）：

1. **契约优先**：以 `experts.api.js` 52 个前端函数为唯一 API 契约来源，后端端点与之逐一对应。
2. **生产引擎复用**：不重写 `mox-ai-expert-svc` 已有引擎（6 阶段管线/评分/门禁/KG 连接/LLM 路由/RBAC/审计），而是**桥接 + 补齐 + 修复**。
3. **可验证**：每个里程碑有可运行的验收命令与回读断言。

---

## 2. 现状基线（证据）

### 2.1 生产引擎：`mox-ai-expert-svc`（代码存在，未运行）

| 项 | 证据 | 状态 |
|---|---|---|
| 版本 | `Cargo.toml` version.workspace，报告 v3.0.0-ai-powered | confirmed |
| 规模 | `src/` 39 文件；`tests/` 11 集成测试；约 151 单测 + 68 集成测试（报告口径） | confirmed |
| 核心管线 | `src/alliance/mod.rs`：Intent→Team→Debate→Synthesize→Gate→Learn→Done，SSE 事件流（`AllianceEvent{phase,trace_id,latency_ms,degraded}`） | confirmed |
| 专家域 | `src/experts/`：algorithm/architecture/business/code_quality/data/documentation/maintainability/observability/performance/permission/resource/security/security_code/testing（15 类） | confirmed |
| 治理 | `src/govern.rs`、`src/alliance/gate.rs`（治理闸门 approved/vetoed）、`src/rbac/`（policy/check） | confirmed |
| KG 集成 | `src/alliance/kg_connector/`：traits/adapter/http/sdk/mock 四路实现 | confirmed |
| LLM | `src/llm/`：chat/consultant/react/router/tools | confirmed |
| 审计 | `src/audit/`：event/sink/s3/syslog/integration | confirmed |
| 验证 | `src/verify/`：cem/code_rt/conflict/data_dep/gains/topology/tests | confirmed |
| 自身 HTTP | `src/bin/mox.rs`：`mox serve [--port 8080]` 启动 axum（`/api/health`、`/api/optimize`、`/api/alliance/*`） | confirmed |

### 2.2 生产引擎已暴露的 HTTP 端点（`src/server.rs`）

```
GET  /api/health
POST /api/optimize | /api/ingest | /api/run | /api/alliance/{route,consult,multi-consult,debate,full,orchestrate,algorithm-analysis}
GET  /api/live | /api/trace | /api/closedloop | /api/alliance/debate/stream | /api/alliance/full/stream
GET  /api/alliance/experts | /api/alliance/experts/:id | /api/alliance/overview | /api/alliance/metrics
POST /api/alliance/experts/register
```

### 2.3 前端契约（`experts.api.js`，52 函数）

按能力分组：

| 分组 | 前端函数（示例） | 目标路径 |
|---|---|---|
| 专家 CRUD | getExperts/getExpert/registerExpert/updateExpert/removeExpert | `/experts`、`/experts/:id` |
| 咨询 | consultExpert/multiExpertConsult/expertDebate/intelligentConsult | `/experts/:id/consult`、`/experts/multi-consult`、`/experts/debate`、`/experts/intelligent-consult` |
| 路由/能力 | routeExperts/getExpertCapabilities | `/experts/route`、`/experts/capabilities` |
| 分析 | algorithmAnalysis | `/experts/algorithm-analysis` |
| 指标 | getExpertMetrics/getExpertOverview/getSingleExpertMetrics | `/experts/metrics`、`/experts/overview`、`/experts/:id/metrics` |
| 会话 | create/get/list/stats/update/delete/appendMessage/similarSearch/semanticSearch/export/archive | `/experts/sessions/*` |
| 调度器 | get/updateDispatcherConfig、dispatcherDispatch/Consult/MultiConsult、reset | `/experts/dispatcher/*` |
| 专家图谱 | getExpertGraph/Stats/Neighbors/Collaborators/Path/Communities/optimalTeam/rebuild | `/expert-graph/*` |
| 企业级 | enterpriseConsult/enterpriseAnalyze | `/experts/enterprise/*` |
| 编排 | expertOrchestrate/expertGeneratePlan/expertExecutePlan/Stats/Plugins/History | `/experts/orchestrate`、`/experts/plan/*`、`/experts/orchestration/*` |

### 2.4 运行层现状（关键矛盾）

- 前端经 Vite（3020）→ `/api` → **legacy 网关（8080）**；其 `/experts/*`、`/expert-graph/*` 约 50 路由由 `handlers.rs` 用**内存演示数据**返回（`experts_list` 读 `state.experts`，仅 3 个演示专家）。
- **生产引擎 `mox-ai-expert-svc` 未运行**；且其默认端口 8080 与 legacy 网关**冲突**，路径前缀 `/api/alliance/*` 与前端 `/experts/*` **不一致**。
- 结论：**前端连的是"内存桩"，生产引擎"代码已就绪但无人接线"。**

### 2.5 已知问题清单（开发第一步的修复对象）

| # | 问题 | 状态（2026-09-02 核实） |
|---|---|---|
| B1 | `ExpertWorkspaceView.vue` `kbSearch` 重复声明致前端构建失败 | **已修复**（现用 `kbSearchQuery`，line 3647；API 导入 `kbSearch` line 1942） |
| B2 | `algorithm.rs`/`orchestration.rs` 测试模块 `BTreeMap::new()` 与 `HashMap` 字段不匹配 | **待运行 `cargo test` 确认**（报告行号 694/416 已位移，需实测） |
| B3 | `platform/clippy.toml` 非法字段 `allow/deny/warn` 致 clippy 门禁不可用 | 未处理 |
| B4 | `src/alliance/gate.rs:320` `never_loop`（clippy deny） | 未处理 |
| B5 | 端口 8080 冲突：expert-svc 需独立端口 + 网关桥接 | 本文档设计 |

---

## 3. 目标架构

### 3.1 服务拓扑与端口规划

```text
Vite(3020) ──/api──▶ legacy 网关(8080)
                            │ 1) /experts/*、/expert-graph/* 改为反代
                            ▼
                     mox-ai-expert-svc(:3002)  ←── 生产引擎（本次接入）
                            ├─▶ mox-kg-* 服务（图谱，可选：先降级走 kg_connector mock/sdk）
                            ├─▶ LLM 路由（llm/router，接真实 provider 或 mock）
                            └─▶ 审计 sink（audit/s3、audit/syslog）
```

端口规划：

| 服务 | 端口 | 说明 |
|---|---|---|
| Vite dev | 3020 | 现有 |
| legacy 网关 | 8080 | 现有，仅保留**桥接 + 非专家域**路由 |
| **mox-ai-expert-svc** | **3002** | 本次启动 `mox serve --port 3002` |
| PrimiFlow | 8000 | 现有 |

### 3.2 请求主链路（以 `/experts/multi-consult` 为例）

```text
ExpertPlazaView.vue ──POST /api/experts/multi-consult──▶ legacy 网关(8080)
    ▶ 反代 ──POST /api/alliance/multi-consult──▶ expert-svc(:3002)
        ▶ AllianceService.multi_expert_consult() ──▶ (Debate 阶段, kg_connector + llm/router)
    ◀── SSE/JSON ── 网关透传 ── 前端渲染
```

### 3.3 路径前缀桥接（本设计的关键决策）

**方案 A（推荐，网关反代 + 白名单映射）**：legacy 网关为 `/experts/*` 与 `/expert-graph/*` 增加**反代路由**，按表映射到 expert-svc `/api/alliance/*`，保留现状前端路径零改动。

| 前端路径 | expert-svc 目标 |
|---|---|
| `/experts` (GET/POST) | `/api/alliance/experts` (list/register) |
| `/experts/:id` (GET/PUT/DELETE) | `/api/alliance/experts/:id` |
| `/experts/:id/consult` | `/api/alliance/consult`（body 带 expert_id） |
| `/experts/multi-consult` | `/api/alliance/multi-consult` |
| `/experts/debate` | `/api/alliance/debate` |
| `/experts/route` | `/api/alliance/route` |
| `/experts/algorithm-analysis` | `/api/alliance/algorithm-analysis` |
| `/experts/orchestrate` | `/api/alliance/orchestrate` |
| `/experts/overview` | `/api/alliance/overview` |
| `/experts/metrics` | `/api/alliance/metrics` |
| `/expert-graph/*` | 由 expert-svc 新增一组 `/api/alliance/graph/*` 路由（见 §5.5） |
| `/experts/sessions/*`、`/experts/dispatcher/*`、`/experts/plan/*`、`/experts/orchestration/*` | expert-svc 新增对应路由（见 §5.5） |

**方案 B（前端改路径）**：`experts.api.js` 的 basePath 改为 `/api/alliance`。改动大、影响面广，不推荐（违背"零前端改动"）。

> 采用方案 A：**前端零改动**，桥接逻辑收敛在网关 + expert-svc 新增路由两个点。

---

## 4. 领域模型

> 以 `src/types.rs` 既有类型为准，本节约定持久化结构（与内存结构一致，便于无缝替换）。

### 4.1 Expert

```jsonc
{
  "id": "exp-arch",
  "name": "架构专家",
  "role": "architect",            // architect | algorithm | security | data | ...
  "domain": ["system", "architecture"],
  "capabilities": ["系统设计", "技术选型", "架构评审"],
  "status": "online",             // online | busy | offline
  "quota": { "max_concurrent": 2, "used": 0 },   // ResourceQuota
  "score_history": {},            // 按 query_type 的评分历史（Learn 阶段写入）
  "created_at": "2026-09-02T00:00:00Z"
}
```

### 4.2 Session / Message

```jsonc
{
  "id": "sess-xxx", "expert_id": "exp-arch", "project_id": "proj-xxx",
  "status": "active",              // active | archived
  "messages": [{ "role": "user|assistant", "content": "", "ts": "" }],
  "similar_key": ""                // 用于 similar-search / 归档检索
}
```

### 4.3 Dispatch（调度器）

```jsonc
{
  "id": "disp-xxx", "query": "", "intent": "",
  "candidates": ["exp-arch", "exp-algo"], "selected": "exp-arch",
  "strategy": "capability_match|score_weighted|round_robin",
  "status": "routed|consulted|failed"
}
```

### 4.4 Team / Orchestration

```jsonc
{
  "id": "team-xxx", "goal": "", "members": ["exp-arch","exp-security"],
  "plan": { "phases": [{ "phase": "intent", "expert_id": "", "output": "" }] },
  "status": "planned|executing|done"
}
```

### 4.5 ExpertGraph（专家协作图谱，复用 `mox-kg-algo-core` 语义）

```jsonc
{ "nodes": [{ "id": "exp-arch", "label": "架构专家", "node_type": "expert",
              "properties": { "role": "architect", "domain": ["system"] } }],
  "edges": [{ "source": "exp-arch", "target": "exp-security",
              "relation_type": "collaborates", "weight": 0.8 }] }
```

---

## 5. API 契约映射（52 前端函数 → 实现点）

> 表内"实现点"分为：`已有`（expert-svc 已暴露）、`新增`（需在 expert-svc `server.rs` 添加路由 + `AllianceService` 补方法）、`桥接`（仅 legacy 网关反代）。

| 前端函数 | 路径 | 实现点 | 责任人 |
|---|---|---|---|
| getExperts / registerExpert / getExpert / updateExpert / removeExpert | `/experts`、`/experts/:id` | 桥接→已有 | 网关 |
| consultExpert / multiExpertConsult / expertDebate / intelligentConsult | `/experts/:id/consult`、`/experts/multi-consult`、`/experts/debate`、`/experts/intelligent-consult` | 桥接→已有（consult/multi/debate）；intelligent 新增 | 网关+svc |
| routeExperts / getExpertCapabilities | `/experts/route`、`/experts/capabilities` | 桥接→已有（route）；capabilities 新增 | 网关+svc |
| algorithmAnalysis | `/experts/algorithm-analysis` | 桥接→已有 | 网关 |
| getExpertMetrics / getExpertOverview / getSingleExpertMetrics | `/experts/metrics`、`/experts/overview`、`/experts/:id/metrics` | 桥接→已有（overview/metrics）；:id/metrics 新增 | 网关+svc |
| getExpertSessions 等 11 个会话函数 | `/experts/sessions*` | **新增**（svc 无会话端点） | svc |
| getDispatcherConfig 等 8 个调度函数 | `/experts/dispatcher/*` | **新增** | svc |
| getExpertGraph 等 8 个图谱函数 | `/expert-graph/*` | **新增** `/api/alliance/graph/*` | svc |
| enterpriseConsult / enterpriseAnalyze | `/experts/enterprise/*` | **新增**（可映射到 orchestrate + team） | svc |
| expertOrchestrate 等 7 个编排函数 | `/experts/orchestrate`、`/experts/plan/*`、`/experts/orchestration/*` | 桥接→已有（orchestrate）；plan/history 新增 | 网关+svc |

**新增接口清单（svc 侧，M2）**：sessions CRUD + messages + similar-search + semantic-search + export + archive；dispatcher config/status/dispatch/consult/multi/reset；graph stats/neighbors/collaborators/path/communities/optimal-team/rebuild；enterprise consult/analyze；plan generate/execute；orchestration stats/plugins/history。

---

## 6. 核心管线与算法（复用，不重写）

### 6.1 6 阶段全维分析管线（`src/alliance/mod.rs`，SSE）

```text
Intent(意图识别) → Team(专家组队) → Debate(并行咨询+辩论)
→ Synthesize(归一合成) → Gate(质量门禁) → Learn(指标学习) → Done
```

- 事件契约：`AllianceEvent{ phase, payload, trace_id, latency_ms, ts, degraded?, degrade_reason? }`
- **降级语义（必须保留）**：graph 或 ai-agent 不可用时置 `degraded=true` + 原因，**不阻断**主流程（`kg_connector` 已有 mock 实现）。

### 6.2 专家评分与治理闸门（`src/alliance/gate.rs`、`src/experts/*`）

- 15 类专家域对请求分别打分 → `expert_scores`；
- 治理闸门输出 `{ status, approved, reason }`，`verify` 子命令在 vetoed 时退出码 2（最高权限否决语义，保留）。

### 6.3 最优组队（`/expert-graph/optimal-team`）

- 基于专家协作图谱 + `mox-kg-algo-core` 社区/路径算法，按目标能力组合选取最小成本团队；
- 输入 `{ goal, required_capabilities[], constraints{} }`，输出 `{ team[], score, rationale }`。

### 6.4 调度器（`/experts/dispatcher/*`）

- 策略：`capability_match`（默认）→ `score_weighted` → `round_robin`；可配置、可重置（reset/reset-all）。

### 6.5 专家图谱（`/expert-graph/*`）

- 节点：expert；边：`collaborates`（按历史共现/评分相关性）；`rebuild` 从会话与评分历史重算。

---

## 7. 集成设计

| 依赖 | 方式 | 降级 |
|---|---|---|
| KG | `kg_connector` traits + sdk/http（接 mox-kg-* 或 legacy 网关 `/api/graph`） | mock 实现（已存在） |
| LLM | `llm/router`（provider 路由） | consultant mock / react 回退 |
| 审计 | `audit/event` → sink（s3/syslog） | 落内存缓冲 |
| RBAC/租户 | `GovernContext{ Tenant, Principal }` + `rbac/policy` | 默认 admin/editor |
| 项目上下文 | 请求透传 `project_id`（Vite 已注入） | 无则全局 |

---

## 8. 数据持久化方案

现状：expert-svc 无持久化（内存）；legacy 网关为内存演示。

**M3 持久化（推荐 SQLite，复用仓库 `rusqlite`/既有 db 文件模式）**：

- 表：`experts`、`sessions`、`session_messages`、`dispatches`、`teams`、`expert_graph_edges`、`expert_scores`；
- 位置：`data/expert-alliance.db`（对齐 `operator_data.db` 同层）；
- 迁移：启动时 `CREATE TABLE IF NOT EXISTS` + 幂等 seed（把 legacy 演示专家并入）。

> 先内存 + 幂等 seed 跑通（M1），SQLite 落地（M3）；KG 持久化依赖平台级决策，不在本设计范围内。

---

## 9. 阻断问题修复清单（M0，先修后跑）

| 编号 | 动作 | 验证 |
|---|---|---|
| M0-1 | 运行 `cargo test -p mox-ai-expert-svc --lib`，若仍报 BTreeMap/HashMap 错误则改 `#[cfg(test)]` 初始化 | `cargo test -p mox-ai-expert-svc --lib` 全绿 |
| M0-2 | 修 `platform/clippy.toml`：`allow/deny/warn` 迁入 `[workspace.lints.clippy]` | `cargo clippy -p mox-ai-expert-svc` 0 error |
| M0-3 | 修 `gate.rs:320` `never_loop`（改普通代码块或命名循环+条件 break） | clippy 无该 lint |
| M0-4 | `cargo check -p mox-ai-expert-svc --lib` 0 警告级错误 | 编译通过 |

---

## 10. 开发里程碑

| 里程碑 | 内容 | 出口（可验证） |
|---|---|---|
| **M0** | 修复阻断问题，`mox serve --port 3002` 可启动 | `GET :3002/api/health` 200；cargo test 全绿 |
| **M1** | 网关桥接：`/experts/*`、`/expert-graph/*` 反代 → :3002 | 前端专家工作台真实调用返回非空（不再是演示 3 专家） |
| **M2** | svc 新增路由：sessions / dispatcher / graph / enterprise / plan / orchestration | 52 个前端函数逐一对通（回归清单） |
| **M3** | SQLite 持久化 + 幂等 seed | 重启后数据保留；重复 seed 0 新增 |
| **M4** | 专家图谱 rebuild + 最优组队 + 评分学习闭环；对接真实 LLM 路由 | `/expert-graph/optimal-team` 输出可解释团队 |

---

## 11. 验收标准

1. 前端 52 个 `experts.api.js` 函数调用全部返回 `{success:true}` 且数据非演示占位。
2. `GET /experts/overview`、`/experts/metrics` 返回真实注册专家集合。
3. `POST /experts/multi-consult`、`/experts/debate` 走 6 阶段管线，返回阶段事件/评分/门禁结果。
4. `POST /expert-graph/optimal-team` 返回可解释组队结果。
5. 服务重启后 sessions / experts 数据保留（M3 后）。
6. `cargo test -p mox-ai-expert-svc` 全绿、`cargo clippy` 0 error、前端 `npm run build` 通过。

---

## 12. 风险与未决项

| 项 | 说明 | 建议 |
|---|---|---|
| LLM 真实性 | `llm/router` 是否接真实 provider 未验证 | M4 前先 mock，验证管线正确性后再接入 |
| 网关反代实现 | legacy 网关需新增反代 handler（axum `reqwest` forward） | M1 实现，保留失败时返回 502 且前端可感知 |
| 与 mox-kg-* 对接 | expert-svc 的 kg_connector 目标服务未运行 | 先走 mock/legacy `/api/graph`，M4 再决策 |
| 会话/调度持久化范围 | 是否需要多租户隔离 | 本期单租户，预留 `tenant_id` 字段 |

---

---

## 9A. M0 执行记录（2026-09-02，已完成）

| 里程碑项 | 结果（实测） |
|---|---|
| M0-1 测试编译 | `cargo test -p mox-ai-expert-svc --lib` → **186 passed / 0 failed / 1 ignored**（B2 BTreeMap/HashMap 编译错误已不存在，前人已修复） |
| M0-2 clippy 配置 | 已修复：`platform/clippy.toml` 非法字段已移除，lint 级别在根 `Cargo.toml` `[workspace.lints.clippy]`（L159+，allow 5 项 + deny 5 项） |
| M0-3 gate.rs never_loop | 已不存在（`gate.rs` 中 `loop` 仅出现在注释 L347，无实际循环） |
| M0-4 编译与 clippy | `cargo clippy -p mox-ai-expert-svc --lib` → **0 error / 0 warning**；`cargo build -p mox-ai-expert-svc --bin mox` → 成功（v3.0.0-ai-powered，32.70s） |
| 出口：服务启动 | `target\debug\mox.exe serve --port 3002` 已启动（PID 35624） |
| 出口：健康检查 | `GET http://localhost:3002/api/health` → **200 `ok`** |
| 出口：联盟端点 | `GET /api/alliance/experts` → 200，**14 个专家**；`GET /api/alliance/overview` → 200（14 专家 / 70 能力 / 14 维度） |

> 注：M0 期间两次出现 `os error 5`（增量编译文件锁）与 clippy exit 1 抖动，均为 Windows 并行构建文件锁所致，重跑即绿，非代码问题。

**下一步（M1）**：legacy 网关（8080）为 `/experts/*`、`/expert-graph/*` 增加反代 → `:3002`；涉及重启网关（清空内存 KG）→ 需幂等重灌 FR-KG。

---

## 9B. M1 执行记录（2026-09-02，已完成）

**目标**：legacy 网关（8080）为 `/experts/*` 反代 → `mox-ai-expert-svc`（:3002），前端专家工作台接入真实引擎。

**改动（已落盘并编译）**：

| 文件 | 改动 |
|---|---|
| `platform/legacy/backend-rust/src/api/mod.rs` | ① import 补 `extract::{Path, State}`；② 9 条路由 handler 替换为反代（experts 列表/详情/overview/metrics/multi-consult/debate/route/algorithm-analysis/orchestrate）；③ 新增通用 `proxy_forward()` + 9 个反代 handler，把 expert-svc 裸响应适配为 legacy 同款 `{success:true,data}` 信封（`/experts` 列表提取 `experts` 数组为 data） |
| `platform/legacy/backend-rust/src/api/handlers.rs` | 补齐上一会话超时未落地的 3 个 system 处理器：`system_permissions`（返回 roles/permissions/deptId/dataScope/customDeptIds/menus，roles 含 admin）、`system_dept_tree`、`system_menu_tree`（修复编译 error） |
| 未改前端 | `experts.api.js` 52 函数路径零改动（方案 A 网关反代） |

**阻断问题修复**：编译期发现 mod.rs 已引用 3 个 system 处理器但 handlers.rs 缺失（上会话 RPC 超时遗留），本次补齐后编译通过。

**实测结果（全链路）**：

| 端点 | 结果 |
|---|---|
| `GET /api/health` | 200 `{"status":"healthy","version":"3.0.0"}` |
| `GET /api/graph`、`/api/graph/stats` | 200，KG 重灌后 nodes=701 / edges=792（幂等，0 失败） |
| `GET /api/system/permissions` | 200，`{success:true,data:{roles:["admin","developer","viewer"],...}}` |
| `GET /api/experts` | **200，14 个真实专家**（信封 data 数组，原为 3 个演示专家） |
| `GET /api/experts/overview`、`/metrics` | 200，14 专家 / 70 能力 / avg_gate_score 0.82 |
| `GET /api/experts/:id` | 200，architecture 专家详情 |
| `POST /api/experts/multi-consult` | 200，consensus 1.0 / overall_vetoed false |
| `POST /api/experts/debate` | 200，gate_grade C / gate_total 0.76 / 观点数组 |
| `POST /api/experts/route` / `algorithm-analysis` / `orchestrate` | 200，真实匹配 / 分析 / 编排报告 |

**当前运行服务**：legacy 网关 8080（PID 19228，含反代）、mox serve 3002（PID 24712）、Vite 3020、PrimiFlow 8000。

> 说明：反代依赖 `:3002` 存活。mox serve 以持久后台任务运行；若被回收需重启（`target\debug\mox.exe serve --port 3002`）。`/experts/:id/consult`、`/experts`(POST register)、sessions/dispatcher/expert-graph/plan/enterprise 仍走内存桩，留待 M2 补齐。

**下一步（M2）**：expert-svc 新增 sessions / dispatcher / graph / enterprise / plan / orchestration 路由，使 52 个前端函数全数对通。

---


## 9C. M2 执行记录（2026-09-02，已完成：52 函数全对通）

**目标**：使前端 `experts.api.js` 52 个导出路径全部对通真实生产引擎 `mox-ai-expert-svc`（经 legacy 网关 8080 反代 → :3002）。

### M2-1 expert-svc 扩展模块（`src/alliance_ext.rs`，33.8KB）
- 新增 `AllianceExtState { alliance: Arc<AllianceService>, sessions/dispatcher_cfg/plans: Mutex<HashMap<..>>, orch_history: Mutex<Vec<..>> }`（无 dashmap，用 tokio Mutex + HashMap）。
- 约 30 个方法：sessions（create/list/stats/get/update/delete/append/similar-search/export/archive）、dispatcher（config/status/dispatch/consult/multi-consult/reset-expert/reset-all）、graph（build/get/stats/neighbors/collaborators/BFS path/communities/optimal-team/rebuild，专家图谱实时由注册表构建，共享能力→边）、enterprise（consult/analyze）、plan（generate/execute）、capabilities、intelligent-consult、expert 更新/删除/单指标/按 ID 咨询、orchestration（stats/plugins/history）。

### M2-2 接入（lib.rs / server.rs / bin/mox.rs）
- `lib.rs` 注册 `pub mod alliance_ext;`；`server.rs` `AppState` 增加 `ext: Arc<AllianceExtState>`（与 alliance 共享同一 `AllianceService` 实例）；新增 40 条路由（前缀 `/api/alliance/`）+ 40 个薄 handler（axum `State<Arc<AppState>>` + `(StatusCode, Json)` 模式）；`bin/mox.rs` cmd_serve 同步构造 `ext`。
- 编译通过（cargo check 0 error / 0 warning；build dev 20.8s）。
- 编译错误修复 3 轮：`RouteMatch.expert_id`→`expert.id`（字段实为 `expert: ExpertMeta`）；`Vec<Value>` 无 Ord 改 `sort_by(as_str)`；`orchestrate(&req)`→按值传参；`drain(0..len)` 借用冲突拆两语句；`unwrap_or(&mut Vec::new())` 临时值借用。

### M2-3 扩展端点直连验证（:3002，40/40 OK）
sessions / dispatcher / graph / capabilities / intelligent-consult / experts 增改删·单指标 / plan / orchestration / enterprise 全部 200 返回真实数据。

### M2-4 legacy 网关反代补齐（mod.rs）
- 新增 `proxy_query_string` + 42 个 proxy 函数；41 条路由由内存桩切换为反代（含 Query 透传：sessions list / collaborators）。
- 至此 `/experts*`、`/expert-graph*` 共 50 组路由全部反代 → `/api/alliance/*`（M1 9 条 + M2 41 条）。
- 修复：`proxy_dispatcher_config_update` 初版误用 POST 转发（405），改为 PUT 后通过。

### M2-5 全链路回归（:8080，50/50 OK）
按 `experts.api.js` 52 个导出逐项打真实网关：列表/详情/注册/更新/删除/单专家咨询/多专家/辩论/能力/路由/智能咨询/算法分析/总指标/overview/会话全流程/语义搜索/调度全流程/专家图谱全流程/企业咨询/编排/计划/编排统计·插件·历史——全部 200 + `{success:true}` 信封（含 2 个别名导出）。

### M2 已知局限（非阻断）
- `plan/execute` 返回 `{error:"计划不存在", status:"failed"}`：`plan_generate` 当前不持久化计划，execute 查不到（设计行为，留待 M3 计划持久化）。
- 专家图谱 `edges=0`：14 个内置专家能力无交集，`rebuild` 无共享能力边（M3 可加"同维度/同能力"建边策略）。
- 反代依赖 `:3002` 存活；mox serve 以持久后台任务运行，被回收需重启。

### 当前运行服务
legacy 网关 8080（反代全量）、mox serve 3002（M2 扩展）、Vite 3020、PrimiFlow 8000。KG 重灌后 nodes=701 / edges=792 / components=1。

**下一步（M3，未决）**：需求状态枚举（开发/待开发/计划/已弃用）+ KG 持久化 + plan 持久化（SQLite），待用户拍板。


## 9D. M3 执行记录（2026-09-02，已完成：SQLite 持久化 + 幂等 seed，重启数据保留）

**目标**：expert-svc 引入 SQLite 持久化，服务重启后专家注册表 / 会话 / 计划 / 调度配置保留；重复 seed 0 新增。

### M3-1 持久化层（`src/persistence.rs`，新增）
- `PersistenceDb`（rusqlite 0.31 bundled，WAL 模式，`Mutex<Connection>` 保护）。
- 4 张表：`experts(id, meta_json, updated_at)` / `sessions(id, data_json, ...)` / `plans(id, data_json, ...)` / `kv(key, value_json, ...)`。
- 方法：专家 upsert/delete/load；会话 upsert/delete/load；计划 upsert/load；kv save/load/exists。
- `Cargo.toml` 加 `rusqlite = { workspace = true }`（根 workspace 已有定义）；`lib.rs` 注册模块。

### M3-2 专家注册表持久化（services.rs）
- `RegistryImpl` 增加 `db: Option<Arc<PersistenceDb>>`；`register()` 写通 SQLite（注册/更新专家持久化）。
- `RegistryImpl::new_with_db`：首次（`kv['mox_experts_seeded']` 不存在）把 14 个内置专家幂等写入库并打 seed 标记；之后以库为准加载（用户注册/更新的专家、被下架的内置专家重启后保持一致）。
- `AllianceService::new_with_db(db)`：以持久化注册表构造（`new()` 保持纯内存，供测试）。

### M3-3 会话/计划/配置持久化（alliance_ext.rs）
- `AllianceExtState` 增加 `db` 字段；`new(alliance, db)` 启动时从库加载 sessions / plans / dispatcher_cfg。
- 10 处写通：create/update/delete/archive/append_message/record_consult_session → upsert/delete_session；update_dispatcher_config → save_kv；plan_generate → upsert_plan；plan_execute → 修改后 upsert_plan。
- **附带修复 M2 局限**：`plan_execute` 未传 `plan_id` 时按 `task_id` 匹配计划（前端契约兜底），解决"计划不存在"。

### M3-4 装配（server.rs / bin/mox.rs）
- `AppState::open_db()`：env `MOX_EXPERT_DB` 覆盖，默认 `data/mox-expert-svc.db`（自动建目录），失败回退 `:memory:`。
- `new_state()` 与 `cmd_serve` 均改为 `AllianceService::new_with_db(Some(db))` + `AllianceExtState::new(alliance, db)`。

### M3-5 验证（实测）
- 编译：cargo check 0 error / 0 warning；`cargo test -p mox-ai-expert-svc --lib` → **186 passed / 0 failed**。
- 阶段 1（写）：注册 `exp-persist-001`、创建会话+消息、生成计划 `task-persist-1`、更新调度配置 → 全部 200。
- 阶段 2（重启后读）：
  - 专家 total=15（14 内置 + 1 持久化），幂等 seed 无重复 ✅
  - 会话保留（含 1 条消息）✅
  - `plan_execute(task_id)` → `status=done`（不再"计划不存在"）✅
  - 调度配置 strategy=multi-consult 保留 ✅
- DB 落盘：`data/mox-expert-svc.db`（WAL：-shm/-wal 伴生）。

### 当前运行服务
legacy 网关 8080、mox serve 3002（M3 持久化）、Vite 3020、PrimiFlow 8000。KG nodes=701 / edges=792。

**下一步（M4，未决）**：专家图谱 rebuild + 最优组队 + 评分学习闭环；对接真实 LLM 路由。需求状态枚举（开发/待开发/计划/已弃用）与 KG 持久化仍为独立未决项。


## 9E. M4 执行记录（2026-09-02，已完成：专家图谱 rebuild + 最优组队可解释 + 评分学习闭环）

**目标**：修复专家图谱 edges=0（能力交集稀疏）、optimal_team score=0/无解释、expert_metrics 占位；建立"咨询评分 → 持久化 → 反哺组队"学习闭环。

### M4-1 专家图谱 rebuild（alliance_ext.rs `build_graph`）
- 新增 `dimension_family(dim)`：维度 → 领域族（engineering/security/data/business/resource/persistence/general，共 7 族）。
- 建边双策略：`collaborates`（能力交集，保留）+ **`same_family`**（同领域族协作边，weight 0.5，family 标注）。
- 实测：nodes=15, **edges 0 → 24**, density 0 → 0.229（14 内置专家维度各异的"无交集"问题解决）。

### M4-2 最优组队可解释（`optimal_team`）
- 输入兼容：goal / task / query 三选一，required_capabilities / required 二选一，team_size / top_n 二选一。
- **5 维加权评分**：能力关键词命中(+1) / 必需能力精确(+2) / 领域族角色契合(+1) / 图谱协作度(degree×0.08, 上限 0.5) / 历史评分(avg_rating 0..1×0.3)。
- 输出新增：每个成员 `reason`（入选理由）、`score`；团队级 `covered_capabilities` / `missing_capabilities` / `top_score` / `rationale`（可解释组队逻辑）。
- 实测：带 required → score=4.21、covered=16、missing=[]；空输入 → score>0（degree 支撑），不再 0.0。

### M4-3 评分学习闭环
- 新增 `ExpertMetric{consultations, rating_sum, latency_sum}`（0..1 评分 ×5 → 0..5 平均分）+ `AllianceExtState.metrics`（tokio Mutex<HashMap>）。
- `record_metric(id, score, latency)`：每次咨询累计并持久化 kv `metrics:<id>`；`new()` 启动经 `PersistenceDb::load_kv_prefix("metrics:")` 恢复。
- `expert_metrics` 由占位（4.8/120 写死）改为真实统计；`consult_expert_by_id` 记录每次评分与耗时。
- `optimal_team` 以历史评分加权排序（评分高的专家更易入选）。
- 实测：咨询 algorithm 2 次 → metrics consultations=2, avg_rating=5.0 → **重启后保留**；组队排序反映评分加成。

### M4-4 修复
- **metrics 前缀键 bug**：加载时用完整 kv 键（`metrics:algorithm`）作内存键，与查询用的专家 id（`algorithm`）不匹配导致重启后丢失；修复为 `trim_start_matches("metrics:")` 剥离前缀。
- persistence.rs 新增 `load_kv_prefix`（LIKE 参数化防注入）。

### M4-5 验证（实测）
- 单测：新增 `test_kv_prefix_roundtrip` / `test_kv_prefix_file_reopen`（文件库重开）/ `test_expert_metric_roundtrip`；全量 **189 passed / 0 failed**。
- 直连 :3002：graph stats edges=24；optimal_team 可解释；metrics 真实。
- 重启：metrics consultations=2 / avg_rating=5.0 保留。
- 网关 :8080 全链路：`/api/expert-graph/stats`（15/24）、`/api/expert-graph/optimal-team`（score=4.21 可解释）、`/api/experts/algorithm/metrics`（consultations=2）均 200。

### 边界与未决
- 真实 LLM 路由未接入：当前 consult/评分来自 mock 报告（report.score 驱动），接入真实 provider 需环境变量与密钥配置，列为 M4 之后的可选工程。
- 需求状态枚举（开发/待开发/计划/已弃用）与 KG 持久化仍为独立未决项。


## 9F. M5 执行记录（2026-09-02，已完成：需求功能状态枚举归一化 开发/待开发/计划/已弃用）

**目标**：把需求功能节点状态归一化为用户愿景枚举（开发/待开发/计划/已弃用），前端可按状态过滤/着色查看，并修复"seed 变更无法传播到已存在 KG 节点"的缺口。

### M5-1 状态枚举归一化（seed 数据层）
- 旧枚举（英文细粒度）：implemented / frontend_only / partial / open / pending。
- 统一映射为新枚举 + 中文 `status_label`：
  - implemented / frontend_only / partial → `developing`（开发）
  - open → `todo`（待开发）
  - pending → `planned`（计划）
  - （deprecated 已弃用：当前 0 个，如实不造数）
- 归一化节点：100 个（feature/gap/pending_decision）；实测分布 **developing 75 / todo 19 / planned 6**；其余 601 个非需求节点（domain/module/api_function 等）无 status（设计使然）。

### M5-2 ingest 幂等 upsert（seed 变更可传播）
- **缺口修复**：原 `ingest_nodes` 对已存在节点直接跳过，seed 变更无法更新已有节点（此前重灌"新增=0/跳过=776"即表现）。
- 网关 `POST /graph/node` 为 HashMap `insert`（天然 upsert 覆盖）；脚本补差异检测：已存在节点比对 `properties`，有差异则以 seed 为准覆盖更新。
- 新增 / 更新 / 跳过 三计数；实测重灌 **更新=100 / 跳过=587 / 失败=0**。

### M5-3 GraphView.vue 前端（状态过滤/着色/图例）
- 过滤区新增"需求状态"4 态多选（`statusFilter` Set + `toggleStatus`，重置纳入 `resetFilter`）。
- 过滤逻辑：带状态的需求节点（feature/gap/pending_decision）按勾选状态筛选，无状态节点不过滤。
- 节点着色：feature 节点按状态取 `STATUS_COLORS`（developing 绿 / todo 橙 / planned 黄 / deprecated 灰），`nodeColor` 优先 `n.color`。
- 图例新增"需求状态"色卡；快捷过滤"只看未实现"改用新枚举（todo/planned）。

### M5-4 验证（实测）
- graph API：`/api/graph` 节点 properties 含新 status + status_label（透传正常）。
- ingest upsert：100 节点更新 / 0 失败。
- 前端：Vite HMR 编译 GraphView.vue 200；生产构建 `vite build` 25.44s 成功无错误。

### 边界与未决
- `deprecated` 当前无数据（无废弃节点）；状态枚举在服务端未做枚举校验（前端软约束），后续可在 seed/API 层加校验。
- 其余未决：KG 持久化（网关重启重灌仍为运维动作，ingest upsert 已降低其成本）、真实 LLM 路由接入。


## 9G. M5.3 未决项收尾（2026-09-02，已完成：KG 持久化 / 真实 LLM 路由接入 / 服务端状态枚举校验）

**背景**：M5 状态枚举归一化完成后遗留三项未决，本轮逐项落地并验证。

### 9G-1 KG 图谱持久化（网关重启自动恢复，无需重灌）
- **改动**：
  - 网关新增 `api/kg_persist.rs`：`load_snapshot/save_snapshot`（JSON 快照 `{nodes,edges}`，自动建目录）、`load_seed`（从功能需求 seed JSON 灌入，edge 生成稳定 id `edge-{src}-{tgt}-{rel}`，幂等）。
  - `AppState` 新增 `kg_file: Option<String>`（env `MOX_KG_FILE`）；`load_kg_from_env()` 装配优先级：**快照 > seed（并落快照）> 演示数据**；启动日志打印 KG 来源。
  - `graph_add_node/graph_add_edge` 写库后同步写快照（运行时变更也持久化）。
- **验证（实测）**：
  - 冷启动：`[KG] 从 seed 冷启动` → 701 节点 / 792 边 + 落快照 `data/mox-kg-graph.json`（~438KB）。
  - 运行时变更：POST 新增节点 → 写快照。
  - **重启网关**：日志 `[KG] 从快照恢复: 702 节点 / 792 边`；新增节点重启后保留；状态分布 开发76/待开发19/计划6 不变。**彻底去掉"重启重灌"运维动作**。

### 9G-2 真实 LLM 路由接入（配置驱动，已实测 deepseek-chat）
- **改动**：`services.rs` `AllianceService::new_with_db` 的 consultant 由硬编码本地引擎改为 `crate::llm::llm_consultant_from_env()`：
  - 配置了 `MOX_LLM_*` / `DEEPSEEK_API_KEY` / `OPENAI_API_KEY` → 启用真实 LLM 咨询器（OpenAI 兼容 + 多 Provider 路由熔断 + 失败自动回退本地规则引擎，此能力 chat.rs/router.rs 已具备，本轮完成接入）。
  - 未配置 → 打印提示并回退本地规则引擎（行为不变）。
- **验证（实测）**：环境存在 `DEEPSEEK_API_KEY` → 启动日志 `[M5] 专家联盟: 已启用真实 LLM 咨询器`；consult 返回 `steps: ["[ReAct] 真实 LLM(deepseek-chat) 专家推理", ...]`，report 为 deepseek 真实生成（如快速排序 O(n log n) 专业分析，score 0.95）。**配置即启用、无配置即回退，零侵入**。

### 9G-3 服务端状态枚举校验（前端软约束 -> 服务端强约束）
- **改动**：网关 `graph_add_node` 增加校验：`properties.status` 若存在必须属于枚举 `{developing(开发)/todo(待开发)/planned(计划)/deprecated(已弃用)}`，否则返回 400 `invalid_status` + 中文错误；无 status 节点（domain/module/api_function 等）不受影响。
- **验证（实测）**：POST `status=invalid_enum` → 400 `非法需求状态...合法枚举: developing(开发)/todo(待开发)/planned(计划)/deprecated(已弃用)`；POST `status=developing` → 200。非法枚举无法再进入 KG。

### 验证与回归
- 编译：backend-rust release（mox-gateway）成功；mox-ai-expert-svc debug（mox）成功。
- 反代回归：`/api/experts`、`/api/expert-graph/stats`（15/24）、`/api/expert-graph/optimal-team`（covered_capabilities 返回）经 8080 网关全通。
- 服务：mox serve 3002 + 网关 8080 常驻运行；Vite 3020 在线。

### 边界与说明
- 真实 LLM 依赖外部 provider 可用性：deepseek 不可达/熔断时自动回退本地引擎（M4 边界保留）。
- KG 快照为单文件 JSON：多网关实例并发写同一文件有覆盖风险（当前单实例场景无影响）；后续可升级 SQLite。
- 状态枚举校验为网关层强约束；expert-svc 侧不受影响。

## 9H 企业级测试验证与修复优化（M5.4）

> 对前后端所有需求、功能、代码、数据库开展企业级验证：分层测试 → 发现缺陷 → 修复 → 回归 → 优化。全部实测。

### 一、测试分层与方法

| 层 | 范围 | 方法与工具 |
|---|---|---|
| 单元 | mox-ai-expert-svc 库测试 | `cargo test -p mox-ai-expert-svc --lib`（189 用例） |
| 集成 | expert-svc 58 路由 + 网关 44 反代/图谱 API | 企业级集成测试脚本（74 用例：专家/会话/调度/图谱/计划/编排/企业/联盟/反代/图） |
| 契约 | 前端 experts.api.js(52 函数) + graph.api.js HTTP 路径 | 前端 → 网关 → 服务端逐路径探测 |
| 数据库 | SQLite 4 表 + KV + KG 快照 | sqlite3 直接核验 + 幂等/重启保留验证 |
| 端到端 | 前端(Vite) → 网关(8080) → 服务(3002) → 数据库 | 真实调用链（咨询/图谱/计划/页面） |
| 性能 | 关键 API 延迟 | 3 次采样中位数/最大值 |
| 边界 | 空/缺字段/非法枚举/不存在资源/超长输入 | 异常路径探测 |

### 二、测试结果汇总

| 批次 | 用例 | 通过 | 结果 |
|---|---|---|---|
| 单元（库测试） | 189 | 189 | 全绿（0 失败 1 忽略） |
| 后端集成（74 项） | 74 | 74 | 修复后全绿 |
| 前端契约探测 | 57 | 57 | 全通（0 缺失 0 断连） |
| 前端生产构建 | — | — | vite build 成功（28.81s） |
| 数据库持久化 | 4 表+KV+KG | 全过 | 14 专家干净基线；sessions/plans/dispatcher_cfg 重启保留 |
| 端到端链路 | 5 | 5 | 全通（咨询 4.7s/图 4ms/KG 22ms/计划 10ms/页面 33ms） |
| 边界/异常 | 17 | 17 | 全过（含 422 参数强校验、400 非法状态） |

### 三、发现并修复的真实缺陷（3 处）

1. **缺失资源未返回 404（REST 契约缺陷）**
   - 现象：GET 不存在的专家/会话返回 `200 {found:false, expert:null}`；consult 不存在专家返回 200 且**误触真实 LLM**。
   - 修复：`server.rs` 三个 handler 增加 404 契约（`expert_not_found` / `session_not_found` + 中文错误）；`alliance_ext.rs consult_expert_by_id` 前置专家存在性检查，缺失直接返回 not-found 标记（避免误调 LLM）。
   - 实测：GET/consult 缺失专家 → 404；GET 缺失会话 → 404；真实资源 → 200。

2. **删除专家为空实现（功能缺陷）**
   - 现象：`delete_expert` 仅返回"已受理"标记，专家既不从内存删除也不落库；前端删除后专家仍存在，重启后复活。
   - 修复：`services.rs` RegistryImpl 新增 `remove`（内存移除 + 写通 SQLite）、AllianceService 新增 `remove_expert`；`alliance_ext.rs delete_expert` 真实删除并级联清理评分学习指标（`metrics:{id}`）。
   - 实测：删除后列表立即消失且 DB 同步删除；二次删除返回 `deleted:false 专家不存在`（幂等语义正确）。

3. **会话 ID 语义不一致（契约缺陷）**
   - 现象：`create_session` 忽略传入 `session_id`，用自动生成的 `sess-xxx` 作存储键；调用方用传入 `session_id` 查询 → 404。
   - 修复：`create_session` 优先接受 `id`/`session_id` 作为存储键，并保持二者一致（`id` 与 `session_id` 字段统一）。
   - 实测：create 用 `session_id` → 返回 id 一致 → get/delete 用同一 id 全部命中。

### 四、数据治理

- 清理测试脏数据：`qa-expert-001`、`exp-persist-001`（M3 验证专家）、`metrics:not-exist`（修复前 consult 缺失误写）。
- 恢复干净基线：expert-svc 14 内置专家（DB 与内存一致）；metrics 仅保留合法 `metrics:algorithm`。
- KG 快照保留 2 个合法状态测试节点（`test-valid-status` developing、`bd-2` planned）作为状态校验功能痕迹。

### 五、性能基线（本机实测）

| API | 中位数 | 最大值 |
|---|---|---|
| /api/graph/stats | 2ms | 2ms |
| /api/expert-graph/stats | 16ms | 21ms |
| /api/graph/centrality | 322ms | 335ms |
| /api/expert-graph/optimal-team | 4ms | 15ms |
| /api/alliance/consult（本地规则 LLM） | 3068ms | 3634ms |

说明：咨询类延迟由真实/本地 LLM 推理主导，属预期；图谱/组队/统计均亚秒级。

### 六、优化与说明

- **服务端参数强校验**：空 body / 缺字段 POST 返回 422（axum 反序列化校验），符合 REST 规范，前端 http.js 已归一化中文提示。
- **服务进程生命周期**：测试期间网关(8080)/Vite(3020) 后台进程曾随会话回收消失，已按 M5.3 约定用后台任务方式重启并恢复（KG 快照 703 节点）；企业级部署应使用进程守护（如 systemd/supervisor）。
- **KG 快照并发写**：单文件 JSON 多实例并发写有覆盖风险（当前单实例无影响）；后续可升级 SQLite。
- **真实 LLM 依赖外部 provider**：deepseek 不可达/熔断时自动回退本地规则引擎（配置驱动，零侵入）。

### 七、回归确认

- 修复后重跑企业级集成测试 74/74 全绿；库测试 189 全绿；前端契约 57/57 全通；服务三端口（3002/8080/3020）+ 8000 在线。
- 改动文件：`server.rs`（404 契约 3 handler）、`alliance_ext.rs`（consult 前置检查 / delete_expert 真实删除 / create_session ID 统一）、`services.rs`（RegistryImpl.remove / AllianceService.remove_expert / registry_impl 字段）、`persistence.rs`（delete_kv）。

*文档生成：2026-09-02 | 依据：企业级测试实测 + M0-M5.4 全部执行记录*

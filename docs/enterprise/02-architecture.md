# 璇玑系统 · 企业级架构文档（多视图）

> **文档类型**：企业架构（Enterprise Architecture，多视图 / TOGAF 风格切面）
> **文档版本**：v1.0 (ENT) · 最后更新 2026-08-16
> **配套**：`01-requirements.md`（需求）、`03-design.md`（设计）、`04-business-processing.md`（业务处理）、`docs/architecture.md`（OUS 总架构）
>
> 本文以「璇玑系统」为切面，沿 **业务 / 信息 / 应用 / 技术 / 安全 / 集成 / 部署** 七视图展开，
> 并附 **架构决策记录（ADR）** 与 **跨视图 NFR 落地表**。

---

## 0. 架构一句话

璇玑系统是 OUS 的**协作治理子系统**：以「数学内核 + 插件运行时」为底座，在**多租户璇玑**边界内，
通过 **RBAC 统一鉴权闸门 + 领域事件总线 + 反应器** 将「成员/任务/权限/通信」四类能力解耦协同，
并把组织决策（谁来做、做什么、是否通过）交给 `xuanji-system`，把技术决策（怎么做得更快、是否可信）交给 `xuanji-expert` 的璇玑治理。

---

## 1. 业务架构视图（Business）

### 1.1 业务能力地图

```
                璇玑系统（协作治理）
   ┌──────────────┬──────────────┬──────────────┬──────────────┐
   │ 成员管理      │ 任务协作      │ 权限分配      │ 通信机制      │
   │ Member Mgmt  │ Task Collab  │ Permission   │ Communication │
   └──────┬───────┴──────┬───────┴──────┬───────┴──────┬───────┘
          │              │              │              │
          ▼              ▼              ▼              ▼
   [入璇玑/激活/停权]  [立项/派发/推进]  [RBAC/作用域]  [频道/消息/通知]
          └──────────────┬──────────────┘
                         ▼
                  [审计留痕 / 事件溯源]
                         │
                         ▼
                  [璇玑融合（璇玑治理）] → 算子市场上架
```

### 1.2 价值流

`组建璇玑 → 邀请专家 → 立项任务 → 分派协同 → 推进验收 → 融合优化 → 上架复用`。
全程横切**审计留痕**与**事件驱动的实时通信**。

### 1.3 组织角色（与 RBAC 对齐）

`XuanjiAdmin / Coordinator / Expert / Member / Auditor`（详见 `01-requirements` §2、`03-design` §RBAC）。

---

## 2. 信息/数据架构视图（Information）

### 2.1 领域模型（核心实体）

| 实体 | 关键属性 | 生命周期 |
|------|----------|----------|
| Xuanji（璇玑） | id, name, created_by, channels[] | 创建后常驻 |
| Member（成员） | id, xuanji_id, name, email, status, tier, expertise[] | Invited→Active→{Suspended\|Left} |
| Task（任务） | id, xuanji_id, title, status, assignees[], deps[], subtasks[], comments[] | Draft→…→Done/Cancelled |
| RoleBinding（角色绑定） | member_id, role, scope | 随成员/治理变更 |
| Channel（频道） | id, kind(Xuanji/Task/Direct), members[] | 惰性创建 |
| Message（消息） | id, channel_id, sender, body, kind | 追加不可变 |
| Notification（通知） | id, member_id, body, read | 推送+留存 |
| AuditRecord（审计） | id, member_id, action, permission, scope, reason, ts | 仅追加 |
| DomainEvent（领域事件） | 9 类（MemberInvited/TaskCreated/TaskAssigned/TaskStatusChanged/CommentAdded/…） | 事件溯源源 |

### 2.2 数据流（核心闭环）

```
写操作(成员/任务/权限/通信)
   │  produce
   ▼
DomainEvent ──▶ EventBus(broadcast)
                    │  subscribe
                    ▼
                Reactor(反应器)
                 ├─▶ CommService.send_message  → Channel(系统消息)
                 └─▶ CommService.notify        → Notification → WebSocket 推送
```

### 2.3 数据驻留与保留

- **持久化态（I-01/I-02 已落地）**：`Store` 通过 `trait Repository` 抽象，写透 + 启动重放，重启不丢且幂等。
- **多后端可移植（NFR-03）**：支持 `SQLite`（默认，单租户/单节点）/ `PostgreSQL` / `MySQL` 三种后端，由环境变量选择，业务层对后端无感知。方言差异统一在 `repo/schema.rs` 由 `sea-query` 按方言生成，详见 §7.4。
- **内存态**：`XUANJI_PERSIST=false` 时为纯内存 `RwLock<State>`（重启失忆），仅用于测试/演示，接口与持久化态完全一致。
- 审计数据：`AuditChain` 已落盘且不可变追加，支持重放（I-02）。

---

## 3. 应用/服务架构视图（Application）

### 3.1 分层映射（对齐 OUS 五层，见 `docs/architecture.md` §2）

```
┌─────────────────────────────────────────────────────────────┐
│ 接入层 Ingress   Vue3/Three.js · REST · WebSocket(SSE)        │
├─────────────────────────────────────────────────────────────┤
│ 运行时 Runtime   令牌↔成员解析 · RBAC 鉴权闸门(middleware)     │
├─────────────────────────────────────────────────────────────┤
│ 编排层 Orchestration  XuanjiSystem 门面                     │
│                  ├─ require() 统一鉴权                        │
│                  └─ Reactor 事件→通信 反应器                  │
├─────────────────────────────────────────────────────────────┤
│ 核心域 Core Domain   MemberService · TaskService             │
│                    PermissionService · CommService           │
├─────────────────────────────────────────────────────────────┤
│ 数据层 Data         Store(接口) · EventBus(broadcast)         │
└─────────────────────────────────────────────────────────────┘
        ▲ 融合治理旁路
        │
   xuanji-expert（双璇玑十四维 · 璇玑）
```

### §3.2 Rust Workspace 16 Crate · AIS 分层全表（三方对账基准）

本表同步自 16 个 crate 的 `pub const CRATE_ID` / `pub const CRATE_META` 常量（与 project-atlas 注册表 Rust 条目三方对账一致）。每行 = 1 个璇玑独立子项目。

| 序号 | Crate ID (kebab) | 路径 | AIS 分层 | 核心职责（1句） | 顶层入口 codePath | 引擎节点 id (engine-rust-*) | Owner 项目 (owner_project) |
|------|-------------------|------|---------|----------------|-------------------|---------------------------|---------------------------|
| 1 | operator-core crate | platform/services/operator-core | L4-Core, L6-Kernel | 算子代数/守恒律/类型核心 | src/lib.rs + types.rs | engine-rust-operator-core | proj-graph-infra |
| 2 | operator-wasm crate | platform/services/operator-wasm | L5-Infra | WASM 字节码算子沙箱 | src/lib.rs | module-rust-operator-wasm | proj-auto-dev |
| 3 | graph-algorithms crate | platform/services/graph-algorithms | L4-Core | 8 图算法:PR/CNM/介数/harmonic/Hebbian/推荐/密度/模块度 | src/lib.rs | engine-rust-graph-algorithms | proj-ai-engine |
| 4 | optimizer crate | platform/services/optimizer | L4-Core | DAG/CPM 关键路径 + RCPSP 贪心调度 | src/lib.rs | engine-rust-optimizer | proj-graph-infra |
| 5 | flow-ai crate | platform/services/flow-ai | L4-Core, L3-Service, L7-Tool | 9 模块:数据冒险/CPM/冲突/调度/拓扑/代码gen/流水线/原语/可视化 | src/lib.rs (+ bin/flowopt.rs) | engine-rust-flow-ai | proj-graph-infra |
| 6 | xuanji-expert crate | platform/services/xuanji-expert | L3-Service, L2-Gateway, L1-Ingress, L5-Infra | 14 专家并行+裁决+4验证+审计S3/Kafka+RBAC+流程加载器 | src/lib.rs (+ bin/xuanji.rs) | engine-rust-xuanji-expert | proj-expert-alliance |
| 7 | hermes-flow-bridge crate | platform/services/hermes-flow-bridge | L2-Gateway, L3-Service, L7-Tool | Hermes Agent 桥接：normalize/recorder/router/拦截注入 | src/lib.rs (+ bin/bridge_demo.rs) | module-rust-hermes-flow-bridge | proj-auto-dev |
| 8 | business-catalog crate | platform/services/business-catalog | L3-Service, L7-Tool | 7 预置 FlowGraph + TopologyGraph (政务/法院/财务/客服/ETL/MCP/螺旋) | src/lib.rs (+ bin/catalog.rs) | module-rust-business-catalog | proj-auto-dev |
| 9 | ai-agent crate | platform/services/ai-agent | L3-Service, L4-Core, L6-Kernel | 多阶段 Engine/LLMClient 路由/浏览器自动化/需求编译器/BPMN Workflow/MultiAgent/ProviderRegistry | src/lib.rs (+ tests/caomei_e2e.rs) | engine-rust-ai-agent | proj-ai-dialogue |
| 10 | template-market crate | platform/services/template-market | L3-Service | 模板市场发布/列表/加载/评分/排序/Fork/2商城种子 | src/lib.rs | module-rust-template-market | proj-auto-dev |
| 11 | xuanji-system crate | platform/services/xuanji-system | L1-Ingress, L2-Gateway, L3-Domain, L5-Infra, L6-Kernel | 成员/任务/权限/通信核心业务+RBAC/限流/事件编排/多后端 SQLite+PG+MySQL repo | src/lib.rs (server.rs, orchestrator.rs, repo/) | engine-rust-xuanji-system | proj-xuanji-core |
| 12 | primiflow-core crate | platform/services/primiflow-core | L4-Core, L5-Infra, L1-Ingress | PrimiFlow 解析/代码生成/8 类骨架模板/执行/持久化 | src/lib.rs (parse.rs, generate.rs, persistence.rs) | engine-rust-primiflow-core | proj-xuanji-core |
| 13 | primiflow-fusion crate | platform/services/primiflow-fusion | L2-Gateway, L3-Service, L1-Ingress, L6-Kernel | 六维融合/守恒闸门/Registry/平台编排/12Factor+可观测 | src/lib.rs (registry.rs, sixdim.rs, platform.rs) | engine-rust-primiflow-fusion | proj-xuanji-core |
| 14 | kg-hub crate | platform/services/kg-hub | L3-Service, L6-Kernel, L1-Ingress | HybridIndex+URN+本体/摄入/推理/治理/影响/热点/闭环8段/5连接器 | src/lib.rs (index.rs, ingest.rs, reason.rs, loop_engine.rs) | engine-rust-kg-hub | proj-knowledge |
| 15 | runtime (gateway) crate | platform/gateway/runtime | L1-Ingress, L2-Gateway | 16 crate 聚合网关:routes/handlers/Cordis5子模块/RBAC中间件/market DSL/迁移/治理/OpenAPI/operator-server | src/lib.rs (routes/, handlers/, cordis/, main.rs) | engine-rust-runtime | proj-xuanji-platform |

### 3.3 模块依赖

```
server.rs ──▶ orchestrator.rs(XuanjiSystem) ──▶ services.rs(Member/Task/Permission/Comm)
                                                    │
                                                    ▼
                                              store.rs + event.rs(EventBus)
xuanji-system ◀── POST /api/xuanji/* ── xuanji-expert(pipeline)
```

### 3.4 服务职责（单一职责）

| 服务 | 职责 | 不负责 |
|------|------|--------|
| MemberService | 成员生命周期、邀请幂等、状态机 | 任务逻辑 |
| TaskService | 任务 FSM、分派校验、DoD、DAG、评论 | 权限判定（交由 orchestrator.require） |
| PermissionService | 角色绑定增删、权限解析 | 请求上下文（交由 orchestrator） |
| CommService | 频道/消息/通知的读写 | 事件产生（由领域动作产生） |
| Orchestrator | 鉴权闸门 + 反应器编排 | 具体领域规则（下沉到 Service） |

---

## 4. 技术架构视图（Technology）

### 4.1 技术栈

| 层 | 技术 | 说明 |
|----|------|------|
| 语言 | Rust 2021 + Tokio | 异步运行时 |
| Web | Axum 0.7 + tower-http(CORS) | REST + WebSocket |
| 序列化 | serde / serde_json | 事件与 API 载荷 |
| 并发原语 | tokio::sync::{RwLock, broadcast} | Store 锁 + 事件总线 |
| 前端 | Vue3 + Three.js + ECharts | 融合工作台 / 监控台 |
| 插件沙箱 | WASM(wasmer) | 第三方算子隔离（OUS 内核） |

### 4.2 关键设计取舍

- **内存态优先**：降低首版复杂度，接口预留可替换（NFR-03）。
- **broadcast 事件总线**：多消费者（反应器、潜在的审计/指标消费者）解耦。
- **令牌即身份**：`X-Auth-Token` ↔ member_id 映射，缺失即 401（NFR-12）。

---

## 5. 安全架构视图（Security）

### 5.1 威胁模型（STRIDE 对齐 xuanji-expert `security.rs`）

| 威胁 | 缓解 |
|------|------|
| **S**poofing 伪造身份 | 令牌鉴权；bootstrap 唯一无鉴权入口 |
| **T**ampering 篡改 | 写操作统一 `require()`；事件不可变追加 |
| **R**epudiation 抵赖 | 领域事件 + 审计记录 + 鉴权拒绝留痕 |
| **I**nfo 泄露（跨租户） | 查询按 xuanji_id 过滤；分派三重校验（GAP-2） |
| **D**oS | 配额/限流（NFR-09，路线图中） |
| **E**levation 提权 | 最小权限 + 作用域 + `*Own` 所有权 + 试探式鉴权不落审计防探测 |

### 5.2 RBAC 模型

- **角色**：XuanjiAdmin / Coordinator / Expert / Member / Auditor，继承链 `Coordinator→Expert→Member`。
- **权限**：14 原子（task:create/assign/edit:all/edit:own/view:all/view:assigned/comment/transition:all/transition:own、member:invite/manage、comm:send:xuanji/send:task/send:direct、audit:view）。
- **作用域**：Global（仅 bootstrap 管理员）/ Xuanji（受邀默认）/ Task（临时授权）。
- **所有权**：`*Own` 类权限额外要求调用者在 `task.assignees` 中（前提：assignees 可信，见 GAP-2 修复）。

### 5.3 安全护栏（关键）

- **跨租户提权路径已闭环**：修复前 `assign()` 对 assignees 零校验 → 可写入他璇玑成员 ID 获得 own 权限；修复后 `validate_assignees` 三重校验（不存在/跨璇玑/非Active 均拒）。
- **审计不可被噪声淹没**：两段式鉴权，仅终局裁决失败留痕（见 `01-requirements` FR-PERM-06 / `br18`）。
- **AuthzDenied 事件不回推当事人**：避免实时通道成为权限探测反馈信道。

---

## 6. 集成架构视图（Integration）

### 6.1 对外接口

| 类别 | 端点 | 说明 |
|------|------|------|
| 健康检查 | `GET /api/health` | 探针 |
| 身份 | `POST /api/bootstrap`、`GET /api/me` | 建璇玑/令牌解析 |
| 成员 | `POST/GET /api/members` 等 | 邀请/列表/状态 |
| 任务 | `POST/GET /api/tasks`、`POST /api/tasks/:id/transition` 等 | 全生命周期 |
| 通信 | `GET /api/channels`、`POST /api/channels/:id/messages` | 频道消息 |
| 实时 | `WS /api/ws?token=` | 通知推送 |
| 融合 | `POST /api/xuanji/optimize`、`/publish` | 璇玑治理→上架 |

### 6.2 事件契约

`DomainEvent` 9 类，是审计、实时推送、潜在指标采集的**唯一事实源**（单一数据源原则）。

### 6.3 与外部系统

- `xuanji-expert`：归一化/治理/优化（旁路集成）。
- 算子市场：`/api/xuanji/publish` 上架优化产物（见 `docs/modules/business-process-flowcharts.md` §8）。
- LLM（可选）：企业流程 AiTask 真实执行，未配置 fail-closed。

---

## 7. 部署/运维视图（Deployment & Ops）

### 7.1 部署视图（运行形态）

- 单体进程：`cargo run -p xuanji-system` → `:3000`（REST+WS）；`--demo` 端到端演示。
- 作为 OUS 子系统：由 `runtime` 主服务聚合各 crate 端点。

#### 7.1.1 runtime crate 聚合内部架构（L1+L2 细项）

**L1 Ingress（路由+处理器薄层）：**
- `src/routes/mod.rs` 路由总入口：agent.rs (AI 智能体端点) + governance.rs（治理台 HITL/审批/指标） + market.rs（算子市场 DSL/版本化/迁移）
- `src/handlers/mod.rs` HTTP 处理器薄层：agent.rs / governance.rs / hitl.rs（纯 request→response 适配，不含业务算法）
- `main.rs` 二进制入口 `operator-server`（axum server 启动 + 生命周期 + 优雅停机）

**L2 Gateway（编排/中间件/聚合）：**
- `src/cordis/mod.rs`（OUS-Cordis 插件内核 5 子模块）：profile + bundle + seam(SeamRegistry fs 注册) + event_bus(事件瀑布) + lifecycle(Start/Stop/Pause)
- `src/rbac_middleware.rs`：RBAC 鉴权闸门（X-Auth-Token TokenRegistry → member_id + 角色校验）
- `src/subservers.rs`：聚合 16 crate 的子服务（ai-agent/xuanji-expert 等）挂载编排
- Feature gates: `market`（算子市场）/ `governance`（治理台）/ `openapi`（OpenAPI 生成） — 默认开启
- `src/automation.rs` + `api_standard.rs` + `openapi.rs`：API 标准化响应 / OpenAPI schema 生成

### 7.2 可观测性（设计态 → 路线图）

| 信号 | 现状 | 目标（NFR-08） |
|------|------|----------------|
| 日志 | 结构化（tracing） | 统一采集 |
| 指标 | 未采集 | 鉴权拒绝率/状态迁移分布/优化耗时（Prometheus） |
| 追踪 | trace_id 注入 | 跨 crate 链路 |

### 7.3 灾备

- **已落地**：Store 持久化写透 + 启动重放、审计链落盘重放（I-01/I-02）。SQLite 单文件可冷备；PostgreSQL / MySQL 可复用其原生主从与 PITR 备份体系。
- **待办**：WAL 快照与混沌演练（I-12，见 `05` 路线图）。对齐 `docs/architecture.md` §16。

### 7.4 持久化后端选型与配置矩阵（唯一权威落点）

> 本节是璇玑系统持久化配置的**唯一事实基准**。根 `README.md` 与其他文档仅可指向本节，不得重复定义，避免配置口径漂移。

**设计原则**：12-Factor 配置注入，零代码切换；默认 `SQLite` 保证开箱即用、无外部依赖。

| 环境变量 | 取值 | 默认 | 说明 |
|----------|------|------|------|
| `XUANJI_PERSIST` | `true`/`1` \| `false`/`0` | `false` | 是否落盘。`false` 为纯内存态（重启失忆，仅测试/演示） |
| `XUANJI_STRICT_PERSIST` | `true`/`1` \| `false`/`0` | `false` | **生产级 fail-fast**：打开后若连库或建表失败，**启动直接中止**，杜绝"连不上库却照常起服务、数据只进内存、重启即丢"的静默故障 |
| `XUANJI_BACKEND` | `sqlite` \| `postgres` \| `mysql` | `sqlite` | 后端方言。无法识别时安全回退 `sqlite` |
| `XUANJI_DB_URL` | 连接串 | `./data/xuanji.db` | SQLite 为文件路径；PG/MySQL 为标准 URL |

**推荐组合矩阵**：

| 场景 | `PERSIST` | `STRICT_PERSIST` | `BACKEND` | `DB_URL` |
|------|-----------|------------------|-----------|----------|
| 本地开发 / 单节点（默认） | `false` | `false` | `sqlite` | 默认 |
| SQLite 持久化 | `true` | `false` | `sqlite` | `./data/xuanji.db` |
| **PostgreSQL 生产** | `true` | **`true`** | `postgres` | `postgres://user:pass@host:5432/db` |
| **MySQL 生产** | `true` | **`true`** | `mysql` | `mysql://user:pass@host:3306/db` |

**方言归一化实现**：upsert 语义按后端生成——SQLite `INSERT OR REPLACE` + `?N`；PostgreSQL `ON CONFLICT DO UPDATE` + `$N`；MySQL `ON DUPLICATE KEY UPDATE` + `?`。落点 `crates/xuanji-system/src/repo/`（`schema.rs` 方言层 + `sqlite.rs`/`postgres.rs`/`mysql.rs` 驱动层）。

**启动可观测性**：启动日志如实回显后端与严格模式，便于运维核对实际生效配置：

```
持久化模式: 开启 (后端=Postgres, 严格模式=开(连库失败即中止))
```

**fail-fast 错误路径**：`Store::open` 内 `migrate()` 失败已 `?` 成 `Err`，故 `XUANJI_STRICT_PERSIST` **同时覆盖「连接失败」与「建表失败」**两条路径；错误经 `with_config` 上浮至 `main` 以规整致命信息 + 非零退出码终止（非 panic backtrace），契合容器编排重启探针语义。

---

## 8. 架构决策记录（ADR）

| ADR | 决策 | 背景 | 后果 |
|-----|------|------|------|
| ADR-01 | RBAC 作为统一鉴权闸门，所有写操作经 `require()` | 避免散落权限判断 | 单一入口易审计；需防绕过 |
| ADR-02 | 领域事件 + 反应器解耦通信 | 写逻辑与推送解耦 | 可加指标/审计消费者；需保证幂等 |
| ADR-03 | ~~内存 Store + 稳定接口，暂不实持久化~~（**已被 ADR-07 取代**） | 首版降复杂度 | 🟡 SUP：稳定接口设计使后续替换零成本 |
| ADR-07 | `trait Repository` 多后端持久化：SQLite（默认）/ PostgreSQL / MySQL，12-Factor 选择 + `STRICT_PERSIST` fail-fast | 兑现 NFR-03 可移植性；生产需外部库与主从备份，开发需零依赖 | 重启不丢；方言差异集中于 `repo/schema.rs`；严格模式下连库失败即中止启动（见 §7.4） |
| ADR-04 | `*Own` 所有权权限依赖可信 `assignees` | 精细授权 | 分派必须校验（GAP-2 修复） |
| ADR-05 | 两段式鉴权（试探不落审计） | 防审计噪声/权限探测 | 实现稍复杂；测试双向断言 |
| ADR-06 | 融合治理与协作治理分离为两域 | 关注点分离 | 双验收联动待补（FR-FUSE-05） |

---

## 9. 跨视图 NFR 落地表

| NFR | 业务视图 | 应用视图 | 技术视图 | 安全视图 |
|-----|----------|----------|----------|----------|
| NFR-01 多租户 | 璇玑边界 | 查询过滤 | Store 隔离 | 分派校验 |
| NFR-04 解耦 | 事件闭环 | Reactor | broadcast | 审计独立 |
| NFR-08 可观测 | 指标定义 | 指标埋点 | tracing | — |
| NFR-11 一致性 | 幂等 | Reactor 幂等 | broadcast | 事件不可变 |

---

## 10. 与父系统 OUS 的关系

- 本文是 `docs/architecture.md`（v7.0，79KB 总架构）的**璇玑子系统切面**。
- 能力对齐见 `docs/enterprise-architecture-analysis.md`（双璇玑十四维、能力覆盖矩阵）。
- 融合链路见 `docs/modules/xuanji-expert-alliance-fusion-flows.md`。

---

*本文七视图 + ADR 构成企业级架构骨架；详细模块设计见 `03-design.md`，业务处理见 `04-business-processing.md`。*

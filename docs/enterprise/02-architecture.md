# 专家联盟系统 · 企业级架构文档（多视图）

> **文档类型**：企业架构（Enterprise Architecture，多视图 / TOGAF 风格切面）
> **文档版本**：v1.0 (ENT) · 最后更新 2026-08-16
> **配套**：`01-requirements.md`（需求）、`03-design.md`（设计）、`04-business-processing.md`（业务处理）、`docs/architecture.md`（OUS 总架构）
>
> 本文以「专家联盟系统」为切面，沿 **业务 / 信息 / 应用 / 技术 / 安全 / 集成 / 部署** 七视图展开，
> 并附 **架构决策记录（ADR）** 与 **跨视图 NFR 落地表**。

---

## 0. 架构一句话

专家联盟系统是 OUS 的**协作治理子系统**：以「数学内核 + 插件运行时」为底座，在**多租户联盟**边界内，
通过 **RBAC 统一鉴权闸门 + 领域事件总线 + 反应器** 将「成员/任务/权限/通信」四类能力解耦协同，
并把组织决策（谁来做、做什么、是否通过）交给 `alliance-system`，把技术决策（怎么做得更快、是否可信）交给 `expert-alliance` 的璇玑治理。

---

## 1. 业务架构视图（Business）

### 1.1 业务能力地图

```
                专家联盟系统（协作治理）
   ┌──────────────┬──────────────┬──────────────┬──────────────┐
   │ 成员管理      │ 任务协作      │ 权限分配      │ 通信机制      │
   │ Member Mgmt  │ Task Collab  │ Permission   │ Communication │
   └──────┬───────┴──────┬───────┴──────┬───────┴──────┬───────┘
          │              │              │              │
          ▼              ▼              ▼              ▼
   [入盟/激活/停权]  [立项/派发/推进]  [RBAC/作用域]  [频道/消息/通知]
          └──────────────┬──────────────┘
                         ▼
                  [审计留痕 / 事件溯源]
                         │
                         ▼
                  [联盟融合（璇玑治理）] → 算子市场上架
```

### 1.2 价值流

`组建联盟 → 邀请专家 → 立项任务 → 分派协同 → 推进验收 → 融合优化 → 上架复用`。
全程横切**审计留痕**与**事件驱动的实时通信**。

### 1.3 组织角色（与 RBAC 对齐）

`AllianceAdmin / Coordinator / Expert / Member / Auditor`（详见 `01-requirements` §2、`03-design` §RBAC）。

---

## 2. 信息/数据架构视图（Information）

### 2.1 领域模型（核心实体）

| 实体 | 关键属性 | 生命周期 |
|------|----------|----------|
| Alliance（联盟） | id, name, created_by, channels[] | 创建后常驻 |
| Member（成员） | id, alliance_id, name, email, status, tier, expertise[] | Invited→Active→{Suspended\|Left} |
| Task（任务） | id, alliance_id, title, status, assignees[], deps[], subtasks[], comments[] | Draft→…→Done/Cancelled |
| RoleBinding（角色绑定） | member_id, role, scope | 随成员/治理变更 |
| Channel（频道） | id, kind(Alliance/Task/Direct), members[] | 惰性创建 |
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

- 当前：`Store`（内存 `RwLock<State>`），重启失忆（见 NFR-03 / 路线图持久化）。
- 设计：接口稳定，可替换为 SQLite（单租户）或分布式 KV（多租户分片）。
- 审计数据：建议 WAL 持久化 + 不可变追加，支持重放（路线图 `05`）。

---

## 3. 应用/服务架构视图（Application）

### 3.1 分层映射（对齐 OUS 五层，见 `docs/architecture.md` §2）

```
┌─────────────────────────────────────────────────────────────┐
│ 接入层 Ingress   Vue3/Three.js · REST · WebSocket(SSE)        │
├─────────────────────────────────────────────────────────────┤
│ 运行时 Runtime   令牌↔成员解析 · RBAC 鉴权闸门(middleware)     │
├─────────────────────────────────────────────────────────────┤
│ 编排层 Orchestration  AllianceSystem 门面                     │
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
   expert-alliance（双联盟十四维 · 璇玑）
```

### 3.2 模块依赖

```
server.rs ──▶ orchestrator.rs(AllianceSystem) ──▶ services.rs(Member/Task/Permission/Comm)
                                                    │
                                                    ▼
                                              store.rs + event.rs(EventBus)
alliance-system ◀── POST /api/alliance/* ── expert-alliance(pipeline)
```

### 3.3 服务职责（单一职责）

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

### 5.1 威胁模型（STRIDE 对齐 expert-alliance `security.rs`）

| 威胁 | 缓解 |
|------|------|
| **S**poofing 伪造身份 | 令牌鉴权；bootstrap 唯一无鉴权入口 |
| **T**ampering 篡改 | 写操作统一 `require()`；事件不可变追加 |
| **R**epudiation 抵赖 | 领域事件 + 审计记录 + 鉴权拒绝留痕 |
| **I**nfo 泄露（跨租户） | 查询按 alliance_id 过滤；分派三重校验（GAP-2） |
| **D**oS | 配额/限流（NFR-09，路线图中） |
| **E**levation 提权 | 最小权限 + 作用域 + `*Own` 所有权 + 试探式鉴权不落审计防探测 |

### 5.2 RBAC 模型

- **角色**：AllianceAdmin / Coordinator / Expert / Member / Auditor，继承链 `Coordinator→Expert→Member`。
- **权限**：14 原子（task:create/assign/edit:all/edit:own/view:all/view:assigned/comment/transition:all/transition:own、member:invite/manage、comm:send:alliance/send:task/send:direct、audit:view）。
- **作用域**：Global（仅 bootstrap 管理员）/ Alliance（受邀默认）/ Task（临时授权）。
- **所有权**：`*Own` 类权限额外要求调用者在 `task.assignees` 中（前提：assignees 可信，见 GAP-2 修复）。

### 5.3 安全护栏（关键）

- **跨租户提权路径已闭环**：修复前 `assign()` 对 assignees 零校验 → 可写入他联盟成员 ID 获得 own 权限；修复后 `validate_assignees` 三重校验（不存在/跨联盟/非Active 均拒）。
- **审计不可被噪声淹没**：两段式鉴权，仅终局裁决失败留痕（见 `01-requirements` FR-PERM-06 / `br18`）。
- **AuthzDenied 事件不回推当事人**：避免实时通道成为权限探测反馈信道。

---

## 6. 集成架构视图（Integration）

### 6.1 对外接口

| 类别 | 端点 | 说明 |
|------|------|------|
| 健康检查 | `GET /api/health` | 探针 |
| 身份 | `POST /api/bootstrap`、`GET /api/me` | 建盟/令牌解析 |
| 成员 | `POST/GET /api/members` 等 | 邀请/列表/状态 |
| 任务 | `POST/GET /api/tasks`、`POST /api/tasks/:id/transition` 等 | 全生命周期 |
| 通信 | `GET /api/channels`、`POST /api/channels/:id/messages` | 频道消息 |
| 实时 | `WS /api/ws?token=` | 通知推送 |
| 融合 | `POST /api/alliance/optimize`、`/publish` | 璇玑治理→上架 |

### 6.2 事件契约

`DomainEvent` 9 类，是审计、实时推送、潜在指标采集的**唯一事实源**（单一数据源原则）。

### 6.3 与外部系统

- `expert-alliance`：归一化/治理/优化（旁路集成）。
- 算子市场：`/api/alliance/publish` 上架优化产物（见 `business-process-flowcharts.md` §8）。
- LLM（可选）：企业流程 AiTask 真实执行，未配置 fail-closed。

---

## 7. 部署/运维视图（Deployment & Ops）

### 7.1 运行形态

- 单体进程：`cargo run -p alliance-system` → `:3000`（REST+WS）；`--demo` 端到端演示。
- 作为 OUS 子系统：由 `runtime` 主服务聚合各 crate 端点。

### 7.2 可观测性（设计态 → 路线图）

| 信号 | 现状 | 目标（NFR-08） |
|------|------|----------------|
| 日志 | 结构化（tracing） | 统一采集 |
| 指标 | 未采集 | 鉴权拒绝率/状态迁移分布/优化耗时（Prometheus） |
| 追踪 | trace_id 注入 | 跨 crate 链路 |

### 7.3 灾备（路线图 `05`）

- 当前：进程内存态，无持久化。
- 目标：Store 持久化 + 审计链 WAL 重放 + 快照（对齐 `docs/architecture.md` §16）。

---

## 8. 架构决策记录（ADR）

| ADR | 决策 | 背景 | 后果 |
|-----|------|------|------|
| ADR-01 | RBAC 作为统一鉴权闸门，所有写操作经 `require()` | 避免散落权限判断 | 单一入口易审计；需防绕过 |
| ADR-02 | 领域事件 + 反应器解耦通信 | 写逻辑与推送解耦 | 可加指标/审计消费者；需保证幂等 |
| ADR-03 | 内存 Store + 稳定接口，暂不实持久化 | 首版降复杂度 | NFR-03 可替换；重启失忆 |
| ADR-04 | `*Own` 所有权权限依赖可信 `assignees` | 精细授权 | 分派必须校验（GAP-2 修复） |
| ADR-05 | 两段式鉴权（试探不落审计） | 防审计噪声/权限探测 | 实现稍复杂；测试双向断言 |
| ADR-06 | 融合治理与协作治理分离为两域 | 关注点分离 | 双验收联动待补（FR-FUSE-05） |

---

## 9. 跨视图 NFR 落地表

| NFR | 业务视图 | 应用视图 | 技术视图 | 安全视图 |
|-----|----------|----------|----------|----------|
| NFR-01 多租户 | 联盟边界 | 查询过滤 | Store 隔离 | 分派校验 |
| NFR-04 解耦 | 事件闭环 | Reactor | broadcast | 审计独立 |
| NFR-08 可观测 | 指标定义 | 指标埋点 | tracing | — |
| NFR-11 一致性 | 幂等 | Reactor 幂等 | broadcast | 事件不可变 |

---

## 10. 与父系统 OUS 的关系

- 本文是 `docs/architecture.md`（v7.0，79KB 总架构）的**联盟子系统切面**。
- 能力对齐见 `docs/enterprise-architecture-analysis.md`（双联盟十四维、能力覆盖矩阵）。
- 融合链路见 `docs/expert-alliance-alliance-fusion-flows.md`。

---

*本文七视图 + ADR 构成企业级架构骨架；详细模块设计见 `03-design.md`，业务处理见 `04-business-processing.md`。*

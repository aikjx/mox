# 璇玑系统 · 详细设计文档（Detailed Design）

> **文档类型**：详细设计（模块 / 接口 / 算法 / 数据流）
> **文档版本**：v1.0 (ENT) · 最后更新 2026-08-16
> **代码基线**：`crates/mox-system/`（Rust，已 `cargo build/test` 通过）
> **配套**：`01-requirements.md`、`02-architecture.md`、`04-business-processing.md`

---

## 1. 模块总览

```
crates/mox-system/src/
├── lib.rs          导出模块 + MoxSystem 类型别名
├── main.rs         CLI（--demo 端到端 / 启动 HTTP+WS 服务）
├── model.rs        领域模型（Mox/Member/Task/Channel/Message/Notification/Audit/枚举）
├── error.rs        AppError + IntoResponse（HTTP 映射）
├── rbac.rs         角色/权限/作用域/继承/所有权判定
├── event.rs        DomainEvent 枚举 + EventBus(broadcast)
├── store.rs        Store 门面：内存态（RwLock<State>）+ 持久化写透/启动重放
├── repo/           持久化仓库层（多后端，见 02 §7.4 / ADR-07）
│   ├── mod.rs      trait Repository（migrate / load_all / persist_*）
│   ├── schema.rs   sea-query 方言层：DDL + upsert（三方言归一化）
│   ├── sqlite.rs   SQLite 驱动（默认，rusqlite）
│   ├── postgres.rs PostgreSQL 驱动（sqlx）
│   └── mysql.rs    MySQL 驱动（sqlx）
├── config.rs       12-Factor 配置：backend / db_url / persist / strict_persist / 配额 / 限流 / CORS
├── crypto.rs       令牌 SHA-256 哈希（仅存哈希，不存明文）
├── metrics.rs      Prometheus 文本指标（I-04）
├── ratelimit.rs    固定窗口限流（I-04）
├── services.rs     MemberService / TaskService / PermissionService / CommService
├── orchestrator.rs MoxSystem 门面：require() 鉴权 + Reactor 事件反应器
└── server.rs       Axum REST + WebSocket + 鉴权中间件 + CORS + /metrics + /health
tests/
├── integration.rs  端到端（越权拦截/事件→通知/角色继承/持久化/令牌哈希/配额）
└── business_rules.rs BR-01…BR-21 规则固化
```

**依赖方向**：`server → orchestrator → services → store/event`；`store → repo/*`（按 `config.backend` 选择实现）；`event` 被 `orchestrator.Reactor` 订阅。持久化仓库层对上游完全透明——`services` 及以上不感知后端方言。

---

## 2. 领域模型（`model.rs`）

### 2.1 关键枚举

```rust
enum MemberStatus { Invited, Active, Suspended, Left }      // Left 为终态
enum TaskStatus   { Draft, Assigned, InProgress, InReview, Done, Cancelled }
enum Role         { MoxAdmin, Coordinator, Expert, Member, Auditor }
enum Permission   { TaskCreate, TaskAssign, TaskEditAll, TaskEditOwn,
                    TaskViewAll, TaskViewAssigned, TaskComment,
                    TaskTransitionAll, TaskTransitionOwn,
                    MemberInvite, MemberManage,
                    CommSendMox, CommSendTask, CommSendDirect, AuditView }
enum Scope       { Global, Mox(String), Task(String) } // 权限生效边界
enum ChannelKind { Mox, Task, Direct }
enum MessageKind { User, System, Notification }
enum Tier        { Junior, Senior, Principal }              // 成员等级
```

### 2.2 核心结构（节选）

```rust
struct Mox { id, name, created_at, created_at }
struct Member   { id, mox_id, name, email, status: MemberStatus,
                  tier: Tier, expertise: Vec<String>, invited_by }
struct Task     { id, mox_id, title, desc, status: TaskStatus,
                  assignees: Vec<String>, deps: Vec<String>,
                  subtasks: Vec<SubTask>, comments: Vec<Comment>,
                  created_at, created_at, updated_at }
struct RoleBinding { member_id, role: Role, scope: Scope }
struct Channel  { id, mox_id, kind: ChannelKind, name, members: Vec<String> }
struct Message  { id, channel_id, sender_id, body, kind, created_at }
struct Notification { id, member_id, body, read: bool, created_at }
struct AuditRecord  { id, member_id, action, permission: Option<Permission>,
                      scope: Option<Scope>, reason: Option<String>, ok: bool, ts }
```

---

## 3. RBAC 引擎设计（`rbac.rs`）

### 3.1 角色权限矩阵（含继承）

| 权限 | Admin | Coordinator | Expert | Member | Auditor |
|------|:--:|:--:|:--:|:--:|:--:|
| task:create | ✓ | ✓ | — | — | — |
| task:assign | ✓ | ✓ | — | — | — |
| task:edit:all | ✓ | ✓ | — | — | — |
| task:edit:own | — | ↑ | ✓ | — | — |
| task:view:all | ✓ | ✓ | — | — | ✓ |
| task:view:assigned | — | ↑ | ✓ | ✓ | — |
| task:comment | ✓ | ✓ | ✓ | ✓ | — |
| task:transition:all | ✓ | ✓ | — | — | — |
| task:transition:own | — | ↑ | ✓ | — | — |
| member:invite | ✓ | ✓ | — | — | — |
| member:manage | ✓ | — | — | — | — |
| comm:send:mox | ✓ | ✓ | — | — | — |
| comm:send:task | ✓ | ✓ | ✓ | ✓ | — |
| comm:send:direct | ✓ | ✓ | ✓ | ✓ | — |
| audit:view | ✓ | ✓ | — | — | ✓ |

> `↑` = 经继承获得（Coordinator 继承 Expert，Expert 继承 Member）。

### 3.2 鉴权算法

```
authorize(member_id, permission, ctx) -> Result<()>:
  1. 取该 member 全部 RoleBinding（跨作用域）
  2. 对每个 binding：
       a. 若 role 直接/继承拥有 permission → 进入作用域检查
       b. 作用域匹配：Global 恒匹配；Mox(id) 需 ctx.mox_id==id；
          Task(id) 需 ctx.task==Some(id)
       c. 若 permission 为 *Own 类 → 额外要求 member_id ∈ task.assignees
  3. 任一 binding 满足即放行；否则 Forbidden
```

### 3.3 设计要点

- **所有权前提**：`*Own` 类权限依赖 `task.assignees` 可信（ADR-04）。因此 `assign()` 必须校验被分派者（见 §6.3）。
- **两段式鉴权**（GAP-5/FR-PERM-06）：`transition_task` 先试 `TaskTransitionAll`（专家本无，纯试探，**不落审计**），失败再试 `TaskTransitionOwn`（终局裁决，失败**必落审计** + 发 `AuthzDenied`）。避免每次专家正常操作产生假拒绝噪声，且 `AuthzDenied` 不回推当事人以防权限探测。

---

## 4. 任务状态机设计（`model.rs` + `services.rs`）

### 4.1 迁移表

```
           ┌─────────┬──────────┬────────────┬──────────┬──────┬───────────┐
From \ To   │ Draft   │ Assigned │ InProgress │ InReview │ Done  │ Cancelled│
┌──────────┼─────────┼──────────┼────────────┼──────────┼──────┼───────────┤
│ Draft    │   —     │   ✓      │     —      │    —     │  —   │    ✓      │
│ Assigned │   —     │   —      │     ✓      │    —     │  —   │    ✓      │
│ InProgress│  —     │   —      │     —      │    ✓     │  —   │    ✓      │
│ InReview │   —     │   —      │     ✓*     │    —     │  ✓** │    ✓      │
│ Done     │   —     │   —      │     —      │    —     │  —   │    —      │  (终态)
│ Cancelled│   —     │   —      │     —      │    —     │  —   │    —      │  (终态)
└──────────┴─────────┴──────────┴────────────┴──────────┴──────┴───────────┘
  * 打回返工  ** 需通过 DoD 门禁
```

### 4.2 DoD 门禁（`check_done_gate`，BR-10）

```
进入 Done 前：
  ∀ subtask: subtask.done == true
  ∧ ∀ dep in deps: dep.status == Done
  否则 → InvalidState
```

### 4.3 依赖 DAG 约束（`add_dependency`，BR-11）

```
拒绝：self-dependency(A→A) / direct cycle / indirect cycle / cross-mox
      （用 reaches(from, to) 环检测 + mox_id 比对）
幂等：重复依赖忽略
```

---

## 5. 成员状态机设计（`model.rs`）

```
[*] --> Invited      : invite()
Invited   --> Active   : activate()
Invited   --> Left     : 拒绝/撤回
Active    --> Suspended: 停权
Active    --> Left     : 主动退出
Suspended --> Active   : 恢复
Suspended --> Left     : 移除
Left      --> [*]      : 终态，不可复活
```

- `Left → Active` 拒绝（防「复活」）。
- `Invited → Suspended` 拒绝（须先激活）。
- 同状态重设：幂等或明确拒绝（固化一种语义）。

---

## 6. 服务层设计（`services.rs`）

### 6.1 MemberService

| 方法 | 行为 | 关键规则 |
|------|------|----------|
| invite(a,盟,受邀人) | 建 Member(Invited) + 授 Expert@Mox | 同 email 幂等→Conflict(BR-04)；最小权限(BR-03) |
| activate(id) | Invited→Active | 状态机校验 |
| set_status(id, s) | 改状态 | 状态机校验(BR-21) |
| can_take_task(m) | bool | status==Active(BR-05) |

### 6.2 TaskService

| 方法 | 行为 | 关键规则 |
|------|------|----------|
| create(...) | Task(Draft, assignees=[]) | BR-06 不得自带分派 |
| assign(id, list) | 校验→写入→Draft→Assigned | **validate_assignees** 三重校验(BR-07)；全量覆盖(BR-08) |
| transition(id, to) | 状态机 + DoD 校验 | BR-09/BR-10/BR-12 |
| add_dependency(a,b) | DAG 校验 | BR-11 |
| comment(id, ...) | 写评论 + 双事件 | — |

### 6.3 `validate_assignees`（安全核心，GAP-2）

```
for each uid in assignees:
  m = store.get_member(uid)?
  if m is None                      → BadRequest("成员不存在")
  if m.mox_id != task.mox_id → Forbidden("跨璇玑")
  if m.status != Active             → InvalidState("非活跃")
  if uid 重复                        → BadRequest("重复分派")
→ 全部通过才写回 assignees（可信集合）
```

> 这是闭环跨租户提权路径的关键修复：assignees 一旦可信，`*Own` 权限前提成立。

### 6.4 PermissionService

- `assign_role(binding)`：去重同 role+scope 后写入（revoke-then-set 语义）。

### 6.5 CommService

- `ensure_channel(kind, ref_id)`：惰性创建（璇玑大厅 / 任务频道 / 私信）。
- `send_message(ch, sender, body, kind)`：追加 Message。
- `notify(member_id, body)`：仅留存 Notification（WebSocket 由 server 订阅总线推送）。
- `list_notifications / list_channels / list_messages`：查询（受璇玑过滤）。

---

## 7. 编排层与事件反应器（`orchestrator.rs`）

### 7.1 `MoxSystem` 门面

- 持有 `Arc<Store>` + 各 Service + `EventBus` + `Reactor`。
- `require(actor, perm, ctx)`：**统一鉴权闸门**（FR-PERM-05）。失败时：
  - 若非「试探式」调用 → 落 `AuditRecord(ok=false)` + 发 `AuthzDenied` 事件（不回推当事人）。
- 领域方法（bootstrap/invite/create/assign/transition/comment）在执行后 `bus.publish(event)`。

### 7.2 Reactor（事件→通信 数据流）

```
DomainEvent ──▶ EventBus.subscribe()
   │
   ├─ MemberInvited {member, by}
   │     → notify(member, "您已被邀请") + 璇玑大厅系统消息
   ├─ TaskCreated {task}
   │     → 璇玑大厅系统消息
   ├─ TaskAssigned {task, assignees}
   │     → ensure 任务频道 + 逐一 notify + 频道系统消息
   ├─ TaskStatusChanged {task, from, to, by}
   │     → 频道系统消息 + 相关人 notify
   └─ CommentAdded {task, comment}
         → 频道消息 + 提及人 notify
```

- **幂等**：同一事件重放不得产生重复副作用（FR-COMM-06）。
- **异步**：`spawn` 消费总线，不阻塞写路径。

### 7.3 时序：任务分派触发通信

```
Client ──POST /api/tasks/:id/assign──▶ server
server ──▶ sys.assign_task(actor, id, list)
              ├─ require(actor, TaskAssign, ctx)        [闸门]
              ├─ task.assign(list)  → validate_assignees → Draft→Assigned
              └─ bus.publish(TaskAssigned)
                       │
                  EventBus ──▶ Reactor.handle
                       ├─ ensure_channel(Task, id)
                       ├─ comm.notify(each assignee, "您被分派任务")
                       └─ comm.send_message(任务频道, system, "X 将任务分派给 N 位专家")
                                │
                          (WebSocket) ──▶ 各专家客户端实时收到通知+频道消息
```

---

## 8. API 契约（`server.rs`）

### 8.1 鉴权

- 缺失 `X-Auth-Token` → `401`。
- 中间件解析令牌 → member_id → 注入 `Extension<member_id>`。
- 业务方法内部再调 `require()` 做细粒度 RBAC。

### 8.2 端点（节选）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/health` | 公开 | 探针 |
| POST | `/api/bootstrap` | 公开 | 建璇玑+首位管理员+令牌 |
| GET | `/api/me` | 已登录 | 当前成员 |
| GET/POST | `/api/members` | MemberInvite/View | 列表/邀请 |
| GET/POST | `/api/tasks` | TaskView/Create | 列表/立项 |
| POST | `/api/tasks/:id/assign` | TaskAssign | 分派 |
| POST | `/api/tasks/:id/transition` | TransitionAll/Own | 推进 |
| GET/POST | `/api/channels` | — | 频道列表/发消息 |
| GET | `/api/ws?token=` | 已登录 | WebSocket 通知 |
| GET | `/api/audit` | AuditView | 审计查询 |

> 完整契约与请求/响应 schema 见 `crates/mox-system/src/server.rs`；融合端点见 `docs/modules/business-process-flowcharts.md` §8。

---

## 9. 错误处理（`error.rs`）

`AppError` 变体 → HTTP 状态映射（`IntoResponse`）：

| 错误 | HTTP | 场景 |
|------|------|------|
| Unauthorized | 401 | 缺令牌 |
| Forbidden | 403 | RBAC 拒绝 / 跨璇玑 |
| BadRequest | 400 | 参数/不存在成员/重复分派 |
| InvalidState | 409/400 | 状态机/DoD 违规 |
| Conflict | 409 | 邀请幂等 |
| NotFound | 404 | 资源缺失 |

---

## 10. 设计权衡与遗留

| 决策 | 收益 | 代价 / 后续 |
|------|------|-------------|
| `trait Repository` 多后端 Store（SQLite 默认 / PG / MySQL） | 兑现 NFR-03 可移植；开发零依赖、生产可用外部库 | 方言差异需集中维护于 `repo/schema.rs`（已收口，见 `02` §7.4 / ADR-07） |
| broadcast 单总线 | 解耦、易扩展消费者 | 审计链已落盘重放（I-02）；WAL 快照待 I-12 |
| 两段式鉴权 | 审计可用、防探测 | 实现复杂度；测试双向断言固化 |
| 分派全量覆盖 | 语义明确 | 调用方需传完整名单 |

---

*本设计直接映射到 `crates/mox-system/src/*`；业务规则与流程见 `04-business-processing.md`，两者经 `01-requirements` 追踪矩阵闭环。*

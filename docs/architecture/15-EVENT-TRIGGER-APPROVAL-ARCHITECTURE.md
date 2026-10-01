---
title: 定时触发 × 事件监听 企业级架构 + 审批能力盘点（复用现有，不造轮子）
version: V1.2
authority: 🟢权威
doc_id: ARC-EVENT-APPROVAL
last_updated: 2026-09-27
source_of_truth: 代码事实（2026-09-27 轮子盘点：审批复用 mox-flow-unified-process-core / mox-alliance-scheduler-core::approval_gate；EventScheduler 落 mox-event-core/src/scheduler.rs；测试与 cargo 验证见 §7）
---

# 定时触发 × 事件监听 企业级架构 + 审批能力盘点

## 一、核心原则

1. **三种机制分开管**：定时触发器只负责"到点发事件"，事件监听器只负责"响应事件"，审批是业务状态机——三者通过事件总线解耦，不互相耦合。
2. **不重复造轮子（本轮铁律）**：审批能力**复用仓库既有实现**（`mox-flow-unified-process-core` 流程审批 + `mox-alliance-scheduler-core::approval_gate` 人审门），不再自建审批 crate；定时触发抽象仅保留仓库缺失的一件（EventScheduler）。
3. **纯算法与 IO 分离**（仓库既有决策）：调度器与状态机核心逻辑无 IO；存储/网络/通知由外层服务与事件总线负责。
4. **分布式语义外置**：多副本"到点只执行一次"由租约选主（leader）限定触发源；消息"不丢不重"由持久化 + 幂等消费解决。

## 二、现状盘点与轮子边界（2026-09-27 实测）

| 能力 | 位置 | 状态与定位 |
|------|------|------|
| 事件总线核心（订阅/通配符/同步异步发布/并发/超时/DLQ/重试/统计/优雅关停） | `platform/shared/mox-event-core` | ✅ **唯一事件基础设施权威** |
| **定时触发（EventScheduler：once/repeat/abort 取消/统计）** | `mox-event-core/src/scheduler.rs` | 🆕 **仓库唯一的"事件定时触发"抽象**（本轮保留） |
| 任务调度（专家匹配/排队/执行委派） | `mox-alliance-scheduler-core::scheduler` | ✅ 既有——**业务调度**，与 EventScheduler 职责不同 |
| 租约选主（多活 leader） | `mox-alliance-scheduler-core::leadership` | ✅ 既有——分布式语义接缝 |
| 卷容量调度（SeaweedFS 风格） | `mox-cloud-master-svc::scheduler` | ✅ 既有——**存储调度**，与 EventScheduler 职责不同 |
| **流程级审批**（Any/All/Sequential/Majority 四策略 + reject_to_start + 审批记录 + 待办/我发起的） | `mox-flow-unified-process-core::process_engine`（StepType::Approval + approve_step/reject_step/ApprovalRecord） | ✅ 既有——**流程审批权威**，复用 |
| **DAG 人审门**（Pending/Approved/Rejected/Expired + TTL + Hold/Release/Fail + reap_expired） | `mox-alliance-scheduler-core::approval_gate` | ✅ 既有——**节点级人审权威**，复用 |
| 前端 HITL 审批面板 | `frontend-ui/src/views/admin/panels/AdminHitl.vue`（/admin/hitl） | ✅ 既有——**前端审批 UI 权威**，复用 |
| ~~通用审批状态机（新建）~~ | ~~`mox-approval-core`~~ | ❌ **已撤销**（V1.0 曾新建，发现与上两项重复且弱于现有，2026-09-27 删除） |

## 三、定时触发管理（EventScheduler）

### 3.1 设计

```
注册（once / repeat） → 后台 tokio 任务计时 → 到点 bus.publish(event) → 订阅者响应
```

- **API**：`schedule_once(id, event_type, delay, factory)` / `schedule_repeat(id, event_type, interval, factory)` / `cancel(id)`（abort）/ `stats()`
- **载荷工厂闭包**：到点才构造事件，避免提前序列化
- **取消**：abort 后台任务句柄；once 任务自然结束后自动移出活跃表
- **容量上限**：`SchedulerConfig.max_tasks` 防失控注册

### 3.2 与既有调度器的边界（不重复造轮子的依据）

| 调度器 | 职责 | 是否与 EventScheduler 重复 |
|------|------|------|
| EventScheduler | **到点发事件**（时间→事件，基础设施抽象） | — |
| TaskSchedulerImpl（alliance） | 任务提交/排队/专家匹配/计划/执行委派（业务调度） | 否 |
| cloud-master scheduler | Volume 容量感知/副本放置/均衡（存储调度） | 否 |
| synchronizer（alliance） | 具体业务：定时同步外部专家源 | 否（具体业务；如改用 EventScheduler 驱动可渐进对齐"不自建 cron 循环"口径） |

### 3.3 分布式语义（调用方约定）

| 场景 | 约定 |
|------|------|
| 多副本重复触发（非幂等操作） | 由租约选主（leadership.rs 同族）限定**唯一 leader** 驱动本调度器 |
| **幂等周期任务（如专家同步）** | 无需 leader 约束——alliance synchronizer 的全量/增量同步为"拉取替换本地"幂等操作，多副本并行最终一致（2026-09-27 实测确认其 loop 不受 leadership 约束，属安全设计，不强制改造） |
| 重启丢任务 | 任务表当前为内存态（重启即失）；需要恢复时接 SQLite/WAL 存储实现（同 `MOX_ALLIANCE_STORAGE_MODE=sqlite` 约定） |
| 任务幂等 | 触发的事件载荷携带业务幂等键，订阅者按事件 ID 去重（at-least-once 消费） |
| **审批过期重估** | EventScheduler 周期发布 `approval.due.check` → 订阅者调 `approval_gate::reap_expired`（既有实现） |

## 四、事件监听管理（mox-event-core）

已有能力（**无需重开发**）：订阅管理 / 通配符（`test.*`）/ 同步与异步发布 / 并发信号量 / 订阅者超时 / 失败进死信队列（DLQ）/ 优雅关停 / 统计。

**分布式补充约定**（文档级，实现为接缝）：
1. **Outbox 模式**：业务事务内写事件表，由独立投递器发到总线——避免"业务成功但事件没发"
2. **幂等消费**：消费者按 `EventMetadata.event_id` 去重（业务表写消费记录）
3. **DLQ 重放**：死信条目支持人工介入与重放（`RetryPolicy` 已有）
4. **跨进程实现**：预留 Redis/Kafka 实现（bus.rs 注释已声明），当前单进程 MemoryEventBus 已覆盖单机部署

## 五、审批能力（复用现有，不造轮子）

### 5.1 两套既有审批权威的职责划分

| 场景 | 复用实现 | 能力 |
|------|------|------|
| **流程级审批**（表单/流程中的审批步骤，如请假、发布） | `mox-flow-unified-process-core`（StepType::Approval） | 四策略：Any（或签）/ All（会签）/ Sequential（顺序）/ Majority（多数）；`approve_step` / `reject_step`（含 reject_to_start 回退起点）；`ApprovalRecord` 审计记录；`get_pending_approvals`（待办）/ 我的发起 / 记录查询 |
| **DAG 节点人审门**（联盟任务执行到一半挂起等审批） | `mox-alliance-scheduler-core::approval_gate` | `ApprovalStatus::Pending/Approved/Rejected/Expired`；TTL 过期判定（`is_expired`）；`derive_effect → Hold/Release/Fail` 投影；`submit/list_pending/decide/reap_expired`；幂等（AlreadyDecided） |
| **审批 UI** | `frontend-ui` AdminHitl.vue（/admin/hitl） | HITL 待审批队列、审批通过/拒绝操作 |

### 5.2 EventScheduler 与审批的接缝（唯一新增点）

审批本身**不新增代码**；EventScheduler 只做一件事——**定时驱动过期重估**：

```
EventScheduler::schedule_repeat("approval-due-reap", "approval.due.check", interval)
  → 订阅者（联盟调度侧）调 approval_gate::reap_expired(now, ttl)
```

（approval_gate 的 TTL 过期→Expired 判定已有；EventScheduler 补齐"周期性触发"来源。）

### 5.3 既有审批的可扩展点（如需，向现有实现增补而非新建 crate）

- 创建者撤回（Withdrawn）：process_engine 无撤回语义，需要时在 `ProcessInstance` 增 `withdraw` 方法（小改动，不新建 crate）
- 审计时间线统一展示：前端已有 AdminHitl，扩展其展示 ApprovalRecord 时间线即可

## 六、架构模块化归一化

### 6.1 归一化口径

1. **事件总线单一权威**：`mox-event-core` 为唯一事件基础设施 crate；其余实现定位为**域内适配**，渐进对齐。
2. **定时触发单一实现**：EventScheduler 为唯一"事件定时触发"抽象；各服务不得自建 cron 循环（既有 synchronizer 的具体定时后续可对齐到本抽象）。
3. **审批单一权威**：流程级 = process_engine；节点级 = approval_gate；UI = AdminHitl。**不再新建审批 crate**。
4. **Crate 分层**：共享基础设施（event-core 等）位于 `platform/shared/`。

### 6.2 现有 event_bus 差距分析（2026-09-27 实测）

| 维度 | `mox-event-core`（权威） | `mox-flow-unified-platform/src/event_bus.rs` | `mox-platform-orchestrator-svc/src/cordis/event_bus.rs` |
|------|------|------|------|
| 事件模型 | 泛型 Event + 字符串 EventType + EventMetadata | `PlatformEvent`：枚举类型 + tenant_id + 源体系 + 事件溯源 | `Event` 枚举（Profile/Bundle/Turn）+ domain 双层路由 |
| 路由 | 订阅表 + 通配符匹配 | 精确 EventType 分组 | domain → event_type 精确 |
| 发布 | 同步/异步 + 并发 + 超时 | 仅同步、无并发/超时 | async 外壳同步分发 |
| 容错 | DLQ + 重试 | 无 | 无 |
| 定位 | 基础设施 | 域内适配（六大归一化体系联动） | 域内适配（OUSS 三联盟） |

**渐进对齐路径（不破坏现有调用方）**：① 域事件 `impl mox_event_core::Event`（类型映射 name()）→ ② 路由内核替换为 MemoryEventBus（保留业务 facade）→ ③ 超时/DLQ 依赖共享层 → ④ arch-test 同名 EventBus 收敛为"facade + 基础设施权威"。

**第①步已落地（2026-09-27，纯加法零破坏）**：
- `mox-flow-unified-platform/src/event_bridge.rs`：`PlatformEvent` impl 共享 `Event` trait（`event_type()` → `EventType::new(self.event_type.name())`，snake_case 全名）+ 2 测试（映射 + 默认序列化）✓
- `mox-platform-orchestrator-svc/src/cordis/event_bridge.rs`：cordis `Event` 枚举 impl 共享 `Event` trait（`domain.action` 点分命名，天然支持 `profile.*` 通配符）+ 2 测试 ✓
- 两处桥接均保留域内固有 `event_type()`（String，向后兼容），共享总线侧用 `mox_event_core::Event::event_type(&e)` 调用
- 第②③步（路由内核替换）涉及域内同步 API 破坏（如 flow `publish → Vec<EventHandleResult>`），需业务域排期渐进执行

### 6.3 链路集成验证（EventScheduler → EventBus → 订阅者）

`mox-event-core/tests/scheduler_event_integration.rs`（2 用例）：
- ① once 闭环：`schedule_once(approval.due.check, 30ms)` → 总线 → 订阅者计数 +1；`active_tasks` 归 0
- ② repeat + cancel：周期任务多次触发；`cancel` 后计数不再增长；`cancelled_total=1`

## 七、验证记录（2026-09-27 实跑）

- `cargo test -p mox-event-core`：**34/34 单元 + 2/2 集成 + doc-tests 3/3 通过**
- `cargo clippy -p mox-event-core --all-targets`：**0 warning**
- `cargo build`（默认 members 27 个）：**通过**（改动共享 crate 未波及依赖）
- **桥接落地**：`cargo test -p mox-flow-unified-platform event_bridge` 2/2 ✓；`cargo test -p mox-platform-orchestrator-svc --lib event_bridge` 2/2 ✓；`cargo check -p mox-flow-unified-platform` / `-p mox-platform-orchestrator-svc` 均通过（归一化第①步）
- **撤销记录**：V1.0 曾新建 `mox-approval-core`（审批状态机），2026-09-27 轮子盘点发现与 `process_engine`（四策略更强）和 `approval_gate`（TTL/Expired）重复且弱于现有——**已删除**（代码 + workspace 登记 + 测试），独有价值以 §5.3 扩展点形式保留
- 修复记录：EventBus 泛型方法 → 非 object-safe（`Arc<dyn EventBus>` 弃用，改泛型 `EventScheduler<B: EventBus>`）；取消语义修正（JoinHandle drop 仅 detach，须 `abort()`）；once 任务自然结束自动移出活跃表（tasks 改 `Arc<RwLock<HashMap>>`）

## 八、服务层与前端接缝（复用现有，不新建）

- **审批 API**：不新建 `/api/approval/*`；流程审批走 process_engine 既有能力（实例/步骤/审批记录查询），网关侧按现有 alliance/flow 路由挂载方式接入
- **前端**：审批 UI 复用 `AdminHitl.vue`（/admin/hitl）；定时任务管理展示（stats/cancel）如需，走既有 admin 面板模块化路径扩展，遵循前端模块治理规范（barrel/别名门禁）

## 九、轮子盘点与决策记录（2026-09-27）

| 新建/复用 | 对象 | 决策 |
|------|------|------|
| 复用 | 事件总线（mox-event-core 既有 10+ 测试） | ✅ 未重复开发 |
| 新建 | EventScheduler（定时触发） | ✅ 保留——仓库唯一"事件定时触发"抽象，与 3 个既有 scheduler 职责均不同 |
| ~~新建~~ | ~~mox-approval-core~~ | ❌ 已删除——与 process_engine / approval_gate 重复且弱于现有 |
| 复用 | 流程审批 / DAG 人审门 / HITL 面板 | ✅ 权威归属确认 |

*相关：[CURRENT-ARCHITECTURE.md](../expert-alliance/CURRENT-ARCHITECTURE.md)（alliance leadership/存储约定）· [前端模块治理规范](../architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md) · [流程能力历史说明](docs/modules/business-process-flows.md#1-概述)（旧实现资料，不作当前联盟事实）*

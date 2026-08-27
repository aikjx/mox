# 璇玑 RelGraph · 需求规格说明书（SRS）

> **文档类型**：软件需求规格（IEEE 830 风格，企业级）
> **文档版本**：v1.1 (ENT) · 最后更新 2026-08-23
> **权威链**：🟢 L0 第一级 → [`18-全域顶层总设计-三联盟模式-V1.0.md`](18-全域顶层总设计-三联盟模式-V1.0.md)（TOP-MASTER）。本文为 L2 第三级（需求层），不得与 18 冲突。
> **主责联盟**：产品联盟（收口需求） · 联合对齐：算法联盟 + 开发联盟
> **适用范围（与代码事实一致 · 路径零老化）**：
> - `platform/domains/mox-system/`（协作治理域：成员 / 任务 / 权限 / 通信 / 多方言 repo fail-fast）
> - `platform/domains/mox-expert/`（璇玑融合引擎域：双璇玑十四维治理 / 归一化 / 裁决 / ⛨最高权限验证 / 审计三汇 / RBAC）
> - `platform/gateway/runtime/`（Rust 聚合网关：四端点 `/ai/engine/{process,analyze,capabilities,metrics}`、AC-10 路由语义、RBAC 中间件、Node Sidecar 通信）
> - `frontend-ui/`（用户端单应用 + `/admin` 系统管理区 5 面板，共 28 视图）
> **权威来源**：业务规则以 `docs/modules/mox-expert-business-requirements.md` 为基线；本文将其提升为结构化 SRS 并补充可度量 NFR、验收与文档级 ADR 注册。
>
> **编写原则**：每条需求**必须可验证**——要么映射到代码位置，要么标记 `GAP` 并附验收断言。新增需求须先过 18 TOP-MASTER 的三联盟铁律，再补本 SRS。

---

## 1. 引言

### 1.1 目的

定义「**璇玑 RelGraph**」（全域归一化知识图谱协同平台）的功能性与非功能性需求，作为架构设计、编码、测试与验收的唯一需求基准。系统承载**三联盟协同**的三条互补业务主线（见 18 TOP-MASTER §一~§四）：

- **产品联盟 · 需求治理主线**：需求判重（P9 闸门） → 需求入图谱 Bind 六维 → 需求追踪矩阵发布。
- **璇玑（协作治理）**：组建璇玑 → 专家入璇玑 → 任务派发 → 协同推进 → 全程留痕（`mox-system`）。
- **璇玑融合（算子融合）**：归一化 → 十四维会诊 → 冲突消解 → 治理裁决 → 产出可复用优化算子并上架（`mox-expert`）。

### 1.2 术语

| 术语 | 含义 |
|------|------|
| 璇玑（Mox） | 多租户隔离的协作单元，权限与作用域的最小边界 |
| 成员（Member） | 璇玑内的参与者，持有角色绑定（RoleBinding） |
| 任务（Task） | 受状态机约束的工作单元，可被分派给多名专家 |
| 角色（Role） | Admin/Coordinator/Expert/Member/Auditor，含继承 |
| 作用域（Scope） | 权限生效范围：Global / Mox / Task |
| 领域事件（DomainEvent） | 写操作产出的不可变事实，是审计与实时推送的唯一数据源 |
| 反应器（Reactor） | 订阅事件总线、将事件转译为系统消息与通知的组件 |
| 璇玑（Mox） | `mox-expert` 的算法最高权限校验，不可被治理覆盖 |

---

## 2. 干系人与用户类

| 用户类 | 业务目标 | 关键关注 |
|--------|----------|----------|
| 璇玑管理员 MoxAdmin | 治理璇玑、任免、审计 | 全局管控、合规留痕 |
| 协调员 Coordinator | 任务运营、邀请专家 | 高效派发、不越权管理成员 |
| 专家 Expert | 承接并推进被分配的任务 | 最小权限、协作顺畅 |
| 普通成员 Member | 受限参与、评论 | 只读自身相关、不被赋予越权 |
| 审计员 Auditor | 全局只读审计 | 不可抵赖、可追溯 |
| 系统/集成方 | 通过 API/事件消费 | 契约稳定、幂等 |

---

## 3. 范围

### 3.1 In Scope（本期）

1. 成员生命周期管理（邀请/激活/停权/退出/复活防护）。
2. 任务协作（立项/派发/推进/评论/子任务/依赖）。
3. 权限分配（RBAC 5 角色 + 继承 + 14 原子权限 + 三级作用域 + 所有权权限）。
4. 通信机制（璇玑/任务/私信频道、消息、通知、WebSocket 实时推送）。
5. 审计留痕（领域事件、鉴权拒绝留痕、审计查询）。
6. 璇玑融合治理（双璇玑十四维、一票否决、不变式验证、可视化）。

### 3.2 Out of Scope（本期边界，入路线图）

- ~~持久化存储~~（**已出范围**：I-01/I-02 已落地，SQLite/PostgreSQL/MySQL 三后端，见 `02` §7.4）。
- 可观测性指标采集（NFR-08，设计就绪未落地）。
- 配额/限流（NFR-09）。
- ABAC 属性权限、租户维度策略分层、审计链 WAL 重放。

---

## 4. 功能需求（FR）

### 4.1 成员管理 `FR-MEM`

| ID | 需求 | 优先级 | 状态 | 落点 |
|----|------|:--:|:--:|------|
| FR-MEM-01 | 创建璇玑并确立首位管理员（状态直接 Active） | P0 | ✅ | orchestrator.bootstrap |
| FR-MEM-02 | 邀请专家入璇玑，默认授 `Expert@Mox`（最小权限） | P0 | ✅ | services.invite |
| FR-MEM-03 | 同璇玑同 email 邀请幂等（忽略大小写/空格）→ Conflict | P1 | ✅ | services.invite |
| FR-MEM-04 | 成员激活 `Invited→Active` | P0 | ✅ | services.activate |
| FR-MEM-05 | 成员生命周期受状态机约束（Left 终态不可复活） | P0 | ✅ | model.MemberStatus |
| FR-MEM-06 | 仅 Active 成员可承接任务 | P0 | ✅ | model.can_take_task |
| FR-MEM-07 | 管理员可暂停/移除成员（member:manage） | P0 | ✅ | services.set_status |
| FR-MEM-08 | 成员专长档案与等级（Tier）管理 | P2 | ✅ | model.Member |

### 4.2 任务协作 `FR-TASK`

| ID | 需求 | 优先级 | 状态 | 落点 |
|----|------|:--:|:--:|------|
| FR-TASK-01 | 创建任务，初始 `Draft`，`assignees` 必须为空 | P1 | ✅ | services.create |
| FR-TASK-02 | 分派任务，写入 assignees 并 `Draft→Assigned`，全量覆盖语义 | P1 | ✅ | services.assign |
| FR-TASK-03 | 分派身份三重校验（存在/同璇玑/Active） | P0 | ✅ | services.validate_assignees |
| FR-TASK-04 | 任务状态机校验，非法迁移→InvalidState | P0 | ✅ | model.TaskStatus |
| FR-TASK-05 | 进入 `Done` 需通过 DoD 门禁（子任务全完成 ∧ 前置依赖全 Done） | P0 | ✅ | services.check_done_gate |
| FR-TASK-06 | 依赖图须为 DAG（拒自依赖/成环/跨璇玑） | P1 | ✅ | services.add_dependency |
| FR-TASK-07 | 终态（Done/Cancelled）不可迁出 | P1 | ✅ | model 终态表 |
| FR-TASK-08 | 评论写入任务频道并双事件 | P1 | ✅ | services.comment |
| FR-TASK-09 | 分派自动建任务频道并拉入被分派者 | P1 | ✅ | orchestrator 反应器 |

### 4.3 权限分配 `FR-PERM`

| ID | 需求 | 优先级 | 状态 | 落点 |
|----|------|:--:|:--:|------|
| FR-PERM-01 | 5 角色 + 继承链（Coordinator→Expert→Member） | P0 | ✅ | rbac.Role |
| FR-PERM-02 | 14 原子权限，含 `*Own` 所有权权限 | P0 | ✅ | rbac.Permission |
| FR-PERM-03 | 三级作用域 Global/Mox/Task | P0 | ✅ | rbac.Scope |
| FR-PERM-04 | 所有权权限要求调用者在 assignees 中 | P0 | ✅ | rbac.authorize |
| FR-PERM-05 | 所有写操作先经统一 `require()` 鉴权 | P0 | ✅ | orchestrator.require |
| FR-PERM-06 | 鉴权失败留痕（试探式鉴权不落审计，避免噪声） | P1 | ✅ | orchestrator.require + AuthzDenied |

### 4.4 通信机制 `FR-COMM`

| ID | 需求 | 优先级 | 状态 | 落点 |
|----|------|:--:|:--:|------|
| FR-COMM-01 | 璇玑大厅（公共频道）惰性创建 | P1 | ✅ | orchestrator.bootstrap |
| FR-COMM-02 | 璇玑/任务/私信三类频道 | P2 | ✅ | model.Channel |
| FR-COMM-03 | 消息发送与存储 | P1 | ✅ | comm.send_message |
| FR-COMM-04 | 事件→系统消息 + 成员通知（反应器） | P0 | ✅ | orchestrator.Reactor |
| FR-COMM-05 | 通知经 WebSocket 实时推送 | P1 | ✅ | server.ws |
| FR-COMM-06 | 反应器幂等，事件重放无副作用 | P1 | ✅ | orchestrator.Reactor.handle |

### 4.5 审计与合规 `FR-AUDIT`

| ID | 需求 | 优先级 | 状态 | 落点 |
|----|------|:--:|:--:|------|
| FR-AUDIT-01 | 所有领域写操作发布领域事件（9 类） | P0 | ✅ | event.DomainEvent |
| FR-AUDIT-02 | 审计查询受 `audit:view` 约束 | P1 | ✅ | orchestrator.query_audit |
| FR-AUDIT-03 | 融合治理一票否决（安全/合规专家阻断即不可发布） | P0 | ✅ | govern.rs |
| FR-AUDIT-04 | 优化产物经不变式验证方可发布 | P0 | ✅ | verify.rs |

### 4.6 璇玑融合 `FR-FUSE`

| ID | 需求 | 优先级 | 状态 | 落点 |
|----|------|:--:|:--:|------|
| FR-FUSE-01 | 双璇玑十四维治理（业务七维 + 开发七维 CodeIR 驱动） | P0 | ✅ | mox-expert |
| FR-FUSE-02 | 归一化 IR → 七维会诊 → 冲突消解 → 治理裁决 | P0 | ✅ | pipeline |
| FR-FUSE-03 | 璇玑最高权限校验（不可覆盖） | P0 | ✅ | verify.rs |
| FR-FUSE-04 | 优化结果可解释（剪伪依赖数/加速比/算力压缩比） | P1 | ✅ | bench.rs |
| FR-FUSE-05 | 双验收（组织 Done ∧ 技术验证）方可上架 | P1 | 📋 部分 | 路线图 |

> **融合治理全维需求（权威基准 AA-STD-V1.0）**：FR-FUSE 仅作摘要，融合域的**完整企业级需求**
> （8 阶段 S1~S8 / 4 道强制闸门 G0~G3 / 双璇玑十四维专家矩阵 / 优先级权重）以
> `璇玑-全维需求业务处理流程图-归一化企业级.md` 为唯一事实基准，并已在
> `06-requirements-architecture-map.md` §2 逐条映射到架构视图与代码模块。
> 简言之：需求接入(S1)→归一化(S2,过 **G0**)→双璇玑并行诊断(S3)→归一化裁决(S4,过 **G1**)→
> flow-ai 最优求解(S5)→⛨璇玑验证(S6,过 **G2** 最高否决)→治理闸门(S7,过 **G3**)→出码(S8)。
> 任意闸门拒绝即阻断出码，无降级旁路。

---

## 5. 非功能需求（NFR，可度量）

| ID | 类别 | 需求 | 目标值 | 现状 |
|----|------|------|--------|------|
| NFR-01 | 多租户隔离 | 所有查询按 mox_id 过滤；跨璇玑引用拒绝 | 100% 拦截 | 查询侧 ✅；写入侧已修 GAP-2 |
| NFR-02 | 安全 | 最小权限：受邀成员仅 `Expert@单璇玑` | 默认满足 | ✅ |
| NFR-03 | 可移植性 | 存储层接口/实现分离；`trait Repository` 支持 SQLite/PostgreSQL/MySQL 零代码切换 | 接口稳定 + 多后端可选 | ✅ 三后端已落地（`02` §7.4） |
| NFR-04 | 解耦 | 领域动作与通信经事件总线解耦 | 全量覆盖 | ✅ |
| NFR-05 | 代码质量 | crate `cargo clippy` 零告警 | 0 warning | ✅ |
| NFR-06 | 可回归 | 全量单测/集成测试一键回归 | 全绿 | ✅ 644 passed / 0 failed / 6 ignored（2026-08-18 实测 `cargo test --workspace`） |
| NFR-07 | 性能 | 融合优化可复现基准，加速比≥2.32× | ≥2.32× | ✅ 实测 2.32× |
| NFR-08 | 可观测性 | 关键路径指标（鉴权拒绝率/状态迁移分布/优化耗时） | 指标可采集 | 📋 未落地 |
| NFR-09 | 配额 | 成员数/任务数/依赖深度上限 | 可配置 | 📋 未落地 |
| NFR-10 | 可用性 | 服务健康探针 `/api/health` | 99.9% | ✅ HTTP 层 |
| NFR-11 | 一致性 | 事件反应器幂等，重放无副作用 | 幂等 | ✅ 声明+测试 |
| NFR-12 | 安全-传输 | 令牌经 `X-Auth-Token`/Bearer 传递，缺失→401 | 100% | ✅ |

---

## 6. 约束与假设

- **约束 C-1**：所有写操作必须经由 `require()` 鉴权（BR-02），bootstrap 为唯一例外。
- **约束 C-2**：`assignees` 为可信集合（经 GAP-2 修复），所有权类权限前提成立。
- **约束 C-3**：生产部署必须设 `MOX_PERSIST=true` 且 `MOX_STRICT_PERSIST=true`（fail-fast），否则连库失败会静默回退内存态导致数据不可恢复（见 `02` §7.4）。
- **假设 A-1**：内存态（`MOX_PERSIST=false`）仅用于测试/演示，重启即失忆；生产依 NFR-03 使用 SQLite/PostgreSQL/MySQL 持久化。
- **假设 A-2**：LLM 可选；未配置时企业流程 fail-closed 走拒绝路径（合规默认拒绝）。

---

## 7. 需求追踪矩阵（需求 → 设计 → 测试）

| 需求 | 设计章节 | 测试固化 | 验收断言 |
|------|----------|----------|----------|
| FR-MEM-03 | `03-design` §成员服务 | `br04_invite_is_idempotent_per_email` | 重复 invite→Conflict，成员数不增 |
| FR-MEM-05 | `03-design` §成员 FSM | `br21_member_status_machine_enforced` | Left→Active 拒绝；全链路通过 |
| FR-TASK-03 | `03-design` §分派校验 | `br07_assign_validates_assignee_identity` | 不存在/跨璇玑/非Active→拒绝 |
| FR-TASK-05 | `03-design` §DoD 门禁 | `br10_done_gate_blocks_incomplete_work` | 子任务/依赖未完成→InvalidState |
| FR-TASK-06 | `03-design` §DAG | `br11_dependency_graph_is_dag` | 自依赖/环/跨璇玑拒绝 |
| FR-PERM-06 | `03-design` §审计 | `br18_authz_denial_is_audited` | 越权恰好 1 条记录；正常 0 条 |

> 完整 21 条业务规则追踪见 `docs/modules/mox-expert-business-requirements.md` §6。

---

## 8. 验收标准

1. **功能验收**：FR-MEM/FR-TASK/FR-PERM/FR-COMM/FR-AUDIT 中标记为 ✅ 的项均有正向+负向测试。
2. **回归验收**：`cargo test -p mox-system -p mox-expert -p flow-ai` 全绿。
3. **静态验收**：相关 crate `cargo clippy` 零告警。
4. **安全验收**：跨租户权限提升路径（GAP-2 攻击链）被阻断；鉴权失败可审计。
5. **性能验收**：`mox bench` 加速比 ≥ 2.32×。

---

## 9. 文档级架构决策注册（ADR-DOC-001 ~ 012 · 对齐 18 TOP-MASTER）

> 本文档集级 ADR 是「对 TOP-MASTER（18）第一级权威的变更与治理决策」的唯一登记点。所有 18 的重大变更须在此取得 ADR 编号再执行。

| ADR 编号 | 决策标题 | 背景/问题 | 决策内容 | 影响范围 | 主责联盟 | 状态 |
|----------|----------|-----------|----------|----------|:--:|:--:|
| ADR-DOC-001 | **L0 第一级权威**：`18-全域顶层总设计-三联盟模式-V1.0.md` 为统摄全局最高级文档 | 原无单一 TOP-MASTER，文档集权威互相矛盾（01 vs 02 vs 14） | 所有 00~17 文档（及任何新增文档）标题、术语、路径、架构声明、流程编号、算法选择均以 18 为最高准则；下级文档声明显式权威链 | 全部 19 份文档 + 双 README + GLOSSARY | 三联盟签署 | ✅ 生效 |
| ADR-DOC-002 | **三联盟模式**：产品 / 算法 / 开发 三联盟协同闭环作为全系统组织模型 | 原只有"开发联盟"、"算法联盟"被提及，产品侧治理缺位 | 每文档强制登记「主责联盟」；每业务流程强制登记「流经联盟」；四归三连（需求→架构→业务→文档四归一；联盟/流程/代码三连） | 04 BP-01~10、06 映射矩阵、07 铁律、08 自动化责任列 | 三联盟联合 | ✅ 生效 |
| ADR-DOC-003 | **六层金字塔架构**：L6 产品应用→L5 业务流程→L4 图谱核心→L3 算法推理→L2 Rust 自研底座→L1 部署运维 | 原七视图架构没有"统一分层金字塔"作为跨文档对齐锚点 | 所有架构文档（02 / 14 / architecture.md）的视图与章节须显式标注其所属 L 层级 | 02-architecture §0、14 §3、AA-STD 分层 | 算法联盟 + 开发联盟 | ✅ 生效 |
| ADR-DOC-004 | **八层知识图谱（L0~L7）× 14 节点族 × 19 边族**作为统一图模型 | 原关图 GR-STD 仅 12 类节点/7 类边，不足以承载 REQ→FUN→BIZ→ALG→TSK→COD 的全链路 | 图谱统一分层：L0 需求根层 / L1 业务层 / L2 功能层 / L3 算法层 / L4 实现层 / L5 资源层 / L6 治理层 / L7 表现层 | 关图建模、kg-hub、graph-algorithms | 算法联盟 | ✅ 生效 |
| ADR-DOC-005 | **前后端与路径统一**：frontend/ → frontend-ui/、crates/ → platform/domains/、backend/ → edge-node（M0） | 路径老化：README 仍引用 frontend/、crates/、backend-node 命名含混 | M0 阶段完成三路径归一化：`frontend-ui/` 单应用（+ /admin 5 面板）、`platform/domains/` 15 crate、`platform/edge-node/` 瘦身 4 文件 | 双 README、02 §3.2、00-INDEX §适用系统 | 开发联盟 | ✅ M0 执行中 |
| ADR-DOC-006 | **10 大标准业务流程（BP-01~10）** 替换原 8 BP | 原 BP 缺少 P9 判重与文档治理两条标准链路 | 新增 BP-9「P9 先判重后立项 / 图谱注入」与 BP-10「文档同步归一 / ADR 治理」，共 10 BP，每 BP 强制 6 字段（编号/主责联盟/前置/核心步骤/闸门规则/审计产物） | 04-business-processing §3、06 映射 | 产品联盟 + 开发联盟 | ✅ 生效 |
| ADR-DOC-007 | **八大算法家族标准化**：禁止自研等价实现 | 社区检测（CNM vs LPA）、介数（Brandes 2001 vs BFS 近似）等实现选择在文档中混用 | 算法家族固定：①CNM 社区检测 ②Brandes 2001 介数 ③Harmonic 紧密中心性 ④PageRank（含转置图）⑤激活扩散（个性化 PageRank d=0.85，30 轮）⑥RRF 结果融合 ⑦CEM 交叉熵优化 ⑧CPM·RCPSP 调度；其他实现一律废弃 | graph-algorithms / mox-expert / optimizer | 算法联盟 | ✅ 生效 |
| ADR-DOC-008 | **9 里程碑（M0~M8）+ 三级验收 L0/L1/L2**作为统一排期口径 | 原路线图（05）中「迭代 1~4+」粒度不一致、无 L0/L1/L2 验收门槛 | M0 全域归一化 / M1 业务闭环 / M2 算法核 / M3 AI 编排统一 / M4 存储分布式 / M5 可观测 HA / M6 多云 / M7 生态 / M8 自治；三级验收：L0 单元全绿、L1 集成全过、L2 SLO 达标 | 05-iteration-roadmap §7+、18 §八 | 三联盟联合 | ✅ 生效 |
| ADR-DOC-009 | **统一 AI 入口**：Rust Gateway 四端点 `/ai/engine/{process,analyze,capabilities,metrics}`，路由语义 AC-10 | 原 AI 查询散落在 Node sidecar、Caomei、ChatView 多处，语义不统一 | 所有 AI 能力路由至 Rust Gateway；process=自动意图识别→能力路由，analyze=显式执行，capabilities=能力矩阵自描述，metrics=成功率/降级率/延迟；AC-10：静态路由优先于参数化路由 | runtime（platform/gateway/runtime）、ai-agent | 开发联盟 + 算法联盟 | ✅ 生效 |
| ADR-DOC-010 | **四归三连铁律**：所有改动必须经需求↔架构↔业务↔文档四归一，且联盟、流程、代码三连对应 | 历史存在代码改动未同步文档、联盟决策未落流程的缺口 | 改动前先补 06 映射行；改动后必过 08 自动化 8 步闭环；PR CI 强制勾选「四归三连完成」复选框 | CI 模板、PR checklist、07 §三联盟铁规 | 三联盟联合 | ✅ 生效 |
| ADR-DOC-011 | **命名统一为「璇玑 RelGraph」**，禁止「璇玑系统 / 璇玑子系统 / OUS 子系统」混用 | 文档间命名漂移（璇玑系统 / OUS / 关图治理平台…）导致对外口径混乱 | 项目对外产品名统一为「璇玑 RelGraph」；OUS 保留为父系统底层算子代号；所有文档标题、首页 README、docs/README 统一替换 | GLOSSARY §1、双 README、00-INDEX、01/02/04/05/14 标题 | 产品联盟 | ✅ 生效 |
| ADR-DOC-012 | **ADR 变更治理**：18 TOP-MASTER 自身的任何变更必须先走 ADR-DOC-NEW 申请流程 | TOP-MASTER 改动未留痕会导致下级文档批量失配 | 18 每章改动须先在本 ADR 表新增行，取得编号与三联盟会签后再改正文；00-INDEX 留痕「最后更新」 | 18-全域顶层总设计、00-INDEX 变更记录 | 三联盟签署 | ✅ 生效 |

---

## 10. 变更记录

| 版本 | 说明 |
|------|------|
| v1.1 (ENT) | **三联盟对齐升级**：标题 / 术语改为「璇玑 RelGraph」；权威链显式指向 18 TOP-MASTER；适用范围路径全部刷新（platform/domains/、platform/gateway/runtime/、frontend-ui/）；新增「§9 文档级 ADR-DOC-001~012 决策注册」，与 18 §十二 ADR 治理一一对应。三联盟主责联盟字段全面引入。 |
| v1.0 (ENT) | 首版企业级 SRS：结构化 FR/NFR、可度量目标、追踪矩阵、验收标准；与 `docs/modules/mox-expert-business-requirements.md` 对齐并提升格式标准。 |

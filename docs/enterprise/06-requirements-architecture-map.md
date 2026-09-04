# 璇玑 RelGraph · 企业级需求—架构全景映射图（含三联盟映射矩阵）

> **文档类型**：需求—架构可追溯映射（Traceability Master Map · 四归三连强制对齐）
> **文档版本**：v1.1 (ENT) · 最后更新 2026-08-23
> **权威链**：🟢 L0 → [`18-全域顶层总设计-三联盟模式-V1.0.md`](18-全域顶层总设计-三联盟模式-V1.0.md)（TOP-MASTER）。
> **主责联盟**：三联盟联合（产品=需求侧映射 · 算法=图谱/算法侧映射 · 开发=代码侧映射）
> **配套**：`00-INDEX`（治理）、`01-requirements`（需求）、`02-architecture`（架构）、`03-design`（设计）、`04-business-processing`（业务）
> **权威需求基准**：
> - 协作治理域需求 → `docs/modules/mox-expert-business-requirements.md`（BR-01…BR-21）
> - 融合治理域mox 模块化系统架构需求 → `璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级.md`（**AA-STD-V1.0**，唯一归一化事实基准）
>
> **本文目的**：把散落在两套需求基准里的「企业级需求」，逐条映射到**架构视图 / 模块 / 代码落点 / 三联盟责任**，
> 使「需求 → 架构 → 设计 → 业务 → 联盟 → 代码」形成一条可审计的明确闭环（四归三连对齐 ADR-DOC-010）。

---

## 0. 企业级需求总览（两大域）

系统企业级需求分为**互补的两大域**，各自的"需求事实基准"与"架构承载"不同：

| 域 | 业务语义 | 需求基准 | 架构承载 | 关键文档 |
|----|----------|----------|----------|----------|
| **协作治理域** | 谁来做、做什么、是否通过（组织决策） | `docs/modules/mox-expert-business-requirements.md`（BR-01…BR-21） | `mox-system`（02-architecture 七视图） | 01/03/04 |
| **融合治理域** | 怎么做得更快、是否可信（技术决策） | `璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级.md`（AA-STD-V1.0，8 阶段 / 4 闸门 / 双璇玑十四维） | `mox-expert` + `primiflow-fusion`（治理闸门） | 本文 §2 |

> **为什么分两套基准**：协作治理是"人—组织"的权限/状态/审计问题，用 BR 规则集描述；
> 融合治理是"图—算法—治理"的归一化问题，用 AA-STD 的 8 阶段流程 + 4 闸门描述。二者在
> **BP-6 璇玑融合优化 / BP-7 交付验收** 处汇合（组织验收 ∧ 技术验收，见 BR-16）。

---

## 1. 协作治理域：需求 → 架构 → 模块 → 代码 映射

> 下列 FR 取自 `01-requirements.md` §4，逐条显式绑定到 `02-architecture.md` 的视图与 `03-design.md` 的模块。

| 功能需求 | 架构视图落点（02） | 设计模块（03） | 业务规则 | 代码落点（`crates/mox-system/src/`） |
|----------|--------------------|----------------|----------|-------------------------------------------|
| FR-MEM-01 建璇玑+首位管理员 | 业务视图 §1.1 / 安全视图 §5.2 | §6.1 MemberService | BR-01 | `orchestrator.rs` `bootstrap` |
| FR-MEM-02 邀请专家(最小权限) | 安全视图 §5.2 | §6.1 `invite` | BR-03 | `services.rs::MemberService::invite` |
| FR-MEM-03 同 email 幂等 | 信息视图 §2.1 | §6.1 | BR-04 (GAP-1) | `br04_*` 测试固化 |
| FR-MEM-04 激活 | 信息视图 §2.2 | §5 成员 FSM | BR-05 | `model.rs::MemberStatus` |
| FR-MEM-05 终态不可复活 | 信息/安全视图 | §5 成员 FSM | BR-21 (GAP-6) | `model.rs::can_transition` |
| FR-MEM-06 仅 Active 可承接 | 安全视图 §5.2 | §6.1 `can_take_task` | BR-05 | `services.rs::can_take_task` |
| FR-MEM-07 管理员停/移除 | 安全视图 §5.2 | §6.1 `set_status` | BR-02 | `services.rs::set_status` |
| FR-TASK-01 立项(空分派) | 应用视图 §3 | §6.2 `create` | BR-06 | `services.rs::TaskService::create` |
| FR-TASK-02 分派(全量覆盖) | 应用视图 §3.3 | §6.2 `assign` | BR-08 | `services.rs::assign` |
| FR-TASK-03 分派三重校验 | **安全视图 §5.3（核心）** | §6.3 `validate_assignees` | BR-07 (GAP-2, P0) | `services.rs::validate_assignees` |
| FR-TASK-04 状态机校验 | 信息视图 §2.2 | §4 任务 FSM | BR-09 | `model.rs::TaskStatus` |
| FR-TASK-05 DoD 门禁 | 业务视图 §1.2 | §4.2 `check_done_gate` | BR-10 (GAP-3, P0) | `services.rs::check_done_gate` |
| FR-TASK-06 依赖 DAG | 信息视图 §2.2 | §4.3 `add_dependency` | BR-11 (GAP-4, P1) | `services.rs::add_dependency` + `reaches` |
| FR-TASK-07 终态不可迁出 | 信息视图 | §4.1 | BR-12 | `model.rs` 终态表 |
| FR-TASK-08 评论+双事件 | 集成视图 §6.2 | §6.2 `comment` | BR-19 | `event.rs::DomainEvent` |
| FR-TASK-09 分派建频道 | 集成视图 §6.1 | §7.2 Reactor | BR-19 | `orchestrator.rs::Reactor` |
| FR-PERM-01 5 角色+继承 | 安全视图 §5.2 | §3.1 矩阵 | BR-02 | `rbac.rs::Role` |
| FR-PERM-02 14 原子权限 | 安全视图 §5.2 | §3.1 | — | `rbac.rs::Permission` |
| FR-PERM-03 三级作用域 | 安全视图 §5.2 | §3.2 算法 | — | `rbac.rs::Scope` |
| FR-PERM-04 所有权权限 | 安全视图 §5.3 | §3.2 / §6.3 | BR-07 | `rbac.rs::authorize` |
| FR-PERM-05 统一 `require()` | 安全视图 §5.1 / ADR-01 | §7.1 门面 | BR-02 | `orchestrator.rs::require` |
| FR-PERM-06 鉴权留痕(非试探) | 安全视图 §5.3 / ADR-05 | §3.3 两段式 | BR-18 (GAP-5, P1) | `orchestrator.rs::require` + `AuthzDenied` |
| FR-COMM-01 璇玑大厅 | 应用视图 §3 | §6.5 `ensure_channel` | — | `comm.rs` |
| FR-COMM-04 事件→消息+通知 | 集成视图 §6.2 | §7.2 Reactor | BR-19/BR-20 | `orchestrator.rs::Reactor` |
| FR-COMM-05 WS 实时推送 | 集成视图 §6.1 | §6.5 `notify` | BR-20 | `server.rs::ws` |
| FR-AUDIT-01 领域事件 | 集成视图 §6.2 | §7.2 | BR-19 | `event.rs` |
| FR-AUDIT-02 审计查询 | 安全视图 §5.3 | §7.1 | BR-18 | `orchestrator.rs::query_audit` |
| FR-AUDIT-03 一票否决 | 安全视图 §5.1 | §7（govern） | BR-13 | `crates/mox-expert/src/govern.rs` |
| FR-AUDIT-04 不变式验证 | 安全视图 §5.1 | §7（verify） | BR-14 | `crates/mox-expert/src/verify.rs` |

**非功能需求 → 架构视图映射**（取自 02 §9 跨视图 NFR 落地表，补全景）

| NFR | 业务 | 应用 | 技术 | 安全 | 状态 |
|-----|------|------|------|------|------|
| NFR-01 多租户 | 璇玑边界 | 查询过滤 | Store 隔离 | 分派校验 | ✅ |
| NFR-02 最小权限 | — | — | — | RBAC 默认 | ✅ |
| NFR-03 持久化 | — | Store 接口 | SQLite/PG/MySQL 三后端（`02` §7.4） | — | ✅ I-01 |
| NFR-04 解耦 | 事件闭环 | Reactor | broadcast | 审计独立 | ✅ |
| NFR-05 clippy | — | — | CI | — | ✅ |
| NFR-06 回归 | — | 测试 | — | — | ✅ |
| NFR-07 性能 | — | bench | — | — | ✅ 2.32× |
| NFR-08 可观测 | 指标定义 | 埋点 | tracing | — | ✅ I-04 |
| NFR-09 配额 | — | 配置 | — | — | ✅ I-03 |
| NFR-10 可用性 | — | `/health` | — | — | ✅ |
| NFR-11 一致性 | 幂等 | Reactor 幂等 | broadcast | 事件不可变 | ✅ |
| NFR-12 传输安全 | — | 中间件 | — | 令牌 401 | ✅ |

---

## 2. 融合治理域（mox 模块化系统架构需求 AA-STD-V1.0）：需求 → 架构 → 模块 映射

> 权威基准：`璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级.md`（AA-STD-V1.0）。
> 架构承载：`crates/mox-expert`（双璇玑十四维 / 归一化 / 裁决 / 璇玑）+ `crates/primiflow-fusion`（治理闸门 / 守恒 / 零孤儿）。

### 2.1 八阶段需求 → 架构落点

| 阶段 | mox 模块化系统架构需求（AA-STD） | 架构落点 | 模块 / 代码 | 对应闸门 |
|------|--------------------|----------|-------------|----------|
| S1 需求接入 | REQ 根经 Bind 六维骨架接入，带租户/RBAC 上下文 | 02 业务视图 §1 / 安全视图 §5.2 | `flow-ai` 入口 / `mox-expert` | G0 |
| S2 归一化 | 四类流程图→同一 `FlowGraph`；auto_dimension 着色；租户配额→ResourcePool | 02 信息视图 §2（统一图） | `mox-expert::normalize` | **G0 归一化闸门** |
| S3 双璇玑并行诊断 | 14 位专家并行 `ExpertOpinion` | 02 应用视图 §3（插件化运行时） | `mox-expert::run_experts` | — |
| S4 归一化裁决 | 按 DIM_PRIORITY 合并→`ReconciledPlan`（硬约束优先） | 02 应用视图 §3.1 / `lib.rs::DIM_PRIORITY` | `mox-expert::reconcile` | **G1 裁决闸门** |
| S5 flow-ai 最优求解 | CPM+RCPSP+伪依赖剪除+冲突修复+出码 | 02 技术视图 §4.1 | `flow-ai::optimize` | — |
| S6 ⛨璇玑验证网关 | 5 项阻断级数学/语义检查，最高权限 | 02 安全视图 §5.1（STRIDE） | `mox-expert::verify` `govern.rs` | **G2 璇玑否决** |
| S7 治理闸门 Govern | 审计哈希链+版本状态机+SLA+成本+RBAC 审批 | 02 安全视图 §5 / 部署 §7 | `mox-expert::govern` `primiflow-fusion::full_gate` | **G3 治理闸门** |
| S8 出码/出图 | 代码工程+拓扑+可视化+指标 | 02 集成视图 §6.3 | `emit` | — |

### 2.2 四道强制闸门 → 架构/模块 映射（需求闭环的控制点）

| 闸门 | mox 模块化系统架构需求（AA-STD） | 架构承载 | 模块/代码落点 | 拒绝后果 |
|------|--------------------|----------|----------------|----------|
| **G0 归一化闸门** | IR 可拓扑排序(DAG)；维度着色完整；孤儿/悬空边=0（对齐 GR-E1/E2） | 02 信息视图 §2.1 | `primiflow-fusion::unified` `binding_report` / `governance_report` | 阻断出码 |
| **G1 裁决闸门** | 硬约束(Blocking) 优先于软约束；同优先级冲突升级 `Risk(Blocking)` | 02 应用视图 §3.1 | `mox-expert::reconcile` + `DIM_PRIORITY` | 阻断出码 |
| **G2 ⛨璇玑否决** | 任一阻断级检查失败→`vetoed=true`→强制 `Blocked`，**任何 RBAC/合规不可覆盖** | 02 安全视图 §5.1 | `mox-expert::verify` `govern.rs::GateResult` | 强制 Blocked |
| **G3 治理闸门** | `approved = !algorithm_veto ∧ status.can_emit() ∧ blocking==0 ∧ sla_ok ∧ budget_ok` | 02 部署 §7 / 安全 §5 | `mox-expert::govern` `primiflow-fusion::full_gate` | 拒，仅 dry-run |

> **与协作治理的衔接点**：G3 治理闸门的 `status.can_emit()` 来自 `FlowStatus` 状态机（见 `govern.rs`），
> 而 BP-7（BR-16）规定"任务 Done（组织验收）∧ 融合验证通过（技术验收）"才允许 `/publish`——
> 即协作域的 `FR-TASK-05 DoD 门禁` 与融合域的 `G2/G3 闸门` 在"上架"这一动作上 AND 闭合。

### 2.3 双璇玑十四维 → 架构视图 映射

| 维度 | 璇玑 | 优先级（DIM_PRIORITY） | 架构落点 | 产出类型 |
|------|------|------------------------|----------|----------|
| Permission 权限 | 业务 | **100（最高）** | 安全视图 §5.2 | 硬 / `Risk(Blocking)` |
| Security 安全 | 业务 | **100（最高）** | 安全视图 §5.1 | 硬 / `Risk(Blocking)` |
| Resource 资源 | 业务 | 90 | 信息视图 §2.3 | 硬 |
| Data 数据 | 业务 | 80 | 信息视图 §2.1 | 硬 |
| Business 业务 | 业务 | 70 | 业务视图 §1.2 | 软 |
| Observability 可观测 | 业务 | 60 | 技术视图 §4.2 | 软 |
| Algorithm 算法 | 业务 | 50 | 技术视图 §4.1 | 软 |
| （开发七维） | 开发 | 参与同优先级仲裁 | 应用视图 §3（CodeIR 驱动） | 混合 |

> 优先级数值取自 `crates/mox-expert/src/lib.rs::DIM_PRIORITY`，是"权限功能归一化"在裁决阶段落地的单一数据源。

---

## 3. 需求—架构一致性自检（企业级 DoD 对齐）

| 企业级 DoD 项（`05` §4） | 协作治理域 | 融合治理域 |
|--------------------------|:--:|:--:|
| 需求可验证（FR/NFR→代码/测试） | ✅ 见 §1 | ✅ 见 §2（AA-STD 8 阶段/4 闸门） |
| 架构多视图一致 | ✅ 02 七视图 | ✅ §2 各阶段已落视图 |
| 设计可追溯（模块→文件→契约） | ✅ 03 | ✅ §2.1/§2.2 已落模块 |
| 流程可闭环（FSM+BR） | ✅ 04（8BP+2FSM+21BR） | ✅ AA-STD 4 闸门控制点 |
| 安全可审计（RBAC/审计/威胁） | ✅ 02 §5 | ✅ G2 璇玑 + G3 治理 |
| 质量门禁（clippy/test） | ✅ | ✅ `primiflow-fusion verify` |
| 可观测 | ✅ NFR-08(I-04) | 📋 指标待补 |
| 文档同步 | ✅ | ✅ 本文建立映射 |

---

*本文是 enterprise 文档集的"需求—架构中枢"：协作治理需求来自 `01/04`，融合治理需求来自 AA-STD-V1.0，
二者经本文统一映射到 `02-architecture` 七视图与代码模块，形成企业级可追溯闭环。任何新增需求须先在此登记映射，再回填 `01` 追踪矩阵。*

---

## 4 · mox 模块化系统架构自动化处理（流水线入口 → 架构落点）

> **"mox 模块化系统架构自动化处理" = 把 §1/§2 的需求与闸门，由一条全自动流水线一次性跑通闭环**，无需人工逐段拼接。
> 它既是 `07-mox 模块化系统架构需求明确书` §3 铁律"双收口"的**自动化执行体**，也是本映射表所有架构落点的串联入口。

| 项 | 内容 | 架构/模块落点 |
|----|------|----------------|
| 入口 | 前端 `MoxFusionView.vue` 提交流程图 → `POST /api/optimize` | `crates/mox-expert/src/server.rs::run()` |
| 流水线主体 | `mox_optimize(raw, ctx)`：归一化→14专家并行→裁决→flow-ai求解→璇玑验证→治理闸门→审计 | `crates/mox-expert/src/pipeline.rs` |
| 归一化（G0 前） | 维度着色 + 唯一 `FlowGraph` | `mox_expert::ir::auto_dimension` |
| mox 模块化系统架构分析验证 | 双璇玑十四维专家并行派发（插件化运行时） | `harness::run_experts` + `experts::all_experts()` |
| 最优求解 | CPM/RCPSP/冲突修复/出码 | `flow_ai::pipeline::optimize` |
| 璇玑否决（G2） | 5 项阻断级检查，最高权限 | `mox_expert::verify` |
| 治理闸门（G3） | 审计链 + 状态机 + SLA + 成本 + RBAC | `mox_expert::govern` + `primiflow_fusion::full_gate` |
| 闭环产物 | `GovernanceReport`（专家分 + 优化报告 + 璇玑 + 闸门 + 审计哈希） | `pipeline.rs::GovernanceReport` |
| 端到端验证 | `mox_end_to_end_runs` / `mox_double_league_fourteen_dimensions` / 越权拦截测试 | `pipeline.rs` 测试模块 |

**自动化闭环步骤（与 `07` §2 四闸门一一对应）**：

```
① 提交流程图 → ② 归一化(G0 前) → ③ 14专家mox 模块化系统架构分析验证 → ④ 裁决(G1)
→ ⑤ flow-ai 最优求解 → ⑥ ⛨璇玑否决(G2,最高) → ⑦ 治理闸门(G3) → ⑧ 审计哈希链 → 出码/Blocked
```

> **铁律对齐**：该流水线**不容旁路**——任一步失败即 `Blocked`，无"先上后补"开关。
> 它即 `07` §3 所述"双收口"的自动化形态：流程图归一化在 ②、mox 模块化系统架构分析验证在 ③~⑥，均由同一函数一次完成。

---

## 5 · 三联盟责任映射矩阵（R/A/C/I · 对齐 ADR-DOC-002 四归三连）

> **责任矩阵（RACI）语义**：R=执行（主责 / 交付物作者），A=最终问责（一票否决），C=被咨询（评审参与），I=被知会（接收结果）。**同一事项的 R 唯一**，三联盟均对 ADR-DOC-010「四归三连」负最终会签责任。

| 工作域 / 事项 | 产品联盟 | 算法联盟 | 开发联盟 | 说明 / 交付物 |
|--------------|:--:|:--:|:--:|------|
| **TOP-MASTER 18 编写与 ADR 治理** | **R** | C | C | 产品联盟主笔业务口径；三联盟共同签署 ADR；A 角色=三联盟联合 |
| **需求规格（01 SRS · FR/NFR 定义）** | **R** | C | C | 需求必须「可验证」；GAP 登记与验收断言由产品联盟写 |
| **需求追踪（06 映射表 · 07 铁律 · 05 里程碑口径）** | **R** | C | C | 需求↔架构↔业务↔文档 四归的「需求侧」发起者；产品联盟对需求缺口 A |
| **架构（02 七层视图 · 六层金字塔）** | C | **R**（图/算法分层） + C（业务分层） | **R**（技术/工程分层） | 算法联盟 R=图谱 L 层级 / 8 大算法家族；开发联盟 R=分层工程 / 15 crate；产品联盟 C=业务视图 |
| **业务流程（04 BP-1~10 6 字段）** | **R**（BP-1/3/9）· C（其余） | **R**（BP-6/9）· C（其余） | **R**（BP-2/4/5/8/10）· C（其余） | BP-7 交付验收三联盟 R 共同会签（A 角色=联合） |
| **测试体系 · DoD 门禁（05 §4 · 09 归档）** | C（业务 DoD 定义） | C（算法 Δ/基准 Δ） | **R**（clippy/单测/集成/E2E） | 三联盟均在里程碑 L0/L1/L2 上 C；开发联盟 R |
| **八大算法家族 A1~A8 实现（graph-algorithms）** | I | **R**（算法正确性 / 论文对齐 / 精度） | **R**（Rust 工程实现 / 单测 / Δ 对账） | 算法联盟 A=精度达标；开发联盟 A=工程可维护 |
| **图谱建模（kg-hub · 八层 L0~L7 · 14 节点族 · 19 边族）** | C（需求节点/边语义） | **R**（分层/族划分/算法建模） | **R**（摄入/推理/治理/影响面/闭环 8 段实现） | ADR-DOC-004 对齐 |
| **存储分布式 M4（图谱后端 / 对象 / 元数据）** | I | C（图谱接口约束） | **R**（trait / 三后端 / 双写灰度） | |
| **统一 AI 编排 M3（Gateway 四端点 · AC-10 路由）** | C（产品语义） | **R**（A5 激活扩散意图识别 / A7 CEM） | **R**（routes/handlers/middleware） | ADR-DOC-009 对齐 |
| **前端 28 视图 + /admin 5 面板（frontend-ui）** | **R**（交互/P 原则落实） | C（图谱可视化/算法图表） | **R**（Vue3 + Three.js 工程实现） | admin-ui 旧目录已裁撤；单应用归一（ADR-DOC-005） |
| **可观测 & HA M5（指标/追踪/告警 & 混沌）** | I | C（算法维度指标模板） | **R**（运维组 · 部署组） | SLO A=开发联盟 + 产品联盟共同签署对外承诺 |
| **文档同步治理 BP-10（00-INDEX · GLOSSARY · README）** | **R**（产品命名 · 文案 · 术语） | **R**（算法术语 · 公式对齐） | **R**（代码路径 · 目录 · 锚点） | 三联盟 R=各自归口；A=联合签署 ADR-DOC-012 |
| **对外交付 & 客户签署（10 交付清单 · ISD-V1.0 报告）** | **R**（交付项定义 · 客户沟通） | C（算法交付项说明） | **R**（质量证据 · 测试报告 · 安装包） | A=三联盟联合 |
| **安全合规审计（RBAC · 审计链 · 越权拦截）** | C（合规口径） | C（算法层面零 toFixed 等硬约束） | **R**（安全组 · middleware · verify · govern） | A=开发联盟安全组 |
| **P9 判重闸门（BP-9 · tools/info-graph dedup · guantu_gate.py）** | **R**（是否真需新建判断签字 · ≥0.85 复用判定） | **R**（子图匹配算法 · Match Score 阈值） | **R**（CI 门禁 · 工具实现） | ADR-DOC-006 对齐 |
| **CEM AI 引擎优化（M8 自治 · 7 类基准 加权分）** | I（对外效果验收 L2） | **R**（A7 算法 + σ̄<0.06 / 3 轮无改善停止 + 加权分） | **R**（chatWithProvider 单次无重试调用） | hard constraint：禁止 retry/降级、严格单调用 |

> **使用方式**：任何 PR / 需求 / 里程碑，先在本表找到对应事项的 R 联盟（执行人），再确定 C 会签方；**三联盟的 C 方未响应 ≤ 3 工作日视为默认通过，但 A 方一票否决永远有效。**

---

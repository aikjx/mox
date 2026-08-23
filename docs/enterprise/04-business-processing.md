# 璇玑 RelGraph · 业务处理文档（10 大标准流程 · BP-01~10 三联盟版）

> **文档类型**：业务处理 / 流程 / 规则（BPMN 风格 + 状态机 + 规则目录 · 三联盟标注）
> **文档版本**：v1.1 (ENT) · 最后更新 2026-08-23
> **权威链**：🟢 L0 → [`18-全域顶层总设计-三联盟模式-V1.0.md`](18-全域顶层总设计-三联盟模式-V1.0.md)（TOP-MASTER §四：10 大标准业务流程）。本文为 L2 第三级（业务层）。
> **主责联盟**：产品联盟（业务规则定义） + 开发联盟（流程实现） · 会签：算法联盟（BP-6/BP-9 算法落地）
> **配套**：`01-requirements.md`、`02-architecture.md`、`03-design.md`
> **权威来源**：`docs/modules/xuanji-expert-business-requirements.md`（BR-01…BR-21、GAP 清单）
>
> **流程强制标准（对齐 18 §四.4 四归三连）**：每流程 6 字段必齐——①编号 ②主责联盟 ③前置条件 ④核心步骤 ⑤闸门规则 ⑥审计与产物。

---

## 1. 业务处理框架

系统回答四个约束闭合问题（见需求 §1.2）：

1. **谁能做什么** → RBAC（角色/权限/作用域/所有权）。
2. **什么状态能到什么状态** → 任务/成员生命周期状态机。
3. **什么条件下算完成** → DoD 门禁（子任务 + 依赖）。
4. **做过什么留了什么痕** → 领域事件 + 审计链。

两条业务主线串联：**协作治理产出「谁/做什么/是否通过」；融合治理产出「怎么更快/是否可信」**。

---

## 2. 端到端业务流程总览（三联盟 · 10 大标准 BP）

> 横切说明：BP-8「审计留痕」、BP-9「P9 判重闸门」、BP-10「文档同步治理」三条为**全链路横切流程**，分别贯穿组织决策、需求接入、代码改动三个关键入口；其余 BP-1~7 为端到端主链路。

```
(入口 · 产品联盟)  BP-9 P9判重闸门（先判重后立项 / 子图匹配）
   ↓ 判重通过（新 REQ 根进入图谱）
(产品联盟)  BP-1 璇玑组建
   ↓
(产品+开发)  BP-2 专家入璇玑
   ↓
(产品联盟)  BP-3 任务立项（Bind 六维 · 06 映射行登记）
   ↓
(开发联盟)  BP-4 任务派发
   ↓
(开发联盟)  BP-5 协同推进
   ↓
(算法联盟)  BP-6 璇玑融合优化（14 维会诊 + flow-ai 求解 + ⛨璇玑 G2）
   ↓
(三联盟会签) BP-7 交付验收与上架（G3 治理 + BR-16 双验收 AND）
   ↑___________________________________________↓
          BP-8 审计留痕（横切 BP-1~BP-7 全程）
          BP-10 文档同步归一 / ADR 治理（横切代码改动与需求变更）
```

---

## 3. 十大标准业务流程（BP-01~10 · 6 字段齐）

> **6 字段标准（强制）**：① 编号 ② 主责联盟（RA） ③ 前置条件 ④ 核心步骤 ⑤ 闸门规则 ⑥ 审计与产物。

---

### BP-1 璇玑组建
| 6 字段 | 内容 |
|--------|------|
| **RA 主责联盟** | 产品联盟（发起） + 开发联盟（`orchestrator.bootstrap` 实现） |
| **前置条件** | BP-9 判重通过；多租户命名在全局唯一；bootstrap 令牌一次性有效 |
| **核心步骤** | 1.1 创建璇玑实体（多租户隔离单位）→ 1.2 创建首位管理员（状态 Active）→ 1.3 授 XuanjiAdmin@Global → 1.4 惰性创建「璇玑大厅」频道 → 1.5 签发访问令牌 |
| **闸门规则** | BR-01 璇玑须先于成员/任务存在；命名全局冲突 → 拒绝；bootstrap 令牌过期/重复使用 → 拒绝；唯一无鉴权入口（bootstrap）限一次调用 |
| **审计与产物** | DomainEvent=`XuanjiCreated`；首次令牌哈希入库；RBAC 绑定=1（首管理员）；产物=璇玑 ID + 大厅频道 ID |

### BP-2 专家入璇玑
| 6 字段 | 内容 |
|--------|------|
| **RA 主责联盟** | 产品联盟（专家角色定义） + 开发联盟（`MemberService`） |
| **前置条件** | BP-1 完成（璇玑存在）；调用者持 `member:invite@Xuanji`；被邀 email 经 BR-04 幂等检查 |
| **核心步骤** | 2.1 鉴权 `member:invite@Xuanji`（BR-02）→ 2.2 创建成员（Invited）→ 2.3 授 **Expert@Xuanji（最小权限，BR-03 禁止授 Global）** → 2.4 发 `MemberInvited` → 通知+大厅播报 → 2.5 激活 `Invited→Active`（BR-05） |
| **闸门规则** | BR-03 不得越权授 Global；BR-04 同 email 重复 → Conflict；跨璇玑成员注入 → 三重校验 Forbidden |
| **审计与产物** | DomainEvent=MemberInvited/MemberActivated；成员 Tier 档案初始化；璇玑成员列表增量；幂等冲突审计 |

### BP-3 任务立项
| 6 字段 | 内容 |
|--------|------|
| **RA 主责联盟** | 产品联盟（REQP → TSK 拆解） + 开发联盟（`TaskService::create`） |
| **前置条件** | BP-9 判重登记（REQ 根在图上）；BP-2 专家激活；调用者持 `task:create@Xuanji`；`06-requirements-architecture-map.md` 已登记对应映射行（四归三连强制） |
| **核心步骤** | 3.1 鉴权 `task:create@Xuanji` → 3.2 建任务（Draft，assignees=[]，**BR-06 立项不得自带分派**）→ 3.3 发 TaskCreated → 大厅播报 → 3.4 自动 Bind：REQ→FUN→BIZ→ALG→TSK（六维绑定 Chain 写入关图） |
| **闸门规则** | assignees 非空 → 拒绝；REQ 根在关图不可达 → 偏离告警并阻断（GR-E6） |
| **审计与产物** | DomainEvent=TaskCreated；关图新增 `TSK-*` 节点 + Bind 边 ×5；追踪矩阵导出行 |

### BP-4 任务派发
| 6 字段 | 内容 |
|--------|------|
| **RA 主责联盟** | 开发联盟（Coordinator 角色执行） · 产品联盟 C |
| **前置条件** | BP-3 完成；调用者持 `task:assign@Xuanji` |
| **核心步骤** | 4.1 鉴权 `task:assign` → 4.2 读当前状态 → 4.3 写 assignees，`Draft→Assigned`，**BR-07 分派身份三重校验（存在/同璇玑/Active，GAP-2 跨租户 P0 已闭环）** → 4.4 被分派者加入任务频道 → 4.5 发 `TaskAssigned` → 通知+系统消息 |
| **闸门规则** | BR-07 任一不满足 → Forbidden/InvalidState；BR-08 全量覆盖语义；历史 assignees 未在新集合 → 自动退订通知 |
| **审计与产物** | DomainEvent=TaskAssigned；RBAC assignee 写入审计；频道成员增量 |

### BP-5 协同推进
| 6 字段 | 内容 |
|--------|------|
| **RA 主责联盟** | 开发联盟（Expert 执行） · 产品联盟验收 |
| **前置条件** | BP-4 Assigned 状态；调用者持 `task:transition:all/own` 之一 |
| **核心步骤** | 5.1 分级鉴权：先 TaskTransitionAll，回退 TaskTransitionOwn（BR-09 状态机）→ 5.2 FSM 合法性校验 → 5.3 写新状态 + updated_at → 5.4 发 `TaskStatusChanged` → 系统消息+通知 → 5.5 评论 → 频道+双事件 → 5.6 InReview → DoD 门禁 → Done |
| **闸门规则** | BR-10 Done 门禁：子任务全完成 ∧ 依赖全 Done（GAP-3 P0）；BR-11 依赖 DAG（自依赖/成环/跨璇玑 拒绝，GAP-4）；BR-12 终态（Done/Cancelled）不可迁出 |
| **审计与产物** | DomainEvent=TaskStatusChanged × N；CommentAdded；DoD 门禁校验通过签名；子任务与依赖 DAG 快照 |

### BP-6 璇玑融合优化
| 6 字段 | 内容 |
|--------|------|
| **RA 主责联盟** | 算法联盟（14 维诊断 / 裁决 / CPM+RCPSP） + 开发联盟（流水线编排） |
| **前置条件** | BP-5 InReview/Done；流程图唯一 FlowGraph（已归一化）；`06` 映射行存在 |
| **核心步骤** | XuanjiFusionView → `POST /api/optimize` → `xuanji_optimize(raw, ctx)`：归一化 IR → 14 专家并行会诊 → reconcile 裁决 → flow-ai CPM+RCPSP 最优求解 → ⛨璇玑验证（G2） → 治理闸门（G3） |
| **闸门规则** | BR-13 治理一票否决（安全/合规专家 veto → Blocked）；BR-14 5 项不变式（真依赖不剪/真并行无数据竞争）；BR-15 加速比 ≥2.32×、省时 50%、算力压缩 52.9% 可解释 |
| **审计与产物** | GovernanceReport（专家分 + 裁决 + ⛨结论 + G3 签名 + 哈希链）；优化前后关键路径对比可视化；产物可上传算子市场草稿态 |

### BP-7 交付验收与上架
| 6 字段 | 内容 |
|--------|------|
| **RA 主责联盟** | **三联盟会签**（产品=组织验收 · 算法=技术验收 · 开发=可观测性/可部署性） |
| **前置条件** | BP-5 Done ∧ BP-6 GovernanceReport.approved=true |
| **核心步骤** | 7.1 产品联盟确认「任务 Done、需求覆盖、合规口径」→ 7.2 算法联盟确认「⛨vetoed=false、无 Blocking 风险」→ 7.3 开发联盟确认「clippy 0 warning、测试全绿、可部署包就绪」→ 7.4 `POST /api/xuanji/publish` → 算子市场上架 |
| **闸门规则** | **BR-16 双验收 AND**：组织 Done 为假 → 拒绝；融合 G2/G3 任一未过 → 拒绝；上架请求缺失 BR-17 元数据（璇玑/任务 ID、优化前后指标）→ 拒绝 |
| **审计与产物** | Publish 事件 + 双验收签名；算子市场条目含来源追溯 ProvenanceMetrics；可一键回滚下架 |

### BP-8 审计留痕（横切 BP-1~BP-7）
| 6 字段 | 内容 |
|--------|------|
| **RA 主责联盟** | 开发联盟 · 安全组（`rbac_audit_middleware` + `AuditChain`） |
| **前置条件** | 全链路写操作入口（middleware 自动拦截）；读操作的 `audit:view` 显式调用 |
| **核心步骤** | 8.1 写操作：统一 `require()` → 通过/拒绝双写审计 → 8.2 领域事件 9 类进 EventBus → 8.3 反应器幂等处理（BR-20）→ 8.4 AuditChain 哈希链 append（prev_hash→hash，不可篡改） |
| **闸门规则** | **BR-18** 鉴权失败留痕（非试探式鉴权才写，避免噪声，GAP-5 已闭环）；**BR-19** 写操作 100% 发事件；哈希链断裂 → 服务拒绝启动（fail-fast） |
| **审计与产物** | 审计链落盘可查询；`GET /api/audit` 端点支持按 member_id/action/ts 过滤；拒绝探针与通过探针数量平衡统计 |

### BP-9 P9 先判重后立项（全入口横切 · 对齐 enterprise/16 验收报告）
| 6 字段 | 内容 |
|--------|------|
| **RA 主责联盟** | 算法联盟（子图匹配 / 图指纹计算） + 产品联盟（「是否真的需要新建」判断） |
| **前置条件** | 任何「新项目 / 新璇玑 / 新需求 REQ 根 / 新算子」立项动作；必须先走到 BP-9（**P9 判重闸门在 BP-1~3 之前执行，铁律**） |
| **核心步骤** | 9.1 调用 `tools/info-graph dedup`（M4 快速判重）→ 对关图 `graph.enterprise.json` 做子图匹配（A1 CNM 社区 + 子图同构候选）→ 9.2 输出 Match Score / 已存在节点清单 / 推荐复用路径 / 需增量补充边 → 9.3 三种决策：已全匹配 → 直接复用 Bind 到既有 REQ 根 / 局部匹配 → 增量补边 / 零匹配 → 允许立项 → 写入新 REQ 根 → `tools/guantu_gate.py` CI 门禁通过 |
| **闸门规则** | Match Score ≥ 0.85 且核心依赖完全一致 → 立项 **Blocked**（必须复用）；0.6 ≤ 分数 < 0.85 → 须产品联盟签字说明为何不复用；< 0.6 → 放行 |
| **审计与产物** | P9 判重报告（match_score/已存在节点/推荐复用路径）入库；关图新增 REQ 根节点；棘轮基线归零后残差报告 |

### BP-10 文档同步归一 / ADR 治理（代码改动与需求变更全链路横切）
| 6 字段 | 内容 |
|--------|------|
| **RA 主责联盟** | 产品联盟（07/15 口径） + 算法联盟（02/04 架构/流程） + 开发联盟（实现侧与 ADR 流程） —— 三联盟联合 R |
| **前置条件** | 任一 PR 改动代码 / 任一 ADR-DOC 新申请 / 任一 18 TOP-MASTER 正文改动 |
| **核心步骤** | 10.1 代码改动 → 检查 `06` 映射表是否缺行 → 补映射 → 10.2 流程改动 → 检查 `04` BP-xx 6 字段是否齐 → 补 BP → 10.3 架构改动 → 登记 `01 §9` 新 ADR-DOC-xxx → 三联盟会签 → 10.4 18 TOP-MASTER 改动 → ADR-DOC-NEW 申请 → 三联盟签署 → 再改正文 → 10.5 CI 阶段：PR 必须勾选「四归三连完成」复选框，否则阻断合并 |
| **闸门规则** | ADR-DOC 未会签 → 18 正文改动 PR 阻断；06 映射表缺行 → 代码改动 PR 阻断；路径命名不一致（如 `crates/`、`frontend/` 残留）→ lint 告警 fail |
| **审计与产物** | ADR 变更日志；00-INDEX §变更记录留痕；docs 覆盖率报告（06 映射行数 / 代码模块数）；每发布 bump ENT 版本 |

---

## 4. 生命周期状态机

### 4.1 任务 FSM
```
[*] → Draft: create()
Draft → Assigned: assign()           [BR-07 三重校验]
Draft → Cancelled
Assigned → InProgress
Assigned → Cancelled
InProgress → InReview
InProgress → Cancelled
InReview → Done: 需 DoD 门禁 [BR-10]
InReview → InProgress: 打回
InReview → Cancelled
Done → [*]        (终态，BR-12)
Cancelled → [*]   (终态，BR-12)
```

### 4.2 成员 FSM
```
[*] → Invited: invite()
Invited → Active: activate()
Invited → Left: 拒绝/撤回
Active → Suspended: 停权
Active → Left: 退出
Suspended → Active: 恢复
Suspended → Left: 移除
Left → [*]: 终态不可复活 [BR-21]
```

---

## 5. 业务规则目录（BR-01…BR-21）

| 编号 | 类别 | 严重度 | 规则 | 状态 |
|------|------|:--:|------|:--:|
| BR-01 | 完整性 | P1 | 璇玑须先于成员/任务存在 | ✅ |
| BR-02 | 安全 | P0 | 写操作统一鉴权 | ✅ |
| BR-03 | 安全 | P0 | 受邀成员最小权限 Expert@Xuanji | ✅ |
| BR-04 | 一致性 | P1 | 邀请幂等（同 email） | ✅ |
| BR-05 | 完整性 | P0 | 仅 Active 可承接任务 | ✅ |
| BR-06 | 职责分离 | P1 | 立项不得自带分派 | ✅ |
| BR-07 | **安全** | **P0** | 分派身份三重校验（GAP-2） | ✅ |
| BR-08 | 明确性 | P2 | 分派全量覆盖 | ✅ |
| BR-09 | 完整性 | P0 | 任务状态机校验 | ✅ |
| BR-10 | 完整性 | **P0** | DoD 完成门禁（GAP-3） | ✅ |
| BR-11 | 完整性 | P1 | 依赖 DAG 约束（GAP-4） | ✅ |
| BR-12 | 完整性 | P1 | 终态不可迁出 | ✅ |
| BR-13 | 安全 | P0 | 治理一票否决 | ✅ |
| BR-14 | 正确性 | P0 | 不变式验证 | ✅ |
| BR-15 | 可信 | P1 | 优化可解释 | ✅ |
| BR-16 | 治理 | P1 | 双验收才可上架 | 📋 部分 |
| BR-17 | 合规 | P2 | 产物来源追溯 | 📋 部分 |
| BR-18 | **合规** | P1 | 鉴权失败留痕（GAP-5） | ✅ |
| BR-19 | 可审计 | P0 | 写操作必发事件 | ✅ |
| BR-20 | 可靠性 | P1 | 反应器幂等 | ✅ 声明 |
| BR-21 | 完整性 | **P0** | 成员状态机校验（GAP-6） | ✅ |

### 5.1 GAP 实施结果（六项全部闭环）

| GAP | 需求 | 级别 | 关键实现 | 验收测试 |
|-----|------|:--:|----------|----------|
| GAP-2 | BR-07 | 🔴 P0 | `validate_assignees` 三重校验 | `br07_*` |
| GAP-6 | BR-21 | 🔴 P0 | `MemberStatus::can_transition` | `br21_*` |
| GAP-3 | BR-10 | 🔴 P0 | `check_done_gate` | `br10_*` |
| GAP-4 | BR-11 | 🟠 P1 | `add_dependency` + `reaches` 环检测 | `br11_*` |
| GAP-1 | BR-04 | 🟠 P1 | email 唯一→Conflict | `br04_*` |
| GAP-5 | BR-18 | 🟠 P1 | `require` 落审计 + `AuthzDenied` | `br18_*` |

---

## 6. 异常处理与 SLA

| 异常 | 处理 | SLA 目标 |
|------|------|----------|
| 跨租户引用 | 拒绝 Forbidden + 审计 | 100% 拦截 |
| 状态机违规 | InvalidState，不写态 | 即时返回 |
| 分派非法成员 | BadRequest/Forbidden/InvalidState | 即时返回 |
| 鉴权失败 | 拒绝 + 留痕（非试探） | 审计可查 |
| 事件重放 | 反应器幂等，无副作用 | 无重复 |

> 量化 SLA（响应时延、可用性 99.9%）在 `02-architecture` §7.2 设计，采集实现见路线图 `05`。

---

## 7. 与执行引擎的关系

- 本文聚焦**协作治理域**业务处理（成员/任务/权限/通信）。
- **企业级流程执行**（WorkflowEngine + 6 模板）见 `docs/modules/business-process-flows.md`；可视化见 `docs/modules/business-process-flowcharts.md`。
- **融合优化链路**见 `docs/modules/xuanji-expert-alliance-fusion-flows.md`。

---

*业务规则与代码一一对应，经 `01-requirements` 追踪矩阵与 `03-design` 模块设计闭环。*

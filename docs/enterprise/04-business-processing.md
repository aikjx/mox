# 璇玑系统 · 业务处理文档（Business Processing）

> **文档类型**：业务处理 / 流程 / 规则（BPMN 风格 + 状态机 + 规则目录）
> **文档版本**：v1.0 (ENT) · 最后更新 2026-08-16
> **配套**：`01-requirements.md`、`02-architecture.md`、`03-design.md`
> **权威来源**：`docs/modules/xuanji-expert-business-requirements.md`（BR-01…BR-21、GAP 清单）

---

## 1. 业务处理框架

系统回答四个约束闭合问题（见需求 §1.2）：

1. **谁能做什么** → RBAC（角色/权限/作用域/所有权）。
2. **什么状态能到什么状态** → 任务/成员生命周期状态机。
3. **什么条件下算完成** → DoD 门禁（子任务 + 依赖）。
4. **做过什么留了什么痕** → 领域事件 + 审计链。

两条业务主线串联：**协作治理产出「谁/做什么/是否通过」；融合治理产出「怎么更快/是否可信」**。

---

## 2. 端到端业务流程总览

```
BP-1 璇玑组建 → BP-2 专家入璇玑 → BP-3 任务立项 → BP-4 任务派发
   → BP-5 协同推进 → BP-6 璇玑融合优化 → BP-7 交付验收与上架
                                   ↑__________________↓
                  BP-8 审计留痕（横切 BP-2~BP-7 全程）
```

---

## 3. 八大业务流程（BP）

### BP-1 璇玑组建
| 步骤 | 动作 | 规则 |
|------|------|------|
| 1.1 | 创建璇玑实体（多租户隔离单位） | BR-01 璇玑须先于成员/任务存在 |
| 1.2 | 创建首位管理员（状态 Active） | 无需邀请自环 |
| 1.3 | 授 XuanjiAdmin@Global | bootstrap 唯一无鉴权入口 |
| 1.4 | 惰性创建「璇玑大厅」频道 | — |
| 1.5 | 签发访问令牌 | — |

### BP-2 专家入璇玑
| 步骤 | 动作 | 规则 |
|------|------|------|
| 2.1 | 鉴权 `member:invite@璇玑` | BR-02 写操作先鉴权 |
| 2.2 | 创建成员（Invited） | — |
| 2.3 | 授 Expert@Xuanji（最小权限） | **BR-03** 不得 Global |
| 2.4 | 发 MemberInvited 事件 → 通知+大厅播报 | — |
| 2.5 | 激活 Invited→Active | BR-05 仅 Active 可承接任务 |

> **BR-04**（已修复）：同璇玑同 email 邀请幂等 → Conflict。

### BP-3 任务立项
| 步骤 | 动作 | 规则 |
|------|------|------|
| 3.1 | 鉴权 `task:create@璇玑` | — |
| 3.2 | 建任务（Draft，assignees=[]） | **BR-06** 不得自带分派 |
| 3.3 | 发 TaskCreated → 大厅播报 | — |

### BP-4 任务派发
| 步骤 | 动作 | 规则 |
|------|------|------|
| 4.1 | 鉴权 `task:assign` | — |
| 4.2 | 读当前状态 | — |
| 4.3 | 写 assignees，Draft→Assigned | **BR-07** 三重校验 |
| 4.4 | 被分派者加入任务频道 | — |
| 4.5 | 发 TaskAssigned → 通知+系统消息 | — |

> **BR-07（GAP-2，安全 P0）**：被分派者须逐一校验 ①存在 ②同璇玑 ③Active。修复前可写入他璇玑 ID 构成跨租户提权。
> **BR-08**：分派为全量覆盖语义。

### BP-5 协同推进
| 步骤 | 动作 | 规则 |
|------|------|------|
| 5.1 | 分级鉴权：先 TaskTransitionAll，回退 TaskTransitionOwn | BR-09 状态机 |
| 5.2 | 状态机合法性校验 | — |
| 5.3 | 写新状态 + updated_at | — |
| 5.4 | 发 TaskStatusChanged → 系统消息+通知 | — |
| 5.5 | 评论 → 频道+双事件 | — |

> **BR-10（GAP-3，P0）**：进 Done 需 DoD 门禁（子任务全完成 ∧ 依赖全 Done）。
> **BR-11（GAP-4，P1）**：依赖图须 DAG（拒自依赖/成环/跨璇玑）。
> **BR-12**：终态不可迁出。

### BP-6 璇玑融合优化
```
XuanjiFusionView → POST /api/xuanji/optimize
  → xuanji_expert::pipeline::xuanji_optimize()
     归一化 IR → 七维会诊 → 冲突消解 → 治理裁决 → 不变式验证
  → POST /api/xuanji/publish → 算子市场
```
> **BR-13** 治理一票否决；**BR-14** 不变式验证；**BR-15** 结果可解释（加速比 2.32×、省时 50%、算力压缩 52.9%）。

### BP-7 交付验收与上架
> **BR-16** 上架前置：任务 Done（组织验收）∧ 融合验证通过（技术验收），AND 关系。
> **BR-17** 产物携来源追溯（璇玑/任务 ID、优化前后指标）。

### BP-8 审计留痕（横切）
> **BR-18（GAP-5，P1）**：鉴权失败留痕（member_id/permission/scope/reason）。
> **BR-19**：所有写操作发领域事件（9 类），是审计与推送唯一源。
> **BR-20**：反应器幂等，重放无副作用。

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

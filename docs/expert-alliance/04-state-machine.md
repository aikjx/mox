---
doc_id: EA-SM-001
title: 专家联盟状态机与业务规则
version: V1.0
authority: 🟢权威
last_updated: 2026-09-25
---

# 专家联盟状态机与业务规则

## 1. 任务状态机

### 1.1 任务主状态

```
pending → planning → executing → fusing → completed
   ↓          ↓          ↓          ↓
 cancelled  failed    failed    failed
```

| 状态 | 说明 | 进入条件 | 退出条件 |
|------|------|---------|---------|
| pending | 待处理 | 任务创建 | 调度器接收 |
| planning | 规划中 | 开始匹配+生成计划 | 计划生成完成 |
| executing | 执行中 | DAG开始执行 | 全部节点完成 |
| fusing | 融合中 | 开始结果融合 | 融合完成 |
| completed | 已完成 | 融合成功 | — |
| failed | 已失败 | 任一关键节点失败 | — |
| cancelled | 已取消 | 用户主动取消 | — |

### 1.2 节点子状态机

```
pending → running → completed
   ↓         ↓
 skipped   failed → (重试) → pending
```

| 状态 | 说明 |
|------|------|
| pending | 待执行（依赖未满足） |
| running | 执行中 |
| completed | 执行成功 |
| failed | 执行失败（可重试） |
| skipped | 跳过（非关键节点失败后跳过） |

---

## 2. 专家状态机

```
inactive → active → busy → active
    ↓         ↓
  offline   offline
```

| 状态 | 说明 | 进入条件 |
|------|------|---------|
| active | 在线可用 | 注册成功 |
| busy | 忙碌中 | 正在执行任务 |
| offline | 离线 | 心跳超时 |
| inactive | 未激活 | 初始状态 |

**注意：** `availability.status` 是登记值，不是实时探活结果。健康检查由 scheduler-core 侧维护。

---

## 3. 审批状态机

```
pending → approved
   ↓
 rejected
```

| 状态 | 说明 |
|------|------|
| pending | 待审批 |
| approved | 已通过 |
| rejected | 已驳回 |

**幂等保护：** 已处理的审批不可重复操作。

---

## 4. 匹配评分规则

### 4.1 评分权重

| 维度 | 权重 | 计算方式 |
|------|------|---------|
| 领域匹配 domain_match | 30% | 匹配领域数 / 总需求领域数 |
| 能力匹配 capability_match | 30% | 70%能力覆盖度 + 30%描述文本相似度 |
| 健康状态 health_score | 15% | is_healthy ? 1.0 : 0.3 |
| 优先级 priority_score | 15% | priority / 10.0 |
| 历史表现 performance_score | 10% | health.success_rate |

### 4.2 过滤条件

| 条件 | 阈值 | 说明 |
|------|------|------|
| 租户隔离 | tenant_id 匹配 | 仅同租户或 system 专家可见 |
| 状态过滤 | status = Active | 仅在线可用专家参与匹配 |
| 优先级过滤 | priority ≥ min_priority | 优先级过低不入选 |
| 总分阈值 | total_score ≥ 0.2 | 低于此分淘汰 |

---

## 5. Dynamic 模式决策规则

### 5.1 决策优先级（自上而下短路）

| 优先级 | 规则 | 选定模式 | 说明 |
|--------|------|---------|------|
| 1 | 任务描述含"迭代/优化/refine" | Iterative | 用户明确要迭代优化 |
| 2 | 任务描述含"评审/审核/review" | Sequential | 先产出后复核 |
| 3 | 专家数 ≤ 1 | Sequential | 单专家无需并行 |
| 4 | 领域去重数 ≥ 3 | Hierarchical | 跨域分层协同 |
| 5 | 匹配分极差 ≥ 0.25 | Hierarchical | 强弱分明，高分领衔 |
| 6 | 匹配分极差 < 0.10 且专家 ≥ 3 | Voting | 实力接近，投票裁决 |
| 7 | 默认 | Debate | 存在分歧，辩论仲裁 |

### 5.2 阈值常量

| 常量 | 值 | 说明 |
|------|---|------|
| DYNAMIC_SPREAD_HIERARCHICAL | 0.25 | 极差≥此值→分层 |
| DYNAMIC_SPREAD_VOTING | 0.10 | 极差<此值且≥3人→投票 |
| DYNAMIC_DOMAIN_DIVERSITY_MIN | 3 | 领域去重数≥此值→分层 |

---

## 6. 融合策略规则

| 策略 | 说明 | 权重来源 |
|------|------|---------|
| MajorityVote | 多数投票，取出现次数最多的结果 | 等权 |
| WeightedVote | 加权投票，按匹配分加权 | 匹配分 → 真权重 |
| Concatenation | 按专家顺序拼接输出 | — |
| BestOf | 择优选择，取分数最高的专家结果 | — |
| DebateArbitration | 裁判专家裁决正反方观点 | — |
| IterativeRefinement | 多轮迭代精炼，逐步优化 | — |

**关键规则：** 融合权重来自匹配分，不是等权 1.0。匹配分高的专家在融合中权重更大。

---

## 7. 重试与降级规则

| 规则 | 参数 | 说明 |
|------|------|------|
| 最大重试次数 | 3 次 | 节点失败后最多重试 |
| 退避策略 | 指数退避 1s→2s→4s | 避免连续失败冲击 |
| 关键节点定义 | 决策节点 + 主路径首节点 | 关键节点失败 → 任务失败 |
| 非关键节点失败 | 标记 Skipped | 跳过，继续执行其他节点 |
| 0匹配兜底 | 退化为单个通用专家 | 不静默出空计划 |

---

## 8. 数据一致性规则

| 规则 | 说明 |
|------|------|
| 幂等保护 | 已处理的审批不可重复操作 |
| 拓扑验证 | DAG 生成后必须 validate()，环检测通过才执行 |
| 状态一致性 | 节点状态变更必须同步更新到调度器 |
| 租户隔离 | 所有查询必须带 tenant_id 过滤 |

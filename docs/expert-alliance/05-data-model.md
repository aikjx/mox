---
doc_id: EA-DATA-001
title: 专家联盟数据模型与字典
version: V1.0
authority: 🟢权威
last_updated: 2026-09-25
source_of_truth: common-proto/types.rs + experts_db.rs（SQLite）
---

# 专家联盟数据模型与字典

## 1. 核心实体清单

| 实体 | 说明 | 存储位置 |
|------|------|---------|
| Task | 任务 | SQLite（tasks 表） |
| Node | DAG 节点 | SQLite（nodes 表） |
| CollaborationPlan | 协作计划 | SQLite（plans 表） |
| Expert | 专家 Agent | 内存 + SQLite（experts 表） |
| ExpertHealth | 专家健康指标 | 内存（随 Expert 存储） |
| MatchedExpert | 匹配结果 | 内存（不持久化） |
| MatchScoreBreakdown | 匹配分数明细 | 内存（不持久化） |
| PlanDynamicRoute | 动态路由规则 | SQLite（plans 表内 JSON） |

---

## 2. Task（任务）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| task_id | Uuid | 是 | 任务唯一标识 |
| tenant_id | Uuid | 是 | 租户 ID |
| user_id | Uuid | 是 | 提交用户 ID |
| title | String | 是 | 任务标题 |
| description | String | 是 | 任务描述 |
| task_type | String | 是 | 任务类型（默认 "custom"） |
| status | TaskStatus | 是 | 任务状态（见状态机） |
| priority | TaskPriority | 是 | 优先级（Low=1/Normal=5/High=8/Critical=10） |
| progress | f32 | 是 | 进度百分比 0.0~1.0 |
| current_node_id | Option\<String> | 否 | 当前执行节点 ID |
| mode | AllianceMode | 是 | 协作模式（7种） |
| fusion_strategy | FusionStrategy | 是 | 融合策略（9种） |
| created_at | DateTime\<Utc> | 是 | 创建时间 |
| started_at | Option\<DateTime\<Utc>> | 否 | 开始时间 |
| completed_at | Option\<DateTime\<Utc>> | 否 | 完成时间 |
| duration_ms | Option\<i64> | 否 | 执行时长（毫秒） |
| fusion_result | Option\<Value> | 否 | 融合结果（JSON） |

### TaskStatus 枚举

| 值 | 说明 |
|---|------|
| pending | 待处理 |
| planning | 规划中（生成协作计划） |
| running | 执行中 |
| paused | 已暂停 |
| completed | 已完成（终态） |
| failed | 失败（终态） |
| cancelled | 已取消（终态） |

### TaskPriority 枚举

| 值 | 数字 | 说明 |
|---|------|------|
| low | 1 | 低 |
| normal | 5 | 普通（默认） |
| high | 8 | 高 |
| critical | 10 | 紧急 |

---

## 3. Node（DAG 节点）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| node_id | String | 是 | 节点唯一标识（如 "node-1"） |
| task_id | Uuid | 是 | 所属任务 ID |
| expert_id | String | 是 | 执行该节点的专家 ID |
| module_id | Option\<String> | 否 | 模块配置 ID（专家→模块映射） |
| name | String | 是 | 节点名称 |
| description | Option\<String> | 否 | 节点描述（含任务描述） |
| status | NodeStatus | 是 | 节点状态 |
| retry_count | u32 | 是 | 已重试次数 |
| dependencies | Vec\<String> | 是 | 依赖的上游节点 ID 列表 |
| input_refs | Vec\<String> | 是 | 输入数据引用 |
| output_ref | Option\<String> | 否 | 输出数据引用 |
| started_at | Option\<DateTime\<Utc>> | 否 | 开始时间 |
| completed_at | Option\<DateTime\<Utc>> | 否 | 完成时间 |
| duration_ms | Option\<i64> | 否 | 执行时长 |
| error_message | Option\<String> | 否 | 错误信息 |

### NodeStatus 枚举

| 值 | 说明 |
|---|------|
| pending | 待执行（依赖未满足） |
| ready | 就绪（依赖已满足，可执行） |
| running | 执行中 |
| completed | 已完成（终态） |
| failed | 失败（终态，可重试） |
| skipped | 跳过（终态，非关键节点失败后跳过） |
| cancelled | 已取消（终态） |

---

## 4. CollaborationPlan（协作计划）

| 字段 | 类型 | 说明 |
|------|------|------|
| task_id | Uuid | 所属任务 ID |
| mode | AllianceMode | 协作模式 |
| fusion_strategy | FusionStrategy | 融合策略 |
| nodes | Vec\<Node> | 节点列表 |
| dynamic_routes | Vec\<PlanDynamicRoute> | 动态路由规则（Dynamic 模式专用） |
| expert_weights | HashMap\<String, f64> | 专家权重（来自匹配分） |
| version | u32 | 计划版本 |
| created_at | DateTime\<Utc> | 创建时间 |

---

## 5. Expert（专家 Agent）

| 字段 | 类型 | 说明 |
|------|------|------|
| expert_id | String | 专家唯一标识 |
| tenant_id | String | 租户 ID（"system" 为全局共享） |
| name | String | 专家名称 |
| version | String | 专家版本 |
| description | String | 专家描述 |
| domains | Vec\<String> | 领域标签列表 |
| capabilities | Vec\<Capability> | 能力声明列表 |
| tools | Vec\<Tool> | 可用工具列表 |
| status | ExpertStatus | 专家状态 |
| health | ExpertHealth | 健康指标 |
| priority | u32 | 优先级（1~10） |
| created_at | DateTime\<Utc> | 创建时间 |
| updated_at | DateTime\<Utc> | 更新时间 |

### ExpertStatus 枚举

| 值 | 说明 |
|---|------|
| active | 在线可用 |
| busy | 忙碌中 |
| offline | 离线 |
| inactive | 未激活 |

### ExpertHealth 结构

| 字段 | 类型 | 说明 |
|------|------|------|
| is_healthy | bool | 是否健康 |
| success_rate | f64 | 历史成功率（0.0~1.0） |
| avg_response_time_ms | u64 | 平均响应时间 |
| last_heartbeat | DateTime\<Utc> | 最后心跳时间 |

---

## 6. AllianceMode（协作模式）

| 值 | 说明 | DAG 拓扑 |
|---|------|---------|
| sequential | 串行 | 链式依赖，依次执行 |
| parallel | 并行 | 无依赖，同时执行 |
| debate | 辩论 | 正方+反方+裁判 |
| hierarchical | 分层 | 按领域分组，组内并行组间串行 |
| iterative | 迭代 | 多轮循环，逐步精炼 |
| voting | 投票 | 同题多解，投票融合 |
| dynamic | 动态 | 决策节点+主路径+兜底分支 |

---

## 7. FusionStrategy（融合策略）

| 值 | 说明 | 适用场景 |
|---|------|---------|
| voting | 加权投票融合 | 分类/决策任务 |
| weighted | 加权融合 | 数值融合 |
| confidence_weighted | 置信度加权融合 | 动态置信度加权 |
| concatenation | 拼接融合 | 结果拼接 |
| best_of | 择优融合 | 选最优结果 |
| stacking | 堆叠融合（元学习器） | 多模型组合 |
| debate | 辩论融合 | 多智能体辩论裁决 |
| map_reduce | Map-Reduce 融合 | 大规模数据分治 |
| iterative | 迭代精炼融合 | 多轮迭代优化 |

---

## 8. PlanDynamicRoute（动态路由规则）

| 字段 | 类型 | 说明 |
|------|------|------|
| decision_node | String | 决策节点 ID |
| field | String | 判断字段（如 "success"） |
| operator | String | 比较运算符（如 "eq"） |
| value | Value | 比较值 |
| true_branch | Vec\<String> | 成立时激活的节点列表 |
| false_branch | Vec\<String> | 不成立时激活的节点列表 |

---

## 9. 存储架构

| 数据 | 存储方式 | 说明 |
|------|---------|------|
| Task / Node / Plan | SQLite | 持久化存储（experts_db.rs） |
| Expert 注册表 | 内存（HashMap） | scheduler 侧内存版，Phase 1 用 |
| 匹配结果 | 内存 | 不持久化，每次重新计算 |
| 会话历史 | 内存 + SQLite | 网关侧 experts_session.rs |
| 审计日志 | SQLite | 网关侧 experts_db.rs |

**生产级演进：** SQLite → PostgreSQL（主从）+ Redis（缓存）+ pgvector（向量检索）

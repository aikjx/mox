# mox_sys 知识图谱与通用关系模型

## 1. 关系分工

| 层 | 保存什么 | 真相来源 |
|---|---|---|
| 交易关系 | 用户、租户、订单、任务、审批等强一致字段 | SQL owner module |
| 语义关系 | `expert -> has_capability -> capability`、资源继承、相似案例 | mox_sys graph projection |
| 证据关系 | 来源表、文件、事件、版本、抽取模型和置信度 | `mox_sys_evidence`/事件日志 |
| 检索关系 | embedding、倒排、图算法分数 | 可重建索引 |

## 2. 节点

节点由 `graph_id + entity_type + entity_key` 唯一确定。`entity_key` 是业务稳定键，不直接暴露数据库自增值；`payload` 保存少量可演进属性，原始实体仍归 owner 模块管理。

节点必须记录：

- `tenant_id` 和 graph namespace；
- `entity_type`、稳定 key、展示 label；
- 来源模块/来源 ID；
- 有效时间 `valid_from/valid_to`；
- 状态、版本、敏感级别；
- 可选 `embedding_ref`，不把向量强绑定到 MySQL 表。

## 3. 关系

关系是有向事实：`from_node -> relation_type -> to_node`。关系类型定义方向、逆关系、对称性、传递性、基数和敏感级别；事实记录来源、证据、置信度和有效期。

关系代码使用小写 snake_case，例如：

```text
tenant_has_enterprise
enterprise_contains_org_unit
member_belongs_to_org
expert_has_capability
capability_requires_tool
case_solved_by_expert
resource_inherits_from
task_depends_on_task
```

禁止把 `can_view`、`can_edit` 这种计算权限直接写成用户事实；用户/组/角色与对象的关系由权限引擎根据授权模型计算。

## 4. 最优授权组合

SQL 保存稳定组织、角色、资源和租户边界；复杂资源继承可投影给关系授权引擎。建议同时保留：

```text
RBAC：member -> role -> permission
ABAC：tenant/org/status/time/context 条件
ReBAC：user/group -> relation -> object
```

访问判断顺序：租户边界 → 成员状态 → 组织数据范围 → 角色/权限 → 资源关系 → 字段脱敏。任何一层拒绝都不能由下一层放宽。

## 5. 图谱一致性

图谱是异步投影：

```text
SQL transaction
  -> outbox event
  -> graph projector
  -> idempotent upsert
  -> checkpoint / rebuild
```

投影器必须支持按租户、按 graph、按事件 offset 重建；删除事件采用 tombstone，避免迟到事件把删除对象重新写回。图查询结果需要返回 `evidence_id` 或来源事件，禁止无来源的“AI 推断关系”进入可信知识层。

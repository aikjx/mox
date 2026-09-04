# ARC-INDEX · 架构规范归一化（ARC-xx）

> 编号：**DOC-NORM-ARC-V1.0** · 归属：[README.md](README.md)（SSoT 枢纽）
> 内容：mox 六层架构栈、mox_sys 内核、关系模型、依赖规则、插件契约。

---

## 1. 六层架构栈（mox 模块化系统架构 modular stack）

| 层 | 真实资产 | 职责 | 权威文档 |
|----|----------|------|----------|
| L1 母版 | `mox_sys`（module-registry.yml） | 身份/租户/组织/IAM/审计/出盒/幂等/文件/图谱 | `docs/database/mox_sys/README.md` |
| L2 生成编排 | `meta`(codegen) / `mox-flow-primiflow-svc`(DAG) / verify+闸门 | 代码生成、流程编排、合规门禁 | `docs/database/mox_sys/module-registry.yml` |
| L3 知识图谱 | `kg`(property_graph/provenance/rebuild) / `relation-model.md` | 语义关系、来源、可重建投影 | `docs/database/mox_sys/relation-model.md` |
| L4 AI 闭环 | ChatView / InfiniteOptimizerView / `aiOptimize` / `moxOptimize` | 分析→生成→验证→优化→再生成 | `MOX-AI驱动mox 模块化系统架构平台-企业级设计-mox 模块化系统架构分析-v3.0.md` |
| L5 模板+插件 | TPL / PluginsView / McpView / MarketView | 业务形态模板、外部能力即插即用 | `docs/normalization/TPL-INDEX.md` |
| L6 领域包 | iam/meta/ea/kg/ai（可扩展行业包） | 行业专属能力装配 | `docs/database/mox_sys/module-registry.yml` |

---

## 2. mox_sys 内核模块（SSoT = module-registry.yml）

| 模块 | kind | tenant_mode | 依赖 | 能力 |
|------|------|-------------|------|------|
| `mox_sys` | core | hybrid | — | identity, tenancy, organisation, iam, audit, outbox, idempotency, file, settings, graph |
| `iam` | platform | hybrid | mox_sys | rbac, abac, rebac, sso, mfa |
| `meta` | platform | logical | mox_sys | **catalog, metadata, lowcode, codegen** |
| `ea` | domain | logical | mox_sys, iam | expert_registry, collaboration_dag, case_memory |
| `kg` | projection | logical | mox_sys | property_graph, provenance, rebuild |
| `ai` | domain | logical | mox_sys, kg | model_registry, run_trace, tool_call, metering |

安装顺序：`mox-v3.0-baseline.sql` → `mox_sys-template.sql` → 按 registry 装 iam/meta/ea/kg/ai → migration smoke test。增量 migration，禁止重建生产库。

---

## 3. 关系模型（kg 归一化，事实来源：relation-model.md）

- 节点：`graph_id + entity_type + entity_key`（稳定业务键，不暴露自增 ID）。
- 关系：有向事实 `from_node → relation_type → to_node`，小写 snake_case（`task_depends_on_task` / `expert_has_capability`）。
- 最优授权：RBAC(member→role→permission) + ABAC(tenant/org/status/time) + ReBAC(user/group→relation→object)，拒绝不可被下层放宽。
- 图谱一致性：`SQL → outbox → projector → idempotent upsert → checkpoint/rebuild`；删除用 tombstone；入可信层须带 `evidence_id`。

---

## 4. 依赖与跨域规则

- 跨模块只引用稳定 ID / 发布事件 / 图谱关系，不直接依赖对方私有表（ADR-09：`docs/enterprise/29-跨域依赖规则与架构一致性治理-ADR-09.md`）。
- 每租户业务表必须有 `tenant_id`；企业/部门字段用 `enterprise_id` / `org_unit_id`，禁 `org_id`。
- 状态/类型代码英文短码 + 版本化字典；二值字段 `Y/N`。

---

## 5. 插件契约（L5 归一化）

| 插件形态 | 落点 | 契约 |
|----------|------|------|
| AI 插件 | `PluginsView` | plugin manifest + SPI |
| MCP 工具 | `McpView` | MCP schema |
| 算子 | `MarketView`→`OperatorsView` | operator manifest |
| 领域包 | module-registry | module_code + SemVer |

---

## 6. 登记规则

- 架构文档 `ARC-{两位序号}-{中文短名}.md` 放 `docs/normalization/arch/`，须含 ADR 关联与依赖图。
- 跨文档引用 `docs/normalization/ARC-INDEX.md#章节`。

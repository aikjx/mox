# BP-GOV-政务审批 · 行业领域包流程样例（领域包 `gov`）

> 编号：**DOC-NORM-BP-GOV-V1.0** · 归属：[BP-INDEX.md](../BP-INDEX.md)（业务流程归一化）
> 定位：首个行业领域包样例——证明"任何行业业务系统 = mox_sys 母版 + 领域包 + 模块模板，无需重造轮子"。
> 复用底座：mox_sys（身份/租户/组织/IAM/审计/文件）+ iam(sso) + audit（ADR-09 RBAC/ABAC/ReBAC）。
> 复用模板：`TPL-05` 工作流 + `TPL-02` 树表（见 [TPL-INDEX.md](../TPL-INDEX.md)）。

---

## 1. 领域包装配（不写一行内核代码）

| 项 | 取值 | 来源 |
|----|------|------|
| 领域包标识 | `gov` | module-registry 扩展（kind=domain, tenant_mode=logical） |
| 复用内核 | mox_sys + iam + audit | 已有，零新增 |
| 业务形态 | 事项申报→受理→审批→出证→监管 | `TPL-05` 工作流 |
| 组织形态 | 委办局树（省/市/区/科室） | `TPL-02` 树表 |
| 权限 | 申报人/受理员/审批人/监管员 4 角色 | iam RBAC（复用现有 6 角色体系扩展） |

---

## 2. 归一化流程（5 阶段 SOP，与 BP-INDEX §1 一致）

| 阶段 | 政务审批动作 | 复用模板 | 现有落点 |
|------|--------------|----------|----------|
| 需求 | 事项要素采集 → 情形引导（问答式填表） | `TPL-06` AI 域 | `CaomeiView` 需求编译 |
| 架构 | 审批流建模（节点/边/条件分支）→ ER 图 | `TPL-05` + `TPL-02` | `aiGenerateFlowDiagram` / `aiGenerateErd` |
| 实现 | 一键生成：受理页 + 审批台 + 出证页 + 监管看板 | `meta` codegen（待实现） | `aiFullComplete` |
| 测试 | 流程贯通 + 权限矩阵 + 审计链验证 | `VAL-INDEX` | `aiDevTestFix` |
| 验收 | 发布闸门 → 入图（事项/审批/证照节点）→ 发布 KB | verify + 治理闸门 | `aiPublishArtifactsToKb` / `aiGenerateProjectGraph` |

---

## 3. 流程明细（政务审批主线）

```
申报人提交事项(TPL-02树表定位委办局)
  → 情形引导填表(TPL-06 AI 域自动生成表单)
  → 受理员受理(TPL-01 单表状态=已受理)
  → 审批人审批(TPL-05 工作流: 条件分支→并联/串联合议)
  → 通过→出证(TPL-01 单表+文件落 mox_sys file)
  → 监管员监管(TPL-04 图谱: 事项-审批-证照 关系入图, evidence_id 溯源)
```

---

## 4. 为什么"零重复造轮子"

- 身份/租户/组织/权限/审计/文件/流程/图谱/AI **全部来自 mox_sys + iam + kg + ai**，政务包只装"业务参数 + 流程定义 + 2 张页面模板参数"。
- 差异被**数据化、配置化、模板化**，而非代码化：新增一类事项 = 新增一条工作流定义 + 表单字段配置，平均从"数月研发"降到"配置即上线"。
- 本样例即"mox 模块化系统生成平台"首个行业落地证据；其余行业（金融/医疗/零售）按同构复制 `BP-<行业>` + 复用对应 `TPL`。

---

## 5. 登记与后续

- 已登记：[BP-INDEX.md](../BP-INDEX.md) §3 领域包流程表 `gov` 行。
- 待补：`meta` codegen 实现后，本样例的"实现"阶段由 manifest 驱动一键生成（见 [TPL-INDEX.md](../TPL-INDEX.md) §4 管线）。
- 跨文档引用统一 `docs/normalization/business/BP-GOV-政务审批.md`。

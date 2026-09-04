# 关图/璇玑（mox）· 文档归一化总索引（SSoT 枢纽）

> 编号：**DOC-NORM-HUB-V1.0**
> 定位：mox 模块化系统的**归一化文档单一事实源（SSoT）枢纽**。
> 上层愿景：[reports/html/codegen-normalization-plan/index.html](../reports/html/codegen-normalization-plan/index.html)
> 下层权威：各分类索引（BP / API / ARC / VAL / TPL）见本目录。
> 物理治理：沿用 `docs/DOC-NORMALIZATION-REPORT.md`（DOC-GOV-V1.0）目录职责与引用规则（仓根相对 `docs/<rel>`）。

---

## 0. 一句话定位

所有业务系统 = **mox_sys 母版 + 领域包 + 模块模板 + 插件**，由 AI 持续优化闭环生成并归一化。本枢纽把散落在 `docs/` 各处的设计、架构、模块、规范，收敛为 5 类归一化文档，使"任何行业建系统"都有统一可查的事实源。

---

## 1. 归一化分类（5 类 + SSoT）

| 类 | 编号 | 内容 | 本目录索引 | 主要事实来源 |
|----|------|------|-----------|-------------|
| 业务流程 | `BP-xx` | 5 阶段 SOP、领域包流程、项目联动/专家调度/知识沉淀流 | [BP-INDEX.md](BP-INDEX.md) | `对话开发系统-端到端流水线.mmd` / 功能图谱 §9 |
| 接口契约 | `API-xx` | 17 个 API 模块、191 端点、REST/事件/SPI 契约 | [API-INDEX.md](API-INDEX.md) | `frontend-ui/src/MODULE-MANIFEST.md` §4 |
| 架构规范 | `ARC-xx` | 六层栈、mox_sys 内核、ADR、关系模型、依赖规则 | [ARC-INDEX.md](ARC-INDEX.md) | `docs/database/mox_sys/` / `enterprise/29-...ADR-09.md` |
| 验证报告 | `VAL-xx` | verify 5 项、治理闸门 G1~G8、页面/API 验证矩阵 | [VAL-INDEX.md](VAL-INDEX.md) | `docs/DOC-NORMALIZATION-REPORT.md` / 功能图谱 §10 |
| 模板清单 | `TPL-xx` | 代码生成模板（单表/树表/主子表/图谱/工作流/AI 域） | [TPL-INDEX.md](TPL-INDEX.md) | `meta` codegen / `primiflow` / `MODULE-MANIFEST.md` |

**SSoT 铁律**（与 DOC-GOV-V1.0 一致）：
1. 命名 `{前缀}-{两位序号}-{中文短名}.md`；状态/类型/关系代码一律英文短码 + 版本化字典，二值字段 `Y/N`。
2. 引用统一仓根相对 `docs/<rel>`，禁止 `../`、裸名。
3. 跨文档引用带 `#anchor` 锚点；每篇文档头部声明权威等级 🟢/🟡/🟡归档。
4. 跨模块只引用稳定 ID / 事件 / 图谱关系，不直接依赖对方私有表。
5. 核心交易在事务库；图谱/向量/检索为可重建投影。

---

## 2. 平台分层 ↔ 文档映射

```
L6 行业领域包(domain pack)   → BP-INDEX（金融/医疗/政务/零售流程）
L5 模块模板+插件             → TPL-INDEX + API-INDEX（Plugins/MCP/算子商城）
L4 AI 持续优化闭环           → VAL-INDEX（优化产出回 verify + evidence 入图）
L3 mox 模块化系统架构知识图谱(kg)          → ARC-INDEX §关系模型 + API-INDEX(graph)
L2 生成与编排(meta+primiflow)→ TPL-INDEX + ARC-INDEX
L1 mox_sys 母版              → ARC-INDEX §内核 + API-INDEX(system/iam)
```

---

## 3. 现有 docs 总导航（依 DOC-GOV-V1.0 物理归位）

- 🟢 治理中心：`docs/enterprise/00-INDEX.md` · 术语表 `docs/GLOSSARY.md`
- 🟢 架构权威：`docs/architecture.md` · `docs/enterprise/02-architecture.md`
- 🟢 母版内核：`docs/database/mox_sys/`（README / module-registry / relation-model / module-contract / cross-database）
- 🟡 模块文档：`docs/modules/`（PrimiFlow 蓝图 / 业务流程 / mox-expert 系列）
- 🌐 交互中心：`docs/docs-hub/docs-hub.html` · `docs/architecture-hub.html`
- 📘 设计依据：`MOX-AI驱动mox 模块化系统架构平台-企业级设计-mox 模块化系统架构分析-v3.0.md`

---

## 4. 落地状态（与愿景对照）

| 能力 | 文档覆盖 | 代码成熟度 |
|------|----------|-----------|
| mox_sys 母版 | 🟢 ARC-INDEX §内核 | ✅ 已成型 |
| 前端功能图谱（38/191/33） | 🟢 API-INDEX / BP-INDEX | ✅ 全渲染 |
| 知识图谱 kg | 🟢 ARC-INDEX §关系模型 | ✅ 投影/溯源 |
| AI 优化闭环 | 🟡 VAL-INDEX（待串闭环） | ⚠️ 分析/优化有·闭环待串 |
| meta.codegen 生成 | 🟡 TPL-INDEX（待实现） | ⚠️ 已声明·待实现 |
| 完成页/历史回查 | 🟡 VAL-INDEX / BP-INDEX | ⚠️ 后端有·前端待串 |
| 行业领域包 | 🔴 BP-INDEX（尚无样例） | ❌ 未落地 |

---

> 本枢纽为活文档；新增归一化文档须在对应分类索引登记，并在 `docs/enterprise/00-INDEX.md` 变更记录留痕。

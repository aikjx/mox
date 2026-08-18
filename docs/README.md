# 信息关联关系图 · 全维分析文档导航

> 编号：**DOC-INDEX-V1.1**
> **权威治理中心**：企业级文档以 `docs/enterprise/00-INDEX.md` 为唯一治理入口与权威等级定义（文档集 `00`~`16`）。本文仅作**关图 / 全维专题的快捷分区索引**，不与 `00-INDEX` 重复，所有权威等级以 `00-INDEX` §1.2 为准。
> 顶层项目说明见仓库根 [`README.md`](../README.md)；数学内核见 `docs/modules/mathematical-foundation.md`。

---

## 1. 文档布局架构（归一化分区 · 物理真实状态）

```
docs/
├── enterprise/          # 🟢 权威治理中心（00-INDEX 为唯一入口，文档集 00~16）
│   ├── 00-INDEX.md                        # 唯一治理入口 + 版本注册 + RACI + 权威分级
│   ├── 01~16-*.md                          # 需求/架构/设计/业务/路线图/映射/全维明确/自动化/归档/验收/总纲/产品规范/P9
│   └── 璇玑-信息化系统开发验收报告-V1.0.md/.html   # 🟢 ISD-V1.0 交付验收报告（已并入本区）
├── specs/               # 🟢 企业级规范：PT-STD（Primi 架构）/ GR-STD（关图规范）/ OUS 业务规划
├── full-dimensional/    # 🟢 关图骨架 + 编号索引 + 治理台 API + 原始过程稿归档
│   ├── guantu-skeleton.md                  # 🟢 GR-STD-V1.0 关图骨架（REQ 根 + 六维绑定 + 偏离检测）
│   ├── xuanji-requirement-baseline.md      # 🟢 编号归一化收口（①-⑩ / C1-C8 → S1-S8）
│   ├── GOVERNANCE_CONSOLE_API_READY_20260816.md  # 🟢 治理台 API 契约（RBAC/审计链）
│   ├── xuanji-tracematrix.html             # 🟡 六维绑定可视化（与 full-dimensional 源同位）
│   └── (原始文档已归一承载于 guantu-skeleton，已迁 `docs/_archive/2026-08-16/`)
├── modules/             # 🟡 模块级设计 / 参考文档（market/automation/数学内核/业务流程/xuanji-expert 系列/设计蓝图）
├── graph/               # 关图机读产物：graph.json / graph.enterprise.json / guantu.req.json + requests/ 判重入口
├── ai-architecture/     # AI 架构专题：ai-unified-intelligent-system-architecture.html（AUS · L4 Agentic 闭环）
├── _archive/2026-08-16/ # 🟡 过程稿 / 验证快照归档（PrimiFlow-*-20260816、xuanji-expert-验证总结-20260816，非权威）
├── 璇玑-全维需求业务处理流程图-归一化企业级.md   # 🟢 AA-STD-V1.0 融合域唯一事实基准（位于 docs/ 根）
├── 璇玑-全维需求业务处理流程图-归一化企业级.html/.mmd  # 🟡 AA-STD 可视化（与 .md 同位）
├── 璇玑-璇玑验证子流程-归一化企业级.html     # 🟡 S6 验证网关子流程可视化（与 AA-STD 同位）
├── 璇玑-全维流水线.mmd                      # 🟡 全维流水线机读图（与 AA-STD 同位）
├── xuanji-system-business-architecture.html # 🟡 全维度分层架构交互图（源 architecture.md，同位 root）
├── architecture.md       # 🟢 OUS 父系统总架构（v7.0）
├── enterprise-architecture-analysis.md  # 🟢 双璇玑十四维能力矩阵
└── README.md             # 本文：关图/全维专题导航
```

> 说明：`docs/` 根仅保留 🟢 顶层权威文档（architecture / enterprise-architecture-analysis / AA-STD）与它们的同位可视化产物；其余按主题归位到 `modules/`、`full-dimensional/`、`enterprise/`、`_archive/`，杜绝"散落 + 索引声称已治理但物理未归位"的模糊态。

---

## 2. 编号归一化基准（唯一）

全维分析流程**唯一阶段基准 = AA-STD 的 S1-S8**；闸门 **G0-G3**；护栏 **G-A~G-E**；规范 **GR-STD / PT-STD**。编码层 ①-⑩ 与旧 C1-C8 仅作对照，不独立使用。详见 `docs/full-dimensional/xuanji-requirement-baseline.md` §2 与 `docs/enterprise/00-INDEX.md` §1.2。

---

## 3. 专题快速导航

| 查什么 | 去哪 |
| --- | --- |
| 企业级文档总入口 / 权威等级 / RACI | **`docs/enterprise/00-INDEX.md`** |
| 规范术语表（唯一事实源） | **`docs/GLOSSARY.md`**（DOC-GLOSSARY-V1.0） |
| 全维需求验收铁律（四闸门说死） | `docs/enterprise/07-全维需求明确书.md` |
| 全维自动化处理流水线（xuanji_optimize 8 步） | `docs/enterprise/08-全维自动化处理明确书.md` |
| 需求—架构映射 / 交付清单 | `docs/enterprise/06` `10` |
| 融合域流程基准（S1-S8） | `docs/璇玑-全维需求业务处理流程图-归一化企业级.md`（AA-STD） |
| 六维绑定（REQ→…→COD） | AA-STD §3 + `crates/primiflow/trace_matrix.md`（PT-STD） |
| 关图骨架 / REQ 根 / 偏离检测（GR-E6） | `docs/full-dimensional/guantu-skeleton.md` |
| 关图机读产物 | `docs/graph/` |
| 新需求判重入口（P9 先判重后立项） | `docs/graph/requests/README.md` |
| 关图 / Primi 规范 | `docs/specs/`（GR-STD / PT-STD / OUS） |
| 治理台 API / RBAC / 审计链 | `docs/full-dimensional/GOVERNANCE_CONSOLE_API_READY_20260816.md` |
| 归一化设计规范（reconcile/契约） | `docs/modules/xuanji-expert-normalization.md` |
| 模块级设计（商城/自动化/数学内核/业务流程） | `docs/modules/` |
| 交付验收报告（ISD-V1.0） | `docs/enterprise/璇玑-信息化系统开发验收报告-V1.0.md` |
| 过程稿 / 验证快照（仅供追溯） | `docs/_archive/2026-08-16/` |

> 四份原始文档（关图骨架定义 / TraceMatrix / 测试验证报告 / 业务处理流程图）已迁至 `docs/_archive/2026-08-16/`，其内容为**过程稿归档**，已归一承载于 AA-STD 与 `docs/full-dimensional/guantu-skeleton.md`，查阅以 AA-STD 为准（见 `docs/full-dimensional/00-README.md`）。

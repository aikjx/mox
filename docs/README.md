# 信息关联关系图 · 全维分析文档导航

> 编号：**DOC-INDEX-V1.1**
> **权威治理中心**：企业级文档以 `docs/enterprise/00-INDEX.md` 为唯一治理入口与权威等级定义（文档集 `00`~`10`）。本文仅作**关图 / 全维专题的快捷分区索引**，不与 `00-INDEX` 重复，所有权威等级以 `00-INDEX` §1.2 为准。
> 顶层项目说明见仓库根 [`README.md`](../README.md)；数学内核见 `docs/mathematical-foundation.md`。

---

## 1. 文档布局架构（归一化分区）

```
docs/
├── enterprise/          # 🟢 权威治理中心（00-INDEX 为唯一入口，docs 集 00~10）
├── specs/               # 🟢 企业级规范：PT-STD（Primi 架构）/ GR-STD（关图规范）/ OUS 业务规划
├── full-dimensional/    # 🟡 全维分析（璇玑）专题：AA-STD + 关图骨架 + 治理台 API + 原始文档归档
├── graph/               # 关图机读产物：graph.json / graph.enterprise.json / guantu.req.json
├── ai-architecture/  enterprise-architecture-analysis.md  # AI 架构 / 双璇玑十四维能力矩阵
├── 璇玑-全维需求业务处理流程图-归一化企业级.md  # 🟢 AA-STD-V1.0 融合域唯一事实基准
├── xuanji-expert-*.md  PrimiFlow-*-20260816.md           # 🟡 过程稿/验证快照（结论已沉淀 enterprise/）
└── *.html  *.mmd        # 🟡 可视化产物（以同名 .md 为源）
```

---

## 2. 编号归一化基准（唯一）

全维分析流程**唯一阶段基准 = AA-STD 的 S1-S8**；闸门 **G0-G3**；护栏 **G-A~G-E**；规范 **GR-STD / PT-STD**。编码层 ①-⑩ 与旧 C1-C8 仅作对照，不独立使用。详见 `full-dimensional/xuanji-requirement-baseline.md` §2 与 `enterprise/00-INDEX.md` §1.2。

---

## 3. 专题快速导航

| 查什么 | 去哪 |
| --- | --- |
| 企业级文档总入口 / 权威等级 / RACI | **`docs/enterprise/00-INDEX.md`** |
| 全维需求验收铁律（四闸门说死） | `docs/enterprise/07-全维需求明确书.md` |
| 全维自动化处理流水线（xuanji_optimize 8 步） | `docs/enterprise/08-全维自动化处理明确书.md` |
| 需求—架构映射 / 交付清单 | `docs/enterprise/06` `10` |
| 融合域流程基准（S1-S8） | `docs/璇玑-全维需求业务处理流程图-归一化企业级.md`（AA-STD） |
| 六维绑定（REQ→…→COD） | AA-STD §3 + `crates/primiflow/trace_matrix.md`（PT-STD） |
| 关图骨架 / REQ 根 / 偏离检测（GR-E6） | `docs/full-dimensional/guantu-skeleton.md` |
| 关图机读产物 | `docs/graph/` |
| 关图 / Primi 规范 | `docs/specs/`（GR-STD / PT-STD / OUS） |
| 治理台 API / RBAC / 审计链 | `docs/full-dimensional/GOVERNANCE_CONSOLE_API_READY_20260816.md` |
| 归一化设计规范（reconcile/契约） | `docs/xuanji-expert-normalization.md` |
| 测试验证事实 | `docs/xuanji-expert-验证总结-20260816.md` / `docs/PrimiFlow-三层递进开发-验证总结-20260816.md` |

> `full-dimensional/` 内的四份原始文档（关图骨架定义 / TraceMatrix / 测试验证报告 / 业务处理流程图）为**过程稿归档**，其内容已归一承载于 AA-STD 与 `guantu-skeleton.md`，查阅以 AA-STD 为准（见 `full-dimensional/00-README.md`）。

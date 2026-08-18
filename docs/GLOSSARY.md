# 企业级规范术语表 (GLOSSARY · DOC-GLOSSARY-V1.0)

> **唯一事实源（Single Source of Truth）**：本文档是 `infotopograph` 项目全部文档的**权威术语基准**。
> 所有 `.md` 文档的术语以本表为准；各权威文档在文末以「见 `docs/GLOSSARY.md`」引用，**禁止各自复制维护术语表以避免漂移**。
> 命名约定：中文术语为主，英文/缩写首次出现附原文；专有名词大小写固定。

---

## 1. 系统与架构主体

| 术语 | 英文 / 缩写 | 定义 | 载体 |
|------|------------|------|------|
| **璇玑** | Xuánjī System / `xuanji-expert` | 归一化 IR 驱动的元调度诊断系统：双璇玑十四维并行诊断 → 裁决 → flow-ai 求解 → ⛨验证网关 → 治理闸门 → 出码/出图。 | `xuanji-expert` crate |
| **关图** | 信息关联关系图 / GR-STD-V1.0 | 「一切皆是信息」：所有信息实体抽象为**节点**，关联关系抽象为**边**，以需求为根节点无限扩展，构成全栈信息关联图，作为项目唯一基准。 | `docs/specs/GR-STD-信息关联关系图开发规范-V1.0.md` |
| **AA-STD** | 全维需求业务处理流程图-归一化企业级 | 融合域**需求事实基准**，承载 REQ→FUN→BIZ→ALG→TSK→COD 五向绑定的归一化主流程。 | `docs/璇玑-全维需求业务处理流程图-归一化企业级.md` |
| **PT-Primi / PrimiFlow** | 全域拓扑原语架构 V1.0 | operator-unified-system 之上的**元调度大脑层**（meta-scheduling brain）；κ-τ 拓扑原语调度，守恒律 `C² = κ² + τ²`。 | `docs/specs/PT-Primi-架构规范-V1.0-完整版.md` |
| **OUS** | operator-unified-system | 算子统一系统（Rust v3.0.0-ai-powered，多 crate 架构），提供算子侧稳定能力。 | 仓库根 `crates/` |
| **双璇玑十四维** | Dual-Xuánjī 14-Dim | 业务 7 维 + 开发 7 维并行诊断的体系化维度模型。 | 璇玑系统 |
| **全维** | full-dimensional | 覆盖需求/架构/设计/业务/测试/验收/归档的全维度工程视图。 | `docs/full-dimensional/` |

## 2. 核心机制与契约

| 术语 | 定义 | 说明 |
|------|------|------|
| **TraceMatrix / 六维绑定** | `REQ→FUN→BIZ→ALG→TSK→COD` 的逐层可追溯绑定矩阵，保证零孤儿节点。 | 承载于 AA-STD §3 + `crates/primiflow/trace_matrix.md` |
| **五向绑定** | requirement→function→algorithm→flow→code 的端到端可追溯链。 | 见 glossary §1 "AA-STD" |
| **κ-τ 拓扑原语调度** | PrimiFlow 原生调度算法：κ（曲率/结构复杂度）与 τ（扭转/时序约束）守恒。 | 守恒律 `C² = κ² + τ²` |
| **⛨ 璇玑验证网关** | 闭环出码/出图前的**最高权限验证网关**，对诊断结论做最终裁决与放行。 | 治理闸门上游 |
| **治理闸门** | Governance Gate：在出码前对合规性、零死代码、禁伪代码做门禁拦截。 | `govern` crate |
| **归一化** | Normalization：将分散/重复/过程稿文档统一为单一事实源、统一命名/编号/引用/锚点的治理动作。 | 见 `docs/DOC-NORMALIZATION-REPORT.md` |
| **判重闸门 (P9)** | 需求判重与去噪闸门，防止重复需求进入流水线。 | enterprise/16 |
| **关图骨架** | guantu-skeleton：GR-STD 的 REQ 根 + 六维绑定骨架 + 偏离检测承载文件。 | `docs/full-dimensional/guantu-skeleton.md` |

## 3. 文档治理等级

| 标记 | 含义 |
|------|------|
| 🟢 权威 (Authoritative) | 以该文档为准的单一事实源 |
| 🟡 参考 / 过程稿 (Reference / Draft) | 仅供追溯，结论已沉淀入 🟢 文档 |
| ⛨ 网关级 (Gateway) | 最高权限验证/裁决节点 |

## 4. 常见产物与图类型

| 术语 | 含义 | 归属 |
|------|------|------|
| 需求辐射图 | 以需求为根向关联实体辐射的关图视图 | 关图 / PrimiFlow |
| 业务流程图 | 业务处理 S1–S8 主流程可视化 | AA-STD |
| ER 图 | 实体-关系图 | 关图 |
| 功能关联图 | PrimiFlow 输出的功能关联拓扑视图 | PrimiFlow（与关图概念邻近，不混用） |
| 定时任务甘特 | 定时任务时序甘特视图 | PrimiFlow |

## 5. 命名与大小写约定（强制）

- 专有名词固定写法：**璇玑**（非「旋玑」）、**关图**（信息关联关系图简称）、**OUS**、**AA-STD**、**GR-STD**、**PT-Primi**（非 PT-PRIMI）、**TraceMatrix**（非 Tracematrix）。
- 英文专名首次出现附中文：*璇玑 (Xuánjī)*；后续可用简称。
- 文档引用一律使用仓根相对路径 `docs/<rel>`，禁止 `../`、裸名、同级简写混用。
- 编号章节配 anchor 锚点，便于跨章节引用与导航。

---

*本术语表为活文档，新增专有名词须同步登记；变更须经 `docs/enterprise/00-INDEX.md` 变更记录留痕。*

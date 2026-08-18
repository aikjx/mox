# 璇玑 · 全维分析需求 文档归一化索引（Single Source of Truth 收口）

> 编号：**AA-STD-IDX-V1.0**
> 目的：将原始四份散落文档（关图骨架、TraceMatrix、测试验证报告、业务处理流程图）的内容**归一**到 `docs/` 既有企业级体系，消除编号并存与重复描述，仅做索引与交叉引用，不重复承载内容。
> 唯一流程基准：`璇玑-全维需求业务处理流程图-归一化企业级.md`（AA-STD，S1-S8 / G0-G3）

---

## 1. 原始文档 → docs 归一化映射（消重）

| 原始散落文档 | 归一去向（docs 内权威承载） | 处理方式 |
| --- | --- | --- |
| 关图骨架定义.md | `docs/full-dimensional/guantu-skeleton.md`（GR-STD-V1.0） | 已建，独立承载 REQ 根 + 六维绑定骨架 + 偏离检测 |
| 璇玑-…-TraceMatrix-六维绑定追溯.md | AA-STD §3（六维绑定）+ `crates/primiflow/trace_matrix.md`（PT-STD） | 并入既有，不再单列 |
| 璇玑-…-测试分析验证报告.md | `xuanji-expert-验证总结-20260816.md`（164 项全绿）+ `PrimiFlow-三层递进开发-验证总结-20260816.md` | 并入既有验证总结 |
| 璇玑-…-业务处理流程图.md | `璇玑-全维需求业务处理流程图-归一化企业级.md`（AA-STD）+ `docs/modules/xuanji-expert-normalization.md`（归一化规范） | 并入既有基准与规范书 |

> 结论：四份原始文档的内容在 `docs/` 内**均已有权威承载**，本索引仅负责编号收口与防漂移，不重复正文。

---

## 2. 编号归一化收口（消除三套编号并存）

历史上同一流程被三套编号描述，造成归一化漂移。自本索引起**以 AA-STD 的 S1-S8 为唯一阶段基准**：

| 唯一基准（AA-STD） | 编码层 ①-⑩（对照，已弃用） | 闸门（G0-G3） | 护栏（G-A~G-E） |
| --- | --- | --- | --- |
| **S1** 需求接入 | ① `normalize_requirement` | — | G-A |
| **S2** 归一化建模 | ② `auto_dimension` 建模 | G0 | G-A |
| **S3** 双璇玑并行诊断 | ③ 七/十四专家并行 | — | — |
| **S4** 归一化裁决 | ④ `reconcile` | G1 | — |
| **S5** flow-ai 最优求解 | ⑤ `optimize` + 草稿 codegen | — | — |
| **S6** ⛨璇玑验证 | ⑥ `verify` | G2 最高否决 | — |
| **S7** 治理闸门 | ⑨ `govern` | G3 | G-C 三证 |
| **S8** 出码·双向校验 | ⑦ `emit` + ⑧ 双向 | — | G-C |
| （审计闭环，横切） | ⑩ 审计 | — | G-D 署名 |

**顺序分歧唯一裁决**：编码层把 `emit`(⑦) 排在治理闸门(⑨)前，与 AA-STD「闸门先于出码」相反。采用 AA-STD 时序——S8 出码仅生成**草稿代码**，须经 S7 `approved` 方可交付（护栏 G-C「三证齐全方可出码」：S6 `verify` 通过 ＋ S8 双向一致 ＋ S7 `Approved`）。后续不再并存两套编号。闸门 G0-G3（阶段控制点）与护栏 G-A~G-E（跨阶段原则）为不同层级，不混用。

---

## 3. 归一化后文档体系（防重复导航）

| 文档 | 承载内容 | 互不重复定位 |
| --- | --- | --- |
| `璇玑-全维需求业务处理流程图-归一化企业级.md` | 流程唯一基准：S1-S8、G0-G3、双璇玑十四维矩阵、⛨5 检查、审计链、关图 CI | 时序事实源 |
| `docs/modules/xuanji-expert-normalization.md` | 归一化规范：单图多维铁律、reconcile 细节、IN-/OUT-/KB-* 契约、P0-P2 落地 | 设计规范书 |
| `xuanji-expert-验证总结-20260816.md` | 验证报告：164 项测试全绿、clippy 0 告警、修复记录 | 验证事实 |
| `docs/modules/xuanji-expert-business-requirements.md` | 业务需求 SRS + 21 BR/9 NFR 追踪矩阵 | 需求事实 |
| `docs/full-dimensional/guantu-skeleton.md` | REQ 根（D01-D13/R01-R08）+ 六维绑定骨架 + 偏离检测（GR-E6）+ CI 门禁 | 关图承载 |
| `docs/modules/xuanji-expert-alliance-fusion-flows.md` | 联盟融合流程（业务维度） | 业务扩展 |
| `PrimiFlow-*.md`（系列） | PrimiFlow κ-τ 引擎与三层递进开发验证 | 引擎侧验证 |

> 任一新增/修改须以本索引 §2 编号与 §3 分工为准，避免再产生第四套编号或内容重复。

---

## 4. 闭环引用（原四份文档诉求的最终落点）

- **「需求有根」** → `docs/full-dimensional/guantu-skeleton.md` §3（REQ:D04 Bind 到 `crates/xuanji-expert/src/lib.rs`）。
- **「六维绑定可追溯」** → AA-STD §3（REQ→FUN→BIZ→ALG→TSK→COD）+ `crates/primiflow/trace_matrix.md`（PT-STD）。
- **「全维分析流程」** → AA-STD §1（S1-S8）+ `docs/modules/xuanji-expert-normalization.md` §1-§2。
- **「测试验证通过」** → `xuanji-expert-验证总结-20260816.md`（164 项 0 失败）。
- **「企业级治理」** → AA-STD §4（G0-G3 + 护栏 G-A~G-E + 审计链）+ `docs/modules/xuanji-expert-normalization.md` §1.4。

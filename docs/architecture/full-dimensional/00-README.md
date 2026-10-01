# full-dimensional/ 分区说明（全维分析·璇玑专题）

> 本目录为「全维分析（璇玑）」专题文档区，隶属 `docs/` 归一化布局。
> **权威治理中心**：`docs/enterprise/00-INDEX.md`（文档集 `00`~`16`，统一权威等级与 RACI）。
> 本目录内容等级遵循 `00-INDEX` §1.2：🟢 权威 / 🟡 过程稿或可视化产物。
> AA-STD（融合域唯一事实基准）物理位于 `docs/` 根（`docs/璇玑-全维需求业务处理流程图-归一化企业级.md`），本文不重复承载。

---

## 文件等级与用途

| 文件 | 等级 | 说明 |
| --- | --- | --- |
| `docs/architecture/full-dimensional/guantu-skeleton.md` | 🟢 权威（GR-STD-V1.0） | 关图骨架：REQ 根（D01-D13/R01-R08）+ 六维绑定 + 偏离检测（GR-E6） |
| `docs/architecture/full-dimensional/mox-requirement-baseline.md` | 🟢 索引 | 编号归一化收口（①-⑩ / C1-C8 → S1-S8）+ 交叉引用 |
| `docs/architecture/full-dimensional/GOVERNANCE_CONSOLE_API_READY_20260816.md` | 🟢 权威 | 治理台前端 API 就绪（v3.0，RBAC/审计链契约） |
| `docs/architecture/full-dimensional/mox-tracematrix.html` | 🟡 可视化 | 六维绑定 TraceMatrix 渲染图（源为 `_archive/2026-08-16/璇玑-全维分析-TraceMatrix-六维绑定追溯.md`） |
| `docs/_archive/2026-08-16/关图骨架定义.md` | 🟡 过程稿（已归档） | 原始文档，内容已承载于 `docs/architecture/full-dimensional/guantu-skeleton.md` |
| `docs/_archive/2026-08-16/璇玑-全维分析-TraceMatrix-六维绑定追溯.md` | 🟡 过程稿（已归档） | 原始文档，六维绑定已归并于 AA-STD §3 + `crates/primiflow-core/trace_matrix.md` |
| `docs/_archive/2026-08-16/璇玑-全维分析需求-测试分析验证报告.md` | 🟡 过程稿（已归档） | 原始文档，验证事实已沉淀 `docs/_archive/2026-08-16/mox-expert-验证总结-20260816.md`（164 项全绿） |
| `docs/_archive/2026-08-16/璇玑-全维分析需求业务处理流程图.md` | 🟡 过程稿（已归档） | 原始文档，流程已归一于 AA-STD（S1-S8） |

> **编号一致性**：原始过程稿内仍含旧 `C1-C8` 编号，仅为历史痕迹；全维流程以 AA-STD 的 `S1-S8` 为唯一基准，旧编号不作为规范引用。
> **可视化同位**：`mox-tracematrix.html` 与源过程稿 `.md` 同位存放于本目录；其余 `*.html`/`*.mmd` 均与各自源 `.md` 同位（root / `modules/` / `enterprise/`）。

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 需求、配置维度、流程、模块、契约、测试与证据 |
| 处理与边界 | 维护追踪矩阵与缺口；配置保存/代码完成/运行验证/发布就绪分别登记 |
| 输出 | 全维追踪与待核项，不计算伪精确完成率 |
| 维护角色 | 需求追踪owner（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/standards/lowcode-dynamic-configuration.md#acceptance) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。

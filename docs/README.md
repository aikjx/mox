# 璇玑 RelGraph · 信息关联关系图 · 全维分析文档导航（统一命名版）

> 编号：**DOC-INDEX-V1.2**
> **权威治理中心**：企业级文档以 `docs/enterprise/00-INDEX.md` 为**唯一治理入口**与权威等级定义（文档集 `00`~`18` 共 19 份）。本文仅作**关图 / 全维专题的快捷分区索引**，不与 `00-INDEX` 重复，所有权威等级以 `00-INDEX` §1.2 为准。
> **🟢 最高级权威（统摄全局 · 第一级 L0）**：[`docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`](enterprise/18-全域顶层总设计-三联盟模式-V1.0.md)（TOP-MASTER），三联盟签署；所有下文索引条目不得与 18 声明冲突。
> **唯一术语事实源**：[`docs/GLOSSARY.md`](GLOSSARY.md)（DOC-GLOSSARY-V1.1，已注册 7 新术语：璇玑 RelGraph / 三联盟模式 / 六层金字塔 / 八层图谱 L0~L7 / 四归三连铁律 / 9 里程碑 M0~M8 / 八大算法家族 A1~A8）。
> 顶层项目说明见仓库根 [`README.md`](../README.md)；数学内核见 `docs/modules/mathematical-foundation.md`。

---

## 1. 文档布局架构（归一化分区 · 物理真实状态 · M0 全域归一化对齐）

```
docs/
├── enterprise/          # 🟢 权威治理中心（00-INDEX 为唯一入口，文档集 00~18）
│   ├── 00-INDEX.md                        # 唯一治理入口 + 分级权威 L0~L4 + RACI + 版本注册 + 三联盟阅读路径
│   ├── 18-全域顶层总设计-三联盟模式-V1.0.md  # 🟢🟢 第一级 L0（TOP-MASTER，12 章统摄全局），三联盟共同签署
│   ├── 01~17-*.md                          # 需求/架构/设计/业务/路线图/映射/全维明确/自动化/归档/验收/总纲/产品规范/P9
│   └── 璇玑-信息化系统开发验收报告-V1.0.md/.html   # 🟢 ISD-V1.0 交付验收报告（已并入本区）
├── specs/               # 🟢 企业级规范：PT-STD（Primi 架构）/ GR-STD（关图规范）/ OUS 业务规划
├── full-dimensional/    # 🟢 关图骨架 + 编号索引 + 治理台 API + 原始过程稿归档
│   ├── guantu-skeleton.md                  # 🟢 GR-STD-V1.0 关图骨架（REQ 根 + 六维绑定 + 偏离检测）
│   ├── mox-requirement-baseline.md      # 🟢 编号归一化收口（①-⑩ / C1-C8 → S1-S8）
│   ├── GOVERNANCE_CONSOLE_API_READY_20260816.md  # 🟢 治理台 API 契约（RBAC/审计链）
│   ├── mox-tracematrix.html             # 🟡 六维绑定可视化（与 full-dimensional 源同位）
│   └── (原始文档已归一承载于 guantu-skeleton，已迁 `docs/_archive/2026-08-16/`)
├── modules/             # 🟡 模块级设计 / 参考文档（market/automation/数学内核/业务流程/mox-expert 系列/设计蓝图）
├── graph/               # 关图机读产物：graph.json / graph.enterprise.json / guantu.req.json + requests/ 判重入口
├── ai-architecture/     # AI 架构专题：ai-unified-intelligent-system-architecture.html（AUS · L4 Agentic 闭环）
├── _archive/2026-08-16/ # 🟡 过程稿 / 验证快照归档（PrimiFlow-*-20260816、mox-expert-验证总结-20260816，非权威）
├── 璇玑-全维需求业务处理流程图-归一化企业级.md   # 🟢 AA-STD-V1.0 融合域唯一事实基准（位于 docs/ 根）
├── 璇玑-全维需求业务处理流程图-归一化企业级.html/.mmd  # 🟡 AA-STD 可视化（与 .md 同位）
├── 璇玑-璇玑验证子流程-归一化企业级.html     # 🟡 S6 验证网关子流程可视化（与 AA-STD 同位）
├── 璇玑-全维流水线.mmd                      # 🟡 全维流水线机读图（与 AA-STD 同位）
├── mox-system-business-architecture.html # 🟡 全维度分层架构交互图（源 architecture.md，同位 root）
├── architecture.md       # 🟢 OUS 父系统总架构（v7.0 · L2 Rust 自研底座视角）
├── enterprise-architecture-analysis.md  # 🟢 双璇玑十四维能力矩阵
├── GLOSSARY.md           # 🟢 唯一术语事实源（DOC-GLOSSARY-V1.1 · 7 新术语已注册）
├── DOC-NORMALIZATION-REPORT.md  # 🟢 全仓 docs 归一化治理报告（DOC-GOV-V1.0）
└── README.md             # 本文：关图/全维专题导航
```

> 说明：`docs/` 根仅保留 🟢 顶层权威文档（architecture / enterprise-architecture-analysis / AA-STD）与它们的同位可视化产物；其余按主题归位到 `modules/`、`full-dimensional/`、`enterprise/`、`_archive/`，杜绝"散落 + 索引声称已治理但物理未归位"的模糊态。

---

## 2. 编号归一化基准（唯一）

全维分析流程**唯一阶段基准 = AA-STD 的 S1-S8**；闸门 **G0-G3**；护栏 **G-A~G-E**；规范 **GR-STD / PT-STD**。编码层 ①-⑩ 与旧 C1-C8 仅作对照，不独立使用。详见 `docs/full-dimensional/mox-requirement-baseline.md` §2 与 `docs/enterprise/00-INDEX.md` §1.2。

---

## 3. 三联盟 × 专题快速导航（四归三连推荐路径）

> 三联盟推荐差异化阅读路径（对齐 00-INDEX §0 阅读顺序）：

| 查什么 | 产品联盟入口（需求 & 验收铁律） | 算法联盟入口（图/算法 & 对账） | 开发联盟入口（工程 & 落地 & 路径） |
| --- | --- | --- | --- |
| 企业级文档总入口 / 权威等级 / RACI / 三联盟阅读路径 | **`docs/enterprise/00-INDEX.md`** → 权威链 L0→L4 表 | **`docs/enterprise/00-INDEX.md`** | **`docs/enterprise/00-INDEX.md`** → §1 表主责联盟列 |
| **🟢 最高级权威 L0（三联盟必读首件）** | [`enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`](enterprise/18-全域顶层总设计-三联盟模式-V1.0.md) TOP-MASTER 12 章 | 同左（§二六层金字塔 / §三八层图谱 / §四 10BP / §五八大算法 / §八 9 里程碑） | 同左（§六工程标准 / §七目录优化 / §九测试体系） |
| 规范术语表（唯一事实源 · 7 新术语） | **`docs/GLOSSARY.md`**（DOC-GLOSSARY-V1.1）· 三联盟术语 | 同左 · 图谱/算法 7 大新术语 | 同左 · 路径格式（`platform/domains/` / `frontend-ui/`） |
| 需求铁律（四闸门 + 三联盟四条铁规 All-01~04） | `docs/enterprise/07-全维需求明确书.md` · **首读** | 同左 · All-02 判重铁规 | 同左 · All-03/04 四归三连 & 自证自验 |
| 自动化流水线 8 步 + 每步主责联盟 | `docs/enterprise/08-全维自动化处理明确书.md` | 同左 · ②③④⑤⑥ 步责任 | 同左 · ①⑦⑧ 步责任 |
| 三联盟 RACI 矩阵（18 行映射） | `docs/enterprise/06-requirements-architecture-map.md` §5 | 同左 · A1~A8 & 图谱建模 R 行 | 同左 · 工程/安全/运维/前端 R 行 |
| 融合域流程基准（S1-S8 / G0-G3） | `docs/璇玑-全维需求业务处理流程图-归一化企业级.md`（AA-STD） | `docs/enterprise/02-architecture.md` §0.2 八大算法家族 | 同左 · 08 自动流水线每步落点 |
| 六维绑定（REQ→…→COD） | AA-STD §3 + `docs/modules/algorithm-verification.md` 对账（AV-STD） | `docs/enterprise/02` §二 八层图谱 L0~L7 | `platform/domains/primiflow-fusion` sixdim 注册表 |
| 关图骨架 / REQ 根 / 偏离检测（GR-E6） | `docs/full-dimensional/guantu-skeleton.md` | 同左 · CNM 社区 & 偏离 BFS | `tools/guantu_gate.py` CI 闸门 |
| 关图机读产物 / 新需求 P9 判重入口（先判重后立项） | `docs/graph/requests/README.md` · 判重报告归档 | `tools/info-graph dedup` 子图匹配 A1+A5 | 同左 · guantu_gate.py |
| 关图 / Primi / OUS 规范 | `docs/specs/`（GR-STD / PT-STD / OUS）· BP-10 文档治理 | 同左 · GR-STD 12 节点 / 7 边 → 八层图谱扩集对齐 | 同左 · ADR-DOC-004/005 执行 |
| 治理台 API / RBAC / 审计链 | `docs/full-dimensional/GOVERNANCE_CONSOLE_API_READY_20260816.md` | 同左 · A7 CEM 指标 | 同左 · rbac_audit_middleware 管线 |
| 10 大标准业务流程（BP-1~10 · 6 字段齐） | `docs/enterprise/04-business-processing.md` 首读 | BP-6（融合 A1~A8）· BP-9（A5 激活扩散判重） | BP-2/4/5/8/10（工程落地实现） |
| 9 里程碑 M0~M8 · L0/L1/L2 三级验收 | `docs/enterprise/05-iteration-roadmap.md` §7 · 产品联盟目标定义 | 同左 · M2/M8 算法里程碑 | 同左 · M0/M3/M4/M5/M6 工程里程碑 |
| 模块级设计（商城/自动化/数学内核/业务流程） | `docs/modules/`（产品视角） | `docs/modules/mathematical-foundation.md`（六大公理）· `algorithm-verification.md`（L1/L2/L3） | `docs/modules/`（代码落点） |
| 对外交付 & 客户签署（三联盟五签） | `docs/enterprise/10-企业级交付清单.md` · 第一类「顶层设计交付物」 | 同左 · 第三类「质量 & 验收证据」算法对账项 | 同左 · 第二类「能力交付物」路径零老化 |
| 交付验收报告（ISD-V1.0 · 对外附件） | `docs/enterprise/璇玑-信息化系统开发验收报告-V1.0.md` | 同左 · 第 4 类算法合规 | 同左 · 第 3 类工程合规 |
| 过程稿 / 验证快照（仅供追溯 · 非权威） | `docs/_archive/2026-08-16/` | 同左 · 算法对账原始脚本快照 | 同左 · 集成测试原始日志 |

> 四份原始文档（关图骨架定义 / TraceMatrix / 测试验证报告 / 业务处理流程图）已迁至 `docs/_archive/2026-08-16/`，其内容为**过程稿归档**，已归一承载于 AA-STD 与 `docs/full-dimensional/guantu-skeleton.md`，查阅以 AA-STD 为准（见 `docs/full-dimensional/00-README.md`）。

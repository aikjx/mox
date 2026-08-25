# docs/ 企业级归一化标准与审计报告（DOC-GOV-V1.0）

> 编号：**DOC-GOV-V1.0**
> 生成时间：2026-08-18
> 适用范围：`infotopograph` 仓库 `docs/` 全量文档（结构归一化后，53 份 Markdown + 可视化/JSON 产物）
> 治理入口：`docs/enterprise/00-INDEX.md`（唯一权威等级定义）
> 配套导航：`docs/README.md`（关图/全维专题索引）

---

## 0. 执行摘要（一句话结论）

`docs/` 此前**物理布局与治理索引不一致**：根目录散落 7 份过程稿、10 份松散模块文档、1 份未注册的验收报告；索引声称 `guantu-skeleton.md` / `mox-requirement-baseline.md` 位于 `full-dimensional/` 但物理上在根；可视化产物未与源 `.md` 同位；内部交叉引用格式不统一。本次已**全部归位、修复引用、统一口径、补齐治理缺口**，使"索引声明 = 物理事实"。

---

## 1. 审计发现（明确、不模糊）

| # | 发现 | 严重度 | 事实依据 |
|---|------|:--:|----------|
| F1 | 无字节级重复文件 | — | 65 份文件 sha256 两两比对，0 对相同 |
| F2 | 索引路径与物理位置错位 | 🔴 | `00-INDEX.md` / `full-dimensional/00-README.md` 指向 `docs/full-dimensional/guantu-skeleton.md`、`docs/full-dimensional/mox-requirement-baseline.md`，但两文件实际在 `docs/` 根（FIX-01 已纠正） |
| F3 | 根目录散落过程稿 | 🔴 | 7 份 `*-20260816*.md`（PrimiFlow-/mox-expert-验证总结-）混在权威文档区，与"过程稿非权威"声明矛盾（FIX-02 归档） |
| F4 | 松散模块文档未归类 | 🟠 | `market-module` / `automation-module` / `mathematical-foundation` / `business-process-flows` / `business-process-flowcharts` / `mox-expert-*`(4) / `PrimiFlow-设计蓝图` 共 10 份游离根目录（FIX-03 归 `modules/`） |
| F5 | 验收报告未注册 | 🟠 | `璇玑-信息化系统开发验收报告-V1.0.{md,html}`（ISD-V1.0）在根目录，未被 `00-INDEX` 任何表登记（FIX-04 入 `enterprise/` 并注册） |
| F6 | 可视化产物未同位 | 🟡 | `mox-tracematrix.html`、`mox-flow.html` 与各自源 `.md` 不同目录（FIX-05 同位化） |
| F7 | 交叉引用格式不统一 | 🟠 | 同一被引文件存在 `docs/xxx`、`../xxx`、`xxx`、`full-dimensional/xxx` 四种写法，且移动后大面积失效（FIX-06 全量重写） |
| F8 | 索引"散落文档"说明过时 | 🟡 | `00-INDEX` §1.2 仍称"根目录散落文档"，与归位后事实不符（FIX-07 改写） |
| F9 | 悬空引用（GAP-1） | 🔴→✅ | `enterprise-architecture-analysis.md:216` 引用 `algorithm-verification.md`，原文件在 `docs/` 全树不存在；**本轮已新建 `docs/modules/algorithm-verification.md`（AV-STD-V1.0）补全结构，引用同步修正为 `docs/modules/algorithm-verification.md`（FIX-10）** |
| F10 | 索引计数错误 | 🟡 | `00-INDEX` §0 称"16 份（00~16）"，00~16 实为 17 份（FIX-08 修正为 17） |
| F11 | 索引文件名错误 | 🟡 | `00-INDEX` §1.2 写 `docs/mox-expert-mox-fusion-flows.md`，实际文件为 `mox-expert-alliance-fusion-flows.md`（FIX-09 修正） |
| F12 | 文件名笔误（跨多文档） | 🟠 | `02-architecture.md:295`、`04-business-processing.md:197`、`modules/mox-expert-business-requirements.md:5,234`、`graph/graph.mmd:88` 引用 `mox-expert-mox-fusion-flows.md`，实际文件为 `mox-expert-alliance-fusion-flows.md`（FIX-11 全量修正） |
| F13 | 相对路径缺 `docs/` 前缀 | 🟡 | `00-INDEX.md:83` 写 `full-dimensional/00-README.md`，应为仓根相对 `docs/full-dimensional/00-README.md`（FIX-12 修正） |
| F14 | **术语表（glossary）缺失** | 🔴 | 53 份 Markdown 仅 `algorithm-verification.md` 1 份含 glossary，与其「企业级 + 完整 glossary」强约定直接冲突；术语无统一事实源，存在漂移风险 | 
| F15 | 原始过程稿滞留权威目录 | 🟠 | `full-dimensional/` 仍与 🟢 权威文档同置 4 份原始过程稿（关图骨架定义 / TraceMatrix / 测试验证报告 / 业务处理流程图），与 §2.1「full-dimensional/ 仅关图骨架·索引·治理台 API」及「过程稿仅可位于 `_archive/`」规则冲突 |

---

## 2. 归一化标准（明确规则，后续强制）

### 2.1 目录职责（唯一划分）

| 目录 | 职责 | 权威等级 |
|------|------|:--:|
| `docs/enterprise/` | 唯一治理中心：`00-INDEX` + 编号 `01~16` 企业级文档 + ISD 交付验收报告 | 🟢 |
| `docs/specs/` | 企业级规范：PT-STD / GR-STD / OUS 业务规划 | 🟢 |
| `docs/full-dimensional/` | 关图骨架（guantu-skeleton）、编号索引（baseline）、治理台 API、原始过程稿归档 | 🟢/🟡 |
| `docs/modules/` | 模块级设计 / 参考文档（商城、自动化、数学内核、业务流程、mox-expert 系列、设计蓝图） | 🟡 |
| `docs/graph/` | 关图机读产物（graph.json 等）+ `requests/` 判重入口 | 🟡 产物 |
| `docs/ai-architecture/` | AI 架构专题（AUS · L4 Agentic 闭环可视化） | 🟡 |
| `docs/_archive/YYYY-MM-DD/` | 过程稿 / 验证快照归档（非权威，仅供追溯） | 🟡 归档 |
| `docs/`（根） | 仅保留 🟢 顶层权威文档：`architecture.md`、`enterprise-architecture-analysis.md`、AA-STD（`璇玑-全维需求业务处理流程图-归一化企业级.md`）及其同位可视化 | 🟢 |

### 2.2 命名规则

- **权威基准**：`璇玑-全维需求业务处理流程图-归一化企业级.md`（AA-STD）、`guantu-skeleton.md`（GR-STD 骨架）、`mox-requirement-baseline.md`（编号索引）保持既有命名，不缩写。
- **过程稿**：统一带 `-YYYYMMDD` 后缀，且**仅**可位于 `docs/_archive/`。
- **可视化产物**：`*.html` / `*.mmd` 必须与源 `.md` **同位存放**（同目录），文件名主体一致。
- **禁止**：根目录新增松散 `.md`；同一文件跨目录复制（单一事实源 Single Source of Truth）。

### 2.3 引用规则（强制）

所有文档间引用统一为**仓根相对**形式 `docs/<相对路径>/<文件>`（例：`docs/modules/mox-expert-business-requirements.md`）。禁止 `../`、`full-dimensional/`、`bare-name` 等相对/混用写法（脚本 `fix_links.py` 已全量归一）。

### 2.4 权威分级与锚点

- 每篇文档头部须声明 🟢 权威 / 🟡 参考 / 🟡 过程稿 / 🟡 归档。
- 长文档须含**编号章节 + anchor 锚点**（如 `## 5.6 FSM 形式化建模 {#sec-fsm}`）以便跨文档引用。
- 术语须配 **glossary**（AA-STD / GR-STD / PT-STD 已具备，后续新文档强制）。

---

## 3. 已执行的归一化动作（逐文件）

### FIX-01 权威文件归位（与索引声明对齐）
| 源（根） | 目标 |
|----------|------|
| `docs/guantu-skeleton.md` | `docs/full-dimensional/guantu-skeleton.md` |
| `docs/mox-requirement-baseline.md` | `docs/full-dimensional/mox-requirement-baseline.md` |

### FIX-02 过程稿归档（`docs/_archive/2026-08-16/`）
`PrimiFlow-API健康探活与CORS-20260816.md`、`PrimiFlow-API项目注册表与重启复现-20260816.md`、`PrimiFlow-三层递进开发-验证总结-20260816.md`、`PrimiFlow-企业级验证-20260816.md`、`PrimiFlow-全维分析-核心功能补全-20260816.md`、`PrimiFlow-真实执行层开发-验证-20260816.md`、`mox-expert-验证总结-20260816.md`

### FIX-03 模块文档归 `docs/modules/`
`market-module.md`、`automation-module.md`、`mathematical-foundation.md`、`business-process-flows.md`、`business-process-flowcharts.md`、`mox-expert-alliance-fusion-flows.md`、`mox-expert-normalization.md`、`mox-expert-product.md`、`mox-expert-business-requirements.md`、`PrimiFlow-设计蓝图.md`

### FIX-04 验收报告入 `docs/enterprise/`
`璇玑-信息化系统开发验收报告-V1.0.md`、`.html`（ISD-V1.0），并在 `00-INDEX` §1.2 注册为 🟢 权威交付物

### FIX-05 可视化同位
`mox-tracematrix.html` → `docs/full-dimensional/`（源为 full-dimensional 过程稿）；`mox-flow.html` → `docs/modules/`（源为 `mox-expert-alliance-fusion-flows.md`）；其余 viz 已与源 `.md` 同位（root / enterprise）

### FIX-06 交叉引用全量重写
24 个文件、共 60+ 处引用按 §2.3 重写为 `docs/<rel>` 形式（脚本幂等，可重复执行）

### FIX-07 / 08 / 09 治理文档修正
`00-INDEX.md`：删除"散落文档"过时表述、计数 16→17、修正 `mox-expert-alliance-fusion-flows.md` 文件名、注册 ISD、新增 `_archive`/`modules` 说明；`README.md` 与 `full-dimensional/00-README.md` 重写以匹配真实树。

### FIX-10 补全悬空文档（GAP-1 收口）
新建 `docs/modules/algorithm-verification.md`（AV-STD-V1.0 · 🟢 权威）：以仓内真实校验源为基础——`verify_axioms.py`（六公理 + 守恒律自洽）、`docs/specs/pt-primi-架构规范-v1.0-完整版.md` §9（ε≤1e-3 / 六维绑定 A4 / 可追溯 A5 / seed）、`docs/full-dimensional/GOVERNANCE_CONSOLE_API_READY_20260816.md`（⛨璇玑验证网关最高权限 + AuditChain）——定义 L1 数学自洽 / L2 PT-Primi 合规 / L3 璇玑治理闸门的统一验证矩阵。同步将 `enterprise-architecture-analysis.md:216` 引用修正为 `docs/modules/algorithm-verification.md`，并在 `00-INDEX` §1.2 注册（计数 17→18）。

### FIX-11 文件名笔误全量修正
`mox-expert-mox-fusion-flows.md` → `mox-expert-alliance-fusion-flows.md`，涉及 `02-architecture.md:295`、`04-business-processing.md:197`、`modules/mox-expert-business-requirements.md:5,234`（含 markdown 链接 `./...`）、`graph/graph.mmd:88`（节点标签）。

### FIX-12 相对路径补 `docs/` 前缀
`00-INDEX.md:83` 的 `full-dimensional/00-README.md` → `docs/full-dimensional/00-README.md`，统一为 §2.3 仓根相对形式。

### FIX-13 死链回归验证
重跑精确文档引用审计（仅判定指向 `.md`/`.html` 文档产物的引用）：**真实断链 0 处**。残余 41 条"未命中"均为噪声——通配符（`*.md`/`*.html`）、源码/仓根文件引用（`crates/...`、`CLAUDE.md`、`README.md`）、报告内目录树代码块、以及本报告描述性"旧路径→新路径"文字，均非文档交叉引用，不计入断链。

### FIX-14 规范术语表（GAP / F14 收口）
新建**企业级规范术语表 `docs/GLOSSARY.md`（DOC-GLOSSARY-V1.0 · 🟢 权威）**作为全项目**术语唯一事实源（Single Source of Truth）**，覆盖：璇玑/关图(GR-STD)/AA-STD/PT-Primi/OUS/双璇玑十四维/TraceMatrix 六维绑定/⛨璇玑验证网关/治理闸门/归一化/全维/关图骨架；并定义命名与大小写强制约定（璇玑非「旋玑」、PT-Primi 非 PT-PRIMI、引用一律 `docs/<rel>` 等）。
为避免 52 份文档各自复制术语表导致漂移，采用「**统一规范术语表为唯一基准 + 各权威文档文末附速查段**」：通过脚本向 8 份核心权威文档（`architecture.md`、`enterprise-architecture-analysis.md`、AA-STD、`full-dimensional/guantu-skeleton.md`、`full-dimensional/mox-requirement-baseline.md`、`specs/PT-Primi`、`specs/GR-STD`、`specs/OUS`）追加「## 术语表 (Glossary)」速查段（已存在则跳过），均指向 `docs/GLOSSARY.md`。并在 `00-INDEX` §1.2 注册为 🟢 权威、在 `README.md` 导航增列入口。

### FIX-15 原始过程稿归档（F15 收口）
将 `full-dimensional/` 下 4 份原始过程稿整体 `git mv` 至 `docs/_archive/2026-08-16/`（同目录整体迁移，其内部以同目录裸名互引仍有效）：
`关图骨架定义.md`、`璇玑-全维分析-TraceMatrix-六维绑定追溯.md`、`璇玑-全维分析需求-测试分析验证报告.md`、`璇玑-全维分析需求业务处理流程图.md`。
同步 5 处外部描述性引用路径：`enterprise/00-INDEX.md:84`、`full-dimensional/00-README.md:18-21`、`README.md:23,68`、`full-dimensional/mox-requirement-baseline.md:13`（其余为内部裸名互引，迁移后无新增断链）。`full-dimensional/` 现仅余 🟢 权威文档，消除「过程稿与权威同目录」冲突。

---

## 4. 归一化后目标树（物理真实状态）

```
docs/
├── enterprise/        00-INDEX + 01~16 + 璇玑-信息化系统开发验收报告-V1.0.{md,html}
├── specs/             PT-Primi / GR-STD / OUS 业务规划
├── full-dimensional/  guantu-skeleton · mox-requirement-baseline · GOVERNANCE_API · mox-tracematrix.html
├── modules/           market/automation/mathematical/business-flow(s/charts)/mox-expert-*（含 algorithm-verification）/ PrimiFlow-设计蓝图 + mox-flow.html
├── graph/             graph.json · graph.enterprise.json · guantu.req.json · requests/
├── ai-architecture/   ai-unified-intelligent-system-architecture.html
├── _archive/2026-08-16/   11 份过程稿（4 份原始分析稿 + 7 份 -20260816 稿，非权威）
├── 璇玑-全维需求业务处理流程图-归一化企业级.{md,html,mmd}   # AA-STD 🟢（根）
├── 璇玑-璇玑验证子流程-归一化企业级.html
├── 璇玑-全维流水线.mmd
├── mox-system-business-architecture.html
├── GLOSSARY.md（术语表 🟢）· architecture.md · enterprise-architecture-analysis.md
└── README.md · DOC-NORMALIZATION-REPORT.md
```

---

## 5. 验证结论

- ✅ 全量文档纳入 git；18 次移动均经 `git mv`，**历史可回溯、可回退**；本轮新增 `docs/modules/algorithm-verification.md`（FIX-10）与 `docs/GLOSSARY.md`（FIX-14），均新建补全。
- ✅ 残留引用扫描：**0** 处指向旧根路径（`docs/mox-expert-*` 非 modules、`docs/market-module.md` 等）。
- ✅ 索引声明与物理事实一致：F2/F3/F4/F5/F7/F8/F10/F11/F12/F13/F15 全部闭合。
- ✅ 死链回归（FIX-13）：文档交叉引用**真实断链 0 处**；GAP-1（F9）已通过新建文档收口。
- ✅ **术语表缺口（F14）已收口**：建立 `docs/GLOSSARY.md` 唯一事实源，并向 8 份核心权威文档附速查段；重跑 glossary 覆盖体检由 1/53 升至 9/54（8 追加 + 1 原有），其余文档统一引用规范术语表，不重复维护。
- ✅ **原始过程稿（F15）已收口**：4 份迁 `_archive/2026-08-16/` 并同步 5 处引用，`full-dimensional/` 现为纯 🟢 权威目录。
- ✅ 全文权威分级与锚点/glossary 规范在 AA-STD / GR-STD / PT-STD / AV-STD / GLOSSARY 均具备；锚点完整性（跨文档 `#anchor`）体检 **0 断锚**；标题层级（跳过代码围栏）**0 跳级**。

---

## 6. 残留 GAP 与后续动作

| ID | 问题 | 建议动作 | 责任 |
|----|------|----------|------|
| GAP-1 | ~~`enterprise-architecture-analysis.md:216` 引用 `algorithm-verification.md`，文件不存在~~ **已解决（FIX-10）**：新建 `docs/modules/algorithm-verification.md`（AV-STD-V1.0），引用同步修正 | —（已闭环） | — |
| 整合-1 | ~~4 份原始过程稿滞留 `full-dimensional/`（与 🟢 权威同目录）~~ **已解决（FIX-15）**：整体迁 `_archive/2026-08-16/` 并同步 5 处引用 | —（已闭环） | — |
| GAP-2 | `modules/` 混合 🟢（mox-expert-business-requirements 为 BR 权威源）与 🟡 参考 | 在 `00-INDEX` 已显式标注各文件等级；后续若需更严，可将 BR 源单独提至 `specs/` 或 `enterprise/` | 文档维护者 |
| GAP-3 | AA-STD 物理位于根而非 `full-dimensional/` | 维持现状（与 `00-INDEX` 声明一致），如需统一可后续迁移并在 INDEX 同步 | 文档维护者 |
| GAP-4（🟡 观察） | `modules/PrimiFlow-设计蓝图.md:65,230` 使用「功能关联图」，与 GR-STD「关图（信息关联关系图）」概念邻近，存在术语重叠风险 | 判定为 PrimiFlow 自有产物命名，未强行改；已在 `GLOSSARY.md §4` 显式标注「功能关联图属 PrimiFlow 产物，与关图概念邻近，不混用」。若后续统一，建议 PrimiFlow 文档增加一句「功能关联图是关图的一个 PrimiFlow 渲染视图」 | 文档维护者 |

> 本报告为活文档，任何 `docs/` 结构变更须在 `00-INDEX` 变更记录留痕。

---

## 7. 内容重叠与整合评估（整合维度）

### 7.1 发现
跨 `modules/` · `full-dimensional/` · `enterprise/` 的段落级重复扫描（≥120 字符）发现 **1 处真实重叠簇**：

- `docs/full-dimensional/guantu-skeleton.md`（🟢 权威，GR-STD 骨架）与 `docs/_archive/2026-08-16/关图骨架定义.md`（🟡 过程稿，已归档）共享 **8 段**大段内容（每段 135–518 字符）。

### 7.2 性质判定（明确，不模糊）
二者为 **「原始草稿源 → 归一化权威」** 的衍生关系，而非平行冗余：
- `关图骨架定义.md` 是 `guantu-skeleton.md` 的**前置原始稿**；其内容与 4 份同目录原始文档一起，已按 `full-dimensional/00-README.md` 与 `00-INDEX` §1.2 声明"归一承载于 AA-STD / guantu-skeleton"。
- 4 份原始文档（`关图骨架定义.md`、`璇玑-全维分析-TraceMatrix-六维绑定追溯.md`、`璇玑-全维分析需求-测试分析验证报告.md`、`璇玑-全维分析需求业务处理流程图.md`）**仅在治理文档中被描述为过程稿**，且彼此以同目录裸名互引——属设计内的追溯稿。

### 7.3 处置决策（企业级，单一事实源优先）
- ✅ **唯一权威承载**为 `guantu-skeleton.md`（GR-STD 骨架）+ AA-STD（融合域需求基准）；任何引用以二者为准。
- ✅ **本轮已执行「更严归一」（FIX-15）**：4 份原始过程稿整体迁至 `docs/_archive/2026-08-16/`，并同步 `00-INDEX:84`、`full-dimensional/00-README.md`、`README.md:23,68`、`mox-requirement-baseline.md:13` 共 5 处引用（内部互引为同目录裸名，迁移后仍可解析，无新增断链）。`full-dimensional/` 现为纯 🟢 权威目录，消除「过程稿与权威同目录」冲突，同时完整保留追溯链（git 历史 + 归档位置）。

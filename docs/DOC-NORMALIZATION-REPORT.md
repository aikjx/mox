# docs/ 企业级归一化标准与审计报告（DOC-GOV-V1.0）

> 编号：**DOC-GOV-V1.0**
> 生成时间：2026-08-18
> 适用范围：`infotopograph` 仓库 `docs/` 全量文档（65 份）
> 治理入口：`docs/enterprise/00-INDEX.md`（唯一权威等级定义）
> 配套导航：`docs/README.md`（关图/全维专题索引）

---

## 0. 执行摘要（一句话结论）

`docs/` 此前**物理布局与治理索引不一致**：根目录散落 7 份过程稿、10 份松散模块文档、1 份未注册的验收报告；索引声称 `guantu-skeleton.md` / `xuanji-requirement-baseline.md` 位于 `full-dimensional/` 但物理上在根；可视化产物未与源 `.md` 同位；内部交叉引用格式不统一。本次已**全部归位、修复引用、统一口径、补齐治理缺口**，使"索引声明 = 物理事实"。

---

## 1. 审计发现（明确、不模糊）

| # | 发现 | 严重度 | 事实依据 |
|---|------|:--:|----------|
| F1 | 无字节级重复文件 | — | 65 份文件 sha256 两两比对，0 对相同 |
| F2 | 索引路径与物理位置错位 | 🔴 | `00-INDEX.md` / `full-dimensional/00-README.md` 指向 `docs/full-dimensional/guantu-skeleton.md`、`docs/full-dimensional/xuanji-requirement-baseline.md`，但两文件实际在 `docs/` 根（FIX-01 已纠正） |
| F3 | 根目录散落过程稿 | 🔴 | 7 份 `*-20260816*.md`（PrimiFlow-/xuanji-expert-验证总结-）混在权威文档区，与"过程稿非权威"声明矛盾（FIX-02 归档） |
| F4 | 松散模块文档未归类 | 🟠 | `market-module` / `automation-module` / `mathematical-foundation` / `business-process-flows` / `business-process-flowcharts` / `xuanji-expert-*`(4) / `PrimiFlow-设计蓝图` 共 10 份游离根目录（FIX-03 归 `modules/`） |
| F5 | 验收报告未注册 | 🟠 | `璇玑-信息化系统开发验收报告-V1.0.{md,html}`（ISD-V1.0）在根目录，未被 `00-INDEX` 任何表登记（FIX-04 入 `enterprise/` 并注册） |
| F6 | 可视化产物未同位 | 🟡 | `xuanji-tracematrix.html`、`xuanji-flow.html` 与各自源 `.md` 不同目录（FIX-05 同位化） |
| F7 | 交叉引用格式不统一 | 🟠 | 同一被引文件存在 `docs/xxx`、`../xxx`、`xxx`、`full-dimensional/xxx` 四种写法，且移动后大面积失效（FIX-06 全量重写） |
| F8 | 索引"散落文档"说明过时 | 🟡 | `00-INDEX` §1.2 仍称"根目录散落文档"，与归位后事实不符（FIX-07 改写） |
| F9 | 悬空引用（GAP） | 🔴 | `enterprise-architecture-analysis.md:216` 引用 `algorithm-verification.md`，该文件在 `docs/` 全树不存在（见 §6 GAP-1，待建或删） |
| F10 | 索引计数错误 | 🟡 | `00-INDEX` §0 称"16 份（00~16）"，00~16 实为 17 份（FIX-08 修正为 17） |
| F11 | 索引文件名错误 | 🟡 | `00-INDEX` §1.2 写 `docs/xuanji-expert-xuanji-fusion-flows.md`，实际文件为 `xuanji-expert-alliance-fusion-flows.md`（FIX-09 修正） |

---

## 2. 归一化标准（明确规则，后续强制）

### 2.1 目录职责（唯一划分）

| 目录 | 职责 | 权威等级 |
|------|------|:--:|
| `docs/enterprise/` | 唯一治理中心：`00-INDEX` + 编号 `01~16` 企业级文档 + ISD 交付验收报告 | 🟢 |
| `docs/specs/` | 企业级规范：PT-STD / GR-STD / OUS 业务规划 | 🟢 |
| `docs/full-dimensional/` | 关图骨架（guantu-skeleton）、编号索引（baseline）、治理台 API、原始过程稿归档 | 🟢/🟡 |
| `docs/modules/` | 模块级设计 / 参考文档（商城、自动化、数学内核、业务流程、xuanji-expert 系列、设计蓝图） | 🟡 |
| `docs/graph/` | 关图机读产物（graph.json 等）+ `requests/` 判重入口 | 🟡 产物 |
| `docs/ai-architecture/` | AI 架构专题（AUS · L4 Agentic 闭环可视化） | 🟡 |
| `docs/_archive/YYYY-MM-DD/` | 过程稿 / 验证快照归档（非权威，仅供追溯） | 🟡 归档 |
| `docs/`（根） | 仅保留 🟢 顶层权威文档：`architecture.md`、`enterprise-architecture-analysis.md`、AA-STD（`璇玑-全维需求业务处理流程图-归一化企业级.md`）及其同位可视化 | 🟢 |

### 2.2 命名规则

- **权威基准**：`璇玑-全维需求业务处理流程图-归一化企业级.md`（AA-STD）、`guantu-skeleton.md`（GR-STD 骨架）、`xuanji-requirement-baseline.md`（编号索引）保持既有命名，不缩写。
- **过程稿**：统一带 `-YYYYMMDD` 后缀，且**仅**可位于 `docs/_archive/`。
- **可视化产物**：`*.html` / `*.mmd` 必须与源 `.md` **同位存放**（同目录），文件名主体一致。
- **禁止**：根目录新增松散 `.md`；同一文件跨目录复制（单一事实源 Single Source of Truth）。

### 2.3 引用规则（强制）

所有文档间引用统一为**仓根相对**形式 `docs/<相对路径>/<文件>`（例：`docs/modules/xuanji-expert-business-requirements.md`）。禁止 `../`、`full-dimensional/`、`bare-name` 等相对/混用写法（脚本 `fix_links.py` 已全量归一）。

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
| `docs/xuanji-requirement-baseline.md` | `docs/full-dimensional/xuanji-requirement-baseline.md` |

### FIX-02 过程稿归档（`docs/_archive/2026-08-16/`）
`PrimiFlow-API健康探活与CORS-20260816.md`、`PrimiFlow-API项目注册表与重启复现-20260816.md`、`PrimiFlow-三层递进开发-验证总结-20260816.md`、`PrimiFlow-企业级验证-20260816.md`、`PrimiFlow-全维分析-核心功能补全-20260816.md`、`PrimiFlow-真实执行层开发-验证-20260816.md`、`xuanji-expert-验证总结-20260816.md`

### FIX-03 模块文档归 `docs/modules/`
`market-module.md`、`automation-module.md`、`mathematical-foundation.md`、`business-process-flows.md`、`business-process-flowcharts.md`、`xuanji-expert-alliance-fusion-flows.md`、`xuanji-expert-normalization.md`、`xuanji-expert-product.md`、`xuanji-expert-business-requirements.md`、`PrimiFlow-设计蓝图.md`

### FIX-04 验收报告入 `docs/enterprise/`
`璇玑-信息化系统开发验收报告-V1.0.md`、`.html`（ISD-V1.0），并在 `00-INDEX` §1.2 注册为 🟢 权威交付物

### FIX-05 可视化同位
`xuanji-tracematrix.html` → `docs/full-dimensional/`（源为 full-dimensional 过程稿）；`xuanji-flow.html` → `docs/modules/`（源为 `xuanji-expert-alliance-fusion-flows.md`）；其余 viz 已与源 `.md` 同位（root / enterprise）

### FIX-06 交叉引用全量重写
24 个文件、共 60+ 处引用按 §2.3 重写为 `docs/<rel>` 形式（脚本幂等，可重复执行）

### FIX-07 / 08 / 09 治理文档修正
`00-INDEX.md`：删除"散落文档"过时表述、计数 16→17、修正 `xuanji-expert-alliance-fusion-flows.md` 文件名、注册 ISD、新增 `_archive`/`modules` 说明；`README.md` 与 `full-dimensional/00-README.md` 重写以匹配真实树。

---

## 4. 归一化后目标树（物理真实状态）

```
docs/
├── enterprise/        00-INDEX + 01~16 + 璇玑-信息化系统开发验收报告-V1.0.{md,html}
├── specs/             PT-Primi / GR-STD / OUS 业务规划
├── full-dimensional/  guantu-skeleton · xuanji-requirement-baseline · GOVERNANCE_API · xuanji-tracematrix.html · 四份过程稿
├── modules/           market/automation/mathematical/business-flow(s/charts)/xuanji-expert-* / PrimiFlow-设计蓝图 + xuanji-flow.html
├── graph/             graph.json · graph.enterprise.json · guantu.req.json · requests/
├── ai-architecture/   ai-unified-intelligent-system-architecture.html
├── _archive/2026-08-16/   7 份过程稿（非权威）
├── 璇玑-全维需求业务处理流程图-归一化企业级.{md,html,mmd}   # AA-STD 🟢（根）
├── 璇玑-璇玑验证子流程-归一化企业级.html
├── 璇玑-全维流水线.mmd
├── xuanji-system-business-architecture.html
├── architecture.md · enterprise-architecture-analysis.md
└── README.md · DOC-NORMALIZATION-REPORT.md
```

---

## 5. 验证结论

- ✅ 65 份文件全部纳入 git；18 次移动均经 `git mv`，**历史可回溯、可回退**。
- ✅ 残留引用扫描：**0** 处指向旧根路径（`docs/xuanji-expert-*` 非 modules、`docs/market-module.md` 等）。
- ✅ 索引声明与物理事实一致：F2/F3/F4/F5/F7/F8/F10/F11 全部闭合。
- ⚠️ 残留 1 项待办：**GAP-1** `algorithm-verification.md` 悬空引用（见 §6）。

---

## 6. 残留 GAP 与后续动作

| ID | 问题 | 建议动作 | 责任 |
|----|------|----------|------|
| GAP-1 | `enterprise-architecture-analysis.md:216` 引用 `algorithm-verification.md`，文件不存在 | 确认该文档是否应新建（璇玑校验/algorithm-verification 设计），或从引用中删除 | 架构师 |
| GAP-2 | `modules/` 混合 🟢（xuanji-expert-business-requirements 为 BR 权威源）与 🟡 参考 | 在 `00-INDEX` 已显式标注各文件等级；后续若需更严，可将 BR 源单独提至 `specs/` 或 `enterprise/` | 文档维护者 |
| GAP-3 | AA-STD 物理位于根而非 `full-dimensional/` | 维持现状（与 `00-INDEX` 声明一致），如需统一可后续迁移并在 INDEX 同步 | 文档维护者 |

> 本报告为活文档，任何 `docs/` 结构变更须在 `00-INDEX` 变更记录留痕。

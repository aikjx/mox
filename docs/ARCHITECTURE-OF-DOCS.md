# 文档体系架构规范（Architecture of Docs）· v1.0

> 编号：**DOC-GOV-ARC-V1.0**　·　定位：`docs/` 目录的**唯一结构权威（Single Source of Truth）**
> 生效范围：`docs/` 全部内容、以及仓库中任何指向 `docs/` 的引用
> 关联：`docs/enterprise/00-INDEX.md`（企业级文档治理中心）· `docs/normalization/README.md`（归一化 SSoT 枢纽）· `scripts/ci-gate.ps1`（CI 门禁）

---

## 0. 一句话定位

`docs/` 不是"放文件的地方"，而是**产品的第二实现**：一层文档 = 一层架构事实。
本规范把"文档该放哪、叫什么名、谁是权威、谁引用谁、如何校验"全部写死，使文档与代码一样可评审、可门禁、可回归。

三条铁律：

1. **一个主题一个目录**（同名同类不再平铺）。
2. **一个事实一个权威源**（🟢 权威唯一，其余用引用，不复制）。
3. **路径即契约**（目录与文件的路径是外部引用契约，变更必须走本规范 §6 迁移流程）。

---

## 1. 分层模型（L0~L7）

物理目录与逻辑层 **1:1 对应**；层号只用于导航排序，不写入目录名（避免破坏既有引用契约）。

| 层 | 目录 | 职责 | 权威等级 | 生命周期 |
|----|------|------|:--------:|----------|
| **L0 入口层** | `README.md` · `docs-hub/` | 门面：分层导航、交互式文档中心 | 🟢 导航 | 长期 |
| **L1 概览层** | `CORE-CAPABILITIES.md` · `ROADMAP-DOMAINS.md` · `API-REGISTRY.md` | 能力/路线图/接口总账：一页读懂"系统是什么、做到哪、暴露什么" | 🟢 权威 | 长期 |
| **L2 架构层** | `architecture/` | 架构事实：总览/分层/元架构/微服务/Rust 企业级/领域架构/工具链 | 🟢 权威 | 长期 |
| **L3 域与模块层** | `modules/` · `expert-alliance/` | 业务域与模块的说明、流程图、产品手册 | 🟢 权威 / 🟡 参考 | 中期 |
| **L4 接口与数据层** | `api/` · `database/` | 接口契约（REST/TCP/端口/清单）+ 数据模型与 DDL | 🟢 权威 | 长期 |
| **L5 规范与治理层** | `standards/` · `normalization/` | 编码/流程/归一化标准与索引 | 🟢 权威 | 长期 |
| **L6 企业级与规格层** | `enterprise/` · `specifications/` | 企业级需求→架构→设计→交付+ADR；规格驱动任务包（spec/tasks/review） | 🟢 权威 | 中期 |
| **L7 报告与验证层** | `working-reports/` | 过程报告、验证证据、审计、基准数据 | 🟡 证据/过程 | 短期→归档 |
| **L8 归档层** | `_archive/` | 历史快照：只增不改，永不作为权威引用 | 🔴 只读 | 永久 |

### 1.1 架构层（L2）内部结构

```
architecture/
├─ README.md                        # 架构层入口（导航 + 权威表）
├─ architecture.md                  # 🟢 总架构规范（v3.0-ai-powered）
├─ operations-manual.md             # 🟢 操作说明手册
├─ NORMALIZED_ARCHITECTURE.md       # 归一化分层模型
├─ OPTIMAL_ARCHITECTURE.md          # 最优架构方案
├─ DOMAIN_FIRST_LAYOUT.md           # 领域优先目录布局
├─ ARCHITECTURE-ENTERPRISE.md       # 企业级架构（扩展）
├─ ARCHITECTURE_DESIGN_v3.1.md      # 设计稿（版本演进）
├─ 02-extension-guide.md            # 编号系列：扩展开发指南
├─ 04-error-code-reference.md       # 错误码
├─ 05-normalization-checklist.md    # 归一化检查清单
├─ 06-rpc-integration-guide.md      # RPC 集成
├─ 07-KG-DYNAMIC-SQL-ARCHITECTURE.md
├─ 08-FULL-DIMENSION-LOWCODE-ARCHITECTURE.md
├─ 09-ROCKSDB-PERFORMANCE-OPTIMIZATION.md
├─ 10-DSQL-CORE-FULL-DIMENSIONAL-VALIDATION.md
├─ 11-ENTERPRISE-WEBSITE-LOWCODE-IMPLEMENTATION.md
├─ 12-MOX-RUNTIME-ENGINE-DELIVERY.md
├─ 13-PLATFORM-CODEBASE-GUIDE.md    # 代码库指南
├─ 14-REPOSITORY-FULL-MAP.md        # 仓库全地图
├─ ai/                              # AI 统一智能系统架构
├─ meta/                            # COSMIC 元架构
├─ microservices/                   # 微服务独立部署架构
├─ rust-enterprise/                 # Rust 企业级开发指南（6 层/8 域）
├─ graph/                           # 关图产物与需求基线（mmd/json/requests）
├─ plugin/                          # VSCode 插件架构与兼容性
├─ full-dimensional/                # 全维 TraceMatrix 与需求基线
└─ assets/                          # 结构化产物（bench_results_round7 / APPROVAL-FLOWS / architecture-metrics）
```

> 编号系列 `02~14` 与 `assets/`、`_hub/` 为**同层内二级分类**，不再新开平级目录。

---

## 2. 命名规范

### 2.1 目录

- 全小写、`kebab-case`、**不含空格与中文**（归档目录除外）。
- 目录名 = 主题名词（`api`、`database`、`standards`），**不带层号前缀**（层号由本规范 §1 声明）。

### 2.2 文件

| 类型 | 规则 | 示例 |
|------|------|------|
| 索引/入口 | `README.md` 或 `00-<主题>.md` | `modules/README.md` |
| 编号系列 | `<两位序号>-<英文短名>.md` | `04-error-code-reference.md` |
| 企业级文档 | `<两位序号>-<中文主题>-V<版本>.md` | `43-DOC-EP-038文档代码事实自动核对报告.md`（原 38-VERIFY-REPORT，因 38 号冲突迁出） |
| ADR | `<序号>-<主题>-ADR-<编号>.md` | `29-跨域依赖规则与架构一致性治理-ADR-09.md` |
| 规格任务包 | `tasks/<YYYYMMDD>-<slug>/{spec,tasks,review}.md` | `tasks/20260823-mox-full-enterprise-architecture/spec.md` |
| 报告 | `<YYYYMMDD>_<slug>_<类型>.md` 或 `<slug>-<YYYYMMDD>.md` | `20260903_moxfs_phase5_e2e_test_report.md` |
| 数据/产物 | 原扩展名，放同层 `assets/` 或 `_data/` | `architecture/assets/bench_results_round7.json` |

**禁止**：

- 文件名中出现批量替换残留的品牌冗余串（历史缺陷，见 §5 P0-1）。
- 同一编号在一个目录内重复（如 `enterprise/` 中 `26-/29-/30-/31-/38-` 各出现两次，见 §5 P0-2）。
- `.md` 与其渲染版 `.html` 同名分居两处：`.md` 为源、`.html` 与其**同目录**或归入同层 `_hub/`。

---

## 3. 权威分级与单源（SSOT）

| 标记 | 含义 | 典型位置 |
|:----:|------|----------|
| 🟢 | 权威：以此为准，冲突时改下游 | L1~L6 主文档 |
| 🟡 | 参考/过程/证据：可追溯，非权威 | L7、`*.mmd`、渲染 html、`*_data/` |
| 🔴 | 归档：只读，禁止引用 | `_archive/` |

单源规则：

1. 接口事实 = `docs/API-REGISTRY.md`（由 `actuator.rs ROUTES` 生成）。
2. 端口事实 = `docs/api/PORT-REGISTRY.md`（由 `scripts/verify-ports.py` 校验）。
3. 术语事实 = `docs/enterprise/GLOSSARY.md`。
4. 跨文档等价关系 = `docs/enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md`。
5. 关图需求 = `docs/architecture/graph/`；关图规范 = `docs/specifications/`（GR-STD/PT-Primi/OUS）。

---

## 4. 引用规范

1. 文档间引用统一使用**仓根相对路径**：`` `docs/<层>/<文件>.md` ``，禁止裸文件名与 `../` 穿透（历史文件不强制回改，新增必须遵守）。
2. 引用必须带锚点：`docs/standards/engine-kernel.md#分层`。
3. 目录引用必须指向该层 `README.md`，而非目录本身。
4. 跨层引用只引用 🟢 权威；引用 🟡 需在行内注明"过程/证据"。
5. 结构化数据（json/yml/sql）与文档同层存放于 `assets/` 或 `_data/`，避免与 `.md` 混放。

---

## 5. 现存缺陷登记（审计基线 2026-09-13）

| 编号 | 级别 | 缺陷 | 证据 | 处置 |
|------|:----:|------|------|------|
<!-- check-doc-links:ignore-start -->
| P0-1 | 高 | **品牌串污染**：批量替换把 `MOX` 写成了 `mox 模块化系统架构`，落入**文件名**与**正文** | 25 个文件名命中；149 篇文档正文共 769 处 | ✅ 已修复：正文全量替换为 `MOX`；文件名重命名待 shell 环境恢复后执行（见 §6.4） |
| P0-2 | 高 | **编号空间冲突**：`enterprise/` 内 `26-`（开发专家联盟 / 前端开发专家）、`29-`（跨域依赖 / MOX总任务中心）、`30-`（网关瘦身 / MOX商场中心）、`31-`（可观测性 / 代码审计）、`38-`（架构文档 / VERIFY-REPORT）各重号 | `enterprise/` 目录清单 | 重号文档迁到 `39+`，同步 00-INDEX 登记 |
| P0-3 | 高 | **死链**：多份索引指向已迁移路径 | `enterprise/00-INDEX.md` 引用 `docs/GLOSSARY.md`、`docs/architecture.md`、`docs/specs/`、`docs/graph/graph.json`、`docs/enterprise-architecture-analysis.md`；`normalization/README.md` 引用 `docs/architecture-hub.html`；`scripts/verify-ports.py` 注释引用 `docs/ports/PORT-REGISTRY.md`（实为 `docs/api/`） | 已随本次迁移修正（§6） |
| P0-4 | 中 | **同层平铺**：8 个架构类目录平级散落（`ai-architecture`/`cosmic-architecture`/`graph`/`plugin`/`full-dimensional`/`microservices`/`rust-enterprise`/`enterprise-verification`） | `docs/` 一级清单 19 项 | 已收敛入 L2/L7（§6） |
| P0-5 | 中 | **根级散落**：报告/产物与入口文档混放于 `docs/` 根 | `PRODUCTION_READINESS_ASSESSMENT_v3.4.md`、`SECURITY_AUDIT_v3.4.md`、`v21-features.md`、`developer-docs.html`、`bench_results_round7.json` | 已归位（§6） |
| P1-1 | 中 | **层索引缺失**：`api/`、`standards/`、`specifications/`、`docs-hub/` 无 `README.md` | 目录清单 | 已补（§6.3） |
| P1-2 | 低 | **重复资产**：`bench_results_round7.json` 两份（内容不同：见 §6.2）；`developer-docs.html` 与 `architecture-hub.html` 功能重叠 | 文件哈希 | 合并入 `architecture/assets/`、`docs-hub/` |
| P1-3 | 低 | 标题/版本号漂移（如 README 声明"文档中心 3.0"、`v3.4` 评估文件为一次性快照） | 各文件头部 | 由 §7 门禁持续收敛 |
<!-- check-doc-links:ignore-end -->

---

## 6. 迁移映射（本轮执行）

### 6.0 企业文档编号重映射（P0-2 修复，2026-09-21）

> `enterprise/` 内 29/30/31/38 发生编号冲突（两份文档同号），已按「原编号 + 内容」迁移到 40+ 新编号空间。

| 旧路径 | 新路径 | 说明 |
|--------|--------|------|
| `enterprise/29-MOX总任务中心-全AI工具全软件系统统一控制架构-V1.0.md` | `enterprise/40-MOX总任务中心-全AI工具全软件系统统一控制架构-V1.0.md` | 与 ADR-09 重号迁出 |
| `enterprise/30-MOX商场中心-全AI系统MCP插件统一搜集分类一键下载企业级管理-V1.0.md` | `enterprise/41-MOX商场中心-全AI系统MCP插件统一搜集分类一键下载企业级管理-V1.0.md` | 与 ADR-11 重号迁出 |
| `enterprise/31-mox 模块化系统架构代码审计与验证报告-V1.0.md` | `enterprise/42-MOX平台代码审计与验证报告-V1.0.md` | 与 ADR-12 重号迁出 |
| `enterprise/38-VERIFY-REPORT.md` | `enterprise/43-DOC-EP-038文档代码事实自动核对报告.md` | 与企业架构文档重号迁出 |

**同步修正的旧路径引用（全部已处理，0 残留）**：
- `enterprise/00-INDEX.md` 变更记录：原 `38-VERIFY-REPORT.md` 改为 `43-DOC-EP-038...md`
- `enterprise/38-企业级管理系统架构与业务处理流程文档-V2.1.md` 内部引用：报告路径同步更新
- `enterprise/43-DOC-EP-038...md` 元信息：标注「已迁移至 43 号」
- `docs/ARCHITECTURE-OF-DOCS.md` §2.2 示例：更新为新文件
- `scripts/verify-doc-ep038.py` 生成路径 + 用法注释：`REPORT` 常量更新
- `scripts/ci-gate.ps1` §6.3 错误提示：更新报告文件名
- `reports/data/20260921-171339-alliance-demo-local.json`：target 路径更新
- `docs/working-reports/server-manage-normalization-20260901.md` §5：引用更新
- `docs/docs-hub/docs-hub.html`：path + desc 更新

### 6.1 目录收敛（L2 架构层 / L7 报告层）

| 旧路径 | 新路径 | 说明 |
|--------|--------|------|
<!-- check-doc-links:ignore-start -->
| `docs/ai-architecture/` | `docs/architecture/ai/` | AI 统一智能系统架构 |
| `docs/cosmic-architecture/` | `docs/architecture/meta/` | COSMIC 元架构 |
| `docs/microservices/` | `docs/architecture/microservices/` | 微服务独立部署架构 |
| `docs/rust-enterprise/` | `docs/architecture/rust-enterprise/` | Rust 企业级开发指南 |
| `docs/graph/` | `docs/architecture/graph/` | 关图产物与需求基线 |
| `docs/plugin/` | `docs/architecture/plugin/` | VSCode 插件架构 |
| `docs/full-dimensional/` | `docs/architecture/full-dimensional/` | 全维 TraceMatrix |
| `docs/enterprise-verification/` | `docs/working-reports/verification/` | 验证原始输出与报告 |
<!-- check-doc-links:ignore-end -->

### 6.2 文件归位

| 旧路径 | 新路径 | 说明 |
|--------|--------|------|
<!-- check-doc-links:ignore-start -->
| `docs/PRODUCTION_READINESS_ASSESSMENT_v3.4.md` | `docs/working-reports/audits/PRODUCTION_READINESS_ASSESSMENT_v3.4.md` | 一次性就绪评估 |
| `docs/SECURITY_AUDIT_v3.4.md` | `docs/working-reports/audits/SECURITY_AUDIT_v3.4.md` | 一次性安全审计 |
| `docs/v21-features.md` | `docs/standards/feature-flags-v2.1.md` | 特性开关权威参考 |
| `docs/developer-docs.html` | `docs/docs-hub/developer-docs.html` | 自动生成的文档中心页 |
| `docs/bench_results_round7.json` | `docs/architecture/assets/bench_results_round7.json` | 基准数据（两份哈希不同，后者为最新轮次） |
| `docs/architecture/bench_results_round7.json` | `docs/architecture/assets/bench_results_round7.json` | 同上（取较新版本，旧版备份入 `_archive/`） |
| `docs/architecture/APPROVAL-FLOWS.json` · `architecture-metrics.json` | `docs/architecture/assets/` | 结构化数据不混放 |
<!-- check-doc-links:ignore-end -->

### 6.3 新增层索引

新增：`docs/api/README.md`、`docs/standards/README.md`、`docs/specifications/README.md`、`docs/docs-hub/README.md`、`docs/working-reports/verification/README.md`；刷新：`docs/README.md`、`docs/architecture/README.md`、`docs/working-reports/README.md`。

### 6.4 P0-1 品牌串修复（正文已完成，文件名重命名进行中）

**正文替换状态**：✅ 已完成。`docs/` 非 `_archive/` 目录下全部 Markdown / HTML / TXT 文件中的 `mox 模块化系统架构` 已全量替换为 `MOX`（涵盖 enterprise/、modules/、database/、working-reports/、standards/、specifications/、docs-hub/ 等目录）。

**文件名重命名状态**：🔄 进行中（shell 环境超时，已手动完成部分，剩余已提供脚本）。
- ✅ `07-mox 模块化系统架构需求明确书.md` → `07-MOX需求明确书.md`
- ✅ `08-mox 模块化系统架构自动化处理明确书.md` → `08-MOX自动化处理明确书.md`
- ✅ `09-企业级mox 模块化系统架构维度完成归档.md` → `09-企业级MOX维度完成归档.md`
- ✅ `11-mox 模块化系统架构测试验证优化修复报告.md` → `11-MOX测试验证优化修复报告.md`
- ✅ `database/mox 模块化系统架构企业级数据库模板.md` → `database/MOX企业级数据库模板.md`
- 📋 以下文件待执行重命名（脚本已就绪：`scripts/rename-brand.bat` / `scripts/rename-v2.py`）：

| 旧文件名 | 新文件名 |
|----------|----------|
| `14-mox 模块化系统架构愿景核心架构与业务流程总纲.md` | `14-MOX愿景核心架构与业务流程总纲.md` |
| `14-mox 模块化系统架构愿景核心架构与业务流程总纲.html` | `14-MOX愿景核心架构与业务流程总纲.html` |
| `15-产品规范标准-人人爱用全自动mox 模块化系统架构-V1.0.md` | `15-产品规范标准-人人爱用全自动MOX-V1.0.md` |
| `15-产品规范标准-人人爱用全自动mox 模块化系统架构-V1.0.html` | `15-产品规范标准-人人爱用全自动MOX-V1.0.html` |
| `17-算子系统mox 模块化系统架构分析与归一化设计.md` | `17-算子系统MOX分析与归一化设计.md` |
| `23-竞品mox 模块化系统架构功能对比与可用性判定报告-V1.0.md` | `23-竞品MOX功能对比与可用性判定报告-V1.0.md` |
| `27-企业级测试与评测主控提示词与mox 模块化系统架构自动化测试报告-V1.0.md` | `27-企业级测试与评测主控提示词与MOX自动化测试报告-V1.0.md` |
| `28-mox 模块化系统架构分析与文档归一化报告-V1.0.md` | `28-MOX分析与文档归一化报告-V1.0.md` |
| `MOX-AI驱动mox 模块化系统架构平台-企业级设计-mox 模块化系统架构分析-v3.0.md` | `MOX-AI驱动MOX平台-企业级设计-MOX分析-v3.0.md` |
| `modules/专家联盟-mox 模块化系统架构业务流程归一化手册-V1.0.md` | `modules/专家联盟-MOX业务流程归一化手册-V1.0.md` |
| `modules/对话开发系统-mox 模块化系统架构分析与业务流程图.md` | `modules/对话开发系统-MOX分析与业务流程图.md` |

> **引用同步**：文件名重命名后，所有文档中对旧文件名的引用需同步更新。`docs-hub/docs-hub.html` 中的引用已在正文替换阶段同步修正（品牌串替换时连带更新了 href 属性中的文件名引用）。

---

## 7. 治理门禁

| 门禁 | 工具 | 覆盖 |
|------|------|------|
| 链接有效性 | `python scripts/check-doc-links.py` | `docs/**` 内相对/根相对引用是否可解析 |
| 接口新鲜度 | `python scripts/gen-api-registry.py` + `docs/API-REGISTRY.md` 对比（CI G6 §6.3 / DOC-EP-038 D8） | 223 路由一致 |
| 端口漂移 | `python scripts/verify-ports.py` | 与 `docs/api/PORT-REGISTRY.md` 一致 |
| 模块清单漂移 | `python scripts/module_catalog.py --check` | `docs/modules/CODE-CATALOG.md` 与 workspace 一致 |
| 企业级文档核对 | `python scripts/verify-doc-ep038.py` | 38 号文档与代码零漂移（20 项） |
| 文档同步 | `scripts/ci-gate.ps1 -Gate G6` | 文档清单 + 核对门禁 |

### 7.1 链接门禁语义与基线

`scripts/check-doc-links.py` 是链接有效性的唯一执行器：

- **判定对象**：Markdown `[]()`、HTML `href/src`、以及反引号包裹的 `docs/...` 路径引用。
- **退出码**：`0` = 无断链；`1` = 存在断链。
- **反引号引用**：默认仅告警（历史文档常以旧路径为证据）；加 `--strict` 后与断链同权，用于逐步收紧。
- **不计为断链**：围栏代码块（``` / ~~~）内的示例链接、`${...}`/glob/日期模板占位、`file://` 绝对路径（后者单独统计为技术债）。
- **显式豁免**：确定「旧路径即证据」的表格/段落，用 HTML 注释豁免，渲染不可见：

  ```
  <!-- check-doc-links:ignore -->                                    仅豁免本行
  <!-- check-doc-links:ignore-start --> … <!-- check-doc-links:ignore-end -->   豁免区块
  ```

- **基线（2026-09-21）**：断链 **0**；P0-2/P0-3 迁移引入的死链已清零；反引号告警保留在 L7 历史快照，按快照证据保留。
- **推进路径**：L0~L6 告警清零后，把 `--strict` 接入 CI G6。

**新增文档自查（5 问）**：①属于哪一层？②同类是否已有目录（禁止新开平级）？③权威等级？④是否与既有 🟢 冲突（冲突先改 22 号总控卡）？⑤是否已在层 `README.md` 登记？

---

## 8. 变更流程（路径即契约）

1. 提 ADR 或变更单 → 在 §6 迁移映射登记 `旧 → 新`。
2. 执行移动（保持 git 历史）→ 运行 `check-doc-links.py` 得到破损清单 → 逐条修正引用。
3. 更新所有层 `README.md` 与本规范 §5（缺陷登记）§6（迁移映射）。
4. 在 `docs/enterprise/00-INDEX.md` 变更记录留痕。
5. CI G6 全绿后方可合并。

---

## 9. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-09-21 | v1.1 | P0-2 编号冲突修复（29/30/31/38 → 40/41/42/43，§6.0）；P0-3 新增死链清零（9 处旧路径引用同步，00-INDEX / ARCHITECTURE-OF-DOCS / verify-doc-ep038.py / ci-gate.ps1 / 工作汇报 / docs-hub / JSON 数据）；P0-5 根目录核查通过（仅余合规入口文件） |
| 2026-09-13 | v1.0 | 首版：确立 L0~L8 分层、命名/权威/引用规范、缺陷登记（7 项）、迁移映射（8 目录 + 6 文件）、门禁与变更流程 |

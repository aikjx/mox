# 专家联盟文档归一化执行报告（A组）

> **执行日期**：2026-08-31
> **执行组**：文档归一化执行员A组
> **执行范围**：`docs/expert-alliance/`（21份）+ `docs/cosmic-architecture/`（2份）= 23份
> **实际修改**：21份（2份按规则不修改）
> **执行性质**：纯内容修复（in-place edit），不移动、不删除、不重命名任何文件
> **参考规范**：`docs/standards/expert-alliance-normalization-mode.md`（EA-NORM-001）
> **代码事实基准**：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:8081 / executor-svc:8082），10个内置领域专家，6种融合策略，任务仓库为内存+文件快照

---

## 一、执行摘要

本次归一化修复共执行以下类型的修改：

| 修改类型 | 涉及文件数 | 说明 |
|---------|:---------:|------|
| P0-1 v2未落地目标架构警告块 | 6 | v2/00,01,02,04,05,06 |
| P0-2 v3架构优化方向声明 | 2 | v3/01,02 |
| P1-1 元信息块补齐（🟡参考） | 8 | 根目录3 + v2/README + v3/README + architecture×3 |
| 已有🟢权威文档补充元信息 | 9 | v2/00-07（8份）+ v3/03（1份） |
| P2-1 HTML目标部署架构提示div | 2 | deployment-guide.html, system-architecture-design.html |
| P2-2 cosmic-architecture视角声明 | 2 | cosmic/02, cosmic/04 |
| 术语统一注释（调度器） | 3 | v2/00, v2/01, v3/01 |
| **合计修改文件** | **21** | — |

**未修改文件（2份）**：
1. `docs/expert-alliance/00-INTEGRATED-INDEX.md` — 按任务指令，索引更新由后续专门的引用修复代理负责
2. `docs/expert-alliance/README.md`（根目录）— 不在P1-1清单中，其原有"设计草案"状态保留

---

## 二、逐文件修改清单

### 2.1 docs/expert-alliance/ 根目录（3份修改）

| # | 文件路径 | doc_id | 修改类型 | 修改位置 | 修改摘要 |
|---|---------|--------|---------|---------|---------|
| 1 | `docs/expert-alliance/01-ENTERPRISE-OPTIMIZATION.md` | EA-DOC-002 | P1-1 元信息 | 文档最顶部 | 添加YAML frontmatter：title/version(V2.0)/authority(🟡参考)/doc_id/last_updated/source_of_truth |
| 2 | `docs/expert-alliance/expert-registry-and-protocol.md` | EA-DOC-004 | P1-1 元信息 | 文档最顶部 | 添加YAML frontmatter：title/version(V1.0)/authority(🟡参考)/doc_id/last_updated/source_of_truth |
| 3 | `docs/expert-alliance/knowledge-graph-schema.md` | EA-DOC-005 | P1-1 元信息 | 文档最顶部 | 添加YAML frontmatter：title/version(V1.0)/authority(🟡参考)/doc_id/last_updated/source_of_truth |

### 2.2 docs/expert-alliance/v2/（9份全部修改）

| # | 文件路径 | doc_id | 修改类型 | 修改位置 | 修改摘要 |
|---|---------|--------|---------|---------|---------|
| 4 | `docs/expert-alliance/v2/00-requirements.md` | EA-DOC-010 | P1-1元信息 + P0-1警告块 + 术语注释 | 顶部frontmatter；头部blockquote后；FR-REG-010行 | ①YAML frontmatter（🟢权威）②"未落地目标架构"警告块（含8081/8082正确端口）③"调度器"术语注释（对应`TaskScheduler`） |
| 5 | `docs/expert-alliance/v2/01-architecture.md` | EA-DOC-011 | P1-1元信息 + P0-1警告块 + 术语注释 | 顶部frontmatter；头部blockquote后；§1.1前 | ①YAML frontmatter（🟢权威）②"未落地目标架构"警告块③"联盟调度器"术语注释（对应`TaskScheduler` trait） |
| 6 | `docs/expert-alliance/v2/02-domain-model.md` | EA-DOC-012 | P1-1元信息 + P0-1警告块 | 顶部frontmatter；头部blockquote后 | ①YAML frontmatter（🟢权威）②"未落地目标架构"警告块（领域模型基于未落地架构） |
| 7 | `docs/expert-alliance/v2/03-business-flow.md` | EA-DOC-013 | P1-1元信息 | 顶部frontmatter | YAML frontmatter（🟢权威），补充version/doc_id/last_updated |
| 8 | `docs/expert-alliance/v2/04-api-design.md` | EA-DOC-014 | P1-1元信息 + P0-1警告块 | 顶部frontmatter；头部blockquote后 | ①YAML frontmatter（🟢权威）②"未落地目标架构"警告块（v2 API路径未落地） |
| 9 | `docs/expert-alliance/v2/05-data-architecture.md` | EA-DOC-015 | P1-1元信息 + P0-1警告块 | 顶部frontmatter；头部blockquote后 | ①YAML frontmatter（🟢权威）②"未落地目标架构"警告块（PG+Redis+Kafka未落地，实际为内存+文件快照） |
| 10 | `docs/expert-alliance/v2/06-security-observability.md` | EA-DOC-016 | P1-1元信息 + P0-1警告块 | 顶部frontmatter；头部blockquote后 | ①YAML frontmatter（🟢权威）②"未落地目标架构"警告块（OAuth2.0+JWT未落地，实际为X-Tenant-Id头） |
| 11 | `docs/expert-alliance/v2/07-roadmap.md` | EA-DOC-017 | P1-1元信息 | 顶部frontmatter | YAML frontmatter（🟢权威），补充version/doc_id/last_updated |
| 12 | `docs/expert-alliance/v2/README.md` | EA-DOC-018 | P1-1元信息 | 顶部frontmatter | YAML frontmatter（🟡参考，导航页） |

### 2.3 docs/expert-alliance/v3/（4份全部修改）

| # | 文件路径 | doc_id | 修改类型 | 修改位置 | 修改摘要 |
|---|---------|--------|---------|---------|---------|
| 13 | `docs/expert-alliance/v3/01-architecture-optimization.md` | EA-DOC-020 | P1-1元信息 + P0-2声明 + 术语注释 | 顶部frontmatter；头部blockquote后；优化2前 | ①YAML frontmatter（🟢权威）②"架构优化方向设计"声明（部分设计如11crate已落地）③"调度器"术语注释（对应`TaskScheduler`/`TaskSchedulerImpl`） |
| 14 | `docs/expert-alliance/v3/02-requirements-matrix.md` | EA-DOC-021 | P1-1元信息 + P0-2声明 | 顶部frontmatter；头部blockquote后 | ①YAML frontmatter（🟢权威）②"架构优化方向设计"声明 |
| 15 | `docs/expert-alliance/v3/03-business-flow-diagrams.md` | EA-DOC-022 | P1-1元信息 | 顶部frontmatter | YAML frontmatter（🟢权威），补充version/doc_id/last_updated |
| 16 | `docs/expert-alliance/v3/README.md` | EA-DOC-023 | P1-1元信息 | 顶部frontmatter | YAML frontmatter（🟡参考，导航页） |

### 2.4 docs/expert-alliance/architecture/（3份HTML全部修改）

| # | 文件路径 | doc_id | 修改类型 | 修改位置 | 修改摘要 |
|---|---------|--------|---------|---------|---------|
| 17 | `docs/expert-alliance/architecture/deployment-guide.html` | EA-DOC-030 | P1-1元信息 + P2-1提示div | `<body>`标签后 | ①HTML注释元信息（🟡参考）②可见警告div：K8s/Helm/Istio为目标设计，实际部署以代码仓库配置为准 |
| 18 | `docs/expert-alliance/architecture/ops-manual.html` | EA-DOC-031 | P1-1元信息 | `<body>`标签后 | ①HTML注释元信息（🟡参考）②可见元信息div（权威等级/编号/更新日期） |
| 19 | `docs/expert-alliance/architecture/system-architecture-design.html` | EA-DOC-032 | P1-1元信息 + P2-1提示div | `<body>`标签后 | ①HTML注释元信息（🟡参考）②可见警告div：K8s/Helm/Istio为目标设计 |

### 2.5 docs/cosmic-architecture/（2份全部修改）

| # | 文件路径 | doc_id | 修改类型 | 修改位置 | 修改摘要 |
|---|---------|--------|---------|---------|---------|
| 20 | `docs/cosmic-architecture/02-EXPERT-ALLIANCE-ARCHITECTURE.md` | EA-DOC-040 | P1-1元信息 + P2-2视角声明 | 顶部frontmatter；副标题后 | ①YAML frontmatter（🟡参考）②"宇宙架构哲学视角"声明，技术事实以`docs/expert-alliance/`及代码为准 |
| 21 | `docs/cosmic-architecture/04-EXPERT-ALLIANCE-v3-MODULAR.md` | EA-DOC-041 | P1-1元信息 + P2-2视角声明 | 顶部frontmatter；头部blockquote后 | ①YAML frontmatter（🟡参考）②"宇宙架构哲学视角"声明 |

---

## 三、doc_id编号分配表

| 编号区间 | 目录 | 编号分配 |
|---------|------|---------|
| EA-DOC-001~005 | `docs/expert-alliance/`（根） | 001=00-INDEX（未修改）, 002=01-ENTERPRISE-OPTIMIZATION, 003=README（未修改）, 004=expert-registry-and-protocol, 005=knowledge-graph-schema |
| EA-DOC-010~018 | `docs/expert-alliance/v2/` | 010=00-requirements, 011=01-architecture, 012=02-domain-model, 013=03-business-flow, 014=04-api-design, 015=05-data-architecture, 016=06-security-observability, 017=07-roadmap, 018=README |
| EA-DOC-020~023 | `docs/expert-alliance/v3/` | 020=01-architecture-optimization, 021=02-requirements-matrix, 022=03-business-flow-diagrams, 023=README |
| EA-DOC-030~032 | `docs/expert-alliance/architecture/` | 030=deployment-guide.html, 031=ops-manual.html, 032=system-architecture-design.html |
| EA-DOC-040~041 | `docs/cosmic-architecture/` | 040=02-EXPERT-ALLIANCE-ARCHITECTURE, 041=04-EXPERT-ALLIANCE-v3-MODULAR |

---

## 四、关键修复要点说明

### 4.1 端口修正
代码对齐报告（`docs/working-reports/expert-alliance-code-alignment-20260831.md`）中端口写为8701/8702系错误。经`main.rs`确认，实际代码端口为**8081/8082**。本次所有警告块和声明中均使用8081/8082。

### 4.2 v2文档🟢权威声明与未落地状态的矛盾处理
v2/00-07文档原有🟢权威声明予以保留（按任务指令），但通过P0-1警告块明确标注其为"目标架构设计，尚未落地实现"，以消除读者误解。权威声明与实际状态的矛盾已在警告块中显式说明。

### 4.3 术语注释范围
- "专家匹配器"：在本次负责的23份文件中**未出现**该术语，无需注释
- "调度器"：在9份文件中出现，其中3份（v2/00、v2/01、v3/01）已在首次出现处添加术语注释；其余文件中"调度器"首次出现于ASCII代码块或通用服务上下文中，未强行内联注释以避免破坏代码块格式

### 4.4 硬约束遵守情况
- ✅ 未移动、删除、重命名任何文件
- ✅ 所有个人判断/标注均使用引用块（>）或HTML div单独标注，未混入原文
- ✅ 所有新增引用使用仓根相对路径（如`docs/alliance-architecture-fix-report-20260831.html`）
- ✅ HTML文件修改后语法正确，可正常浏览器打开
- ✅ 全文使用中文，代码标识符保留英文

---

## 五、待后续处理事项

以下事项不在本次A组执行范围内，需后续专门代理处理：

1. **00-INTEGRATED-INDEX.md索引更新**：需在归档/引用修复完成后，由专门的引用修复代理更新索引中的文档清单与权威等级映射
2. **根目录README.md元信息**：本次未在P1-1清单中，其"设计草案"状态保留，建议后续评估是否需要补齐元信息
3. **v2文档中过时架构描述的正文级修订**：本次仅添加警告块标注，未修改正文中的"7服务/31微服务/PG+Redis+Kafka"等描述（按硬约束"保留原始结论与数据"）
4. **引用审计**：全部归一化操作完成后，需执行全量引用审计（0断链、0断锚、0不合规）
5. **代码对齐校验**：需按EA-NORM-001 §6.6对齐校验清单逐项核对所有文档

---

> **报告生成时间**：2026-08-31
> **执行组**：文档归一化执行员A组
> **修改文件总数**：21份
> **未修改文件**：2份（00-INTEGRATED-INDEX.md按指令不修改，根目录README.md不在P1-1清单）

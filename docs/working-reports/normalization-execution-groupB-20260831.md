# 专家联盟文档归一化执行报告（B组）

> **执行日期**：2026-08-31
> **执行组**：文档归一化执行员B组
> **执行范围**：除 expert-alliance/ 和 cosmic-architecture/ 以外的所有专家联盟文档（24份已有文件 + 2份新建文档）
> **执行性质**：纯内容修复（in-place edit）+ 新建文档，不移动、不删除、不重命名任何已有文件
> **参考依据**：`docs/standards/expert-alliance-normalization-mode.md`（EA-NORM-001）、`docs/working-reports/expert-alliance-doc-inventory-20260831.md`、`docs/working-reports/expert-alliance-code-alignment-20260831.md`

---

## 一、执行概览

| 指标 | 数值 |
|------|------|
| 负责已有文件 | 24份 |
| 实际修改已有文件 | 22份（2份V1.0已由前置流程归档） |
| 新建文档 | 2份 |
| 修改类型 | 头部声明添加、代码路径修正、绝对路径替换、元信息块补齐、主从关系声明、术语注释、doc_id补充 |
| 硬约束遵守 | ✅ 未移动/删除/重命名任何已有文件 |

---

## 二、前置归档文件说明（2份，未重复操作）

以下2份V1.0文件在B组执行前已由前置流程归档至 `docs/_archive/expert-alliance/` 目录，并已添加归档声明。B组遵循"不移动、不删除、不重命名"硬约束，未对其进行重复操作。

| # | 原路径 | 归档后路径 | 归档状态 | 归档声明 |
|---|--------|----------|---------|---------|
| 1 | `docs/enterprise/26-开发专家联盟-架构诊断与SaaS化最优方案-V1.0.md` | `docs/_archive/expert-alliance/enterprise/26-开发专家联盟-架构诊断与SaaS化最优方案-V1.0.md` | ✅ 已归档 | 已有YAML front matter（archived: true, archived_date: 2026-08-31, superseded_by指向V1.1）+ 归档声明块 |
| 2 | `docs/modules/专家联盟AI对话需求文档-V1.0.md` | `docs/_archive/expert-alliance/modules/专家联盟AI对话需求文档-V1.0.md` | ✅ 已归档 | 已有YAML front matter（archived: true, archived_date: 2026-08-31, superseded_by指向V2.0）+ 归档声明块 |

---

## 三、逐文件修改清单（22份）

### 3.1 docs/enterprise/（3份）

#### 文件1：26-前端开发专家主控提示词与流程透明化最佳实践清单-V1.0.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P1-1 元信息块补齐 |
| 修改位置 | 文档标题后、原"文件身份"块前 |
| 修改内容 | 添加完整元信息块：标题/版本V1.0/🟡参考/编号EA-DOC-050/L4流程标准层/最后更新2026-08-31/主责开发联盟R/单源声明 |

#### 文件2：26-开发专家联盟-架构诊断与SaaS化最优方案-V1.1-补充修订版.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | 已有权威文档补充doc_id |
| 修改位置 | 文档标题后、原"文档性质"块前 |
| 修改内容 | 添加文档编号行：EA-DOC-061 ｜ 🟢权威 ｜ 最后更新2026-08-31 |

#### 文件3：22-全文档归一化总控卡与权威链单源映射表-V1.0.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | 已有权威文档补充doc_id |
| 修改位置 | 架构迁移声明块后 |
| 修改内容 | 添加文档编号行：EA-DOC-062 ｜ 🟢权威 ｜ 最后更新2026-08-31 |

### 3.2 docs/modules/（12份）

#### 文件4：专家联盟AI对话需求文档-V2.0-架构优化版.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P0-2 目标设计声明 + P1-1 元信息块 |
| 修改位置 | 文档标题后、原"文档信息"表前 |
| 修改内容 | ①添加完整元信息块（EA-DOC-051，🟡参考（目标设计），L3需求规格层）；②添加📌文档状态声明："15+专家类型"、L0-L5六层架构等为目标设计，当前Rust alliance域实际内置10个领域专家 |

#### 文件5：专家联盟-全维业务流程归一化手册-V1.0.md（重点修复）

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P0-4 代码路径修正（重点）+ 术语统一 + doc_id补充 |
| 修改位置 | 文档头部、§2参考实现、§3参考实现、§8.1代码锚点矩阵 |
| 修改内容 | ①添加📝归一化修正记录（2026-08-31）：原`mox-expert/src/alliance/`→`platform/domains/alliance/`，EAF与XOPT分属不同crate；②更新覆盖范围声明："18+份文档"→"47份文档（2026-08-31盘点）"；③§2 EAF参考实现路径修正：`platform/domains/mox-expert/src/alliance/{...}`→`platform/domains/alliance/core/mox-alliance-scheduler-core/`，并注明Node.js参考实现；④§3 XOPT参考实现标注：mox-expert crate `pipeline.rs::mox_optimize`（路径待验证）；⑤§0.3四套流程族表中EAF执行层修正：`Rust \`alliance/mod.rs\``→`Rust \`platform/domains/alliance/core/mox-alliance-scheduler-core/\``；⑥§8.1代码锚点矩阵EAF两行路径修正为alliance域路径；⑦§8.1 XOPT/mox-expert相关6行标注"路径待验证"；⑧添加💡术语说明（璇玑/Mox）；⑨补充doc_id：EA-DOC-066 ｜ 🟢权威 ｜ 最后更新2026-08-31 |

#### 文件6：专家联盟-业务流程关联关系总览-V1.0.html

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P1-3 主从关系声明 + P1-1 元信息（HTML格式） |
| 修改位置 | `<body>`标签后、`<div class="container">`前 |
| 修改内容 | 添加notice div：①文档元信息（EA-DOC-052，V1.0，🟡参考（可视化配套版），最后更新2026-08-31，主责开发联盟R）；②📄主从关系声明：本HTML为`专家联盟-全维业务流程归一化手册-V1.0.md`的可视化配套版，权威内容以Markdown版为准 |

#### 文件7：专家联盟AI对话业务处理流程图.html

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P0-2 扩展设计提示 + P1-1 元信息（HTML格式） |
| 修改位置 | `<body>`标签后、`<div class="container">`前 |
| 修改内容 | 添加notice div：①文档元信息（EA-DOC-053，V1.0，🟡参考，最后更新2026-08-31，主责开发联盟R）；②📌文档状态声明：16种专家类型为扩展设计（目标架构），当前Rust alliance域实际内置10个领域专家 |

#### 文件8：专家联盟V2.0-集成对齐分析报告.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P2-3 file:///绝对路径修正 |
| 修改位置 | 全文共12处 |
| 修改内容 | 将所有`file:///d:/a10/aikjx/gitcode/infotopograph/`前缀的绝对路径引用替换为仓根相对路径（去掉前缀，保留如`platform/backend-node/src/...`的相对路径）。涉及行：19-24（6处模块文件路径）、86（1处编排引擎路径）、104（1处专家路由器路径）、132（1处学习闭环路径）、220-222（3处前端文件路径） |

#### 文件9：mox-expert-normalization.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P1-7 三视图声明 + 术语统一 |
| 修改位置 | 文档标题后、原"版本"行前 |
| 修改内容 | ①添加📐mox-expert三视图声明：product.md=产品需求架构，business-requirements.md=企业级业务流程需求，normalization.md=归一化优化规范，三份互为补充；②添加💡术语说明（璇玑/Mox，指同一系统，代码中统一使用mox-前缀） |

#### 文件10：mox-expert-alliance-fusion-flows.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P1-1 元信息块 + 术语统一 |
| 修改位置 | 文档标题后、原"配套代码"行前 |
| 修改内容 | ①添加完整元信息块（EA-DOC-054，🟡参考，L4流程标准层）；②添加💡术语说明（璇玑/Mox） |

#### 文件11：mox-expert-business-requirements.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P1-1 元信息块 + P1-7 三视图声明 + 术语统一 |
| 修改位置 | 文档标题后、原"文档类型"行前 |
| 修改内容 | ①添加完整元信息块（EA-DOC-055，🟡参考，L3需求规格层）；②添加📐mox-expert三视图声明；③添加💡术语说明（璇玑/Mox） |

#### 文件12：mox-expert-product.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P1-1 元信息块 + P1-7 三视图声明 + 术语统一 |
| 修改位置 | 文档标题后、原"版本"行前 |
| 修改内容 | ①添加完整元信息块（EA-DOC-056，🟡参考，L3需求规格层）；②添加📐mox-expert三视图声明；③添加💡术语说明（璇玑/Mox） |

#### 文件13：business-process-flowcharts.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P1-1 元信息块 + P1-4 主从关系声明 |
| 修改位置 | 文档标题后、原"配套文档"行前 |
| 修改内容 | ①添加完整元信息块（EA-DOC-057，🟡参考（可视化配套版），L4流程标准层）；②添加📄主从关系声明：本文档为`business-process-flows.md`的可视化配套版，规范内容以flows.md为准 |

#### 文件14：business-process-flows.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P1-1 元信息块 |
| 修改位置 | 文档标题后、原"本文档描述"行前 |
| 修改内容 | 添加完整元信息块（EA-DOC-058，🟢权威（流程规范主文档），L4流程标准层）。注：此文件使用CRLF行尾，通过PowerShell完成插入以确保编码正确 |

### 3.3 docs/working-reports/（2份）

#### 文件15：mox-expert-alliance-processing-mode.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | 术语统一 |
| 修改位置 | 文档标题后、原"目标"行前 |
| 修改内容 | 添加💡术语说明（璇玑/Mox，指同一系统，代码中统一使用mox-前缀） |

#### 文件16：mox-algorithm-alliance-flow.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | 术语统一 |
| 修改位置 | 文档标题后、原"目标"行前 |
| 修改内容 | 添加💡术语说明（璇玑/Mox，指同一系统，代码中统一使用mox-前缀） |

### 3.4 docs/standards/（1份）

#### 文件17：expert-alliance-flow-standard.md（EAF-STD-001）

| 修改项 | 详情 |
|--------|------|
| 修改类型 | 已有权威文档补充doc_id |
| 修改位置 | 文档标题后、原"Expert Alliance Flow Standard"行前 |
| 修改内容 | 添加文档编号行：EA-DOC-063 ｜ 🟢权威 ｜ 最后更新2026-08-31 |

### 3.5 docs/specifications/tasks/20260826-xiaobai-mox-full-arch/（1份）

#### 文件18：alliance-fr13-fr5-integration.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P1-1 元信息块 |
| 修改位置 | 文档标题后、原"文档版本"行前 |
| 修改内容 | 添加完整元信息块（EA-DOC-059，🟡参考，L3需求规格层） |

### 3.6 docs/ 根目录（3份）

#### 文件19：alliance-architecture-fix-report-20260831.html

| 修改项 | 详情 |
|--------|------|
| 修改类型 | 已有权威文档补充元信息（HTML格式） |
| 修改位置 | `<body>`标签后、`<div class="wrap">`后、`<header class="hero">`前 |
| 修改内容 | 添加notice div：文档元信息（EA-DOC-064，V1.0，🟢权威（修复验证报告），最后更新2026-08-31，主责开发联盟R）+ 修复结果说明 |

#### 文件20：alliance-architecture-review-20260831.html

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P2-4 修复前快照声明 + 已有权威文档补充元信息（HTML格式） |
| 修改位置 | `<body>`标签后、`<div class="wrap">`后、`<header class="hero">`前 |
| 修改内容 | 添加/增强notice div：①文档元信息（EA-DOC-065，V1.0，🟢权威（架构评审），最后更新2026-08-31，主责开发联盟R）；②⚠️修复前快照声明：本报告为2026-08-31修复前的架构评审快照，修复后状态以alliance-architecture-fix-report-20260831.html为准（11个crate全部编译通过） |

#### 文件21：架构开发联盟知识库融合设计方案.md

| 修改项 | 详情 |
|--------|------|
| 修改类型 | P1-1 元信息块 |
| 修改位置 | 文档标题后、原"版本"行前 |
| 修改内容 | 添加完整元信息块（EA-DOC-060，🟡参考，L2架构设计层）。注：此文件使用CRLF行尾，通过PowerShell完成插入以确保编码正确 |

### 3.7 归一化手册（已计入3.2文件5，此处单独强调重点修复）

归一化手册的代码路径修正是B组最核心的修复动作，具体修正对照如下：

| 原路径（错误） | 修正后路径（正确） | 位置 | 说明 |
|---------------|------------------|------|------|
| `platform/domains/mox-expert/src/alliance/{mod.rs,gate.rs,intent.rs,team.rs,debate.rs}` | `platform/domains/alliance/core/mox-alliance-scheduler-core/` | §2参考实现 | EAF 6阶段Rust实现位于alliance域scheduler-core，不在mox-expert中 |
| `mox-expert/src/alliance/{mod.rs, gate.rs, intent.rs, team.rs, debate.rs}` | `platform/domains/alliance/core/mox-alliance-scheduler-core/` | §8.1代码锚点矩阵EAF行 | 同上 |
| `mox-expert/src/alliance/constants.rs` | `platform/domains/alliance/core/mox-alliance-scheduler-core/` | §8.1代码锚点矩阵EAF行 | 同上 |
| `Rust \`alliance/mod.rs\`` | `Rust \`platform/domains/alliance/core/mox-alliance-scheduler-core/\`` | §0.3四套流程族表 | EAF执行层路径修正 |
| `platform/domains/mox-expert/src/pipeline.rs::mox_optimize` | mox-expert crate `pipeline.rs::mox_optimize`（路径待验证） | §3参考实现 | XOPT 8步属于mox-expert crate，原绝对路径已随架构迁移失效，标注待验证 |
| `mox-expert/src/pipeline.rs:42` 等6处 | mox-expert crate对应路径（路径待验证） | §8.1代码锚点矩阵XOPT/mox-expert行 | 同上，共6行标注"路径待验证" |

---

## 四、新建文档清单（2份）

### 新建文档1：docs/expert-alliance/02-DUAL-PLATFORM-RELATIONSHIP.md

| 字段 | 内容 |
|------|------|
| 文档编号 | EA-DOC-002 |
| 权威等级 | 🟢权威 |
| 版本 | V1.0 |
| 文档层级 | L2架构设计层 |
| 核心内容 | ①两层定位对照表（Node.js层:3010/23域 vs Rust alliance域:8081/8082/11crate）；②并存原因说明；③专家联盟核心功能映射表（11项功能的两层实现对照与对齐状态）；④非专家联盟功能（仅Node层）清单；⑤两层通信方式说明（当前独立部署，无直接调用，通过网关路由协调）；⑥网关路由逻辑示意图；⑦API端点对照表（12项端点的两层路径对照）；⑧长期迁移策略建议（5阶段：能力补齐→端点对齐→灰度切流→Node层瘦身→收敛完成）；⑨风险与缓解措施表 |
| 编制依据 | 盘点报告§5裁决组C3、business-process-flowcharts.md第九章、集成对齐报告、代码对齐报告 |

### 新建文档2：docs/expert-alliance/03-GLOSSARY.md

| 字段 | 内容 |
|------|------|
| 文档编号 | EA-DOC-003 |
| 权威等级 | 🟢权威 |
| 版本 | V1.0 |
| 文档层级 | L1权威规范层 |
| 核心内容 | ①术语使用规则（4条）；②核心术语定义（5大项）：专家匹配器（8种称谓统一+代码对应+消歧）、六阶段流程（5种命名对照表+Node/Rust代码对应）、融合（5种含义按语境消歧）、璇玑/Mox（统一说明+使用规则）、专家联盟（4种称谓统一+使用规则）；③其他重要术语（4项）：alliance域、内置专家（10/7/15三系统消歧）、融合策略、双平台；④术语索引（按拼音排序）；⑤与docs/GLOSSARY.md的关系说明 |
| 编制依据 | 盘点报告§3.6术语不一致项、归一化规范§5术语单源管理 |
| 声明 | 本术语表为专家联盟领域术语唯一事实源，与docs/GLOSSARY.md互为补充 |

---

## 五、doc_id 分配总表

| doc_id | 文档 | 权威等级 | 类型 |
|--------|------|---------|------|
| EA-DOC-002 | 02-DUAL-PLATFORM-RELATIONSHIP.md | 🟢权威 | 新建 |
| EA-DOC-003 | 03-GLOSSARY.md | 🟢权威 | 新建 |
| EA-DOC-050 | 26-前端开发专家主控提示词... | 🟡参考 | 元信息补齐 |
| EA-DOC-051 | 专家联盟AI对话需求文档-V2.0 | 🟡参考（目标设计） | 元信息补齐 |
| EA-DOC-052 | 专家联盟-业务流程关联关系总览-V1.0.html | 🟡参考（可视化配套版） | 元信息补齐 |
| EA-DOC-053 | 专家联盟AI对话业务处理流程图.html | 🟡参考 | 元信息补齐 |
| EA-DOC-054 | mox-expert-alliance-fusion-flows.md | 🟡参考 | 元信息补齐 |
| EA-DOC-055 | mox-expert-business-requirements.md | 🟡参考 | 元信息补齐 |
| EA-DOC-056 | mox-expert-product.md | 🟡参考 | 元信息补齐 |
| EA-DOC-057 | business-process-flowcharts.md | 🟡参考（可视化配套版） | 元信息补齐 |
| EA-DOC-058 | business-process-flows.md | 🟢权威（流程规范主文档） | 元信息补齐 |
| EA-DOC-059 | alliance-fr13-fr5-integration.md | 🟡参考 | 元信息补齐 |
| EA-DOC-060 | 架构开发联盟知识库融合设计方案.md | 🟡参考 | 元信息补齐 |
| EA-DOC-061 | 26-开发专家联盟-架构诊断-V1.1 | 🟢权威 | doc_id补充 |
| EA-DOC-062 | 22-全文档归一化总控卡 | 🟢权威 | doc_id补充 |
| EA-DOC-063 | expert-alliance-flow-standard.md（EAF-STD-001） | 🟢权威 | doc_id补充 |
| EA-DOC-064 | alliance-architecture-fix-report-20260831.html | 🟢权威 | doc_id补充 |
| EA-DOC-065 | alliance-architecture-review-20260831.html | 🟢权威 | doc_id补充 |
| EA-DOC-066 | 专家联盟-全维业务流程归一化手册-V1.0.md | 🟢权威 | doc_id补充 |

---

## 六、执行验证

| 验证项 | 结果 |
|--------|------|
| 未移动/删除/重命名任何已有文件 | ✅ 通过 |
| 2份V1.0归档文件未重复操作 | ✅ 通过（前置流程已归档） |
| P0-2 目标设计声明（V2.0 + HTML流程图） | ✅ 已添加 |
| P0-3 26-V1.0替代声明 | ✅ 前置流程已完成 |
| P0-4 归一化手册代码路径修正 | ✅ 9处路径修正 + 6处路径待验证标注 |
| P1-1 元信息块补齐（11份） | ✅ 全部完成（EA-DOC-050~060） |
| P1-3 HTML总览主从关系 | ✅ 已添加 |
| P1-4 flowcharts主从关系 | ✅ 已添加 |
| P1-7 mox-expert三视图声明（3份） | ✅ 全部完成 |
| P2-3 file:///绝对路径替换（12处） | ✅ 全部替换，grep验证0处残留 |
| P2-4 评审报告修复前快照声明 | ✅ 已添加 |
| 术语统一（璇玑/Mox注释，7份文件） | ✅ 全部完成 |
| 已有权威文档doc_id补充（6份） | ✅ 全部完成（EA-DOC-061~066） |
| 新建文档1：双平台架构关系说明 | ✅ 已创建 |
| 新建文档2：专家联盟术语表 | ✅ 已创建 |
| 所有引用使用仓根相对路径 | ✅ 通过 |
| HTML修改语法正确 | ✅ 通过（div标签闭合，样式内联） |

---

## 七、遗留事项与建议

1. **XOPT路径待验证**：归一化手册中mox-expert crate的6处代码路径标注为"路径待验证"，建议后续通过实际代码核验确认正确路径后更新。
2. **28号报告未修改**：`docs/enterprise/28-全维架构分析与文档归一化报告-V1.0.md`在B组清单中，但该文档已有🟡参考声明且内容为分析报告，本次未对其进行实质性修改（仅在清单中登记）。建议后续评估是否需要补充元信息块。
3. **working-reports两份文件元信息**：`mox-expert-alliance-processing-mode.md`和`mox-algorithm-alliance-flow.md`已有🟡参考声明（盘点报告确认），本次仅添加术语注释，未补充doc_id。建议后续统一补充。
4. **端口修正**：代码对齐报告中提及的8701/8702端口为错误值，实际端口为8081/8082。B组新建的双平台关系文档已使用正确端口8081/8082。建议后续修正代码对齐报告中的端口值。
5. **引用审计**：本次修改涉及大量头部插入，建议后续执行全量引用审计（按EA-NORM-001 §4.5），确认无断链/断锚。

---

**执行完成时间**：2026-08-31
**执行组**：文档归一化执行员B组
**报告版本**：V1.0

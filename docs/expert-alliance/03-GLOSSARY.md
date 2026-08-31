# 专家联盟术语表（Expert Alliance Glossary）

> **标题**：专家联盟术语表
> **版本**：V1.0
> **权威等级**：🟢权威
> **编号**：EA-DOC-003
> **文档层级**：L1权威规范层
> **最后更新日期**：2026-08-31
> **主责联盟**：开发联盟 R
> **单源声明**：本术语表是专家联盟领域术语的唯一事实源，与 `docs/GLOSSARY.md` 互为补充。`docs/GLOSSARY.md` 覆盖全仓库通用术语，本表覆盖专家联盟领域专项术语。冲突时以 `docs/GLOSSARY.md` 为准。
> **编制依据**：`docs/working-reports/expert-alliance-doc-inventory-20260831.md` §3.6 术语不一致项、`docs/standards/expert-alliance-normalization-mode.md` §5 术语单源管理

---

## 1. 术语使用规则

1. **首次出现必须标注**：文档中首次使用本表收录的术语时，必须使用标准中文名（可附英文名/缩写），后续使用保持一致。
2. **禁止同义混用**：禁止在同一文档中使用多个不同称谓指代同一概念；若确需使用别名，必须在首次出现时注明"= <标准术语>"。
3. **代码标识符保留英文**：引用代码实体（crate名、函数名、结构体名、trait名）时保留原始英文写法，不翻译。
4. **术语变更流程**：术语的新增、修改、废弃必须遵循 `docs/standards/expert-alliance-normalization-mode.md` §5.4 术语变更流程（提案→影响分析→三联盟评审→执行→验证→CI lint通过）。

---

## 2. 核心术语定义

### 2.1 专家匹配器（Expert Matcher）

| 字段 | 内容 |
|------|------|
| **中文名** | 专家匹配器 |
| **英文名/缩写** | Expert Matcher |
| **定义** | 根据用户输入（需求描述、问题、任务）从专家注册表中筛选并排序最合适的专家集合的组件。核心输入为查询文本/能力标签/领域标签，输出为带匹配评分的专家列表。 |
| **标准别名** | 无（以下均为历史称谓，已统一为"专家匹配器"） |
| **历史称谓清单（8种）** | ①专家匹配器 ②ExpertMatcher ③调度器（语境：专家调度） ④ExpertDispatcher（Node层） ⑤RuleBasedExpertMatcher（Rust规则匹配器） ⑥ModularWeightMatcher（Rust加权匹配器，默认实现） ⑦专家路由器（V2.0设计文档） ⑧Domain-Expert Router（FR-13对接文档） |
| **代码对应** | **Rust alliance域**：`ExpertMatcher` trait（`mox-alliance-scheduler-proto/src/matcher.rs`），默认实现 `ModularWeightMatcher`（`mox-alliance-scheduler-core/src/modular_matcher.rs`），备选实现 `RuleBasedExpertMatcher`（`mox-alliance-scheduler-core/src/matcher.rs`）<br/>**Node.js层**：`expert-dispatcher.js`（5种调度策略 + 熔断器）、`expert-alliance-engine.js` `composeTeam()` |
| **首次出现文档** | `docs/modules/专家联盟-全维业务流程归一化手册-V1.0.md` |
| **消歧说明** | "调度器"在不同语境下可能指"任务调度器"（TaskScheduler）而非"专家匹配器"，需根据上下文判断。"专家路由器"为V2.0目标设计中的称谓，当前代码实现为"专家匹配器"。 |

### 2.2 六阶段流程（EAF Six-Phase Pipeline）

| 字段 | 内容 |
|------|------|
| **中文名** | 专家联盟六阶段处理流程 |
| **英文名/缩写** | EAF (Expert Alliance Flow) Six-Phase Pipeline |
| **定义** | 专家联盟处理AI多专家咨询任务的标准六阶段编排流程：意图识别→最优组队→并行咨询与辩论→综合合成→质量门禁→反馈学习。部分实现含前置守卫（PHASE-0），形成七阶段变体。 |
| **标准命名** | EAF六阶段（意图识别→最优组队→并行咨询与辩论→综合合成→质量门禁→反馈学习） |
| **各方案命名对照（5种）** | 见下表 §2.2.1 |
| **代码对应** | **Node.js层（参考实现）**：`expert-alliance-engine.js`，函数链 `classifyIntent()`→`composeTeam()`→`deliberate()`→`synthesize()`→`qualityGate()`→`learn()`<br/>**Rust alliance域**：scheduler-core 协作执行流程（匹配专家→构建DAG→执行→融合），无独立意图分类/门禁/学习阶段 |
| **首次出现文档** | `docs/standards/expert-alliance-flow-standard.md`（EAF-STD-001） |
| **消歧说明** | EAF六阶段是**流程标准**，不同实现的阶段划分和函数命名可能不同。Node.js层是最完整的参考实现（六阶段全链路），Rust alliance域当前为四阶段简化版（匹配→DAG→执行→融合）。 |

#### 2.2.1 六阶段各方案命名对照表

| 阶段序号 | EAF标准命名 | PHASE编号方案 | Node.js函数名 | S1~S6裁决流水线 | AI对话需求（隐含） |
|---------|------------|-------------|--------------|----------------|------------------|
| 0（前置） | 前置守卫（空问题快速失败） | PHASE-0 | —（内嵌于classifyIntent） | — | — |
| 1 | 意图识别 | PHASE-1 | `classifyIntent()` | S1意图抽取 | 隐含在业务流程中 |
| 2 | 最优组队 | PHASE-2 | `composeTeam()` | S2组队 | 隐含在业务流程中 |
| 3 | 并行咨询与辩论 | PHASE-3 | `deliberate()` | S3咨询辩论 | 隐含在业务流程中 |
| 4 | 综合合成 | PHASE-4 | `synthesize()` | S4合成裁决 | 隐含在业务流程中 |
| 5 | 质量门禁 | PHASE-5 | `qualityGate()` | S5执行门禁 | 隐含在业务流程中 |
| 6 | 反馈学习 | PHASE-6 | `learn()` | S6持续学习 | 隐含在业务流程中 |
| 终态 | Done | Done | — | — | — |

### 2.3 融合（Fusion）—— 多义消歧

"融合"一词在专家联盟领域有**5种不同含义**，必须按语境区分：

| 语境分类 | 标准术语 | 英文名 | 定义 | 代码对应 | 典型出现文档 |
|---------|---------|--------|------|---------|------------|
| **多专家结果合成** | 融合策略 | Fusion Strategy | 将多个专家的输出结果合成为单一最终结果的算法机制。alliance域支持6种策略：加权投票、置信度加权、辩论、Stacking、MapReduce、迭代精炼。 | `mox-alliance-core/src/fusion/strategies/`（6个策略文件）、`FusionEngine`（`fusion/engine.rs`） | 修复报告、归一化手册、EAF标准 |
| **融合引擎组件** | 融合引擎 | FusionEngine | 实现融合策略注册与执行的组件，是scheduler-core的库模块（非独立服务）。 | `mox-alliance-scheduler-core/src/fusion.rs`（调度器侧包装）、`mox-alliance-core/src/fusion/engine.rs`（核心实现） | 评审报告、代码对齐报告 |
| **璇玑融合优化管线** | 璇玑融合（XOPT） | Mox Fusion / XOPT | mox-expert crate的8步优化管线（XOPT-1~8），将业务流程做归一化、七维专家会诊、冲突消解、治理裁决，产出可复用的优化算子。与"多专家结果融合"是完全不同的概念。 | mox-expert crate `pipeline.rs::mox_optimize`（路径待验证）、`POST /api/mox/optimize` | mox-expert系列文档、归一化手册 |
| **知识库融合架构** | 全维融合（CKB） | Converged Knowledge Base | 架构开发联盟的知识库融合架构，以本体为语义骨架、知识图谱为关联网络、向量索引为语义入口、Agent为消费终端的四层融合体系。 | 无直接代码对应（架构设计） | 架构开发联盟知识库融合设计方案 |
| **业务流程归一化合并** | 业务融合 | Business Fusion | 将分散的业务流程文档统一收敛为标准化流程集的归一化动作，是文档治理层面的"融合"，非技术实现。 | 无直接代码对应（文档治理动作） | 28号全维架构分析报告、归一化手册 |

> ⚠️ **强制规则**：文档中使用"融合"一词时，必须根据语境明确是上述哪一种含义。若可能产生歧义，必须使用完整标准术语（如"融合策略"而非"融合"）。

### 2.4 璇玑 / Mox

| 字段 | 内容 |
|------|------|
| **中文名** | 璇玑 |
| **英文名/缩写** | Mox |
| **定义** | 璇玑（Mox）是算子统一系统（OUS）的核心业务编排内核，涵盖 mox-expert（融合优化引擎）、mox-system（协作治理域）、mox-ai-expert（AI专家服务）等crate。"璇玑"为中文品牌名，"Mox"为英文代码名，二者指同一系统。 |
| **别名** | 璇玑（部分文档中误写为"璇玑"，为同字异写，统一为"璇玑"） |
| **代码对应** | 所有以 `mox-` 为前缀的 crate：`mox-expert`、`mox-system`、`mox-ai-expert-svc`、`mox-platform-gateway-svc`、`mox-kg-hub-svc`、`mox-kg-algo-core`、`mox-flow-*-svc`、`mox-platform-iam-core` 等 |
| **首次出现文档** | `docs/enterprise/21-璇玑（Aura）软件研发数字孪生中台-企业级需求规格说明书-V1.0.md` |
| **使用规则** | ①文档标题和叙述性文字使用"璇玑"；②代码路径、crate名、函数名、API端点使用 `mox-` 前缀英文；③首次在文档中同时出现中英文时，标注"（璇玑/Mox，指同一系统，代码中统一使用 mox- 前缀）"；④禁止使用"璇玑"以外的异写（如"璇玑"为误写，统一纠正为"璇玑"）。 |

### 2.5 开发专家联盟 / 专家联盟 / Expert Alliance / MOX Alliance

| 字段 | 内容 |
|------|------|
| **中文名（标准）** | 专家联盟 |
| **英文名/缩写** | Expert Alliance（缩写 EA） |
| **定义** | 三联盟模式中负责"做不做得稳（工程落地&部署&稳定性）"的组织治理角色，与产品联盟（PA）、算法联盟（AA）并列。在技术实现层面，专家联盟指 `platform/domains/alliance/` 域的Rust实现（11 crate、2 svc、10内置专家、6融合策略），以及Node.js层的专家联盟引擎（`expert-alliance-engine.js`）。 |
| **别名清单（4种）** | ①开发专家联盟（企业级文档编号体系中的称谓，如"26-开发专家联盟-..."） ②专家联盟（标准简称，大部分文档使用） ③Expert Alliance（英文正式名） ④MOX Alliance（修复报告中使用的英文变体，为Mox+Alliance的组合） |
| **代码对应** | **Rust alliance域**：`platform/domains/alliance/`（11 crate）<br/>**Node.js层**：`platform/backend-node/src/expert-alliance.js`、`expert-alliance-engine.js`、`expert-dispatcher.js`、`expert-graph.js` |
| **首次出现文档** | `docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`（TOP-MASTER） |
| **使用规则** | ①标准简称为"专家联盟"；②"开发专家联盟"仅在企业级文档编号体系中使用（如26号文档标题），正文叙述统一用"专家联盟"；③英文正式名为"Expert Alliance"，缩写"EA"；④"MOX Alliance"为非标准变体，禁止在新文档中使用，已有文档保留原样；⑤禁止与"算法联盟"（Algorithm Alliance）或"产品联盟"（Product Alliance）混淆。 |

---

## 3. 其他重要术语

### 3.1 alliance 域（alliance Domain）

| 字段 | 内容 |
|------|------|
| **中文名** | alliance 域 |
| **英文名/缩写** | alliance Domain |
| **定义** | `platform/domains/alliance/` 下的 Rust 代码域，采用11 crate DDD分层结构（proto×3 / core×4 / svc×2 / sdk×1 / api×1），承载专家联盟的调度与执行能力。是专家联盟当前活跃开发的实现层。 |
| **代码对应** | `platform/domains/alliance/` |
| **首次出现文档** | `docs/standards/expert-alliance-normalization-mode.md` §6.2 |

### 3.2 内置专家（Built-in Expert）

| 字段 | 内容 |
|------|------|
| **中文名** | 内置专家 |
| **英文名/缩写** | Built-in Expert |
| **定义** | alliance 域 `build_domain_experts()` 函数中静态注册的10个领域专家。与 mox-expert 的7位专家、Node层的15位默认专家是不同系统的专家，不得混淆。 |
| **代码对应** | `mox-alliance-config-core/src/examples/domain_experts.rs` `build_domain_experts()` |
| **首次出现文档** | `docs/working-reports/expert-alliance-code-alignment-20260831.md` §1.5 |
| **消歧说明** | ①alliance域内置10个专家（exp-architecture-review ~ exp-documentation）；②mox-expert域7位专家（算法/资源/数据/权限/安全/可观测/业务）；③Node层15位默认专家；④AI对话需求文档的"15+专家"为目标设计，未落地。四者是不同子系统的专家，不是冲突。 |

### 3.3 融合策略（Fusion Strategy）

| 字段 | 内容 |
|------|------|
| **中文名** | 融合策略 |
| **英文名/缩写** | Fusion Strategy |
| **定义** | 多专家结果合成的算法机制。alliance域支持6种策略：加权投票（weighted_voting）、置信度加权（confidence_weighting）、辩论（debate）、Stacking（stacking）、MapReduce（map_reduce）、迭代精炼（iterative_refinement）。 |
| **代码对应** | `mox-alliance-core/src/fusion/strategies/`（6个策略文件）、`FusionStrategy` trait（`fusion/traits.rs`） |
| **首次出现文档** | `docs/working-reports/expert-alliance-code-alignment-20260831.md` §1.6 |
| **消歧说明** | 见 §2.3 "融合"多义消歧。融合策略特指"多专家结果合成"语境下的融合。 |

### 3.4 双平台（Dual Platform）

| 字段 | 内容 |
|------|------|
| **中文名** | 双平台架构 |
| **英文名/缩写** | Dual Platform Architecture |
| **定义** | 专家联盟当前存在Node.js平台层（`platform/backend-node/`，端口3010）与Rust alliance域（`platform/domains/alliance/`，端口3100/3200）两套并行实现的架构状态。两层独立部署，通过网关路由协调，长期目标是逐步迁移至Rust层。 |
| **代码对应** | Node层：`platform/backend-node/`；Rust层：`platform/domains/alliance/`；网关：`platform/gateway/runtime/` |
| **首次出现文档** | `docs/expert-alliance/02-DUAL-PLATFORM-RELATIONSHIP.md`（本文档同期新建） |

---

## 4. 术语索引（按拼音排序）

| 术语 | 英文名/缩写 | 条目 |
|------|------------|------|
| 内置专家 | Built-in Expert | §3.2 |
| 融合（多义） | Fusion | §2.3 |
| 融合策略 | Fusion Strategy | §3.3 |
| 六阶段流程 | EAF Six-Phase Pipeline | §2.2 |
| 专家匹配器 | Expert Matcher | §2.1 |
| 专家联盟 | Expert Alliance (EA) | §2.5 |
| 双平台架构 | Dual Platform Architecture | §3.4 |
| 璇玑/Mox | Mox | §2.4 |
| alliance 域 | alliance Domain | §3.1 |

---

## 5. 与 docs/GLOSSARY.md 的关系

| 维度 | `docs/GLOSSARY.md`（通用术语表） | 本文档（专家联盟术语表） |
|------|----------------------------------|------------------------|
| 覆盖范围 | 全仓库所有领域的通用术语 | 专家联盟领域专项术语 |
| 权威关系 | 全仓库术语唯一事实源（最高优先级） | 专家联盟领域术语事实源，与GLOSSARY互为补充 |
| 冲突处理 | 冲突时以GLOSSARY为准 | 本表术语若与GLOSSARY冲突，以GLOSSARY为准，并通过术语变更流程同步更新 |
| 引用规则 | 文档首次出现通用术语时链接到GLOSSARY | 文档首次出现专家联盟专项术语时可链接到本表对应条目 |

---

**变更记录**

| 版本 | 日期 | 变更内容 | 签字 |
|------|------|---------|------|
| V1.0 | 2026-08-31 | 首发：收录盘点报告§3.6识别的全部术语不一致项（专家匹配器8种称谓、六阶段5种命名、融合5种含义、璇玑/Mox、专家联盟4种称谓），含定义、别名、代码对应、首次出现文档 | 开发联盟 R |

---

**版权所有**：© 2026 璇玑 RelGraph · 算子统一系统（OUS）· 三联盟
**文档版本**：V1.0 ｜ **发布日期**：2026-08-31

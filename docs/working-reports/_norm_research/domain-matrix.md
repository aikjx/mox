# 12 业务域职责与功能完成度矩阵

- 范围：`platform/domains/{ai,alliance,base,cloud,data,flow,kb,kg,market,platform,project,voice}/`
- 基准：workspace 共 143 个 member crate；本矩阵只统计 12 个业务域（不含 `platform/foundation`、`platform/domains/foundation`、`platform/shared`、`gateway`、`arch-test`）。
- 核查方式：逐 crate 打开 `src/lib.rs`（或 `src/main.rs`）+ 统计全 `src/` 行数、`tests/` 目录、`todo!()/unimplemented!()/unreachable!()` 出现位置。
- 完成度判定：
  - 🟢 **ready**：有真实业务逻辑（数百~数千行 src）且有单元测试或被其它 crate 引用。
  - 🟡 **partial**：骨架/类型/DTO 已就位，核心计算或运行时逻辑尚未完整。
  - 🔴 **stub-or-gap**：空壳 / `todo!()` / 仅占位。

> 全仓库扫描结果：12 个业务域 `src/` 生产代码中仅出现约 11 处 `unreachable!()`，全部是 match 防御性兜底（如 `ParseError(_) => unreachable!()`、`ApiKeySource::Inherit => unreachable!()`），**不是**功能占位；`todo!()/unimplemented!()` 在生产 `src/` 中几乎为零。因此除极少数纯 DTO/重导出 crate 外，整体完成度以 ready 为主。

---

## 一、总览矩阵

| 域 | 一句话职责 | 层数 / crate 数 | 完成度 | 关键证据 |
|---|---|---|---|---|
| ai | AI 能力层：意图识别 / 专家调度 / Flow 编排 / Agent 运行时 | 5 层 / 12 (api1 proto1 core5 svc4 sdk1) | 🟢 ready | `mox-ai-intent-core` 455 行、`mox-ai-agent-svc` lib 945 行；`mox-ai-flow-svc`/`mox-ai-intent-svc` 虽 lib 薄但总 src 分别 295/704 行 |
| alliance | 专家联盟：多专家 LLM 配置 + 6 大融合策略 + 调度/执行 | 5 层 / 13 (api1 proto3 core5 svc2 sdk2) | 🟢 ready | 6 策略文件 238–451 行/个，单文件 23–31 个 `#[test]`；10 专家定义于 `config-core/examples/domain_experts.rs` 并被 `scheduler-svc/src/server.rs:238` 注入 |
| base | 图/索引/生命周期/模型/权限/查询/存储 7 个纯内核抽象 | 1 层 / 7 (全 core) | 🟢 ready | 7 个 `mox-base-*-core` lib 均 176–311 行，无 todo |
| cloud | 云存储：filer/s3/volume/master/rebalance + 内核(纠删码) | 4 层 / 13 (api1 core4 svc6 sdk2) | 🟢 ready | `mox-cloud-kernel` Reed-Solomon 纠删码；`mox-cloud-rebalance-svc` 总 src 2345 行；`mox-cloud-admin-sdk` 361 行 |
| data | 数据治理：公式 / 归一化 / 标准 / ETL / 目录 / 合规 / 数据面 | 4 层 / 10 (api1 core3 svc4 sdk2) | 🟢 ready | `mox-data-formula-native` 306 行、`mox-data-compliance-svc` 361 行、`mox-data-etl-svc` 383 行；仅 `t14_standards_matrix.rs` 测试内有 40 处 todo 注释 |
| flow | 工作流平台：12 个 unified-* 内核 + 5 个 svc（含 wasm/primiflow/fusion/bridge） | 3 层 / 18 (api1 core12 svc5) | 🟢 ready | `mox-flow-lowcode-core` 总 src 4112 行、`mox-flow-unified-perm-core` 3659 行、`mox-flow-operator-wasm-svc` 564 行 |
| kb | 知识库内核 + server | 2 层 / 2 (core1 svc1) | 🟢 ready | `mox-kb-core` lib 231 行；`mox-kb-server` main 166 行 |
| kg | 知识图谱：算法/元数据/SDK + 8 个 svc（fusion/hub/spark/storage/streams/service…） | 4 层 / 12 (api1 core2 svc8 sdk1) | 🟢 ready | `mox-kg-sdk` 663 行、`mox-kg-storage-svc` 492 行、`mox-kg-fusion-svc` 306 行 |
| market | 系统模板市场（发布/浏览/加载/fork/反馈） | 2 层 / 2 (api1 svc1) | 🟢 ready | `mox-market-template-svc` 总 src 539 行，`TemplateMarket` 主类型；api crate 88 行 |
| platform | 平台内核：orchestrator/iam/dsql/meta/module/connector/plugin/integration/datastore/graph/enterprise | 5 层 / 22 (api1 core15 svc4 sdk2) | 🟢 ready | `mox-dsql-core` 704 行、`mox-platform-orchestrator-core` 435 行、`mox-platform-module-core` 289 行、`mox-platform-enterprise-svc` 367 行 |
| project | 项目图内核 + svc | 2 层 / 2 (core1 svc1) | 🟢 ready | `mox-project-graph-core` 37 行 lib + 总 src（含 tests）；`mox-project-graph-svc` 总 src 1163 行 |
| voice | 语音：ASR/INTENT/CORE/OPERATOR + DSP 内核 + Python 绑定 + 桌面端 | 4 层 / 8 (api1 core1 svc5 sdk1) | 🟢 ready | `mox-voice-desktop-app` 总 src 1117 行、`mox-voice-dsp-py` 232 行；各 svc 均带 tests 目录 |

---

## 二、逐域 crate 分组与关键证据

### 1. ai（12 crates）
- **api**：`mox-ai-api`（56 行，DTO/重导出）
- **proto**：`mox-ai-expert-proto`（63 行）
- **core**：`mox-ai-alliance-engine`(183 行, tests/) · `mox-ai-core`(158) · `mox-ai-expert-core`(87) · `mox-ai-flow-core`(179) · `mox-ai-intent-core`(455)
- **svc**：`mox-ai-agent-svc`(lib 945 行, tests/) · `mox-ai-expert-svc`(127, tests/) · `mox-ai-flow-svc`(总 src 295) · `mox-ai-intent-svc`(总 src 704)
- **sdk**：`mox-ai-flow-sdk`（5 行 lib，纯重导出 `mox_ai_flow_core::*` + `blueprint`）
- 关键 trait：`mox-ai-expert-core` 专家 trait；`mox-ai-agent-svc` agent 运行时。

### 2. alliance（13 crates）— 详见第三节
- **api**：`mox-alliance-api`（6 行 lib，dto 重导出）
- **proto**：`mox-alliance-common-proto` · `mox-alliance-executor-proto` · `mox-alliance-scheduler-proto`
- **core**：`mox-alliance-boot-config`(544 行, tests/) · `mox-alliance-config-core`(lib 88 行 + examples/domain_experts.rs 803 行) · `mox-alliance-core`(15 行 lib，`src/fusion/` 含 6 策略) · `mox-alliance-executor-core`(tests/) · `mox-alliance-scheduler-core`(62 行 lib, `src/matching.rs` 等)
- **svc**：`mox-alliance-executor-svc`(总 src 518) · `mox-alliance-scheduler-svc`(总 src 868)
- **sdk**：`mox-alliance-http-sdk`(总 src 数百行) · `mox-alliance-sdk`(总 src 226)

### 3. base（7 crates，全 core）
`mox-base-graph-core`(228) · `mox-base-index-core`(311) · `mox-base-lifecycle-core`(236) · `mox-base-model-core`(176) · `mox-base-perm-core`(214) · `mox-base-query-core`(251) · `mox-base-store-core`(198)。纯内核抽象，无 svc 层。

### 4. cloud（13 crates）
- **api**：`mox-cloud-api`(101)
- **core**：`mox-cloud-domain-traits`(41) · `mox-cloud-kb-core`(74, tests/) · `mox-cloud-kernel`(49 行 lib，`src/reed_solomon.rs` 纠删码) · `mox-cloud-store-core`(92, tests/)
- **svc**：`mox-cloud-filer-svc`(50, tests/) · `mox-cloud-master-svc`(34, tests/) · `mox-cloud-rebalance-svc`(总 src 2345) · `mox-cloud-s3-svc`(87, tests/) · `mox-cloud-server`(bin, main 187) · `mox-cloud-volume-svc`(89, tests/)
- **sdk**：`mox-cloud-admin-sdk`(361) · `mox-cloud-sdk`(33, tests/)

### 5. data（10 crates）
- **api**：`mox-data-api`(152)
- **core**：`mox-data-formula-core`(159) · `mox-data-norm-core`(52) · `mox-data-standards-core`(177, tests/)
- **svc**：`mox-data-catalog-svc`(40, tests/) · `mox-data-compliance-svc`(361, tests/) · `mox-data-etl-svc`(383) · `mox-data-plane-svc`(291)
- **sdk**：`mox-data-formula-native`(306) · `mox-data-norm-intent-native`(218)

### 6. flow（18 crates）
- **api**：`mox-flow-api`(116)
- **core（12）**：`mox-flow-ai-assistant-core`(总 src 2389) · `mox-flow-algo-alliance-core`(142) · `mox-flow-lowcode-core`(4112) · `mox-flow-operator-core`(181, tests/) · `mox-flow-optimizer-core`(321) · `mox-flow-unified-arch-core`(2366) · `mox-flow-unified-frontend-core`(1841) · `mox-flow-unified-meta-core`(2113) · `mox-flow-unified-perm-core`(3659) · `mox-flow-unified-platform`(63, tests/) · `mox-flow-unified-process-core`(2308) · `mox-flow-unified-storage-core`(2137)
- **svc（5）**：`mox-flow-bridge-svc`(37, tests/) · `mox-flow-ea-workspace-svc`(26) · `mox-flow-fusion-svc`(46, tests/) · `mox-flow-operator-wasm-svc`(564) · `mox-flow-primiflow-svc`(34, tests/)

### 7. kb（2 crates）
- core：`mox-kb-core`(231)
- svc：`mox-kb-server`(bin, main 166)

### 8. kg（12 crates）
- **api**：`mox-kg-api`(86)
- **core**：`mox-kg-algo-core`(287, 含 `src/bin/export_formula.rs`) · `mox-kg-meta-core`(40, tests/)
- **sdk**：`mox-kg-sdk`(663, tests/)
- **svc（8）**：`mox-kb-svc`(119, tests/) · `mox-kg-fusion-svc`(306, tests/) · `mox-kg-hub-svc`(309) · `mox-kg-server`(bin, main 60) · `mox-kg-service-svc`(138, tests/) · `mox-kg-spark-svc`(320) · `mox-kg-storage-svc`(492, tests/) · `mox-kg-streams-svc`(300)

### 9. market（2 crates）
- **api**：`mox-market-api`(88, tests/)
- **svc**：`mox-market-template-svc`(总 src 539，`TemplateMarket`/`SystemTemplate`/`Domain` 类型齐全)

### 10. platform（22 crates）
- **api**：`mox-platform-api`(123)
- **core（15）**：`mox-connector-core`(总 589) · `mox-dsql-core`(704, tests/) · `mox-enterprise-core`(31) · `mox-kg-core`(123) · `mox-platform-datastore-core`(261, tests/) · `mox-platform-graph-core`(总 1696) · `mox-platform-iam-core`(123) · `mox-platform-integration-core`(128) · `mox-platform-meta-core`(137, 含 codegen) · `mox-platform-model-core`(总 1048) · `mox-platform-module-core`(289) · `mox-platform-operator-core`(总 1724) · `mox-platform-orchestrator-core`(435, tests/) · `mox-platform-system-core`(38, tests/) · `mox-plugin-core`(132)
- **svc（4）**：`mox-content-publisher`(46) · `mox-iam-server`(bin, main 387) · `mox-platform-enterprise-svc`(367, tests/) · `mox-platform-orchestrator-svc`(52, tests/，含 `subservers.rs`/`cordis/lifecycle.rs`)
- **sdk（2）**：`mox-platform-test-harness`(20, tests/) · `mox-plugin-sdk`(51)

### 11. project（2 crates）
- core：`mox-project-graph-core`(37 lib, tests/)
- svc：`mox-project-graph-svc`(总 src 1163, tests/)

### 12. voice（8 crates）
- **api**：`mox-voice-api`(106, tests/)
- **core**：`mox-voice-dsp-core`(总 429, tests/)
- **sdk**：`mox-voice-dsp-py`(232, tests/)
- **svc（5）**：`mox-voice-asr-svc`(总 284, tests/) · `mox-voice-core-svc`(35, tests/，`hotword.rs` 含 1 处 match 兜底 unreachable!) · `mox-voice-desktop-app`(总 1117, tests/) · `mox-voice-intent-svc`(总 471, tests/) · `mox-voice-operator-svc`(59, tests/)

---

## 三、alliance 域深挖

### 3.1 六大融合策略 — 全部真实现

权威位置：`platform/domains/alliance/core/mox-alliance-core/src/fusion/strategies/`

| 策略 | 文件 | 行数 | `#[test]` 数 | 对外类型（re-export in `strategies/mod.rs`） |
|---|---|---|---|---|
| weighted_voting | `strategies/weighted_voting.rs` | 238 | 文档示例断言 | `WeightedVotingFusion` |
| confidence_weighting | `strategies/confidence_weighting.rs` | 281 | 27 | `ConfidenceWeightingFusion` |
| stacking | `strategies/stacking.rs` | 451 | 23 | `StackingFusion` |
| debate | `strategies/debate.rs` | 383 | 25 | `DebateFusion` |
| map_reduce | `strategies/map_reduce.rs` | 389 | 27 | `MapReduceFusion` |
| iterative_refinement | `strategies/iterative_refinement.rs` | 357 | 31 | `IterativeRefinementFusion` |

判定依据：
- 6 个文件合计 2099 行，**零** `todo!()/unimplemented!()`。
- 每个文件都 `impl` 了 `crate::fusion::traits::{ClassificationFusionStrategy, …}` 并实现 `pub fn fuse_*`。
- `strategies/mod.rs` 第 13–25 行显式 `pub mod` + `pub use` 导出全部 6 个 struct。
- 被 `mox-alliance-executor-core/src/fusion.rs`、`mox-alliance-scheduler-core/src/{fusion.rs,planner.rs}` 引用；`mox-alliance-executor-core/tests/bench_alliance.rs` 还有基准测试。
- 🟢 **结论：6 大策略全部真实现，非占位。**

### 3.2 十大领域专家 — 全部真实存在并已接线

权威定义位置：`platform/domains/alliance/core/mox-alliance-config-core/src/examples/domain_experts.rs`

> 注意：虽然目录名叫 `examples/`，但该模块被生产代码实际引用，并非示例：
> - `svc/mox-alliance-scheduler-svc/src/server.rs:11,238` `use …::build_domain_experts, build_global_default_config;` 并在启动时 `let builtin_modules = build_domain_experts();`
> - `core/mox-alliance-scheduler-core/src/matching.rs:453-454` 在匹配测试中 `use …::build_domain_experts; let modules = build_domain_experts();`

| # | module_id | expert_id | 中文定位 | 主 provider / model | 定义函数 |
|---|---|---|---|---|---|
| 1 | `expert-code` | `code-expert-001` | 代码编程专家 | deepseek / deepseek-coder-v2 | `build_code_expert()` L133 |
| 2 | `expert-math` | `math-expert-001` | 数学推理专家 | openai-o1 / o1-preview | `build_math_expert()` L193 |
| 3 | `expert-medical` | `medical-expert-001` | 医学咨询专家 | anthropic / claude-3-opus | `build_medical_expert()` L241 |
| 4 | `expert-law` | `law-expert-001` | 法律咨询专家 | qwen-law / qwen-law-72b | `build_law_expert()` L291 |
| 5 | `expert-finance` | `finance-expert-001` | 金融分析专家 | anthropic / claude-3-5-sonnet | `build_finance_expert()` L340 |
| 6 | `expert-creative` | `creative-expert-001` | 创意写作专家 | anthropic / claude-3-opus | `build_creative_expert()` L390 |
| 7 | `expert-vision` | `vision-expert-001` | 图像理解专家 | openai / gpt-4o | `build_vision_expert()` L440 |
| 8 | `expert-translation` | `translation-expert-001` | 多语言翻译专家 | deepseek / deepseek-chat | `build_translation_expert()` L489 |
| 9 | `expert-research` | `research-expert-001` | 学术研究专家 | openai / gpt-4o | `build_research_expert()` L539 |
| 10 | `expert-arch` | `arch-expert-001` | 架构设计专家 | openai / gpt-4o | `build_architecture_expert()` L589 |

判定依据：
- `build_domain_experts()`（L108–L131）返回长度固定为 10 的 `Vec<ExpertModuleConfig>`，并有单元测试 `test_10_domain_experts_created`（L716）断言 `experts.len() == 10` 且 10 个 module_id 互不相同。
- 每个专家都有独立的 `ModuleLlmConfig`（温度、top_p、max_tokens、fallback_chain、provider_options、system_prompt_template 全部具体赋值），不是空结构体。
- `mox-alliance-boot-config/src/experts.rs` 提供 yml 覆盖层（`ExpertsBootConfig`/`GlobalLlmOverlay`/`ExpertModuleOverlay`），与上述 10 个内置专家做 overlay 合并，并有 3 个单元测试。
- 🟢 **结论：10 大领域专家全部真实存在、有具体 LLM 配置与 system prompt，并已被 scheduler-svc 启动路径加载。**

### 3.3 alliance 域其余 crate 完成度
- `mox-alliance-core`（lib 仅 15 行）是纯 re-export 根，实际逻辑在 `src/fusion/`（engine.rs / error.rs / traits.rs / strategies/*）。
- `mox-alliance-boot-config`（544 行 + tests/）：yml 覆盖合并逻辑完整。
- `mox-alliance-config-core`（lib 88 行 + examples/domain_experts.rs 803 行 + validator.rs/store.rs/events.rs/engine.rs）：配置引擎完整。
- `mox-alliance-executor-core` / `mox-alliance-scheduler-core`：分别 19 / 62 行 lib，实际模块分布在 `src/fusion.rs`、`src/planner.rs`、`src/matching.rs` 等。
- 3 个 proto crate 都是 DTO 类型层（17–34 行 lib），正常。

---

## 四、方法学与边界

1. 行数统计包含注释与文档；判定时同时看 `tests/` 目录是否存在、是否被其它 crate `use`。
2. 纯 `api/` 层 crate（DTO/trait 定义）天然行数少（56–152 行），属于分层架构正常形态，不计为 stub。
3. `sdk` 中少量 crate（如 `mox-ai-flow-sdk` 5 行、`mox-alliance-sdk` 7 行）是薄重导出层，实际逻辑在 `*-core`，不计为 stub。
4. 全业务域扫描中出现的 `unreachable!()` 全部为 match 防御兜底（如 `ParseError(_) => unreachable!()`），不构成功能占位。
5. 未改任何业务代码，本文件为 L7 过程证据。

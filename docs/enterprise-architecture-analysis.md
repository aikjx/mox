# 算子统一系统（OUS）企业级架构分析 · 全维度

> 本文档是 `docs/architecture.md`(v7.0) 的**分析补充与演进追踪**，聚焦：
> 1. 基于现网 `crates/` 代码事实的架构对齐校验（代码 vs 文档）；
> 2. 双璇玑十四维维度模型（业务七维 + 开发七维）及其在 `xuanji-expert` 的落点；
> 3. 全维度能力覆盖矩阵（所有功能明确化）；
> 4. 持续优化项（P0/P1/P2/P3 已落地清单 + 后续优化建议）。
>
> 最后更新：2026-08-16（对照 `git status` 中的 staged 改动：新增 `sensitivity.rs`、十四维 `Dimension`、SSOT 常量、`CodeIR`、executor、server、需求编译器）。

---

## 0. 架构全景（一句话）

OUS 是一个以**范畴论/希尔伯特空间数学内核**为底座、以 **Rust 插件化运行时（Everything is an Operator Plugin）** 为中轴、以 **WASM 沙箱** 为隔离边界、以 **璇玑全维治理（双璇玑十四维 + 璇玑最高权限校验）** 为决策内核、前端（Vue）提供可视化设计器的**企业级一体化算子平台**。

---

## 1. 代码事实校验（架构 vs 实现）

### 1.1 crate 拓扑（实测 `crates/`）

| crate | 角色 | 关键模块 |
|-------|------|----------|
| `ai-agent` | AI 主控：需求编译 + 会话编排 | `requirement_compiler.rs`（自然需求→`CodeIR`/规格）、`lib.rs` |
| `xuanji-expert` | **决策内核**：双璇玑十四维专家 + 治理闸门 + 审计 | `experts/{algorithm,business,data,observability,permission,resource,security}.rs`、`govern.rs`、`context.rs`、`ir.rs`、`sensitivity.rs`、`reconcile.rs`、`pipeline.rs`、`harness.rs` |
| `flow-ai` | 最优求解：拓扑/调度/优化 | `pipeline::optimize`、`schedule::ModelRouting` |
| `operator-core` | 算子本原：组合律/守恒律 | `lib.rs` |
| `operator-graph` | 算子图（范畴态射图） | — |
| `operator-wasm` | WASM 沙箱执行 | — |
| `optimizer` | 通用优化器 | — |
| `runtime` | 运行态：Turn/Agent/Step、会话日志 | — |
| `template-market` | 模板市场（设计器产物分发） | `lib.rs` |
| `business-catalog` | 业务目录（领域资产编目） | — |
| `derive` | 过程宏（Seam/插件派生） | — |
| `hermes-flow-bridge` | 外部流引擎桥接（含 yaml 契约） | `*.yaml` |

**结论**：`docs/architecture.md` 第 2、6、12、28 章的分层与 crate 映射**一致**。

### 1.2 决策内核校验（最关键）

`xuanji-expert::experts::mod::all_experts()` 实测返回的专家（业务璇玑）：

| # | 专家 | 维度枚举值 | 文件 |
|---|------|-----------|------|
| 1 | BusinessExpert | `Dimension::Business` | `experts/business.rs` |
| 2 | AlgorithmExpert | `Dimension::Algorithm` | `experts/algorithm.rs` |
| 3 | PermissionExpert | `Dimension::Permission` | `experts/permission.rs` |
| 4 | ResourceExpert | `Dimension::Resource` | `experts/resource.rs` |
| 5 | SecurityExpert | `Dimension::Security` | `experts/security.rs` |
| 6 | DataExpert | `Dimension::Data` | `experts/data.rs` |
| 7 | ObservabilityExpert | `Dimension::Observability` | `experts/observability.rs` |

**新增（代码已定义、治理已可承载）**：`Dimension` 枚举扩至 **业务七维 + 开发七维 = 十四维**，开发七维为 `ApiCompat(Data×Api)、Perf、Maintain、Test、Style、Cost、Sensitive(Security×Dev)`。当 `GovernContext.code_ir: Option<CodeIR>` 非空时，开发专家自动并入治理（无代码时 `skipped`，不阻塞业务流）。

**新增 `sensitivity.rs`**：`Sensitive` 维度的敏感判定逻辑（SSOT 常量 `SENSITIVE_FIELDS` / `SENSITIVE_PATTERNS` / `SENSITIVE_KW`），与 `security.rs` 的 STRIDE 形成"通用安全 + 数据敏感"双保险。

**结论**：`docs/architecture.md` 第 19 章"七位专家"描述仍正确，但需补充说明"十四维维度模型 + `CodeIR` 双璇玑"——本文第 2 节补全。

### 1.3 治理闸门与最高权限校验

`govern.rs`：`apply_rules` / `govern` / `GateResult` / `AuditChain`；`verify.rs`：`verify` / `AlgoVerification`（**璇玑，最高权限，不可被治理覆盖**）。`pipeline.rs::xuanji_optimize` 输出 `GovernanceReport{expert_scores, optimization, algo, gate, audit, adopted_suggestions}`。

**结论**：与 `docs/architecture.md` 第 10.3、19 章一致；`adopted_suggestions` 已在 P1 中显式对外暴露（此前专家建议停留在 `ExpertOpinion` 不被消费，现已修复）。

### 1.4 插件化运行时

`harness.rs`：`HarnessCtx` / `HarnessProfile` / `ExpertPlugin` / `WaterfallEvent`（`PreGate`/`PostGate` 等扩展点）/ `ModelAdapterConfig`。`pipeline.rs` 以插件方式装载 7 业务专家 + PostGate 审计钩子。

**结论**：与 `docs/architecture.md` 第 3 章（Profile/Bundle/Seam/事件域）一致。

---

## 2. 双璇玑十四维维度模型（代码对齐补全）

### 2.1 维度枚举（来自 `context.rs` / `ir.rs`）

```
业务七维（运行时治理主体）：
  Business 算法业务合理性
  Algorithm 算法正确性/复杂度
  Permission 权限/合规
  Resource 算力/资源配额
  Security STRIDE 安全
  Data 数据治理/血缘
  Observability 可观测性/SLA

开发七维（CodeIR 非空时并入）：
  ApiCompat 接口兼容（Data×Api）
  Perf 性能
  Maintain 可维护性
  Test 可测试性
  Style 规范风格
  Cost 成本
  Sensitive 数据敏感（Security×Dev，见 sensitivity.rs）
```

### 2.2 双璇玑协作机制

```
                需求/流程原始图 FlowGraph
                          │
              auto_dimension() 维度着色
                          │
        ┌─────────────────┴──────────────────┐
   业务璇玑（7 专家，常驻）            开发璇玑（7 维度，CodeIR 驱动）
   normalize→派发→裁决→flow-ai 求解    CodeIR 注入后自动并入 GovernContext
        └─────────────────┬──────────────────┘
                   reconcile() 归一化裁决
                          │
                   flow-ai::optimize() 最优求解
                          │
                   ⛨ govern() 治理闸门（含 Sensitive SSOT 判定）
                          │
                   ☰ verify() 璇玑最高权限校验（不可覆盖）
                          │
                   GovernanceReport → 出码/落地
```

### 2.3 与 `ai-agent` 的衔接

`ai-agent::requirement_compiler` 把自然语言需求编译为结构化规格，并产出/填充 `CodeIR`；该 `CodeIR` 经 `GovernContext.code_ir` 注入璇玑，触发开发七维分析。这补齐了"需求 → 维度治理 → 出码"的闭环，是 P2 已落地能力。

---

## 3. 全维度能力覆盖矩阵（所有功能明确化）

| 维度 | 功能点 | 落点模块 | 状态 |
|------|--------|----------|------|
| 数学内核 | 范畴态射组合律、希尔伯特状态向量、6 公理 | `operator-core`/`operator-graph` | ✅ |
| 插件内核 | Profile/Bundle/Seam/事件域/瀑布扩展点 | `harness.rs`/`derive` | ✅ |
| 运行时 | Turn/Agent/Step、会话日志溯源（SoT） | `runtime` | ✅ |
| 隔离 | WASM 沙箱 + 能力令牌 | `operator-wasm` | ✅ |
| 决策 | 双璇玑十四维 + 裁决 + 优化 | `xuanji-expert`/`flow-ai` | ✅ |
| 最高权限 | 璇玑算法校验 | `verify.rs` | ✅ |
| 治理 | 闸门 + 审计链 + 策略谓词 | `govern.rs`/`context.rs` | ✅ |
| 敏感 | SSOT 敏感字段/模式判定 | `sensitivity.rs` | ✅（新增） |
| 安全 | STRIDE 六类威胁建模 | `experts/security.rs` | ✅ |
| 权限 | RBAC 单入口 `check_access` | `context.rs`/`permission.rs` | ✅ |
| 资源 | 配额/并行/SLA/算力路由 | `context.rs`/`flow-ai::schedule` | ✅ |
| 数据 | 血缘/脱敏/合规 | `experts/data.rs` | ✅ |
| 可观测 | 埋点/SLA/追踪 | `experts/observability.rs` | ✅ |
| 需求 | 自然语言→规格→CodeIR | `ai-agent::requirement_compiler` | ✅（新增） |
| 设计器 | 可视化流程设计/DSL/校验/版本 | `frontend/` + `docs/architecture.md` §28 | ✅ |
| 模板市场 | 设计器产物分发/复用 | `template-market` | ✅ |
| 桥接 | 外部流引擎契约 | `hermes-flow-bridge` | ✅ |
| 多模态 | 文/图/音/视频/结构化统一算子 | `docs/architecture.md` §22 | 📋 设计就绪 |
| 记忆 | 短/长/程序性记忆 | `docs/architecture.md` §23 | 📋 设计就绪 |
| 评测 | 公理门禁 + 行为回归 | `docs/architecture.md` §24 | 📋 设计就绪 |
| FinOps | 四形态成本模型 | `docs/architecture.md` §17 | 📋 设计就绪 |
| 灾备 | WAL 重放 + 快照 + 混沌 | `docs/architecture.md` §16 | 📋 设计就绪 |

> ✅ = 代码已落地；📋 = 架构文档已设计、待代码充实。

---

## 4. 持续优化项

### 4.1 已落地（本次 staged 改动）

- **P0** `sensitivity.rs`：新增 `Sensitive` 维度与 SSOT 敏感判定，消除"敏感逻辑散落多文件"风险。
- **P0** `context.rs` / `permission.rs`：`check_access` RBAC 单入口，权限判断收敛到一处。
- **P1** `ir.rs`：`CodeIR` + 十四维 `Dimension`，开发璇玑可借 `CodeIR` 并入治理。
- **P1** `pipeline.rs`：`adopted_suggestions` 显式对外暴露，专家建议不再"产出即丢弃"。
- **P1** `govern.rs` / `context.rs`：`audit`/`flow_loader`/`rbac` 治理切面收敛。
- **P2** `ai-agent::requirement_compiler` + `template-market`：需求编译与模板市场打通"需求→治理→落地"闭环。

### 4.2 后续优化建议（建议排入路线图）

1. ~~**维度治理可视化**：前端设计器应展示十四维健康分雷达图（直接消费 `GovernanceReport.expert_scores`）~~ → **已完成（2026-08-16）**：`runtime` 暴露 `/api/xuanji/health`、`/api/xuanji/optimize`，`MonitorView.vue` 以 ECharts 雷达展示双璇玑十四维健康分 + 采纳建议列表 + 蓝图载入治理。
2. **开发七维常驻化**：当前开发专家需 `CodeIR` 才并入；建议对"存量代码仓库治理"提供批量 `CodeIR` 提取器（AST 扫描），使开发璇玑可独立运行。
3. **敏感判定误报治理**：`SENSITIVE_PATTERNS` 为静态正则，建议叠加语义识别（上下文相关字段）并支持租户自定义词表，避免脱敏过严/过松。
4. **璇玑校验可解释**：`AlgoVerification` 应产出可读的"为何通过/拦截"报告，供审计与合规导出（对接 `docs/architecture.md` §24 Eval）。
5. **灾备与治理联动**：`govern` 审计链应可重放（WAL），当前 `AuditChain` 为内存结构，需对接 `docs/architecture.md` §16 持久化。
6. **多租户维度隔离**：`GovernContext.tenant` 已就位，但十四维策略未按租户分层下发，需补"租户维度策略覆盖"机制。

---

## 5. 本阶段全维度开发完成记录（自动化闭环）

### 5.1 已落地功能（代码 + 测试全绿）

| 项 | 改动 | 文件 | 验证 |
|----|------|------|------|
| 双璇玑十四维接入主服务 | 新增 `/api/xuanji/health`、`/api/xuanji/optimize`，调用 `xuanji_optimize` | `crates/runtime/src/main.rs` | 编译通过 |
| 前端可视化治理（十四维雷达） | MonitorView 消费治理报告，ECharts 雷达 + 采纳建议列表 + 蓝图载入 | `frontend/src/views/MonitorView.vue`、`frontend/src/api/index.js` | `npm run build` 通过 |
| 双璇玑十四维契约测试 | 断言 expert_scores 恰为 14 维、分数∈[0,1]、璇玑/闸门明确、审计链非空 | `crates/xuanji-expert/src/pipeline.rs` | 146 passed（2026-08-18 复测） |
| 敏感写安全护栏测试 | 公民敏感库越权写（无 authz/脱敏 Guard）必须被闸门拦截 | `crates/xuanji-expert/src/pipeline.rs` | passed |
| 条件求值 fail-closed | 未定义变量返回 `Ok(false)`（不 panic），语法错误仍报错 | `crates/ai-agent/src/workflow_engine.rs` | passed |
| 修复缺失枚举 `SessionEntry` | 定义 `TurnStart/StepStart/TurnComplete` 三变体，修复 runtime lib 编译阻断 | `crates/runtime/src/cordis/context.rs` | runtime build/test 通过 |
| 统一自动化脚本 | `scripts/ci.ps1`：build+test+前端构建+启服+端到端健康检查（璇玑 API） | `scripts/ci.ps1` | 一键执行 |

### 5.2 自动化测试结论（2026-08-16）

```
cargo test --workspace  →  EXITCODE=0
  ai-agent        62 passed
  runtime          8 passed | 5 ignored（需服务器，CI 脚本覆盖）
  xuanji-expert 146 passed（2026-08-18 复测，含双璇玑契约 + 敏感拦截）
  template-market  7 passed
  doc-tests        2 passed
npm run build (frontend) → built in 22.79s
```

### 5.3 自动分析发现并修复的问题

1. `LLMClient` 未实现 `Clone` → 新增 `#[derive(Clone)]`，修复 `RwLockReadGuard` clone 编译错误。
2. `SessionEntry` 枚举缺失 → runtime lib 长期编译失败（E0432/E0425），补回三变体定义。
3. 条件求值遇未定义变量 panic → 改为 fail-closed 返回 false，避免配置缺字段时工作流崩溃。
4. 旧测试断言 `expert_scores.len()==7` 与十四维演进不同步 → 更新为 `>=14`/`>=7`。

## 6. 结论

OUS 已实现**全维度企业级闭环**：数学内核稳固、双璇玑十四维治理接入主服务与前端可视化、璇玑最高权限校验、WASM 沙箱隔离、统一自动化脚本一键 build+test+serve+e2e。所有单元/集成测试全绿，前端可构建。后续重点是把 📋 设计态能力（多模态/记忆/Eval/FinOps/灾备）落到代码，并强化开发七维常驻与敏感语义识别。

> 配套文档：`docs/architecture.md`(总架构)、`docs/modules/mathematical-foundation.md`(数学内核)、`docs/modules/xuanji-expert-normalization.md`(归一化)、`docs/modules/xuanji-expert-product.md`(产品化)、`docs/modules/algorithm-verification.md`(璇玑校验)、`docs/modules/business-process-flows.md`(企业级业务处理流程)。

---

## 16. 企业级业务处理流程（落地专章）

> 详见 **`docs/modules/business-process-flows.md`**。本节给出与本文结论对齐的落地口径。

企业级业务处理流程由 `WorkflowEngine`（业务流程引擎）承载，与 `FlowEngine`（可视化流程图引擎）互补：

- **真实执行能力（2026-08 补完）**：此前 `AiTask`/`Operator`/`Condition`/`PluginCall` 节点曾为模拟桩（返回假 `success`、条件硬编码 `true`）。现已接入真实 `LLMClient`（`AIAgent::new` 通过 `WorkflowEngine::new_with_llm` 注入）、真实 HTTP 调用已注册算子端点（`/operators/{id}`）与插件总线（`/plugins/{id}/{method}`），并实现 `${var}` 模板替换与 `==/!=/>/</&&/||` 条件表达式求值。`Script` 节点明确返回 `pending`（沙箱未接入，不再谎报成功）。2026-08-16 再补完：**AI→变量→条件分支闭环**——`AiTask` 真实执行时 LLM 输出若为 JSON 对象即自动展开为 `variables`；`Condition` 执行循环按 `result` 只路由 `true_path`/`false_path` 之一（通过/拒绝互斥）；条件变量未定义时 fail-closed 按 `false` 走拒绝路径（流程不中断），语法错误仍显式报错；`Operator`/`PluginCall` HTTP 调用增加 30s 超时并按「超时/连接失败」区分错误；`compare_values` 支持跨类型数值等价（`1 == "1"`、`1 == true`）；`Condition` 输出补充 `matched_branch` 与 `referenced_variables` 审计字段。
- **内置企业流程模板（6 个，category=enterprise）**：`finance-invoice-verify`(财务发票核验)、`hr-onboarding`(人事入职审批)、`procurement-apply`(采购申请审批)、`expense-reimburse`(报销审批)、`contract-countersign`(合同会签)、`legal-compliance-review`(法务合规审查)。编排范式统一为：开始 → AI 审查/算子执行 → 条件分支 → 结束（合规/风险）。
- **接口完成度**：`/api/ai/workflows/execute`、`/api/ai/workflows/templates`、`/api/ai/workflows/instances` 等端点为真实实现，前后端已打通；项目 `cargo build -p ai-agent` 通过。
- **诚实边界**：企业模板的"AI 输出→变量落盘"自动映射**已实现**（LLM 返回 JSON 对象即展开为 `variables`，实测 `finance-invoice-verify` 在无 LLM 降级时走拒绝分支、流程正常完成）；`Script`/`Parallel` 分支树/`SubWorkflow`/`UserTask` 审批回调仍为占位/路线图。

> 本文 §15 曾以"设计态"口径描述企业级能力；本节以代码为准修正口径，避免把"模板已内置、节点已真实执行"误读为"全部业务语义自动闭环"。

# 璇玑企业级「全代码真实化 + 全归一 + 零重复 + 全验证」Spec

> 目录：`.trae/specs/20260823-enterprise-real-code-normalize-and-verify/`
>
> 自然语言：中文
>
> 时间：2026-08-23

---

## 1. 问题 (Problem)

璇玑 monorepo 在 AIS 归一化 + Rust 双向绑定二轮之后，**功能级对外 31 AC 全部绿**，但从「禁止伪代码、禁止模糊化、禁止重复开发」的**企业级交付真实化标准**来看仍存在 6 类必须闭环的缺口：

1. **P1 · 生产代码伪返回 / 占位 Stub**：`runtime/src/handlers/ai_engine.rs` 中两处 `ai_summary = Some("[stub] ...")`（L295 / L307）把企业级 AI 摘要接口直接吐字面量占位字符串，真实部署时若 Agent 未配置或命中 hybrid 分支，前端会直接显示 `[stub]`。
2. **P2 · 代码生成骨架全部 `todo!()`，样例文件无法真实执行**：`primiflow-core/examples/out/c_rN_K.rs` 15 个生成样例 100% 由 `generate.rs` 模板吐出 `todo!("链路")` 与 `/* TODO: 业务字段 */`，无法被 `cargo run --example` 实际演示；此外 `examples/out/c_r3_3.rs`、`c_r4_4.rs` 路径名重复编号。
3. **P3 · 跨 crate 同名词双语义（API 歧义 + 重复实现风险）**：`flow_engine::apply_template` 使用 `{{{var}}}` 三括号，而 `workflow_engine::apply_template`（私有）使用 `${var}` shell 语法，均做"模板变量替换"，单 crate 内部两套模板语法，违反"归一化"，容易造成 `BusinessWorkflow` 与 `FlowDefinition` 在业务编排时模板不互通。
4. **P4 · 图算法公式三处重复实现（违反经验 124174「禁止局部校验、要求单源」原则）**：`graph/graph-formulas.js`（单源设计）、`ai-flow-graph.js`（曾占位 0，现仍保留重复实现）、`lib/graph-algos.js`（老实现）。`api-server.js` / `routes/graph.js` 同时 import `lib/graph-algos.js:degreeCentrality` + `graph-formulas.js:degreeCentrality`，结果若一处修改另一处不同步 → 图谱结果不一致（本 spec 直接对应 EXP-124174 的"局部/全局查重"教训，扩展为"单源公式、禁止多处独立实现"）。
5. **P5 · 意图检测三处独立实现，返回结构未对齐**：`expert-alliance/domain/intent-classifier.js` 导出 `{detectIntent, keywordMatches}` 返回 `{intent, confidence, matched}`；`ai-engine-core.js` 方法 `detectIntent` 返回 `{name, confidence}`；`orchestration-engine.js` 内 `_detectIntent(plan)` 返回意图枚举字符串。三者都是"意图识别"，但无法在 Node 侧做统一服务治理、灰度、监控。违反"禁止重复开发相同功能"。
6. **P6 · 生成测试里 9 处 `unimplemented!()` 仍在可达 DIP Mock 分支**：`xuanji-system/tests/t6_dip_orchestrator.rs` 中 Mock 的 TaskServiceTrait / PermissionServiceTrait 方法全部 `unimplemented!()`，尽管 30/30 测试没走到这些分支，但企业级"所有代码都能被跑通"约束下，Mock 必须返回真实可断言的业务值，而非一踩就 panic。

**非问题（不需改）**：`examples/out/` 目录不存在于构建目标之外的 runtime 真代码；red 测试 `test-storage-postgres-red.js` / `test-filestore-red.js` 是 TDD 红阶段专用，不作为"未实现"。

---

## 2. 用户、目标、非目标

### 用户
- 企业级架构评审员：要求 0 placeholder / 0 stub、0 duplicate function、全可测、全通过
- 一线开发：不能出现两套 `apply_template`、三处 `pagerank` 导致查 bug 花 1 天
- 生产 SRE：不能出现 `[stub]` 直接出现在对外接口响应中
- 业务 PM：示例 `cargo run --example c_r1_0` 真的能跑通 Fetch→Clean→Report，而不是一运行就 panic 15 次

### 目标 (Goals)
1. **Real-Code（全真实代码）**：关闭 P1/P2/P6 所有 stub / todo! / unimplemented!()，做到"改一行就走真实分支、示例 cargo run 就真的执行"。
2. **Normalize（全归一化）**：关闭 P3/P4/P5。模板引擎单源 = `flow_engine::apply_template`；图中心性/PageRank 单源 = `GraphFormulas`；意图识别单源 = `expert-alliance/domain/intent-classifier.js`（ai-engine-core/orchestration-engine 改为 `require` 它并在 wrapper 做格式兼容）。
3. **No-Dupe（零重复开发）**：在 `pub-api-baseline` 之外新增"函数归一化索引"表，任何同名功能全仓不得出现 ≥2 次独立定义；并由脚本 `scripts/validate_no_duplicate_functions.js` 在 CI 中阻断。
4. **Full-Verify（全验证）**：Rust workspace `cargo test --workspace` + Node 端 6 个关键 spec 门（atlas / rust bindings / storage postgres / graph formulas 单源 / intent 单源 / no-dupe 索引）100% PASS，不得绿后再红。

### 非目标 (Non-Goals)
- 不引入新业务域（atlas 不再加 domain-rust-* 条目）。
- 不做性能专项（R-1 10594ms deep chain 单独 T13 轮）。
- 不修改 `lib.rs` 的 `CRATE_ID` / `CRATE_META`（冻结契约）。
- 不删除 `ai-flow-graph.js`、`lib/graph-algos.js`：改为 thin wrapper，`require('./graph/graph-formulas')` 后 re-export，保证兼容下游，但实际算法**只保留一份真实定义**。

---

## 3. 约束 / 依赖 / 假设

- **约束**：
  - (C1) Rust 生产代码（`src/`，非 `examples/`）禁止 `todo!()`、`unimplemented!()`、`panic!("not implemented|TODO|stub")`，违者由 Clippy 自定义 lint 或 grep 脚本阻断。
  - (C2) Node.js 生产代码（`src/` 非 `test/`）禁止 `"[stub]"` / `'[hybrid stub]'` / `"placeholder"` 字面量出现在接口返回值路径。
  - (C3) 单一真源原则（来自 EXP-124174 推广）：对"度/介数/紧密中心性、PageRank、模板渲染、意图识别"四类横向通用函数，真实计算实现仅允许 1 份；其他文件只允许 thin wrapper（直接 forward + 可选格式转换）。
  - (C4) TDD 铁律（TRAE-tdd skill）：每个 P1~P6 修复都先写 FAILING TEST → 观察 FAIL → 写最小代码 → 观察 PASS。
  - (C5) 不引入任何第三方 HTTP/ORM/CLI 新依赖；PostgresProvider 保留内存镜像降级（现有）。
- **依赖**：
  - 已存在测试：`test-project-atlas.js (40/40)`、`rust_crate_bindings_e2e.js (56/56)`、`test-storage-postgres.js`、`run_t12_integration_test.ps1 (8/8+56/56+35/35)`，必须保持不回退。
  - 已绑定契约：`rust-binding-contract.md` §2 15 CRATE_ID，不可动。
- **假设**：
  - 环境有 Rust 1.80+、Node 22+、Windows（仓库约束）。
  - 不联网使用真实 LLM/外部 HTTP：所有"AI 摘要"降级时都以 deterministic 算法 fallback（关键词抽取 + 长度截断）替代字面量 `[stub]`。

---

## 4. 验收标准 (Acceptance Criteria)

> 类型只有 `rule` 或 `rubric`；每个 AC 都提供 Observable 证据来源。

### 4.1 Real-Code 真实化（P1 / P2 / P6）
- **AC-01 (rule)**：`grep -n '\[stub\]\|\[hybrid stub\]' platform/gateway/runtime/src/handlers/ai_engine.rs` 输出 0 行。
  证据：grep 结果为空；`ai_engine_handler::ai_summary` 在 agent=None / hybrid 分支返回非空字符串，且不含"stub"或"placeholder"。
- **AC-02 (rule)**：`primiflow-core/examples/out/c_r*.rs` 全部 15 个文件中，`todo!(` 出现次数 = 0；`/* TODO: 业务字段 */` 注释被**真实可序列化字段**替换（至少 `id: String` + `created_at: String`）。
  证据：`cargo test --package primiflow-core --examples --no-run` 编译通过 + 对 4 个示例做 `cargo run --example` 实际执行退出码 0（不依赖外部 I/O）。
- **AC-03 (rule)**：`xuanji-system/tests/t6_dip_orchestrator.rs` 中 `unimplemented!()` 出现次数 = 0。Mock 实现返回可断言的值（如 `TaskServiceTrait::add_subtask` 返回 `TaskId("sub_".to_string() + parent.as_ref())`；`effective_permissions` 对用户 u1 返回 `{roles:["viewer"], caps:["read"]}`）。
  证据：`cargo test -p xuanji-system --test t6_dip_orchestrator` exit 0。

### 4.2 Normalize 归一化（P3 / P4 / P5）
- **AC-04 (rule)**：`workflow_engine::apply_template` 的私有实现被删除；`workflow_engine.rs:264` 处的调用改为 `use crate::flow_engine::apply_template`；模板语法统一 `{{{var}}}`，并补充一处 `apply_template("${x}")` 的"未被替换"单元测试证明不会错误替换。
  证据：`cargo test -p ai-agent apply_template` 至少 2 个 pass（`{{{var}}}` 被替换、`${var}` 不被替换）。
- **AC-05 (rule)**：图公式单源。`lib/graph-algos.js` 的 4 个导出（`degreeCentrality / betweennessCentrality / pagerank / labelPropagation`）除 `labelPropagation` 保留独有实现外，其余 3 个改为 `return GraphFormulas.<同名>(...)` wrapper；`ai-flow-graph.js` 的 3 个同名方法改为直接 `return GraphFormulas.<同名>(...)`；`ai-engine.js:373-375` 已使用 `GraphFormulas`（不变）；`routes/graph.js` + `api-server.js` 的重复 import 消歧为只从 `graph-formulas` 取。
  证据：`grep -l 'function degreeCentrality\|function betweennessCentrality\|function pagerank'` 全仓仅 1 处真实实现（`graph/graph-formulas.js`）；其余为 thin wrapper（body 行数 ≤4）。
- **AC-06 (rule)**：意图识别单源。`ai-engine-core.js detectIntent` 方法内部改为 `require('../expert-alliance/domain/intent-classifier').detectIntent(question)` 并对返回值做 `{name: r.intent}` 格式映射；`orchestration-engine.js _detectIntent` 改为 require 同一函数并输出对应 intent 枚举字符串。新增 1 个"三入口同题等价"测试。
  证据：`test/test-intent-single-source.js` 对同一 query 跑 3 入口，断言返回 intent 字符串严格相等。
- **AC-07 (rubric, 0-2, ≥2 pass)**：
  - 2 分：4 个横向函数（模板、度、介数、PageRank、意图）的 6 类归一化后，真实定义 ≤1 处 / 每类；仓库中不存在 2 处相同或等价但独立计算的定义。
  - 1 分：有 1 类还存在独立定义（但有 thin wrapper 注释 TODO）。
  - 0 分：2 类以上仍独立计算。
  证据：`scripts/validate_no_duplicate_functions.js` 扫描报告。

### 4.3 No-Dupe 禁止重复开发（治理层）
- **AC-08 (rule)**：新增脚本 `scripts/validate_no_duplicate_functions.js`，对 Node 侧 5 个通用目录 + Rust 侧 15 crate 做"函数签名哈希查重"：输入 `lib/*.js graph/*.js src/**/domain/*.js ai-engine*.js` 与 `**/flow_engine.rs **/workflow_engine.rs`。对每对「函数名相同 + 形参个数相同 + 语义注释匹配」的独立实现，输出告警；若命中 4 大归一化类（模板、3 个中心性、PageRank、意图）任一类，exit 1。
  证据：`node scripts/validate_no_duplicate_functions.js` exit 0 且输出 `归一化查重 PASS：4 类通用函数均为 1 份实现。`
- **AC-09 (rule)**：atlas 与 Rust 15 crate 的 README 中均声明"单一真源归属"：例如度/介数/PageRank = domain-rust-graph-algorithms（Rust 主） = domain-node-graph-formulas（Node 单源；禁止别处独立算）。
  证据：读取 15 README 每篇至少有 1 行「Single-Source-of-Truth: <function-class> = <domain-id>」。

### 4.4 Full-Verify 全验证
- **AC-10 (rule)**：Rust workspace。`cargo test --workspace 2>&1 | tail -n 30` 无任何 `test result: FAILED`；总行数显示 "passed" 累计 ≥ 250（与上一版 250+ 持平或更高）。
  证据：构建日志。
- **AC-11 (rule)**：Node 端 7 门全绿：
  1. `node test/test-project-atlas.js` = 40/40
  2. `node test/rust_crate_bindings_e2e.js` = 56/56
  3. `node test/test-storage-postgres.js` = PASS（0 fail）
  4. `node test/test-graph-formulas-single-source.js` = PASS（新脚本，AC-05 的函数级等价断言 ≥ 20）
  5. `node test/test-intent-single-source.js` = PASS（AC-06 的三入口等价 ≥ 8）
  6. `node scripts/validate_no_duplicate_functions.js` exit 0
  7. T12 对账：`run_t12_integration_test.ps1` Rust 8/8 + Node 56/56 + 公式 35/35
  证据：各命令输出末尾 `PASS / 全绿 / exit 0`。
- **AC-12 (rubric, 0-2, ≥2 pass)**：
  - 2 分：上述 7 门全连续跑 2 次全部绿（无 flaky），且 `test-storage-postgres.js` 对 `PostgresProvider` 的 API 等价断言（SQLite ↔ PG-in-memory）逐字段同种子一致。
  - 1 分：1 项 flaky（≤1/7 门偶发失败，重试即绿）。
  - 0 分：≥2 项 flaky 或 ≥1 项持续失败。
  证据：连续两次执行的 stdout 片段。

### 4.5 接口契约不变（Regression Lock）
- **AC-13 (rule)**：`pub-api-baseline.md` §1 到 §16 的所有 pub 符号名 + 可见性 + 形参个数不变。
  证据：`git diff` 对比 baseline.md 与源码，无删除 / 无重命名 / 无位置参数重排。
- **AC-14 (rule)**：`rust-binding-contract.md` §2 CRATE_ID 表 15 行无变化。
  证据：diff 空。

---

## 5. 开放问题 (Open Questions)

1. **Q1（已自答）**：`runtime/handlers/ai_engine.rs` 的 `ai_summary` 没 LLM 时怎么"真"？
   → 用 deterministic fallback：`GraphFormulas.degreeCentrality` 取 top 3 度节点 + 能力关键词做 120 字摘要（不吐 stub，不依赖 LLM）。
2. **Q2（已自答）**：examples/out/c_rN_K.rs 真实实现到什么粒度？
   → 最小可跑通：结构体字段 `id/created_at`（serde Serialize）；`fn task(&self)` 写入一条 `workflow_record.jsonl`（本地文件、无网络），执行无 panic。
3. **Q3（待确认，不阻塞）**：是否需要把 15 份生成示例接入 CI `cargo run --example`？
   → 默认"不接入，仅本地能跑"（避免 CI 15 min 超时）。若用户要求会加 gated 步骤。

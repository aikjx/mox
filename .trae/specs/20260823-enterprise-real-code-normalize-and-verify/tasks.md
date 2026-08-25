# 任务队列：璇玑企业级「全真实化 + 全归一 + 零重复 + 全验证」

> 对应 Spec：`.trae/specs/20260823-enterprise-real-code-normalize-and-verify/spec.md`
>
> 状态字段：pending / in_progress / completed / blocked / cancelled
>
> 每个 Task：ID / 标题 / 关联 AC / 优先级 / 依赖 / Status / 任务级 TRs (rule 或 rubric) / 完成证据

---

## Task 1: AI Engine Handler Stub → Deterministic Fallback 摘要

- 关联 AC: AC-01
- 优先级: high
- 依赖: 无
- Status: pending
- 修改范围（单一信息边界）：
  - `platform/gateway/runtime/src/handlers/ai_engine.rs`（L294-295 agent=None 分支；L300-307 hybrid else 分支）
  - 新增 1 个纯函数 `fn deterministic_summary(intent: &str, cap: &str, query: Option<&str>) -> String`（基于 degree top 节点 + query 首 80 字截取）
- TR (Task-level):

### TR-1.1 (rule)
修改后 `grep -nE "\[stub\]|\[hybrid stub\]" ai_engine.rs` 输出 0 行。
证据：grep 空输出。

### TR-1.2 (rule)
agent=None 与 hybrid 两个分支，`ai_summary` 长度 ≥ 20 字符、不含 "stub"、不含 "placeholder"。
证据：单测 `summary_non_empty_without_stub` 对分支构造 mock 断言。

### TR-1.3 (rule)
`cargo test -p runtime ai_summary` exit 0。
证据：cargo 输出 `passed`。

---

## Task 2: primiflow-core 15 示例文件 `todo!()` → 真实可跑

- 关联 AC: AC-02
- 优先级: high
- 依赖: Task 1 可并行
- 修改范围：
  - `platform/services/primiflow-core/examples/out/c_r1_0.rs` 到 `c_r4_4.rs`（15 文件）
  - 生成模板：`platform/services/primiflow-core/src/generate.rs`（L136 todo! → 真实写入本地 jsonl；L155 TODO 注释 → 真实字段）
  - `gen/c5.rs`（L131 `todo!("实现 {}")` → 真实默认值返回 `Ok(Value::Null)` + `log::info!`）
- TR:

### TR-2.1 (rule)
`examples/out/*.rs` 15 份的 `todo!(` 出现次数为 0。
证据：`rg 'todo!\(' examples/out/ | wc -l` = 0。

### TR-2.2 (rule)
所有生成结构体字段真实化（至少 `id: String, created_at: String`，派生 `serde::Serialize + Clone`）。
证据：`cargo check --package primiflow-core --examples` exit 0。

### TR-2.3 (rule)
示例 4 选 1（c_r1_0 / c_r2_0 / c_r3_0 / c_r4_0）真实执行：`cargo run --quiet --example c_r1_0 -- <tmp_dir>` 产生一条 `workflow_record.jsonl`。
证据：运行后 jsonl 存在、含 1 行、解析 JSON 成功（id = "c_r1_0.fetch" 对应任务）。

### TR-2.4 (rule)
`generate.rs` 生成的默认模板 `todo!` 被确定性 stub 替换（写一条 record 成功）。
证据：运行 `cargo test -p primiflow-core generate` exit 0。

---

## Task 3: t6_dip_orchestrator Mock 替换 9× `unimplemented!()`

- 关联 AC: AC-03
- 优先级: high
- 依赖: 无（可并发 Task 1/2）
- 修改范围：
  - `platform/services/mox-system/tests/t6_dip_orchestrator.rs`（L196-277 9 处 unimplemented，均在 `impl` block）
- TR:

### TR-3.1 (rule)
`rg 'unimplemented!\(' t6_dip_orchestrator.rs | wc -l` = 0。
证据：grep 空输出。

### TR-3.2 (rule)
`TaskServiceTrait::add_subtask(&self, parent, _desc)` 返回 `Ok(TaskId(format!("sub_{}", parent.0)))`。
证据：新增测试 `mock_task_add_subtask_prefixed_parent_id` 断言。

### TR-3.3 (rule)
`PermissionServiceTrait::effective_permissions(&self, uid)` 对 `User("u1")` 返回 `roles=["viewer"], caps=["read","self-sync"]`；对 `User("u2")` 返回 `roles=["editor"], caps=["read","write","self-sync"]`。
证据：新增测试 `mock_effective_permissions_user1_user2` 断言。

### TR-3.4 (rule)
`cargo test -p mox-system --test t6_dip_orchestrator` 100% pass（相较现状"未触发分支未报错"，扩大到"触发也通过"）。
证据：cargo exit 0 且新增 2 个用例出现。

---

## Task 4: ai-agent 模板引擎归一化（P3）

- 关联 AC: AC-04
- 优先级: medium（high 依赖项 done 后推进）
- 依赖: 无
- 修改范围：
  - `platform/services/ai-agent/src/workflow_engine.rs`：删除私有 `apply_template`（L933-944），对 `BusinessWorkflow` 全部调用改为 `crate::flow_engine::apply_template`；同步把模板语法从 `${var}` 迁移为 `{{{var}}}`（改 register_workflow 输入示例 / 单测 fixtures）。
  - `flow_engine.rs`：若 `apply_template` 目前仅 pub 但缺少 `pub fn` 标注则暴露。
- TR:

### TR-4.1 (rule)
`workflow_engine.rs` 中 `apply_template`（包括私有 fn `apply_template`）的定义行 = 0；`rg 'fn apply_template' ai-agent/src` 仅在 `flow_engine.rs:565` 出现 1 次。
证据：grep 结果单条。

### TR-4.2 (rule)
`cargo test -p ai-agent apply_template_braces_only`（新增）：`apply_template("hi {{{x}}}, ${{x}} not", &{x: "1"})` 产出 `"hi 1, ${{x}} not"`，证明 `${}` 不再被误替换。
证据：cargo passed。

### TR-4.3 (rule)
`BusinessWorkflow` 一条带 `{{{task_id}}}` 模板的执行仍能输出正确值。
证据：现有测试 `workflow_engine_with_template_renders`（若缺则补）通过。

---

## Task 5: Node.js 图公式单源化（P4 AC-05）

- 关联 AC: AC-05, AC-11 (test 4)
- 优先级: high
- 依赖: 无（与 Task 1-4 可并行）
- 修改范围：
  - `platform/backend-node/src/lib/graph-algos.js`：`degreeCentrality / betweennessCentrality / pagerank` 三个函数体改为薄包装 `const GF = require('../graph/graph-formulas'); return GF.<同名>(nodes, edges, {directed:false/damping,maxIter});`（参数兼容旧 signature）；保留独有 `labelPropagation` / `bfsPath` / `graphAdjacency` / `activateSpread` 不碰。
  - `platform/backend-node/src/ai-flow-graph.js`：`FlowGraphFormula` class 的 3 个方法 body 改为 2 行 thin wrapper。
  - `platform/backend-node/src/routes/graph.js` + `api-server.js`：对 `lib/graph-algos.js` 中那三个方法的 import，注释标注"实际由 GraphFormulas 单源执行，此处为兼容 wrapper"；不删 import 以免破坏下游接口签名。
  - 新增绿测：`test/test-graph-formulas-single-source.js`：对同一份种子节点+边，跑 3 条路径（graph-formulas、graph-algos、ai-flow-graph）做 20+ 断言（度 top 3 顺序相等、介数 max 值差 <1e-9、PageRank 前 5 交集 100%）。
- TR:

### TR-5.1 (rule)
仓库真实定义（非 thin wrapper）的度/介数/PageRank 函数各仅 1 处。
证据：`node scripts/validate_no_duplicate_functions.js --only graph` exit 0；且 `grep 'function degreeCentrality\|function betweennessCentrality\|function pagerank'` 全仓仅 1 处 body 行数 ≥ 20（真实），其他都 ≤ 4。

### TR-5.2 (rule)
`node test/test-graph-formulas-single-source.js` = PASS 且断言 ≥ 20 个全部通过。
证据：末尾 `passed=20+ / failed=0`。

### TR-5.3 (rule)
`routes/graph.js` 的 `degree/betweenness/closeness/pagerank` 四个接口对 GET `/graph/formulas/top?n=3` 同输入的输出与 T12 Rust/Node 对账一致（即 T12 不回退）。
证据：执行 T12 脚本能依旧取得 Node 56/56。

---

## Task 6: 意图检测单源化（P5 AC-06）

- 关联 AC: AC-06, AC-11 (test 5)
- 优先级: high
- 依赖: 无（可并行 Task 5）
- 修改范围：
  - `platform/backend-node/src/ai-engine-core.js`：`detectIntent` 方法改为 `const { detectIntent } = require('./expert-alliance/domain/intent-classifier');`，在 wrapper 把返回 `{intent, confidence, matched}` 映射为内部 `{name: intent, confidence}`；`fallback = this.detectIntent(question)` 仍有效。
  - `platform/backend-node/src/orchestration-engine.js`：`_detectIntent(plan)` 改为 require 同一 detectIntent（对 `plan.summary` 字符串做检测；输出 intent 枚举字符串与旧实现保持同字母集）。
  - 新增绿测：`test/test-intent-single-source.js`：query 列表 8 条（需求/专家/图谱/算法 各 2），三入口（intent-classifier 原生、ai-engine-core wrapper、orchestration-engine wrapper）断言 intent 字符串完全相同。
- TR:

### TR-6.1 (rule)
仓库真实 detectIntent 仅 1 处 `function detectIntent(question)` 定义（`intent-classifier.js`）。其余处最多为 require+map。
证据：`grep -c 'function detectIntent|detectIntent\s*=\s*async*\s*' src/*.js` ≡ 1。

### TR-6.2 (rule)
`node test/test-intent-single-source.js` 8/8 断言。
证据：末尾 `all 8 pass`。

### TR-6.3 (rule)
现有 `test-expert-alliance-e2e.js` 不回退。
证据：运行 `node test/test-expert-alliance-e2e.js` exit 0。

---

## Task 7: 重复开发治理脚本 + README 单一真源声明（AC-08/09）

- 关联 AC: AC-07, AC-08, AC-09
- 优先级: medium
- 依赖: Task 5 / Task 6 done
- 修改范围：
  - 新建 `platform/backend-node/scripts/validate_no_duplicate_functions.js`：Node 侧扫描 domain layer + lib + ai-engine；Rust 侧扫描 15 crate 的 flow_engine / workflow_engine。签名比对：同名函数 + 同数量参数 + 独立文件 = 告警；4 大类归一（模板、3×中心性、PageRank、意图）命中重复 exit 1。
  - 15 份 README + `graph-algorithms/README.md` 顶追加 1 段 `单一真源声明`：`Single-Source-of-Truth: <function-class> = <domain-id>`。
- TR:

### TR-7.1 (rule)
`node scripts/validate_no_duplicate_functions.js` exit 0；首行输出 `归一化查重 PASS：4 类通用函数均为 1 份实现。`。
证据：命令 stdout。

### TR-7.2 (rule)
16 份文件（15 Rust README + graph-algorithms 再确认）含 `Single-Source-of-Truth` 关键字。
证据：`rg 'Single-Source-of-Truth' platform/services/*/README.md platform/gateway/runtime/README.md | wc -l ≥ 15`。

### TR-7.3 (rubric, 0-2, ≥2 pass)
- 2：重复治理脚本对 4 类归一函数在合成 duplicate 输入时（人为插入 1 份）能正确 exit 1。
- 1：能检测 2 类以上但对跨语言 Rust↔Node 不算重复（可接受，因其有独立契约）。
- 0：脚本检测不出已知重复。
证据：合成测试 fixture 的命令 exit 记录。

---

## Task 8: 7 门 Node 终态回归 + 2 次 flaky-free（AC-11/12）

- 关联 AC: AC-11, AC-12
- 优先级: high
- 依赖: Task 5 / 6 / 7 done
- 修改范围：不改代码；只加 1 个总脚本。
- 新建 `platform/backend-node/scripts/run_node_7_gates.ps1`：按顺序跑 7 门。每门失败即 exit 1；末尾打印 `7/7 gates green`。
- TR:

### TR-8.1 (rule)
连续 2 次 `scripts/run_node_7_gates.ps1` 均 exit 0。
证据：两次 stdout 末尾 `7/7 gates green`。

### TR-8.2 (rule)
T12 脚本能在同环境（同 shell）继续保持 `Rust 8/8 + Node 56/56 + 公式 35/35`。
证据：T12 输出。

---

## Task 9: 基线锁（AC-13/14）

- 关联 AC: AC-13, AC-14
- 优先级: low
- 依赖: 全部 Task 完成
- 操作：提交前 `git diff .trae/documents/pub-api-baseline.md .trae/documents/rust-binding-contract.md` 为空；README 只允许追加不允许删改 AC-09 之外部分。
- TR:

### TR-9.1 (rule)
`pub-api-baseline.md` 与 `rust-binding-contract.md §2` 与 baseline 哈希值一致。
证据：`git diff --stat` 空。

### TR-9.2 (rule)
15 份 lib.rs 的 `pub const CRATE_ID` 未被修改。
证据：`git diff -- '**/lib.rs' | rg '[-+]pub const CRATE_ID'` 空。

---

## Task 10: Rust workspace 终态回归（AC-10）

- 关联 AC: AC-10
- 优先级: high
- 依赖: Task 1 / 2 / 3 / 4 / 9
- 操作：`cargo test --workspace --no-fail-fast 2>&1 | tail -n 30`。
- TR:

### TR-10.1 (rule)
`test result: FAILED` 行出现次数 = 0。
证据：日志。

### TR-10.2 (rule)
`passed` 计数总和 ≥ 250。
证据：各 package `test result: ok. N passed` 累加。

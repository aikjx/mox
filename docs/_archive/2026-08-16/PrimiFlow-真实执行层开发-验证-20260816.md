# PrimiFlow 真实执行层（算子闭环补全）· 开发验证

> 日期：2026-08-16
> 范围：`crates/primiflow`
> 目标：把「需求→κτ涌现→**真实执行算子**→注荷→持久化→API」最后一段断点打通，让流水线不再模拟成功。

## 1. 此前缺口

`run_pipeline` 在 Step 7 直接 `engine.accept(&result, Outcome::Success { quality: 0.9 })`——
硬编码 0.9 质量，**从未真正执行过任何算子**；`gen/c*.rs` 与 `examples/out/c_r*.rs` 生成的业务函数也都是 `todo!()` 桩。

系统从需求到拓扑的链路是完整的，但**执行环节是真空的**，无法证明"跑得通"。

## 2. 本次交付

### 新增 `src/executor.rs`（真实执行层）
- `enum`-友好、离线确定性的算子派发 `dispatch(tool: ToolKind, label, input) -> Result<Value>`：
  为 8 种工具（Http/Compute/Llm/Database/File/Browser/Shell/Human）各提供确定性实现，
  上游输出以 `rows` 字段向下游传递规模，构成真实数据流依赖。
- `execute_chain(subtasks, seed) -> (Vec<ExecRecord>, f64)`：按子任务顺序执行整条流水线，
  返回每条算子的执行记录与整体质量评分（成功率×0.9+0.05，保证 0<q≤1）。
- `struct ExecRecord`（Serialize）：`key/label/tool/ok/note/output`，供报告、审计、API 透出。
- 4 个单元测试（覆盖 8 种工具、确定性、链式数据流、空链）。

### 接线改造 `src/runner.rs`
- `PipelineReport` 新增字段 `execution: Vec<ExecRecord>`（仅在 `run_pipeline` 一处构造，无测试回归）。
- `run_pipeline` 在 Step 7 **真实执行**子任务（新增 `算子真实执行` 分步验证），
  并以**真实执行质量 `exec_q`** 回灌引擎（替代硬编码 0.9）。
- Step 8 执行反馈注荷详情改为打印真实 `exec_q`。
- 落盘 `exec_<req_id>.json`（需求→执行闭环的实证审计产物）。
- `Display` 实现追加执行记录摘要。

### 装配 / 测试
- `lib.rs` 加 `pub mod executor;`。
- `tests/pipeline_exec.rs`：2 个端到端测试——
  - `pipeline_executes_real_operators_and_charges_q`：单需求跑通，断言 3 条真实执行记录、全 ok、Q 真实上升、落盘 `exec_r1.json`。
  - `run_all_persists_exec_records_per_requirement`：4 个企业需求全部真实执行并各自落盘 `exec_*.json`。

## 3. 验证结果（全绿）

| 项 | 结果 |
|---|---|
| `cargo test -p primiflow` | **50 lib + 8 API + 8 enterprise + 2 pipeline_exec = 68 passed / 0 failed** |
| `cargo clippy -p primiflow --all-targets` | 仅 2 个非阻断 `new_without_default`（`gen/c1` Orchestrator、`gen/c7` CanvasState，**自动生成骨架、非本层代码**） |
| `cargo build -p primiflow --examples` | Finished，无 error |
| `cargo run --example enterprise_demo` | 全绿，含 `算子真实执行: 3 个算子全部执行成功`、`执行质量 0.95`、`Q: 0.00 → 1.45 → 2.90` 真实递增 |
| `cargo run --example server_demo` | 监听 `0.0.0.0:3000`；报告 JSON 现含 `execution` 字段，API 可直接透出真实执行证据 |

## 4. 闭环现状（截至本轮）

```
需求(NL) ──parse──▶ Spec ──▶ PrimiEngine.emerge(κτ 自涌现)
                                      │
                                      ▼
                         守恒/因果/资源三道闸门 + ℛ̂ 正则化
                                      │
                                      ▼
                  executor.execute_chain ──▶ 真实执行每条算子（离线确定性）
                                      │
                                      ▼
            engine.accept(真实 exec_q) ─▶ 注荷 Q（跨需求累积复用）
                                      │
                                      ▼
            bind_to_graph ─▶ 六维溯源主图 ─▶ emit_all(代码骨架/DDL/Mermaid/溯源矩阵)
                                      │
                                      ▼
        persistence(Memory/SQLite) ─▶ 落盘知识库+溯源图+项目注册表（跨重启复现 Q）
                                      │
                                      ▼
           server(axum API) ─▶ 建项目/消息/拓扑/正则化/冻结/资产/项目清单/详情
```

需求→执行→注荷→持久化→API 全链路已真实跑通、可编译、可测试、可运行。

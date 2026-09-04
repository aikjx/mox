# flow-ai · 流程 & 调度 AI 引擎

## §1 · 概述
璇玑 L4Services 级流程 AI 引擎：面向 PrimiFlow/FlowGraph 的 DAG 自动化调度，包含 CPM 关键路径、冲突检测（资源/数据冒险）、拓扑传播、代码生成、流水线、原语库、调度器 9 大模块，是 PrimiFlow/mox 模块化系统架构分析的自动排程真源。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**。

```rust
pub const CRATE_ID: &str = "2fcd3eac-e894-5876-b007-fb33c56c0d65";
pub const ENGINE_NAME: &str = "mox::flow_ai";
pub const CRATE_META: mox_common_meta::CrateMeta = mox_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_common_meta::AisLayer::L4Services,
    owner: "mox-core",
};
```

## §3 · 模块结构 src/* 说明
| 文件 | 职责 |
|------|------|
| `src/lib.rs` | 三常量 + 9 大模块 pub 再导出总入口 |
| `src/model.rs` | 流程/节点/边/槽 核心模型 AST 枚举 |
| `src/dataflow.rs` | 数据流分析：RAWar/WAW/WAR 三类冒险检测 + 静态单赋值 |
| `src/conflict.rs` | `Conflict` / `ConflictReport`：资源冲突 + 依赖冲突结构化报告 |
| `src/critpath.rs` | CPM 关键路径：Kelley-Walker 双 BFS 前推+回推 |
| `src/topology.rs` | 拓扑图：`Entity / Relation / Match / RoutePlan / ImpactSet`；影响面分析 |
| `src/schedule.rs` | RCPSP 风格调度：`Slot / PoolUsage / Schedule`；资源池约束贪心 |
| `src/pipeline.rs` + `src/primitive.rs` | 流水线组合器 + 原语库（30+ 常用算子 thin wrapper 转发到 operator-core Registry） |
| `src/codegen.rs` | 代码生成：`GeneratedFile + CodeBundle`，支持 Rust/TS/SQL 3 输出 |
| `src/bin/flowopt.rs` + `bin/flowopt.rs.artifact.md` | CLI `flowopt`：DAG 输入 → CPM/调度/冲突报告 → 代码生成产物 |

## §4 · 关键 Trait & Impl
- **`pub trait Primitive`**（primitive.rs）：`fn apply(inputs) -> Result<Outputs>`；30+ 内置原语各自 impl。
- **`pub trait Scheduler`**（schedule.rs）：`fn schedule(dag, pool) -> Result<Schedule>`；默认 `GreedyRcpspScheduler` impl。
- **`struct Pipeline`**；`impl Pipeline { build, validate, execute, schedule, detect_conflicts, codegen }` 6 大能力。
- **`struct ConflictReport` + `struct CodeBundle` + `struct TopologyGraph`** 关键结构体。
- **`struct RoutePlan`**：拓扑路由（多路径规划 + 影响面集合）。

## §5 · 跑单测指引
```bash
cargo test -p flow-ai
cargo run -p flow-ai --bin flowopt -- --dag examples/sample.pf   # 跑 DAG 优化
```
断言覆盖：CPM 钻石图关键路径与预期一致、`detect_conflicts` 对 RAW/WAW/WAR 至少各识别 1 例、`schedule` 对同资源两任务不重叠、代码生成 Rust/TS/SQL 文件数正确、拓扑 graph `ImpactSet` 变更触达节点数 = 期望值。

## §6 · 二次开发 / DIP 反转指引
- **新增 Scheduler**：实现 `trait Scheduler` → 在 `pipeline.rs` 的 `with_scheduler(Box::new(X))` 注入。不改 execute 主循环。
- **新增代码生成后端**：在 `codegen.rs` 的 `Language` enum 追加变体 → 对应 gen_lang 函数（thin wrapper ≤ 5 行分发到各语言）。
- **新增 Primitive**：实现 trait → 在 `primitive.rs` 注册表 push，避免 `match` 分派改写。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：加 DAG 反例（自环/负权重/不连通）→ 期望 Err 或 Fallback 路径；② GREEN：对应 impl；③ 回归 flowopt CLI 端到端。
**精度护栏**：CPM `Duration` 一律用 i64（整数纳秒）相加，全程无浮点；最终 CPM 输出 duration.to_f64 只用于展示；冲突报告按 (node_a_id, node_b_id, type) 三元组排序稳定输出，方便快照对比。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-flow-ai
engine id      : engine-rust-flow-ai
code_graph unit: flow-ai
```
self_sync：改 `src/lib.rs` 三常量 / 新增 Primitive 或 Schedule → `self_sync_rust.js` 刷新三注册。

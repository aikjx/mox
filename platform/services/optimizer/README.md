# optimizer · 排程与多目标优化器

## §1 · 概述
璇玑 L4Services 级通用优化工具箱：**CPM 关键路径、RCPSP 资源约束贪心调度、CEM 交叉熵多目标优化**三大优化器家族，服务 flow-ai（流程排程）、xuanji-expert（架构参数调优）、primiflow-core（执行器最优并行化）与 AI 引擎配置。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**。

```rust
pub const CRATE_ID: &str = "e56676c7-ec1f-5415-9587-ba8249d0178a";
pub const ENGINE_NAME: &str = "xuanji::optimizer";
pub const CRATE_META: xuanji_common_meta::CrateMeta = xuanji_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: xuanji_common_meta::AisLayer::L4Services,
    owner: "xuanji-core",
};
```
（上为从 `src/lib.rs` 原样拷贝的三常量；与 T2 `all_crate_metas()[optimizer]` 条目完全一致）。

## §3 · 模块结构 src/* 说明
| 文件 | 职责 |
|------|------|
| `src/lib.rs` | 三常量 + 三优化器家族对外统一入口 pub fn：`cpm_critical_path / rcpsp_greedy / multi_objective_eval_cem` |

## §4 · 关键 Trait & Impl
- **`pub trait Objective<Solution>`**：`fn evaluate(&self, s: &Solution) -> f64` 标量目标（越小越优）。
- **`pub trait Schedule`**：`fn makespan(&self) -> u64`、`fn resource_profile(&self, r: ResourceId) -> Vec<u64>`。
- **Impl 三优化器家族**：
  - `cpm_critical_path(dag: &DAG) -> (Vec<NodeId /*critical*/>, u64 /*makespan*/)`；Kelley-Walker 前推+回推双 BFS。
  - `rcpsp_greedy(dag, resources, priorities) -> Schedule`；按最早开始 + 资源空闲贪心。
  - `multi_objective_eval_cem<Solution: Clone + Sample>(obj, pop_size, iters)`；交叉熵：高斯采样 N(μ,σ²) + 精英 γ=0.1 拟合更新。

## §5 · 跑单测指引
```bash
cargo test -p optimizer
# 也可作为依赖被 graph-algorithms 的测试间接调用
```
断言覆盖：CPM 钻石小图（A→{B,C}→D）关键路径长度 == 20；RCPSP 在 3 资源下给出可行调度（没有任一时间片资源超限）；CEM 对 2 维 Ackley 函数在 ≤300 次 eval 内收敛到 `f<0.1`（与种子一致）。

## §6 · 二次开发 / DIP 反转指引
- **新增目标函数**：实现 `trait Objective<MySolution>` → 直接传入 `multi_objective_eval_cem`。
- **新增调度启发式**：实现 `trait RcpspHeuristic`（trait 定义在 `lib.rs` 顶部可扩展点）→ 注入 `rcpsp_greedy_with` 变体。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：写反例（CPM 已知钻石图 → 期望值）；② GREEN：实现算法；③ 回归性能 ≥ P4 基线。
**精度护栏**：CPM 的双 BFS 必须对**带自环的非法 DAG** 返回 Err（不允许无限循环）；CEM 采样使用固定 `seed: u64 = 0xA10_CAFE` 保证可复现，严禁 `rand::thread_rng()` 无种子导致的不稳定。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-optimizer
engine id      : engine-rust-optimizer
code_graph unit: optimizer
```
self_sync：改 `src/lib.rs` 三常量 / 新增优化器对外 pub fn → `self_sync_rust.js` 刷新三注册。

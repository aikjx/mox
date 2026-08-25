# operator-core · 算子内核（AIS L6Kernel）

## §1 · 概述
璇玑 OUS 算子系统的**唯一纯内核**（AIS L6Kernel，全 workspace 唯一 L6 层 crate）：定义算子代数、守恒律、Monad 容器、类别体系、注册中心、Kernel 与 KernelExt；是所有上层算子（operator-wasm、各业务服务）的零业务依赖公共基础。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L6Kernel**（内核层，全仓唯一）。

```rust
pub const CRATE_ID: &str = "acf14283-3931-5528-adce-2c0cd3815363";
pub const ENGINE_NAME: &str = "mox::operator_core";
pub const CRATE_META: mox_common_meta::CrateMeta = mox_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_common_meta::AisLayer::L6Kernel,
    owner: "mox-core",
};
```

## §3 · 模块结构 src/* 说明
| 文件 | 职责 |
|------|------|
| `src/lib.rs` | 三常量 + 所有子模块统一再导出；pub 对外 API 面 |
| `src/types.rs` | `OperatorId / OpResult / OpError / ResourceId` 基础类型 |
| `src/operator.rs` | `trait Operator` 定义：`fn name(&self) -> &str`、`fn run(&self, inputs) -> Result<Outputs>`；类别枚举 `OperatorCategory` |
| `src/monad.rs` | **Monad 实现**：`impl<A,B> Functor<Result<A,E>>::map(f:A→B)`、`and_then`；保证链式算子组合零副作用 |
| `src/resource.rs` | `trait ResourceContainer`：算子运行时资源（CPU/GPU/内存/IO Token）池 |
| `src/state.rs` | 算子运行时状态（Idle / Running / Completed / Failed）状态机 transition |
| `src/conservation.rs` | `trait ConservationLaw` + 4 条守恒律闸门（质量守恒/能量守恒/信息守恒/维度守恒）输入输出张量不变式 |
| `src/kernel.rs` | `trait Kernel { fn exec(inputs) -> Result }`；4 个内置 Kernel（Linear/Map/Reduce/Fold） |
| `src/kernel_ext.rs` | `trait KernelExt`：泛化 blanket impl for `T: Kernel`（高阶组合 kernel.zip / kernel.and_then） |
| `src/category.rs` | `Category` 体系：算子按领域（数学/图/NLP/IO/可视化/…）标签归类 + 检索 |
| `src/registry.rs` | `struct Registry`：全局算子注册表（insert/get/list/by_category）；线程安全 `parking_lot::RwLock` |
| `src/engine.rs` | `Engine`：调度器，从 Registry 取算子并执行；支持 Fn/FnMut 闭包类型自动 impl Operator |
| `benches/operator_benches.rs` | Criterion 性能基准（P4 基线） |
| `tests/integration_full.rs` + `tests/pipeline.rs` + `tests/t7_kernel_zero_external_deps.rs` | 集成测试 / 管线编排 / T7「零重型外部依赖」契约 |

## §4 · 关键 Trait & Impl
- **`pub trait Operator`**（operator.rs）：算子最小契约 trait；任何可被 Registry 注册的结构必须 impl。
- **`pub trait ConservationLaw`**（conservation.rs）：`fn validate(inputs, outputs) -> Result<()>`；4 条定律（mass/energy/info/dim）均 impl 此 trait。
- **`pub trait Kernel` + `pub trait KernelExt`**（kernel.rs + kernel_ext.rs）：纯数学内核；KernelExt 是泛化 blanket impl（零新增依赖）。
- **`pub struct Registry` / `pub struct Engine`**：注册 + 调度。`impl Registry::{register, query_all}`；`impl Engine::{run_op(id, inputs), run_pipeline(ids, inputs)}`。

## §5 · 跑单测指引
```bash
cargo test -p operator-core
cargo test -p operator-core t7_kernel_zero_external_deps   # T7 零外部依赖契约
cargo bench -p operator-core                             # P4 Criterion 性能基线
```
断言覆盖：Monad 左右单位律 + 结合律、4 条守恒律各 3+ 反例均被拒绝、Registry ID 唯一性、T7：`pub deps` 扫描结果仅 `std + alloc + core + mox-common-meta`（禁止 serde 等重型依赖进入内核）。

## §6 · 二次开发 / DIP 反转指引
- **新增算子类型**：实现 `trait Operator for MyOp` → `Registry::global().register(Box::new(MyOp))`。严禁直接改 `engine.rs` `match` 分派。
- **新增守恒律**：实现 `trait ConservationLaw` → `conservation::register_law(...)`。
- **新增 Kernel**：实现 `trait Kernel` → 免费获得 `KernelExt` 的组合算子 blanket impl。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：`tests/integration_full.rs` 加失败断言（比如新算子守恒律闸门失败或 pipeline 串联错误）；② GREEN：最小 trait impl；③ 跑 T7 零外部依赖。
**精度护栏**：所有数值输入输出在守恒律 validate 时必须按位（NaN/Inf/±0 语义）逐维度比对，不得仅用 `approx::assert_abs_diff_eq!` 粗略宽容；浮点相对误差阈值 ≤ 1e-12。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-operator-core
engine id      : engine-rust-operator-core
code_graph unit: operator-core
```
self_sync：改 `src/lib.rs` 三常量 / 新增 trait / Kernel → `self_sync_rust.js` 刷新三注册。

# business-catalog · 业务拓扑预置目录

## §1 · 概述
璇玑 L4Services 层的**业务拓扑 & 流程预置目录**：内置 7 类高复用预置 FlowGraph/TopologyGraph（政务/法院/财务/客服/ETL/MCP/螺旋维度分析），供新需求一键骨架化、benchmark 对比和业务聚类算法样本。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**（12 个 L4 crate 之一）。

```rust
pub const CRATE_ID: &str = "62b2cca1-d98f-5e41-b26e-8d2a43966117";
pub const ENGINE_NAME: &str = "xuanji::business_catalog";
pub const CRATE_META: xuanji_common_meta::CrateMeta = xuanji_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: xuanji_common_meta::AisLayer::L4Services,
    owner: "xuanji-core",
};
```

## §3 · 模块结构 src/* 说明
| 文件 | 职责 |
|------|------|
| `src/lib.rs` | 三常量声明 + Catalog 主入口；pub 导出 `Business` / `SpiralParams` / `DimensionCheck` 等所有结构 |
| `src/spiral.rs` | 螺旋维度分析家族 9 个 struct：`Dimension / PhysicalConstants / SpiralParams / SpiralKinematics / DimensionCheck / NumericalCoincidence / SpiralAnalysisReport` + 物理常数数据库 + 螺旋运动学解析 |
| `src/bin/catalog.rs` | CLI：列出所有预置拓扑 / 导出 GraphML / 运行螺旋基准测试 |
| `tests/t8_business_dip.rs` | T8 DIP 反转契约测试：业务模板注入 vs 直接修改的合规性 |

## §4 · 关键 Trait & Impl
- **`pub trait CatalogProvider`**：统一 `fn list_topologies() -> Vec<Topology>`、`fn list_flowgraphs() -> Vec<FlowGraph>`、`fn load_by_id(id: &str) -> Option<BusinessAsset>`；下游通过 `CatalogProvider` 抽象，不直接耦合到 `Catalog`。
- **`pub struct Business`**：预置业务资产（名称/领域/分类/拓扑引用/流程图引用/标签）；`impl Business { has_tag, in_domain, to_owned }`。
- **`pub struct SpiralAnalysisReport`**：螺旋全链路报告；`impl SpiralAnalysisReport { dimension_check_summary, coincidence_list, kinetic_error_bars }`。

## §5 · 跑单测指引
```bash
cargo test -p business-catalog
cargo test -p business-catalog t8_business_dip   # T8 DIP 合规专项
cargo run -p business-catalog --bin catalog -- --list  # CLI 列出 7 类预置
```
断言覆盖：每个预置 Topology 节点数 ≥4 边数 ≥3 且连通、螺旋分析数值巧合与物理常数列精确到 6 位小数、DIP 禁止直接改 Catalog 内部数组（通过 trait 扩展注入）。

## §6 · 二次开发 / DIP 反转指引
- **新增业务目录模板**：实现 `trait CatalogProvider` → 在 `Catalog` 的 `registry: Vec<Box<dyn CatalogProvider>>` 追加。不允许直接在 `lib.rs` 写死硬编码新模板。
- **新增螺旋家族维度**：在 `spiral.rs` 追加 `Dimension` 变体，并把新增维度注册到 `DimensionCheck::registry`（DIP 点）。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**标准流程**：① RED：写失败的 `catalog::get::<template>()` 断言（新模板未注册 → None）；② GREEN：通过 `CatalogProvider` 实现注册；③ 回归全量。
**精度护栏**：螺旋常数 `PhysicalConstants` 数值固定到至少 12 位有效数字；`DimensionCheck::coincidence(p, q)` 判定阈值必须是相对误差 `|p-q|/p < ε=1e-6`，禁止用绝对差（否则大数 scale 下误判为巧合）。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-business-catalog
engine id      : module-rust-business-catalog
code_graph unit: business-catalog
```
修改 `src/lib.rs` 三常量或 `src/spiral.rs` 新增 pub struct → 运行 `self_sync_rust.js` 自动刷新三注册。

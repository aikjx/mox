# graph-algorithms · 8 大图算法内核

## §1 · 概述
璇玑知识图谱核心算法库（L4Services），8 大家族图算法（CNM 社区检测/Brandes 介数/Harmonic 紧密/PageRank/激活扩散/模块度/密度/RRF 融合）的 Rust 原生零第三方重实现，供 AI 路由、关图治理与检索重排多下游消费。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**。

```rust
pub const CRATE_ID: &str = "fbd31c6a-41cd-5274-be2f-2a28066eaf0a";
pub const ENGINE_NAME: &str = "xuanji::graph_algorithms";
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
| `src/lib.rs` | 三常量 + 9 个核心数据结构 + 8 大算法 impl 的统一入口 |
| `src/flow_graph.rs` | 意图规则 `IntentRule`、能力元 `CapabilityMeta`、意图激活结果 `IntentResult` + 统一激活扩散调用 |
| `src/bin/export_formula.rs` | CLI：把 8 算法公式导出为 JSON 供前端/治理台展示 |
| `scripts/reconcile_7x8.js` | 与 JS 侧 `graph-formulas.js` 的 7×8 结果对账脚本（C1-C7 × 8 算法） |

## §4 · 关键 Trait & Impl
- **`pub trait GraphAlgorithm<Input, Output>`**：统一算法签名 `fn run(graph: &KnowledgeGraph, params: Input) -> Result<Output>`；8 大算法各自 impl。
- **`pub struct KnowledgeGraph`**：主图谱容器（邻接表 + 节点 HashMap）；`impl KnowledgeGraph { add_node, add_edge, pagerank, cnm_community, brandes_betweenness, harmonic_closeness, activation_spread, density, modularity, rrf_rank_fuse, shortest_path }`。
- **`pub struct KnowledgeGraphBuilder`**：链式 builder + RAW 双边展开（单一真源：防止度中心性被双份 RAW 污染）。
- **`pub struct CentralityMetrics { degree, betweenness, closeness_harmonic, pagerank }`** 四维中心性打包。

## §5 · 跑单测指引
```bash
cargo test -p graph-algorithms
cd platform/backend-node && node ../services/graph-algorithms/scripts/reconcile_7x8.js   # Rust↔JS 7×8 对账
```
断言覆盖：PageRank 转置图处理（`pagerank(转置图)=重要性沿出边正确传播`）、CNM 社区检测模块度 ΔQ 贪心一致、Harmonic 对不可达节点给出有限距离贡献（`1/∞=0`，避免 NaN 污染）、激活扩散个性化 PR d=0.85 迭代 30 轮收敛误差 <1e-6。

## §6 · 二次开发 / DIP 反转指引
- **新增图算法家族**：实现 `impl GraphAlgorithm<MyInput, MyOutput>` → 挂在 `KnowledgeGraph` 作为 `pub fn my_algo(...)` thin wrapper（1 行转发，体 ≤4 行）。
- **新增强相似性/个性化融合算子**：实现 `trait GraphAlgorithm` 的签名并在 `rrf_rank_fuse` 侧通过枚举新增策略（不要独立实现 RRF 本体）。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**标准流程**：① RED：先写失败的 `test-my-algo-*.rs`，比如对已知小图断言节点排名；② GREEN：最小实现（算法本体在 `lib.rs` 单一真源）；③ 回归 `reconcile_7x8.js`。
**精度护栏**：所有公式返回值必须是 `f64` 全精度；**禁止在库内部或 wrapper 任何一层使用 `round` / `toFixed` / 截断**；密度指标必须同时附带人读解读文案（`密度 0-0.2=稀疏 0.2-0.6=中等 0.6-1=稠密`）。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-graph-algorithms
engine id      : engine-rust-graph-algorithms
code_graph unit: graph-algorithms
```
改 `src/lib.rs` 三常量 / 新增算法 pub fn → `self_sync_rust.js` 自动登记三注册。

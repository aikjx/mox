# kg-hub · 知识图谱中枢与关图治理

## §1 · 概述
L4Services 级**全域关图中枢**：承载璇玑 RelGraph 的 8 层知识图谱（L0~L7）建模、摄入、推理、治理、影响分析、热点发现与闭环 8 段循环引擎；对外统一 HybridIndex（向量+符号+倒排混合索引）+ URN 全局标识，是知识图谱领域的唯一写入真源。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**；同时作为关图核心在架构视图中扮演 L4 核心域。

```rust
pub const CRATE_ID: &str = "cb909f06-c0df-55ec-b397-543623a8c349";
pub const ENGINE_NAME: &str = "mox::kg_hub";
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
| `src/lib.rs` | 三常量 + 对外统一 API；9 个核心模块 glob 导出 |
| `src/index.rs` | `HybridIndex` 主入口：符号索引 + 倒排 + 向量 HNSW 的混合查询 |
| `src/urn.rs` | 全局统一资源名 URN：`urn:mox:<type>:<uuid>` 解析 + 稳定性保证 |
| `src/ontology.rs` | 本体模型：类层次、属性约束、rdfs:subClassOf / domain / range 推理 |
| `src/ingest.rs` | `trait Connector` + 5 种连接器（SQLite / JSON / HTTP / CSV / API），批式摄入管线 |
| `src/reason.rs` | `trait Reasoner`：前向链规则 + OWL-Horst 子集 + 激活扩散推理 |
| `src/consolidator.rs` | 实体合并 Consolidator：同 URN 多源冲突按策略（last-write-wins / LWW + timestamp / quorum）解决 |
| `src/govern.rs` | `GovPolicy` 治理策略：TTL、敏感属性遮蔽、RBAC 作用域、判重闸门 P9 |
| `src/loop_engine.rs` | 闭环 8 段循环：`Observe→Orient→Decide→Act→Verify→Learn→Archive→Plan` |
| `src/api.rs` | HTTP API 薄层（被 runtime 聚合端点使用时 re-export） |

## §4 · 关键 Trait & Impl
- **`pub trait Connector`**：`fn pull(&self, src: &Source) -> Result<Vec<Entity>>`；5 个内置实现 `{JsonFile, CsvFile, SqliteSource, HttpSource, ApiSource}Connector`。
- **`pub trait IngestPipeline`**：`extract → transform → validate → load` 四阶段；支持插件扩展。
- **`pub trait Reasoner`**：`fn apply_rules(graph &Graph, rules) -> Result<GraphDiff>`。
- **`pub struct HybridIndex`**；`impl { insert, delete, search_hybrid, vector_search, symbol_lookup }`。
- **`pub struct LoopEngine`**；`impl LoopEngine::run_cycle(once | continuous)` 闭环推进。

## §5 · 跑单测指引
```bash
cargo test -p kg-hub
```
断言覆盖：URN round-trip 解析不丢字段、本体 subClassOf 推理层级闭包、HybridIndex 召回与单独索引一致（RRF k=60 融合）、Consolidator 三策略正确性、LoopEngine 一整轮循环 ≥8 个阶段全部 tick、判重闸门 P9 重复实体合并后实体数 ≡ 真值唯一集。

## §6 · 二次开发 / DIP 反转指引
- **新增 Connector**：实现 `trait Connector` → 在 `ingest::register_connector(...)` 注册，不用改 ingest 主管线代码。
- **新增 Reasoner 规则**：在 `reason.rs` 的 RuleSet 追加 Rule enum 变体，不用改推理主循环。
- **治理策略**：实现 `GovPolicy::trait Policy`（已有 trait）→ 注入到 `govern::apply_policies` 链。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：新增 Connector → 单测 `test_connector_xxx_pull()` 返回预期实体数；② GREEN：最小实现；③ 跑 LoopEngine 全循环。
**精度护栏**：RRF 融合（`rrf_rank_fuse`）常数 k=60 不可改（必须与 graph-algorithms 单一真源一致）；判重闸门使用 Jaccard ≥0.92 + UUID 命中双条件，严禁只看 UUID 导致误合并。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-kg-hub
engine id      : engine-rust-kg-hub
code_graph unit: kg-hub
```
`self_sync_rust.js`：改 `src/lib.rs` 三常量 / 新增 Connector / Reasoner → 自动刷新三注册表。

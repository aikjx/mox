# xuanji-common-meta · T2 真源 · 元数据分层共享库（AIS L5Domain）

## §1 · 概述
璇玑 Workspace 的**统一元数据 & AIS 分层共享库**（L5Domain，全仓唯一 L5 层 crate）：定义 AIS 七层枚举、CrateMeta 结构体 + 16 个 crate 的 `all_crate_metas()` 硬编码真源 + `lookup_meta_by_engine()` 查询。所有其他 crate 声明 CRATE_META / ENGINE_NAME 时类型均从此引入；是四方对账的 T2 第一真源。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L5Domain**（纯数据元层，零网络 I/O / 零重型依赖）。

```rust
pub const CRATE_ID: &str = "34a20231-1a80-5426-b392-40d7a2ddd9f7";
pub const ENGINE_NAME: &str = "xuanji::xuanji_common_meta";
pub const CRATE_META: CrateMeta = CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: AisLayer::L5Domain,
    owner: "xuanji-core",
};
```

## §3 · 模块结构 src/* 说明
| 文件 | 职责 |
|------|------|
| `src/lib.rs` | **全部实现**：AisLayer 7 枚举 + CrateMeta 5 字段 struct + impl engine_name() + all_crate_metas() 16 条目硬编码真源 + lookup_meta_by_engine(name) 按 ENGINE_NAME 反查。**本 crate 故意只有一个源文件，保证真源单点**。 |
| `tests/crate_id_unique.rs` | 对 all_crate_metas() 16 id 唯一性 + 16 engine_name 唯一性 + 分层计数断言（12 L4 / 1 L3 / 1 L5 / 1 L6 / 1 L7）。 |
| `tests/lookup.rs` | lookup_meta_by_engine(xuanji::...) 全部 16 ENGINE_NAME 命中且正确。 |

## §4 · 关键 Trait & Impl
本 crate 是**纯数据元类型，没有对外业务 trait**。核心 Impl 为：
- **`pub enum AisLayer`**：`{ L2Gateway, L3Orchestration, L4Services, L5Domain, L6Kernel, L6KernelExt, L7Infrastructure }` 七个枚举变体，`#[derive(Copy,Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]`。
- **`pub struct CrateMeta`**：`{ id: &'static str, name: &'static str, version: &'static str, layer: AisLayer, owner: &'static str }` 5 字段（与 02-architecture.md §3.2 矩阵列严格对齐）。
- **`impl CrateMeta`**：`pub fn engine_name(&self) -> String` → 格式 `"xuanji::" + name.replace('-',"_")`（ENGINE_NAME 生成器）。
- **`pub fn all_crate_metas() -> Vec<CrateMeta>`**：16 条字面量硬编码，**T2 唯一真源**；顺序=四方对账基准顺序。
- **`pub fn lookup_meta_by_engine(name: &str) -> Option<CrateMeta>`**：对 all_crate_metas() 线性扫描 + `engine_name()` 字符串匹配（16 条线性足够，无性能问题；避免额外 Map 静态）。

## §5 · 跑单测指引
```bash
cargo test -p xuanji-common-meta
cargo test -p xuanji-common-meta crate_id_unique   # 唯一性契约（T2 核心）
cargo test -p xuanji-common-meta lookup             # ENGINE_NAME 反查
```
断言覆盖：`all_crate_metas().len() == 16`；16 CRATE_ID 皆 UUIDv5 格式 + 互不相同；16 ENGINE_NAME (`xuanji::<snake>`) 互不相同；L4=12 / L3=1 / L5=1 / L6=1 / L7=1；16 ENGINE_NAME lookup 全部命中。

## §6 · 二次开发 / DIP 反转指引
本 crate 是**零扩展点的纯真源**，几乎所有变更都属于「新增 crate」SOP 的一部分（见 project-atlas §7 Step 3c）：
1. 新增 crate → 必须在 `all_crate_metas()` 末尾追加新条目（不得插入中间，避免破坏现有顺序索引）。
2. 同步在 `platform/gateway/runtime/tests/_tmp_t2_crate_meta.rs` 加 `extern crate`、断言块、唯一性数组（否则 T2 红）。
3. 同步在 `docs/enterprise/02-architecture.md §3.2` 表格追加行，`atlas_auto_registry.json` 追加 domain-rust-<name>（四方对账）。

**禁止**在此 crate 新增业务逻辑（如网络/数据库/算法）——L5Domain 要求纯数据零重型依赖。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**（新增 crate 时）：① RED：先在 `tests/crate_id_unique.rs` 扩展期望数量 → 失败；② GREEN：在 `all_crate_metas()` 追加条目 → 再跑 T2。
**精度护栏**：CRATE_ID 必须为 UUID v5（第 14 位字符 == '5'），禁止用 UUID v4；16+ ENGINE_NAME 必须 `"xuanji::" + snake_case(name)`，与 lib.rs `pub const ENGINE_NAME` 字面量**逐字节一致**（T2 断言 `==` 而非忽略大小写）。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-xuanji-common-meta
engine id      : module-rust-xuanji-common-meta
code_graph unit: xuanji-common-meta
```
**变更触发**：只要修改 `all_crate_metas()` 条目数或某 crate 的 id/name/layer，必须同时：
1. 跑 `cargo test -p runtime --test _tmp_t2_crate_meta`（T2 回归）
2. 跑 `node platform/backend-node/test/test-t10-arch-fourway-diff.js`（四方对账 AC-22）
3. 运行 `scripts/self_sync_rust.js` 自动刷新 atlas_auto_registry 与 code_graph_bindings 对应条目。

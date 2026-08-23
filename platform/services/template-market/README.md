# template-market · 流程模板商城

## §1 · 概述
璇玑 L4Services 级流程/蓝图模板交易市场：模板发布、浏览、搜索、加载、评分、排序、Fork、2 个预置商城种子（政务流程模板 + ETL 模板）；作为 PrimiFlow 骨架 & FlowAI 流程图的统一复用真源。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**。

```rust
pub const CRATE_ID: &str = "4d2e50c1-9d64-525d-86cf-2d7d610a27b9";
pub const ENGINE_NAME: &str = "xuanji::template_market";
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
| `src/lib.rs` | 三常量 + Market 主入口：pub 导出 `Template / MarketSeed / Rating` + `Market::{list,publish,load,fork,sort_by_score}` |

## §4 · 关键 Trait & Impl
- **`pub trait MarketProvider`**：`fn list(&self, page, size, filters) -> Result<Vec<Template>>`、`fn publish(&self, tmpl: Template) -> Result<TemplateId>`、`fn load(&self, id) -> Result<Template>`；下游实现 Storage (File/SQLite/S3) 通过 DIP 注入。
- **`pub struct Template`**：id / name / kind(PrimiFlow / FlowGraph) / owner / version / content_hash / tags / created_at；`content_hash = blake3(content)` 用于幂等判重。
- **`pub struct Rating`**：{user, template_id, score 1~5, comment}；Bayesian 平滑评分修正。
- **Market impl**：`list / publish / load / fork (clone+inherit) / sort_by_score` 全方法。

## §5 · 跑单测指引
```bash
cargo test -p template-market
```
断言覆盖：publish 同 content_hash 第二次提交自动合并为新版本、fork 继承原模板 tags 并新增 `fork_of:<id>` 标签、评分 Bayesian 平滑对 0 评论给出先验均值、搜索按 tags + 全文匹配返回 ≥3 条高质量。

## §6 · 二次开发 / DIP 反转指引
- **新存储后端**：实现 `trait MarketProvider` → `Market::new(Box::new(Provider))` 注入，不改 Market 主体。
- **新模板种类**：在 `TemplateKind` enum 追加变体 → `publish()` 的 kind 校验自动放行。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：写新存储后端的 `test_provider_list_publish_load()` 失败；② GREEN：实现 Provider trait 注入。
**精度护栏**：content_hash 必须用 `blake3::hash(&content).to_hex()`，绝不允许用 md5/sha1（弱碰撞）；评分 Bayesian 强度 C=10（虚拟样本数）固定，不可配置避免被操纵。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-template-market
engine id      : module-rust-template-market
code_graph unit: template-market
```
self_sync：改 `src/lib.rs` 三常量 / 新增 Template 字段 → `self_sync_rust.js` 刷新三注册。

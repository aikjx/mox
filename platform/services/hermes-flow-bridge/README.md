# hermes-flow-bridge · Hermes 对话桥接层

## §1 · 概述
璇玑平台与外部 Hermes Agent 框架的 L4Services 级双向桥：归一化 Hermes 会话协议、拦截注入璇玑能力（LLM/关图/工作流复用）、录音全链路（session recorder）、失败降级到 MiniHermes 本地沙箱，保证璇玑 → Hermes 无感知集成。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**。

```rust
pub const CRATE_ID: &str = "9bfaf43b-385a-5a44-9fb2-65b4003ee80d";
pub const ENGINE_NAME: &str = "xuanji::hermes_flow_bridge";
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
| `src/lib.rs` | 三常量 + 对外 bridge 总入口（`HermesBridge::new/route/apply_plugins`） |
| `src/bridge.rs` | Bridge 主结构：路由表 + 插件注册表 + 录音器 + 状态池 |
| `src/normalize.rs` | Hermes 协议 ↔ 璇玑 AI 消息双向归一化（字段映射、补全缺省） |
| `src/hooks.rs` | 拦截钩子系统：pre_route（输入）/ post_route（输出）两级钩子 |
| `src/plugin.rs` + `src/integration/hermes_shim.rs` | `trait BridgePlugin`；官方 3 插件：LLM 代理 / 关图写入 / 工作流启动；hermes_shim 是 Hermes SDK thin adapter（≤8 行有效代码，禁止重写 Hermes 协议） |
| `src/router.rs` | 意图路由：`RouteTable::match_intent` → 璇玑本地能力 or 透传给 Hermes |
| `src/state.rs` | 会话状态池（内存 + 可切换持久化接口 `trait StateStore`） |
| `src/recorder.rs` | `trait SessionRecorder`；全链路会话录到 JSON lines（供审计和关图注入） |
| `src/mini_hermes.rs` | 降级：当外部 Hermes 不可达时，MiniHermes 本地规则引擎顶替（规则集在 `src/mini_hermes.rs`） |
| `src/live.rs` | 实时桥 WebSocket 推送；`src/bin/bridge_demo.rs` 交互式演示 |
| `tests/session_e2e.rs` + `tests/t8_hermes_dip.rs` | 会话端到端；T8 DIP 插件注入合规 |

## §4 · 关键 Trait & Impl
- **`pub trait BridgePlugin`**：`fn pre_hook(ctx) -> Result<Ctx>` / `fn post_hook(ctx, resp) -> Result<Resp>`；下游实现后在 `Bridge::register_plugin(...)` 注入（DIP 反转点）。
- **`pub trait SessionRecorder`**：`fn record(session_id, event)`；默认 `FileRecorder`，可替换为 Kafka/S3 实现。
- **`pub struct HermesBridge`**；`impl Bridge { new, route, apply_plugins, start_recording, enable_mini_hermes_fallback }`。
- **`pub struct Router`**：前缀 + 意图双匹配；MiniHermes 失败回退内置。

## §5 · 跑单测指引
```bash
cargo test -p hermes-flow-bridge
cargo test -p hermes-flow-bridge session_e2e
cargo test -p hermes-flow-bridge t8_hermes_dip    # DIP 合规（禁止改 bridge 内部）
cargo run -p hermes-flow-bridge --bin bridge_demo  # CLI demo
```
断言覆盖：归一化往返（Hermes→璇玑→Hermes 字段全等）、录音器至少记录 ≥ 8 个关键事件、MiniHermes 降级在 Hermes connect 失败 3 次后自动切换、插件注入通过 DIP 测试（直接硬编码 `if` 判定被 T8 FAIL）。

## §6 · 二次开发 / DIP 反转指引
- **新增 BridgePlugin**：实现 trait → `bridge.register_plugin()`。不得改 `bridge.rs::apply_plugins` 内部流程。
- **切换 Recorder 后端**：实现 `SessionRecorder` → `bridge.with_recorder(Box::new(X))`。
- **新增 Router 规则**：`Router::add_rule(prefix, handler)`，不要手写 `match` 分支在 `router.rs` 主 switch 内。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：`tests/session_e2e.rs` 加新场景（如某插件拦截改写字段）；② GREEN：通过 trait 注入实现；③ 跑 `t8_hermes_dip`。
**精度护栏**：`normalize.rs` 字段归一化时 JSON number 必须按字符串解析到 i128 再转 f64，避免 IEEE 754 浮点截断丢失大 ID；时间戳必须保留毫秒 3 位且 UTC。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-hermes-flow-bridge
engine id      : module-rust-hermes-flow-bridge
code_graph unit: hermes-flow-bridge
```
self_sync：改 `src/lib.rs` 三常量 / `trait` 定义 → `self_sync_rust.js` 刷新三注册。

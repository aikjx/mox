# primiflow-fusion · 六维融合 & 算子市场平台

## §1 · 概述
璇玑 L4Services 级融合/市场/平台聚合 crate：提供六维度量（SixDim）、守恒闸门、服务注册中心（Registry）、平台编排（Platform）、PT 文档（10 文档索引）、可观测、统一接口、server & CLI、配置加载；是 PrimiFlow 六维融合 + 算子市场上架的聚合层。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**（能力≥5，engine）。

```rust
pub const CRATE_ID: &str = "75238345-b48b-534b-818b-8d9abe083a41";
pub const ENGINE_NAME: &str = "mox::primiflow_fusion";
pub const CRATE_META: mox_common_meta::CrateMeta = mox_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_common_meta::AisLayer::L4Services,
    owner: "mox-core",
};
```

## §3 · 模块结构 src/* 说明
| 文件/目录 | 职责 |
|-----------|------|
| `src/lib.rs` | 三常量 + 8 大模块 re-export 对外 API 面 |
| `src/config.rs` | 配置加载：TOML / ENV 双源合并 + schema 校验 |
| `src/registry.rs` | `trait FusionRegistry`：服务注册/查询/列表；crate Plugin Manager |
| `src/sixdim.rs` | 六维度量 SixDim：`完备性 / 正确性 / 可维护性 / 性能 / 安全 / 可观测性` 六家族 + fuse() 融合打分 |
| `src/envelope.rs` | `trait Envelope`：请求/响应/事件 密封信封 + HMAC 签名（crypto 复用 mox-system） |
| `src/platform.rs` + `src/unified.rs` | `trait Platform` + 统一接口层（Platform→Registry→SixDim→Envelope 串联调度） |
| `src/ptdoc.rs` | 10 PT 文档索引 + `data/fusion_docs/` 10 份文档（INDEX + 10 内容）加载器 |
| `src/observability.rs` | tracing + metrics + log 三通道；OpenTelemetry 兼容 |
| `src/server.rs` + `src/main.rs` | fusion-server HTTP/WS Server 二进制 + CLI 命令分发 |
| `benches/development_experts.rs` | 专家调用 30 轮并发 fusion 性能 bench（Criterion P4） |
| `examples/fuse.rs` + `examples/registry_demo.rs` | 六维融合 + Registry 插件注册使用示例 |
| `tests/server_test.rs` | server 端到端测试：9 端点（/registry / /fusion / /sixdim / /health …） |
| `data/fusion_docs/` | 10+ 融合配套文档（INDEX.json + PT-DOC-01~10.md） |
| `.dockerignore` + `Dockerfile` | 容器化部署：fusion-server 单容器镜像 + 多阶段构建 40MB 产物 |

## §4 · 关键 Trait & Impl
- **`pub trait Platform`**：`fn start(&self) -> Result<()>` / `fn register_service(&self, spec)` / `fn run_fusion(spec) -> Result<SixDimReport>`。
- **`pub trait Envelope`**：`fn seal(&self, payload) -> Result<Sealed>` / `fn unseal(sealed) -> Result<Payload>`；含 HMAC 校验。
- **`pub trait FusionRegistry`**（registry.rs）：register / query / list / by_capability。
- **`struct Sixdim`**：6 维度量结构体 + `impl Sixdim::score() -> f64` 加权融合（权重来自 config，默认均匀）。
- **`struct Registry / Server / Config / PTDoc / Observability`**：六大结构体 + Platform 串联 impl。

## §5 · 跑单测指引
```bash
cargo test -p primiflow-fusion
cargo test -p primiflow-fusion server_test     # fusion-server HTTP 9 端点
cargo bench -p primiflow-fusion                 # P4 基准：fusion 六维 30 轮 <500ms/p99
cargo run -p primiflow-fusion                   # 默认 fusion-server :8788
cargo run -p primiflow-fusion --example fuse     # 融合示例
```
断言覆盖：SixDim 六维 ∈ [0,1]00 且 score = Σ w·dim（浮点误差 ≤1e-9）；Registry 两次 register 同 id 返回 Err（AlreadyExists）；Envelope seal→unseal 往返且篡改 HMAC 失败；server `/health` 返回 ok；Dockerfile 构建产物 <60MB（build size 断言）。

## §6 · 二次开发 / DIP 反转指引
- **新增 Fusion 插件**：实现 `trait FusionPlugin`（通过 registry.rs 的 PluginManager 注入）→ 不用改 `platform.rs::run_fusion`。
- **切换 SixDim 权重**：在 config.toml 修改 `[sixdim.weights]`，不改代码。如需新度量维度 → 在 `SixDim` 追加字段 + `score()` 的 `match` 追加 arm（thin wrapper）。
- **新增 Observability 后端**：实现 `trait MetricsSink` → `Observability::with_metrics(Box::new(X))` 注入。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：新增失败 fusion 场景（某 Plugin 维度为 110 → 应 clamp 到 100）；② GREEN：sixdim.rs 修正；③ 回归 bench 上限。
**精度护栏**：SixDim 权重和必须 = 1.0（sum(w)==1.0±1e-12），否则启动 fail-fast panic（不允许权重配置不平衡）；Envelope 签名时间戳窗口 30s，超过则拒收（防重放）。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-primiflow-fusion
engine id      : engine-rust-primiflow-fusion
code_graph unit: primiflow-fusion
```
self_sync：改 `src/lib.rs` 三常量 / Registry / SixDim 字段 → `self_sync_rust.js` 刷新三注册。

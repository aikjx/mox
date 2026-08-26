# runtime · 16 Crate 聚合网关（AIS L3Orchestration）

## §1 · 概述
璇玑 Workspace 的**统一对外聚合网关**（AIS L3Orchestration，全仓唯一 L3 层）：把 15 个 services crate + 自身，按 Cordis-5 生命周期框架组织起来；暴露 REST + WebSocket 路由、Cordis 事件总线 / 配置 / 生命周期 / 上下文 + 5 个子模块 + RBAC 鉴权中间件 + 算子市场 DSL + 版本迁移 + 治理台 HITL + OpenAPI spec + operator-server 二进制。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L3Orchestration**（编排层，全仓唯一）。

```rust
pub const CRATE_ID: &str = "a6f7ad5c-dbc8-5c27-837f-d8332fd6f27b";
pub const ENGINE_NAME: &str = "mox::runtime";
pub const CRATE_META: mox_common_meta::CrateMeta = mox_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_common_meta::AisLayer::L3Orchestration,
    owner: "mox-core",
};
```

## §3 · 模块结构 src/* 说明
| 文件/目录 | 职责 |
|-----------|------|
| `src/lib.rs` | 三常量 + crate 对外聚合入口；pub 再导出 routes/handlers/cordis/ai_router/rbac/market/subservers 全能力 |
| `src/main.rs` | operator-server 二进制入口；tokio runtime + 优雅停机 + Cordis::startup() |
| `src/routes/` (4 files) | HTTP 路由总入口 + agent（AI 智能体）/ ai_engine（四端点 process/analyze/capabilities/metrics）/ governance（HITL/审批/指标/审计）/ market（算子市场 CRUD） |
| `src/handlers/` (4 files) | 处理器薄层（request→response 适配；不含业务算法）：`agent / ai_engine / governance / hitl` |
| `src/cordis/` (6 files) — Cordis-5 框架 | `bundle.rs Bundle` 聚合 + `context.rs Request/Global` + `event_bus.rs broadcast` + `lifecycle.rs 5 钩子（startup/before_handle/after_handle/shutdown/profile）` + `seam.rs 依赖注入容器` + `profile.rs` |
| `src/ai_router.rs` | `RouterTable / CapabilityRouter / RegisteredRoute / CapabilityEntry / RouterDecision`；统一 AI 能力路由表（对应 engine-bindings 真源） |
| `src/rbac_middleware.rs` + `src/security.rs`(gateway 侧薄层) | RBAC 中间件：令牌解析→成员→作用域→权限→允许/拒绝 + 拒绝落审计不回推探测者 |
| `src/market.rs` + `src/market_dsl.rs` + `src/market_version.rs` + `src/market_migration.rs` | 算子市场 4 件套：REST/DSL/版本化/迁移；三注册 market engine |
| `src/api_standard.rs` + `src/openapi.rs` | `ProblemDetail` RFC 7807 标准错误 + OpenAPI v3 spec 自动生成（覆盖所有路由） |
| `src/automation.rs` + `src/automation_asset.rs` + `src/subservers.rs` + `src/market.rs` | 自动化（自动开发/修复/蓝图）、自动化资产（生成代码+运行记录）、子 server（拆分端口）、market |
| `src/sidecar/` (mod.rs + node_sidecar.rs) | Node.js sidecar：对 backend-node 启动/心跳/健康/降级管理（当 backend-node 挂，cordis life 优雅旁路） |
| `tests/` (7 files) | `_tmp_t2_crate_meta`（T2 回归）+ `ai_engine_e2e` + `runtime_integration` + `router_semantics` + `market_version` + `sidecar_degrade` + `mox_e2e` |

## §4 · 关键 Trait & Impl
- **`pub trait Lifecycle`**（cordis/lifecycle.rs）：`fn startup / fn shutdown / fn before_handle / fn after_handle / fn profile`。
- **`pub trait CordisBundle`**（cordis/bundle.rs）：`fn dependencies(&self) -> Vec<TypeId>` 依赖声明 + DAG 启动排序。
- **`pub trait AiRouter`**（ai_router.rs）：`fn decide(req: &Request) -> RouterDecision`；`CapabilityRouter impl AiRouter`。
- **`pub trait RbacPolicy`**（rbac_middleware.rs）：`fn authorize(token, action, resource) -> Result<Member>`。
- **Impl**：`struct RouterTable`（16 crate 路由）、`MarketDSL` 解释器、`MigrationEngine`（版本迁移 V1..Vn up/down）、`Governance` 治理台面板聚合 + `Sidecar` Node.js sidecar 管理者。

## §5 · 跑单测指引
```bash
cargo test -p runtime
cargo test -p runtime --test _tmp_t2_crate_meta   # T2 16 crate 回归（必跑！）
cargo test -p runtime ai_engine_e2e                # 四端点 /ai/engine E2E
cargo test -p runtime runtime_integration          # 全量聚合集成
cargo test -p runtime sidecar_degrade               # Node sidecar 降级
cargo run -p runtime --bin operator-server          # 启动 operator-server（默认 :3000）
```
断言覆盖：`_tmp_t2_crate_meta` 16 crate CRATE_ID/ENGINE_NAME 唯一性；路由语义 `/ai/engine/process` 200；market 版本迁移 V1→V2 roundtrip；sidecar Node.js 未启动时 fallback 无 panic；RBAC 未登录 401；治理台 metrics 端点返回预期 7 大类字段。

## §6 · 二次开发 / DIP 反转指引
- **新增 AI 路由能力**：实现 `trait AiRouter` → 在 `ai_router::register(...)` 注入。不写 match 分支。
- **新增子 crate 集成**：在 `CordisBundle::dependencies()` 声明 + 在 `routes/` 新增对应路由文件 + 在 `subservers.rs` 注册；不动 Cordis 主循环。
- **新增 RbacPolicy**：实现 trait → `rbac_middleware::with_policy(Box::new(X))`。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：先加失败的 router / sidecar 降级用例（如 `/ai/engine/capabilities` 缺 engineName → 422）；② GREEN：对应 handler + ai_router rule；③ 回归 `runtime_integration`。
**精度护栏**：所有对外错误走 `ProblemDetail {type, title, status, detail, instance}` RFC 7807 严格格式，字段不可多不可少（由 api_standard.rs struct 保证）；治理台指标小数保留 2 位但服务端仍传全精度 f64（展示层 rounded，传输层禁止舍入）。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-runtime
engine id      : engine-rust-runtime
code_graph unit: runtime
```
self_sync：改 `src/lib.rs` 三常量 / 新增路由或 CapabilityEntry → `self_sync_rust.js` 刷新三注册 + ai_router 路由表。

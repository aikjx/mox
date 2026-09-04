# primiflow-core · PrimiFlow 骨架 & 执行核心

## §1 · 概述
璇玑 L4Services 级 PrimiFlow 蓝图引擎核心：把 DDL 式 PrimiFlow 领域语言解析 → 代码生成（8 类骨架模板 C1~C8）→ 持久化 → 执行（Runner/Server），承担企业级mox 模块化系统架构分析流程的编译时+运行时双生命周期。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**。

```rust
pub const CRATE_ID: &str = "8c8d2382-6f9f-5218-894e-a07a43aa9554";
pub const ENGINE_NAME: &str = "mox::primiflow_core";
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
| `src/lib.rs` | 三常量 + 对外 API 总入口（解析/生成/持久化/执行/服务五大入口 pub fn） |
| `src/parse.rs` | PrimiFlow DDL `.pf` 文件解析器 → AST（枚举 + 结构体） |
| `src/generate.rs` | DIP 代码生成调度器：按模板种类分派到 `gen/` 8 个子模板 |
| `src/gen/c1.rs ~ c8.rs` + `gen/ddl.rs` + `gen/schema.rs` + `gen/graph.*` | 8 类骨架模板：C1 业务规则、C2 CRUD、C3 工作流、C4 数据流、C5 集成契约、C6 审计/鉴权、C7 指标/监控、C8 关图实体；附带 DDL+Schema+可视化 |
| `src/assoc.rs` | 关联关系建模（模板→模板 / 资产→资产 的边） |
| `src/persistence.rs` + `src/runner.rs` | 持久化 Store（SQLite/JSON 双后端）+ 生成代码的执行 Runner 调度 |
| `src/server.rs` | PrimiFlow 独立 server（axum 路由）：DDL 提交 → 生成 → 执行 → 返回 |
| `examples/*.rs` (15+ files) | C1-C10015 企业级种子示例 + 生成示例 + 集成示例 + server_demo + enterprise_demo |
| `examples/out/*` | 模板生成产物样本 + trace_matrix 文档 |
| `tests/pipeline_exec.rs` / `tests/enterprise_validation.rs` / `tests/api_server.rs` | 管线执行回归、企业级 15 项指标验证、API Server 端到端 |

## §4 · 关键 Trait & Impl
- **`pub trait Executor`**：`fn execute(&self, plan: &Plan) -> Result<ExecutionReport>`；Runner 默认真执行。
- **`pub trait Store`**：`fn save_artifact(...) / fn load_artifact(...)`；Persistence 双后端 impl。
- **`pub trait Generator`**：`fn generate(ast: &AST, tmpl: TemplateKind) -> Result<CodeBundle>`；8 个子模板各自 impl。
- **`pub struct Parse` / `struct Persistence` / `struct Runner` / `struct Server`**：5 大核心结构体；`impl Parse::from_str / Server::serve("0.0.0.0:8787")` 等。

## §5 · 跑单测指引
```bash
cargo test -p primiflow-core
cargo test -p primiflow-core enterprise_validation   # 15 企业级指标全量
cargo test -p primiflow-core api_server              # PrimiFlow Server HTTP E2E
# 直接跑交互式 server: cargo run -p primiflow-core --example server_demo
```
断言覆盖：解析 AST round-trip → generate 字节级产物稳定（examples/out 对比）、执行管线 4 步全通过、enterprise_validation 15 指标 ≥ 14.5（企业级合格线）、API Server 8 端点 CRUD 全绿。

## §6 · 二次开发 / DIP 反转指引
- **新增第 9 类模板**：在 `src/gen/` 新建 `c9.rs`，实现 `trait Generator` → 在 `generate.rs` 的模板分派 match 中追加 1 个 arm（thin wrapper 2 行）。不得改 C1~C8 既有模板。
- **切换 Persistence 后端**：实现 `trait Store` → `Persistence::with_store(Box::new(X))` 注入。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：新增 C9 模板 → `enterprise_validation.rs` 对应项先 FAIL；② GREEN：实现 Generator + 分派 arm；③ 跑 examples/out 产物字节级对比（必须与预期一致）。
**精度护栏**：DDL 解析的小数/时间戳必须用 `rust_decimal` + `chrono::DateTime<Utc>`；严禁把 timestamp 舍入到秒（否则 enterprise_validator 里「P9 精度」项 FAIL）。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-primiflow-core
engine id      : engine-rust-primiflow-core
code_graph unit: primiflow-core
```
self_sync：改 `src/lib.rs` 三常量 / 新增模板 gen/c*.rs → `self_sync_rust.js` 刷新三注册。

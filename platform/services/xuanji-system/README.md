# xuanji-system · 璇玑业务 & 基础设施层（AIS L7Infrastructure）

## §1 · 概述
璇玑平台的业务系统主体 + 基础设施（AIS L7Infrastructure，全仓唯一 L7 crate）：覆盖成员/任务/权限/通信四大核心业务域，加 RBAC、限流、事件编排、加密、指标、多后端 Repository（SQLite/PostgreSQL/MySQL），是璇玑产品的业务真源。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L7Infrastructure**（持久化 & 基础设施层，全仓唯一）。

```rust
pub const CRATE_ID: &str = "b81eec75-22ff-5155-ac49-19edf6f6b5ab";
pub const ENGINE_NAME: &str = "xuanji::xuanji_system";
pub const CRATE_META: xuanji_common_meta::CrateMeta = xuanji_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: xuanji_common_meta::AisLayer::L7Infrastructure,
    owner: "xuanji-core",
};
```

## §3 · 模块结构 src/* 说明
| 文件/目录 | 职责 |
|-----------|------|
| `src/lib.rs` | 三常量 + 对外业务/基础设施总入口（pub re-export 全部子模块） |
| `src/main.rs` + `src/server.rs` | 二进制入口 + axum HTTP/WS Server；端点挂载所有 Service + WebSocket 推送 |
| `src/orchestrator.rs` | 总编排：鉴权闸门 `require()`（§5.2 护栏）+ 反应器（DomainEvent→消息/通知/审计） |
| `src/services.rs` | 4 大 Service：`MemberService / TaskService / PermissionService / CommService` |
| `src/domain_traits.rs` | `trait DomainService` 家族；DIP 扩展点 |
| `src/repo/` (5 files) | `trait Repository`；`mod.rs` 总入口 + 3 后端实现 `sqlite.rs / postgres.rs / mysql.rs` + `schema.rs`（sea-query 方言差异统一） |
| `src/persistence_provider.rs` + `src/store.rs` | `trait PersistenceProvider` + 内存态 Store（XUANJI_PERSIST=false 时） |
| `src/rbac.rs` | RBAC：5 角色 + 14 原子权限 + 作用域 Global/Xuanji/Task + 所有权 `*Own` 规则 |
| `src/error.rs` + `src/event.rs` + `src/metrics.rs` + `src/ratelimit.rs` + `src/crypto.rs` + `src/config.rs` + `src/model.rs` | 9 类 DomainEvent、指标收集、令牌桶限流、HMAC+SHA2 签名+AES 加密、配置、核心业务 Model 实体 |
| `tests/integration.rs` / `tests/business_rules.rs` / `tests/t6_dip_orchestrator.rs` | 集成测试 / 业务规则不变式 / T6 DIP 编排器反转合规 |

## §4 · 关键 Trait & Impl
- **`pub trait Repository`**（repo/mod.rs）：`fn members() -> Result<Vec<Member>>; fn tasks() -> Result<Vec<Task>>; fn save_event(&DomainEvent); ...`；3 后端 `impl Repository for SqliteRepo / PostgresRepo / MysqlRepo`。
- **`pub trait PersistenceProvider`**（persistence_provider.rs）：`fn get_repo() -> Box<dyn Repository>`；按 `XUANJI_DB_BACKEND` 路由。
- **`pub trait DomainService`**（domain_traits.rs）：`trait MemberService / TaskService / PermissionService / CommService` 四 trait（DIP 点，可替换实现）。
- **`struct Orchestrator`**；`impl Orchestrator { require(token, perm, scope) -> Result<Member> }` 鉴权闸门 + 反应器（publish 9 DomainEvent → broadcast）。

## §5 · 跑单测指引
```bash
# 默认 SQLite 内存态
cargo test -p xuanji-system
# 启用 Postgres 后端（需要本地 PG 连接）
XUANJI_DB_BACKEND=postgres DATABASE_URL=postgres://... cargo test -p xuanji-system integration
# MySQL
XUANJI_DB_BACKEND=mysql DATABASE_URL=mysql://... cargo test -p xuanji-system
# T6 DIP
cargo test -p xuanji-system t6_dip_orchestrator
# 运行演示
cargo run -p xuanji-system -- --demo
```
断言覆盖：5 角色 RBAC 14 权限 9 类边界（跨璇玑/非Active/作用域）、Orchestrator `require()` 失败留痕终局才落、assign 成员跨璇玑 triple check 通过、3 后端 Repository 同一接口读写语义一致、9 DomainEvent 广播后消费者都被触发。

## §6 · 二次开发 / DIP 反转指引
- **新增 Service 扩展**：实现对应 `trait MemberService / ...`（在 domain_traits.rs）→ 在 orchestrator 注入，不改 Service 主 struct。
- **新增 DB 后端（如 MongoDB）**：实现 `trait Repository` → 在 `persistence_provider.rs::from_env()` 追加 match arm（thin wrapper）。
- **新增业务事件**：在 `DomainEvent` enum 追加变体 → 自动触发反应器 `broadcast`，无需改 orchestrator 主代码。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：新增业务规则不变式（如 invite 重复成员应 409）→ `tests/business_rules.rs` 先 FAIL；② GREEN：在对应 Service impl；③ 回归 3 后端。
**精度护栏**：`crypto.rs` 的 HMAC 签名密钥至少 256-bit，签名比较必须使用 `constant_time_eq`（计时攻击防御）；RateLimit 令牌桶每次 refill 使用 `Duration::as_nanos()` 精度，禁止秒级截断。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-xuanji-system
engine id      : engine-rust-xuanji-system
code_graph unit: xuanji-system
```
self_sync：改 `src/lib.rs` 三常量 / trait / Repository 后端 → `self_sync_rust.js` 刷新三注册。

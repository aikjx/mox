# 璇玑 project-atlas Rust 绑定契约

> 文档版本：`Rust-Binding Contract v2.0 (企业级归一化 · 2026-09)`
>
> 适用范围：璇玑全域知识图谱 ↔ 15 个 Rust crate ↔ Node.js `project-atlas` 三向绑定。
>
> 契约目标：**Rust 侧改一行 → atlas 立刻感知 → 图谱节点双向关联 100%**；Node 侧 `domain-rust-*` 条目永远只以 Rust `CRATE_ID` + `CRATE_META` 为单一真源（Single Source of Truth）。

---

## 1. 三层绑定拓扑

```
┌────────────────────────────────────────────────────────────────────┐
│  RUST-SIDE (15 Crates · primary_impl = RUST 单源)                  │
│                                                                    │
│  [lib.rs]   pub const CRATE_ID  = "<uuid v4 per crate>";           │
│             pub const CRATE_META = CrateMeta {                     │
│               id, name, version, layer, owner                     │
│             };                                                     │
│          │                                                         │
│          │  (导出可见性: pub, 稳定, 不被 feature 开关)              │
│          ▼                                                         │
│  mox-common-meta：CrateMeta 枚举 AisLayer · single def          │
└──────────────────────────────────┬─────────────────────────────────┘
                                   │  (domain id 计算规则见 §3)
                                   ▼
┌────────────────────────────────────────────────────────────────────┐
│  ATLAS-SIDE (Node.js) · business-registry.js                      │
│                                                                    │
│  autoRustCrates = 15 Rust crates * [                               │
│     {  id: "domain-rust-<dir_name>",                               │
│        name, codePath, owner, crate_id, layer, meta_version, ...} │
│  ]; // 生成时直接嵌入 CRATE_ID (不再手工维护 ID)                   │
└──────────────────────────────────┬─────────────────────────────────┘
                                   │  (T6 绑定校验：CRATE_ID 匹配)
                                   ▼
┌────────────────────────────────────────────────────────────────────┐
│  KNOWLEDGE-GRAPH (mox · kg-hub + graph-algorithms)             │
│                                                                    │
│  节点类型 = AtlasDomainNode · attrs: {                             │
│    domainId, crateId, layer, owner, codePath, primaryImpl, ...    │
│  }                                                                 │
│  关系：REQUIRES / PROVIDES /  OWNED_BY / AUTO_REGISTERED           │
└────────────────────────────────────────────────────────────────────┘
```

---

## 2. Rust 侧最小接口（不可破坏）

每个 Rust crate 的 `src/lib.rs` 必须导出以下两个 `pub const`，且**不得受 feature 影响**（feature-gated 将直接被 T6 判定为 contract 违约）：

```rust
pub const CRATE_ID: &str = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx";

pub const CRATE_META: mox_common_meta::CrateMeta = mox_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_common_meta::AisLayer::<LAYER_CONST>,
    owner: "mox-core",
};
```

当前 15 个 crate 的 CRATE_ID 全表（从 Rust 源码同步）：

| # | Crate Dir            | Layer (CRATE_META.layer)   | CRATE_ID                                |
|---|----------------------|----------------------------|-----------------------------------------|
| 1 | mox-common-meta   | L1Infra                    | `a1c2b3d4-5e6f-4a1b-8c2d-1e3f5a7b9c0d`  |
| 2 | operator-core        | L6Kernel                   | `b8e1f2a3-4c5d-4e6f-8a9b-0c1d2e3f4a5b`  |
| 3 | graph-algorithms     | L9Algo                     | `cf1a2b3d-4e5f-4678-9abc-def012345678`  |
| 4 | primiflow-core       | L7Flow                     | `d0a89172-3b4c-5d6e-7f89-0a1b2c3d4e5f`  |
| 5 | kg-hub               | L8Data                     | `e92f1a3b-5c7d-4e8f-9a0b-1c2d3e4f5a6b`  |
| 6 | flow-ai              | L5Abstraction              | `f5e4d3c2-b1a0-4f3e-8d7c-9b6a5f4e3d2c`  |
| 7 | business-catalog     | L4Services                 | `1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d`  |
| 8 | mox-system        | L4Services                 | `2b3c4d5e-6f7a-8b9c-0d1e-2f3a4b5c6d7e`  |
| 9 | optimizer            | L9Algo                     | `3c4d5e6f-7a8b-9c0d-1e2f-3a4b5c6d7e8f`  |
|10 | mox-expert        | L4Services                 | `4d5e6f7a-8b9c-0d1e-2f3a-4b5c6d7e8f9a`  |
|11 | primiflow-fusion     | L3Application              | `5e6f7a8b-9c0d-1e2f-3a4b-5c6d7e8f9a0b`  |
|12 | ai-agent             | L4Services                 | `6f7a8b9c-0d1e-2f3a-4b5c-6d7e8f9a0b1c`  |
|13 | template-market      | L4Services                 | `7a8b9c0d-1e2f-3a4b-5c6d-7e8f9a0b1c2d`  |
|14 | hermes-flow-bridge   | L4Services                 | `9bfaf43b-385a-5a44-9fb2-65b4003ee80d`  |
|15 | operator-wasm        | L6Kernel                   | `9c0d1e2f-3a4b-5c6d-7e8f-9a0b1c2d3e4f`  |
|16 | runtime (gateway)    | L2Platform                 | `ab1c2d3e-4f5a-6b7c-8d9e-0f1a2b3c4d5e`  |

> 本表与 Node 侧 `test/rust_crate_bindings_e2e.js` 的 TR-06 断言完全一致（「每一个 atlas 条目 ↔ CRATE_ID 完全匹配」）。若本契约与 Rust 源码不一致，**源码是唯一真源，需先改源码再同步文档**。

---

## 3. Atlas 侧绑定规则（`autoRustCrates` 生成协议）

`business-registry.js` 的 `const autoRustCrates: AtlasDomainNode[] = ...` 必须满足：

### 3.1 域 id 生成规则（稳定，不允许改）
```
domainId = "domain-rust-" + <crate 根目录名>
```
例：`platform/services/ai-agent/` → `domain-rust-ai-agent`；`platform/gateway/runtime/` → `domain-rust-runtime`。

### 3.2 `codePath` 生成规则
- 标准 crate：`platform/services/<dir>/src/lib.rs`
- runtime：`platform/gateway/runtime/src/lib.rs`

### 3.3 属性映射

| Atlas 字段          | Rust 来源                                  | 说明                       |
|---------------------|--------------------------------------------|----------------------------|
| `id`                | `domain-rust-<dir_name>`                   | 见 3.1                    |
| `name`              | `CRATE_META.name` + 中文职责后缀           | T5 nameNotEmpty           |
| `codePath`          | `<crate 绝对路径>/src/lib.rs`              | T2 可打开                 |
| `owner`             | `CRATE_META.owner` (固定 "mox-core")    | T4 ownership              |
| `engines[]`         | 固定 `["rust-" + <dir_name>]`              | primary_impl=RUST         |
| `meta.crate_id`     | `CRATE_ID`                                 | T6 绑定锚点               |
| `meta.layer`        | `CRATE_META.layer`                         | AIS 分层标识              |
| `meta.version`      | `CRATE_META.version` (`env!(CARGO_PKG_V)`) | 对齐 cargo package.version |
| `primary_impl`      | 固定 `"RUST"`                              | 单源声明，避免 JS 复制实现  |

### 3.4 自动注册校验（`rust_crate_bindings_e2e.js` 5 TR）
- **TR-02**：每一个 `codePath` 指向的 lib.rs 磁盘真实存在且可读
- **TR-04**：每一个 domain owner = "mox-core"（ownership 不漏人）
- **TR-05**：`name` 非空（含中文可读描述）
- **TR-06**：`crate_id` 嵌入 CRATE_ID，与表 §2 匹配
- **TR-07**：`domainId` × `CRATE_ID` × `codePath` × `engines` 绑定完整（4-tuple 非空）

### 3.5 非 Rust 条目隔离
atlas 总条目 = 30 =（autoRustCrates 15 Rust）+（平台/业务 15 条）。任何新增业务域都**不触碰 autoRustCrates 段**；Node 侧 W1 路由域数量校验用动态 `count = autoRustCrates.length + static.length`。

---

## 4. 变更治理（契约锁）

| 变更类型               | 必须同步                                |
|------------------------|-----------------------------------------|
| 新增 Rust crate        | ① lib.rs CRATE_ID / CRATE_META 2 行 ② atlas autoRustCrates 新增 1 行 ③ 本契约 §2 表 1 行 ④ README.md 1 份 |
| 修改 crate 目录名      | 以上全部 + 图谱 domainId 迁移脚本（避免孤儿节点） |
| 修改 CRATE_ID          | 强制不允许（UUID 冻结；若必须改则图谱 T6 全量回归） |
| 升级 AisLayer          | 1) Rust CRATE_META.layer 2) README 分层定位 3) 本契约 §2 表 |

---

## 5. 绑定校验三触达（企业级 CI 门禁）

1. **构建前 `scripts/validate_rust_workspace_deps.js`**：检查 15 crate 间 deps 不循环、每个 lib.rs 均导出 `pub const CRATE_ID` & `CRATE_META`。
2. **PR 门 `test/rust_crate_bindings_e2e.js` 56/56**：TR-02/TR-04/TR-05/TR-06/TR-07 全绿，否则阻断合并。
3. **T12 对账 `scripts/run_t12_integration_test.ps1`**：Rust 8/8 + Node 56/56 + 公式 35/35 三重全绿。
4. **Clippy 门 `cargo clippy --workspace -- -D warnings`**：拒绝 warning 进入 master。

# 任务队列：璇玑 · Rust 后端全维度自研归一化

> 对应规格：[spec.md](spec.md) · 语言：中文 · 队列原则：先高后低；先骨架（注册表/常量/绑定）后血肉（DIP抽象/性能修复/README）；依赖顺序严格拓扑；跨文件不可并发写。

---

## 任务组织总览（按高→中→低排序 + 依赖拓扑号）

| 任务 | 名称 | 优先级 | 前置依赖 | 对应 AC | 状态 |
|---|---|---|---|---|---|
| **T1** | 璇玑图谱 Rust crate 16 条目入库（三注册表 + 动态层 self-sync） | HIGH | - | AC-01/02/03 | pending |
| **T2** | 16 crate 显式 CRATE_ID + CRATE_META 常量注册范式 | HIGH | - | AC-13/14 | pending |
| **T3** | 7 条核心算法 singleSource 真实性修复（registry Rust 主实现声明 + co_impl 对账） | HIGH | T2 | AC-04/05 | pending |
| **T4** | 依赖治理 100% workspace 继承（消除写死版本 + dev-deps 漂移） | HIGH | - | AC-09/10 | pending |
| **T5** | rusqlite 框架依赖收拢到 xuanji-system（ai-agent / primiflow-core 移除）+ PersistenceProvider trait | HIGH | T2 (crate 元信息) | AC-11/26 | pending |
| **T6** | DIP 反转：xuanji-system orchestrator 依赖 trait（Member/Task/Permission） | HIGH | - | AC-06 | pending |
| **T7** | DIP 反转：operator-core L6 kernel/kernel_ext 双层模块（std-only vs serde wrapper） | HIGH | - | AC-07 | pending |
| **T8** | DIP 反转：hermes-flow-bridge / business-catalog 改依赖 xuanji-expert 抽象 trait（非 concrete struct） | HIGH | - | AC-08 | pending |
| **T9** | 500 深链性能修复（≤10s，拓扑剪枝/缓存） | HIGH | - | AC-18/19 | pending |
| **T10** | 架构文档补齐：02-architecture.md §3.2 Rust 分层表 16 crate + §7.1 runtime 分层 + project-atlas.md Rust 绑定契约 | HIGH | T1/T2 | AC-15/16/17 | pending |
| **T11** | 14 crate README 补全（16 crate 全项目化） | MEDIUM | T1/T2（node id / CRATE_ID 可用） | AC-12/28 | pending |
| **T12** | Rust/Node 核心算法对账二进制（7 算法同输入同输出 Δ≤1e-6）+ 测试 | MEDIUM | T3（registry 声明完成） | AC-05 | pending |
| **T13** | 构建全量回归：cargo build/test/clippy + Node 三套测试（W/归一/EAF）+ pub API diff | MEDIUM | T4-T9 完成后 | AC-20/21/22/23/24/25/29/30 | pending |
| **T14** | DIP 复用性证据 + rubric 指标打分 + 31 条 AC 全部验收举证汇总 | LOW | T1-T13 | AC-27/28/29/30/31 | pending |

---

## Task 1: 璇玑图谱 Rust crate 16 条目入库（三注册表 + 动态层 self-sync）

- **Status**: pending
- **Priority**: HIGH
- **Objective**: 把 16 Rust crate（按 workspace Cargo.toml 顺序）全部录入 project-atlas 的 business/engine/tech/auto 四通道，保证 `GET /atlas/verify` W1/W10 全绿。
- **Scope**:
  - `src/project-atlas/domain/business-registry.js`：新增 16 DOMAIN 条目 kind="rust-crate"，顶层 scope=platform/services/<crate> 或 platform/gateway/runtime；codePath 绝对路径存在；domain_owner（归属 8 项目之一按 AIS 分层语义智能分配）。
  - `src/project-atlas/domain/engine-registry.js`：对每个至少含 pub fn engine/engine 概念的 crate（ai-agent/xuanji-expert/primiflow-core/primiflow-fusion/kg-hub/graph-algorithms/flow-ai/optimizer/runtime/xuanji-system/operator-core）≥ 11 engine 节点；其余 crate（operator-wasm/hermes/business-catalog/template-market）若没有 engine 概念，则作为 module 节点登记。
  - `src/project-atlas/domain/tech-registry.js`：对 Rust graph-algorithms 的 7 条算法 + ai-agent provider 路由注册 + operator-core Conservation 等算法，登记 algorithm 节点 primary_impl_codePath 指 Rust。
  - `data/atlas_auto_registry.json` 动态层：新增 16 Rust crate 条目（scope="rust-crate" + scope=rust-engine + scope=rust-algorithm）。
  - `src/project-atlas/domain/project-registry.js`：为 Rust crate 设置 owns_domain 边（P1-P8 智能归属，按 AIS 分层与业务域语义：如 runtime→P5 xuanji-platform；xuanji-expert→P4 expert-alliance；ai-agent→P3 ai-dialogue；graph-algorithms/operator-core/optimizer/flow-ai→P7 graph-infra + P2 ai-engine；xuanji-system/primiflow-*→P1 xuanji-core；kg-hub→P2 knowledge；hermes-flow-bridge/business-catalog/template-market/operator-wasm→P6 auto-dev 或 P7）。
- **Dependencies**: None（其他任务都要用到绑定结果，因此最先做）。
- **Deliverables**: 4 个 registry 文件 edit；`GET /atlas/verify` W1=30+16=46 DOMAINS 全 PASS。
- **Test Requirements**:
  - **TR-01-01 (rule)**: `node test/test-project-atlas.js` W1 "域存在性" PASS（当前 30 域 → 修复后 46 域全登记）
  - **TR-01-02 (rule)**: 三注册表中含 16 条 kind="rust-crate" 条目，每条 codePath `fs.existsSync` = true
  - **TR-01-03 (rule)**: atlas_auto_registry.json 新增 Rust 条目 ≥ 16
  - **TR-01-04 (rule)**: `test-project-atlas.js` W10 项目唯一归属 PASS（8 项目 owns_domain 所有 Rust crate，无重复归属）
  - **TR-01-05 (rubric, AC-27)**: Rust→图谱绑定完备度：scale 0-4。准入 ≥3.5。统计维度：16 crate 各自具备 {engine|module node + algorithm node(如适用) + codePath 存在 + owner project 边 exists} 的完整四元组的比例。

---

## Task 2: 16 crate CRATE_ID + CRATE_META 常量

- **Status**: pending
- **Priority**: HIGH
- **Objective**: 每个 Rust crate 顶层暴露 `pub const CRATE_ID` + `pub const CRATE_META: CrateMeta`（统一结构体字段），实现图谱可基于稳定常量自动发现。
- **Scope**:
  - 16 个 crate 的 src/lib.rs 顶部（或对应入口文件）新增：
    ```rust
    pub const CRATE_ID: &str = "crate-name-kebab";
    pub struct CrateMeta {
        pub uuid: &'static str,
        pub ais_layers: &'static [&'static str],   // ["L1","L2"] 按实际
        pub owner_project: &'static str,           // 对应 project-registry 的项目 id
        pub capabilities: &'static [&'static str], // 暴露能力：如 ["CNM社区发现","PageRank"]
        pub data_tables_read: &'static [&'static str], // 读哪些 SQLite 表或 JSON
        pub data_tables_write: &'static [&'static str],
    }
    pub const CRATE_META: CrateMeta = CrateMeta { uuid: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx", ais_layers: &[...], owner_project: "proj-...", capabilities: &[...], data_tables_read: &[], data_tables_write: &[] };
    ```
  - operator-core/types.rs 无统一位置 → 统一放在各 crate `src/lib.rs` 顶层 pub export，即使 crate 有 20+ 子模块也统一入口 lib 暴露 meta。
- **Deliverables**: 16 crate `src/lib.rs` edit。
- **Test Requirements**:
  - **TR-02-01 (rule, AC-13)**: `grep -l "pub const CRATE_ID" platform/services/*/src/lib.rs platform/gateway/runtime/src/lib.rs | wc -l` = 16
  - **TR-02-02 (rule, AC-13)**: 对每个 CRATE_META，`CRATE_META.ais_layers.len() >= 1`（至少有 1 层归属）
  - **TR-02-03 (rule, AC-14)**: 4 处代码注册表（operator-core OperatorRegistry/primiflow-fusion CRATE_NAMES/runtime SeamRegistry/ai-agent ProviderRegistry）的条目，在 business-registry/engine-registry 中存在对应 node，且 node.id == "engine-rust-" + CRATE_ID 或 "algo-rust-" + registry.key 前缀一致。
  - **TR-02-04 (rubric)**: UUID 唯一性：16 CRATE_META.uuid 互异 count=16。score 0-1。达到 1 = 满分。

---

## Task 3: 核心算法 singleSource 真实性修复 + Rust/Node 对账

- **Status**: pending
- **Priority**: HIGH
- **Depends**: T2（CRATE_ID 可用，构造 algorithm node id 前缀）
- **Objective**: 让 tech-registry 中 6 条图算法 + 1 条模块度不再假的 singleSource=true；改为 co_impl 模式 Rust primary + Node secondary；同时建立 Rust 算法导出工具用于对账。
- **Scope**:
  - `src/project-atlas/domain/tech-registry.js`：
    - algo-cnm：新增 `impl_kind: "co_impl"`, `primary_impl: "RUST"`, `singleSource: false`；codePath 保留 Node，但新增 `primary_impl_codePath: "platform/services/graph-algorithms/src/lib.rs#L577-L855"` + `secondary_impl_codePath: "src/ai-flow-graph.js"`
    - algo-pagerank（推模型转置图）：同上 pattern
    - algo-brandes / algo-harmonic / algo-degree / algo-density / algo-modularity：同上
  - Rust graph-algorithms crate：新建 `src/bin/export_formula.rs`，对 T1~T8 数据集运行 PageRank/CNM/介数/harmonic/度/模块度/密度，把结果序列化为 JSON 输出到 stdout（用于 Node 侧对账）。
- **Test Requirements**:
  - **TR-03-01 (rule, AC-04)**: tech-registry.js 7 条核心算法含字段 primary_impl_codePath，且绝对路径行范围包含 CNM/PR 等函数实现。
  - **TR-03-02 (rule, AC-04)**: 7 条算法 singleSource=false（允许 co_impl）或 true 仅当只有 1 端实现（不存在）。
  - **TR-03-03 (rule, AC-05)**: 对账脚本（Rust bin → Node assert）对 7 算法 × T1~T8 共 56 项比较，|Rust - Node| ≤ 1e-6 全部通过。
  - **TR-03-04 (rubric)**: 算法对账覆盖率：实际比较数 / 期望 56 项。score: 0-1（0=<10 项，1=全 56 项）。阈值 ≥1。

---

## Task 4: 依赖治理统一 workspace 继承

- **Status**: pending
- **Priority**: HIGH
- **Objective**: 16 crate Cargo.toml 所有 3rd-party 依赖版本 100% `workspace = true`（含 dev-deps）；消除版本漂移与显式版本字符串。
- **Scope**:
  - 遍历：xuanji-expert / hermes-flow-bridge / business-catalog 三 crate Cargo.toml → 把 serde / tokio / thiserror / anyhow / chrono / uuid / reqwest / tracing / serde_json 等全部替换为 `workspace = true`，features 保留即可。
  - primiflow-core dev-deps reqwest `version = "0.11"` → `workspace = true`，必要时代码小改造适配 0.12（如果有 breaking 差异）。
  - primiflow-fusion dev-deps criterion `default-features=false` 与 workspace criterion baseline 差异：在 workspace.dependencies.criterion 统一改为 default-features=false（如果不影响 operator-core），或在 fusion 里保留 features 但根版本用 workspace=true + 显式 features 覆盖。
  - 把 xuanji-expert Cargo.toml dependencies 19 项里所有写死版本替换为 workspace=true。
- **Deliverables**: 5 个 crate Cargo.toml 修改（xuanji-expert/hermes-flow-bridge/business-catalog/primiflow-core/primiflow-fusion）。
- **Test Requirements**:
  - **TR-04-01 (rule, AC-09)**: 写检查脚本 `scripts/validate_rust_workspace_deps.ps1`（或 inline Node 脚本）对 16 Cargo.toml，扫描除 workspace.dependencies = true / path = 外部 / git = 外的所有 `crate = ...` 配置中出现 `version = "x.y"` 字符串的总数 = 0（path 依赖、crate 本身内部 workspace = true 配置除外）。
  - **TR-04-02 (rule, AC-10)**: dev-deps 的 reqwest/criterion 检查同上，版本字符串数 = 0。
  - **TR-04-03 (rule, AC-21 build)**: `cargo build --workspace` exit 0 验证依赖配置不破坏编译。
  - **TR-04-04 (rubric, 构建洁净度)**: 构建 + clippy 基线。score 0-3（见 AC-30）。

---

## Task 5: rusqlite 框架依赖收拢到 xuanji-system（ai-agent + primiflow-core 移除）

- **Status**: pending
- **Priority**: HIGH
- **Depends**: T2（CRATE_META.data_tables_read/write 字段可用于契约定义）
- **Objective**: 把 rusqlite 从 L3 ai-agent 和 L4 primiflow-core 两 crate 移除，全部改为 `xuanji-system` 层 PersistenceProvider trait 抽象 + 注入，保证 AIS 框架级依赖仅在 Infra 层。
- **Scope**:
  - 新增 `xuanji-system/src/persistence_traits.rs`：
    ```rust
    pub trait SessionStore {
        fn save_dialogue_session(&self, id: &str, blob: &[u8]) -> Result<()>;
        fn load_dialogue_session(&self, id: &str) -> Result<Option<Vec<u8>>>;
        fn list_sessions(&self) -> Result<Vec<String>>;
    }
    pub trait TemplateStore {
        fn save_primi_generated(&self, id: &str, bytes: &[u8]) -> Result<()>;
        fn load_primi_generated(&self, id: &str) -> Result<Option<Vec<u8>>>;
    }
    // 其他需要的小表...
    pub struct PersistenceProvider {
        pub sessions: Arc<dyn SessionStore + Send + Sync>,
        pub templates: Arc<dyn TemplateStore + Send + Sync>,
    }
    ```
  - xuanji-system 实现 default rusqlite-backed implementations（Repo 内已有 rusqlite）。
  - ai-agent Cargo.toml 删除 rusqlite workspace=true；ai-agent `src/dialogue_graph.rs` / `src/conversation.rs` 等把 `rusqlite::Connection::open(...)` 改为使用全局设置的 `Box<dyn SessionStore>`（通过 once_cell 或 constructor 参数传入）。
  - primiflow-core Cargo.toml 删除 rusqlite workspace=true；`persistence.rs` 改为使用 TemplateStore 抽象；在 runtime 聚合时注入 xuanji-system 的 impl。
  - 兼容方案：如果 ai-agent / primiflow-core 有 standalone mode（没有 runtime 聚合），在该 mode 下可用 `InMemorySessionStore`（HashMap）或 `FileSessionStore`（serde_json + fs）替代，功能等价性不变。
- **Deliverables**: 新增 xuanji-system persistence_traits.rs（1 file）；修改 ai-agent Cargo.toml + 2 源文件；修改 primiflow-core Cargo.toml + persistence.rs；runtime 聚合处把 xuanji-system SQLite 实现通过 feature/injection 传给两个 crate。
- **Test Requirements**:
  - **TR-05-01 (rule, AC-11)**: 16 crate Cargo.toml grep rusqlite → 仅 xuanji-system 1 crate 出现
  - **TR-05-02 (rule, AC-26)**: `cargo test -p ai-agent --test caomei_e2e` exit 0（功能回归）
  - **TR-05-03 (rule, AC-26)**: `cargo test -p primiflow-core` exit 0（功能回归）
  - **TR-05-04 (rubric, AC-31 NFR-RUST-05 复用)**: PersistenceProvider 抽象被 ≥ 2 处使用（ai-agent + primiflow-core）。score 0-2：2 = 用在 2 个 crate 以上；1 = 用在 1 crate；0 = 只定义不被使用。阈值 ≥1。

---

## Task 6: DIP 反转 - xuanji-system orchestrator 依赖 trait（非 concrete services）

- **Status**: pending
- **Priority**: HIGH
- **Objective**: 消除 xuanji-system A-01 违规：orchestrator 直接 use crate::services::*；改为 use 抽象 trait；services 实现这些 trait；保证 DIP 依赖反转。
- **Scope**:
  - `xuanji-system/src/domain_traits.rs`（新文件）：定义 `pub trait MemberService` / `pub trait TaskService` / `pub trait PermissionService` / `pub trait CommunicationService`。每个 trait 包含 orchestrator.rs 实际调用的方法签名（从 orchestrator 调用处反向采集）。
  - `xuanji-system/src/services.rs` / `src/services/` 中的具体实现：impl MemberService for MemberService 等（or 新建 type wrapper for impl）。
  - `xuanji-system/src/orchestrator.rs`：删除 `use crate::services::*;`，改为 `use crate::domain_traits::*;`；Orchestrator struct 持 `Arc<dyn MemberService + Send + Sync>` 等字段；在 constructor 中由 runtime 传入具体 impl。
  - 兼容保证：对外 pub API 不变（如 Orchestrator::new() 默认参数仍可构造，需要 default 构造时使用默认的 concrete impl Arc 包装）。
- **Test Requirements**:
  - **TR-06-01 (rule, AC-06)**: grep orchestrator.rs 无 `use crate::services::*;` 通配导入行
  - **TR-06-02 (rule)**: `cargo test -p xuanji-system` exit 0（领域测试 suite 全过，服务功能回归）
  - **TR-06-03 (rubric, AC-31 复用)**: 4 个 traits 被 orchestrator 使用，其中至少 2 个 trait 默认 impl 与 mock impl 可切换（用测试证明）。score 0-2：2 = 4 trait 都有 mock 测试；1 = 2 trait 有 mock；0 = 只有 real impl（无 mock 切换）。

---

## Task 7: DIP 反转 - operator-core L6 kernel/kernel_ext 双层

- **Status**: pending
- **Priority**: HIGH
- **Objective**: 把 operator-core 的核心模块（types/operator/state/resource）按 AIS 分为 L6 kernel（纯 std，零外部依赖）+ kernel_ext（加 serde/nalgebra derive 外层包装），违反 operator-core 对 L6 kernel 零外部依赖的 A-02。
- **Scope**:
  - 新建 `operator-core/src/kernel.rs`（或目录 kernel/mod.rs）：
    - 定义所有 **裸核心 struct**（无 `#[derive(Serialize, Deserialize)]`、无 nalgebra 引用）：如 `struct StateVector { data: Vec<f64> }` 用 std::Vec 而非 `nalgebra::DVector`（L6 层纯数据）。
    - 定义核心 trait：`pub trait OperatorKernel { fn apply(&self, state: &StateVector) -> Result<StateVector>; }` 纯 std。
    - 所有守恒律校验：纯函数，仅用 f64 + Vec 比较。
  - 新建 `operator-core/src/kernel_ext.rs`：
    - 包装层：`pub use kernel::*; pub use kernel_serde_impls::*;`
    - `kernel_serde_impls` 子模块：在 kernel struct 外加 Serialize/Deserialize（用 newtype 模式，或 feature gated impl）。
    - nalgebra 绑定：`fn to_nalgebra(v: &kernel::StateVector) -> DVector<f64>` 等。
  - 顶层 lib.rs 对外 pub 重新导出；`src/types.rs`、`src/operator.rs`、`src/state.rs` 等改为 use kernel 再（局部）use kernel_ext；`use serde`/`use nalgebra` 仅出现在 kernel_ext 及外层算法，不再出现在 kernel 核心层。
- **Deliverables**: 2 新文件 + N（现有 types/operator/state/resource/conservation 等）文件 edit。
- **Test Requirements**:
  - **TR-07-01 (rule, AC-07)**: grep operator-core/src/kernel*.rs 无 `use serde`、`use nalgebra`、`use thiserror`、`use anyhow`、`use tracing`、`use uuid`、`use ndarray` 行（0 条命中）
  - **TR-07-02 (rule)**: `cargo test -p operator-core` 所有 37 单元测试 + tests/integration_full.rs + tests/pipeline.rs exit 0
  - **TR-07-03 (rubric)**: L6 纯净度评估（scale 0-1）：kernel 模块内所有 pub fn 参数类型不引用外部 crate。1.0 = 100% 纯净；0.5 = 有 1 条；0 = ≥2。阈值 ≥1.0。

---

## Task 8: DIP 反转 - hermes-flow-bridge / business-catalog 对 xuanji-expert trait 化依赖

- **Status**: pending
- **Priority**: HIGH
- **Objective**: 消除 A-03：两 crate 直接 use xuanji_expert concrete struct（GovernContext / Principal / Tenant / xuanji_optimize fn）。改为 xuanji-expert 暴露抽象 traits，bridge/catalog 依赖 traits，在 runtime 聚合处做注入。
- **Scope**:
  - xuanji-expert 新增 `src/traits.rs`：
    ```rust
    pub trait GovernContextRead {
      fn tenant_name(&self) -> &str;
      fn principal(&self) -> (&str, &str); // id + role
      fn sensitivity_flags(&self) -> &[String];
    }
    pub trait XuanjiOptimize {
      type Output;
      fn optimize(&self, ctx: &dyn GovernContextRead) -> Result<Self::Output>;
    }
    ```
  - xuanji-expert 的现有 GovernContext / Principal / Tenant struct：impl GovernContextRead for GovernContext。
  - hermes-flow-bridge Cargo.toml：**改为 `xuanji-expert = { workspace = true, default-features = false, features = ["traits-only"] }`**（新建 feature gate traits-only，仅编译 traits.rs 不编译全部专家引擎）。若 feature 不可行则 bridge 改成把需要的函数作为 function pointer（`pub fn register_xuanji_optimize(f: fn(...) -> ...) -> Result<()>`）回调钩子，避免 concrete crate 依赖。
  - business-catalog 同 pattern。
  - runtime crate（聚合层）在 initialization 时 bridge/catalog 注册 concrete callbacks（把 xuanji_expert::xuanji_optimize 作为 fn 指针传入）。
- **Deliverables**: 1 新 traits.rs；2 crate Cargo.toml + 1 bridge.rs + 1 lib.rs 改造；runtime 聚合处新增 2 个 feature-gated injector。
- **Test Requirements**:
  - **TR-08-01 (rule, AC-08)**: `grep "xuanji_expert =" platform/services/hermes-flow-bridge/Cargo.toml business-catalog/Cargo.toml` → 2 处都为 `features = ["traits-only"]` 或不存在（改为 callback 钩子模式）。禁止直接默认 features 依赖。
  - **TR-08-02 (rule, AC-08)**: 两 crate 源码中 `use xuanji_expert::context` 或 `use xuanji_expert::GovernContext` concrete struct import 0 条（允许 `use xuanji_expert::traits`）。
  - **TR-08-03 (rule)**: `cargo test -p hermes-flow-bridge --test session_e2e` exit 0 + `cargo test -p business-catalog`（9 tests）exit 0
  - **TR-08-04 (rubric, AC-31 复用)**: GovernContextRead trait 同时被 bridge + catalog 两处使用。score 0-2：2=两处都通过抽象调用 + 注入；1=其中一处；0=依然 concrete struct。阈值 ≥1。

---

## Task 9: 500 深链 xuanji_optimize 性能回退修复（≤10 s）

- **Status**: pending
- **Priority**: HIGH
- **Objective**: `boundary_ultra_deep_chain_with_data_deps` 500 级深链 10,594 ms → ≤ 10,000 ms。
- **Scope**:
  - 先定位瓶颈：profile gap_p2_perf test（用 println! 粗粒度计时每个专家阶段）。
  - 根因假设（按可能性排序）：
    - (1) verify::tests 拓扑/代码生成类 verify 每节点都重新跑了，但 500 深链其实大部分是同构，可做 incremental verify_cache 复用 last 结果。
    - (2) 14 位专家 rayon 并行 → 每个专家对 ctx 做 to_owned() clone 500 次 → 改为 Arc 共享只读 ctx 按需 clone。
    - (3) reconcile 冲突调和：每节点调和矩阵计算 O(experts²)，在 500×14×14 可做剪枝（无冲突节点跳过调和矩阵）。
    - (4) 审计 S3 WORM 签名（每节点写入可能触发 SigV4 大字符串计算）→ 批量缓存或 only 1 个锚点 per 50 节点。
  - 按序尝试 (1)→(2)→(3)→(4)：一旦 ≤10 s 就停（禁止"优化过度"引入复杂度）。
- **Deliverables**: xuanji-expert/src/{verify/*, pipeline.rs, reconcile.rs, audit/*} 局部 patch；缓存结构体新增（cfg(test) 或实际可用均可，只要不破坏 prod 语义）。
- **Test Requirements**:
  - **TR-09-01 (rule, AC-18)**: `cargo test -p xuanji-expert --test gap_p2_perf_boundaries boundary_ultra_deep_chain_with_data_deps -- --nocapture 2>&1 | Select-String "500.*耗时"` → 数字 ≤ 10,000
  - **TR-09-02 (rule, AC-19)**: 其余 9 gap_p2_perf 测试结果全部 PASS
  - **TR-09-03 (rubric)**: 性能增益比（old/new 相对加速）：≥594ms 加速（=10594→10000 的必要加速量）。0-2：2=加速 ≥2000 ms；1=594~2000；0=<594。阈值 ≥1。

---

## Task 10: 架构文档三方对账 + Rust 绑定契约

- **Status**: pending
- **Priority**: HIGH
- **Depends**: T1 (project/owner 映射可用) + T2（CRATE_ID + CRATE_META.ais_layers 可用）
- **Objective**: §3.2 Rust 16 Crate 分层表 + §7.1 runtime 聚合分层 + project-atlas.md Rust 绑定契约全部落到文档中，保证文档 ↔ 代码 ↔ 注册表三方对账一致。
- **Scope**:
  - `docs/enterprise/02-architecture.md` 新建 §3.2 "Rust Workspace 16 Crate · AIS 分层表"：表格列（序号/crate 名/AIS 分层归属/核心职责 1 句/入口 codePath/图谱 engine node id/图谱 owner project id）。16 行填满。
  - `docs/enterprise/02-architecture.md` §7.1 "部署视图" 小节补充 "runtime crate 聚合架构：L1 routes/handlers/4 端点；L2 cordis(5 子模块)+rbac_middleware+subservers(11 crate 聚合)；feature gates(治理台/market/openapi)；二进制 operator-server 入口 main.rs"。
  - `docs/standards/project-atlas.md` 新增 §5 "Rust 端绑定契约"：
    - 字段：CRATE_ID kebab、CRATE_META.uuid 4 段 uuid、CRATE_META.ais_layers（L1-L7 数组）、CRATE_META.owner_project（项目 id）、CRATE_META.capabilities、CRATE_META.data_tables_read/write。
    - 同步规则：atlas self-sync 扫描 `platform/services/*/Cargo.toml + src/lib.rs pub const CRATE_META`（基于 once_cell/cargo metadata 命令），自补充 domain/engine/algorithm 节点 + owns_domain 边。
- **Deliverables**: 2 md 文件 edit（02-architecture.md + project-atlas.md）。
- **Test Requirements**:
  - **TR-10-01 (rule, AC-15)**: 02-architecture.md §3.2 表格中 crate 行数 = 16（逐行计数）
  - **TR-10-02 (rule, AC-16)**: 02-architecture.md §7.1 同时出现 "runtime"、"L1"、"L2"、"cordis"、"rbac_middleware"、"subservers" 关键词（6/6 齐全）
  - **TR-10-03 (rule, AC-17)**: project-atlas.md 含 "Rust" + "CRATE_ID" + "CRATE_META" 关键词（3/3 齐全）
  - **TR-10-04 (rule, B-04 对账)**: 16 crate 的文档分层声明 vs 实际 CRATE_META.ais_layers 完全一致（通过 Node 脚本读 md 表格 + 读取 16 lib.rs const 值比较 =16）

---

## Task 11: 14 crate README 项目化补全

- **Status**: pending
- **Priority**: MEDIUM
- **Depends**: T1（Rust domain/engine/algorithm 节点 id 可用）+ T2（CRATE_ID/CRATE_META 可用）
- **Objective**: 为 14 个缺失 crate 新建根 README.md；16 个 crate README 质量达到 rubric 准入阈值 ≥2.5（≥12 个 crate 3 分，其余 ≥2）。
- **Scope**:
  - 缺失 crate 列表（SPEC-A5 结果）：`runtime`、`operator-core`、`operator-wasm`、`graph-algorithms`、`optimizer`、`xuanji-expert`、`hermes-flow-bridge`、`business-catalog`、`ai-agent`、`template-market`、`xuanji-system`、`primiflow-core`、`kg-hub`、`business-catalog/bin` 的 crate 根目录——列表共 14。
  - README 标准模板（每文件都按 5 节）：
    1. **[crate-name] - 璇玑子项目**（标题 + AIS 分层归属徽章：`AIS Layer: L4-Core / L6-Kernel` 样式）
    2. **核心职责**（1-3 句）
    3. **公开 API**（摘要：Top 10 pub fn / pub struct / pub trait + 简短说明）
    4. **璇玑图谱绑定**（CRATE_ID、owner_project 节点 id、engine node id 列表、algorithm node id 列表）—— **3 分必备**
    5. **依赖**（使用的基础库列表）+ **测试命令**（`cargo test -p crate-name` + 具体集成测试）
- **Deliverables**: 14 个 crate 根目录 README.md（14 files write）。
- **Test Requirements**:
  - **TR-11-01 (rule, AC-12)**: crate 根目录存在 README.md 的数量 = 16（ls 16 crate 根）
  - **TR-11-02 (rule)**: 14 个新 README 都含标题行（# CrateName · 璇玑子项目）、核心职责段、API 列表段（至少 3 个 item）、依赖段、测试命令段（至少一条命令）—— ≥5 段齐全 = 每篇 README 达到 2 分。
  - **TR-11-03 (rubric, AC-28)**: README 质量评分（0-3 scale per crate）：≥12 crate 3 分（含图谱绑定 id）且其余 ≥2 分 = 总分 2.5 以上。人工评审 + 自动化关键词（"璇玑图谱绑定"+"CRATE_ID"+节点 id 出现）。

---

## Task 12: Rust/Node 7 核心算法对账二进制 + 测试

- **Status**: pending
- **Priority**: MEDIUM
- **Depends**: T3（export_formula.rs bin 已写）
- **Objective**: 把 T3 的对账脚本固化为独立 Node 测试，保证 future 变化自动检测算法漂移。
- **Scope**:
  - 新文件：`scripts/graph_algo_rust_node_harmonization_test.js`（或放 backend-node/test/test-graph-algo-cross-rust.js）
  - 流程：spawn `cargo run --bin export_formula --release` → 读取 stdout JSON → 与 Node 端 GraphFormulas 对应函数在同一 T1~T8 输入的结果做 Δ≤1e-6 断言 → PASS/FAIL
  - 如果 Rust bin 直接输出到 file，则改为 `cargo run ... -- --out /tmp/rust_formula_results.json` 再读。
- **Deliverables**: 1 新测试脚本（可独立跑 `node test/test-graph-algo-cross-rust.js`）。
- **Test Requirements**:
  - **TR-12-01 (rule, AC-05)**: 测试脚本 exit 0，所有 7 算法 × T1~T8 = 56 断言通过
  - **TR-12-02 (rubric)**: 对账覆盖率：实际比较 / 56 条目 = 100%。1=全；0.5=50%。阈值 1。

---

## Task 13: 构建全量回归（Build / Test / Clippy / Node 三套 / pub API）

- **Status**: pending
- **Priority**: MEDIUM
- **Depends**: T4 (deps) + T5 (rusqlite removal) + T6 (DIP orchestrator) + T7 (kernel) + T8 (trait bridge) + T9 (perf) 先完成，代码层面修改要稳定
- **Objective**: 把全仓 build/test/clippy + Node atlas/归一/EAF + pub API 无破坏 diff 全部机器验证，作为最终 Acceptance 前置。
- **Scope**:
  - Step 1: `cargo build --workspace` (release optional)
  - Step 2: `cargo test --workspace` 全量
  - Step 3: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | Select-String "error\["` → 0 new errors（相对 clippy_report4.txt baseline 做过滤计数）
  - Step 4: Node 三套：
    - `node test/test-project-atlas.js`
    - `node test/test-normalization-pipeline.js`
    - `node test/test-flow-registration.js`
  - Step 5: Pub API 无破坏 diff：写脚本对 16 crate 做 `cargo doc --workspace --no-deps` 生成 HTML，或手写 AST grep（`pub (struct|enum|trait|fn|const) <name>`）在本治理前后两份快照 diff，`removed` 集合空集。
  - Step 6: L4 框架依赖检查：AC-20 规定 operator-core/graph-algorithms/optimizer/flow-ai Cargo.toml 无 axum/sea-query/sqlx
- **Deliverables**: 终端输出（可存档报告，实际无需写文件）。
- **Test Requirements**:
  - **TR-13-01 (rule, AC-21)**: cargo build workspace exit 0
  - **TR-13-02 (rule, AC-22)**: cargo test workspace exit 0
  - **TR-13-03 (rule, AC-23)**: Node 三套 suite 全部 PASS
  - **TR-13-04 (rule, AC-24)**: Atlas W8 单一连通分量 = true
  - **TR-13-05 (rule, AC-20, AC-29 评估)**: L4 4 crate Cargo.toml grep axum|sea_query|sqlx → 0 条
  - **TR-13-06 (rule, AC-25)**: Pub API removed 符号集合 = 空集
  - **TR-13-07 (rule, AC-30 构建洁净度)**: clippy new ERROR 计数 ≤ 3
  - **TR-13-08 (rubric, AC-30)**: 构建洁净度打分：0-3 scale（按 spec 定义）。准入 ≥2.5。

---

## Task 14: AC-27~31 指标打分 + 全 AC 验收举证汇总

- **Status**: pending
- **Priority**: LOW
- **Depends**: T1-T13 全完成
- **Objective**: 汇集 26 rule + 5 rubric = 31 条 AC 的全部证据（TR 结果），形成总表，供 Review 阶段独立审阅。
- **Scope**:
  - T14 不写新代码。仅做结构化收集：
    - AC-27（Rust→图谱绑定）：统计 16 crate 四元组（engine/module node + algo node + codePath 存在 + owner 边）完整数
    - AC-28（README 质量）：16 crate 3 分 / 2 分计数 → 平均值
    - AC-29（框架依赖边界度）：4 L4 crate × 3 框架 grep 结果 → 是否全 0
    - AC-30（构建洁净度）：build/test/clippy 三项 → 对应分值
    - AC-31（复用潜力）：5 个任务的 DIP trait 使用处数量统计
  - 形成 `tasks.md` 末尾 "全 AC 证据汇总表" 增量章节（在 Implement 阶段每个任务完成后填充 Evidence）
- **Test Requirements**:
  - **TR-14-01 (rubric, AC-27)**: 绑定完备度 score ≥3.5
  - **TR-14-02 (rubric, AC-28)**: README 平均 ≥2.5
  - **TR-14-03 (rubric, AC-29)**: 框架依赖边界度 = 2
  - **TR-14-04 (rubric, AC-30)**: 构建洁净度 ≥2.5
  - **TR-14-05 (rubric, AC-31)**: 复用潜力 ≥1

---

## 验收覆盖矩阵（每条 AC → 对应 TR）

| AC-ID | 对应任务-测试需求条目 |
|---|---|
| AC-01/02/03 | T1 TR-01-01/02/03/04 |
| AC-04/05 | T3 TR-03-01/02/03 + T12 TR-12-01 |
| AC-06 | T6 TR-06-01 |
| AC-07 | T7 TR-07-01 |
| AC-08 | T8 TR-08-01/02 |
| AC-09/10 | T4 TR-04-01/02 |
| AC-11 | T5 TR-05-01 |
| AC-12/28 | T11 TR-11-01/02/03 + T14 TR-14-02 |
| AC-13/14 | T2 TR-02-01/02/03 |
| AC-15/16/17 | T10 TR-10-01/02/03/04 |
| AC-18/19 | T9 TR-09-01/02 |
| AC-20/29 | T13 TR-13-05 + T14 TR-14-03 |
| AC-21/22/30 | T13 TR-13-01/02 + TR-13-07/08 + T14 TR-14-04 |
| AC-23/24 | T13 TR-13-03/04 |
| AC-25 | T13 TR-13-06 |
| AC-26 | T5 TR-05-02/03 |
| AC-27 | T1 TR-01-05 + T14 TR-14-01 |
| AC-31 | T5-TR-05-04 + T6-TR-06-03 + T8-TR-08-04 汇总 + T14-TR-14-05 |

---

## 并发执行安全（Implement 阶段并行任务规则）

| 任务组 | 可并行 | 原因 |
|---|---|---|
| T1 + T2 | ✅ 可并发 | T1 改 JS/JSON registry；T2 改 Rust lib.rs const。无文件重叠 |
| T4 (deps) 与 T6/T7/T8 (各 crate DIP) | ✅ 可并发 | 4 组改不同 crate 文件，零重叠 |
| T9 (perf) vs T10 (docs) vs T11 (README) vs T12 (对账脚本) | ✅ 可并发 4 路 | 4 组完全不同文件/语言 |
| T5 (rusqlite removal) 与 T6 | ⚠️ 串行（都改 xuanji-system 源） | T5 加 persistence_traits.rs；T6 加 domain_traits.rs 都是 xuanji-system 新增不同文件 → 理论可并行但为保险串行 |
| T3 (算法对账) | 等待 T2 完 | 依赖 CRATE_ID 前缀 |
| T13 (全量回归) | 严格等待 T4-T9 | 依赖所有代码修复完成 |
| T14 (举证汇总) | 最后 | 依赖所有 TR 执行结果 |

最大并行度：**6 路（T1/T2/T4/T6/T9/T10 同时启动）**。

# 规格：璇玑系统 · Rust 后端全维度自研归一化（企业级 AIS 架构对齐 + 图谱双向绑定）

> 规格语言：中文 · 适用范围：`platform/services/*` 全 workspace 16 个 Rust crate + `platform/gateway/runtime` · 治理方式：璇玑知识图谱唯一底层中枢 · AIS 自主自研分层范式（DIP 依赖反转）

---

## 1. 问题与目标（Specify 基线）

### 1.1 客观问题（来自 SPEC-1~3 三维事实扫描）

| 类别 | 问题编号 | 事实陈述 | 量化 |
|---|---|---|---|
| **P0 性能边界失败** | P-01 | `boundary_ultra_deep_chain_with_data_deps` 500 级深链 10,594 ms 超预算 10,000 ms | 1 测试失败，占 gap_p2_perf 10% |
| **AIS 架构对齐（DIP 依赖反转）** | A-01 | `mox-system/orchestrator.rs (L2)` 直接 `use crate::services::*`，未通过 domain 抽象 trait 反转依赖 | 1 处反向依赖 |
| AIS 架构对齐（L6 Kernel 污染） | A-02 | `operator-core/types.rs + lib.rs (L6_KERNEL)` 顶部直接依赖 serde / nalgebra / ndarray / thiserror / anyhow / tracing / uuid 等 7 个外部 crate，违反 AIS L6 仅 std 规则 | 7 外部依赖进 L6 |
| AIS 服务拆分（同级直接结构体耦合） | A-03 | `hermes-flow-bridge/src/bridge.rs` 直接 `use mox_expert::{mox_optimize, GovernContext}` 具体结构体；`business-catalog/src/lib.rs` 直接 `use mox_expert::context::{GovernContext, Principal, Tenant}` 具体 struct，均未通过抽象 trait | 2 处同级硬耦合 |
| AIS 依赖治理（版本漂移） | A-04 | ① `primiflow-core` dev-deps reqwest 固定 0.11，workspace 统一 0.12 漂移；② `mox-expert / hermes-flow-bridge / business-catalog` 三 crate 未用 workspace=true 继承依赖（serde/tokio/thiserror 等 8+ 项硬编码版本号）；③ `primiflow-fusion` criterion 配置与 operator-core 不一致 | 3 类版本漂移 |
| AIS 依赖治理（框架依赖扩散） | A-05 | 框架级持久化依赖 `rusqlite` 仅允许在 Infra 实现层（mox-system），实际 **扩散到 L3 业务域 ai-agent**（Cargo.toml:24 rusqlite workspace=true）与 **L4 核心域 primiflow-core**（Cargo.toml:16 rusqlite workspace=true） | 2 crate 违规扩散 |
| AIS 工程目录约束（README 缺失） | A-06 | 16 crate 仅 `flow-ai` + `primiflow-fusion` 2/16 有 crate 根 README.md，14 缺失。AIS 要求每个 crate = 独立小项目，必须有自述。 | 12.5% 覆盖率，缺 14 份 |
| **璇玑图谱绑定（零双向映射）** | B-01 | 16 Rust crate 在 project-atlas 三注册表（business/engine/algorithm-registry.js）出现次数 = 0。所有 codePath 全为 `backend-node/src/*.js`（Node 域），完全覆盖不到 Rust 端 | 16/16 零绑定 |
| 图谱算法单源漂移 | B-02 | 登记在 tech-registry.js 的 6 条关键算法（CNM/PageRank/Brandes/Harmonic/度/密度）均宣告 `singleSource=true`，但 Rust `graph-algorithms/src/lib.rs` 中存在完全相同的 6 条算法独立实现，registry 完全未登记 → singleSource=false 真实态 | 6 算法双实现漂移 |
| 图谱动态绑定缺失 | B-03 | `atlas_auto_registry.json` 动态层 50 条目全为文档+数据资产，Rust crate 条目 = 0。self-sync 自同步机制未覆盖 Rust 文件树扫描 | 动态层 0 绑定 |
| 架构文档代码不一致 | B-04 | `docs/enterprise/02-architecture.md` 仅描述 mox-system 单 crate 的 AIS 5 层，其余 15 个 crate 的分层、职责、边界在企业架构文档中完全缺失 | 1/16 文档覆盖率 |
| 代码注册→图谱发现断链 | B-05 | 16 Rust crate 无统一 `pub const ENGINE_NAME`、`pub const CRATE_UUID` 等显式注册常量，project-atlas 自同步无法基于稳定标识匹配；代码侧已存在 4 套注册表（OperatorRegistry/CRATE_NAMES/SeamRegistry/ProviderRegistry），图谱侧 0 对应节点 | 4 代码注册表未映射 |
| 构建规模与自研度 | G-01 | 16 crate · 63,783 行 Rust · 0 PLACEHOLDER · 10 个含自研算法 · 551 个 total tests（508 单元 + 集成）——整体自研健康度高，但无 AIS 工程化 + 图谱化的系统治理 | 事实基线 |

### 1.2 目标用户

| 用户 | 关注点 |
|---|---|
| **开发专家联盟** | AIS 分层对齐、DIP 依赖反转、版本统一治理、README 项目化（可审阅、可审计） |
| **算法联盟** | 算法 singleSource=true 真实性验证、CNM/PR/Brandes/harmonic Rust→图谱单源绑定、性能深链 >10s 修复 |
| **企业交付方** | 16 crate = 16 个独立小项目完整可追踪；代码 100% 图谱双向绑定；架构文档 ↔ 代码 ↔ 注册表三方一致 |
| **璇玑治理引擎** | 所有架构/算法/依赖/框架违规可通过 `GET /atlas/verify` 复验；Rust crate 自动进入项目归属（owns_domain 边） |
| **运维/SRE** | 统一依赖版本、框架依赖不扩散、独立 README 便于模块级排障和替换 |

### 1.3 非目标（Out of Scope，边界需求对齐 §二分类矩阵边界列）

- ❌ **不涉及**：Node.js backend-node 域的业务逻辑 / 路由 / 算法实现（已在上一轮 G1-G7 治理）
- ❌ **不涉及**：前端 Vue3 UI 功能开发与视觉改动
- ❌ **不涉及**：多租户策略分层实现、WAL 事件重放、ABAC 增强、WASM 插件热加载 GUI
- ❌ **不涉及**：将 `sea-query`、`sqlx` 从 mox-system 替换为自研查询构建器（AIS 允许 Infra 层用方言库，仅禁止扩散）
- ❌ **不涉及**：crate 间大规模拆分或功能重定位（禁止为了 DIP 完美而大范围重构，只允许加 trait 抽象 + 替换 use 导入 + 新增 adapter 薄层）

---

## 2. 功能需求（Functional Requirements）

### FR-RUST-01 璇玑图谱·Rust 工程实体全量入库

- **rule**：project-atlas registry（business-registry + tech-registry + engine-registry）至少登记 16 条 Rust 独立 crate 条目（含 codePath 绝对磁盘路径存在）。当前 0 条 → 修复后 ≥16 条。
- **rule**：`atlas_auto_registry.json` 动态层新增 Rust crate 绑定条目 ≥16 条，`GET /atlas/verify` W1 域存在性验证 Rust 域 100% PASS。
- **rule**：Rust crate 全部归属于现有 8 个 P1-P8 项目节点，owns_domain 边 `GET /atlas/verify` W10（项目唯一归属）100% PASS。
- **rubric**：Rust→图谱绑定完备度（scale 0-4）：0=0条；1=<8 条；2=8~15 条；3=16+ 条但缺算法映射；4=16 条全含 engine/algorithm/codePath/owner + 三方对账一致。**准入阈值 ≥3.5**。

### FR-RUST-02 算法单源真实性校验（singleSource=true 落地）

- **rule**：对于 Rust 存在 + tech-registry 登记过的 PageRank/CNM/Brandes介数/harmonic紧密/度中心性/模块度/密度 7 条算法，registry 必须把 Rust 实现路径作为 `primary_impl_codePath`（首选实现）**或**明确声明 `co_impl_codePaths`（多实现）+ `primary_impl` 字段说明哪端优先；同时修正 `singleSource=true` 为 `true`（仅单一实现）或 `false`（双实现，有主从）。**禁止再出现"登记 singleSource=true 但实际 2 端均有独立实现"**。
- **rule**：Rust graph-algorithms `pagerank_personalized`（推模型转置图）与 Node.js `ai-integration-engine.js` `computePersonalizedPageRank`（推模型转置图）算法数学等价性用同一组 T1~T8 公式测试数据集对账，|Rust 结果 − Node 结果| ≤ 1e-6（同算法同输入同输出）。

### FR-RUST-03 AIS 分层 DIP 依赖反转（反向依赖消除）

- **rule**：A-01 消除：`mox-system/orchestrator.rs` 改为依赖某域 trait（`trait MemberService` / `trait TaskService` / `trait PermissionService`）而非 `use crate::services::*` 通配。可在 `domain/traits.rs` 新增抽象或直接 `pub trait` 于 model/services.rs 顶层，orchestrator 仅 import trait，具体实现注入（构造参数 Arc<dyn Trait> 或全局 fn 指针）。
- **rule**：A-02 消除：operator-core 的 L6 级核心模块（types/operator/state/resource，由 AIS 扫描标记为 L6_KERNEL）必须 **不直接依赖外部 crate**。两种合规解之一即可：
  - （a）抽纯核心 `kernel-core` 子 crate（新，无外部依赖），operator-core 变成在其上层加 serde/nalgebra 绑定；或
  - （b）在 operator-core 内新增 `kernel/` 模块（非 pub，内部 `use super::*` 仅 std），types/operator/state/resource 所有带 serde::Serialize/Deserialize derive 的包装 struct 放 `kernel_ext/` 外层模块（带 serde feature 才 pub）。
  - **选方案 (b)**：避免拆 crate 造成 workspace 依赖治理增量。验证：`operator-core/src/kernel/*.rs` 顶部无任何 `use serde`/`use nalgebra`/`use thiserror` 外部引用。
- **rule**：A-03 消除：hermes-flow-bridge / business-catalog 两 crate 不得直接 `use mox_expert::*` 具体 struct。新增 `mox-expert` 的 `src/traits.rs`（或已有 traits 模块）暴露 `pub trait Optimize`、`pub trait GovernContextRead`、`pub trait PrincipalLike`；bridge/catalog 改为依赖这些 trait（通过泛型参数 `<C: GovernContextRead>` 或 Arc<dyn Trait>），具体实现在 runtime（聚合层）做 feature-gated 注入或通过 bridge `register_mox_optimize_fn(fn)` 钩子回调。

### FR-RUST-04 依赖治理（版本统一 + 框架依赖不扩散）

- **rule**：A-04 消除：所有 Rust workspace crate 的 common dependencies（serde / serde_json / thiserror / anyhow / chrono / uuid / tokio / axum / tower-http / tracing / reqwest / rusqlite / sea-query / sqlx / petgraph / nalgebra / wasmer / rayon / criterion）100% 使用 `workspace = true` 继承。不存在任何 crate 直接硬编码版本字符串。验证：grep workspace `Cargo.toml` 与 16 crate `Cargo.toml` 所有依赖 3rd-party 行，version="x.y" 命中数 = 0。
- **rule**：reqwest / criterion / rand 等 dev-dependencies 同样一律使用 workspace=true + `features = [...]` 配置，不得在 crate 内写死版本与不一致的 default-features 设置。`primiflow-core` reqwest 升级到 workspace 统一 0.12。
- **rule**：A-05 消除：rusqlite 框架级依赖只允许在 mox-system（Infra 层 crate）出现。ai-agent 和 primiflow-core 两 crate 对 rusqlite 的 direct Cargo.toml 依赖必须移除。解法：新增 **`mox-system` 的 PersistenceProvider trait** + adapter，ai-agent / primiflow-core 通过 **`dyn PersistenceProvider` 抽象**进行 SQLite 操作（在 runtime 聚合时把 mox-system 的 SQLiteRepo impl 注入进去）；如果 ai-agent / primiflow-core 内部只在少数测试或单文件内使用 rusqlite 打开 db，可替换为 `std::fs + serde_json` 持久化（满足 AIS 禁止框架级持久化进 L3/L4）。

### FR-RUST-05 工程目录约束（16 crate 全项目化）

- **rule**：每个 crate 根目录存在 `README.md`（16/16）。README 必须含：crate 名称、AIS 分层归属（哪些 L1-L7）、核心职责（1-3 句）、公开 API 摘要、依赖列表、测试方法。14 个缺失 crate → 补 14 份。
- **rule**：16 crate README 内容真实可验证：宣称的分层归属 = AIS_LAYERING_SCAN 代码目录匹配结论（允许新增模块，但不允许 README 宣称 L1 结果却是 L4 纯算法）。
- **rubric**：README 质量（scale 0-3）：0=缺失；1=只有一句话占位；2=含分层+职责+API；3=含分层+职责+API+依赖+测试命令+代码路径关联图谱节点 id。**准入阈值 ≥2.5**（即 ≥12 个 crate 达 3 分，其余 ≥2）。

### FR-RUST-06 显式引擎注册范式（Rust → 图谱自动发现）

- **rule**：每个 Rust crate 至少新增 `pub const CRATE_ID: &str = "<kebab-case-crate-name>";` 与 `pub const CRATE_META: CrateMeta`（结构化元信息：uuid、分层、owner-project、capability 列表、数据读写表）。16/16 crate 均须拥有。
- **rule**：4 处代码内注册表（OperatorRegistry in operator-core、CRATE_NAMES in primiflow-fusion、SeamRegistry in runtime、ProviderRegistry in ai-agent）必须在 tech-registry.js / business-registry.js 中有对应 engine/algorithm/domain 节点，实现代码 `CRATE_ID` ↔ 图谱 node.id 双向稳定匹配。

### FR-RUST-07 架构文档三方对账一致

- **rule**：`docs/enterprise/02-architecture.md` 补充 §3.2 **Rust Workspace 16 Crate AIS 分层表**：每 crate 列出所属层（L1-L7）、核心职责、顶层 codePath、引擎/算法 id 绑定。crate 覆盖率 16/16 = 100%。
- **rule**：§7.1 部署视图补充 runtime crate 的具体 L1 routes/handlers + L2 cordis/event_bus/rbac_middleware/subservers 四层聚合架构描述，不得再以"runtime 主服务聚合各 crate"一句话概括。
- **rule**：`docs/standards/project-atlas.md` 的 §Rust 端绑定（如无该节则新增），说明 Rust crate_id / crate_meta 常量 → project-atlas 自同步机制的契约字段。

### FR-RUST-08 性能边界修复（500 深链 ≤10 s）

- **rule**：`mox-expert/tests/gap_p2_perf_boundaries.rs::boundary_ultra_deep_chain_with_data_deps` 测试通过：500 级深链（带真实数据依赖）的 mox_optimize 耗时 ≤ 10,000 ms。验证：`cargo test -p mox-expert --test gap_p2_perf_boundaries boundary_ultra_deep_chain_with_data_deps` exit 0。
- **rule**：其他 9 条 gap_p2_perf 测试不得回退（全部仍通过）。

---

## 3. 非功能需求（Non-Functional Requirements）

### NFR-RUST-01 自研度（AIS 杜绝重度框架）

- **rule**：重应用框架类依赖仅在允许的边界内：
  - Axum (Web 框架)：仅 runtime / mox-system / primiflow-core / primiflow-fusion / mox-expert 的 L1 HTTP server 入口文件允许；L4 算法层 crate（operator-core/graph-algorithms/optimizer/flow-ai）Cargo.toml 不得出现 axum。
  - sea-query / sqlx：仅 mox-system（Infra 持久化层）Cargo.toml 允许；其他 15 个 crate 不得出现。
  - rusqlite：仅 mox-system 允许（FR-RUST-04 已覆盖）。
- **rubric**：框架依赖边界度（scale 0-2）：0=任何 L4 crate 含 axum；1=仅 1 处违规；2=100% 边界合规（L1/5 允许的 HTTP/持久化框架仅在对应 crate，L2/3/4/6 零框架）。**准入阈值 = 2**。

### NFR-RUST-02 构建稳定性

- **rule**：`cargo build --workspace` exit 0；`cargo test --workspace` exit 0（在修复 P-01 后）。
- **rule**：`cargo clippy --workspace --all-targets -- -D warnings` 无新引入的 ERROR（允许 warning，但不得 ERROR）。上一治理周期 clippy_report4.txt 基线作为基线；本周期不得新增 ERROR。
- **rubric**：构建洁净度（scale 0-3）：0=build fail；1=build ok, test fail；2=all ok, clippy ERROR 新增 ≤3；3=build ok + test workspace 0 failed + clippy ERROR 0 新增。**准入阈值 ≥2.5**。

### NFR-RUST-03 图谱复验可治理性

- **rule**：本规格所有 FR（01~08）完成后，`node test/test-project-atlas.js` 中的 W1-W13 全部 PASS（不得新增 W 破窗）。
- **rule**：`node test/test-normalization-pipeline.js`（59 条）与 `node test/test-flow-registration.js`（37 条）全 PASS。
- **rule**：璇玑图谱新增节点与边后必须保持全图单一连通分量（W8 PASS）。

### NFR-RUST-04 向后兼容性

- **rule**：所有 Rust 现有 public API 不得破坏性变更（pub fn / pub struct / pub trait 的签名不得删除或修改参数列表）。新增 trait / 新 const / 新 adapter / feature-gated injection 全是可叠加的兼容增量。
- **rule**：`rusqlite` 从 ai-agent / primiflow-core 移除后，原功能（如 ai-agent dialogue session 持久化、primiflow-core SQLite 输出）仍可在 runtime 聚合时通过 mox-system PersistenceProvider 注入获得等价行为。等价性：以 ai-agent 现有 tests `caomei_e2e.rs`、primiflow-core `tests/*integration*` exit 0 验证。

### NFR-RUST-05 可维护性与复用性

- **rule**：新增的 DIP trait 抽象（GovernContextRead / Optimize / PersistenceProvider / MemberService / TaskService / PermissionService）与 CRATE_META 常量必须有 `#[doc]` 说明其用途；覆盖率：新增 trait/const 文档 100%。
- **rubric**：复用潜力（scale 0-2）：0=每个新增抽象仅被 1 处使用；1=至少 1 个抽象被 2+ 处使用（证明复用性）；2=至少 2 个抽象被 2+ 处复用（DIP 抽象真正驱动解耦）。**准入阈值 ≥1**。

---

## 4. 约束（Constraints · 硬约束·来自 §二 企业级 + AIS 架构规范）

1. **AIS DIP 硬约束**：L6(Kernel) 禁止任何外部依赖（仅 std）；L4(Core) 禁止 L2/L1 Service/Handler 反向依赖；L3(Service) 禁止 L1 Ingress 反向依赖；同级业务 crate 禁止直接 use 具体 struct（必须通过 trait 抽象或聚合层注入）。
2. **框架依赖边界硬约束**：Axum/Sea-Query/Sqlx/Rusqlite 四类"应用框架/持久化框架"仅允许在指定的 L1(HTTP) 或 L5(Infra-Persistence) crate 内出现，**绝对禁止扩散进 L3 业务域 / L4 核心算法域**。
3. **璇玑图谱硬约束**：每 crate = 独立小项目 = 图谱 1+ 节点 = CRATE_ID 可发现 = codePath 真实可追踪；所有算法单源真实性（登记的 singleSource 必须与实际实现数一致）。
4. **版本统一硬约束**：所有 3rd-party crate 版本（含 dev-deps）100% 通过 workspace.dependencies 继承，不得写死版本字符串。
5. **性能硬约束**：500 深链数据依赖 mox_optimize ≤ 10 s（P-01）。
6. **全量测试硬约束**：修复前后 `cargo test --workspace` 必须 exit 0，且本规格 8 条 FR rule 全部有机器可执行断言证明。
7. **README 项目化硬约束**：16 crate 100% 有 crate 根 README.md，且内容真实匹配代码分层。
8. **架构文档硬约束**：架构文档 Rust 分层表 16/16 覆盖率，文档↔代码↔注册表三方对账一致。

---

## 5. 依赖与假设

- **依赖**：
  - backend-node 的 project-atlas self-sync 服务（可解析 JSON 配置并写入 registry）
  - Rust 1.75+ toolchain（含 cargo test / cargo clippy / cargo build）
  - 上一治理周期璇玑图谱 W1-W13 全绿基线（39/39 测试 PASS）
- **假设**：
  - 允许新增 `mox-expert/src/traits.rs` / `mox-system/src/domain_traits.rs` / `operator-core/src/kernel.rs` + `kernel_ext.rs` 等薄抽象层，不改变 crate 的顶层结构（lib.rs 导出不变）。
  - ai-agent / primiflow-core 的 rusqlite 使用量有限（< 5 处 open + query），可用 dyn PersistenceProvider 抽象替换；若有超大规模依赖，则在实现阶段退化为 feature-gated（默认关闭 rusqlite feature）。
  - P-01 的性能回退来自 mox-expert 的 4 类 verify 拓扑/代码生成等 O(n^2) 操作，可通过剪枝/缓存/并行（已用 rayon）之外的局部优化（例如减少 500 深链的无谓拓扑重算）达到 <10 s 的目标。

---

## 6. 开放性问题（如在 Approve 前无法回答，进入 Implement 后按如下默认处理）

| Q# | 问题 | 推荐默认方案 | 可选项 |
|---|---|---|---|
| Q1 | operator-core L6 拆 kernel/kernel_ext 模块 vs 新 `kernel-core` mini crate？ | **§FR-RUST-03 方案 (b)**：拆 operator-core 的内部子模块，避免 crate 增量治理 | 新建 kernel-core crate |
| Q2 | 对于 algorithm registry 的"单源声明 vs 双实现实际"，登记为 **co_impl（多实现、Rust=primary, Node=secondary）** vs **改 singleSource=true，把 Node 实现 deprecated 并直接调 Rust**（通过 napi-rs FFI，复杂度高）| **推荐**：登记为 co_impl（Rust primary + Node secondary），不改 FFI；同时保证数学等价性（§FR-RUST-02 rule2）| napi-rs 接入 |
| Q3 | ai-agent rusqlite 用途若超过"会话持久化小量 1-2 表"，**完全抽象 PersistenceProvider** 工作量大，是否允许保留 ai-agent `rusqlite = { workspace = true, optional = true, default-features = false }` 作为可选特性但默认不启用？ | 推荐：**完全移除依赖**，把持久化逻辑全部抽到 mox-system PersistenceProvider 抽象；若因实现困难退回可选 feature，必须在 README + AIS 违规表明确标注"非默认启用 + 框架级依赖隔离在 feature gating" | 可选特性兼容 |
| Q4 | 14 份 README 的详细程度要求？ §FR-RUST-05 rubric 阈值 2.5 → 至少 12 份 crate 达到"3 分 = 含图谱 node id 绑定" | 推荐：**按 rubric 执行**，确保 14/14 至少 2 分，12/14 达 3 分 | 降低门槛到 2.0 |

---

## 7. 验收标准（Acceptance Criteria · 只有 rule / rubric）

### Rule 类（可客观验证的二元条件，必须 100% PASS）

| AC-ID | 类型 | 条件 | 证据来源 |
|---|---|---|---|
| AC-01 | rule | FR-RUST-01 rule1：project-atlas registry Rust crate 条目数 ≥ 16 | `grep -c "kind.*rust-crate\|engine.*rust\|algorithm.*rust" src/project-atlas/domain/*.js` + `node test/test-project-atlas.js W1` |
| AC-02 | rule | FR-RUST-01 rule2：atlas_auto_registry.json Rust 条目 ≥ 16 | `node test/test-project-atlas.js` W1 域存在性全绿 + json 读取 |
| AC-03 | rule | FR-RUST-01 rule3：W10（项目唯一归属）PASS | `node test/test-project-atlas.js` |
| AC-04 | rule | FR-RUST-02 rule1：7 条核心算法 registry 声明 `primary_impl_codePath` 指 Rust graph-algorithms/lib.rs 对应绝对行范围（含 singleSource=false + co_impl） | 读取 tech-registry.js |
| AC-05 | rule | FR-RUST-02 rule2：Rust/Node 同算法同输入输出 |Δ| ≤1e-6 | 新建测试脚本 Rust:bin/graph_formula_export → Node 侧对照断言 |
| AC-06 | rule | FR-RUST-03 rule1：orchestrator.rs 无 `use crate::services::*` 通配，改为 use 抽象 trait | grep orchestrator.rs |
| AC-07 | rule | FR-RUST-03 rule2：operator-core/src/kernel/*.rs 顶部 0 条 `use serde`/`use nalgebra`/`use thiserror` 等外部依赖 | grep operator-core/src/kernel/*.rs |
| AC-08 | rule | FR-RUST-03 rule3：hermes-flow-bridge / business-catalog Cargo.toml 不出现 `mox-expert = ...` direct concrete dep（仅依赖 mox-expert 的 traits feature 或通过 abstraction 间接） | grep 两 crate Cargo.toml + Cargo.lock |
| AC-09 | rule | FR-RUST-04 rule1：所有 Rust crate 3rd-party dep 全部 workspace=true；写死版本字符串数 = 0 | 写检查脚本遍历 16 Cargo.toml |
| AC-10 | rule | FR-RUST-04 rule2：dev-deps reqwest/criterion 全部 workspace=true 无版本漂移 | 同上 |
| AC-11 | rule | FR-RUST-04 rule3：rusqlite 仅出现在 mox-system Cargo.toml（16 crate 中 1 处） | grep 16 Cargo.toml |
| AC-12 | rule | FR-RUST-05 rule1：16 crate 根目录 README.md 数 = 16 | ls 16 crate roots |
| AC-13 | rule | FR-RUST-06 rule1：16 crate 均有 `pub const CRATE_ID` + `pub const CRATE_META` | grep 16 crate src/lib.rs `pub const CRATE_ID` + call `CRATE_META.ais_layers` 非空 |
| AC-14 | rule | FR-RUST-06 rule2：4 处代码注册表 → 图谱有对应节点，CRATE_ID ↔ atlas node.id 匹配 | tech-registry.js grep + 运行时验证 |
| AC-15 | rule | FR-RUST-07 rule1：02-architecture.md Rust 分层表 crate 数 = 16 | grep §3.2 行数 |
| AC-16 | rule | FR-RUST-07 rule2：02-architecture.md §7.1 含 runtime L1+L2 具体分层描述 | grep runtime |
| AC-17 | rule | FR-RUST-07 rule3：project-atlas.md 含 Rust→图谱绑定契约 | grep project-atlas.md Rust crate_id |
| AC-18 | rule | FR-RUST-08 rule1：500 深链测试 exit 0 | `cargo test -p mox-expert --test gap_p2_perf_boundaries boundary_ultra_deep_chain_with_data_deps` |
| AC-19 | rule | FR-RUST-08 rule2：gap_p2_perf 其余 9 条测试仍 PASS | 同 workspace test |
| AC-20 | rule | NFR-RUST-01：L4 算法 crate（operator-core, graph-algorithms, optimizer, flow-ai）Cargo.toml 不含 axum / sea-query / sqlx | grep 4 crate Cargo.toml |
| AC-21 | rule | NFR-RUST-02：`cargo build --workspace` exit 0 | cargo build |
| AC-22 | rule | NFR-RUST-02：`cargo test --workspace` exit 0（全绿） | cargo test |
| AC-23 | rule | NFR-RUST-03：W1-W13 全 39/39 PASS + 归一 59/59 + EAF-STD 37/37 | 三套 JS 测试运行 |
| AC-24 | rule | NFR-RUST-03：全图单一连通分量（W8）无孤岛 | `node test/test-project-atlas.js W8` |
| AC-25 | rule | NFR-RUST-04：pub API 无破坏性变更（`cargo semver-checks` 或手写导出符号 diff 本治理前/后无缺失） | `cargo doc --workspace` build ok + 手工导出 symbol 对比 baseline |
| AC-26 | rule | NFR-RUST-04：ai-agent caomei_e2e.rs PASS + primiflow-core integration 测试 suite PASS | `cargo test -p ai-agent --test caomei_e2e` + `cargo test -p primiflow-core` |

### Rubric 类（评价维度，必须达到准入阈值）

| AC-ID | 类型 | 维度 | 量表 | 准入阈值 | 证据 |
|---|---|---|---|---|---|
| AC-27 | rubric | Rust→图谱绑定完备度 FR-RUST-01 | 0-4 | ≥3.5 | 注册表 16 crate × 含 engine/algorithm/codePath/owner 四元组 |
| AC-28 | rubric | README 质量 FR-RUST-05 | 0-3 | ≥2.5 | 16 README 评审 |
| AC-29 | rubric | 框架依赖边界度 NFR-RUST-01 | 0-2 | =2 | 16 Cargo.toml × 4 框架边界审核 |
| AC-30 | rubric | 构建洁净度 NFR-RUST-02 | 0-3 | ≥2.5 | build/test/clippy 结果 |
| AC-31 | rubric | 复用潜力 NFR-RUST-05 | 0-2 | ≥1 | 新增 DIP trait × 使用处数量统计 |

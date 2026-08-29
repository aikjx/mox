# 任务队列：璇玑 mox v4 · 全维度企业级架构全功能开发

> 语言：中文 · Spec: [spec.md](file:///d:/a10/aikjx/gitcode/infotopograph/.trae/specs/20260823-mox-full-enterprise-architecture/spec.md) · 治理：TDD RED→GREEN + 每任务完成后图谱反向同步（6 层节点+边）

---

## 任务摘要总览

| 任务 | 标题 | 优先级 | 依赖 | 覆盖 AC | 状态 |
|---|---|---|---|---|---|
| **T1** | Rust 16 crate 入璇玑三注册表 + 动态层 self-sync 扫描 | HIGH | — | AC-01/02 | pending |
| **T2** | 16 crate CRATE_ID + CRATE_META + ENGINE_NAME 常量 | HIGH | — | AC-02 | pending |
| **T3** | 7 核心算法 singleSource=true + Rust/Node Δ≤1e-6 对账 | HIGH | T1/T2 | AC-03/05 | pending |
| **T4** | 依赖治理 100% workspace 继承（漂移归零） | HIGH | — | AC-09/10 | pending |
| **T5** | rusqlite 收拢 mox-system 单 crate（ai-agent / primiflow-core 移除 + PersistenceProvider trait） | HIGH | T2 | AC-08 | pending |
| **T6** | DIP 反转 mox-system orchestrator → L5 Member/Task/Permission trait | HIGH | — | AC-06 | pending |
| **T7** | DIP 反转 operator-core L6 kernel/kernel_ext 双层（kernel 0 extern crate） | HIGH | — | AC-04 | pending |
| **T8** | DIP 反转 hermes / business-catalog → mox-expert 抽象 trait | HIGH | — | AC-07 | pending |
| **T9** | 500 深链性能修复（P99≤10,000 ms，拓扑剪枝 + memo） | HIGH | — | AC-16 | pending |
| **T10** | 架构文档三方对账（02-architecture.md 16 crate 分层表 + project-atlas.md Rust 绑定契约） | HIGH | T1/T2 | FR-16 | pending |
| **T11** | 14 crate README 补全（16/16 覆盖率） | MEDIUM | T1/T2 | AC-17/AC-24 | pending |
| **T12** | 核心算法对账 7 条二进制 + Node 测试（Δ≤1e-6） | MEDIUM | T3 | AC-05 | pending |
| **T13** | `/ai/engine/workflow/execute` + 3 内置 workflow + step 图谱写回 | HIGH | — | AC-11/12 | pending |
| **T14** | 企业级 3 端点：/atlas/verify + /atlas/health/enterprise + /atlas/governance/audit | HIGH | — | AC-13/14/15/AC-26 | pending |
| **T15** | 全量回归（cargo build/test/clippy + Node 12 suites + Rust 3 suites + 精度护栏 + 路由语义 + rubric 打分汇总） | HIGH | T1~T14 | AC-18~26 | pending |

---

## Task 1: Rust 16 crate 入璇玑三注册表 + 动态层 self-sync 扫描

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: —
- **ACs Covered**: AC-01, AC-02

### 范围（Scope）
1. 扫描 `platform/services/*/Cargo.toml` + `platform/gateway/runtime/Cargo.toml` — 合计 17 个 Rust 包（services 16 + gateway 1）；SPEC 注册 16 crate 指 services 层，gateway 另外算 infra tier
2. 写入 `project-atlas/registries/business-registry.js`：每个 crate = 1 business 域条目
3. 写入 `project-atlas/registries/engine-registry.js`：每个 crate = 1 engine 条目，`engineName = ENGINE_NAME 常量`
4. 写入 `project-atlas/registries/algorithm-registry.js`：graph-algorithms crate 单独写入 7 条核心算法（main=rust，co_impl=[node:GraphFormulas]）
5. 扩展 `project-atlas/scripts/self_sync.js`（或新建 `project-atlas/scripts/self_sync_rust.js`）：递归扫描 `platform/services/**/*.rs`，产出 `atlas_auto_registry_rust.json`，按 CRATE_ID 匹配

### TRs（Test Requirements）

#### rule
- **TR 1.1**: `business-registry.js` 中 Rust 条目数 === 16；每条 has `kind='rust'`, `codePath` 指向 `platform/services/<crate>/` 绝对路径
- **TR 1.2**: `engine-registry.js` 中 Rust 条目数 === 16；`engineName` 与对应 crate `pub const ENGINE_NAME` 值一致（T2 后可最终验证）
- **TR 1.3**: `algorithm-registry.js` 中 CNM/PageRank/Brandes/harmonic/degree/density/RAW_EXPAND 7 条含 `singleSource=true`, `main='rust'`, `co_impl=['node:GraphFormulas']`
- **TR 1.4**: 运行 self_sync_rust.js 后，`atlas_auto_registry_rust.json` entries ≥ 16 crate + ≥ 100 rust 文件

### rubric
- **TR 1.5**（SCORE 0-2, THRESHOLD 2）：三注册表条目字段完整度（codePath / owns_domain / version / tags）。0=缺 2 项以上 · 1=缺 1 项 · 2=无缺
- **证据源**：`test-rust-registry.js` + diff

### Completion Evidence
（实施完成后填写：测试输出 + 文件路径）

---

## Task 2: 16 crate CRATE_ID + CRATE_META + ENGINE_NAME 常量

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: —
- **ACs Covered**: AC-02

### 范围
1. 每个 `platform/services/<crate>/src/lib.rs` 顶部新增：
```rust
pub const CRATE_ID: &str = "uuid v5(ns=DNS, name=crate.rs.mox.infotopograph/<name>)";
pub const ENGINE_NAME: &str = "mox::<name>";
pub const CRATE_META: CrateMeta = CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: AisLayer::L4Services, // 按 crate 真实层级填 L4/L5/L7
    owner: "mox-core",
};
```
2. operator-core 的 CRATE_META.layer = L6Kernel（kernel）+ L6KernelExt（kernel_ext）两层
3. mox-system.layer = L7Infrastructure

### TRs
#### rule
- **TR 2.1**: 每个 lib.rs 含 `pub const CRATE_ID: &str` / `ENGINE_NAME` / `CRATE_META`；16/16
- **TR 2.2**: 所有 CRATE_ID uuid v5 **互异**；`ENGINE_NAME` = `mox::<crate 目录名>`（下划线=短横线一致）
- **TR 2.3**: `crate_meta_lookup()` 在 `mox_common_meta`（新建 crate：platform/services/mox-common-meta）中返回 16 crate 记录

### rubric
- **TR 2.4**（0-2, threshold 2）：16 个 layer 标签对应 SPEC 2.1 分层正确性。0=>5 错 · 1=1~4 错 · 2=全对

---

## Task 3: 7 核心算法 singleSource=true + Rust/Node Δ≤1e-6 对账

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: T1, T2
- **ACs Covered**: AC-03, AC-05

### 范围
1. graph-algorithms crate 为 7 算法的主实现；每个对外：
```rust
pub fn community_detection_cnm(...) -> CommunityResult;
pub fn personalized_page_rank(...) -> PPRResult; // d=0.85 maxIter=30 硬编码 const
pub fn brandes_betweenness(...) -> BtwResult;
pub fn harmonic_closeness(...) -> ClsResult;
pub fn degree_centrality(...) -> DegResult;
pub fn graph_density(...) -> f64; // NO toFixed
pub fn raw_bidirectional_expand(edges: &[RawEdge]) -> Vec<Edge>; // 双向展开
```
2. Node 端 `GraphFormulas` 仅保留：① 输入标准化；② 调用 `child_process::spawn` 或 FFI 调用 Rust 二进制 `mox-graph-algos-cli`（新建二进制 target）；③ 输出标准化；核心循环完全移除
3. LPA 对外出口继续禁用（graph-algos.js 中 `detectCommunitiesLPA` 抛 deprecation error + 仅 CNM 可用）

### TRs
#### rule
- **TR 3.1**: 输入 SPEC-4 固定 fixture（45 nodes 55 edges + seeds + d=0.85 + 30 iter），Rust/Node 7 算法输出逐行 diff：`max |v| ≤ 1e-6`
- **TR 3.2**: Node GraphFormulas.js 中无算法 for/while 主循环（仅保留 FFI 调用 + 输入输出归一）
- **TR 3.3**: `curl GET /atlas/internal/registry-algos` → 7 条 `{singleSource:true, main:'rust'}`

### rubric
- **TR 3.4**（0-2, threshold 2）：性能对比 Rust vs Node（同 fixture）。0=Rust 不比 Node 快 · 1=快 10%~50% · 2=快 ≥ 50%

---

## Task 4: 依赖治理 100% workspace 继承

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: —
- **ACs Covered**: AC-09, AC-10

### 范围
1. 根 workspace Cargo.toml `[workspace.dependencies]` 汇总所有公共依赖版本
2. 16 crate Cargo.toml 所有依赖写 `dep = { workspace = true, features = [...] }`；禁止 `dep = "x.y.z"` 硬编码（除了必须独立的二进制，≤ 1 例外且文档化）
3. 修复 `primiflow-core` dev-deps reqwest 漂移（0.11 → workspace 0.12）
4. criterion 配置工作区统一（primiflow-fusion 与 operator-core 对齐）

### TRs
#### rule
- **TR 4.1**: `grep -r '=' platform/services/*/Cargo.toml | grep -v 'workspace = true' | grep -v '\[package\]' | grep -v 'name\|version\|edition\|authors\|repository\|license' | wc -l` ≤ 1（例外）
- **TR 4.2**: cargo tree 中 reqwest 所有实例的 major.minor 相同（0.12.x）

### rubric
- **TR 4.3**（0-2, threshold 2）：依赖版本一致性审查。0=>3 crate 不同版本 · 1=1~2 · 2=全同

---

## Task 5: rusqlite 收拢 mox-system 单 crate

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: T2
- **ACs Covered**: AC-08

### 范围
1. L5 新建 `pub trait PersistenceProvider`（mox-common-meta 或新 crate `mox-domain-abstractions`）：
```rust
pub trait PersistenceProvider {
    fn save_member(&self, m: &Member) -> Result<()>;
    fn load_member(&self, id: &str) -> Result<Option<Member>>;
    // ... 其它 10 个 CRUD
}
```
2. ai-agent Cargo.toml 移除 rusqlite；改为依赖 `PersistenceProvider` 抽象
3. primiflow-core Cargo.toml 移除 rusqlite；同上
4. mox-system 独一的 `RusqlitePersistenceProvider implements PersistenceProvider`

### TRs
#### rule
- **TR 5.1**: `grep 'rusqlite' platform/services/{ai-agent,primiflow-core}/Cargo.toml` 0 匹配
- **TR 5.2**: ai-agent 与 primiflow-core 单元测试，注入 `MockPersistenceProvider`（不依赖磁盘），全 GREEN
- **TR 5.3**: mox-system RusqlitePersistenceProvider 集成测试 20 条 CRUD GREEN

---

## Task 6: DIP 反转 mox-system orchestrator → L5 trait

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: —
- **ACs Covered**: AC-06

### 范围
1. 在 L5 crate（mox-domain-abstractions）定义：
```rust
pub trait MemberProvider { fn get(&self, id: &str) -> Result<Option<Member>>; }
pub trait TaskProvider   { fn list(&self, filter: &TaskFilter) -> Result<Vec<Task>>; }
pub trait PermissionProvider { fn check(&self, p: &Principal, a: Action, r: &Resource) -> Result<bool>; }
```
2. mox-system orchestrator.rs：将 `use crate::services::{MoxMember, MoxTask, PermissionService}`（具体 struct）全部替换为 `use crate::domain::{MemberProvider, TaskProvider, PermissionProvider}`；结构体字段改为 `Box<dyn MemberProvider>`

### TRs
#### rule
- **TR 6.1**: orchestrator.rs 的 use 行中无具体 struct；仅 trait
- **TR 6.2**: 单测注入 `Mock*Provider`，10 条编排用例 GREEN

---

## Task 7: DIP 反转 operator-core L6 kernel/kernel_ext 双层

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: —
- **ACs Covered**: AC-04

### 范围
1. operator-core 目录重构：
```
operator-core/src/
  kernel/mod.rs           // ✅ 仅 use std::*; 0 external crate
  kernel_ext/mod.rs       // serde/nalgebra/ndarray 等 wrapper：use crate::kernel::*; use serde::*;
  lib.rs                  // pub use kernel::*; pub use kernel_ext::*;
```
2. types.rs 中所有类型定义拆：
   - 纯数据结构 + 纯运算 → kernel/
   - 需要 serde Serialize/Deserialize / thiserror / anyhow / tracing / uuid / nalgebra → kernel_ext/

### TRs
#### rule
- **TR 7.1**: `grep -E '^\s*use (serde::|nalgebra::|ndarray::|thiserror::|anyhow::|tracing::|uuid::)' operator-core/src/kernel/mod.rs` **0 匹配**
- **TR 7.2**: operator-core 508 单测 + 集成全部 GREEN

### rubric
- **TR 7.3**（0-2, threshold 2）：拆分完整性：kernel = 纯类型+算法，kernel_ext = 所有 serde/第三方 impl。0=>5 类型错放 · 1=1~4 · 2=无错放

---

## Task 8: DIP 反转 hermes-flow-bridge / business-catalog → mox-expert 抽象 trait

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: —
- **ACs Covered**: AC-07

### 范围
1. mox-expert `src/domain/` 新增：
```rust
pub trait GovernExpert {
    fn optimize(&self, ctx: &dyn GovernContext) -> Result<GovernReport>;
}
pub trait GovernContext {
    fn principal(&self) -> &Principal;
    fn tenant(&self) -> &Tenant;
    fn params(&self) -> &BTreeMap<String, String>;
}
```
2. hermes-flow-bridge：将 `use mox_expert::mox_optimize; use mox_expert::context::GovernContext;`（concrete）替换为 `use mox_expert::domain::{GovernExpert, GovernContext};`（trait）；注入 `Arc<dyn GovernExpert>`
3. business-catalog：同上，只 depend GovernContext trait，不依赖具体 struct

### TRs
#### rule
- **TR 8.1**: grep hermes / business-catalog lib.rs 无 `use mox_expert::mox_optimize`（concrete fn）；只有 trait use
- **TR 8.2**: MockGovernExpert 单测 10 条 GREEN

---

## Task 9: 500 深链性能修复（P99≤10,000 ms）

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: —
- **ACs Covered**: AC-16

### 范围
1. `mox_optimize` 500 深链算法：增加 2 个优化
   - **拓扑剪枝**：依赖环检测，已访问节点结果 memoization；重复子图 1 次评估
   - **目标感知剪枝**：若子图已满足所有 objectives（constraints fully satisfied），停止深度展开
2. `RUSTFLAGS=-C target-cpu=native` 编译；使用 `ahash` HashMap（替换 std HashMap，在 kernel_ext/）
3. CEM 停止条件继续采用：`σ̄<0.06` || `连续 3 轮 no improvement`（SPEC-7 T7 baseline）

### TRs
#### rule
- **TR 9.1**: 运行 `test_boundary_ultra_deep_chain_with_data_deps` 100 次，P99 ≤ 10,000 ms
- **TR 9.2**: 100 次运行中结果 correctness 与原算法（unoptimized）diff：Δ 加权分 ≤ 1e-4

---

## Task 10: 架构文档三方对账

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: T1, T2
- **ACs Covered**: FR-16 / AC-22

### 范围
1. `docs/enterprise/02-architecture.md §3.2` 新增 16×10 表：16 crate（行）×（Crate ID / AIS Layer / Owner / 关键 Traits / 关键 impl / Registry 绑定 / 图谱 Node ID / README 链接 / CI 状态 / 版本）
2. `docs/project-atlas.md` 新增 §7 Rust 绑定契约：如何让一个新 Rust crate 在 30 分钟内进入图谱（CRATE_ID → 三注册表 → README → self_sync → /atlas/verify 绿）
3. 交叉校验：文档 ↔ 三注册表 ↔ 代码常量 ↔ 图谱（T1 完成后）

### TRs
#### rubric
- **TR 10.1**（0-2, threshold 2，=AC-22）：四方对账一致。0=>3 处不一致 · 1=1~2 处 · 2=0 处

---

## Task 11: 14 crate README 补全

- **Status**: pending
- **Priority**: MEDIUM
- **Dependencies**: T1, T2
- **ACs Covered**: AC-17, AC-24

### 范围
为缺少 README 的 14 个 crate 补齐：8 节标准模板
```
1. 概述（1 句话定位）
2. CRATE_ID / ENGINE_NAME / AIS 层级（填常量）
3. 模块结构：src/* 说明
4. 关键 Trait 与 Impl（公开 API）
5. 如何跑单测：`cargo test -p <name>`
6. 如何二次开发：依赖反转 trait 实现指引
7. TDD RED→GREEN 工作流 + 精度护栏提示（如适用）
8. 图谱绑定：对应三注册表 key + 自同步规则
```

### TRs
#### rule
- **TR 11.1**: `find platform/services -name README.md | wc -l` === 16
#### rubric
- **TR 11.2**（0-2, threshold 2，=AC-24）：README 8 节完整度：0=<8 crate 有全部 8 节 · 1=8~14 · 2=16
- **证据**：review 打分 + 抽检 3 份 README

---

## Task 12: 核心算法对账 7 条二进制 + Node 测试

- **Status**: pending
- **Priority**: MEDIUM
- **Dependencies**: T3
- **ACs Covered**: AC-05

### 范围
1. 新建 `platform/services/graph-algorithms/src/bin/compare_with_node.rs`：读入 Node 侧 JSON fixture → Rust 计算 → 输出 JSON
2. Node `test-algo-rust-node-diff.js`：对同一 fixture → Node 跑 → Rust 跑 → Δ assert ≤ 1e-6

### TRs
#### rule
- **TR 12.1**: test-algo-rust-node-diff.js 7 算法 10 fixture = 70 case GREEN

---

## Task 13: `/ai/engine/workflow/execute` + 3 内置 workflow + step 图谱写回

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: —
- **ACs Covered**: AC-11, AC-12

### 范围
1. 后端 Node `routes/workflow.js`：新增 `POST /ai/engine/workflow/execute`
   - 输入校验：JSON Schema v7（workflow_id enum wf-graph-bulk-v1/wf-file-upload-v1/wf-ai-rag-v1 + 自定义 id）
   - 调用 EAF 工作流引擎（新建 `src/workflow-engine.js`）：DAG 调度 step，串行/并行，失败回滚策略
   - 每个 step：① Nebula INSERT VERTEX workflow_step ② runs_on 边绑定对应 Rust code 节点
2. Rust Gateway：`ai_engine.rs` 增加 `/ai/engine/workflow/execute` proxy（路由到 Node EAF）
3. 3 workflow 模板定义（JSON，可注册可升级）

### TRs
#### rule
- **TR 13.1**: `test-workflow-3-green.js` 每个 workflow 10 runs：ok≥9/10，shape 统一
- **TR 13.2**: 跑 30 runs 后 Nebula `COUNT(VERTEX workflow_step)` ≥ steps_count × 30
- **TR 13.3**: 每个 workflow_step 节点 runs_on 边数量 ≥ 1（指向 code 函数）

---

## Task 14: 企业级 3 端点

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: —
- **ACs Covered**: AC-13, AC-14, AC-15, AC-26

### 范围
**Node 端新增 `routes/atlas.js`（未存在则新建）**：

1. `GET /atlas/verify`：跑 8 项检查（Spec 2.4 列表），返回 JSON
2. `GET /atlas/health/enterprise`：聚合 SLO 数据（SPEC-13/14 SLA/RPO/RTO/MinIO/Nebula/Gateway HPA）
3. `POST /atlas/governance/audit`：
   - 开源版：JSON/CSV 审计流水（6 层变更 + workflow + traceId）
   - 企业增强版：hash-chain 不可篡改（每条 `audit_entry.next_hash = HMAC(secret, prev_hash || content)`），180 天 TTL

### TRs
#### rule
- **TR 14.1**: GET /atlas/verify 8 checks all ok=true（T1~T9 完成后）
- **TR 14.2**: GET /atlas/health/enterprise：availability.p99 ≥ 99.9，rpo_ms=0，rto_ms < 60000
- **TR 14.3**: POST audit 返回数组 len ≥ 1（至少 1 条 workflow 运行）
- **TR 14.4**（仅企业）：企业 tier 下 audit hash 链校验（chain verify OK）

### rubric
- **TR 14.5**（0-2, threshold 1，=AC-26）：审计完备度。0=字段缺>3 · 1=可追溯但不可篡改仅企业 · 2=开源+企业均满足分级

---

## Task 15: 全量回归 + 指标举证汇总

- **Status**: pending
- **Priority**: HIGH
- **Dependencies**: T1~T14
- **ACs Covered**: AC-18, AC-19, AC-20, AC-21, AC-23, AC-25, AC-26

### 范围
1. 构建
   - `cargo build --workspace --all-targets`
   - `cargo test --workspace`（551+ tests）
   - `cargo clippy --workspace --all-targets -- -D warnings`
2. Node 12 suites（SPEC-15 基线 12 套 + 新增 5 套 = 17 套）
3. Rust Gateway 3 suites（SPEC-6 baseline：router_semantics / sidecar_degrade / ai_engine_e2e）
4. 精度护栏：test-precision-guardrail.js（toFixed / LPA 出口 / RAW 展开 / d=0.85 / maxIter=30）
5. Rubric 汇总打分：AC-22~26 + CEM 分（0.55Q+0.2S+0.1T+0.15Stability）
6. 验收举证清单：26 AC × 每条 Evidence 列全

### TRs
#### rule
- **TR 15.1**: cargo build 0 error；cargo test 0 fail；clippy 0 warning
- **TR 15.2**: Node 17 suites + Rust 3 suites = 合计 GREEN ≥ 129（SPEC-15 基线，因新增 6 套应 150+）
- **TR 15.3**: test-precision-guardrail.js GREEN
- **TR 15.4**: router_semantics.rs GREEN
#### rubric
- **TR 15.5**（0-2, threshold 2，=AC-23）：6 层边密度。0=<0.08 · 1=0.08~0.12 · 2=>0.12
- **TR 15.6**（0-2, threshold 1，=AC-25）：CEM 加权分。0=<0.7 · 1=0.7~0.82 · 2=>0.82
- **TR 15.7**（总体评分）：26 AC 覆盖举证 → 独立 Review 用

---

## 任务执行顺序（依赖顺序建议）

```
Parallel group A（互不依赖）:
  T2 (CRATE_ID 常量)  ─┐
  T4 (依赖治理)       ─┤
  T7 (L6 kernel 拆分) ─┤
  T6 (orchestrator DIP) ├→ 并行开工（4 线程）
  T8 (hermes/business DIP) ─┤
  T9 (深链性能)       ─┘

T2 done → T1 (注册表) → T3 (算法对账) → T12 (对账测试)
T2 done → T5 (rusqlite 收拢)
T1+T2 done → T10 (文档对账) → T11 (README)

独立组 B:
  T13 (workflow 端点)  ┐
  T14 (企业 3 端点)   ─┤→ 并行

全部 T1-T14 → T15 (全回归 + 打分) → Review
```

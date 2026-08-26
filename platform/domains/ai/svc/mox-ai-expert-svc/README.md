# mox-expert · 双璇玑十四维专家联盟（14 Experts + Verify）

## §1 · 概述
璇玑企业级 L4Services 层的**专家联盟并行裁决内核**：14 个垂直领域专家 + 4 类验证器（CEM/拓扑/数据依赖/冲突）+ 审计多 sink（S3/Kafka/syslog/本地文件）+ RBAC 策略 + 流程 YAML 加载器 + 统一 harness 驱动，组成「评估→裁决→验证→审计」全链路十四维治理。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**（能力≥5，注册为 engine）。

```rust
pub const CRATE_ID: &str = "50bb6200-04c5-5e4c-8354-4c6e1b230024";
pub const ENGINE_NAME: &str = "mox::mox_expert";
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
| `src/lib.rs` | 三常量 + 对外总入口：`MoxExpert::evaluate_project` 聚合、harness、pipeline、reconcile 统一导出 |
| `src/expert.rs` + `src/expert_traits.rs` | `struct ExpertContext`；5 个 trait 家族：`trait Expert / Verify / AuditSink / DomainRule / ExpertHarness` 定义 |
| `src/experts/` (14 files) | 14 位专家：`algorithm / architecture / business / code_quality / data / documentation / maintainability / observability / performance / permission / resource / security / security_code / testing` 各自 impl Expert trait |
| `src/verify/` (7 files) | 4+ 类验证：`cem.rs 交叉熵蒙特卡洛`、`topology.rs 拓扑一致性`、`data_dep.rs 数据依赖闭环`、`conflict.rs 冲突检测`、`gains.rs 收益预估`、`code_rt.rs 代码正确性`、`tests.rs`。 |
| `src/audit/` (6 files) + `src/rbac/` (3 files) | 审计链：`event / error / sink（syslog/S3/Kafka）/ integration`；RBAC：`check / error / policy` |
| `src/domain/` (mod.rs 2 traits) + `src/flow_loader/` | 领域规则 + 流程加载 YAML/JSON 校验器 |
| `src/harness.rs` + `src/pipeline.rs` + `src/reconcile.rs` + `src/ir.rs` + `src/context.rs` + `src/types.rs` + `src/programming.rs` + `src/bench.rs` + `src/govern.rs` + `src/sensitivity.rs` + `src/server.rs` + `src/services.rs` + `src/tenant_policy.rs` | harness 驱动、流水线编排、19 路 reconcile 归一化裁决、IR 中间表示、上下文、类型系统、编程式治理、bench 压测、govern 闸门、敏感度分析、HTTP server、后端服务抽象、租户隔离策略 |
| `src/bin/mox.rs` | CLI 二进制 `mox` 专家联盟命令行 |
| `examples/cem_probe.rs` + `examples/profile_deep_chain.rs` | CEM 探针示例 + 深层链剖析示例 |
| `tests/` (10 files) | 单测 (expert_unit_tests / end_to_end / enterprise_algorithm / debug_opt) + 4 GAP 修复专项 (p1_audit_chain_continuity / p1_auto_repair_idempotency / p1_multi_e1_permission_security_veto / p1_topology_route / p2_perf_boundaries) + T8 DIP + T9 P99 深度链 |

## §4 · 关键 Trait & Impl
- **`pub trait Expert`**（expert_traits.rs）：`fn id() -> &'static str` / `fn evaluate(&self, ctx: &ExpertContext) -> Result<ExpertReport>`。
- **`pub trait Verify`**（verify/mod.rs）：6 个验证子项各自 impl。
- **`pub trait AuditSink`**（audit/sink.rs）：`fn emit(&self, event: AuditEvent) -> Result<()>`；3 sink 实现（File / SyslogUdp / S3PutObjectSignature）。
- **`pub trait ExpertHarness`**（harness.rs）：`fn run_all(&self, ctx) -> Result<FinalVerdict>`；默认并行 14 专家、裁决用多数+加权双投票。
- **14 Expert struct impl**：`struct AlgorithmExpert; impl Expert for AlgorithmExpert { ... }`（每位专家独立文件）。
- **RBAC `trait Policy`**：`fn allow(subject, action, resource) -> bool`；默认角色继承链 `Coordinator→Expert→Member`。

## §5 · 跑单测指引
```bash
cargo test -p mox-expert
cargo test -p mox-expert expert_unit_tests          # 14 专家单元
cargo test -p mox-expert end_to_end                 # 端到端评估→裁决→审计
cargo test -p mox-expert t8_dip_mox_expert_traits # T8 DIP 合规
cargo test -p mox-expert t9_deep_chain_p99          # T9 P99 深度链 ≤2s
cargo run -p mox-expert --bin mox -- eval <dir>  # CLI 评估目录
```
断言覆盖：14 专家独立评估各自维度得分 ∈ [0,100]；FinalVerdict 与 14 位报告一致；审计 emit 顺序（按 ts）连续不中断；RBAC Coordinator 拥有 Expert 权限；T8 禁止绕过 trait 直接写专家逻辑；T9 深层链 P99 ≤ 2s（热路径）。

## §6 · 二次开发 / DIP 反转指引
- **新增第 15 位专家**：新建 `src/experts/new_expert.rs`，impl `trait Expert` → 在 `pipeline.rs` 的专家注册向量 push。**严禁**改 harness.rs 主循环写 if-else。
- **新 AuditSink 后端**：impl `trait AuditSink` → `audit::register_sink(...)`。
- **新 Verify 维度**：impl `trait Verify` → `verify::add_verifier(...)`。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：加 GAP 场景（如 permission 安全 veto 误放行）→ `tests/gap_p1_*` 失败；② GREEN：最小 trait impl；③ 跑 T8+T9。
**精度护栏**：CEM 交叉熵 `γ=0.1`、`N=2000`、`iters=80` 超参是 hard const；裁决加权不得超过阈值 `|diff| < 0.02`（双仲裁一致）；RBAC 拒绝留痕「终局才落审计」—— 探测性的 AuthzDenied 不回推（§5.2 企业架构护栏）。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-mox-expert
engine id      : engine-rust-mox-expert
code_graph unit: mox-expert
```
self_sync：改 `src/lib.rs` 三常量 / 新增专家 / trait → `self_sync_rust.js` 刷新三注册。

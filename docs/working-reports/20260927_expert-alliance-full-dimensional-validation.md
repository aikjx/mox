# 专家联盟全维度验证与最小安全增量报告

> 日期：2026-09-27　·　范围：`platform/domains/alliance/`　·　性质：L7 🟡 证据（只读侦察 + 两处纯新增代码增量，非权威）
> 铁律遵守：未删除/覆盖/重命名任何既有文件；未改 workspace members、Cargo.toml 依赖、CI、git；
> 仅新增 2 个 `.rs` 文件 + 对 2 个既有 `lib.rs` 各加一行 `pub mod` 声明。

---

## 一、五维功能现状矩阵（来自三份侦察报告，只读事实）

| 维度 | 已落地（真实实现） | 缺口 / 桩 |
|---|---|---|
| **专家建模** | common-proto `Expert`（运行时）、registry-proto `Expert`（静态档案）+ `RegisteredInstance`（运行实例）、`Dimension` 14 维 | 四套专家模型互不统属；`mox-ai-expert-core` 11/14 专家为空骨架；alliance 域无 Team 概念 |
| **注册** | registry-svc 实例注册/心跳/租约/发现/主动健康探测 + 分级心跳聚合（registry-core 10:1:1）真实落地 | registry-proto `get_expert_metrics` / `get_platform_overview` 仍为默认「未实现」占位（本次增量 B 补纯逻辑） |
| **调度** | 容量/并发上限、幂等键回放、状态机咽喉点、优先级队列、暂停/恢复/取消/人工 complete_task、leader 孤儿对账 | 无独立后台 tick；Dynamic 模式仅规划期选型，执行期改写拓扑未实现；**人审挂起不是一等公民**（本次增量 A 补纯逻辑） |
| **执行** | DAG 依赖调度、超时、指数退避重试、fail_fast、skip_node、恢复续跑、尾部融合落 `fusion_result` | 缺「节点挂起等审批再续跑」的 first-class gate；http-sdk 里的人审节点只是演示数据 |
| **协作编排** | 7 种 AllianceMode 计划生成、9 种 FusionStrategy 真算法、辩论/投票/MapReduce/迭代 | 6 阶段管线存在两份（ai-alliance-engine 与 svc/alliance）未收敛 |

---

## 二、归一化映射表（alliance 平行模型 vs SSOT —— 本次不动代码，列为 ADR 候选）

> 事实：整个 `platform/domains/alliance/`（16 crate）**零依赖**三个 SSOT crate
> （`mox-unified-contract` / `mox-unified-algo-core` / `mox-data-norm-core`）。
> 下列映射**本次不改代码**，仅登记为后续跨 crate 重构的 ADR 候选。

| SSOT 契约（权威） | domains/alliance 平行定义 | 位置 | 差异 | 本次动作 |
|---|---|---|---|---|
| `MoxError`/`ErrorCode`(AL05123)/`MoxResult` | `AllianceErrorCode`(u32 1000–7999)+`AllianceError` | common-proto `error.rs` | 错误码空间与模型不同 | ADR 候选，不动 |
| `MoxEvent`/`StreamEvent`/`ProgressEvent`/7 阶段 | `AllianceEvent{Task,Node,Expert}` | common-proto `events.rs` | 抽象层不同（生命周期 vs 辩论阶段） | ADR 候选，不动 |
| `ApiResponse<T>`(code/msg/data/trace_id) | `{success,error_code}` + foundation `mox_api_protocol` 第三套信封 | api `dto.rs` / http-sdk | 三信封并存，前端已用 nesting 标注绕过 | ADR 候选，不动 |
| `PagedResponse<T>`/`PaginationInfo` | `TaskListResponse{tasks,total,page,page_size}` | api `dto.rs` | 手写分页 | ADR 候选，不动 |
| `TraceId`/`TraceContext`（X-Trace-Id 透传） | 无（事件内裸 Uuid） | — | trace 透传缺口 | ADR 候选，不动 |
| SSOT 无任务模型 | `Task/Node/TaskStatus/NodeStatus/FusionStrategy` | common-proto `types.rs` | 领域原生 DAG 模型，SSOT 未声称拥有 | 非缺口，保持 |

---

## 三、本次新增的两个纯逻辑增量模块

### 增量 A：人审 Approval Gate（`mox-alliance-scheduler-core/src/approval_gate.rs`）

补齐「DAG 节点挂起等审批再续跑」的一等公民纯状态机：

- `ApprovalRequest { task_id, node_id, reason, requested_at, status(Pending/Approved/Rejected/Expired), decided_at }`；
- 内存 `ApprovalGate`：`submit`（同节点重复提交幂等）/ `list_pending`（惰性过期）/ `decide(approve|reject)`（已决策幂等拒绝）/ `is_expired`（TTL 可配，默认 24h）/ `effect_of`；
- 节点作用推导 `GateNodeEffect{Hold,Release,Fail}`：approval pending（含过期）→ Hold；approved → Release（映射 `NodeStatus::Ready`）；rejected → Fail（映射 `NodeStatus::Failed`）。
- **诚实边界**：common-proto `NodeStatus` 无 `Blocked` 变体，本次不改权威枚举；Hold 语义由调度器在 gate 返回 Hold 期间「不派发该节点」实现，故 `Hold.to_node_status()` 返回 `None`。
- 不接 svc 路由、不改 proto、不改既有函数签名。

### 增量 B：专家指标聚合（`mox-alliance-registry-core/src/metrics_agg.rs`）

为 registry-proto 预留的 `get_expert_metrics` / `get_platform_overview` 契约提供纯计算实现：

- `ExpertMetrics { expert_id, invocations, successes, failures, avg_latency_ms, success_rate }`；
- `record_result(&mut m, ok, latency_ms)` 累加计数 + 滚动平均延迟；
- `summarize(iter) -> PlatformOverview { total_experts, healthy, degraded, by_status: BTreeMap<String,u32> }`；
- 健康阈值（固化为常量，ADR 候选）：`success_rate < 0.80` 或 `failures/invocations > 0.20` → Degraded；零样本视为 Healthy（不误报）。

---

## 四、真实验证证据（cargo test 实际输出尾部摘要）

命令均在仓根 `D:\a10\aikjx\gitcode\infotopograph` 下运行，未跑全 workspace build、未跑 clippy 全量。

### 1) `cargo test -p mox-alliance-scheduler-core`

```
running 114 tests
test approval_gate::tests::submit_then_lists_pending ... ok
test approval_gate::tests::approve_releases_node ... ok
test approval_gate::tests::reject_fails_node ... ok
test approval_gate::tests::expired_request_holds_node ... ok
test approval_gate::tests::double_decide_is_idempotent ... ok
test approval_gate::tests::empty_pending_list_and_unknown_node_releases ... ok
...（其余 108 个既有测试全 ok）

test result: ok. 114 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s
```

新增 approval_gate 6 条单测全绿；既有 108 条单测无回归。

### 2) `cargo test -p mox-alliance-registry-core`

```
running 12 tests
test metrics_agg::tests::empty_iterator_yields_zero_overview ... ok
test metrics_agg::tests::all_healthy_experts_counted ... ok
test metrics_agg::tests::one_degraded_expert_detected_by_low_success_rate ... ok
test metrics_agg::tests::success_rate_and_failure_ratio_formulas ... ok
test metrics_agg::tests::average_latency_rolling ... ok
...（7 个既有 aggregation 测试全 ok）

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

新增 metrics_agg 5 条单测全绿；既有 7 条无回归。

> 注：PowerShell 把 cargo 写往 stderr 的进度/警告进度流误报为 NativeCommandError（退出码 1），
> 但两次 `test result:` 行均为 `ok`，测试二进制实际全部通过——以上为真实尾部摘要。

---

## 五、未决事项（需用户决策）

1. **是否批准后续把 alliance 域切到 SSOT 的跨 crate 重构**：错误/事件/响应信封/trace 四组平行模型向 `mox-unified-contract` 收敛，或显式 ADR 声明 `mox_api_protocol` 为该域权威信封（跨约 16 crate，本次不动）。
2. **人审 gate 接线**：本次仅交付纯逻辑；是否在下一增量把 `ApprovalGate` 接入 scheduler-svc 路由（新增 `/tasks/:id/nodes/:node_id/approval` 类端点）与 executor-core 的 DAG 派发门，需单独评审（涉及路由/proto，超出本次最小增量边界）。
3. **metrics 接线**：本次仅交付纯函数；是否在 registry-svc 的 `get_expert_metrics` / `get_platform_overview` 契约默认实现里调用 `summarize`，需评审 trait 默认方法改动。
4. **`NodeStatus` 是否需要一等 `Blocked` 变体**：本次以 gate 侧 `Hold` 投影绕开；若后续要在 DAG 引擎层面真正挂起节点，需走 common-proto 状态机变更评审（含状态机 `can_transition_to` 测试更新）。
5. **健康阈值**（0.80 / 20%）为本次拍脑袋固化，是否纳入 BP/VAL 索引统一口径待治理。

---

*本报告为 L7 证据（🟡），非权威；结论固化后应回填 L1–L6。未修复既有 41 条文档断链（技术债，独立治理项）。*

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

## 六、第二轮接线增量（同日追加）：三项低风险纯增量接线

> 范围：本轮继续遵守铁律（不删/不改既有逻辑签名、不动 proto 既有字段、不动 workspace/Cargo.toml/CI、不 git）。跨 16 crate SSOT 重构本轮**仍不做**。

### 6.1 三项状态总览

| 项 | 状态 | 说明 |
|---|---|---|
| [1] approval_gate 接入派发前判定 | **部分（阻塞点）** | 纯接缝判定 `dispatch_decision` 已交付并测试；真正穿入 executor-core 派发循环需改依赖边/签名，本轮未硬改 |
| [2] 库存概览接入 registry-svc | **已接线** | 新增纯函数 `inventory_platform_overview` + 只读路由 `GET /api/registry/overview` + 集成测试 |
| [3] 健康阈值单一来源 | **已接线** | 抽 `HealthThresholds{0.80,0.20}` 可注入结构，`classify_with`/`summarize_with` 读阈值，默认值单源 |

### 6.2 [1] approval_gate 派发判定：阻塞点与建议接缝

- 新增 `ApprovalGate::dispatch_decision(task,node,now) -> DispatchDecision{Dispatch,Hold,Fail}`（纯包装 `GateNodeEffect`），单测 `dispatch_decision_maps_gate_state`。
- **阻塞点（未硬改）**：真正的 per-node 派发循环在 `mox-alliance-executor-core/src/dag_engine.rs::schedule_ready_nodes`（约 445–493 行：`find_ready_nodes` → 标 Running → spawn）。但 `mox-alliance-executor-core` 对 `mox-alliance-scheduler-core` **仅为 dev-dependency**（见其 Cargo.toml 第 30 行），生产代码不能 import `approval_gate`；且要把 gate 穿进 `DagEngineImpl::new` / `ExecutionOptions` 必须改既有结构体字段与构造签名。两者都违反本轮铁律。
- **建议接缝**（后续评审）：二选一——(a) 把 `approval_gate` 下沉到一个 executor-core 已常规依赖的低层 crate（如 common-proto 或新建 tiny gate crate），再在 `schedule_ready_nodes` 标 Running 前插一行 `match gate.dispatch_decision(...) { Hold => continue, Fail => mark Failed, Dispatch => ... }`；或 (b) 把 gate 作为 `ExecutionOptions` 的一个 `Option<Arc<ApprovalGate>>` 字段注入（需评审签名变更）。

### 6.3 [2] registry-svc 概览接口接线

- registry-core 新增纯函数 `inventory_platform_overview(&[RegisteredInstance]) -> proto::PlatformOverview`：从现存实例视图聚合 total/active/domains，`total_consultations=0`（逐次调用遥测待接入）。
- registry-svc `routes.rs` 新增只读路由 `GET /api/registry/overview`，handler 直接调上述纯函数；未改既有路由、未加 proto 字段。
- **仍阻塞的子项**：`get_expert_metrics(id)` 的 success_rate/avg_latency_ms 需要逐次调用遥测，svc 当前不记录；静态 `ExpertStore` 只有 rating/total_consultations，待后续在 executor 侧补 telemetry 后再喂 `summarize`。

### 6.4 [3] 健康阈值单一来源

- `metrics_agg.rs` 新增 `HealthThresholds { degraded_success_rate, degraded_failure_ratio }`，`const DEFAULT = {0.80, 0.20}`；新增 `classify_with`/`summarize_with` 接受阈值，旧 `classify`/`summarize` 保留为默认阈值包装。
- 测试 `default_thresholds_are_80_percent_and_20_percent`（断言默认=0.80/0.20）+ `thresholds_are_overridable`（放宽到 0.50 后同数据转 Healthy）。
- 说明：本应下沉 config-core 做热更新配置，但 registry-core 当前不依赖 config-core（本轮禁改 Cargo.toml 加依赖边），故先在消费侧固化为可注入结构，后续搬迁零函数签名变化。

### 6.5 真实验证证据（第二轮，尾部摘要）

`cargo test -p mox-alliance-registry-core`：
```
running 16 tests
test metrics_agg::tests::default_thresholds_are_80_percent_and_20_percent ... ok
test metrics_agg::tests::thresholds_are_overridable ... ok
test metrics_agg::tests::inventory_overview_counts_active_and_distinct_domains ... ok
test metrics_agg::tests::inventory_overview_empty ... ok
...
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p mox-alliance-scheduler-core`：
```
running 115 tests
test approval_gate::tests::dispatch_decision_maps_gate_state ... ok
...
test result: ok. 115 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
```

`cargo test -p mox-alliance-registry-svc`：
```
running 14 tests   (lib 单测) ... test result: ok. 14 passed; 0 failed
running 11 tests   (tests/http_registry.rs 集成)
test platform_overview_aggregates_registered_instances ... ok
...
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

> 注：PowerShell 仍把 cargo 写 stderr 的进度流误报为 NativeCommandError（退出码 1），以上 `test result: ok` 行为真实通过依据。

---

## 七、决策项④：健康阈值 svc 侧 provider（启动加载档，同日追加）

> 范围：本轮只做决策项④（健康阈值热更新）。①SSOT / ②approval_gate 穿 executor / ③executor 遥测本轮不做。

### 7.1 分层约束与落点

- **禁止**：registry-core 不得新增对 `mox-alliance-config-core` 的依赖（它是零 IO 纯计算核，config-core 带 tokio/store/broadcast 太重）。本轮 registry-core 未动。
- **结论：做到"启动加载 + 可注入"档，未做真热更新**。原因：registry-svc 的 Cargo.toml 当前**不依赖** `mox-alliance-config-core`，要订阅 config-engine 的 broadcast 变更事件必须新增这条依赖边，违反本轮铁律。故停在 svc 既有 env 配置源上做启动加载。

### 7.2 新增内容（均为纯增量）

- svc `Config` 新增两个字段：`health_healthy_min: f64`（默认 0.80）、`health_degraded_max_ratio: f64`（默认 0.20）。
- `Config::from_env()` 新增两条 env 覆盖：`MOX_ALLIANCE_REGISTRY_HEALTHY_MIN` / `MOX_ALLIANCE_REGISTRY_DEGRADED_MAX_RATIO`（解析失败或越界 [0,1] 则保留默认，与既有 `*_MS` 字段同构）。
- 新增 `Config::health_thresholds(&self) -> registry_core::HealthThresholds`：把 svc 启动配置投影为 registry-core 的阈值单一来源。
- registry-core 纯逻辑（`HealthThresholds`/`classify_with`/`summarize_with`）保持零 IO，未引入 async/tokio。

### 7.3 热更新建议接缝（未做，需评审）

后续若要真热更新：给 registry-svc 加 `mox-alliance-config-core` 依赖边（走 Cargo.toml 变更评审），订阅 config-engine broadcast，收到 `health.*` 变更时用 `Config::health_thresholds()` 重建 `HealthThresholds` 并热替换 `AppState` 句柄；handler 签名不变。

### 7.4 真实验证证据（尾部摘要）

`cargo test -p mox-alliance-registry-svc`：
```
running 17 tests
test app_state::tests::default_thresholds_are_80_20_and_aligned_with_core_default ... ok
test app_state::tests::custom_thresholds_change_classification_outcome ... ok
test app_state::tests::custom_fields_project_into_thresholds ... ok
...（其余 14 个既有测试全 ok）
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

running 11 tests   (tests/http_registry.rs 集成)
test platform_overview_aggregates_registered_instances ... ok
...
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`cargo test -p mox-alliance-registry-core`：
```
running 16 tests
test metrics_agg::tests::default_thresholds_are_80_percent_and_20_percent ... ok
test metrics_agg::tests::thresholds_are_overridable ... ok
...
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

---

## 八、决策项④延伸：config-core 真热更新可行性判定（同日追加）

> 范围：用户已批准给 registry-svc 加 `mox-alliance-config-core` 内部依赖以做真热更新。本轮先判定承载点，再决定是否硬做。

### 8.1 判定结论：env 档 + ADR（未加依赖、未改 config-core）

读 `config-core` 的 `events.rs`/`engine.rs`/`store.rs` 后确认：**当前没有干净承载点**。

- `ConfigStore` trait 全是**类型化槽位**：只有 `GlobalLlmConfig`（全局 LLM provider 默认）与 `ExpertModuleConfig`（专家模块 LLM+Graph+匹配权重）两类持久化对象，**没有通用 KV、没有 system/health 槽位**。
- `ConfigChangeEvent` **不携带值载荷**（只有 module_id/config_type/change_type/version/changed_by/reason/timestamp）；订阅方必须用类型化 getter 回读，而 getter 里没有任何 health 阈值字段。
- 把 `health.healthy_min` 塞进 `GlobalLlmConfig` 或 `ExpertModuleConfig.graph_config`/`llm_config` 都会**污染专家模块 LLM/Graph 语义**——正是任务明令禁止的。
- 真热更新需要先给 config-core 加新类型化配置槽位（动 trait、动 common-proto），超出本轮"不改 config-core 既有公共 API/字段"的铁律。

故本轮**不加** registry-svc/Cargo.toml 那条依赖（无消费方，加了是死依赖），保留上一轮的 env 启动加载档，把前置改造写成 ADR。

### 8.2 ADR：config-core 健康策略槽位前置改造（待评审，未实施）

建议新增（均为 additive，不改既有 LLM/Graph 语义）：

1. `common-proto` 新增 `HealthPolicyConfig { healthy_min: f64, degraded_max_ratio: f64, version: u32, updated_at: DateTime<Utc> }`，默认 `(0.80, 0.20)`。
2. `ConfigType` 新增变体 `Health`（或复用 module_id=`__system__`）。
3. `ConfigStore` trait 新增 `get_health_policy()` / `save_health_policy()`；`MemoryConfigStore` 相应加一个 `RwLock<Option<HealthPolicyConfig>>` 字段。
4. `ConfigEngine` 新增 `get_health_policy()` / `set_health_policy(policy, by, reason)`，后者在保存后 `publish_event(ConfigChangeEvent::new("__system__", ConfigType::Health, Updated, ...))`。
5. registry-svc 侧（待上述落地后）：Cargo.toml 加 `mox-alliance-config-core = { workspace = true }`；启动时 `ConfigEngine::subscribe()`，收到 `module_id=="__system__" && config_type==Health` 事件时回读 `get_health_policy()`，用 `parking_lot::RwLock` 热替换 `AppState` 内的 `HealthThresholds` 句柄；`/overview` 每次读最新值。handler 签名不变。

事件映射：`__system__` + `ConfigType::Health` + `ConfigChangeType::Updated` → registry-svc 回读 → 重建 `HealthThresholds{degraded_success_rate=healthy_min, degraded_failure_ratio=degraded_max_ratio}`。

### 8.3 真实验证证据（本轮无代码变更，基线回归）

`cargo test -p mox-alliance-registry-svc`：
```
running 17 tests ... test result: ok. 17 passed; 0 failed
running 11 tests (tests/http_registry.rs) ... test result: ok. 11 passed; 0 failed
```

`cargo test -p mox-alliance-registry-core`：
```
running 16 tests ... test result: ok. 16 passed; 0 failed
```

---

*本报告为 L7 证据（🟡），非权威；结论固化后应回填 L1–L6。未修复既有 41 条文档断链（技术债，独立治理项）。*

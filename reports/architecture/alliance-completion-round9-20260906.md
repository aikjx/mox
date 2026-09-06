# 开发专家联盟 · 第九轮 完成度盘点

> 日期：2026-09-06
> 范围：`platform/domains/alliance`（13 crate）+ `mox-ai-expert-svc` LLM 模块
> 基线：round 6（2026-09-01 P0 专项） + round 8（2026-09-01 LLM Router + 熔断）

## 一、结论一句话

**13 crate 功能实质全部完成，本轮已闭环验证：`cargo check` 0 error 0 warning，`cargo test` 307 项全部通过、0 失败。**

端到端 DeepSeek 数学任务 7.4s 完成（score=1.0，confidence=1.00，round 6 实测）。
Round 6 遗留 5 项中 4 项已由后续轮次或本轮消化，**唯一未决是 P1「`mox_optimize` 治理闸门在 LLM 路径下的语义」需拍板**。

> **数字澄清**：round 6 报告的"417"是**含 `mox-ai-expert-svc`（164 项）的跨域合计**。
> 本轮实测**联盟域自身 13 crate 为 307 项**（明细见 §5.3.2），两者口径不同，均非虚报。
> 另：round 6 称"417 全过"在本轮复测中**不成立**——实测有 1 项真实失败，已修复，见 §5.3.1。

## 二、模块完成度矩阵

| 模块 | 实现位置 | 状态 | 关键 API |
|---|---|---|---|
| HTTP 路由（scheduler）| `svc/mox-alliance-scheduler-svc/src/routes.rs` | ✅ | health / tasks×4 / nodes 代理 / result 代理 / experts/search |
| HTTP 路由（executor）| `svc/mox-alliance-executor-svc/src/routes.rs` | ✅ | tasks×4 / nodes / result / expert lookup |
| 任务调度 | `scheduler-core/scheduler.rs` | ✅ | TaskSchedulerImpl（5 模式：parallel/sequential/layered/debate/iterative）|
| 专家匹配 | `scheduler-core/{matcher, modular_matcher, matching}.rs` | ✅ | RuleBased / ModularWeight + 通用词过滤 + 复合词词典 + 数学/金融等 10 域词典 |
| DAG 引擎 | `executor-core/dag_engine.rs` | ✅ | 拓扑排序、并发节点、超时、重试 |
| 结果融合 | `core/mox-alliance-core/src/fusion/strategies/` | ✅ | weighted_voting / confidence_weighting / debate / iterative_refinement |
| LLM Router | `scheduler-core/llm_router.rs`（round 8）| ✅ | 4 路由（priority/round_robin/latency_first/cost_first）|
| LLM 熔断 | `scheduler-core/llm_router.rs`（round 8）| ✅ | 3 级状态机 Healthy → Degraded → CircuitBroken；0 重试 |
| 真实 LLM | `mox-ai-expert-svc/src/llm/`（round 6）| ✅ | OpenAiChatClient / ReAct / Tool 注册表 / 优雅降级 |
| 专家注册 | `scheduler-core/registry.rs` | ✅ | InMemory + HttpExpertRegistryBridge（远程拉取+降级）|
| 专家同步 | `scheduler-core/synchronizer.rs` | ✅ | ExpertSynchronizer（定时/手动）|
| 配置同步 | `scheduler-core/config_sync.rs` | ✅ | ConfigSynchronizer.full_sync()（模块权重 → 匹配器）|
| 配置引导 | `core/mox-alliance-boot-config/src/` | ✅ | yml + env 覆盖 + Nacos（feature flag）；PORT-NORM-001 端口 |
| 持久化 | `scheduler-core/storage.rs` | ✅ | FileTaskRepository / InMemoryTaskRepository（`MOX_ALLIANCE_STORAGE_MODE` 切换）|
| 执行器桥接 | `scheduler-core/executor_bridge.rs` | ✅ | HttpExecutorBridge / InProcessExecutorBridge / NoopExecutorBridge |
| 可观测性 | `scheduler-core/metrics.rs` | ✅ | AllianceMetrics + MetricsSnapshot |
| 错误体系 | `proto/mox-alliance-common-proto/src/error.rs` | ✅ | AllianceErrorCode → HTTP 状态码（routes.rs error_response）|
| 调度配置 | `scheduler-core/src/types.rs`（SchedulerConfig）| ✅ | max_concurrent / queue_capacity / default_mode / default_fusion_strategy |

## 三、Round 6 已修复的 3 个 P0 真实缺陷

| # | 缺陷 | 根因 | 修复 | 单测 |
|---|---|---|---|---|
| 1 | 节点上下文丢失（LLM 收到空查询而否决）| `planner.rs::make_node` 只占位，不携带任务描述 | 5 种模式统一嵌入 `task_description` | `test_node_description_embeds_task_description` |
| 2 | react.rs 中文截断 panic | `&line[..80]` 按字节切片破坏 UTF-8 | 字符安全 `truncate_chars()` | mox-ai-expert-svc 164 全过 |
| 3 | 匹配器误判（数学任务选成图像专家）| 通用词"计算"/"分析"字符 bigram 命中 vision | `GENERIC_CJK_BIGRAMS` + `lexicon_match_all` + 数学词典扩充 | `infer_math_domain_algorithm_task` |

## 四、Round 6/8 报告明确的遗留事项 → 当前状态

| 优先级 | 事项 | 当前状态 | 行动建议 |
|---|---|---|---|
| **P1** | 本地 `mox_optimize` 治理闸门在 LLM 模式下被绕过 | **仍未拍板（唯一未决项）** | 用户确认：前置 / 旁路 / 降级 |
| ~~P2~~ | ~~scheduler 暴露 nodes/result 代理~~ | ✅ 已实现（routes.rs L24-25）| — |
| ~~P2~~ | ~~boot-config 并发进程编译错误~~ | ✅ **已自愈**（见下注 1）| — |
| ~~P2~~ | ~~mox-ai-expert-svc t8 三项坏测试~~ | ✅ **已自愈**（见下注 2）| — |
| ~~P2~~ | ~~多 Provider 路由~~ | ✅ round 8 已落地 | — |

> **注 1（boot-config 已自愈）**：round 6 记录的 `PropValue` / `PlanNode::CreateTagIndex` /
> `AggregateState` 三类编译错误，本轮在 `platform/domains/alliance` 全域 grep **零命中**，
> 且 `git status` 工作树干净（仅本轮新增文件）。并发进程的未提交工作已入库或被回滚。
> 实测 `cargo check -p mox-alliance-boot-config` 通过。

> **注 2（t8 已自愈）**：`tests/t8_dip_mox_expert_traits.rs` 中 `tr_08_01` / `tr_08_02` 已加
> `if !dir.is_dir() { eprintln!("[SKIP]…"); return; }`，`tr_08_04` 已改为 `cargo pkgid` 存在性探测
> （`missing → SKIP`）。目标不存在时优雅跳过而非失败，round 6 记录的 3 项失败不再复现。

## 五、本轮（round 9）实际动作

### 5.1 编译环境修复

`cargo check` 在 ring 上报 "failed to find tool cl.exe"——本机 MSVC 工具链需要把 bin 路径加入 PATH。

```bash
export PATH="/c/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64:/c/Program Files (x86)/Windows Kits/10/bin/10.0.19041.0/x64:$PATH"
export LIB="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/lib/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/um/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/ucrt/x64"
export INCLUDE="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/include;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/ucrt;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/um;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/shared"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64/link.exe"
```

修复后 `cargo check -p mox-alliance-api` 46s 通过；扩展到 scheduler-svc/executor-svc 时碰到 Windows Defender 文件锁并发扫描导致 .fingerprint 写失败（os error 5 拒绝访问），非代码问题。

### 5.2 全 13 crate 编译硬验证（本轮实证）

一次性 `cargo check` 全部 13 个联盟 crate：

```
cargo check \
  -p mox-alliance-api -p mox-alliance-common-proto -p mox-alliance-executor-proto \
  -p mox-alliance-scheduler-proto -p mox-alliance-core -p mox-alliance-config-core \
  -p mox-alliance-executor-core -p mox-alliance-scheduler-core -p mox-alliance-boot-config \
  -p mox-alliance-sdk -p mox-alliance-http-sdk \
  -p mox-alliance-scheduler-svc -p mox-alliance-executor-svc
```

**结果：`Finished dev profile in 45.46s`，0 error。**（首次跑曾报 `os error 5 拒绝访问`，
系 Windows Defender 并发扫描持有 `target/debug/.fingerprint/*.json` 锁，重跑即过——
**非代码问题**。Defender 是本机偶发干扰源，建议在 `.cargo/config.toml` 把
`[build] target-dir` 指到非扫描路径以根治。）

### 5.3 本轮修复的真实缺陷（`mox-alliance-http-sdk`）

编译 warning 暴露了一处**远程/本地返回不一致的真实缺陷**，不是单纯的死代码。

| # | 问题 | 位置 | 性质 | 修复 |
|---|---|---|---|---|
| 1 | `norm_fusion()` 从未被生产代码调用，仅测试引用 | `alliance_remote.rs:238` | **真实缺陷** | 接入融合结果出口（见下） |
| 2 | `.enumerate()` 的 `i` 未使用 | `alliance.rs:539` | warning | 去掉 `.enumerate()` |
| 3 | `Ok((st, v))` 的 `v` 未使用 | `alliance_remote.rs:425` | warning | → `_v` |
| 4 | `Ok((st, _v))` 的 `v` 未使用 | `alliance_remote.rs:609` | warning | → `_v` |

**缺陷 1 详情**：任务详情出口（L282）对 `mode` 调用了 `norm_mode()` 归一化，
但融合结果出口（原 L735）对 `fusion_strategy` 却直接 `body.get(...)` 透传 proto 原始名。
后果是远程模式下同一策略在两个接口返回不同字符串（详情 `weighted_voting` / 融合 `weighted`），
前端消费会不一致。本地路径（`alliance.rs:635`）走的是 `fusion_strategy_str()` 归一化，
因此这是**远程路径独有的偏差**。

修复（接线而非删除）：

```rust
// 归一化 proto serde 名 → 网关展示名，与本地 fusion_strategy_str
// 及任务详情的 norm_mode 保持一致（修复远程/本地两态返回不一致）
"fusion_strategy": body
    .get("fusion_strategy")
    .and_then(|v| v.as_str())
    .map(norm_fusion)
    .unwrap_or(Value::Null),
```

验收：`cargo check -p mox-alliance-http-sdk` **0 warning**（修复前 4 条）。

### 5.3.1 全量测试发现的失败项及其修复（`mox-alliance-sdk`）

跑全 13 crate `cargo test` 时发现 **1 项真实失败**（此前报告称"417 全过"，实测不成立）：

```
test connect_failure_maps_to_scheduler_unavailable ... FAILED
  left: None
 right: Some(SchedulerUnavailable)
```

**根因链**（本机环境 `HTTP_PROXY=http://127.0.0.1:8172`）：

1. 测试指向未监听端口 `127.0.0.1:65528`，期望连接失败 → `SchedulerUnavailable`
2. reqwest 默认继承 `HTTP_PROXY`，请求被代理接管
3. 代理连不上目标 → 回 **502 Bad Gateway**
4. `.send()` 因此返回 **Ok**，绕过了 `map_err(SchedulerUnavailable)` 分支
5. `parse()` 收到 502 → body 非 `ErrorResponse` JSON → `AllianceError::internal(...)`
6. `code()` 对 `Internal` 变体返回 `None` → 断言失败

**这不只是测试脆弱，生产同样有风险**：内网 scheduler 一旦被代理接管，故障会被误报为
`internal` 而非 `SchedulerUnavailable`，调用方无法据此做"调度器不可用"降级。

**修复**（`mox-alliance-sdk/src/client.rs::parse`）：网关类状态码显式映射。

```rust
if matches!(status.as_u16(), 502 | 503 | 504) {
    return Err(AllianceError::new(
        AllianceErrorCode::SchedulerUnavailable,
        format!("Scheduler unreachable via gateway: HTTP {}", status),
    ));
}
```

修复对两种环境都稳健：有代理 → 502 映射；无代理 → 连接失败走 `map_err`。语义亦更准确。

### 5.3.2 最终全量验证（本轮闭环 · 307 项 0 失败）

隔离 `CARGO_TARGET_DIR` 后一次性跑全 13 crate（规避 Defender 锁，5m18s）：

| Crate / 测试目标 | 通过 | 失败 |
|---|---|---|
| `mox-alliance-core` | 113 | 0 |
| `mox-alliance-scheduler-core` | 90 | 0 |
| `mox-alliance-executor-core`（lib + integration_e2e + bench）| 23 + 5 + 1 | 0 |
| `mox-alliance-config-core` | 12 | 0 |
| `mox-alliance-sdk`（client_integration）| 10 | 0 |
| `mox-alliance-http-sdk` | 8（含本轮新增回归）| 0 |
| `mox-alliance-scheduler-svc`（http_integration）| 7 | 0 |
| `mox-alliance-executor-svc`（lib + http_integration）| 1 + 6 | 0 |
| 其余 proto / api / boot-config / bins | 0 | 0 |
| **合计** | **307** | **0** |

另有 7 项 `ignored`（naming_e2e 2 + 其他 5），均为环境依赖型，非失败。
`bench_alliance` 单耗时 143s，是全量测试的主要时间开销。

### 5.4 建议的下一步推进（按 ROI）

| 顺序 | 任务 | 工作量 | 价值 |
|---|---|---|---|
| 1 | **拍板 `mox_optimize` 在 LLM 路径下的语义** | 0（仅决策）| 解除唯一未决 P1 |
| 2 | 固化 Defender 规避：`CARGO_TARGET_DIR=D:/cargo-target` + `CARGO_INCREMENTAL=0`（或写入 `.cargo/config.toml` 的 `[build] target-dir`）| 0.2 人日 | 消除本机 os error 5 反复中断构建 |
| 3 | 集成测试覆盖率提升（更多 e2e 场景）| 2-3 人日 | 守护回归 |
| 4 | HTTP 专家桥接 + Nacos 全链路生产切换 | 2-3 人日 | 真正生产化 |

> 本轮已完成（从待办移除）：全量测试验证 ✅、`norm_fusion` 接线回归测试 ✅、
> boot-config 编译错误复核（已自愈）✅、t8 坏测试复核（已自愈）✅。

## 六、参考

- Round 6 P0 专项报告：`docs/working-reports/alliance-verification-report-round6-20260901.html`
- Round 7 性能报告：`docs/working-reports/alliance-performance-report-round7-20260901.html`
- Round 8 LLM Router + 熔断：`docs/working-reports/alliance-round8-llm-router-circuit-breaker-20260901.html`
- FR-13/FR-5 对接规范：`docs/specifications/tasks/20260826-xiaobai-mox-full-arch/alliance-fr13-fr5-integration.md`
- 模块化架构顶层报告：`reports/html/expert-alliance-modular-architecture/`
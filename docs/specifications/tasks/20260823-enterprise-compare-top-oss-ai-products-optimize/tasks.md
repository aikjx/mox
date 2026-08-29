# Tasks：璇玑 v3.1 开源顶级 AI 产品架构对比 + 试验证 + 企业级优化
> 关联 spec：`.trae/specs/20260823-enterprise-compare-top-oss-ai-products-optimize/spec.md`
> 任务总数：13（T1~T12 + Review）· **依赖关系按序号严格推进；其中 T6~T10 优化项可在 Harness before 数据产出后并发开发，after 数据在独立 T11 统一采集。**

---

## Task 1：产出 P0-P4 对照矩阵（MD + JSON 两份）
**Status:** pending
**Priority:** high
**Dependency:** 无
**Scope:**
  - 基于 P1=Dify / P2=LangGraph(OSS) / P3=Flowise / P4=AutoGen 的公开官方文档（README/docs/langchain-ai.github.io/autogen/docs 等），以 L0 文档 §18 TOP-MASTER 的对照维度 D01~D18 作为列，5 个产品 P0~P4 作为行，产出 90 格评分（0-100 整数）+ 详细文字说明（每格至少 1 句"该产品的能力描述"）。
  - P0 璇玑的分数必须以"真实代码审计证据"为基础（比如 Wasm 算子沙箱：D03 给 90，而非拍脑袋）。
  - 评分方法：每格附一行 "evidence=<path_or_url>"（P0 给仓库内文件绝对路径；P1~P4 给公开 URL）。
**Deliverables（实施时写入）:**
  - `.trae/specs/20260823-enterprise-compare-top-oss-ai-products-optimize/T10-comparison-matrix.md`
  - `.trae/specs/20260823-enterprise-compare-top-oss-ai-products-optimize/T10-comparison-matrix.json`
**Test Requirements:**
  - TR-1.1 (rule)：JSON parse 成功，顶层包含 `{meta, scores[][], notes[][]}` 三字段；`scores.length === 5`；每个 `scores[i].length === 18` 且全部整数、0 ≤ x ≤ 100。证据：`node -e "JSON.parse(...)"` exit=0 + schema 校验。
  - TR-1.2 (rule)：MD 文件含 D01~D18 的独立表格，且每格都有 1+ 句非"待补充"文字。证据：`grep -c "TODO\|待补充"` = 0。
  - TR-1.3 (rubric AC-20)：架构对照深度（0-100，见 spec）。自评：依据 18 维度的平均说明句数（≥ 3 句 = 80 分，≥ 5 句 = 100 分，< 3 句线性给分）。
**Notes for Plan:**
  - 可以委派 search 子代理一次性检索 P1~P4 的官方 docs 摘要（只读）。

---

## Task 2：差距分级 & 优化候选
**Status:** pending
**Priority:** high
**Dependency:** T1
**Scope:**
  - 对照矩阵中，提取 P0 分数 < P1~P4 最高得分 的维度（即"P0 至少落后一家顶级开源"）。
  - 为每一个差距维度：给出分级（Critical / High / Medium / Low）+ 企业落地风险说明 + ≥ 1 条优化候选（与 P1~P4 的能力对齐）。
  - 必须把 Critical/High 的前 5 条与 O1~O5 建立一一对应（spec §五 FR7）：LLM LatencyWarm 路由 / Token Bucket / Wasm Fuel+Mem 上限 / /system/slo / ParallelNode+CancellationToken。
**Deliverable:**
  - `.trae/specs/20260823-enterprise-compare-top-oss-ai-products-optimize/T10-gap-analysis.md`
**Test Requirements:**
  - TR-2.1 (rule AC-03)：分级条目数 ≥ 4，且 Critical/High 总条目 ≥ 3。
  - TR-2.2 (rule)：每条 Critical/High 必须链接到至少一个优化任务编号（T6~T10）。

---

## Task 3：H1 高并发治理 harness（200 req/s × 60s，限流 + 路由）
**Status:** pending
**Priority:** high
**Dependency:** 无（可与 T1 并发，但其"before 数据"要在 T6/T7 优化前跑完，作为基线）
**Scope:**
  - 编写 Node 脚本 `platform/backend-node/test/bench_governance_concurrency.js`，使用 **`worker_threads` + `fetch`（或内部 `http` 模块）**，在单进程启动一个"最小 mock 版 server"加载 `security.js` 与 `llm-gateway.js`，对外暴露一个 `POST /bench/chat`（不经真实网络端口）。
  - 并发参数：QPS=200（可配置），时长=60s；混合"普通租户 × 80%、VIP × 15%、未登录 × 5%"。
  - 输出 CSV：每 1s 一条记录，字段=`ts_ms, ok_count, fail_count, rl_blocked, cb_open, p50, p95, p99, mem_rss_kb`。
  - 额外输出一行汇总：`total_ok,total_fail,success_rate,rl_total_blocked,cb_open_count,p50_avg,p95_avg,p99_avg`。
**Deliverables:**
  - `platform/backend-node/test/bench_governance_concurrency.js`
  - `.trae/specs/.../harness-data/h1_before.csv`（实施阶段先跑 before）
**Test Requirements:**
  - TR-3.1 (rule AC-04)：脚本运行 exit=0，h1_before.csv 行数 ≥ 60，列齐全。
  - TR-3.2 (rule NFR2 - 基线)：before 阶段即使有少量被拒，也必须 **进程不崩溃，`total_ok > 0`，`fail_count/total < 1%`（非 rate-limit 导致的失败）**。
  - TR-3.3 (rubric AC-18 基线锚点)：before 成功率记录，用于 O2 after 对比。

---

## Task 4：H2 LLM 路由策略 harness（3 策略 × 1000 次模拟）
**Status:** pending
**Priority:** high
**Dependency:** 无（与 T3 可并发）
**Scope:**
  - 编写 Node 脚本 `platform/backend-node/test/bench_llm_routing_strategies.js`，构造 4 个 Mock Provider：
    * `P-A`（强，但偶发失败 1%、延迟 P99=600ms）
    * `P-B`（快，P99=300ms，但 10% 请求返回 429）
    * `P-C`（稳定但慢，P99=900ms，失败 0.1%）
    * `P-Local`（P99=50ms，失败 0%，仅作为兜底 LocalEngine）
  - 策略 3 个（都已在 llm-gateway 中应存在或新加）：`priority`（优先级链式）、`fallback`（失败即切）、`latency-warm`（O1 新算法，见 T6）。
  - 1000 请求 × 3 策略，记录每次 `provider, latency_ms, status, fallback_used`。
  - 输出 CSV：`strategy, p50, p95, p99, success_rate, fallback_ratio, avg_provider_cost`，以及每请求明细行。
**Deliverables:**
  - `platform/backend-node/test/bench_llm_routing_strategies.js`
  - `.trae/specs/.../harness-data/h2_before.csv`
**Test Requirements:**
  - TR-4.1 (rule AC-05)：脚本 exit=0，CSV 至少有 3 条汇总行（3 策略）+ 明细 ≥ 3000 行。
  - TR-4.2 (rubric AC-17 基线锚点)：记录 before 三种策略下的平均 P99。

---

## Task 5：H3 Wasm 沙箱 harness（1000 次正常 + 1000 次恶意）
**Status:** pending
**Priority:** high
**Dependency:** 无（与 T3/T4 并发）
**Scope:**
  - 编写 Rust 基准测试（或 `#[test]` 用 `std::time::Instant` 计时）：`platform/services/operator-wasm/tests/bench_sandbox.rs`。
  - 两大部分：
    (a) 正常：1000 次 `fib(20)` wasm 调用，记录 `call_latency_us, mem_delta_bytes, status`；
    (b) 恶意：1000 次调用（每轮从 8 种恶意 wasm 字节码中随机选一种：无限循环、超大内存申请、非法 syscall、栈溢出、OOB 读写、未定义函数调用、浮点数 NaN 污染、超长 host 回调字符串）。
  - 同时对"原生算子（纯 Rust 等价实现）"做对照 1000 次，计算沙箱 overhead（%）。
  - 输出 `.trae/specs/.../harness-data/h3_before.csv`，列=`mode[normal/malicious/native], it, latency_us, mem_kb, trapped, trap_reason`。
**Deliverables:**
  - `platform/services/operator-wasm/tests/bench_sandbox.rs`
  - 2~3 个小型 .wasm 字节码（最好是 `wat` 文本直接写进测试，用 `wat::parse_str` 编译；如无 wasmer wat 支持则用 `include_bytes!` 固定字节）。
**Test Requirements:**
  - TR-5.1 (rule AC-06)：`cargo test -p operator-wasm --test bench_sandbox` exit=0，CSV 生成且列齐全。
  - TR-5.2 (rule NFR3 baseline)：before 阶段中，恶意样本的"宿主崩溃数 = 0"（至少现有 wasmer 沙箱已部分隔离，断言不崩溃）。
  - TR-5.3 (rubric AC-19 baseline)：记录恶意捕获率基线。

---

## Task 6：H4 专家联盟并发 harness（7 专家 × 4 并发组）
**Status:** pending
**Priority:** medium
**Dependency:** 无
**Scope:**
  - 编写 Rust 集成测试：`platform/services/mox-expert/tests/bench_alliance_concurrency.rs`。
  - 启动 4 个并发请求（用 `tokio::spawn`），每个请求内部触发"7 专家并行辩论（rayon par_iter）"。
  - 记录：每组 `throughput_req_per_sec, cpu_util_approx, p99_total_ms, peak_mem_kb, expert_wall_ms_sum`。
  - 生成 `.trae/specs/.../harness-data/h4_before.csv`，并用同一代码在 T11 after 再跑一次。
**Deliverables:**
  - `platform/services/mox-expert/tests/bench_alliance_concurrency.rs`
  - `.trae/specs/.../harness-data/h4_before.csv`
**Test Requirements:**
  - TR-6.1 (rule AC-07)：`cargo test -p mox-expert --test bench_alliance_concurrency -- --ignored`（基准默认 ignored，Harness 手动跑）exit=0，CSV 行数 ≥ 4。
  - TR-6.2 (rule)：4 组并发全部完成，无 panics / deadlocks。

---

## Task 7：O1 LLM 路由新增 Latency-WARM（加权平均 + 预热 + EWMA）
**Status:** pending
**Priority:** high
**Dependency:** T4 before（用 before 数据与 T4 同一脚本跑 O1 after 对比）
**Scope:**
  - 文件：`platform/backend-node/src/llm-gateway.js`（与 `routes/ai-engine.js` 的 `route` 接口对接）。
  - 新增策略类 `class LatencyWarmRouting`：
    (1) `ewma_alpha = 0.2`，对每个 provider 维护 ewma_latency_ms、ewma_error_rate、req_count；
    (2) 每 50 请求做一次"预热"：给 Top2 候选各发一次最小 prompt（`ping`）拿到实时延迟；
    (3) 决策分数 = `0.6 * normalized_latency + 0.3 * (1-error_rate) + 0.1 * priority_score`，选最高；
    (4) 失败后 200ms 内自动换 Top2（fallback 兜底）。
  - Feature flag：`process.env.DISABLE_OPTIM_O1_LATENCY_WARM === '1'` 时退回默认 priority 策略。
  - 新 metric：`slo_metrics.routing_strategy_switch_count` 与 `slo_metrics.provider_ewma[providerId]`，供 O4 /system/slo 导出。
**Test Requirements:**
  - TR-7.1 (rule AC-08)：`mocha test/bench_llm_routing_strategies.js --grep "LatencyWarm"` passes ≥ 2。
  - TR-7.2 (rule)：feature flag 能成功关闭策略，退回旧行为。
  - TR-7.3 (rubric AC-17 对 O1 单独评分)：O1 使 P99 相对 priority 下降 ≥ 20% 即 100 分，≥ 10% = 60 分。

---

## Task 8：O2 令牌桶（Token Bucket）+ 租户级 QPS 配额
**Status:** pending
**Priority:** high
**Dependency:** T3 before（用同一 H1 跑 after 对比）
**Scope:**
  - 文件：`platform/backend-node/src/security.js`，新增 `class TokenBucket` 与 `class MultiTenantRateLimiter`。
  - TokenBucket：容量 `capacity`、填速 `tokens_per_sec`、支持 `tryTake(n=1)`。
  - MultiTenantRateLimiter：按 `tenantId`（或未登录 `"anonymous"`）隔离桶；默认：普通 10 qps、VIP 50 qps、匿名 2 qps；配置通过 `config.js security.rate_limits` 覆盖。
  - 在现有 `security.middleware()` 内将旧滑动窗口与新桶 **并行运行**（旧的仅保留 1 个小版本，避免 break API；新的结果写入 `req.attrs.rl_bucket`）。
  - Feature flag：`DISABLE_OPTIM_O2_TOKEN_BUCKET`。
  - 新 metric：`slo_metrics.rl_bucket_allowed_count / rl_bucket_blocked_count / rl_bucket_tenant_count`。
**Test Requirements:**
  - TR-8.1 (rule AC-09)：`mocha` 新增用例 `TokenBucket 速率精确性（100ms 窗口内最多 1 个 token × 1s 内 10 个）` GREEN。
  - TR-8.2 (rule NFR2 after)：H1 重跑后 success_rate ≥ 99.5%，rl_blocked ≤ 5%（200qps 对 VIP 以上合理放行）。
  - TR-8.3 (rubric AC-18 after score)：按成功率打分。

---

## Task 9：O3 Wasm 算子沙箱 Fuel + 内存硬上限 trap
**Status:** pending
**Priority:** high
**Dependency:** T5 before
**Scope:**
  - 文件：`platform/services/operator-wasm/src/lib.rs`（结构体 & impl）。
  - 新增字段：`fuel: Option<u64>`、`mem_pages_limit: Option<u32>`。
  - 新增构造函数：`WasmOperator::with_limits(module_bytes, fuel, mem_pages_limit)`。
  - Fuel 消耗：使用 wasmer `store.set_fuel(Some(fuel))` 或 `Engine::fuel_epochs`（根据 wasmer 真实 API 选择其一），调用前 set，调用后检查剩余 fuel；若为 0 则视为 "timeout/trap"。
  - 内存限制：使用 wasmer `MemoryType::new(min, Some(max))` 或 `Instance 后 memory.grow(max_pages) error` 断言。
  - 新增 2 条 `#[test]`：
    (1) 无限循环.wat（燃料=2 000 000 单位）必须 fuel trap 返回 `Err(WasmError::FuelExhausted)`；
    (2) 超大内存申请.wat（mem_pages_limit=4）必须 grow 失败，返回 `Err(WasmError::MemoryLimit)`。
  - Feature flag：在 `WasmOperator::new()` 中，如果 `DISABLE_OPTIM_O3_WASM_FUEL=1`，则不启用限制，保持向后兼容。
  - Metric：`metrics.wasm_fuel_exhausted_count / wasm_mem_trap_count / wasm_call_avg_us`（写入 mox_system 的 metrics 或直接日志+JSON 导出）。
**Test Requirements:**
  - TR-9.1 (rule AC-10)：2 条测试 GREEN。
  - TR-9.2 (rule NFR3 after)：H3 重跑，1000 恶意样本 trap 率 = 100%；宿主 RSS 增长 < 5%。
  - TR-9.3 (rubric AC-19 after score)。

---

## Task 10：O4 /system/slo SLO 看板 JSON 接口
**Status:** pending
**Priority:** medium
**Dependency:** O1~O3（因为要导出 O1~O3 的新 metric）
**Scope:**
  - 文件：`platform/backend-node/src/routes/system.js` + 新增 `platform/backend-node/src/slo_metrics.js`（聚合器）。
  - slo_metrics 维护 4 个滑动窗口（1m / 5m / 15m / total）：每个 route × method 对记录 `count, ok_count, err_count, latency_ms_bucket[p50/p95/p99 HDR-like approx histogram or p²]`。
  - 公开函数：`slo.record(route, method, status, latency_ms)`、`slo.snapshot() -> SloJson`。
  - 导出：`GET /system/slo` 返回：
    ```json
    {
      "window_1m":   {"routes": [...], "aggregate": {"p50_ms": 32, "p95_ms": 120, "p99_ms": 240, "success_rate": 0.9982, "total": 1234}},
      "window_5m": {...},
      "window_15m": {...},
      "total": {...},
      "routing_latency_ewma": {"P-A": 410, ...},
      "rate_limit": {"allowed": 4210, "blocked": 31, "tenants": 8},
      "wasm_sandbox": {"fuel_exhausted": 2, "mem_trap": 1, "call_count": 1000},
      "alliance_bench": {"throghput_last": "..." }
    }
    ```
  - Feature flag：`DISABLE_OPTIM_O4_SLO` → `/system/slo` 返回 HTTP 503 Feature-Disabled（向后兼容）。
**Test Requirements:**
  - TR-10.1 (rule AC-11)：新单元测试 `mocha test/mocha_system_slo.js` 至少 5 条（字段完整 + 滑窗正确 + 未启用 503）全部 GREEN。
  - TR-10.2 (rule)：返回 JSON 通过 JSON Schema 校验（p50_ms/p95_ms/p99_ms/success_rate 必填）。
  - TR-10.3 (rubric AC-21 / AC-11 SLO completeness)。

---

## Task 11：O5 工作流并发扇出 ParallelNode + 取消传播 CancellationToken
**Status:** pending
**Priority:** high
**Dependency:** 无（与 O1~O3 并发，对 Rust crate 独立）
**Scope:**
  - 文件：`platform/services/ai-agent/src/flow_engine.rs` + （如无已有 `workflow_node` 模块）新建 `platform/services/ai-agent/src/engine/workflow_nodes.rs`。
  - 新增：
    (a) `pub struct CancellationToken`（含 `AtomicBool cancelled` + `tokio::sync::Notify` 单飞唤醒）；
    (b) `pub struct ParallelNode { children: Vec<NodeId>, concurrency_limit: usize, cancel_on_first_err: bool }`。
    (c) `flow_engine.execute_parallel(ctx, node, token)` 语义：
        - 使用 `Semaphore(concurrency_limit)`；
        - `futures::stream::FuturesUnordered` 并发；
        - 如果 `cancel_on_first_err=true`，第一个 child Err → token.cancel() → 其他 children 在 await 点/下一个检查点退出；
        - 返回 `Vec<Result>`，且对"被取消但未产生错误"的条目标记为 `Err(Cancelled)`。
  - 2 条 `#[test]`：
    (1) 并发=8，16 个 child 每个 100ms 延迟，总耗时应接近 `ceil(16/8)*100ms = 200ms`（允许 ±30%）—— 证明真并发；
    (2) 启动 4 child（3 个 1s 延迟，1 个 10ms 返回 err），断言 5s 内全部完成（取消生效），且至少有 Cancelled 标记的 child。
  - Feature flag：`DISABLE_OPTIM_O5_PARALLEL_NODE=1`，ParallelNode::execute 退化成串行。
  - Metric：H4 关联：输出 `flow.parallel.total_spawned / flow.parallel.cancelled_count / flow.parallel.p99_ms`。
**Test Requirements:**
  - TR-11.1 (rule AC-12)：2 条 Rust test GREEN。
  - TR-11.2 (rubric AC-21 / D07 工作流引擎打分)。
  - TR-11.3 (rule)：与 H4 harness 兼容（H4 after 时记录 O5 吞吐提升相对 before）。

---

## Task 12：T11 汇总 before-after 数据 + 补 8+ 补丁自测 + T10-replay-all.ps1 一键重放
**Status:** pending
**Priority:** high
**Dependency:** T1~T11 全部
**Scope:**
  - (A) 对每一个 O1~O5（再加上从 T2 差距分析里挑选的 3 条 "Medium" 补丁凑足 8 条，比如 O6 RAG 文件切块"重叠窗口 + 标题感知"、O7 图谱算法 7 算法 P99 延迟指标上报 /system/slo、O8 前端 SLO 仪表盘 Vue 组件新增 SloDashboard.vue 调用 /system/slo）—— 若用户在 OQ1~OQ3 有选则按其选择，默认用 O6/O7/O8。
  - (B) 为 8+ 补丁写独立自测（每个补丁 ≥ 2 条，累计 ≥ 16 条新用例）。
  - (C) 重跑全部 Harness 产出 h1_after.csv ~ h4_after.csv。
  - (D) 生成 `T10-harness-summary.md`（1 张 before-after 对比表）。
  - (E) 编写 PowerShell 脚本 `scripts/T10-replay-all.ps1`，一键按顺序：T1→T2 对比矩阵 + 差距分析 → H1~H4 before → O1~O8 打补丁（如需 feature flag 显式启用）→ H1~H4 after → T10-harness-summary.md 产出，每一步失败即 exit≠0，并打印 human-readable 失败原因。
**Deliverables:**
  - `.trae/specs/.../harness-data/h{1,2,3,4}_after.csv`
  - `.trae/specs/.../T10-harness-summary.md`（AC-16）
  - `scripts/T10-replay-all.ps1`（AC-24）
**Test Requirements:**
  - TR-12.1 (rule AC-13)：8+ 补丁全部 feature flag 可关（对每个补丁存在 `DISABLE_OPTIM_Ox`）。
  - TR-12.2 (rule AC-14)：Clippy 零告警（exit=0）。
  - TR-12.3 (rule AC-15)：Mocha ≥ 146 passes，failures=0。
  - TR-12.4 (rule AC-16)：T10-harness-summary.md 存在且至少 4 组 before-after 配对。
  - TR-12.5 (rubric AC-17)：before-after P99 平均改善率评分。
  - TR-12.6 (rubric AC-21)：8+ 补丁的 flag/test/doc/metric 齐全率。
  - TR-12.7 (rubric AC-22)：对 P0 主契约的侵入性评分。
  - TR-12.8 (rubric AC-23)：Harness 可重跑性（全 mock 不依赖外部）评分。
  - TR-12.9 (rubric AC-24)：T10-replay-all.ps1 完整性评分。

---

## Task 13（Review）：独立审查对比维度 + 优化效果 + 试验证据
**Status:** pending
**Priority:** high
**Review Gate:** 在 T1~T12 全部 completed（队列 drain）后方可进入。
**Reviewer Contract（独立 Review）：**
  1. 对 T1 矩阵，抽查 P0 × 至少 6 个维度（D03/D05/D06/D07/D08/D11）的分数与证据链接：
     - 检查 P0 证据路径（绝对路径）文件真实存在；
     - 检查 P1~P4 的 evidence URL 格式正确（http/https）。
  2. 对 T2 差距分析：抽查至少 3 个 Critical/High 是否真的对应到至少一个优化任务。
  3. 对 Harness 数据（T3~T6）：独立重跑至少 1 个（推荐 H1）并比较 CSV 列名与行数是否与 T10 汇总一致。
  4. 对 O1~O5 优化代码：抽查"新增函数 / 新增类 / 新增接口"至少 1 个：
     - Clippy 不报错；
     - Feature flag 存在且能关闭。
  5. 对 T12 replay 脚本：执行 `T10-replay-all.ps1 -Smoke true`（Smoke 模式），至少跑对比矩阵 + H1 基线 + O1/O2 自测 + summary 生成，确认 exit=0。
  6. 根据抽查结果与原 T1-T12 证据，对 8 个 rubric 独立打分（不得直接照抄 Implementer 自评，必须给出自己的证据）。
  7. 最终 Review Result = `pass` 当且仅当：
     - 所有 Rule AC 有独立证据（抽查复跑未发现反例）；
     - 所有 Rubric ≥ 阈值（spec §八 AC-17~24 均 ≥ 80）。
**Deliverable（仅 Review 阶段可写）:**
  - `.trae/specs/20260823-enterprise-compare-top-oss-ai-products-optimize/review.md`

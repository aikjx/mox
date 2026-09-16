# 可观测性 + 容错 企业级就绪度差距核查

- 范围：网关 `mox-platform-gateway-svc`(:3080)、编排器 `mox-platform-orchestrator-svc`(:3001)、联盟调度器 `mox-alliance-scheduler-svc`(:3100)、联盟执行器 `mox-alliance-executor-svc`(:3200)，以及内嵌/依赖的代表性域 svc（kg-service-svc、kb-server、cloud-server、iam-server、rbac-engine）。
- 方法：只读源码核查，每条结论落到 crate / 文件 / 行号。未改动任何代码。
- 分级标尺：**P0** = 上生产必须缺了会出事；**P1** = 加固；**P2** = 锦上添花。
- 核查时间：2026-09-16。

---

## 0. 已验证样板（照抄即可）

| 能力 | 样板位置 | 说明 |
|---|---|---|
| Prometheus 文本指标族 | `platform/gateway/mox-platform-gateway-svc/src/o11y.rs:36-154`（`MetricsCollector`），挂载 `lib.rs:285` `GET /metrics` | 真实 CounterVec/HistogramVec/Gauge，Prometheus Registry + TextEncoder；中间件 `actuator.rs:814 observability_middleware` 在请求入口 `record_request/active_inc/active_dec`。 |
| 下游探活 health | `platform/domains/alliance/svc/mox-alliance-scheduler-svc/src/routes.rs:53-80`（`health_check`） | 1.5s 超时调 `{executor_base_url}/health`，body 写 `dependencies.executor: up/down`；HTTP 状态码恒 200，靠 body 摘流。 |
| 原子业务指标 | `platform/domains/alliance/core/mox-alliance-scheduler-core/src/metrics.rs:18-151`（`AllianceMetrics`） | 纯 `AtomicU64` 无锁计数器（match/llm/fusion/dag 四维），`snapshot()` JSON 序列化，由 scheduler `routes.rs:83 metrics_handler` 暴露。 |
| 指数退避重试 | `platform/domains/alliance/core/mox-alliance-executor-core/src/expert_executor.rs:38-56, 77-79, 321-389`（`consult_with_retry`） | `timeout_ms=300s / max_retries=3 / initial=1s / max=30s / factor=2.0`；`tokio::time::timeout` 包裹单次调用；遇 "circuit-broken" 字符串直接放弃重试（`:133-134, 357-362`）。 |
| 每 provider 熔断 | `platform/domains/alliance/core/mox-alliance-scheduler-core/src/llm_router.rs:36,55,79-85,179-180,436-441,477-485` | `ProviderHealth::CircuitBroken` 三态，`circuit_break_threshold=5`，`circuit_break_duration=60s`，到期自动 HalfOpen 探测。 |
| 统一运行时（已接入域 svc） | `platform/shared/mox-server-runtime/src/server.rs:56-137` | 自动挂 `/health/live` `/health/ready` `/metrics`，并装配 `TraceLayer` + `TimeoutLayer` + `RateLimitLayer` + `circuit_breaker_middleware`。 |

---

## A. 可观测性

### A.1 端点清单（/metrics 与 /health）

| 进程 | :端口 | /metrics | /health | 备注 |
|---|---|---|---|---|
| gateway | 3080 | ✅ 真 Prometheus 文本（`o11y.rs` + `lib.rs:397 metrics_handler`） | ⚠️ **假活**：`lib.rs:346 health_handler` 恒返回 `{"ok":true}`；`/actuator/health`（`actuator.rs:927`）也恒 `UP`，**不探任何下游** | 另有 `/actuator/metrics` JSON 概览（`actuator.rs:1034`）。 |
| orchestrator | 3001 | ❌ **无** Prometheus `/metrics`（仅业务级 `/ai/engine/metrics`，`routes/ai_engine.rs:71`） | ⚠️ **假活**：`main.rs:1098 health` 只挂在 `/api/health`，返回硬编码字符串 `"OK - AI Operator System v3.0..."`；**根路径无 `/health`** | `tracing_subscriber` 已初始化（`main.rs:283-287`），但无指标导出。 |
| alliance-scheduler | 3100 | ✅ JSON 快照（`routes.rs:83 metrics_handler` → `AllianceMetrics.snapshot()`） | ✅ **真下游探活**：`routes.rs:53-80` 调 executor `/health`，body 报 `dependencies.executor` | 指标是 JSON 不是 Prometheus 文本，Grafana 抓需转一层。 |
| alliance-executor | 3200 | ❌ **无** `/metrics`（`routes.rs:74-88` 只挂 `/health` + `/tasks/*` + `/internal/executions`） | ⚠️ 半真：`routes.rs:91 health_check` 报本地 `execution_ready` 标志位（启动期一次性设置，`server.rs:83-97`），**不探 LLM provider 下游** | 内部已有 `ExecutorStats` 原子计数（`expert_executor.rs:151-172, 224`），未导出 HTTP。 |
| kg-server / kb-server / cloud-server / iam-server | 独立端口 | ✅ 经 `mox-server-runtime` 自动挂 `/metrics`（`server.rs:74, 228-243`） | ✅ 经 `mox-server-runtime` 自动挂 `/health/live` + `/health/ready`（`server.rs:71-74, 195-226`） | 这四个 crate 在各自 `Cargo.toml:23` 引入 `mox-server-runtime`，零业务代码即拿到三件套。 |
| kg-service-svc（网关内嵌） | — | 跟随网关 `/metrics` | 跟随网关 `/health` | 库形态（`http_adapter.rs`），无独立端口；内部已用 `mox-resilience-core`（`ac15_faults.rs`、`trace_8stages.rs`）。 |
| rbac-engine（foundation/mox-rbac-engine） | — | 库形态，无独立 HTTP | 库形态，无独立 HTTP | 被网关进程内嵌，健康/指标走网关外壳。 |

### A.2 tracing 贯通情况

- **网关入口**：`actuator.rs:814 observability_middleware` 维护 `x-request-id`（客户端传入则透传，否则生成 UUID v4），并在请求/响应头双向写回（`:823-837, :861-865`）。反代 `proxy.rs:129-147` 全量转发 headers，**字符串层面** x-request-id 能流到 :3001。
- **下游消费**：
  - orchestrator：`main.rs:283-287` 只装了 `tracing_subscriber::fmt` + `EnvFilter`，**没有任何中间件读取 x-request-id / traceparent**。
  - scheduler：`routes.rs:32-47` 只解析 `X-Tenant-Id` / `X-User-Id`，**不读 trace 头**。
  - executor：`routes.rs:24-30` 同样只读 `X-Tenant-Id`。
- **W3C TraceContext**：仓库里其实有两套正确实现——`platform/foundation/mox-platform-observability/src/tracing_ctx.rs:50-90`（解析/序列化 `traceparent`）和 `platform/shared/mox-unified-contract/src/trace.rs:115-223`——但**网关、orchestrator、scheduler、executor 四个进程均未接入**。`mox-server-runtime/src/tracing_utils.rs:6` 仅在域 svc 内部使用。
- 结论：**tracing 不是端到端贯通**。网关有一个自造的 `x-request-id` 字符串透传，但下游既不接收入 span，也不回传，更没有 W3C traceparent；`tracing::info!` 宏在各进程里只是本地日志行。

---

## B. 容错

### B.1 `mox-resilience-core` 原语清单

`platform/shared/mox-resilience-core/src/`：

| 原语 | 文件 | 能力 |
|---|---|---|
| `BackoffStrategy` | `retry.rs:10-58` | Fixed / Exponential / ExponentialWithJitter |
| `RetryPolicy` | `retry.rs` | max_retries、should_retry、retry_delay |
| `Retryable` | `retry.rs` | 可重试错误判定 trait |
| `CircuitBreaker` | `circuit_breaker.rs` | Closed/Open/HalfOpen 三态，failure_rate_threshold / minimum_requests / window_size / open_duration / half_open_max_requests |
| `Fallback` / `StaticFallback` / `FunctionFallback` / `NoFallback` | `fallback.rs` | 静态值/函数/无降级 |
| `ResilienceExecutor<T>` | `lib.rs:27-158` | 组合 Retry + CB + Fallback 的 builder |
| `ResilienceMetrics` / `SharedMetrics` | `metrics.rs` | 弹性指标聚合 |

### B.2 谁真正接了 `mox-resilience-core`

Grep `mox_resilience_core` / `ResilienceExecutor` / `CircuitBreaker` / `RetryPolicy` 全仓 `.rs`：

| crate | 是否接线 | 位置 |
|---|---|---|
| `mox-server-runtime` | ✅ 封装 | `server.rs:39,58-60,124-136`（`CircuitBreakerRegistry` + `circuit_breaker_middleware`）、`resilience.rs`、`resilience_metrics.rs` |
| `mox-event-core` | ✅ | `dead_letter.rs` |
| `mox-framework` | ✅ | `foundation/mox-framework/src/resilience.rs` |
| `mox-kg-service-svc` | ✅ | `ac15_faults.rs`、`trace_8stages.rs` |
| `mox-flow-unified-platform` | ✅（自实现一份 CB） | `circuit_breaker.rs`、`platform_facade.rs` |
| **mox-platform-gateway-svc (3080)** | ❌ **未接** | 仅在 `experts_dispatcher.rs:135` 用自己写的 `circuit_breaker_threshold` 原子计数（进程内专家分发，非 HTTP） |
| **mox-platform-orchestrator-svc (3001)** | ❌ **未接** | 无任何 import |
| **mox-alliance-scheduler-svc (3100)** | ❌ **未接**（用自实现 LLM provider CB） | `llm_router.rs` 自写，未复用 resilience-core |
| **mox-alliance-executor-svc (3200)** | ❌ **未接**（用自实现退避） | `expert_executor.rs` 自写指数退避，未复用 resilience-core |

### B.3 各进程后连调用的超时 / 重试 / 熔断 / 限流

| 调用方 → 目标 | 超时 | 重试+退避 | 熔断 | 限流 | 位置 |
|---|---|---|---|---|---|
| gateway → orchestrator:3001（catch-all `/api/{*path}`） | ✅ 120s（connect 5s） | ❌ | ❌ | 网关入口有令牌桶（`lib.rs:333-336 rate_limit_middleware`） | `proxy.rs:53-58` |
| gateway → PrimiFlow:8000（`/api/projects/*`） | ✅ 120s（同上） | ❌ | ❌ | 同上 | `proxy.rs:49-60` |
| gateway → alliance http-sdk | ✅ `REMOTE_TIMEOUT_SECS` | ❌ | ❌ | — | `alliance/sdk/mox-alliance-http-sdk/src/alliance_remote.rs:82-83` |
| gateway → melody2score:8012 | ✅ 15s | ❌ | ❌ | — | `gateway/src/voice.rs:36-37` |
| gateway → integration connector | ✅ 30s | ❌ | ❌ | — | `gateway/src/integration/connector.rs:58-59` |
| orchestrator → voice:30010 | 透传，无统一超时层 | ❌ | ❌ | ❌ | `routes/voice_proxy.rs:33,84` |
| scheduler → executor:3200（proxy GET） | ✅ 10s | ❌ | ❌ | ❌ | `scheduler/routes.rs:286-289` |
| scheduler → executor:3200（HTTP bridge submit） | ✅ 30s | ❌ | ❌ | — | `scheduler/server.rs:156-159 HttpExecutorBridgeConfig.timeout_ms` |
| scheduler → 远程 expert service | ✅ `es.timeout_ms` | ❌ | ❌ | — | `scheduler/server.rs:270-276` |
| scheduler → LLM provider | ✅ 经 `ExpertExecutorConfig.timeout_ms` | ✅ 退避（在 executor 侧） | ✅ 自写 per-provider CB | — | `llm_router.rs:477-485` |
| executor → LLM provider | ✅ `tokio::time::timeout`（默认 300s） | ✅ 指数退避（1s→30s，×2，3 次） | ⚠️ 被动识别（字符串匹配 "circuit-broken"） | — | `expert_executor.rs:302-314, 321-389, 133-134` |

---

## C. 紧凑差距矩阵

图例：✅ 已有 / ⚠️ 部分或假活 / ❌ 缺失。

### C.1 可观测性

| 维度 \ 进程 | gateway:3080 | orchestrator:3001 | scheduler:3100 | executor:3200 | kg/kb/cloud/iam-server |
|---|---|---|---|---|---|
| /metrics 真指标 | ✅ Prom 文本 | ❌ 无 | ✅ JSON 快照（非 Prom） | ❌ 无 | ✅ 经 server-runtime |
| /health 真探下游 | ❌ 恒 200 假活 | ❌ `/api/health` 硬编码 | ✅ 探 executor | ⚠️ 仅本地 flag | ✅ `/health/ready` 真检查 |
| tracing span 入口 | ⚠️ 自造 x-request-id | ⚠️ fmt subscriber | ❌ | ❌ | ✅ TraceLayer |
| tracing 跨进程贯通 | ❌ 下游不消费 | ❌ | ❌ | ❌ | — |
| 在线日志/管理面 | ✅ /actuator/* | ❌ | ❌ | ❌ | — |

### C.2 容错

| 维度 \ 进程 | gateway:3080 | orchestrator:3001 | scheduler:3100 | executor:3200 |
|---|---|---|---|---|
| 出向超时 | ✅ 120s/30s/15s | ❌ 无统一层 | ✅ 10s/30s | ✅ 300s |
| 重试+退避 | ❌ | ❌ | ❌ | ✅ 指数退避（自写） |
| 熔断器 | ⚠️ 仅进程内专家分发计数 | ❌ | ✅ 自写 per-provider CB | ⚠️ 被动字符串识别 |
| 入口限流 | ✅ 令牌桶 | ❌ | ❌ | ❌ |
| 接入 mox-resilience-core | ❌ | ❌ | ❌ | ❌ |
| 接入 mox-server-runtime | ❌ | ❌ | ❌ | ❌ |

---

## D. 分级整改清单

### P0（上生产必须补）

1. **executor:3200 缺 `/metrics`**（`routes.rs:74-88`）。直接照 scheduler 样板：在 `app_state.rs` 注入 `Arc<ExecutorMetrics>`（原子计数 task/submit/cancel/node_complete），`build_router` 加一行 `.route("/metrics", get(metrics_handler))`，复用 `scheduler-core/metrics.rs:18-151` 的 `AtomicU64` 模式。
2. **orchestrator:3001 缺 Prometheus `/metrics`**。最小补法：照 `gateway/o11y.rs:36-154` 引入 `prometheus` crate，在 `main.rs:444` 附近加 `.route("/metrics", get(metrics_handler))`，先出 `mox_orch_requests_total / duration / active_requests` 三个族。
3. **gateway /health 假活**（`lib.rs:346`）。照 scheduler `routes.rs:53-80` 改：对 orchestrator:3001、scheduler:3100、executor:3200 各做一次 1.5s 探活，body 写 `dependencies.{orch,sched,exec}`；HTTP 状态码仍 200，靠 body 让 K8s readiness 摘流。`/actuator/health`（`actuator.rs:927`）同步改。
4. **orchestrator 无根 `/health`**（只有 `/api/health`，`main.rs:444,1098`）。kubelet / 网关探活约定俗成走 `/health`，加一行 `.route("/health", get(health))`。

> 以上 4 条全部是"缺了 /metrics 或 /health 这种低风险、可直接照 alliance 样板补"的 P0，工程量各半天以内。

### P1（加固）

5. **gateway 反代无重试/熔断**（`proxy.rs:53-58, 172-192`）。orchestrator 闪断一次，前端立刻 502。建议：用 `mox-resilience-core::ResilienceExecutor`（`lib.rs:40-98`）包一层，对 5xx / connect error 做最多 2 次指数退避重试，并在 `ProxyState` 上挂一个 `CircuitBreaker`，open 时直接 503。
6. **scheduler→executor 无重试/熔断**（`routes.rs:286-289`、`server.rs:156-159`）。executor 重启窗口内所有 proxy GET 都 503。照 P0-1 同时补。
7. **tracing 端到端断裂**。网关 `actuator.rs:825` 已在透传 `x-request-id`，但下游不消费。最小改法：在 scheduler/executor 的 `build_router` 加一个 axum middleware，从 `x-request-id` 头取 ID 写入 tracing span 字段，并在出向 reqwest 调用时把它回写到 header。W3C `traceparent` 升级留到 P2。
8. **scheduler 的 `/metrics` 是 JSON 不是 Prometheus 文本**。Grafana/Prometheus 抓取要走 JSON 解析器。建议在 `metrics_handler` 里同时输出 Prom 文本（或直接迁到 `o11y.rs` 的 `prometheus` crate 模式）。
9. **四个进程都没接 `mox-server-runtime`**。kg/kb/cloud/iam 四个域 svc 已经零成本拿到 `/health/live /health/ready /metrics + TraceLayer + TimeoutLayer + RateLimitLayer + CB middleware`；建议把 gateway/orchestrator/scheduler/executor 至少迁到 runtime 的 router 层（不必迁 state），一次性补齐 P0-1~P0-4。

### P2（锦上添花）

10. **统一 W3C TraceContext**：把 `mox-platform-observability/tracing_ctx.rs` 或 `mox-unified-contract/trace.rs` 接到网关入口替换自造 `x-request-id`。
11. **`mox-resilience-core` 与 alliance 自实现 CB 合并**：scheduler `llm_router.rs` 和 executor `expert_executor.rs` 都自写了 CB/退避，建议换成 `mox-resilience-core::CircuitBreaker + RetryPolicy`，避免三套语义漂移。
12. **gateway `/actuator/health` 与 `/health` 语义对齐**：目前 `/actuator/health` 是另一套硬编码 UP，两个探针口径不一致。
13. **orchestrator 入口限流**：业务路由无令牌桶，与网关不对等。

---

## E. 附：关键证据索引

- 网关指标族：`platform/gateway/mox-platform-gateway-svc/src/o11y.rs:53-95`（注册）、`lib.rs:397-411`（`/metrics`）
- 网关假活：`lib.rs:346-353`、`actuator.rs:927-940`
- 网关反代：`proxy.rs:53-58`（client）、`proxy.rs:172-192`（错误处理）
- 调度器真探活：`platform/domains/alliance/svc/mox-alliance-scheduler-svc/src/routes.rs:53-80`
- 调度器原子指标：`platform/domains/alliance/core/mox-alliance-scheduler-core/src/metrics.rs:18-151`
- 执行器退避：`platform/domains/alliance/core/mox-alliance-executor-core/src/expert_executor.rs:38-56, 77-79, 302-314, 321-389`
- LLM 熔断：`platform/domains/alliance/core/mox-alliance-scheduler-core/src/llm_router.rs:36, 55, 79-85, 179-180, 436-441, 477-485`
- 统一运行时：`platform/shared/mox-server-runtime/src/server.rs:56-137, 195-243`
- 弹性原语：`platform/shared/mox-resilience-core/src/lib.rs:27-158`、`retry.rs:10-58`、`circuit_breaker.rs`
- orchestrator 硬编码健康：`platform/domains/platform/svc/mox-platform-orchestrator-svc/src/main.rs:444, 1098-1100`

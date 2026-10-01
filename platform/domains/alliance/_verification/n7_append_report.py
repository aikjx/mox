# -*- coding: utf-8 -*-
import pathlib

p = pathlib.Path(r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\_verification\backend-fix-report.md")

section = """

---

## N7 指标文本化（2026-09-30）

### 背景与现状核证
- 12 号 :30「D6/N7 指标三进程格式不统一」、11 号弱②「无可观测控制台」的补课；
  八节总览原把 N7 列为 P2「scheduler/executor 返回 JSON 快照，补 Prometheus 文本需引入 metrics crate」。
- 本轮**不引入 prometheus crate**，手工渲染 exposition 0.0.4 文本（零新依赖，最小可回退）。
- 核证三 svc 现状（文件:行号）：
  - scheduler:3100 `/metrics` 已存在，返回 `Json(AllianceMetrics.snapshot())`（`svc/mox-alliance-scheduler-svc/src/routes.rs:25` 注册、原 handler 直接 `Json`）。
    **存在消费者**：`tests/http_integration.rs:162 metrics_endpoint_returns_snapshot` 以**无 Accept 头**请求 `/metrics` 并断言 JSON 字段——不能破坏。
  - executor:3200 `/metrics` 已存在，返回 `Json(ExecutorMetrics.snapshot())`（`svc/mox-alliance-executor-svc/src/routes.rs:79` 注册）。
  - registry:3400 **未注册 `/metrics` 路由**（`svc/mox-alliance-registry-svc/src/routes.rs` create_router 原无此行），仅 internal_auth 白名单提到它——需新增。

### 方案：Accept 头协商（不破坏既有 JSON 消费者）
- scheduler / executor：`/metrics` 按 `Accept` 头协商——
  - `Accept` 含 `text/plain`（Prometheus 默认抓取头
    `application/openmetrics-text; version=0.0.1,text/plain;version=0.0.4;q=0.5,*/*;q=0.1` 命中）→
    返回 `text/plain; version=0.0.4; charset=utf-8` Prometheus 文本；
  - 否则（无 Accept / `application/json`，含既有前端控制台与 http_integration 测试）→ **保持原 JSON 快照**，零破坏。
- registry：原本无 `/metrics`、无 JSON 消费者，直接新增文本端点。
- 命名统一 `mox_alliance_<svc>_<metric>`，每条带 `# HELP` / `# TYPE`。

### 改动文件:行号
| 文件 | 位置 | 改动 |
|---|---|---|
| `svc/mox-alliance-scheduler-svc/src/routes.rs` | :17 | 新增 `use mox_alliance_scheduler_core::MetricsSnapshot;` |
| 同上 | :163-181 | `metrics_handler` 加 `headers: HeaderMap`，按 `wants_prometheus_text` 分流文本/JSON |
| 同上 | :184 | 新增 `wants_prometheus_text(&HeaderMap)->bool`（Accept 含 text/plain） |
| 同上 | :193 | 新增 `prom_metric(...)` 助手（HELP+TYPE+sample 一行式） |
| 同上 | :198-220 | 新增 `render_scheduler_metrics_prometheus(&MetricsSnapshot)`：17 条 `mox_alliance_scheduler_*`（match/llm/fusion/dag 四维，counter 加 `_total`，avg 为 gauge） |
| 同上 | 文末 `mod n7_prometheus_tests` | 新增 2 个断言（见测试） |
| `svc/mox-alliance-executor-svc/src/routes.rs` | :23 | `use ...::{ExecutorAppState, ExecutorMetricsSnapshot};` |
| 同上 | :176-194 | `metrics_handler` 同样 Accept 协商分流 |
| 同上 | :197 / :206 / :211-220 | 同构 `wants_prometheus_text` / `prom_metric` / `render_executor_metrics_prometheus`：5 条 `mox_alliance_executor_*`（tasks_submitted/completed、nodes_completed、errors、tasks_cancelled，均 counter） |
| `svc/mox-alliance-registry-svc/src/routes.rs` | :37 | 新增 `.route("/metrics", get(metrics_handler))` |
| 同上 | :74 | 新增 `prom_metric(...)` |
| 同上 | :83-99 | 新增 `metrics_handler`：直接出文本，`mox_alliance_registry_instances`（registry.count）+ `mox_alliance_registry_directory_experts`（dir_store.list().len） |

### 导出指标清单（命名规范 `mox_alliance_<svc>_<metric>`）
- scheduler（17）：`..._match_requests_total`、`..._match_errors_total`、`..._match_latency_us_sum/_count`、`..._match_avg_latency_us`；
  `..._llm_calls_total`、`..._llm_errors_total`、`..._llm_latency_ms_sum/_count`、`..._llm_avg_latency_ms`；
  `..._fusion_calls_total`、`..._fusion_latency_ms_sum/_count`、`..._fusion_avg_latency_ms`；
  `..._dag_executions_total`、`..._dag_node_executions_total`、`..._dag_avg_nodes_per_execution`。
- executor（5）：`..._tasks_submitted_total`、`..._tasks_completed_total`、`..._nodes_completed_total`、`..._errors_total`、`..._tasks_cancelled_total`。
- registry（2）：`mox_alliance_registry_instances`（gauge）、`mox_alliance_registry_directory_experts`（gauge）。

### 测试结果
- `cargo check -p mox-alliance-scheduler-svc -p mox-alliance-executor-svc -p mox-alliance-registry-svc` → Finished dev profile，无 error / 无 warning。
- `cargo test` 三 svc：
  - scheduler-svc：lib 12 + http_integration 11 = **23 passed, 0 failed**
    （新增 `routes::n7_prometheus_tests::prometheus_text_contains_expected_metric_lines`：断言 17 条数据行均为 `mox_alliance_scheduler_* <number>`、值可解析；
    `accept_header_drives_negotiation`：无 Accept / `application/json` → false，Prometheus 默认 Accept → true）
  - executor-svc：lib 9 + http_integration 6 = **15 passed, 0 failed**
  - registry-svc：lib 17 + http_registry 11 = **28 passed, 0 failed**
  - 合计 **66 passed, 0 failed**。
- **兼容性证据**：既有 `metrics_endpoint_returns_snapshot`（无 Accept 头 → 断言 JSON 字段齐备）仍通过，证明默认 JSON 路径零破坏。

### 回退说明
- 改动全部在三 svc 的 `routes.rs`：仅改 `metrics_handler` 分流 + 新增渲染/助手函数 + scheduler 末尾一个测试模块 + registry 注册一行。
- 未引入新依赖、未动快照结构（`AllianceMetrics`/`ExecutorMetrics`）、未动业务 handler。
- 回退 = 把 `metrics_handler` 恢复为 `Json(state.metrics.snapshot())` 并删新增函数；registry 删一行路由即可。
- Prometheus 抓取配置：对三 svc 各加一个 `/metrics` job，Accept 默认即命中文本；现有 JSON 面板继续无 Accept 访问，不受影响。
"""

with p.open("a", encoding="utf-8") as f:
    f.write(section)

print("APPENDED to", p)
print("new size", p.stat().st_size)

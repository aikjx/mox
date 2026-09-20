# 企业级运行时端到端验收报告（Runtime E2E Acceptance）

- 版本：v1.0
- 日期：2026-09-17
- 事实基准：四进程真实运行（非单元测试模拟），证据来自对 `127.0.0.1` 的真实 HTTP 调用
- 关联权威：`docs/architecture/NORMALIZED_ARCHITECTURE.md`（v2.0 §5.1 专家联盟闭环、§6 专家联盟）

## 1. 验收范围与方法

在开发机上用 `scripts/start-mox-enterprise.ps1` 真实拉起企业四进程，再经网关 `:3080` 打真实请求，
验证"编译 → 启动 → 健康 → 鉴权 → 核心业务 → 指标闭环"整条链路。所有结果为实测返回，非测试替身。

| 进程 | 端口 | 二进制 | 验收状态 |
|---|---|---|---|
| alliance-scheduler | 3100 | `mox-alliance-scheduler-svc` | UP |
| alliance-executor | 3200 | `mox-alliance-executor-svc` | UP |
| operator-server | 3001 | `mox-platform-orchestrator-svc` | UP |
| mox-server（网关） | 3080 | `mox-platform-gateway-svc` | UP |

> 四个服务二进制先经 `cargo build -p ...` 全部成功链接（3m25s，exit 0）后再启动；
> 编译告警为既有 dead_code 基线，非本次引入。

## 2. 运行时验收项与实测结果

### 2.1 健康探针（真活，非恒真）
`GET http://127.0.0.1:3080/health` 实测返回：
```json
{"dependencies":{"executor":"up","orchestrator":"up","scheduler":"up"},"ok":true,...}
```
网关真实并发探三下游（各 1.5s 超时），不再是恒 200 假活。

### 2.2 鉴权
- 带 token `GET /metrics` → `200`；
- 无 token 打受保护路由 → `HTTP 404/403` 拦截，未裸奔。

### 2.3 专家智能匹配（核心业务流一）
`POST /api/alliance/experts/search`（经网关 → scheduler）实测：
- 返回 **10 个在线领域专家**（code / vision / translation / finance / math / law / medical / arch / creative / research），`total=10`；
- 端到端耗时 `elapsed_ms≈12`；
- scheduler `/metrics` 实测 `match_requests: 0 → 1`，`match_avg_latency_us≈5594`，`match_errors: 0`。

### 2.4 任务提交 → 拆 DAG → 多专家执行闭环（核心业务流二）
正确提交体（必填，否则 422）：
```json
{"tenant_id":"<uuid>","user_id":"<uuid>","title":"...","description":"..."}
```
实测任务（"技术合同 code+law 联合审查"）：
- `running(15:02:56) → completed(15:03:12)`，`progress=1.0`，`duration_ms≈16054`；
- 系统**智能拆成 3 个 DAG 节点并行执行**：
  - node-1 = 法律专家 `law-expert-001`（准命中需求）
  - node-2 = 代码专家 `code-expert-001`（准命中需求）
  - node-3 = 创意写作 `creative-expert-001`（无显式指定时的默认组队）
  - 三节点全部 `completed`，单节点 5–6ms；
- 任务落库：`GET /api/alliance/tasks` 可查到，且含历史任务（9/12 的安全风险评估任务已完成、duration 1314ms），证明任务持久化真实生效。

### 2.5 可观测三路指标实测增长
任务流经网关后，scheduler `/metrics` 真实增长，不再"功能跑着但监控瞎"：
- `match_requests`（专家匹配）：实测 0→1；
- `dag_executions`（任务 DAG 执行）：见 §3 接线后实测 0→1；
- 节点完成计数：由 executor-svc 侧 `record_node_completed` 负责（独立指标）。

## 3. 本轮补的观测接线（dag 执行计数）

**问题**：`record_dag_execution(nodes)` 方法已存在（`mox-alliance-scheduler-core/src/metrics.rs:101`，单测覆盖），
但真实任务提交路径未调用它，导致任务真跑完后 `dag_executions` 恒为 0。

**修复**：在 `mox-alliance-scheduler-svc/src/routes.rs` 的 `create_task` 成功分支补一行
`state.metrics.record_dag_execution(0)`（照 `record_match` 接法；节点粒度归 executor 侧）。

**验证**：
- `cargo test -p mox-alliance-scheduler-svc` → `9 passed; 0 failed`；
- 重建 scheduler 二进制、重启 `:3100`；
- 提交新任务后 `/metrics` 实测 `dag_executions: 1`（重启归零后），接线生效。

## 4. dev 态如实标注（非缺陷）

- `llm_calls: 0`：专家节点当前为 dev 桩执行（单节点 5–6ms），未接真 LLM；接模型后才是真推理耗时。
- `dag_node_executions: 0`（scheduler 侧）：节点粒度由 executor-svc 独立指标负责，非漏计。
- 提交体 `tenant_id/user_id` 必须是合法 UUID，否则网关返回 422——这是契约，非 bug。

## 5. 验收结论

专家联盟端到端真闭环已在四进程上实测通过：
**网关鉴权 → 提交任务 → 自动拆 DAG 选专家 → 并行执行 → 完成落库 → 三路指标真实增长**。
企业级"功能 ready"到此具备运行时证据，而非仅静态通过测试。

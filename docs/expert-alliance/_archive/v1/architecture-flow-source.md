# 专家联盟权威架构与业务流程 — 可视化源文件

> 本文件是 `architecture-diagram.html` / `business-flow.html` 的唯一事实来源（EA-NORM-001 §2.3.2 第 5 条：源与可视化同目录）。
> 所有端口 / crate 名 / 端点均与代码一致，可视化图不得超出本文件事实，亦不得与之冲突。
> 事实核实日期：2026-09-24。

## 1. 16 crate 分层

| 层 | crate | 说明 |
| --- | --- | --- |
| proto(4) | common-proto, executor-proto, scheduler-proto, registry-proto | gRPC 契约 |
| core(6) | boot-config, config-core, core, executor-core, scheduler-core, registry-core | 纯计算；registry-core 含分级心跳聚合 node→rack→cell（10:1:1） |
| svc(3) | scheduler-svc(:3100), executor-svc(:3200), registry-svc(:3400) | 服务进程 |
| sdk(2) | sdk, http-sdk | 网关内联；RemoteAllianceClient 注入 X-Tenant-Id / X-User-Id / x-request-id |
| api(1) | alliance-api | API 层 |

## 2. 服务拓扑与端口

- 网关 :3080（mox-platform-gateway-svc）：`/api/alliance/*` 唯一入口，经 http-sdk 转发。
- 调度 :3100：排队 / 计划生成 / 专家匹配 / 任务持久化 / 租约选主 leader 对账。
- 执行 :3200：DAG 执行 / 节点专家调用 / 结果融合 / state_sink 持久化。
- 注册 :3400：实例注册 / 发现 / 心跳 / 健康跟踪 / 分级心跳聚合。
- Nacos：配置中心 + 命名注册（boot-config 负责）。

## 3. 数据层

- 调度器：SQLite（`data/alliance_tasks.db`，任务 + 节点 + 租约表同库）或 JSON 文件（file mode）。
- 执行器：SQLite（state_sink；融合输出用保留行 `__fusion_output__`）或 JSON 文件。
- 注册中心：内存 HashMap + JSON 快照 + SQLite（旧目录）。
- registry-core：RocksDB（kg-storage-svc 侧，分级心跳聚合持久化）。

## 4. 任务全生命周期

提交（幂等键 `idempotency_key`）→ 匹配（`ModularWeightMatcher` 权威路径）→ 规划 DAG（6 种协作模式：sequential / parallel / debate / hierarchical / iterative / voting）→ 派发（ExecutorBridge HTTP）→ 执行（DagEngine，节点级专家调用）→ 融合（6 种策略：weighted_voting / confidence_weighting / stacking / debate / map_reduce / iterative_refinement + 6 兼容函数）→ 结果持久化（state_sink `persist_fusion_output`）→ 回读（`read_back` / `read_fusion_output`）→ 人工干预（cancel / pause / resume / retry / skip_node）。

状态机：`can_transition_to` 仲裁表拒绝非法转换（含终态离开）。

## 5. 10 内置专家

expert-code, expert-math, expert-medical, expert-law, expert-finance, expert-creative, expert-vision, expert-translation, expert-research, expert-arch（定义于 `mox-alliance-config-core/src/examples/domain_experts.rs`）。

## 6. 多活选主

- 租约选主（`leadership.rs` 纯函数 decide + `SqliteLeaseStore` BEGIN IMMEDIATE）。
- fencing token epoch +1，每次抢占递增。
- 只在 leader 跑 `reconcile_active_tasks`（扫表对账）。
- HA 默认关闭；`MOX_ALLIANCE_HA_MODE=on` 开启，需 sqlite 存储模式。

## 7. 租户身份贯穿

网关入站 `X-Tenant-Id` / `X-User-Id` → http-sdk `RequestContext::from_headers` → `RemoteAllianceClient::call()` 注入出站头 → 调度器 `tenant_from_headers` → 执行器公共 API 租户校验。

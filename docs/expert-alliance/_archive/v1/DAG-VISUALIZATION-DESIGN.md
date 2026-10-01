# DAG 执行状态实时可视化设计方案

> 🟡 **权威等级：参考**。本文档为前端可视化设计方案，仅供参考；当前实现以代码（platform/domains/alliance/）与 EA-NORM-001 §6 为准。
> **⚠️ 落地状态（2026-09-24 对账）**：本文的**读画面已落地**（`GET /api/alliance/tasks/:id/dag` + 控制台 DAG 页签），但**下方的 API 路径、响应形状与 SSE 事件流是当初的设想，与真实代码不一致**——逐条差异与真实坐标见文末「设计与实现对账」，改前端或改后端前先读那一节，不要照本文件上半部分实现。实现态的权威是 `docs/expert-alliance/FRONTEND-MODULE.md`（EA-DOC-FE-MODULE）§5 与 §8。

## 目标

为专家联盟 DAG 执行引擎提供前端实时可视化能力，让用户可以直观看到：
- 节点执行状态（Pending/Running/Completed/Failed/Skipped）
- 节点间依赖关系
- 执行进度与耗时
- Dynamic 路由决策过程

## 技术选型

| 组件 | 选型 | 理由 |
|------|------|------|
| 图表库 | G6 / X6 | AntV 生态，支持 DAG 拓扑图 |
| 实时通信 | SSE (Server-Sent Events) | 比 WebSocket 更简单，单向推送足够 |
| 前端框架 | Vue 3 + TypeScript | 与现有 frontend-ui 技术栈一致 |

## 后端 API 设计

### 1. 订阅执行进度（SSE）

> ⚠️ **本小节未落地**。Rust 侧不存在 `.../events`：SDK 自己的路由表（`platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance.rs:1801-1825`）里唯一的流式端点是 `GET /api/alliance/tasks/:id/logs/stream`（`:1817`，handler `task_logs_stream` 自 `:1720` 起），它只回放 `LogEntry` 并经 broadcast 通道续推日志行——**没有** `node_started` / `node_completed` / `route_decision` 这类生命周期事件，也只发默认 `message` 事件。前端已按真实形状接（`contract/endpoints.js` 的 `taskLogStream` + `model/normalize.js` 的 `normLogEntry`）。要拿到本节设想的实时性，缺的是**后端新增事件流**，不是前端改造。

```
GET /api/v1/alliance/tasks/:task_id/events
Accept: text/event-stream
```

事件类型：
- `node_started`: 节点开始执行
- `node_completed`: 节点执行完成
- `node_failed`: 节点执行失败
- `route_decision`: Dynamic 路由决策
- `task_completed`: 任务完成
- `task_failed`: 任务失败

### 2. 获取当前 DAG 状态

```
GET /api/v1/alliance/tasks/:task_id/dag
```

响应：
```json
{
  "nodes": [
    {
      "id": "node-1",
      "name": "需求分析",
      "status": "completed",
      "duration_ms": 1200
    }
  ],
  "edges": [
    {"from": "node-1", "to": "node-2"}
  ]
}
```

> ⚠️ **路径与形状都不对**（已落地的那一份长这样）：真实端点是 `GET /api/alliance/tasks/:id/dag`（`alliance.rs:1820`，网关不存在 `/api/v1` 前缀的联盟路由），出参外面还套一层 `{code,msg,data}` 信封，真实载荷是 `data.{task_id,nodes,edges,stats}`（`elapsed_ms` 与 `data` 平级，在信封载荷里：`api_ok(json!({ "elapsed_ms": …, "data": { … } }))`，`:1577-1590`）。节点键为 `id/label/name/type/expert_id/status/progress/dependencies/started_at/completed_at/duration_ms/position{x,y}`（`alliance.rs:1540-1553`），边是 `{source,target,label}`（`:1565-1569`，`label` 恒为 `"依赖"`），**不是**本文的 `{from,to}`；`stats` 为 `{total,completed,running,pending,failed,skipped}` 六键，其中 `skipped` 是后端把 skipped 与 cancelled **折叠**后的数（`node_stats` 返回元组第 6 格 `skipped + cancelled`，`:158`）。此外坐标虽由后端给出（`position`），前端刻意不用它——分层由 `model/dag.js` 从 `dependencies` 算。

## 前端组件设计

### DagViewer 组件

```
┌─────────────────────────────────────────┐
│  DAG 执行可视化                         │
│  ┌──────┐     ┌──────┐     ┌──────┐   │
│  │ node1│────▶│ node2│────▶│ node3│   │
│  │ ✓ 1.2s│     │ ⏱ 45%│     │ ○ 待  │   │
│  └──────┘     └──────┘     └──────┘   │
│                                         │
│  进度: 3/5 节点完成 (60%)              │
│  预计剩余: 2.5s                        │
└─────────────────────────────────────────┘
```

### 状态颜色映射

| 状态 | 颜色 | 图标 |
|------|------|------|
| Pending | 灰色 | ○ |
| Running | 蓝色 | ⏱ |
| Completed | 绿色 | ✓ |
| Failed | 红色 | ✗ |
| Skipped | 黄色 | ⊘ |
| Cancelled | 深灰 | ⊠ |

## 实施步骤

1. **后端**：在 executor-svc 中添加 SSE 端点
2. **后端**：在网关中添加 DAG 状态查询 API
3. **前端**：创建 DagViewer 组件
4. **前端**：集成 SSE 订阅，实时更新节点状态
5. **测试**：端到端验证

## 与现有架构的集成

```
executor-svc (3200)
  └─ SSE /api/v1/tasks/:id/events
       ↓
gateway (3080)
  └─ 代理 /api/alliance/tasks/:id/events
       ↓
frontend-ui (3020)
  └─ DagViewer 组件
```

## 设计与实现对账（2026-09-24，逐条到源码坐标）

| 本文设想 | 真实情况 | 结论 |
|---|---|---|
| 状态集合五个（上文「目标」写 Pending/Running/Completed/Failed/Skipped），颜色表却列了六个 | Rust `NodeExecStatus` 是六态且含 `Cancelled`（`alliance.rs:54-61`），`node_status_str` 六个出口（`:390-399`） | 本文自身不一致，以六态为准。前端 `contract/enums.js` 的 `NODE_STATUS`（另含 proto 侧的 `ready`）与 `NODE_STATUS_LABELS` 六个中文名齐备，DAG 节点样式六种齐备——`contract.test.js` 从 Rust 解析出六个变体后逐个要求「标签 + 样式都在」（P6 守样式漂移） |
| 图表库 G6 / X6 | 落地为零依赖：控制台 DAG 页签按 `dependencies` 分层渲染 DOM 节点（`views/AllianceConsoleView.vue:412-426`），算法在 `model/dag.js` | 不引第三方图库是本模块既有条约（存量构建产物里 `3d-force-graph` 那个 1.3 MB chunk 是反面教材）。真要升级成带连线的拓扑图需另立方案，不能照本节直接引库 |
| SSE 驱动实时更新 | 只有日志流（`alliance.rs:1817`），生命周期事件不存在 | **未落地**。缺的是后端事件流，前端改造补不出来 |
| 「进度: 3/5 节点完成 (60%)」「预计剩余: 2.5s」 | 节点 `progress` 是 `match n.status { Completed => 100, Running => 50, _ => 0 }`（`:1535-1539`）；任务级 `progress()` 才是 `(completed + running * 0.5) / total`（`:161-167`）；剩余时间只有 `estimated_remaining_minutes = (total - completed) * 3`（`:1702`，分钟粒度、常数 3 写死） | 一个字段名两种口径，外加编造估算。界面对节点进度**不渲染**、对剩余时间**不显示**；要落地必须先标注"由状态推导" |
| 实施步骤 1「在 executor-svc 中添加 SSE 端点」、步骤 2「在网关中添加 DAG 状态查询 API」 | 步骤 2 已完成但不在网关：`get_task_dag`（`:1517` 起）在 SDK crate 内，先向 executor-svc 取 `remote_dag`、取不到才本地兜底；这条路由与日志流都挂在 SDK 的路由表上（`:1801-1825`） | 步骤 2 关闭（形态与设想不同），步骤 1 仍未开始 |
| 集成图 `executor-svc(3200) → gateway(3080) → frontend-ui(3020)` | 三个端口本身对（权威为 `docs/api/PORT-REGISTRY.md`），但图上那条 `/api/alliance/tasks/:id/events` 不存在，实际存在的流式路径是 `/api/alliance/tasks/:id/logs/stream` | 图按上一行订正后再引用 |

**这一批真正改掉的**（任务 #18）：DAG 页签此前拿 `dag.edges.length` 当"依赖数"，而分层是从 `nodes[].dependencies` 算的、悬空依赖被 `filter(Boolean)` 静默丢掉 —— 报出来的数字与画出来的图不是同一件事；且分层算法住在 SFC 里。现在算法进 `model/dag.js`（`layoutDag`），页脚四笔账同源：参与分层的依赖数、悬空条数、回边数（DAG 侧没有 Kahn 校验，拓扑检查在 `experts_orchestration.rs:443` 起的计划执行路径，所以成环是可能到界面的形状，截断须声明是截断）、后端边数与依赖清单的差值（这条 1:1 不变量在 `alliance.rs:1560-1573`）。折叠的 `stats.skipped` 不参与显示，跳过与取消按节点级分开计数。测试为 `model/dag.test.js` 11 例 + `contract.test.js` 新增 5 例，P 系列 8 个变异体逐个捕获；实现态权威见 EA-DOC-FE-MODULE（`docs/expert-alliance/FRONTEND-MODULE.md`）§3、§5、§8。

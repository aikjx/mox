# 专家联盟端到端演示报告

> 由 `tools/alliance-demo/alliance_demo.py` 自动生成；**工程事实以机器可读 JSON 证据为准**，本文件仅为同一份数据的可读视图。

## 1. 元信息

| 项 | 值 |
|---|---|
| 生成时间 | 2026-09-23 00:03:47 |
| 网关地址 | http://127.0.0.1:3080 |
| Git HEAD | 031d978d |
| Python | 3.8.8 |
| 健康探针 | /api/v1/status HTTP 200 |
| 运行模式 | remote（网关→调度:3100→执行:3200 三层链路） |
| HTTP 调用总数 | 29 |
| JSON 证据 | reports\data\20260923-000347-alliance-demo-prodcheck.json |

## 1b. 部署拓扑（企业级三层）

| 组件 | 地址 | 状态 | 说明 |
|---|---|---|---|
| 网关（唯一入口） | http://127.0.0.1:3080 | UP | 接收 `/api/alliance/*` |
| 联盟调度 scheduler | :3100 | UP | service=mox-alliance-scheduler executor=up |
| 联盟执行 executor | :3200 | UP | 真实执行专家节点 |

- 运行模式判定：远程（三层） —— 创建的 2/2 个任务确认落在调度器 :3100（远程直接证据）；调度:3100=UP 执行:3200=UP。

## 2. 协作模式对比（AllianceMode × FusionStrategy）

| 模式 | 展示串 | 任务ID | DAG 节点/边 | 终态分布 | 融合策略 | 融合状态 | 置信度 | 步骤通过 |
|---|---|---|---|---|---|---|---|---|
| parallel | expert_alliance | b26a7235 | 1 / 0 | completed=1 | weighted_voting | completed | 0.85 | 9/11 |
| voting | voting | a5980c84 | 1 / 0 | completed=1 | rrf | completed | 1.0 | 9/11 |

## 3. 单模式细节

### 3.1 parallel

- task_id: `b26a7235-d7fa-4158-8489-73b12fd6ce67`
- 下发 fusion_strategy（serde 名）：`weighted` → 网关展示串：`weighted_voting`
- DAG 节点名：代码编程专家
- 执行状态：{"status": "completed", "completed_nodes": 1, "total_nodes": 1, "progress": 1.0}
- 融合置信度：0.85（status=completed）
- Key findings：（无）
- 降级步骤（远程 scheduler 模式下按设计不可用，不影响链路）：
  - resume — DemoError: POST /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67/resume 返回 HTTP 409: {"code": 409, "msg": "InvalidTaskStatus: Can only resume paused task, current status: Running"}
  - toggle_done — DemoError: PUT /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67/toggle-done 返回 HTTP 404: {"code": 404, "msg": "任务 b26a7235-d7fa-4158-8489-73b12fd6ce67 不存在"}

### 3.2 voting

- task_id: `a5980c84-79c3-4683-b35a-b1da6d30598f`
- 下发 fusion_strategy（serde 名）：`voting` → 网关展示串：`rrf`
- DAG 节点名：代码编程专家
- 执行状态：{"status": "completed", "completed_nodes": 1, "total_nodes": 1, "progress": 1.0}
- 融合置信度：1.0（status=completed）
- Key findings：（无）
- 降级步骤（远程 scheduler 模式下按设计不可用，不影响链路）：
  - resume — DemoError: POST /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f/resume 返回 HTTP 409: {"code": 409, "msg": "InvalidTaskStatus: Can only resume paused task, current status: Running"}
  - toggle_done — DemoError: PUT /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f/toggle-done 返回 HTTP 404: {"code": 404, "msg": "任务 a5980c84-79c3-4683-b35a-b1da6d30598f 不存在"}

## 4. 网关 API 延迟

| 端点 | 次数 | p50(ms) | max(ms) |
|---|---|---|---|
| GET /api/alliance/stats | 1 | 6 | 6 |
| GET /api/alliance/tasks | 1 | 20 | 20 |
| GET /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f | 1 | 20 | 20 |
| GET /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f/dag | 1 | 20 | 20 |
| GET /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f/execution-status | 1 | 15 | 15 |
| GET /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f/fusion-result | 1 | 15 | 15 |
| GET /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f/logs | 1 | 10 | 10 |
| GET /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f/nodes | 4 | 15 | 26 |
| GET /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67 | 1 | 32 | 32 |
| GET /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67/dag | 1 | 9 | 9 |
| GET /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67/execution-status | 1 | 8 | 8 |
| GET /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67/fusion-result | 1 | 9 | 9 |
| GET /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67/logs | 1 | 8 | 8 |
| GET /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67/nodes | 4 | 13 | 26 |
| GET /api/v1/status | 1 | 20 | 20 |
| POST /api/alliance/tasks | 2 | 30 | 30 |
| POST /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f/qa | 1 | 8 | 8 |
| POST /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f/resume | 1 | 15 | 15 |
| POST /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67/qa | 1 | 19 | 19 |
| POST /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67/resume | 1 | 7 | 7 |
| PUT /api/alliance/tasks/a5980c84-79c3-4683-b35a-b1da6d30598f/toggle-done | 1 | 19 | 19 |
| PUT /api/alliance/tasks/b26a7235-d7fa-4158-8489-73b12fd6ce67/toggle-done | 1 | 7 | 7 |

## 5. 联盟统计快照

```json
{
  "active_experts": 0,
  "avg_completion_minutes": 0.0,
  "completed_tasks": 0,
  "failed_tasks": 0,
  "running_tasks": 0,
  "success_rate": 0.0,
  "total_experts": 0,
  "total_tasks": 0
}
```

## 6. 仓库治理体检（演示后）

- 结论：**未通过**
- summary：治理体检未通过，请按 failures 逐项修复后重跑 python scripts/verify-ports.py 与 python scripts/check-doc-links.py。
  - 端口漂移校验未通过（ERROR=1）
- 端口门禁：passed=False ERROR=1 WARN=5（扫描 113 个端口）
- 文档门禁：passed=True 断链=0（扫描 366 个文件）

## 7. 结论

- 协作模式覆盖：2 种；HTTP 调用 29 次。
- 步骤通过率：18/22（失败 0，降级 4）。
- 演示本身不写入任何源码与端口配置，体检用于证明这一点。

# 专家联盟端到端演示报告

> 由 `tools/alliance-demo/alliance_demo.py` 自动生成；**工程事实以机器可读 JSON 证据为准**，本文件仅为同一份数据的可读视图。

## 1. 元信息

| 项 | 值 |
|---|---|
| 生成时间 | 2026-09-23 00:28:59 |
| 网关地址 | http://127.0.0.1:3080 |
| Git HEAD | 031d978d |
| Python | 3.8.8 |
| 健康探针 | /api/v1/status HTTP 200 |
| 运行模式 | remote（网关→调度:3100→执行:3200 三层链路） |
| HTTP 调用总数 | 29 |
| JSON 证据 | reports\data\20260923-002859-alliance-demo-prodcheck3.json |

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
| parallel | expert_alliance | f0bf4c35 | 1 / 0 | completed=1 | weighted_voting | completed | 0.85 | 9/11 |
| voting | voting | 40a1f01c | 1 / 0 | completed=1 | rrf | completed | 1.0 | 9/11 |

## 3. 单模式细节

### 3.1 parallel

- task_id: `f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc`
- 下发 fusion_strategy（serde 名）：`weighted` → 网关展示串：`weighted_voting`
- DAG 节点名：代码编程专家
- 执行状态：{"status": "completed", "completed_nodes": 1, "total_nodes": 1, "progress": 1.0}
- 融合置信度：0.85（status=completed）
- Key findings：（无）
- 降级步骤（远程 scheduler 模式下按设计不可用，不影响链路）：
  - resume — DemoError: POST /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc/resume 返回 HTTP 409: {"code": 409, "msg": "InvalidTaskStatus: Can only resume paused task, current status: Running"}
  - toggle_done — DemoError: PUT /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc/toggle-done 返回 HTTP 409: {"code": 409, "msg": "任务 f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc 已完成，远程任务不支持通过网关重新打开"}

### 3.2 voting

- task_id: `40a1f01c-c387-4a08-a383-5e84d00a64e7`
- 下发 fusion_strategy（serde 名）：`voting` → 网关展示串：`rrf`
- DAG 节点名：代码编程专家
- 执行状态：{"status": "completed", "completed_nodes": 1, "total_nodes": 1, "progress": 1.0}
- 融合置信度：1.0（status=completed）
- Key findings：（无）
- 降级步骤（远程 scheduler 模式下按设计不可用，不影响链路）：
  - resume — DemoError: POST /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7/resume 返回 HTTP 409: {"code": 409, "msg": "InvalidTaskStatus: Can only resume paused task, current status: Running"}
  - toggle_done — DemoError: PUT /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7/toggle-done 返回 HTTP 409: {"code": 409, "msg": "任务 40a1f01c-c387-4a08-a383-5e84d00a64e7 已完成，远程任务不支持通过网关重新打开"}

## 4. 网关 API 延迟

| 端点 | 次数 | p50(ms) | max(ms) |
|---|---|---|---|
| GET /api/alliance/stats | 1 | 21 | 21 |
| GET /api/alliance/tasks | 1 | 27 | 27 |
| GET /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7 | 1 | 36 | 36 |
| GET /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7/dag | 1 | 17 | 17 |
| GET /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7/execution-status | 1 | 10 | 10 |
| GET /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7/fusion-result | 1 | 9 | 9 |
| GET /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7/logs | 1 | 10 | 10 |
| GET /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7/nodes | 4 | 14 | 16 |
| GET /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc | 1 | 37 | 37 |
| GET /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc/dag | 1 | 15 | 15 |
| GET /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc/execution-status | 1 | 21 | 21 |
| GET /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc/fusion-result | 1 | 25 | 25 |
| GET /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc/logs | 1 | 19 | 19 |
| GET /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc/nodes | 4 | 22 | 48 |
| GET /api/v1/status | 1 | 51 | 51 |
| POST /api/alliance/tasks | 2 | 55 | 55 |
| POST /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7/qa | 1 | 11 | 11 |
| POST /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7/resume | 1 | 14 | 14 |
| POST /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc/qa | 1 | 27 | 27 |
| POST /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc/resume | 1 | 16 | 16 |
| PUT /api/alliance/tasks/40a1f01c-c387-4a08-a383-5e84d00a64e7/toggle-done | 1 | 9 | 9 |
| PUT /api/alliance/tasks/f0bf4c35-886c-4ebf-bda4-0ddb9bece6dc/toggle-done | 1 | 19 | 19 |

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

- 结论：**通过**
- summary：治理体检通过：端口无漂移、文档无断链，可以提交/发布。
- 端口门禁：passed=True ERROR=0 WARN=3（扫描 113 个端口）
- 文档门禁：passed=True 断链=0（扫描 366 个文件）

## 7. 结论

- 协作模式覆盖：2 种；HTTP 调用 29 次。
- 步骤通过率：18/22（失败 0，降级 4）。
- 演示本身不写入任何源码与端口配置，体检用于证明这一点。

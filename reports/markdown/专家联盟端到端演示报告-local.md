# 专家联盟端到端演示报告

> 由 `tools/alliance-demo/alliance_demo.py` 自动生成；**工程事实以机器可读 JSON 证据为准**，本文件仅为同一份数据的可读视图。

## 1. 元信息

| 项 | 值 |
|---|---|
| 生成时间 | 2026-09-21 17:19:21 |
| 网关地址 | http://127.0.0.1:3080 |
| Git HEAD | b53afe6a |
| Python | 3.8.8 |
| 健康探针 | /api/v1/status HTTP 200 |
| 运行模式 | local |
| HTTP 调用总数 | 133 |
| JSON 证据 | reports\data\20260921-171921-alliance-demo-local.json |

## 1b. 部署拓扑（企业级三层）

| 组件 | 地址 | 状态 | 说明 |
|---|---|---|---|
| 网关（唯一入口） | http://127.0.0.1:3080 | UP | 接收 `/api/alliance/*` |
| 联盟调度 scheduler | :3100 | UP | service=mox-alliance-scheduler executor=up |
| 联盟执行 executor | :3200 | UP | 真实执行专家节点 |

- 运行模式判定：本地 —— 创建的 0/6 个任务确认落在调度器 :3100（远程直接证据）；调度:3100=UP 执行:3200=UP。

## 2. 协作模式对比（AllianceMode × FusionStrategy）

| 模式 | 展示串 | 任务ID | DAG 节点/边 | 终态分布 | 融合策略 | 融合状态 | 置信度 | 步骤通过 |
|---|---|---|---|---|---|---|---|---|
| sequential | single_expert | efd4bba8 | 4 / 3 | completed=1, skipped=3 | first_wins | partial | 0.75 | 14/14 |
| parallel | expert_alliance | 085faa9a | 5 / 5 | completed=1, skipped=4 | weighted_voting | partial | 0.75 | 15/15 |
| debate | debate | b9995edb | 5 / 5 | completed=1, skipped=4 | debate | partial | 0.75 | 15/15 |
| hierarchical | autonomous | e51b1b94 | 7 / 8 | completed=1, skipped=6 | stacking | partial | 0.75 | 17/17 |
| iterative | human_in_loop | 53dd9fab | 6 / 5 | completed=1, skipped=5 | iterative | partial | 0.75 | 16/16 |
| voting | voting | 7c0d5dda | 6 / 7 | completed=1, skipped=5 | rrf | partial | 0.75 | 16/16 |

## 3. 单模式细节

### 3.1 sequential

- task_id: `efd4bba8-d26a-44d1-8047-7bafb4d216f3`
- 下发 fusion_strategy（serde 名）：`best_of` → 网关展示串：`first_wins`
- DAG 节点名：需求分析、方案设计、方案评审、融合输出
- 执行状态：{"status": "pending", "completed_nodes": 1, "total_nodes": 4, "progress": 0.25}
- 融合置信度：0.75（status=partial）
- Key findings：需求分析完成：端到端演示-sequential-171600
- 步骤：全部通过（14 步）

### 3.2 parallel

- task_id: `085faa9a-c904-4b94-80f0-0ace99b79e95`
- 下发 fusion_strategy（serde 名）：`weighted` → 网关展示串：`weighted_voting`
- DAG 节点名：需求分析、架构设计、数据建模、方案评审、融合输出
- 执行状态：{"status": "pending", "completed_nodes": 1, "total_nodes": 5, "progress": 0.20000000298023224}
- 融合置信度：0.75（status=partial）
- Key findings：需求分析完成：端到端演示-parallel-171615
- 步骤：全部通过（15 步）

### 3.3 debate

- task_id: `b9995edb-1349-4b68-bbb9-9a5b168f474b`
- 下发 fusion_strategy（serde 名）：`debate` → 网关展示串：`debate`
- DAG 节点名：需求分析、正方陈述、反方陈述、仲裁裁决、融合输出
- 执行状态：{"status": "pending", "completed_nodes": 1, "total_nodes": 5, "progress": 0.20000000298023224}
- 融合置信度：0.75（status=partial）
- Key findings：需求分析完成：端到端演示-debate-171631
- 步骤：全部通过（15 步）

### 3.4 hierarchical

- task_id: `e51b1b94-15a2-44fe-90e2-9acd62b7fbbe`
- 下发 fusion_strategy（serde 名）：`stacking` → 网关展示串：`stacking`
- DAG 节点名：需求分析、协调规划、子任务·架构、子任务·数据、子任务·实现、专家汇总、融合输出
- 执行状态：{"status": "pending", "completed_nodes": 1, "total_nodes": 7, "progress": 0.1428571492433548}
- 融合置信度：0.75（status=partial）
- Key findings：需求分析完成：端到端演示-hierarchical-171647
- 步骤：全部通过（17 步）

### 3.5 iterative

- task_id: `53dd9fab-b23d-41aa-a34b-616a84480a37`
- 下发 fusion_strategy（serde 名）：`iterative` → 网关展示串：`iterative`
- DAG 节点名：需求分析、初稿生成、人工评审、修订迭代、二次确认、融合输出
- 执行状态：{"status": "pending", "completed_nodes": 1, "total_nodes": 6, "progress": 0.1666666716337204}
- 融合置信度：0.75（status=partial）
- Key findings：需求分析完成：端到端演示-iterative-171702
- 步骤：全部通过（16 步）

### 3.6 voting

- task_id: `7c0d5dda-4982-422c-b133-6a9cf529ccf8`
- 下发 fusion_strategy（serde 名）：`voting` → 网关展示串：`rrf`
- DAG 节点名：需求分析、专家意见·甲、专家意见·乙、专家意见·丙、投票裁决、融合输出
- 执行状态：{"status": "pending", "completed_nodes": 1, "total_nodes": 6, "progress": 0.1666666716337204}
- 融合置信度：0.75（status=partial）
- Key findings：需求分析完成：端到端演示-voting-171718
- 步骤：全部通过（16 步）

## 4. 网关 API 延迟

| 端点 | 次数 | p50(ms) | max(ms) |
|---|---|---|---|
| GET /api/alliance/stats | 2 | 31 | 31 |
| GET /api/alliance/tasks | 1 | 24 | 24 |
| GET /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95 | 1 | 20 | 20 |
| GET /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/dag | 1 | 13 | 13 |
| GET /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/execution-status | 1 | 27 | 27 |
| GET /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/fusion-result | 1 | 31 | 31 |
| GET /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/logs | 1 | 19 | 19 |
| GET /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/nodes | 8 | 19 | 26 |
| GET /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37 | 1 | 26 | 26 |
| GET /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/dag | 1 | 17 | 17 |
| GET /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/execution-status | 1 | 24 | 24 |
| GET /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/fusion-result | 1 | 22 | 22 |
| GET /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/logs | 1 | 24 | 24 |
| GET /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/nodes | 8 | 32 | 44 |
| GET /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8 | 1 | 101 | 101 |
| GET /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/dag | 1 | 22 | 22 |
| GET /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/execution-status | 1 | 26 | 26 |
| GET /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/fusion-result | 1 | 23 | 23 |
| GET /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/logs | 1 | 30 | 30 |
| GET /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/nodes | 8 | 23 | 40 |
| GET /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b | 1 | 14 | 14 |
| GET /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/dag | 1 | 35 | 35 |
| GET /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/execution-status | 1 | 24 | 24 |
| GET /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/fusion-result | 1 | 34 | 34 |
| GET /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/logs | 1 | 26 | 26 |
| GET /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/nodes | 8 | 30 | 42 |
| GET /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe | 1 | 15 | 15 |
| GET /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/dag | 1 | 27 | 27 |
| GET /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/execution-status | 1 | 24 | 24 |
| GET /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/fusion-result | 1 | 22 | 22 |
| GET /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/logs | 1 | 15 | 15 |
| GET /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/nodes | 8 | 22 | 29 |
| GET /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3 | 1 | 32 | 32 |
| GET /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/dag | 1 | 29 | 29 |
| GET /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/execution-status | 1 | 15 | 15 |
| GET /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/fusion-result | 1 | 25 | 25 |
| GET /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/logs | 1 | 21 | 21 |
| GET /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/nodes | 8 | 20 | 25 |
| GET /api/v1/status | 1 | 167 | 167 |
| POST /api/alliance/tasks | 6 | 26 | 29 |
| POST /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/nodes/node-2 | 1 | 19 | 19 |
| POST /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/nodes/node-3 | 1 | 22 | 22 |
| POST /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/nodes/node-4 | 1 | 37 | 37 |
| POST /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/nodes/node-5 | 1 | 19 | 19 |
| POST /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/qa | 1 | 17 | 17 |
| POST /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/resume | 1 | 16 | 16 |
| POST /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/nodes/node-2 | 1 | 29 | 29 |
| POST /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/nodes/node-3 | 1 | 18 | 18 |
| POST /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/nodes/node-4 | 1 | 17 | 17 |
| POST /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/nodes/node-5 | 1 | 24 | 24 |
| POST /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/nodes/node-6 | 1 | 34 | 34 |
| POST /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/qa | 1 | 19 | 19 |
| POST /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/resume | 1 | 21 | 21 |
| POST /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/nodes/node-2 | 1 | 28 | 28 |
| POST /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/nodes/node-3 | 1 | 20 | 20 |
| POST /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/nodes/node-4 | 1 | 23 | 23 |
| POST /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/nodes/node-5 | 1 | 32 | 32 |
| POST /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/nodes/node-6 | 1 | 27 | 27 |
| POST /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/qa | 1 | 19 | 19 |
| POST /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/resume | 1 | 26 | 26 |
| POST /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/nodes/node-2 | 1 | 28 | 28 |
| POST /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/nodes/node-3 | 1 | 24 | 24 |
| POST /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/nodes/node-4 | 1 | 32 | 32 |
| POST /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/nodes/node-5 | 1 | 19 | 19 |
| POST /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/qa | 1 | 29 | 29 |
| POST /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/resume | 1 | 16 | 16 |
| POST /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/nodes/node-2 | 1 | 18 | 18 |
| POST /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/nodes/node-3 | 1 | 14 | 14 |
| POST /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/nodes/node-4 | 1 | 22 | 22 |
| POST /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/nodes/node-5 | 1 | 17 | 17 |
| POST /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/nodes/node-6 | 1 | 23 | 23 |
| POST /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/nodes/node-7 | 1 | 16 | 16 |
| POST /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/qa | 1 | 20 | 20 |
| POST /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/resume | 1 | 24 | 24 |
| POST /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/nodes/node-2 | 1 | 18 | 18 |
| POST /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/nodes/node-3 | 1 | 22 | 22 |
| POST /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/nodes/node-4 | 1 | 24 | 24 |
| POST /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/qa | 1 | 24 | 24 |
| POST /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/resume | 1 | 17 | 17 |
| PUT /api/alliance/tasks/085faa9a-c904-4b94-80f0-0ace99b79e95/toggle-done | 1 | 26 | 26 |
| PUT /api/alliance/tasks/53dd9fab-b23d-41aa-a34b-616a84480a37/toggle-done | 1 | 21 | 21 |
| PUT /api/alliance/tasks/7c0d5dda-4982-422c-b133-6a9cf529ccf8/toggle-done | 1 | 19 | 19 |
| PUT /api/alliance/tasks/b9995edb-1349-4b68-bbb9-9a5b168f474b/toggle-done | 1 | 39 | 39 |
| PUT /api/alliance/tasks/e51b1b94-15a2-44fe-90e2-9acd62b7fbbe/toggle-done | 1 | 16 | 16 |
| PUT /api/alliance/tasks/efd4bba8-d26a-44d1-8047-7bafb4d216f3/toggle-done | 1 | 24 | 24 |

## 5. 联盟统计快照

```json
{
  "total_tasks": 0,
  "running_tasks": 0,
  "completed_tasks": 0,
  "failed_tasks": 0,
  "total_experts": 0,
  "active_experts": 0,
  "avg_completion_minutes": 0.0,
  "success_rate": 0.0
}
```

## 6. 仓库治理体检（演示后）

- 结论：**通过**
- summary：治理体检通过：端口无漂移、文档无断链，可以提交/发布。
- 端口门禁：passed=True ERROR=0 WARN=2（扫描 110 个端口）
- 文档门禁：passed=True 断链=0（扫描 366 个文件）

## 7. 结论

- 协作模式覆盖：6 种；HTTP 调用 133 次。
- 步骤通过率：93/93（失败 0，降级 0）。
- 演示本身不写入任何源码与端口配置，体检用于证明这一点。

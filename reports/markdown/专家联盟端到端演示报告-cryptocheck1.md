# 专家联盟端到端演示报告

> 由 `tools/alliance-demo/alliance_demo.py` 自动生成；**工程事实以机器可读 JSON 证据为准**，本文件仅为同一份数据的可读视图。

## 1. 元信息

| 项 | 值 |
|---|---|
| 生成时间 | 2026-09-23 01:55:35 |
| 网关地址 | http://127.0.0.1:3080 |
| Git HEAD | 844a86fa |
| Python | 3.8.8 |
| 健康探针 | /api/v1/status HTTP 200 |
| 运行模式 | remote（网关→调度:3100→执行:3200 三层链路） |
| HTTP 调用总数 | 29 |
| JSON 证据 | reports\data\20260923-015535-alliance-demo-cryptocheck1.json |

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
| parallel | expert_alliance | 868edc40 | 2 / 0 | completed=2 | weighted_voting | completed | 0.85 | 9/11 |
| voting | voting | a8249918 | 2 / 0 | completed=2 | rrf | completed | 0.5447119471201999 | 9/11 |

## 3. 单模式细节

### 3.1 parallel

- task_id: `868edc40-4455-4cfd-b8b8-1ae3adab1005`
- 下发 fusion_strategy（serde 名）：`weighted` → 网关展示串：`weighted_voting`
- DAG 节点名：自研AI代码引擎、代码编程专家
- 执行状态：{"status": "completed", "completed_nodes": 2, "total_nodes": 2, "progress": 1.0}
- 融合置信度：0.85（status=completed）
- Key findings：分析：作为自研AI代码引擎，针对问题「验证联盟管线在 parallel 模式下的 DAG 构建、执行与融合行为」，从专业领域视角进行分析：该问题涉及技术选型、架构约束与业务目标，关键在于明确需求边界与可量化指标。；方案：1）围绕自研AI代码引擎专业领域设计解决方案；2）分阶段验证，先建立最小可行原型再迭代优化；3）输出可执行交付物并配套质量保障措施。；参考：《自研AI代码引擎领域工程实践指南》—— 璇玑 RelGraph 专家联盟知识库；分析：作为代码编程专家，针对问题「验证联盟管线在 parallel 模式下的 DAG 构建、执行与融合行为」，从专业领域视角进行分析：该问题涉及技术选型、架构约束与业务目标，关键在于明确需求边界与可量化指标。；方案：1）围绕代码编程专家专业领域设计解决方案；2）分阶段验证，先建立最小可行原型再迭代优化；3）输出可执行交付物并配套质量保障措施。
- 降级步骤（远程 scheduler 模式下按设计不可用，不影响链路）：
  - resume — DemoError: POST /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005/resume 返回 HTTP 409: {"code": 409, "msg": "InvalidTaskStatus: Can only resume paused task, current status: Running"}
  - toggle_done — DemoError: PUT /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005/toggle-done 返回 HTTP 409: {"code": 409, "msg": "任务 868edc40-4455-4cfd-b8b8-1ae3adab1005 已完成，远程任务不支持通过网关重新打开"}

### 3.2 voting

- task_id: `a8249918-d2e9-4aed-a98d-1777beb5ff3c`
- 下发 fusion_strategy（serde 名）：`voting` → 网关展示串：`rrf`
- DAG 节点名：自研AI代码引擎、代码编程专家
- 执行状态：{"status": "completed", "completed_nodes": 2, "total_nodes": 2, "progress": 1.0}
- 融合置信度：0.5447119471201999（status=completed）
- Key findings：自研AI代码引擎 的执行输出；代码编程专家 的执行输出
- 降级步骤（远程 scheduler 模式下按设计不可用，不影响链路）：
  - resume — DemoError: POST /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c/resume 返回 HTTP 409: {"code": 409, "msg": "InvalidTaskStatus: Can only resume paused task, current status: Running"}
  - toggle_done — DemoError: PUT /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c/toggle-done 返回 HTTP 409: {"code": 409, "msg": "任务 a8249918-d2e9-4aed-a98d-1777beb5ff3c 已完成，远程任务不支持通过网关重新打开"}

## 4. 网关 API 延迟

| 端点 | 次数 | p50(ms) | max(ms) |
|---|---|---|---|
| GET /api/alliance/stats | 1 | 9 | 9 |
| GET /api/alliance/tasks | 1 | 51 | 51 |
| GET /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005 | 1 | 31 | 31 |
| GET /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005/dag | 1 | 25 | 25 |
| GET /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005/execution-status | 1 | 13 | 13 |
| GET /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005/fusion-result | 1 | 32 | 32 |
| GET /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005/logs | 1 | 24 | 24 |
| GET /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005/nodes | 4 | 14 | 22 |
| GET /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c | 1 | 26 | 26 |
| GET /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c/dag | 1 | 31 | 31 |
| GET /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c/execution-status | 1 | 25 | 25 |
| GET /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c/fusion-result | 1 | 11 | 11 |
| GET /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c/logs | 1 | 14 | 14 |
| GET /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c/nodes | 4 | 15 | 27 |
| GET /api/v1/status | 1 | 23 | 23 |
| POST /api/alliance/tasks | 2 | 46 | 46 |
| POST /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005/qa | 1 | 27 | 27 |
| POST /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005/resume | 1 | 14 | 14 |
| POST /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c/qa | 1 | 32 | 32 |
| POST /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c/resume | 1 | 12 | 12 |
| PUT /api/alliance/tasks/868edc40-4455-4cfd-b8b8-1ae3adab1005/toggle-done | 1 | 12 | 12 |
| PUT /api/alliance/tasks/a8249918-d2e9-4aed-a98d-1777beb5ff3c/toggle-done | 1 | 12 | 12 |

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
- 端口门禁：passed=True ERROR=0 WARN=3（扫描 114 个端口）
- 文档门禁：passed=True 断链=0（扫描 367 个文件）

## 7. 结论

- 协作模式覆盖：2 种；HTTP 调用 29 次。
- 步骤通过率：18/22（失败 0，降级 4）。
- 演示本身不写入任何源码与端口配置，体检用于证明这一点。

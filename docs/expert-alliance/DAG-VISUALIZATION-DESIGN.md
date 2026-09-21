# DAG 执行状态实时可视化设计方案

> 🟡 **权威等级：参考**。本文档为前端可视化设计方案（未落地实现），仅供参考；当前实现以代码（platform/domains/alliance/）与 EA-NORM-001 §6 为准。

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

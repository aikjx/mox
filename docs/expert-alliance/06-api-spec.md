---
doc_id: EA-API-001
title: 专家联盟 API 接口规范
version: V1.0
authority: 🟢权威
last_updated: 2026-09-25
source_of_truth: gateway/src/alliance/*.rs 路由注册
---

# 专家联盟 API 接口规范

## 1. 通用约定

| 项 | 说明 |
|---|------|
| Base URL | `http://{gateway-host}:3080` |
| 认证方式 | Bearer Token（JWT） |
| 请求格式 | `Content-Type: application/json` |
| 响应格式 | `{ code: 0, data: ..., message: "ok" }` |
| 错误格式 | `{ code: 4xx/5xx, message: "错误描述" }` |
| 路由前缀 | `/api/experts/*`（专家域）+ `/api/expert-graph/*`（图谱域） |

---

## 2. API 端点清单（50个）

### 2.1 专家注册与查询（10个）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/experts` | 专家列表 |
| GET | `/api/experts/:id` | 专家详情 |
| GET | `/api/experts/overview` | 专家总览 |
| GET | `/api/experts/stats` | 专家统计 |
| GET | `/api/experts/metrics` | 专家指标 |
| GET | `/api/experts/capabilities` | 能力列表 |
| GET | `/api/experts/:id/metrics` | 单个专家指标 |
| POST | `/api/experts/team` | 组队推荐 |
| POST | `/api/experts/:id/consult-now` | 立即咨询 |
| GET | `/api/experts/bookings/:id/consult-room` | 咨询室信息 |

### 2.2 协作编排（9个）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/experts/:id/consult` | 单专家咨询 |
| POST | `/api/experts/multi-consult` | 多专家联合咨询 |
| POST | `/api/experts/debate` | 多轮辩论 |
| POST | `/api/experts/route` | 智能路由 |
| POST | `/api/experts/intelligent-consult` | 智能咨询 |
| POST | `/api/experts/algorithm-analysis` | 算法分析 |
| POST | `/api/experts/enterprise/consult` | 企业级咨询 |
| POST | `/api/experts/enterprise/analyze` | 企业级分析 |
| POST | `/api/ai/expert-chat` | AI 专家对话 |

### 2.3 编排引擎（6个）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/experts/orchestrate` | 一键编排 |
| POST | `/api/experts/plan/generate` | 生成协作计划（DAG） |
| POST | `/api/experts/plan/execute` | 执行协作计划 |
| GET | `/api/experts/orchestration/stats` | 编排统计 |
| GET | `/api/experts/orchestration/plugins` | 插件列表 |
| GET | `/api/experts/orchestration/history` | 编排历史 |

### 2.4 任务分发器（6个）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/experts/dispatcher/status` | 分发器状态 |
| POST | `/api/experts/dispatcher/dispatch` | 任务分发 |
| POST | `/api/experts/dispatcher/consult` | 分发咨询 |
| POST | `/api/experts/dispatcher/multi-consult` | 多分发咨询 |
| POST | `/api/experts/dispatcher/reset/:id` | 重置单个分发 |
| POST | `/api/experts/dispatcher/reset-all` | 重置全部分发 |

### 2.5 知识图谱（8个）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/expert-graph` | 图谱概览 |
| GET | `/api/expert-graph/stats` | 图谱统计 |
| GET | `/api/expert-graph/neighbors/:id` | 邻居节点 |
| GET | `/api/expert-graph/collaborators/:id` | 合作者 |
| GET | `/api/expert-graph/path/:source/:target` | 最短路径 |
| GET | `/api/expert-graph/communities` | 社区发现 |
| POST | `/api/expert-graph/optimal-team` | 最优团队推荐 |
| POST | `/api/expert-graph/rebuild` | 重建图谱 |

### 2.6 会话管理（6个）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/experts/sessions/stats` | 会话统计 |
| POST | `/api/experts/sessions/:id/messages` | 发送会话消息 |
| POST | `/api/experts/sessions/:id/similar-search` | 相似搜索 |
| GET | `/api/experts/sessions/:id/export` | 导出会话 |
| POST | `/api/experts/sessions/:id/archive` | 归档会话 |
| POST | `/api/experts/semantic-search` | 语义搜索 |

### 2.7 扩展功能（5个）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/experts/bookings/mine` | 我的预约 |
| POST | `/api/experts/bookings` | 创建预约 |
| PUT | `/api/experts/bookings/:id/cancel` | 取消预约 |
| POST | `/api/experts/:id/favorite` | 收藏专家 |
| GET | `/api/ai/engine/flow-graph` | 流程图谱 |

---

## 3. 核心请求/响应示例

### 3.1 生成协作计划

```http
POST /api/experts/plan/generate
Authorization: Bearer <token>
Content-Type: application/json

{
  "task_description": "分析某代码仓库的安全风险并给出修复建议",
  "preferred_mode": "dynamic",
  "fusion_strategy": "weighted"
}
```

```json
{
  "code": 0,
  "data": {
    "task_id": "uuid-xxx",
    "mode": "dynamic",
    "nodes": [
      {
        "node_id": "node-decision",
        "expert_id": "security-expert",
        "status": "pending",
        "dependencies": []
      }
    ],
    "dynamic_routes": [...],
    "expert_weights": { "security-expert": 0.95 }
  }
}
```

### 3.2 多轮辩论

```http
POST /api/experts/debate
Content-Type: application/json

{
  "topic": "微服务架构 vs 单体架构，哪个更适合中型团队？",
  "rounds": 3,
  "judge": "architect-expert"
}
```

```json
{
  "code": 0,
  "data": {
    "rounds": [...],
    "winner": "microservices",
    "judgment": "..."
  }
}
```

---

## 4. WebSocket 接口

| 路径 | 说明 |
|------|------|
| `/ws/v1/experts/tasks/:task_id/progress` | 任务执行进度推送 |

---

## 5. 错误码

| HTTP 状态码 | 业务码 | 说明 |
|------------|--------|------|
| 200 | 0 | 成功 |
| 400 | 400 | 请求参数错误 |
| 401 | 401 | 未认证 |
| 403 | 403 | 无权限 |
| 404 | 404 | 资源不存在 |
| 500 | 500 | 服务器内部错误 |

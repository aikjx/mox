# API 层契约设计文档（API LAYER CONTRACT · ADR-13）

> **文档身份**：8 域 api 层的统一契约设计。定义 REST API 设计标准、DTO 规范、OpenAPI 生成、路由注册模式，以及 kg/ai/flow 三大核心域的 api crate 框架。
> **版本**：v1.0 ENT（2026-08-26 · 开发专家联盟）
> **权威链**：`18` TOP-MASTER > `02` 架构七视图 > `29` 跨域依赖规则 > **本文件(ADR-13)**
> **关联 ADR**：ADR-09(跨域依赖规则5)、ADR-11(网关瘦身)、ADR-14(持久化工作流)

---

## §1 API 层定位

### 1.1 职责

| 职责 | 描述 |
|------|------|
| **路由注册** | 注册本域所有 HTTP 端点，返回 axum `Router` |
| **DTO 定义** | 请求/响应数据结构（Request/Response struct），与领域模型解耦 |
| **输入校验** | 请求参数校验（类型、范围、必填），使用 `validator` crate |
| **OpenAPI 生成** | 自动生成 OpenAPI 3.0 规范，使用 `utoipa` / `aide` |
| **错误映射** | 将领域错误（`AppError`）映射为 HTTP 状态码 + 标准错误响应 |
| **响应封装** | 统一响应格式（code/message/data），分页包装 |

### 1.2 不做

- ❌ 业务逻辑（属于 svc 层）
- ❌ 数据库访问（属于 svc 层）
- ❌ 算法调用（属于 svc 层）
- ❌ 鉴权（由网关统一 middleware 完成）
- ❌ 跨域调用（通过本域 svc 间接调用，或经 sdk）

### 1.3 依赖规则

```
mox-{domain}-api
  ├── mox-{domain}-svc     ✅ 本域 svc
  ├── mox-{domain}-sdk     ✅ 本域 sdk（DTO 复用）
  ├── mox-foundation-*     ✅ foundation
  ├── mox-framework-*      ✅ framework（axum 扩展）
  ├── mox-platform-iam-core ✅（如需用户上下文）
  └── 其他域 svc/core       ❌ 禁止
```

---

## §2 REST API 设计标准

### 2.1 URL 命名

- 格式：`/api/v{version}/{domain}/{resource}`
- 版本：从 v1 开始，不兼容变更升版本号
- 资源：名词复数（`/graphs`、`/workflows`、`/agents`）
- 子资源：`/graphs/{graph_id}/nodes`
- 动作：避免动词在 URL 中，使用 HTTP 方法表达；特殊动作用 `/{resource}/{id}:{action}`（如 `/workflows/{id}:run`）

### 2.2 HTTP 方法

| 方法 | 用途 | 幂等 | 成功状态码 |
|------|------|:----:|:----------:|
| GET | 查询资源 | ✅ | 200 |
| POST | 创建资源/执行动作 | ❌ | 201(创建)/200(动作) |
| PUT | 全量更新 | ✅ | 200 |
| PATCH | 部分更新 | ❌ | 200 |
| DELETE | 删除资源 | ✅ | 204 |

### 2.3 统一响应格式

```json
// 成功
{
  "code": 0,
  "message": "success",
  "data": { ... },
  "request_id": "req_abc123"
}

// 分页成功
{
  "code": 0,
  "message": "success",
  "data": {
    "items": [ ... ],
    "pagination": {
      "page": 1,
      "page_size": 20,
      "total": 156,
      "total_pages": 8
    }
  },
  "request_id": "req_abc123"
}

// 错误
{
  "code": 40001,
  "message": "参数校验失败",
  "details": [
    { "field": "graph_id", "reason": "不能为空" }
  ],
  "request_id": "req_abc123"
}
```

### 2.4 错误码规范

| 码段 | 含义 | 示例 |
|------|------|------|
| 0 | 成功 | 0 |
| 40000-40099 | 参数错误 | 40001=校验失败, 40002=缺少参数 |
| 40100-40199 | 认证错误 | 40101=令牌无效, 40102=令牌过期 |
| 40300-40399 | 权限错误 | 40301=无权限, 40302=角色不足 |
| 40400-40499 | 资源不存在 | 40401=图谱不存在, 40402=工作流不存在 |
| 40900-40999 | 冲突 | 40901=资源已存在, 40902=状态冲突 |
| 50000-50099 | 系统错误 | 50001=内部错误, 50002=服务不可用 |
| 50300-50399 | 限流/降级 | 50301=限流, 50302=熔断 |

**域特定错误码**：`{domain_code}{4位序号}`，如 kg=1, ai=2, flow=3 → kg 错误码 10001-19999

### 2.5 分页标准

- 查询参数：`page`（从1开始）、`page_size`（默认20，最大100）
- 响应：`items` + `pagination` 对象
- 大结果集推荐游标分页：`cursor` + `limit`

### 2.6 排序与过滤

- 排序：`sort=field:asc|desc`，多字段逗号分隔
- 过滤：`filter[field]=value`，支持 `gt`/`lt`/`like`/`in` 操作符

---

## §3 核心域 API 框架

### 3.1 kg 域（知识图谱）API

**Crate**：`mox-kg-api`
**路径**：`platform/domains/kg/api/mox-kg-api/`

**端点清单**：

| 方法 | 路径 | 描述 | 对应 svc |
|------|------|------|----------|
| GET | /api/v1/kg/graphs | 图谱列表 | mox-kg-hub-svc |
| POST | /api/v1/kg/graphs | 创建图谱 | mox-kg-hub-svc |
| GET | /api/v1/kg/graphs/{id} | 图谱详情 | mox-kg-hub-svc |
| PUT | /api/v1/kg/graphs/{id} | 更新图谱 | mox-kg-hub-svc |
| DELETE | /api/v1/kg/graphs/{id} | 删除图谱 | mox-kg-hub-svc |
| GET | /api/v1/kg/graphs/{id}/nodes | 节点列表 | mox-kg-hub-svc |
| POST | /api/v1/kg/graphs/{id}/nodes | 批量添加节点 | mox-kg-hub-svc |
| GET | /api/v1/kg/graphs/{id}/edges | 边列表 | mox-kg-hub-svc |
| POST | /api/v1/kg/graphs/{id}/query | 图谱查询(Cypher/Gremlin) | mox-kg-service-svc |
| POST | /api/v1/kg/algorithms/community | 社区检测(CNM) | mox-kg-algo-core |
| POST | /api/v1/kg/algorithms/pagerank | PageRank 计算 | mox-kg-algo-core |
| POST | /api/v1/kg/algorithms/betweenness | 介数中心性 | mox-kg-algo-core |
| POST | /api/v1/kg/fusion | 图谱融合 | mox-kg-fusion-svc |

### 3.2 ai 域（AI 能力）API

**Crate**：`mox-ai-api`
**路径**：`platform/domains/ai/api/mox-ai-api/`

**端点清单**：

| 方法 | 路径 | 描述 | 对应 svc |
|------|------|------|----------|
| POST | /api/v1/ai/engine/analyze | 统一 AI 分析入口(AC-10) | mox-ai-agent-svc |
| POST | /api/v1/ai/expert/verify | 专家校验(14专家) | mox-ai-expert-svc |
| POST | /api/v1/ai/agent/chat | Agent 对话 | mox-ai-agent-svc |
| POST | /api/v1/ai/agent/invoke | Agent 调用工具 | mox-ai-agent-svc |
| GET | /api/v1/ai/agents | Agent 列表 | mox-ai-agent-svc |
| POST | /api/v1/ai/agents | 创建 Agent | mox-ai-agent-svc |
| GET | /api/v1/ai/models | 可用模型列表 | mox-ai-flow-svc |
| POST | /api/v1/ai/inference | 推理调用 | mox-ai-flow-svc |
| GET | /api/v1/ai/tasks/{id} | 异步任务状态 | mox-ai-agent-svc |

### 3.3 flow 域（工作流）API

**Crate**：`mox-flow-api`
**路径**：`platform/domains/flow/api/mox-flow-api/`

**端点清单**：

| 方法 | 路径 | 描述 | 对应 svc |
|------|------|------|----------|
| GET | /api/v1/flow/workflows | 工作流定义列表 | mox-flow-primiflow-svc |
| POST | /api/v1/flow/workflows | 创建工作流定义 | mox-flow-primiflow-svc |
| GET | /api/v1/flow/workflows/{id} | 工作流详情 | mox-flow-primiflow-svc |
| PUT | /api/v1/flow/workflows/{id} | 更新工作流 | mox-flow-primiflow-svc |
| DELETE | /api/v1/flow/workflows/{id} | 删除工作流 | mox-flow-primiflow-svc |
| POST | /api/v1/flow/workflows/{id}:run | 执行工作流 | mox-flow-primiflow-svc |
| GET | /api/v1/flow/instances | 工作流实例列表 | mox-flow-primiflow-svc |
| GET | /api/v1/flow/instances/{id} | 实例详情(含执行轨迹) | mox-flow-primiflow-svc |
| POST | /api/v1/flow/instances/{id}:cancel | 取消执行 | mox-flow-primiflow-svc |
| GET | /api/v1/flow/operators | 算子列表 | mox-flow-operator-core |
| POST | /api/v1/flow/fusion/execute | 融合执行(PrimiFlow) | mox-flow-fusion-svc |
| POST | /api/v1/flow/optimize | 流程优化(CEM/CPM) | mox-flow-optimizer-core |
| GET | /api/v1/flow/bridges | 桥接器列表 | mox-flow-bridge-svc |

### 3.4 其他域 API（规划中）

| 域 | Crate | 优先级 | 主要端点 |
|----|-------|:------:|----------|
| data | mox-data-api | P2 | 数据源/ETL任务/数据质量/血缘 |
| market | mox-market-api | P2 | 需求/业务流程/模板市场 |
| cloud | mox-cloud-api | P3 | 存储卷/对象存储/计算任务 |
| voice | mox-voice-api | P3 | ASR/TTS/语音会话（或独立部署） |

---

## §4 Crate 结构模板

每个域的 api crate 遵循统一结构：

```
mox-{domain}-api/
├── Cargo.toml
├── src/
│   ├── lib.rs              # pub fn router() -> Router
│   ├── dto/
│   │   ├── mod.rs
│   │   ├── request.rs      # 请求 DTO（带 #[derive(Validate)]）
│   │   └── response.rs     # 响应 DTO（带 #[derive(ToSchema)]）
│   ├── handler/
│   │   ├── mod.rs
│   │   ├── graph.rs        # 每个资源一个 handler 文件
│   │   └── ...
│   ├── error.rs            # 域错误码 + 错误→HTTP 映射
│   └── openapi.rs          # OpenAPI 文档配置
└── tests/
    └── api_test.rs         # 集成测试
```

### 4.1 lib.rs 模板

```rust
pub mod dto;
pub mod handler;
pub mod error;
pub mod openapi;

use axum::Router;

pub fn router() -> Router {
    Router::new()
        .merge(handler::graph::router())
        .merge(handler::algorithm::router())
        .merge(handler::fusion::router())
}
```

### 4.2 handler 模板

```rust
use axum::{extract::State, Json, http::StatusCode};
use crate::dto::{CreateGraphRequest, GraphResponse};
use crate::error::ApiResult;
use mox_kg_hub_svc::GraphService;

pub fn router() -> Router {
    Router::new()
        .route("/graphs", get(list_graphs).post(create_graph))
        .route("/graphs/{id}", get(get_graph).put(update_graph).delete(delete_graph))
}

async fn create_graph(
    State(svc): State<GraphService>,
    Json(req): Json<CreateGraphRequest>,
) -> ApiResult<(StatusCode, Json<GraphResponse>)> {
    req.validate()?;
    let graph = svc.create(req.into()).await?;
    Ok((StatusCode::CREATED, Json(graph.into())))
}
```

---

## §5 OpenAPI 集成

### 5.1 技术选型

- **utoipa**：Rust 生态最成熟的 OpenAPI 3.0 生成库
- **utoipa-swagger-ui**：Swagger UI 嵌入
- **utoipa-rapidoc**：RapiDoc 替代 UI（可选）

### 5.2 文档端点

- `/api/v1/{domain}/openapi.json` — OpenAPI 规范 JSON
- `/api/v1/{domain}/docs` — Swagger UI
- `/api/v1/{domain}/rapidoc` — RapiDoc（可选）

### 5.3 全局 OpenAPI 聚合

网关启动时聚合所有域的 OpenAPI 规范，提供统一文档入口：
- `/api/docs` — 全局 API 文档（所有域聚合）
- `/api/openapi.json` — 全局 OpenAPI 规范

---

## §6 实施路线图

| 阶段 | 时间 | 交付物 | 验收 |
|------|------|--------|------|
| 3.1 | 第5周 | kg-api crate 框架 + 核心端点 | 图谱CRUD+查询可运行 |
| 3.2 | 第5-6周 | ai-api crate 框架 + 核心端点 | AC-10 统一入口可运行 |
| 3.3 | 第6周 | flow-api crate 框架 + 核心端点 | 工作流CRUD+执行可运行 |
| 3.4 | 第7周 | 网关路由迁移到各域 api | arch_test R4 违规=0 |
| 3.5 | 第7-8周 | data/market/cloud api 框架 | 所有域 api crate 存在 |
| 3.6 | 第8周 | OpenAPI 聚合 + 全局文档 | /api/docs 可访问所有域 |

---

## 变更记录

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| v1.0 | 2026-08-26 | 首版：API层定位+REST标准+3核心域端点清单+crate模板+OpenAPI+路线图 | 开发专家联盟 |

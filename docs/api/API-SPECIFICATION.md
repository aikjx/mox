# MOX 平台统一 API 规范 (API-SPECIFICATION)

> 版本：v1.0 · 生效日期：2026-09-03
> 适用范围：MOX 平台所有 HTTP API 端点（网关 + 领域服务）
> 协议实现：`platform/foundation/mox-api-protocol`（Rust crate）

---

## 1. 设计原则

| 原则 | 说明 |
|------|------|
| **最精简** | 不引入不必要的抽象层，统一格式简单易用 |
| **规范标准** | 遵循 RESTful + RFC 7231 + JSON API 惯例 |
| **层次明确** | foundation 定义协议 → framework 提供工具 → domain 使用 → gateway 聚合 |
| **模块化** | 每个领域 API 定义独立，统一协议共享 |
| **向后兼容** | 迁移期保持 API 行为兼容，通过版本前缀管理破坏性变更 |

---

## 2. 统一响应格式

### 2.1 响应体结构

所有 HTTP 端点**必须**返回统一的 `ApiResponse<T>` 结构：

```json
{
  "code": 0,
  "message": "ok",
  "data": { ... }
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `code` | `i32` | 业务状态码：`0` = 成功，非 `0` = 失败（对应 HTTP 状态码或 mox-error 域编码） |
| `message` | `string` | 人类可读消息，成功为 `"ok"` |
| `data` | `T \| null` | 业务数据，失败时省略（`skip_serializing_if`） |

### 2.2 成功响应

```json
{
  "code": 0,
  "message": "ok",
  "data": { "id": "123", "name": "example" }
}
```

- HTTP 状态码：`200 OK`
- `code` 固定为 `0`
- `message` 固定为 `"ok"`

### 2.3 失败响应

```json
{
  "code": 404,
  "message": "节点不存在"
}
```

- HTTP 状态码：与 `code` 字段一致（如 `404`、`500`）
- `data` 字段省略
- `message` 为面向用户的可读错误描述

### 2.4 Rust 构造方式

```rust
use mox_api_protocol::{api_ok, api_error, ApiResponse};

// 成功
async fn handler() -> ApiResponse<String> {
    api_ok("hello".into())
}

// 失败
async fn fail_handler() -> ApiResponse<()> {
    api_error(404, "资源不存在")
}

// 从 MoxError 构建
async fn from_err(err: MoxError) -> ApiResponse<()> {
    ApiResponse::from_mox_error(&err)
}
```

---

## 3. 错误码体系

### 3.1 错误码编码规则

复用 `mox-error` crate 的域编码体系，格式为：

```
{域代码:2}{模块代码:02d}{序号:03d}
```

示例：`KG01001` = 知识图谱域 · 存储模块 · 第 1 号错误

### 3.2 域代码分配

| 域代码 | 业务域 | 说明 |
|--------|--------|------|
| `PL` | Platform | 平台系统（通用错误、配置、验证） |
| `KG` | Knowledge Graph | 知识图谱 |
| `AI` | AI Capability | AI 能力（对话、LLM、Agent） |
| `FL` | Flow | 工作流 |
| `OP` | Operator | 算子引擎 |
| `PJ` | Project | 项目管理 |
| `RS` | Resource | 资源中心 |
| `US` | User | 用户/权限 |
| `DT` | Data | 数据处理 |
| `CL` | Cloud | 云存储 |

### 3.3 HTTP 状态码映射

| HTTP 状态码 | 含义 | 典型场景 |
|-------------|------|----------|
| `200` | 成功 | 正常响应 |
| `400` | 请求参数错误 | 参数校验失败 |
| `401` | 未认证 | 缺少/无效 Token |
| `403` | 无权限 | 权限不足 |
| `404` | 资源不存在 | 节点/会话/用户不存在 |
| `409` | 资源冲突 | 重复创建、版本冲突 |
| `422` | 业务规则校验失败 | 算法参数无效、上下文超限 |
| `429` | 请求过于频繁 | 限流触发 |
| `500` | 内部服务器错误 | 未预期异常 |
| `503` | 服务不可用 | 依赖服务宕机 |
| `504` | 请求超时 | 上游服务超时 |

### 3.4 错误等级

| 等级 | 说明 | 日志级别 |
|------|------|----------|
| `info` | 信息级，不影响主流程 | `INFO` |
| `warning` | 警告级，可能影响部分功能 | `WARN` |
| `error` | 错误级，功能异常 | `ERROR` |
| `critical` | 严重级，系统级故障 | `ERROR` + 告警 |

---

## 4. 版本管理

### 4.1 URL 版本前缀

所有业务 API 必须包含版本前缀：

```
/{domain}/v{major}/{resource}
```

示例：
- `/kg/v1/stats`
- `/alliance/v1/tasks`
- `/ai/v1/engine/process`
- `/api/system/users`（系统管理域，`/api` 为统一前缀）

### 4.2 版本策略

- **主版本号**（`v1` → `v2`）：破坏性变更，旧版本至少保留 6 个月
- **次版本号**：向后兼容的新增功能，不体现在 URL 中
- **Actuator 管理面**：`/actuator/*` 不版本化，属于运维接口

---

## 5. 命名规范

### 5.1 URL 路径

- 使用 **kebab-case**（小写连字符）：`/kg/v1/shortest-path`
- 资源名使用复数：`/alliance/v1/tasks`（非 `/task`）
- 嵌套资源不超过 2 层：`/alliance/v1/tasks/:id/nodes`

### 5.2 JSON 字段

- 使用 **snake_case**：`page_size`、`total_pages`、`created_at`
- 布尔字段用 `is_` / `has_` 前缀：`is_active`、`has_permission`
- 时间字段用 `_at` 后缀：`created_at`、`updated_at`
- ID 字段用 `_id` 后缀：`user_id`、`task_id`

### 5.3 HTTP 方法

| 方法 | 用途 | 幂等 |
|------|------|------|
| `GET` | 查询资源 | ✅ |
| `POST` | 创建资源 / 执行操作 | ❌ |
| `PUT` | 全量更新资源 | ✅ |
| `PATCH` | 部分更新资源 | ❌ |
| `DELETE` | 删除资源 | ✅ |

---

## 6. 分页规范

### 6.1 请求参数

所有列表查询端点统一使用以下查询参数：

| 参数 | 类型 | 默认值 | 范围 | 说明 |
|------|------|--------|------|------|
| `page` | `u32` | `1` | `>= 1` | 页码，从 1 开始 |
| `page_size` | `u32` | `20` | `1 ~ 100` | 每页条数 |

### 6.2 响应结构

分页数据封装在 `data` 字段中，使用 `PaginatedResponse<T>`：

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "items": [ ... ],
    "total": 156,
    "page": 1,
    "page_size": 20,
    "total_pages": 8
  }
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `items` | `Vec<T>` | 当前页数据列表 |
| `total` | `u64` | 总条数 |
| `page` | `u32` | 当前页码 |
| `page_size` | `u32` | 每页条数 |
| `total_pages` | `u32` | 总页数（自动计算） |

### 6.3 Rust 使用方式

```rust
use mox_api_protocol::{PageQuery, api_paged};

async fn list_items(Query(q): Query<PageQuery>) -> ApiResponse<PaginatedResponse<Item>> {
    let (page, page_size) = q.normalized();
    let offset = q.offset();
    let limit = q.limit();
    // ... 数据库查询 ...
    api_paged(items, total, page, page_size)
}
```

---

## 7. 时间格式

### 7.1 统一格式

所有时间字段使用 **RFC 3339**（ISO 8601 子集）格式：

```
2026-09-03T12:00:00Z
```

- 时区：统一使用 UTC（`Z` 后缀）
- 精度：秒级（必要时可到毫秒）
- 存储：数据库中使用 Unix 时间戳（毫秒），API 层转换为 RFC 3339

### 7.2 时间字段命名

| 后缀 | 含义 | 示例 |
|------|------|------|
| `_at` | 时间点 | `created_at`、`updated_at`、`deleted_at` |
| `_until` | 截止时间 | `expires_until` |
| `_duration_ms` | 持续时长（毫秒） | `processing_duration_ms` |

---

## 8. 认证方式

### 8.1 认证方案

| 方案 | Header | 适用场景 |
|------|--------|----------|
| JWT Bearer | `Authorization: Bearer <token>` | 用户会话、前端调用 |
| API Key | `X-API-Key: <key>` | 服务间调用、脚本/自动化 |
| 公开端点 | 无 | `/health`、`/actuator/health`、`/api/v1/status` |

### 8.2 认证失败响应

```json
{
  "code": 401,
  "message": "缺少认证 Token"
}
```

### 8.3 权限不足响应

```json
{
  "code": 403,
  "message": "无权限执行此操作"
}
```

---

## 9. 健康检查端点

### 9.1 标准健康检查

所有 HTTP 服务必须提供：

| 端点 | 方法 | 说明 |
|------|------|------|
| `/health` | `GET` | 存活检查（Liveness），返回 `200` 即表示进程存活 |
| `/ready` | `GET` | 就绪检查（Readiness），依赖项就绪后返回 `200` |

### 9.2 健康检查响应

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "status": "up",
    "version": "3.0.0",
    "uptime_seconds": 86400
  }
}
```

---

## 10. Actuator 管理面

网关提供 Spring Boot 风格的 Actuator 管理端点（不版本化）：

| 端点 | 方法 | 说明 |
|------|------|------|
| `/actuator` | `GET` | 管理端点索引 |
| `/actuator/health` | `GET` | 健康检查 |
| `/actuator/info` | `GET` | 构建信息 |
| `/actuator/mappings` | `GET` | 全部 API 路由注册表 |
| `/actuator/metrics` | `GET` | 运行时指标 |
| `/actuator/env` | `GET` | 网关配置（脱敏） |
| `/actuator/loggers` | `GET/POST` | 日志级别查看/调整 |
| `/actuator/logs` | `GET` | 在线查询日志 |
| `/actuator/logs/tail` | `GET` | SSE 实时日志流 |
| `/actuator/api/:id` | `GET/POST` | 按 API 启停管理 |

---

## 11. 迁移指南

### 11.1 旧格式 → 新格式映射

| 旧格式 | 新格式 |
|--------|--------|
| `Json(json!({"ok": true, ...}))` | `ApiResponse::ok(data)` |
| `Json(json!({"success": true, "data": ...}))` | `ApiResponse::ok(data)` |
| `Json(json!({"error": "..."}))` | `ApiResponse::error(code, message)` |
| 直接返回 `String` / `Json<Value>` | 包装为 `ApiResponse<T>` |

### 11.2 迁移步骤

1. 在 crate 的 `Cargo.toml` 中添加依赖：`mox-api-protocol = { workspace = true }`
2. 将 handler 返回类型从 `Json<serde_json::Value>` 改为 `ApiResponse<T>`
3. 用 `api_ok(data)` / `api_error(code, msg)` 替换 `Json(json!(...))`
4. 列表查询端点使用 `PageQuery` + `api_paged(...)`
5. 错误处理使用 `MoxError` + `ApiResponse::from_mox_error(&err)`

### 11.3 过渡期兼容

- 网关层可在迁移期同时支持新旧格式（通过 `Accept` header 或过渡端点）
- 前端统一按新格式解析，旧格式端点逐步迁移
- 所有新端点**必须**使用新格式，不允许新增旧格式端点

---

## 12. 附录：响应格式速查表

```
成功（单对象）:     { "code": 0, "message": "ok", "data": { ... } }
成功（列表分页）:   { "code": 0, "message": "ok", "data": { "items": [...], "total": N, "page": N, "page_size": N, "total_pages": N } }
成功（无数据）:     { "code": 0, "message": "ok" }
失败（参数错误）:   { "code": 400, "message": "参数验证失败" }
失败（未认证）:     { "code": 401, "message": "缺少认证 Token" }
失败（无权限）:     { "code": 403, "message": "无权限执行此操作" }
失败（不存在）:     { "code": 404, "message": "资源不存在" }
失败（限流）:       { "code": 429, "message": "请求过于频繁，请稍后再试" }
失败（内部错误）:   { "code": 500, "message": "系统内部错误，请稍后重试" }
```

# 错误码参考手册 — Error Code Reference

> **版本**: 3.0.0
> **规范**: 6位数字错误码，前2位分类，后4位序号
> **格式**: `E{6位数字}`，如 `E200001`

---

## 目录

1. [错误码规范](#1-错误码规范)
2. [系统错误 (10xxxx)](#2-系统错误-10xxxx)
3. [AI错误 (20xxxx)](#3-ai错误-20xxxx)
4. [插件错误 (30xxxx)](#4-插件错误-30xxxx)
5. [政企错误 (40xxxx)](#5-政企错误-40xxxx)
6. [连接器错误 (50xxxx)](#6-连接器错误-50xxxx)
7. [集成错误 (90xxxx)](#7-集成错误-90xxxx)
8. [HTTP状态码映射](#8-http状态码映射)
9. [错误响应格式](#9-错误响应格式)

---

## 1. 错误码规范

### 1.1 编码规则

```
E 10 0001
│  │  │
│  │  └── 序号 (0001-9999)
│  └───── 分类 (10-99)
└──────── 前缀 (固定E)
```

### 1.2 分类表

| 前缀 | 分类 | 说明 |
|------|------|------|
| 10xxxx | 系统错误 | 内部错误、超时、不可用、配置错误 |
| 20xxxx | AI错误 | Provider不存在、调用失败、限流、模型不支持 |
| 30xxxx | 插件错误 | 插件不存在、加载失败、权限拒绝、执行错误 |
| 40xxxx | 政企错误 | SSO失败、合规违规、数据主权、白标配置 |
| 50xxxx | 连接器错误 | 连接器不存在、连接失败、超时、认证失败 |
| 90xxxx | 集成错误 | 配置错误、Factory不存在、组装失败、路由错误 |

### 1.3 序号分配规则

| 序号范围 | 说明 |
|---------|------|
| 0001-0099 | 通用错误（不存在、超时、认证失败等） |
| 0100-0199 | 配置相关错误 |
| 0200-0299 | 执行/调用相关错误 |
| 0300-0399 | 限流/熔断相关错误 |
| 0400-0499 | 依赖/外部服务错误 |
| 0500-0999 | 预留扩展 |

---

## 2. 系统错误 (10xxxx)

| 错误码 | 名称 | HTTP状态 | 说明 |
|--------|------|---------|------|
| E100001 | SYSTEM_INTERNAL_ERROR | 500 | 系统内部错误 |
| E100002 | SYSTEM_TIMEOUT | 504 | 系统超时 |
| E100003 | SYSTEM_UNAVAILABLE | 503 | 系统不可用 |
| E100004 | SYSTEM_OVERLOAD | 503 | 系统过载 |
| E100005 | SYSTEM_MAINTENANCE | 503 | 系统维护中 |
| E100101 | SYSTEM_CONFIG_ERROR | 400 | 配置错误 |
| E100102 | SYSTEM_CONFIG_NOT_FOUND | 404 | 配置不存在 |
| E100103 | SYSTEM_CONFIG_INVALID | 400 | 配置无效 |
| E100201 | SYSTEM_RESOURCE_EXHAUSTED | 503 | 资源耗尽 |
| E100202 | SYSTEM_MEMORY_EXHAUSTED | 503 | 内存耗尽 |
| E100203 | SYSTEM_CONNECTION_EXHAUSTED | 503 | 连接池耗尽 |

---

## 3. AI错误 (20xxxx)

| 错误码 | 名称 | HTTP状态 | 说明 |
|--------|------|---------|------|
| E200001 | AI_PROVIDER_NOT_FOUND | 404 | AI Provider不存在 |
| E200002 | AI_PROVIDER_ERROR | 502 | AI Provider调用失败 |
| E200003 | AI_RATE_LIMITED | 429 | AI调用限流 |
| E200004 | AI_MODEL_NOT_FOUND | 404 | 模型不存在 |
| E200005 | AI_MODEL_NOT_SUPPORTED | 400 | 模型不支持该能力 |
| E200006 | AI_ALL_PROVIDERS_FAILED | 502 | 所有Provider都失败 |
| E200007 | AI_CIRCUIT_BREAKER_OPEN | 503 | 熔断器已打开 |
| E200008 | AI_FALLBACK_EXHAUSTED | 502 | 降级链耗尽 |
| E200101 | AI_CONFIG_INVALID | 400 | AI配置无效 |
| E200102 | AI_API_KEY_MISSING | 401 | API Key缺失 |
| E200103 | AI_API_KEY_INVALID | 401 | API Key无效 |
| E200201 | AI_CHAT_FAILED | 502 | 对话调用失败 |
| E200202 | AI_STREAM_FAILED | 502 | 流式对话失败 |
| E200203 | AI_EMBEDDING_FAILED | 502 | 嵌入调用失败 |
| E200204 | AI_RESPONSE_PARSE_ERROR | 502 | 响应解析错误 |
| E200301 | AI_TOKEN_LIMIT_EXCEEDED | 429 | Token限制超限 |
| E200302 | AI_CONTEXT_LENGTH_EXCEEDED | 400 | 上下文长度超限 |
| E200401 | AI_UPSTREAM_TIMEOUT | 504 | 上游AI服务超时 |
| E200402 | AI_UPSTREAM_UNAVAILABLE | 503 | 上游AI服务不可用 |
| E200403 | AI_UPSTREAM_RATE_LIMITED | 429 | 上游AI服务限流 |

---

## 4. 插件错误 (30xxxx)

| 错误码 | 名称 | HTTP状态 | 说明 |
|--------|------|---------|------|
| E300001 | PLUGIN_NOT_FOUND | 404 | 插件不存在 |
| E300002 | PLUGIN_LOAD_FAILED | 500 | 插件加载失败 |
| E300003 | PLUGIN_PERMISSION_DENIED | 403 | 插件权限拒绝 |
| E300004 | PLUGIN_EXECUTION_ERROR | 500 | 插件执行错误 |
| E300005 | PLUGIN_TIMEOUT | 504 | 插件执行超时 |
| E300006 | PLUGIN_CRASHED | 500 | 插件崩溃 |
| E300007 | PLUGIN_DISABLED | 403 | 插件已禁用 |
| E300008 | PLUGIN_VERSION_INCOMPATIBLE | 400 | 插件版本不兼容 |
| E300009 | PLUGIN_DEPENDENCY_MISSING | 400 | 插件依赖缺失 |
| E300010 | PLUGIN_DEPENDENCY_CONFLICT | 400 | 插件依赖冲突 |
| E300101 | PLUGIN_MANIFEST_INVALID | 400 | 插件manifest无效 |
| E300102 | PLUGIN_MANIFEST_MISSING | 400 | 插件manifest缺失 |
| E300103 | PLUGIN_WASM_INVALID | 400 | WASM文件无效 |
| E300104 | PLUGIN_WASM_CORRUPTED | 400 | WASM文件损坏 |
| E300201 | PLUGIN_HOST_API_ERROR | 500 | 宿主API调用错误 |
| E300202 | PLUGIN_HOST_API_NOT_FOUND | 404 | 宿主API不存在 |
| E300203 | PLUGIN_EVENT_PUBLISH_FAILED | 500 | 事件发布失败 |
| E300301 | PLUGIN_MARKET_ERROR | 502 | 插件市场错误 |
| E300302 | PLUGIN_MARKET_NOT_FOUND | 404 | 插件市场插件不存在 |
| E300303 | PLUGIN_INSTALL_FAILED | 500 | 插件安装失败 |
| E300304 | PLUGIN_UNINSTALL_FAILED | 500 | 插件卸载失败 |
| E300305 | PLUGIN_UPGRADE_FAILED | 500 | 插件升级失败 |
| E300306 | PLUGIN_ROLLBACK_FAILED | 500 | 插件回滚失败 |

---

## 5. 政企错误 (40xxxx)

| 错误码 | 名称 | HTTP状态 | 说明 |
|--------|------|---------|------|
| E400001 | ENTERPRISE_SSO_FAILED | 401 | SSO认证失败 |
| E400002 | ENTERPRISE_COMPLIANCE_VIOLATION | 403 | 合规违规 |
| E400003 | ENTERPRISE_SSO_PROVIDER_NOT_FOUND | 404 | SSO Provider不存在 |
| E400004 | ENTERPRISE_SSO_TOKEN_INVALID | 401 | SSO Token无效 |
| E400005 | ENTERPRISE_SSO_TOKEN_EXPIRED | 401 | SSO Token已过期 |
| E400006 | ENTERPRISE_SSO_CALLBACK_ERROR | 500 | SSO回调错误 |
| E400007 | ENTERPRISE_SSO_USER_NOT_FOUND | 404 | SSO用户不存在 |
| E400101 | ENTERPRISE_AUDIT_LOG_FAILED | 500 | 审计日志记录失败 |
| E400102 | ENTERPRISE_AUDIT_CHAIN_BROKEN | 500 | 审计哈希链断裂 |
| E400103 | ENTERPRISE_AUDIT_VERIFICATION_FAILED | 500 | 审计验证失败 |
| E400201 | ENTERPRISE_DATA_MASKING_FAILED | 500 | 数据脱敏失败 |
| E400202 | ENTERPRISE_DATA_MASKING_CONFIG_INVALID | 400 | 数据脱敏配置无效 |
| E400301 | ENTERPRISE_DATA_RESIDENCY_VIOLATION | 403 | 数据主权违规 |
| E400302 | ENTERPRISE_CROSS_BORDER_TRANSFER_DENIED | 403 | 跨境传输被拒绝 |
| E400303 | ENTERPRISE_CROSS_BORDER_APPROVAL_REQUIRED | 403 | 跨境传输需要审批 |
| E400304 | ENTERPRISE_DATA_LOCALIZATION_REQUIRED | 403 | 需要数据本地化 |
| E400401 | ENTERPRISE_WHITELABEL_CONFIG_INVALID | 400 | 白标配置无效 |
| E400402 | ENTERPRISE_THEME_NOT_FOUND | 404 | 主题不存在 |
| E400403 | ENTERPRISE_DYNAMIC_FIELD_INVALID | 400 | 动态字段无效 |
| E400501 | ENTERPRISE_TENANT_NOT_FOUND | 404 | 租户不存在 |
| E400502 | ENTERPRISE_TENANT_DISABLED | 403 | 租户已禁用 |
| E400503 | ENTERPRISE_TENANT_SUSPENDED | 403 | 租户已暂停 |

---

## 6. 连接器错误 (50xxxx)

| 错误码 | 名称 | HTTP状态 | 说明 |
|--------|------|---------|------|
| E500001 | CONNECTOR_NOT_FOUND | 404 | 连接器不存在 |
| E500002 | CONNECTOR_CONNECTION_FAILED | 502 | 连接器连接失败 |
| E500003 | CONNECTOR_TIMEOUT | 504 | 连接器超时 |
| E500004 | CONNECTOR_AUTH_FAILED | 401 | 连接器认证失败 |
| E500005 | CONNECTOR_DISABLED | 403 | 连接器已禁用 |
| E500006 | CONNECTOR_EXECUTION_ERROR | 500 | 连接器执行错误 |
| E500007 | CONNECTOR_RESPONSE_PARSE_ERROR | 502 | 连接器响应解析错误 |
| E500008 | CONNECTOR_RETRY_EXHAUSTED | 502 | 连接器重试耗尽 |
| E500009 | CONNECTOR_CIRCUIT_BREAKER_OPEN | 503 | 连接器熔断器打开 |
| E500101 | CONNECTOR_CONFIG_INVALID | 400 | 连接器配置无效 |
| E500102 | CONNECTOR_ENDPOINT_INVALID | 400 | 连接器端点无效 |
| E500103 | CONNECTOR_PROTOCOL_NOT_SUPPORTED | 400 | 连接器协议不支持 |
| E500104 | CONNECTOR_AUTH_TYPE_NOT_SUPPORTED | 400 | 连接器认证类型不支持 |
| E500201 | CONNECTOR_OPERATION_NOT_SUPPORTED | 400 | 连接器操作不支持 |
| E500202 | CONNECTOR_OPERATION_FAILED | 500 | 连接器操作失败 |
| E500301 | CONNECTOR_RATE_LIMITED | 429 | 连接器限流 |
| E500302 | CONNECTOR_QUOTA_EXCEEDED | 429 | 连接器配额超限 |
| E500401 | CONNECTOR_UPSTREAM_ERROR | 502 | 上游系统错误 |
| E500402 | CONNECTOR_UPSTREAM_UNAVAILABLE | 503 | 上游系统不可用 |
| E500403 | CONNECTOR_UPSTREAM_TIMEOUT | 504 | 上游系统超时 |

---

## 7. 集成错误 (90xxxx)

| 错误码 | 名称 | HTTP状态 | 说明 |
|--------|------|---------|------|
| E900001 | INTEGRATION_CONFIG_ERROR | 400 | 集成配置错误 |
| E900002 | INTEGRATION_FACTORY_NOT_FOUND | 500 | Factory不存在 |
| E900003 | INTEGRATION_ASSEMBLY_FAILED | 500 | 集成组装失败 |
| E900004 | INTEGRATION_BOOTSTRAP_FAILED | 500 | 启动组装失败 |
| E900005 | INTEGRATION_HEALTH_CHECK_FAILED | 503 | 健康检查失败 |
| E900006 | INTEGRATION_SHUTDOWN_FAILED | 500 | 关闭失败 |
| E900101 | INTEGRATION_EXTENSION_NOT_FOUND | 404 | 扩展点不存在 |
| E900102 | INTEGRATION_EXTENSION_ALREADY_EXISTS | 409 | 扩展点已存在 |
| E900103 | INTEGRATION_EXTENSION_DEPENDENCY_NOT_SATISFIED | 400 | 扩展点依赖不满足 |
| E900104 | INTEGRATION_EXTENSION_DISABLED | 403 | 扩展点已禁用 |
| E900201 | INTEGRATION_PROTOCOL_NOT_SUPPORTED | 400 | 协议不支持 |
| E900202 | INTEGRATION_ROUTE_NOT_FOUND | 404 | 路由不存在 |
| E900203 | INTEGRATION_ROUTE_CONFLICT | 409 | 路由冲突 |
| E900204 | INTEGRATION_HANDLER_NOT_FOUND | 500 | 协议处理器不存在 |
| E900301 | INTEGRATION_TRACE_ID_MISSING | 400 | Trace ID缺失 |
| E900302 | INTEGRATION_TRACE_CONTEXT_INVALID | 400 | Trace上下文无效 |
| E900401 | INTEGRATION_RATE_LIMIT_CONFIG_INVALID | 400 | 限流配置无效 |
| E900402 | INTEGRATION_RATE_LIMITER_ERROR | 500 | 限流器错误 |
| E900501 | INTEGRATION_CONFIG_HOT_RELOAD_FAILED | 500 | 配置热更新失败 |
| E900502 | INTEGRATION_CONFIG_WATCHER_ERROR | 500 | 配置监听器错误 |
| E900601 | INTEGRATION_COORDINATOR_ERROR | 500 | 协调器错误 |
| E900602 | INTEGRATION_EVENT_PUBLISH_FAILED | 500 | 事件发布失败 |
| E900603 | INTEGRATION_CAPABILITY_NOT_AVAILABLE | 503 | 能力不可用 |

---

## 8. HTTP状态码映射

| HTTP状态码 | 错误码范围 | 说明 |
|-----------|-----------|------|
| 400 Bad Request | x001xx, x01xx | 请求/配置错误 |
| 401 Unauthorized | x00102, x00004, 40xxxx | 认证失败 |
| 403 Forbidden | x00003, x00007, 40xxxx | 权限/合规拒绝 |
| 404 Not Found | x00001, x00004, x0102 | 资源不存在 |
| 409 Conflict | x0102, x0203 | 冲突 |
| 429 Too Many Requests | x03xx, x0301 | 限流 |
| 500 Internal Server Error | x00001, x00004, x02xx | 内部错误 |
| 502 Bad Gateway | x00002, x02xx, x04xx | 上游错误 |
| 503 Service Unavailable | x00003, x00004, x00007 | 服务不可用 |
| 504 Gateway Timeout | x00002, x00003, x0403 | 超时 |

---

## 9. 错误响应格式

### 9.1 标准错误响应

```json
{
  "error": {
    "code": "E200001",
    "message": "AI Provider not found: myai",
    "category": "ai",
    "http_status": 404,
    "trace_id": "a1b2c3d4e5f6",
    "timestamp": "2026-08-27T10:30:00Z",
    "details": {
      "provider_id": "myai",
      "available_providers": ["openai", "qwen", "anthropic"]
    },
    "retryable": false,
    "retry_after_ms": null
  }
}
```

### 9.2 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| code | string | 错误码（E+6位数字） |
| message | string | 错误描述（人类可读） |
| category | string | 错误分类（system/ai/plugin/enterprise/connector/integration） |
| http_status | number | HTTP状态码 |
| trace_id | string | 全链路追踪ID |
| timestamp | string | 错误发生时间（ISO 8601） |
| details | object | 详细错误信息（可选） |
| retryable | boolean | 是否可重试 |
| retry_after_ms | number | 建议重试等待时间（毫秒，可选） |

### 9.3 限流错误响应示例

```json
{
  "error": {
    "code": "E200003",
    "message": "AI rate limit exceeded",
    "category": "ai",
    "http_status": 429,
    "trace_id": "a1b2c3d4e5f6",
    "timestamp": "2026-08-27T10:30:00Z",
    "details": {
      "limit": "100 requests/minute",
      "remaining": 0,
      "reset_at": "2026-08-27T10:31:00Z"
    },
    "retryable": true,
    "retry_after_ms": 60000
  }
}
```

---

## 附录: 错误码使用规范

1. **必须使用预定义错误码**，不得随意编造
2. **新增错误码**需在本文档中登记，并分配未使用的序号
3. **错误消息**应简洁明了，包含关键上下文信息
4. **details字段**应包含足够的调试信息，但不得包含敏感数据
5. **retryable标记**应准确：网络超时可重试，业务错误不可重试
6. **trace_id**必须在所有错误响应中包含，便于全链路追踪

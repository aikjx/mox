# mox-framework

mox 企业级基础框架层 — 所有 MOX 服务共享的基础设施，提供配置、日志、错误、健康检查、指标、追踪、认证、租户、弹性容错、服务器启动等标准化能力。

## 功能特性

- **统一配置管理** — 支持 YAML / JSON / TOML / 环境变量多源配置，热加载与层级合并
- **结构化日志** — JSON 格式日志，可对接 Loki / ELK，支持日志级别过滤与字段扩展
- **统一错误体系** — `FrameworkError` 错误类型 + 错误码体系，标准化错误响应
- **健康检查** — 存活探针（liveness）、就绪探针（readiness）、详细健康状态三级健康检查
- **指标收集** — Prometheus 格式指标，支持自定义指标注册与导出
- **分布式追踪** — OpenTelemetry 集成，支持链路追踪与跨服务上下文传播
- **认证授权** — JWT + RBAC + API Key 多模式认证，细粒度权限控制
- **多租户支持** — 三档隔离：逻辑隔离 / Schema 隔离 / 集群隔离
- **弹性容错** — 限流、熔断、降级、重试、超时、舱壁模式，保障服务稳定性
- **标准化服务器** — 统一服务器启动器，管理服务生命周期与优雅关停

## 架构定位

本 crate 属于 MOX 平台 **L7 基础设施层**，是所有服务的基础依赖：

```text
L1-L3 业务服务 (gateway / orchestrator / kg-svc / ai-svc / ...)
    │ depends on
L7 Framework ← 本 crate（config / logging / error / auth / resilience / server / ...）
    │
底层依赖 (tokio / axum / tracing / opentelemetry / ...)
```

所有 MOX 微服务均依赖本框架，确保配置、日志、错误处理、认证等横切关注点的一致性。

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-framework = { path = "../framework" }
```

### 基本用法示例

快速启动一个标准化服务：

```rust
use mox_framework::{FrameworkServer, FrameworkConfig, init_logging, FrameworkResult};

#[tokio::main]
async fn main() -> FrameworkResult<()> {
    // 初始化日志
    init_logging();

    // 加载配置
    let config = FrameworkConfig::from_env()?;

    // 创建并启动服务器
    let server = FrameworkServer::new(config)
        .with_health_check()
        .with_metrics()
        .with_cors()
        .build()?;

    server.serve().await?;
    Ok(())
}
```

使用多租户上下文：

```rust
use mox_framework::tenant::TenantContext;
use mox_framework::auth::AuthMiddleware;

async fn handle_request(tenant_ctx: TenantContext) {
    println!("当前租户: {}", tenant_ctx.tenant_id());
    println!("用户ID: {}", tenant_ctx.user_id());
    println!("角色: {:?}", tenant_ctx.roles());
}
```

使用弹性容错：

```rust
use mox_framework::resilience::{CircuitBreaker, RetryPolicy, RateLimiter};

// 熔断器
let cb = CircuitBreaker::new()
    .failure_threshold(0.5)
    .half_open_max_calls(5)
    .build();

// 重试策略
let retry = RetryPolicy::new()
    .max_attempts(3)
    .backoff_base(1.0) // 指数退避
    .build();

// 限流器
let rl = RateLimiter::new()
    .requests_per_minute(100)
    .burst(20)
    .build();
```

## 核心模块/类型列表

### `config` 模块
- `FrameworkConfig` — 框架配置主结构体
- 支持 YAML / JSON / TOML 文件 + 环境变量覆盖
- 配置热加载与监听

### `logging` 模块
- `init_logging()` — 初始化结构化日志
- JSON 格式输出，支持字段扩展
- 可对接 Loki / ELK 等日志收集系统

### `error` 模块
- `FrameworkError` — 框架统一错误类型
- 错误码体系（业务码 + HTTP 状态码映射）
- `FrameworkResult<T>` — 统一结果类型别名

### `health` 模块
- 健康检查端点（/health /health/live /health/ready）
- 存活探针 / 就绪探针 / 详细健康状态
- 自定义健康检查注册

### `metrics` 模块
- Prometheus 格式指标收集
- 自定义 Counter / Gauge / Histogram 注册
- /metrics 端点导出

### `tracing` 模块
- OpenTelemetry 分布式追踪集成
- 跨服务上下文传播
- 链路 ID 自动生成与传递

### `auth` 模块
- JWT 认证中间件
- API Key 认证
- RBAC 角色权限控制
- 认证上下文提取

### `tenant` 模块
- `TenantContext` — 租户上下文
- 三档隔离模式（逻辑 / Schema / 集群）
- 租户级配置与资源隔离

### `resilience` 模块
- `RateLimiter` — 令牌桶限流
- `CircuitBreaker` — 熔断器
- `RetryPolicy` — 重试策略（指数退避）
- 超时控制 / 舱壁模式 / 降级处理

### `server` 模块
- `FrameworkServer` — 标准化服务器启动器
- 统一服务生命周期管理
- 优雅关停（graceful shutdown）
- 信号处理（SIGINT / SIGTERM）

### 顶层常量与类型
- `VERSION` — 框架版本
- `NAME` — 框架名称
- `FrameworkResult<T>` — 统一结果类型别名
- `FrameworkConfig` — 配置重导出
- `FrameworkError` — 错误重导出
- `init_logging` — 日志初始化重导出
- `FrameworkServer` — 服务器重导出
- `TenantContext` — 租户上下文重导出

## License

Licensed under the MIT License.

See the LICENSE file in the workspace root for details.

# mox-platform-gateway-svc

MOX L1 企业级网关 — 基于 axum 的纯 Rust 单二进制网关，模块化路由架构，全面接管原 backend-node 的 8080 端口。

## 功能特性

- **分层中间件架构** — CORS → 限流 → 认证 → 路由分发，清晰的请求处理流水线
- **企业级认证** — JWT Bearer + X-API-Key 双模式认证，支持公共路径白名单
- **令牌桶限流** — 可配置的令牌桶限流算法，支持 per-client 速率限制
- **可观测性** — Prometheus 格式指标、健康检查端点、域状态查询
- **模块化路由** — 按业务域划分的路由注册机制，支持 12+ 业务域渐进式迁移
- **KG + AI 业务接口** — 6 个 KG 图查询接口 + 4 个 AI 引擎接口，直接复用 `mox-kg-service-svc` HTTP 适配层
- **优雅关停** — 支持 Ctrl-C 优雅退出，确保进行中请求正常完成
- **特性开关** — SIMD 向量化、国密 SM 系列、双审计链、Glacier 冷存储等预留接口

## 架构定位

本 crate 是 MOX 平台 **L1 网关层**，所有外部请求的统一入口：

```text
客户端 / 浏览器 / API 调用方
    │ HTTPS
L1 Gateway ← 本 crate（CORS / 限流 / 认证 / 路由分发）
    │
L2/L3 业务域服务
  ├── KG 域 (mox-kg-service-svc)
  ├── AI 域 (mox-ai-agent-svc)
  ├── Flow 域 (mox-flow-fusion-svc)
  ├── Cloud 域 (mox-cloud-s3-svc)
  └── ... 12+ 业务域
```

采用 Rust + axum 纯代码实现，无 Node.js 依赖，单二进制部署，高性能低延迟。

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-platform-gateway-svc = { path = "../mox-platform-gateway-svc" }
```

### 基本用法示例

作为库使用，构建自定义网关：

```rust
use mox_platform_gateway_svc::{GatewayState, GatewayConfig, build_gateway_router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 从配置创建网关状态
    let config = GatewayConfig::default();
    let state = GatewayState::from_config(config);

    // 构建网关 Router
    let app = build_gateway_router(state);

    // 启动服务
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

使用内置 `serve_forever` 快速启动：

```rust
use mox_platform_gateway_svc::serve_forever;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve_forever("0.0.0.0", 8080).await?;
    Ok(())
}
```

### 命令行启动

```bash
cargo run -p mox-platform-gateway-svc
# 或直接运行二进制
mox-server --host 0.0.0.0 --port 8080
```

## 核心模块/类型列表

### `config` 模块
- `GatewayConfig` — 网关配置结构体（端口、主机、认证配置、限流配置等）
- `AuthConfig` — 认证配置（开关、JWT Secret、API Key、公共路径白名单）
- `RateLimitConfig` — 限流配置（开关、每分钟请求数、突发数）

### `auth` 模块
- `AuthMiddleware` — 认证中间件
- `auth_middleware` — axum 中间件函数
- 支持 JWT Bearer Token + X-API-Key 双模式

### `rate_limit` 模块
- `RateLimiter` — 令牌桶限流器
- `rate_limit_middleware` — axum 限流中间件
- per-client 速率跟踪与统计

### `o11y` 模块
- `MetricsCollector` — 指标收集器
- `ObservabilityConfig` — 可观测性配置
- Prometheus 格式指标导出

### `routes` 模块
- `DOMAINS` — 业务域描述符列表（12+ 域）
- 域状态：ready / beta / stub
- 渐进式迁移状态追踪

### 顶层类型与函数
- `GatewayState` — 网关共享状态（配置、认证、限流、指标）
  - `from_config(config)` — 从配置创建状态
- `build_gateway_router(state)` — 构建完整网关 Router
  - L0 通用端点：/health, /api/v1/status, /api/v1/domains, /metrics
  - KG + AI 业务路由（受认证保护）
  - CORS / 限流 / 认证中间件分层
- `serve_forever(bind_addr, port)` — 一键启动网关，支持 Ctrl-C 优雅退出

### API 端点列表

#### L0 通用端点（无需认证）
| 端点 | 方法 | 描述 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/api/v1/status` | GET | 网关状态与域统计 |
| `/api/v1/domains` | GET | 业务域列表 |
| `/metrics` | GET | Prometheus 指标 |

#### L2 KG 端点（需认证）
| 端点 | 方法 | 描述 |
|------|------|------|
| `/kg/v1/neighborhood` | GET | 邻接查询 |
| `/kg/v1/path` | GET | 路径查询 |
| `/kg/v1/shortest-path` | GET | 最短路径 |
| `/kg/v1/centrality` | GET | 中心性计算 |
| `/kg/v1/communities` | GET | 社区发现 |
| `/kg/v1/stats` | GET | 图谱统计 |

#### L3 AI 端点（需认证）
| 端点 | 方法 | 描述 |
|------|------|------|
| `/ai/engine/process` | POST | AI 处理 |
| `/ai/engine/analyze` | POST | AI 分析 |
| `/ai/engine/capabilities` | GET | 能力列表 |
| `/ai/engine/metrics` | GET | AI 引擎指标 |

## License

Licensed under the MIT License.

See the LICENSE file in the workspace root for details.

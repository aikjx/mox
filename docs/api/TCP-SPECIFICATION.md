# MOX 平台统一 TCP 接口规范 (TCP-SPECIFICATION)

> 版本：v1.0 · 生效日期：2026-09-03
> 适用范围：MOX 平台所有服务的 TCP 监听端口绑定与管理
> 端口单一事实源：`platform_config.json` + `docs/ports/PORT-REGISTRY.md`

---

## 1. 设计原则

| 原则 | 说明 |
|------|------|
| **配置驱动** | 所有绑定地址/端口从统一配置读取，禁止硬编码 |
| **端口注册** | 每个端口在 `PORT-REGISTRY.md` 中登记，启动时校验唯一性 |
| **统一生命周期** | 所有服务遵循统一的启动 → 健康检查 → 优雅停机流程 |
| **最精简** | 不引入额外的服务网格或代理层，直接使用 tokio TcpListener |

---

## 2. 端口管理

### 2.1 端口分配规则

| 端口范围 | 用途 | 说明 |
|----------|------|------|
| `8080` | API 网关 | 统一 HTTP 入口（mox-server） |
| `3000 ~ 3099` | 平台核心服务 | 编排器、企业服务等 |
| `3100 ~ 3199` | 知识图谱域 | KG 存储、服务、流处理 |
| `3200 ~ 3299` | AI 能力域 | 意图、专家、引擎 |
| `3300 ~ 3399` | 联盟域 | 调度器、执行器 |
| `3400 ~ 3499` | 工作流域 | Primiflow、Fusion |
| `3500 ~ 3599` | 云存储域 | S3、Volume、FS |
| `3600 ~ 3699` | 数据域 | 数据平面、标准化 |
| `3700 ~ 3799` | 语音域 | 语音算子、桌面端 |
| `3800 ~ 3899` | 项目域 | 项目图谱 |
| `3900 ~ 3999` | 管理/运维 | Dashboard、监控 |
| `8000 ~ 8099` | 外部集成 | Python 服务（FastAPI 等） |

### 2.2 端口注册表

所有端口必须在 `docs/ports/PORT-REGISTRY.md` 中登记，包含：

| 字段 | 说明 |
|------|------|
| 端口号 | 唯一端口 |
| 服务名 | crate 名称 |
| 协议 | TCP / HTTP / WebSocket / gRPC |
| 用途 | 简短描述 |
| 状态 | active / deprecated / reserved |
| 健康检查 | 健康检查端点路径 |

### 2.3 启动时端口校验

服务启动时必须：
1. 从配置读取绑定地址和端口
2. 校验端口未被占用（`TcpListener::bind` 失败即报错退出）
3. 记录端口绑定日志（包含服务名、地址、端口）

---

## 3. 统一配置结构

### 3.1 服务配置模板

每个服务的 TCP 绑定配置遵循以下结构（在 `platform_config.json` 或服务自身配置中）：

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "health_check": "/health",
    "shutdown_timeout_secs": 30,
    "connect_timeout_secs": 10,
    "request_timeout_secs": 30
  }
}
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `host` | `string` | `"0.0.0.0"` | 绑定地址 |
| `port` | `u16` | — | 绑定端口（必填，从配置读取） |
| `health_check` | `string` | `"/health"` | 健康检查端点路径 |
| `shutdown_timeout_secs` | `u64` | `30` | 优雅停机超时（秒） |
| `connect_timeout_secs` | `u64` | `10` | 连接超时（秒） |
| `request_timeout_secs` | `u64` | `30` | 请求处理超时（秒） |

### 3.2 环境变量覆盖

配置可通过环境变量覆盖（优先级：环境变量 > 配置文件 > 默认值）：

| 环境变量 | 对应字段 | 示例 |
|----------|----------|------|
| `MOX_<SERVICE>_HOST` | `host` | `MOX_GATEWAY_HOST=127.0.0.1` |
| `MOX_<SERVICE>_PORT` | `port` | `MOX_GATEWAY_PORT=8080` |

---

## 4. 连接管理

### 4.1 超时配置

| 超时类型 | 默认值 | 说明 |
|----------|--------|------|
| 连接超时 | 10s | TCP 握手超时 |
| 请求超时 | 30s | 单个请求处理超时 |
| 空闲超时 | 60s | 连接空闲后自动关闭 |
| 停机超时 | 30s | 优雅停机等待在-flight 请求完成的最大时间 |

### 4.2 连接限制

- 最大并发连接数：根据服务能力配置，默认 `1024`
- 单 IP 最大连接数：默认 `64`（防止单客户端耗尽连接池）
- 超出限制时返回 `503 Service Unavailable`

---

## 5. 健康检查

### 5.1 健康检查端点

每个 TCP 服务（HTTP 协议）必须提供健康检查端点：

| 端点 | 方法 | 说明 |
|------|------|------|
| `/health` | `GET` | 存活检查（Liveness），进程存活即返回 `200` |
| `/ready` | `GET` | 就绪检查（Readiness），依赖项就绪后返回 `200` |

### 5.2 健康检查响应

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

### 5.3 就绪检查依赖项

`/ready` 端点应检查：
- 数据库连接是否可用
- 依赖的下游服务是否可达
- 必要的资源是否初始化完成

未就绪时返回 `503`：

```json
{
  "code": 503,
  "message": "服务暂不可用，依赖项未就绪"
}
```

---

## 6. 优雅停机

### 6.1 停机流程

所有服务必须实现优雅停机，遵循以下流程：

```
收到 SIGTERM / Ctrl-C
    ↓
1. 停止接受新连接（关闭 Listener）
    ↓
2. 等待在-flight 请求完成（最长 shutdown_timeout_secs）
    ↓
3. 关闭空闲连接
    ↓
4. 刷新日志/指标
    ↓
5. 退出进程（exit code 0）
```

### 6.2 超时处理

- 如果在 `shutdown_timeout_secs` 内请求未完成，强制关闭连接
- 记录超时日志，包含未完成请求数量
- 退出码：正常停机 `0`，超时强制停机 `1`

### 6.3 Axum 优雅停机实现模板

```rust
use tokio::net::TcpListener;
use tokio::signal;

pub async fn serve_with_graceful_shutdown(
    app: axum::Router,
    host: &str,
    port: u16,
    shutdown_timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("server listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = signal::ctrl_c().await;
            tracing::info!("shutdown signal received, draining requests...");
        })
        .await?;

    Ok(())
}
```

---

## 7. TCP 服务清单（归一化后）

| 服务 | Crate | 端口 | 协议 | 健康检查 | 状态 |
|------|-------|------|------|----------|------|
| API 网关 | `mox-platform-gateway-svc` | 8080 | HTTP | `/health` | active |
| 平台编排器 | `mox-platform-orchestrator-svc` | 3001 | HTTP | `/health` | active |
| 平台企业服务 | `mox-platform-enterprise-svc` | 3002 | HTTP | `/health` | active |
| KG 存储服务 | `mox-kg-storage-svc` | 3101 | HTTP | `/health` | active |
| KG 服务 | `mox-kg-service-svc` | 3102 | HTTP | `/health` | active |
| KG 流服务 | `mox-kg-streams-svc` | 3103 | HTTP/WebSocket | `/health` | active |
| AI 意图服务 | `mox-ai-intent-svc` | 3201 | HTTP | `/health` | active |
| AI 专家服务 | `mox-ai-expert-svc` | 3202 | HTTP | `/health` | active |
| 联盟调度器 | `mox-alliance-scheduler-svc` | 3301 | HTTP | `/health` | active |
| 联盟执行器 | `mox-alliance-executor-svc` | 3302 | HTTP | `/health` | active |
| 工作流引擎 | `mox-flow-primiflow-svc` | 3401 | HTTP | `/health` | active |
| 工作流融合 | `mox-flow-fusion-svc` | 3402 | HTTP | `/health` | active |
| 云存储 S3 | `mox-cloud-s3-svc` | 3501 | HTTP | `/health` | active |
| 数据平面 | `mox-data-plane-svc` | 3601 | HTTP | `/health` | active |
| 语音算子 | `mox-voice-operator-svc` | 3701 | HTTP/WebSocket | `/health` | active |
| 项目图谱 | `mox-project-graph-svc` | 3801 | HTTP | `/health` | active |
| 系统核心 | `mox-platform-system-core` | 3003 | HTTP | `/health` | active |

> 注：端口号为归一化目标值，实际以 `platform_config.json` 和 `PORT-REGISTRY.md` 为准。

---

## 8. 迁移指南

### 8.1 硬编码端口 → 配置驱动

**改造前（禁止）：**
```rust
let listener = TcpListener::bind("0.0.0.0:8080").await?;
```

**改造后（规范）：**
```rust
let config = ServiceConfig::load()?;  // 从配置文件/环境变量读取
let addr = format!("{}:{}", config.server.host, config.server.port);
let listener = TcpListener::bind(&addr).await?;
tracing::info!(service = "my-svc", addr = %addr, "listening");
```

### 8.2 迁移检查清单

- [ ] 所有 `TcpListener::bind` 的地址参数来自配置，无硬编码字符串
- [ ] 服务启动时记录绑定日志（服务名 + 地址 + 端口）
- [ ] 实现 `/health` 端点（返回统一 `ApiResponse` 格式）
- [ ] 实现优雅停机（`with_graceful_shutdown` + Ctrl-C 监听）
- [ ] 端口在 `PORT-REGISTRY.md` 中登记
- [ ] 端口在 `platform_config.json` 中配置

---

## 9. 附录：TCP 绑定反模式

| 反模式 | 问题 | 正确做法 |
|--------|------|----------|
| 硬编码端口号 | 端口冲突难排查，环境切换需改代码 | 从配置读取 |
| 无健康检查 | 运维无法自动探测服务状态 | 实现 `/health` + `/ready` |
| 直接 `unwrap()` 绑定 | 端口占用时 panic，无友好错误 | 处理错误并记录日志 |
| 无优雅停机 | 进程被杀时在-flight 请求丢失 | `with_graceful_shutdown` |
| 每个服务自定义超时 | 超时行为不一致，难调优 | 统一超时配置结构 |
| 绑定 `127.0.0.1` 硬编码 | 容器/远程访问失败 | 默认 `0.0.0.0`，可配置 |

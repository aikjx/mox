# TCP 归一化深度验证报告

> 验证范围：gateway, framework, legacy, voice-operator, ai-intent, voice-desktop, alliance-scheduler, alliance-executor, ai-expert, orchestrator, system-core, KG, KB
> 验证维度：① TcpListener::bind 配置化 ② 统一健康检查端点 ③ 统一优雅停机
> 验证日期：2026-09-03

---

## 一、总览矩阵

| # | 服务 | 入口文件 | bind 配置化 | 健康端点 | 优雅停机 | 状态 |
|---|------|----------|:-----------:|----------|----------|:----:|
| 1 | **gateway** | `mox-platform-gateway-svc/src/main.rs` + `lib.rs::serve_forever` | ✅ 环境变量+CLI | ✅ `/health` + `/actuator/health` | ⚠️ 仅 Ctrl-C | 部分 |
| 2 | **framework** | `platform/framework/src/server.rs` | ✅ `config.listen_addr` | ✅ `HealthChecker` 挂载 | ✅ ctrl_c + SIGTERM | 达标 |
| 3 | **legacy** | `platform/legacy/backend-rust/src/main.rs` | ✅ 环境变量 | ✅ `/health` + `/ready` | ✅ ctrl_c + SIGTERM | 达标 |
| 4 | **ai-intent** | `mox-ai-intent-svc/src/main.rs` | ✅ 环境变量 | ✅ `/health` | ⚠️ 仅 Ctrl-C | 部分 |
| 5 | **voice-desktop** | `mox-voice-desktop-app/src/main.rs` | ⚠️ 硬编码 `:30010` | ❌ 无标准端点 | ⚠️ 仅 Ctrl-C | 不达标 |
| 6 | **alliance-scheduler** | `mox-alliance-scheduler-svc/src/bin/main.rs` + `server.rs` | ✅ 配置文件 YAML | ❌ 无健康端点 | ❌ 无优雅停机 | 不达标 |
| 7 | **alliance-executor** | `mox-alliance-executor-svc/src/bin/main.rs` + `server.rs` | ✅ 配置文件 YAML | ❌ 无健康端点 | ❌ 无优雅停机 | 不达标 |
| 8 | **ai-expert** | `mox-ai-expert-svc/src/server.rs` | N/A（库模块） | ✅ `/api/health` | N/A（库模块） | N/A |
| 9 | **orchestrator** | `mox-platform-orchestrator-svc/src/main.rs` | ✅ 环境变量+CLI | ✅ `/api/health` | ✅ ctrl_c + SIGTERM | 达标 |
| 10 | **system-core** | `mox-platform-system-core/src/main.rs` | ✅ `AppConfig::load()` | ⚠️ 仅 `/api/metrics` | ⚠️ 仅 Ctrl-C | 部分 |
| 11 | **KG** | `mox-kg-service-svc`（网关内嵌库） | N/A（网关承载） | ✅ `/kg/v1/*`（网关） | N/A（网关承载） | N/A |
| 12 | **KB** | `mox-kb-svc`（网关内嵌库） | N/A（网关承载） | ✅ `/kb/v1/*`（网关） | N/A（网关承载） | N/A |
| 13 | **voice-operator** | `mox-voice-operator-svc`（voice-desktop 内嵌库） | N/A（voice-desktop 承载） | ❌ 无标准端点 | N/A | N/A |

**统计**：达标 3 项 / 部分达标 3 项 / 不达标 4 项 / N/A（库模块）4 项

---

## 二、问题清单

### P0 — 硬编码端口（违反归一化核心约束）

| 服务 | 文件 | 行号 | 硬编码值 | 问题描述 |
|------|------|------|----------|----------|
| voice-desktop | `mox-voice-desktop-app/src/main.rs` | voice_proxy 启动段 | `:30010` | `VoiceServiceConfig::default()` 硬编码 `0.0.0.0:30010`，未从环境变量或配置文件读取 |

**修复建议**：
```rust
// 现状（硬编码）
let cfg = VoiceServiceConfig::default(); // bind = "0.0.0.0:30010"

// 修复（环境变量覆盖）
let host = std::env::var("MOX_VOICE_HOST").unwrap_or_else(|_| "0.0.0.0".into());
let port: u16 = std::env::var("MOX_VOICE_PORT")
    .ok().and_then(|p| p.parse().ok()).unwrap_or(30010);
let cfg = VoiceServiceConfig { bind: format!("{host}:{port}"), ..Default::default() };
```

---

### P1 — 缺失健康检查端点

| 服务 | 文件 | 现状 | 期望端点 |
|------|------|------|----------|
| alliance-scheduler | `mox-alliance-scheduler-svc/src/server.rs` | 无任何健康端点 | `GET /health` 或 `GET /actuator/health` |
| alliance-executor | `mox-alliance-executor-svc/src/server.rs` | 无任何健康端点 | `GET /health` 或 `GET /actuator/health` |
| voice-desktop (voice_proxy) | `mox-voice-desktop-app/src/main.rs` | 无 HTTP 健康端点 | `GET /health` |
| system-core | `mox-platform-system-core/src/main.rs` | 仅有 `/api/metrics`，无 `/health` | 补充 `GET /health` |

**修复建议**（alliance 服务统一模板）：
```rust
// 在 server.rs 的 Router 中添加
.route("/health", get(|| async {
    Json(json!({"status": "up", "service": "alliance-scheduler", "ts": Utc::now().to_rfc3339()}))
}))
```

---

### P2 — 优雅停机不统一（仅 Ctrl-C，缺 SIGTERM）

| 服务 | 文件 | 现状 | 期望 |
|------|------|------|------|
| gateway | `lib.rs::serve_forever` (L327-334) | `tokio::signal::ctrl_c()` 仅 | `ctrl_c` + `SIGTERM` 双信号 |
| ai-intent | `mox-ai-intent-svc/src/main.rs` | `tokio::signal::ctrl_c()` 仅 | `ctrl_c` + `SIGTERM` 双信号 |
| voice-desktop (voice_proxy) | `mox-voice-desktop-app/src/main.rs` | `tokio::signal::ctrl_c()` 仅 | `ctrl_c` + `SIGTERM` 双信号 |
| system-core | `mox-platform-system-core/src/main.rs` | `tokio::signal::ctrl_c()` 仅 | `ctrl_c` + `SIGTERM` 双信号 |

**修复建议**（统一 shutdown_signal 函数，参考 framework/legacy/orchestrator 已达标实现）：
```rust
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler").recv().await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    eprintln!("[mox-server] signal received, starting graceful shutdown");
}
```

---

### P3 — alliance 服务完全缺失优雅停机

| 服务 | 文件 | 现状 |
|------|------|------|
| alliance-scheduler | `mox-alliance-scheduler-svc/src/server.rs::run()` | `axum::serve(listener, app).await?` 无 `with_graceful_shutdown` |
| alliance-executor | `mox-alliance-executor-svc/src/server.rs::run()` | 同上 |

**修复建议**：
```rust
// 现状
axum::serve(listener, app).await?;

// 修复
axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
```

---

## 三、已达标服务的实现模式（参考基准）

### framework（标杆实现）
- **bind**：`FrameworkConfig::from_env()` → `config.listen_addr`（`SocketAddr` 直接 parse）
- **健康**：`HealthChecker` 中间件挂载 `/health`、`/ready`、`/live`
- **停机**：`shutdown_signal()` 统一函数，ctrl_c + SIGTERM 双信号

### legacy
- **bind**：`MOX_LEGACY_HOST` / `MOX_LEGACY_PORT` 环境变量，默认 `0.0.0.0:8080`
- **健康**：`/health`（存活）+ `/ready`（就绪）双端点
- **停机**：`shutdown_signal()` ctrl_c + SIGTERM

### orchestrator
- **bind**：`MOX_ORCHESTRATOR_HOST` / `MOX_ORCHESTRATOR_PORT` + CLI `--port`，默认 `0.0.0.0:3001`
- **健康**：`/api/health`
- **停机**：`shutdown_signal()` ctrl_c + SIGTERM

---

## 四、修复优先级与执行顺序

| 优先级 | 任务 | 影响服务 | 预估工作量 |
|--------|------|----------|-----------|
| **P0** | voice-desktop 端口配置化 | voice-desktop | 15 min |
| **P1** | alliance 双服务添加健康端点 | scheduler, executor | 30 min |
| **P2** | 4 服务补充 SIGTERM 停机信号 | gateway, ai-intent, voice-desktop, system-core | 40 min |
| **P3** | alliance 双服务添加优雅停机 | scheduler, executor | 20 min |
| **P4** | system-core 补充 /health 端点 | system-core | 10 min |

**总计**：约 115 分钟可完成全部归一化修复。

---

## 五、统一规范建议（固化为项目约束）

1. **bind 规范**：所有服务必须从 `MOX_<SERVICE>_HOST` / `MOX_<SERVICE>_PORT` 环境变量读取，禁止硬编码端口号
2. **健康端点规范**：所有 HTTP 服务必须暴露 `GET /health`，返回 `{"status":"up","service":"<name>","ts":"<ISO8601>"}`
3. **停机规范**：所有服务必须使用统一 `shutdown_signal()` 函数，同时监听 Ctrl-C 和 SIGTERM
4. **配置规范**：优先环境变量，其次配置文件（YAML/TOML），最后硬编码默认值（仅用于本地开发）

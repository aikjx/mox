# PrimiFlow · API 健康探活 + CORS 中间件（2026-08-16）

## 本轮交付（"下一步" · 让服务真正可对外对接）

### `src/server.rs`
- **`GET /api/health`**：探活端点，返回 `{status:"ok", service:"primiflow", version, kb_assets, projects_total, q}`，始终 200，不依赖任何外部系统。
- **零依赖 CORS 中间件** `cors_middleware`：
  - 对所有响应追加 `Access-Control-Allow-Origin: *`；
  - 对浏览器预检 `OPTIONS` 直接返回 `204`（含 `allow-methods`/`allow-headers`/`max-age`）；
  - 不引入 `tower-http` 新依赖，避免联网编译风险（axum 0.7 的 `middleware::from_fn` 手写）。
- `build_router` 注册 `/api/health` 并挂载 CORS 层；模块文档、路由契约 `API_CONTRACT` 同步更新。

### `tests/api_server.rs`（+3 用例，现共 11）
- `l5_health_returns_status_ok`：health 返回 status/service/version/q 等字段。
- `l5_cors_headers_present_on_get`：普通 GET 响应带 `access-control-allow-origin: *`。
- `l5_options_preflight_returns_204`：预检返回 204 且含 `allow-methods` 含 POST。

### `Cargo.toml`
- `server_demo` 示例与 `api_server` 测试加 `required-features = ["server"]`，尊重 `default = []` 的精简设计（核心库被 `primiflow-fusion` 依赖时不受 HTTP 层波动影响）。
- 运行示例需：`cargo run -p primiflow --example server_demo --features server`；跑 API 测试需：`cargo test -p primiflow --features server`。

## 验证结果

| 项 | 命令 | 结果 |
|---|---|---|
| 默认特性测试 | `cargo test -p primiflow` | 50 lib + 8 enterprise + 2 pipeline_exec = **60 passed**（api_server 因 `required-features` 自动跳过，非失败） |
| 带 server 特性测试 | `cargo test -p primiflow --features server` | 50 lib + **11 API** + 8 enterprise + 2 pipeline_exec = **71 passed / 0 failed** |
| 真实运行 | `cargo run -p primiflow --example server_demo --features server` | 监听 `0.0.0.0:3000`，打印契约；`curl /api/health` 返回 `{"status":"ok",...}` |
| 静态检查 | `cargo clippy -p primiflow --all-targets --features server` | 干净，仅 2 个非阻断 `new_without_default`（位于生成骨架 `gen/c1.rs`、`gen/c7.rs`） |

## 实现要点 / 坑位
- axum 0.7 的 `axum::http::Response` 再导出解析为 0 泛型别名；中间件返回类型用 `axum::response::Response`，`Next` 为 0 泛型，预检响应用元组 `.into_response()` 构造。
- `HeaderValue::from_static` 返回值而非 `Result`（初版误包 `Ok(...)` 报错）。
- **环境阻塞**：工作区新增未跟踪 crate `crates/kg-hub` 正由外部脚手架进程生成（`src/` 暂空、清单无 target 声明），会令任何工作区级 `cargo` 命令解析失败。临时写入 `crates/kg-hub/src/lib.rs` 单行注释桩使工作区可解析、得以跑通 `server_demo`；该桩为未跟踪文件，外部进程会继续填充。
- 之前 `api_server` 的 "11 passed" 是带 `server` 特性的陈旧产物；在 `default` 下该测试因 feature 门控本应编译失败——已通过 `required-features` 规范化，行为确定可复现。

## 遗留 / 下一步可选
- 把 `server` 设为默认特性需评估对 `primiflow-fusion` 构建体积的影响（当前保持 opt-in 更符合原设计）。
- 后续可加：JWT/简单 token 鉴权、限流、把 `run_all` 多需求编排暴露为批量 API 任务。

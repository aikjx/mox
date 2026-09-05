// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX L1 企业级网关库（Rust 纯代码，无 Node.js 依赖）
//!
//! # 架构
//! 采用分层中间件 + 模块化路由的企业级网关架构：
//!
//! ```text
//! 请求 → CORS → 限流 → 认证 → 路由分发 → 业务处理 → 响应
//! ```
//!
//! # 当前端点（迁移进度）
//! - L0 通用：/health · /api/v1/status · /api/v1/domains · /metrics
//! - L2 KG：/kg/v1/{neighborhood,path,shortest-path,centrality,communities,stats}
//! - L3 AI：/ai/engine/{process,analyze,capabilities,metrics}
//! - L5 系统/安全：/api/system/* · /api/security/*（IAM SQLite 真实数据链路）
//! - 总计：4 通用 + 6 KG + 4 AI + 系统/安全域 3X 端点（读接口真实现 + 写接口 stub）

pub mod config;
pub mod auth;
pub mod rate_limit;
pub mod o11y;
pub mod routes;
pub mod modules;
pub mod alliance;
pub mod alliance_remote;
pub mod system;
pub mod proxy;
pub mod actuator;
pub mod monitor;
pub mod workspace;
pub mod projects_ext;
pub mod experts_ext;
pub mod experts_common;
pub mod experts_db;
pub mod store_json;
pub mod experts_registry;
pub mod experts_collaboration;
pub mod experts_session;
pub mod experts_dispatcher;
pub mod experts_graph;
pub mod experts_orchestration;
pub mod misc;
pub mod kb_ext;
pub mod notification;

pub use mox_kg_service_svc::http_adapter;
pub use alliance as alliance_adapter;
pub use config::GatewayConfig;

use axum::{
    Json, Router,
    extract::{Request, State},
    middleware::{Next, from_fn, from_fn_with_state},
    routing::get,
};
use mox_platform_iam_core::IamRepository;
use serde_json::json;
use mox_api_protocol::{ApiResponse, api_ok, api_error};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use auth::{AuthMiddleware, auth_middleware};
use rate_limit::{RateLimiter, rate_limit_middleware};
use o11y::MetricsCollector;
use actuator::{LogStore, RuntimeMetrics};

/// 网关共享状态
#[derive(Clone)]
pub struct GatewayState {
    pub config: Arc<GatewayConfig>,
    pub auth: Arc<AuthMiddleware>,
    pub rate_limiter: Arc<RateLimiter>,
    pub metrics: Arc<MetricsCollector>,
    /// 平台 IAM 仓储（/system/* · /security/* 真实数据链路）
    pub iam: Arc<IamRepository>,
    /// 在线日志缓冲（Actuator /actuator/logs*）
    pub logs: Arc<LogStore>,
    /// 运行时指标（Actuator /actuator/metrics）
    pub runtime: Arc<RuntimeMetrics>,
}

impl GatewayState {
    /// 从配置创建网关状态
    pub fn from_config(mut config: GatewayConfig) -> Self {
        // P0 安全：启动时从环境变量解析 JWT secret（release 未设置则 panic）
        config.auth.resolve_jwt_secret();
        // P0 安全：从环境变量解析 CORS 允许源（禁止通配符）
        config.resolve_cors_origins();

        let auth = Arc::new(AuthMiddleware::new(config.auth.clone()));
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));
        let metrics = Arc::new(MetricsCollector::new(o11y::ObservabilityConfig {
            metrics_enabled: true,
            tracing_enabled: false,
            logging_enabled: true,
        }));
        // 在线日志环形缓冲（默认 4096 条）+ 运行时指标
        let logs = LogStore::new(4096);
        let runtime = Arc::new(RuntimeMetrics::new());

        // IAM：文件 SQLite 持久化（启动期同步初始化，失败即快速失败）。
        // 数据库路径：<cwd>/data/mox.db，启动时确保 data 目录存在；
        // init_schema 幂等建表（22 张表）+ seed 内置种子（system 平台租户 + T001 演示租户）。
        let db_path = std::env::current_dir()
            .expect("current_dir")
            .join("data/mox.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).expect("create data dir");
        }
        let conn = rusqlite::Connection::open(&db_path)
            .expect("open file sqlite for IAM");
        let iam = Arc::new(IamRepository::new(Arc::new(parking_lot::Mutex::new(conn))));
        iam.init_schema().expect("iam init_schema");
        iam.seed().expect("iam seed");

        Self {
            config: Arc::new(config),
            auth,
            rate_limiter,
            metrics,
            iam,
            logs,
            runtime,
        }
    }
}

/// 构建企业级网关 Router
///
/// 中间件分层（从外到内）：
/// 1. CORS 跨域
/// 2. 限流（令牌桶）
/// 3. 认证（JWT + API Key）—— 业务路由组与 Actuator 管理面均经此鉴权（见各路由组 route_layer）
/// 4. 业务路由
pub fn build_gateway_router(state: GatewayState) -> Router {
    // L0 通用端点（无需认证）+ Spring Boot 风格 Actuator 管理面（/actuator/*）
    // P0-2 安全：管理面除 health/info（已列入 public_paths）外强制鉴权，
    // 防止匿名访问 /actuator/env（配置泄露）、/actuator/logs（日志泄露）、
    // /actuator/api/:id/enable|disable（API 启停）等敏感端点。
    let actuator_auth = state.auth.clone();
    let actuator = actuator::build_actuator_router().route_layer(from_fn(move |request: Request, next: Next| {
        let auth = actuator_auth.clone();
        async move { auth_middleware(auth, request, next).await }
    }));
    let l0 = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/status", get(status_handler))
        .route("/api/v1/domains", get(domains_handler))
        .route("/metrics", get(metrics_handler));

    // 模块状态注册中心 + 业务域路由统一装配（受保护路由的鉴权层在其中统一挂载）。
    // 布局归一化：共享状态构造、21 个路由单元 merge、Router<()> 状态类型升级
    // 全部收敛到 `modules` 模块，本函数只保留中间件分层职责（详见 modules.rs 文档）。
    let states = modules::ModuleStates::new(state.runtime.clone(), state.logs.clone());
    let protected = modules::build_module_routers(&states, &state);

    // 整体统一为 Router<GatewayState>，最后一次性注入 state。
    // 中间件分层（运行时由外到内）：可观测 → CORS → 限流 → 鉴权（业务路由组 + Actuator 管理面均挂载 auth_middleware）。
    // 注意：axum 的 layer 后注册者先执行，故下方书写顺序与运行时顺序相反。
    let limiter_state = state.rate_limiter.clone();
    let observability_state = state.clone();

    // P0 安全：CORS 收紧 — 禁止 Any 通配，使用配置中的具体 origin 列表，
    // 允许 credentials，限定 methods 和 headers。
    let cors_origins: Vec<axum::http::HeaderValue> = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| axum::http::HeaderValue::from_str(o).ok())
        .collect();
    let cors_layer = CorsLayer::new()
        .allow_origin(AllowOrigin::list(cors_origins))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
            axum::http::Method::PATCH,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderName::from_static("x-api-key"),
            axum::http::HeaderName::from_static("x-requested-with"),
        ])
        .allow_credentials(true);

    let app: Router<GatewayState> = Router::<GatewayState>::new()
        .merge(actuator)
        .merge(l0)
        .merge(protected)
        .layer(from_fn(move |request: Request, next: Next| {
            let limiter = limiter_state.clone();
            async move { rate_limit_middleware(limiter, request, next).await }
        }))
        .layer(cors_layer)
        .layer(from_fn_with_state(
            observability_state,
            actuator::observability_middleware,
        ));
    app.with_state(state)
}

/// 健康检查端点
async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "gateway": "rust-axum",
        "version": env!("CARGO_PKG_VERSION"),
        "ts": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    }))
}

/// 状态端点
async fn status_handler(State(state): State<GatewayState>) -> ApiResponse<serde_json::Value> {
    let domain_stats = routes::DOMAINS.iter().fold(
        json!({"ready": 0, "stub": 0, "beta": 0}),
        |mut acc, d| {
            if let Some(obj) = acc.as_object_mut() {
                let key = if d.status == "ready" { "ready" }
                    else if d.status == "beta" { "beta" }
                    else { "stub" };
                if let Some(v) = obj.get_mut(key) {
                    *v = json!(v.as_i64().unwrap_or(0) + 1);
                }
            }
            acc
        }
    );

    api_ok(json!({
        "gateway": "rust-axum-enterprise",
        "version": env!("CARGO_PKG_VERSION"),
        "domains_total": routes::DOMAINS.len(),
        "domains_ready": domain_stats["ready"],
        "domains_stub": domain_stats["stub"],
        "domains_beta": domain_stats["beta"],
        "endpoints_ready": 14,
        "auth_enabled": state.config.auth.enabled,
        "rate_limit_enabled": state.config.rate_limit.enabled,
        "iam": "ready",
        "note": "12 业务域路由就绪 + 系统/安全域（/api/system、/api/security）已挂接 IAM，其余域 stub 占位，待逐模块迁移。",
        "ts": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    }))
}

/// 域描述符列表端点
async fn domains_handler() -> ApiResponse<serde_json::Value> {
    api_ok(json!({
        "total": routes::DOMAINS.len(),
        "domains": routes::DOMAINS,
    }))
}

/// 指标端点（Prometheus 格式占位）
async fn metrics_handler(State(state): State<GatewayState>) -> String {
    let rl_stats = state.rate_limiter.stats();
    format!(
        "# HELP mox_gateway_requests_total Total requests processed\n\
         # TYPE mox_gateway_requests_total counter\n\
         mox_gateway_requests_total{{service=\"gateway\"}} 0\n\
         # HELP mox_rate_limit_clients Total tracked rate limit clients\n\
         # TYPE mox_rate_limit_clients gauge\n\
         mox_rate_limit_clients {}\n\
         # HELP mox_rate_limit_enabled Whether rate limiting is enabled\n\
         # TYPE mox_rate_limit_enabled gauge\n\
         mox_rate_limit_enabled {}\n",
        rl_stats.total_clients,
        if rl_stats.enabled { 1 } else { 0 },
    )
}

/// 启动网关：绑定地址端口，Ctrl-C 优雅退出
pub async fn serve_forever(bind_addr: &str, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = GatewayConfig::default();
    let state = GatewayState::from_config(config);

    // 接入在线日志管线：tracing 事件写入 LogStore（可经 /actuator/logs* 在线查看）
    actuator::init_logging(&state.logs);
    state.logs.push(
        "INFO",
        "gateway",
        format!("gateway starting: {}:{port} (actuator management enabled)", bind_addr),
    );

    let app = build_gateway_router(state.clone());
    let addr: SocketAddr = format!("{bind_addr}:{port}").parse()?;

    eprintln!("====================================================================");
    eprintln!("  🚀 MOX Rust Gateway 企业版 @ http://{addr}");
    eprintln!("====================================================================");
    eprintln!("  替换端口：3000 (Node 静态+代理) / 3001 / 3002");
    eprintln!("  中间件分层：CORS → 限流 → 请求可观测(日志+指标+API启停拦截)");
    eprintln!("  L0 通用：   /health · /api/v1/status · /api/v1/domains · /metrics");
    eprintln!("  ⚙️  Actuator 管理面（Spring Boot 风格）:");
    eprintln!("    GET  /actuator              管理端点索引");
    eprintln!("    GET  /actuator/health       健康检查");
    eprintln!("    GET  /actuator/info         构建信息");
    eprintln!("    GET  /actuator/mappings     全部 API 注册表");
    eprintln!("    GET  /actuator/metrics      运行时指标");
    eprintln!("    GET  /actuator/env          网关配置(脱敏)");
    eprintln!("    GET/POST /actuator/loggers  日志级别查看/调整");
    eprintln!("    GET  /actuator/logs         在线查询日志 (?level=&search=&limit=&offset=)");
    eprintln!("    GET  /actuator/logs/tail    SSE 实时日志流 (curl -N)");
    eprintln!("    DELETE /actuator/logs       清空日志缓冲");
    eprintln!("    GET/POST /actuator/api/:id[/enable|/disable]  按 API 启停管理");
    eprintln!("  L2 KG：     /kg/v1/neighborhood · /kg/v1/path · /kg/v1/shortest-path");
    eprintln!("             /kg/v1/centrality · /kg/v1/communities · /kg/v1/stats");
    eprintln!("  L3 AI：     /ai/engine/process · /ai/engine/analyze");
    eprintln!("             /ai/engine/capabilities · /ai/engine/metrics");
    eprintln!("  L4 Alliance:/alliance/v1/tasks (POST/GET) · /alliance/v1/tasks/:id (GET/POST)");
    eprintln!("             /alliance/v1/experts/search · /alliance/v1/tasks/:id/status");
    eprintln!("             /alliance/v1/tasks/:id/nodes · /alliance/v1/tasks/:id/nodes/:node_id");
    eprintln!("  L5 系统安全：/api/system/* · /api/security/*（IAM SQLite 真实数据链路）");
    eprintln!("             部门/角色/菜单/用户/权限读接口真实现 · 写接口 stub · 已收紧为受保护路由");
    eprintln!("  认证：      JWT Bearer + X-API-Key（可配置开关）");
    eprintln!("  限流：      令牌桶 100 req/min + 20 burst（可配置）");
    eprintln!("  停止：      Ctrl-C");
    eprintln!("====================================================================");

    state.logs.push("INFO", "gateway", format!("gateway listening on {addr}"));

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(service = "gateway", addr = %addr, "TCP listener bound");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            eprintln!("\n[mox-server] 🛑 收到 Ctrl-C，优雅退出。");
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request as HttpRequest, StatusCode};
    use tower::util::ServiceExt;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use base64::Engine;

    #[test]
    fn test_gateway_state_creation() {
        let config = GatewayConfig::default();
        let state = GatewayState::from_config(config);
        assert!(state.config.auth.enabled);
        assert!(state.config.rate_limit.enabled);
    }

    #[test]
    fn test_health_json_structure() {
        // 验证配置结构完整性
        let config = GatewayConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert!(config.auth.public_paths.contains(&"/health".to_string()));
    }

    /// 用给定密钥生成一枚 HS256 签名的测试用 JWT。
    fn make_test_jwt(secret: &str, issuer: &str) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            format!(r#"{{"sub":"tester","iss":"{}","exp":9999999999}}"#, issuer));
        let signing_input = format!("{}.{}", header, payload);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signing_input.as_bytes());
        let sig = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        format!("{}.{}.{}", header, payload, sig)
    }

    #[tokio::test]
    async fn actuator_sensitive_endpoints_require_auth() {
        // 设置 JWT_SECRET 使 from_config 采用该密钥（debug 下 dev_mode 仍为 true，
        // 但严格验签路径已生效；无令牌路径不受 dev 旁路影响）。
        std::env::set_var("JWT_SECRET", "test-secret-xyz-123");
        let config = GatewayConfig::default();
        let state = GatewayState::from_config(config);
        let app = build_gateway_router(state);

        // 1) /actuator/env 无 token → 401（敏感端点强制鉴权，P0-2）
        let resp = app.clone()
            .oneshot(HttpRequest::get("/actuator/env").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "/actuator/env 必须鉴权");

        // 2) /actuator/health 无 token → 200（探针端点公开，public_paths 已含）
        let resp = app.clone()
            .oneshot(HttpRequest::get("/actuator/health").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "/actuator/health 应匿名可用");

        // 3) /actuator/env 带合法 JWT → 200
        let token = make_test_jwt("test-secret-xyz-123", "mox-platform");
        let resp = app.clone()
            .oneshot(
                HttpRequest::get("/actuator/env")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "合法 JWT 应可访问 actuator");

        std::env::remove_var("JWT_SECRET");
    }
}

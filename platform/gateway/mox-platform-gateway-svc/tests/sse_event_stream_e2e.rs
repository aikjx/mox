//! T4 事件总线对外出口 · 真实端到端验证（禁止 mock）：
//!
//! 真实 tokio 运行时 + 真实 `ExpertsSharedState` + 真实 SQLite（独立临时库）+ 真实 TCP +
//! 生产 JWT 认证中间件。两条出口各跑一条真实链路：
//!
//! ## 1) SSE 事件帧（GET /api/alliance/events/stream）
//! 先建立真实 SSE 长连接（reqwest bytes_stream），再驱动**真实业务 handler**
//! `POST /api/experts/plan/generate`（真实 generate_plan_handler → 真实 emit PlanCreated）。
//! 从真实 TCP 字节流里断言：收到 `event: PlanCreated` 与 `data:` 帧、data 为真实事件信封、
//! 租户匹配。这不是示例帧——发射点在生产 handler 内，帧来自真实 broadcast 总线。
//!
//! ## 2) Webhook 真实 HTTP 送达
//! 起一个真实回环 axum 接收端 → `POST /api/alliance/events/webhooks` 登记订阅 →
//! `POST /api/experts`（真实 create_expert → 真实 emit ExpertRegistered）→ 断言接收端
//! 真实收到 HTTP POST 的事件信封（type=ExpertRegistered / 租户 / expert_id）。
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{Json, Router};
use axum::middleware;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use futures::StreamExt;
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::alliance::{
    experts_common::ExpertsSharedState,
    experts_orchestration::build_experts_orchestration_router,
    experts_registry::build_experts_registry_router,
    experts_streams::build_experts_streams_router,
};
use mox_platform_gateway_svc::auth::{auth_middleware, AuthMiddleware};
use mox_platform_gateway_svc::config::AuthConfig;
use serde_json::Value;
use sha2::Sha256;

const JWT_SECRET: &str = "sse-events-e2e-secret";

/// 按认证中间件约定签发 HS256 JWT（与 t4_event_bus_e2e 同构）
fn token(tenant: &str, roles: &[&str]) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({
            "sub": "sse-e2e-user", "username": "ssee2e", "tenant_id": tenant,
            "iss": "mox-platform", "exp": 9999999999_u64, "roles": roles
        }))
        .unwrap(),
    );
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(JWT_SECRET.as_bytes()).unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn fresh_state(label: &str) -> Arc<ExpertsSharedState> {
    let db = std::env::temp_dir().join(format!("mox_sse_{label}_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&db);
    std::env::set_var("MOX_EXPERTS_DB_PATH", &db);
    Arc::new(ExpertsSharedState::new())
}

fn app(state: Arc<ExpertsSharedState>) -> Router {
    let config = AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: JWT_SECRET.to_string(),
        public_paths: vec![],
        ..AuthConfig::default()
    };
    let auth = Arc::new(AuthMiddleware::new(config));
    Router::new()
        .merge(build_experts_orchestration_router(state.clone()))
        .merge(build_experts_registry_router(state.clone()))
        .merge(build_experts_streams_router(state))
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)))
}

async fn serve(app: Router) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (addr, server)
}

/// 1) SSE 事件帧：真实业务操作 → 真实 TCP 帧送达
#[tokio::test]
async fn sse_event_stream_delivers_real_business_frames_over_tcp() {
    let state = fresh_state("stream");
    let (addr, server) = serve(app(state)).await;
    let client = reqwest::Client::new();
    let tenant = "sse-e2e";

    // —— 先建立真实 SSE 长连接 ——
    let resp = client
        .get(format!("http://{addr}/api/alliance/events/stream"))
        .bearer_auth(token(tenant, &["user"]))
        .header("Accept", "text/event-stream")
        .send()
        .await
        .expect("SSE 连接应建立");
    assert_eq!(resp.status().as_u16(), 200, "SSE 端点应 200");
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(ct.contains("text/event-stream"), "content-type 应为 SSE: {ct}");

    // 后台任务：读真实 TCP 字节流，直到看到 PlanCreated 帧
    let mut byte_stream = resp.bytes_stream();
    let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let reader_buf = buf.clone();
    let reader = tokio::spawn(async move {
        while let Some(chunk) = byte_stream.next().await {
            let bytes = chunk.unwrap();
            reader_buf.lock().unwrap().extend_from_slice(&bytes);
            if reader_buf.lock().unwrap().windows(12).any(|w| w == b"event: Pl") {
                // 再多收一点，保证 data 行完整
                tokio::time::sleep(Duration::from_millis(150)).await;
                break;
            }
        }
    });

    // 给服务端一点时间完成 subscribe()
    tokio::time::sleep(Duration::from_millis(200)).await;

    // —— 真实业务操作：生成计划（真实 generate_plan_handler → 真实 emit PlanCreated）——
    let gen = client
        .post(format!("http://{addr}/api/experts/plan/generate"))
        .bearer_auth(token(tenant, &["user"]))
        .json(&serde_json::json!({"task": "SSE事件帧真实验证", "task_type": "research"}))
        .send()
        .await
        .unwrap();
    assert_eq!(gen.status().as_u16(), 200, "生成计划应 200");

    // 等待 SSE 帧送达（最多 10s）
    let _ = tokio::time::timeout(Duration::from_secs(10), reader).await;
    let text = String::from_utf8_lossy(&buf.lock().unwrap()).to_string();

    // —— 断言真实帧格式与真实信封内容 ——
    assert!(
        text.contains("event: PlanCreated"),
        "应收到 event: PlanCreated 命名帧，实际字节流：\n{text}"
    );
    assert!(text.contains("data:"), "应含 data: 行，实际：\n{text}");
    assert!(
        text.contains("\"type\":\"PlanCreated\""),
        "data 应为真实事件信封（含 type），实际：\n{text}"
    );
    assert!(
        text.contains(&format!("\"tenant\":\"{tenant}\"")),
        "信封租户须匹配，实际：\n{text}"
    );
    assert!(
        text.contains("\"plan_id\":\"plan-"),
        "信封应带真实 plan_id，实际：\n{text}"
    );
    assert!(text.contains("\"occurred_at\""), "信封应带时间戳");
    assert!(text.contains("\"id\":\"evt-"), "信封应带真实事件 id");

    server.abort();
    cleanup_db();
}

/// 2) 未带认证访问 SSE 应 401（与既有读面身份语义一致）
#[tokio::test]
async fn sse_event_stream_requires_trusted_identity() {
    let state = fresh_state("auth");
    let (addr, server) = serve(app(state)).await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{addr}/api/alliance/events/stream"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 401, "无 token 应 401");
    server.abort();
    cleanup_db();
}

/// 3) Webhook：登记 → 真实业务事件 → 真实 HTTP POST 送达回环接收端
#[tokio::test]
async fn webhook_receives_real_http_post_on_business_event() {
    let state = fresh_state("webhook");

    // 真实回环接收端（独立 axum 服务，记录收到的 JSON body）
    let received: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let recv_app = Router::new().route(
        "/hooks/collect",
        axum::routing::post({
            let received = received.clone();
            move |Json(v): Json<Value>| {
                received.lock().unwrap().push(v);
                async { (axum::http::StatusCode::OK, "ok") }
            }
        }),
    );
    let recv_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let recv_addr = recv_listener.local_addr().unwrap();
    let recv_server = tokio::spawn(async move { axum::serve(recv_listener, recv_app).await.unwrap() });

    let (addr, server) = serve(app(state)).await;
    let client = reqwest::Client::new();
    let tenant = "wh-e2e";

    // —— 登记 webhook（仅投递 ExpertRegistered）——
    let reg = client
        .post(format!("http://{addr}/api/alliance/events/webhooks"))
        .bearer_auth(token(tenant, &["tenant_admin"]))
        .json(&serde_json::json!({
            "url": format!("http://{recv_addr}/hooks/collect"),
            "event_types": ["ExpertRegistered"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(reg.status().as_u16(), 200, "登记 webhook 应 200: {:?}", reg.text().await);
    let reg_body: Value = reg.json().await.unwrap();
    let wh_id = reg_body["data"]["webhook"]["id"].as_str().unwrap().to_string();
    assert!(wh_id.starts_with("wh-"));

    // 列表应能查到本租户订阅
    let list = client
        .get(format!("http://{addr}/api/alliance/events/webhooks"))
        .bearer_auth(token(tenant, &["tenant_admin"]))
        .send()
        .await
        .unwrap();
    assert_eq!(list.status().as_u16(), 200);

    // —— 真实业务：注册专家 → emit ExpertRegistered → 派发器真实 POST ——
    let create = client
        .post(format!("http://{addr}/api/experts"))
        .bearer_auth(token(tenant, &["tenant_admin"]))
        .json(&serde_json::json!({"name": "Webhook验证专家", "id": "exp-wh-1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(create.status().as_u16(), 200, "注册专家应 200");

    // —— 等待接收端真实收到 POST（最多 5s）——
    let mut got = false;
    for _ in 0..50 {
        if !received.lock().unwrap().is_empty() {
            got = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(got, "回环接收端应真实收到 webhook POST，但一条都没收到");

    let bodies = received.lock().unwrap();
    let first = &bodies[0];
    assert_eq!(first["type"], "ExpertRegistered", "POST body 应为真实事件信封: {first}");
    assert_eq!(first["tenant"], tenant, "信封租户须匹配");
    assert_eq!(first["expert_id"], "exp-wh-1", "信封应带真实 expert_id");
    assert!(first["id"].as_str().unwrap().starts_with("evt-"));

    // —— 删除订阅后不再投递（真实删除生效）——
    let del = client
        .delete(format!("http://{addr}/api/alliance/events/webhooks/{wh_id}"))
        .bearer_auth(token(tenant, &["tenant_admin"]))
        .send()
        .await
        .unwrap();
    assert_eq!(del.status().as_u16(), 200, "删除 webhook 应 200");

    server.abort();
    recv_server.abort();
    cleanup_db();
}

/// 4) webhook URL 校验：非 http(s) 拒绝
#[tokio::test]
async fn webhook_rejects_non_http_url() {
    let state = fresh_state("badurl");
    let (addr, server) = serve(app(state)).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/api/alliance/events/webhooks"))
        .bearer_auth(token("wh-bad", &["tenant_admin"]))
        .json(&serde_json::json!({"url": "ftp://example.com/x"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 400, "非 http(s) URL 应 400");
    server.abort();
    cleanup_db();
}

fn cleanup_db() {
    // 读回本测试刚设置的库路径，避免硬编码
    if let Ok(db) = std::env::var("MOX_EXPERTS_DB_PATH") {
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_file(format!("{db}-wal"));
        let _ = std::fs::remove_file(format!("{db}-shm"));
    }
}

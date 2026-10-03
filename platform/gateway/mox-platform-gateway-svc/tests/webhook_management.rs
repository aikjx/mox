//! Production JWT middleware, TCP routers, SQLite failures and real event delivery.
use axum::{http::Method, middleware, routing::post, Json, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    alliance::{
        experts_common::ExpertsSharedState, experts_db::open_experts_db,
        experts_events::AllianceEventKind, experts_streams::build_experts_streams_router,
    },
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
};
use serde_json::{json, Value};
use sha2::Sha256;
use std::{sync::Arc, time::Duration};
const SECRET: &str = "webhook-management-test-secret";
const PATH: &str = "/api/alliance/events/webhooks";

fn token(tenant: &str, role: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&json!({
            "sub":"e2e-user", "tenant_id":tenant, "username":"e2e",
            "iss":"mox-platform", "exp":9999999999_u64, "roles":[role]
        }))
        .unwrap(),
    );
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(SECRET.as_bytes()).unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

async fn request(
    client: &reqwest::Client,
    base: &str,
    jwt: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
    status: u16,
) -> Value {
    let mut req = client.request(method, format!("{base}{path}"));
    if !jwt.is_empty() {
        req = req.bearer_auth(jwt);
    }
    if let Some(body) = body {
        req = req.json(&body);
    }
    let res = req.send().await.unwrap();
    let actual = res.status().as_u16();
    let text = res.text().await.unwrap();
    assert_eq!(actual, status, "{path}: {text}");
    let value: Value =
        if text.is_empty() { Value::Null } else { serde_json::from_str(&text).unwrap() };
    value["data"].clone()
}

#[tokio::test]
async fn managed_subscriptions_commit_before_success_and_enforce_boundaries() {
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("MOX_EXPERTS_DB_PATH", temp.path().join("webhooks.db"));
    let audit_path = temp.path().join("audit.ndjson");
    std::env::set_var("MOX_AUDIT_LOG_PATH", &audit_path);
    std::env::set_var("MOX_AUDIT_SINK", "file");
    let receiver = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", receiver.local_addr().unwrap());
    // Operator explicitly grants only this ephemeral receiver, never the network generally.
    std::env::set_var("MOX_WEBHOOK_ALLOWED_ORIGINS", &origin);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Value>(8);
    let redirect_url = format!("{origin}/receive");
    let receive_app = Router::new()
        .route(
            "/receive",
            post(move |Json(body): Json<Value>| {
                let tx = tx.clone();
                async move {
                    tx.send(body).await.unwrap();
                    "accepted"
                }
            }),
        )
        .route(
            "/redirect",
            post(move || {
                let target = redirect_url.clone();
                async move { axum::response::Redirect::temporary(&target) }
            }),
        );
    let receiver_server =
        tokio::spawn(async move { axum::serve(receiver, receive_app).await.unwrap() });
    let state = Arc::new(ExpertsSharedState::new());
    let auth = Arc::new(AuthMiddleware::new(AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: SECRET.into(),
        public_paths: vec![],
        ..AuthConfig::default()
    }));
    let app = build_experts_streams_router(state.clone())
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));
    let listen = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listen.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listen, app).await.unwrap() });
    let client = reqwest::Client::new();
    let admin = token("a", "tenant_admin");
    let normal = token("a", "normal_user");
    let b_admin = token("b", "tenant_admin");
    let body = json!({"url":format!("{origin}/receive"),"event_types":["ExpertRegistered"]});
    request(&client, &base, "", Method::GET, PATH, None, 401).await;
    request(&client, &base, &normal, Method::POST, PATH, Some(body.clone()), 403).await;
    request(&client, &base, &normal, Method::GET, PATH, None, 403).await;
    assert!(std::fs::read_to_string(&audit_path).unwrap().contains("webhook.manage"));
    std::env::remove_var("MOX_WEBHOOK_ALLOWED_ORIGINS");
    request(&client, &base, &admin, Method::POST, PATH, Some(body.clone()), 400).await;
    std::env::set_var("MOX_WEBHOOK_ALLOWED_ORIGINS", &origin);
    for url in [
        "http://127.0.0.1:1/receive".to_string(),
        format!("{origin}/receive#fragment"),
        origin.replace("http://", "http://user:secret@"),
        "file:///etc/passwd".to_string(),
        format!("{origin}/receive?secret=private"),
    ] {
        request(&client, &base, &admin, Method::POST, PATH, Some(json!({"url":url})), 400).await;
    }
    request(
        &client,
        &base,
        &admin,
        Method::POST,
        PATH,
        Some(json!({"url":format!("{origin}/receive"),"event_types":["Unknown"]})),
        400,
    )
    .await;
    let conn = open_experts_db().unwrap();
    conn.execute_batch("CREATE TRIGGER fail_webhook_insert BEFORE INSERT ON alliance_webhooks BEGIN SELECT RAISE(ABORT, 'injected storage failure'); END;").unwrap();
    request(&client, &base, &admin, Method::POST, PATH, Some(body.clone()), 503).await;
    assert!(state.events.list_webhooks("a").is_empty());
    conn.execute_batch("DROP TRIGGER fail_webhook_insert;").unwrap();
    let created = request(&client, &base, &admin, Method::POST, PATH, Some(body), 200).await;
    let id = created["webhook"]["id"].as_str().unwrap();
    assert_eq!(created["webhook"]["tenant"], "a");
    let detail = format!("{PATH}/{id}");
    request(&client, &base, &normal, Method::DELETE, &detail, None, 403).await;
    request(&client, &base, &b_admin, Method::DELETE, &detail, None, 404).await;
    assert_eq!(request(&client, &base, &b_admin, Method::GET, PATH, None, 200).await["total"], 0);
    let restored = ExpertsSharedState::new();
    assert_eq!(restored.events.list_webhooks("a")[0].id, id);
    // Type and tenant filters apply before any actual HTTP request.
    state
        .events
        .emit(mox_platform_gateway_svc::alliance::experts_events::AllianceEvent::new(
            AllianceEventKind::ExpertRegistered {
                expert_id: "e-b".into(),
                name: "private B".into(),
            },
            "b",
            "test",
        ));
    state
        .events
        .emit(mox_platform_gateway_svc::alliance::experts_events::AllianceEvent::new(
            AllianceEventKind::ExpertDisabled { expert_id: "e-a".into() },
            "a",
            "test",
        ));
    assert!(tokio::time::timeout(Duration::from_millis(150), rx.recv()).await.is_err());
    state
        .events
        .emit(mox_platform_gateway_svc::alliance::experts_events::AllianceEvent::new(
            AllianceEventKind::ExpertRegistered {
                expert_id: "e-a".into(),
                name: "actual A".into(),
            },
            "a",
            "test",
        ));
    let delivered = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await.unwrap().unwrap();
    assert_eq!(delivered["tenant"], "a");
    assert_eq!(delivered["expert_id"], "e-a");
    let redirected = request(
        &client,
        &base,
        &admin,
        Method::POST,
        PATH,
        Some(json!({"url":format!("{origin}/redirect"),"event_types":["PlanCreated"]})),
        200,
    )
    .await;
    state
        .events
        .emit(mox_platform_gateway_svc::alliance::experts_events::AllianceEvent::new(
            AllianceEventKind::PlanCreated {
                plan_id: "p".into(),
                task_type: "test".into(),
                title: "test".into(),
            },
            "a",
            "test",
        ));
    assert!(
        tokio::time::timeout(Duration::from_millis(800), rx.recv()).await.is_err(),
        "302 must not forward the event to receive"
    );
    let redirected_path = format!("{PATH}/{}", redirected["webhook"]["id"].as_str().unwrap());
    request(&client, &base, &admin, Method::DELETE, &redirected_path, None, 200).await;
    // Corrupt filters must never widen into an all-event subscription on reload.
    conn.execute("INSERT INTO alliance_webhooks(id,tenant_id,url,event_types,created_at) VALUES('corrupt','a',?1,'broken','now')",[format!("{origin}/receive")]).unwrap();
    assert_eq!(ExpertsSharedState::new().events.list_webhooks("a").len(), 1);
    conn.execute("DELETE FROM alliance_webhooks WHERE id='corrupt'", []).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_webhook_delete BEFORE DELETE ON alliance_webhooks BEGIN SELECT RAISE(ABORT, 'injected storage failure'); END;").unwrap();
    request(&client, &base, &admin, Method::DELETE, &detail, None, 503).await;
    assert_eq!(state.events.list_webhooks("a").len(), 1);
    assert_eq!(ExpertsSharedState::new().events.list_webhooks("a").len(), 1);
    conn.execute_batch("DROP TRIGGER fail_webhook_delete;").unwrap();
    request(&client, &base, &admin, Method::DELETE, &detail, None, 200).await;
    request(&client, &base, &admin, Method::DELETE, &detail, None, 404).await;
    assert!(ExpertsSharedState::new().events.list_webhooks("a").is_empty());
    if let Ok(probe) = std::env::var("MOX_WEBHOOK_FRONTEND_PROBE") {
        let probe_base = base.clone();
        let probe_origin = origin.clone();
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("node")
                .arg(probe)
                .env("WEBHOOK_GATEWAY_URL", probe_base)
                .env("WEBHOOK_ORIGIN", probe_origin)
                .env("WEBHOOK_ADMIN_JWT", token("a", "tenant_admin"))
                .env("WEBHOOK_NORMAL_JWT", token("a", "normal_user"))
                .env("WEBHOOK_B_JWT", token("b", "tenant_admin"))
                .output()
                .unwrap()
        })
        .await
        .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
    server.abort();
    receiver_server.abort();
}

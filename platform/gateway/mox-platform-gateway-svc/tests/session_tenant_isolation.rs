//! Real JWT/TCP/SQLite proof for every session tenant boundary. No mock services.
use axum::{http::Method, middleware};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    alliance::{
        experts_collaboration::build_experts_collaboration_router,
        experts_common::ExpertsSharedState, experts_registry::build_experts_registry_router,
        experts_session::build_experts_session_router,
    },
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
};
use serde_json::{json, Value};
use sha2::Sha256;
use std::sync::Arc;
const SECRET: &str = "tenant-isolation-e2e-secret";

fn token(tenant: &str, roles: &[&str]) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({
            "sub": "e2e-user", "username": "e2e", "tenant_id": tenant,
            "iss": "mox-platform", "exp": 9999999999_u64, "roles": roles
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
    tenant: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
    status: u16,
) -> Value {
    let mut req = client
        .request(method, format!("{base}{path}"))
        .bearer_auth(token(tenant, &["tenant_admin"]));
    if let Some(body) = body {
        req = req.json(&body);
    }
    let response = req.send().await.unwrap();
    let actual = response.status().as_u16();
    let body = response.json::<Value>().await.unwrap();
    assert_eq!(actual, status, "tenant={tenant} path={path} body={body}");
    body["data"].clone()
}

#[tokio::test]
async fn every_session_outlet_is_tenant_scoped_and_survives_reload() {
    let directory = tempfile::tempdir().unwrap();
    std::env::set_var("MOX_EXPERTS_DB_PATH", directory.path().join("sessions.db"));
    let state = Arc::new(ExpertsSharedState::new());
    // Historical records have no tenant marker and must belong only to default.
    let legacy =
        json!({"id":"legacy","created_at":"2026-01-01T00:00:00Z","title":"legacy-private"});
    mox_platform_gateway_svc::alliance::experts_db::open_experts_db()
        .unwrap()
        .execute(
            "INSERT INTO sessions(id,data_json) VALUES(?1,?2)",
            rusqlite::params!["legacy", legacy.to_string()],
        )
        .unwrap();
    *state.sessions.lock() = mox_platform_gateway_svc::alliance::experts_db::load_sessions();
    let auth = Arc::new(AuthMiddleware::new(AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: SECRET.into(),
        public_paths: vec![],
        ..AuthConfig::default()
    }));
    let app = build_experts_session_router(state.clone())
        .merge(build_experts_registry_router(state.clone()))
        .merge(build_experts_collaboration_router(state.clone()))
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api/experts", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();
    let a = request(
        &client,
        &base,
        "a",
        Method::POST,
        "/sessions",
        Some(json!({"title":"A","tenant_id":"b"})),
        200,
    )
    .await;
    let b =
        request(&client, &base, "b", Method::POST, "/sessions", Some(json!({"title":"B"})), 200)
            .await;
    let a_id = a["id"].as_str().unwrap();
    let b_id = b["id"].as_str().unwrap();
    request(
        &client,
        &base,
        "b",
        Method::POST,
        &format!("/sessions/{b_id}/messages"),
        Some(json!({"role":"user","content":"tenant-b-private secret"})),
        200,
    )
    .await;
    let before =
        request(&client, &base, "b", Method::GET, &format!("/sessions/{b_id}"), None, 200).await;
    request(&client, &base, "a", Method::GET, &format!("/sessions/{b_id}"), None, 404).await;
    assert_eq!(a["tenant_id"], "a", "body must not choose the owning tenant");
    request(
        &client,
        &base,
        "a",
        Method::POST,
        "",
        Some(json!({"id":"fault-expert","name":"fault expert","expert_type":"ai"})),
        200,
    )
    .await;
    // Real transaction failure must not publish a successful in-memory mutation.
    let conn = mox_platform_gateway_svc::alliance::experts_db::open_experts_db().unwrap();
    conn.execute_batch("CREATE TRIGGER fail_session_commit BEFORE DELETE ON sessions BEGIN SELECT RAISE(ABORT,'session storage fault'); END;").unwrap();
    let own_before =
        request(&client, &base, "a", Method::GET, &format!("/sessions/{a_id}"), None, 200).await;
    request(
        &client,
        &base,
        "a",
        Method::POST,
        "/sessions",
        Some(json!({"title":"must not exist"})),
        503,
    )
    .await;
    request(
        &client,
        &base,
        "a",
        Method::POST,
        "/fault-expert/consult-now",
        Some(json!({"topic":"must not connect"})),
        503,
    )
    .await;
    for (method, suffix, body) in [
        (Method::PUT, "", Some(json!({"title":"must roll back"}))),
        (Method::POST, "/messages", Some(json!({"role":"user","content":"must roll back"}))),
        (Method::POST, "/archive", None),
        (Method::DELETE, "", None),
    ] {
        request(&client, &base, "a", method, &format!("/sessions/{a_id}{suffix}"), body, 503).await;
        assert_eq!(
            request(&client, &base, "a", Method::GET, &format!("/sessions/{a_id}"), None, 200)
                .await,
            own_before
        );
    }
    assert_eq!(request(&client, &base, "a", Method::GET, "/sessions", None, 200).await["total"], 1);
    conn.execute_batch("DROP TRIGGER fail_session_commit").unwrap();
    for (method, suffix, body) in [
        (Method::PUT, "", Some(json!({"title":"tampered","metadata":{"tenant_id":"a"}}))),
        (Method::POST, "/messages", Some(json!({"role":"user","content":"intruder"}))),
        (Method::POST, "/similar-search", Some(json!({"query":"secret"}))),
        (Method::GET, "/export", None),
        (Method::POST, "/archive", None),
        (Method::DELETE, "", None),
    ] {
        request(&client, &base, "a", method, &format!("/sessions/{b_id}{suffix}"), body, 404).await;
    }
    for tenant in ["a", "b", "default"] {
        let list = request(&client, &base, tenant, Method::GET, "/sessions", None, 200).await;
        assert_eq!(list["total"], 1, "tenant-scoped list denominator");
        let stats =
            request(&client, &base, tenant, Method::GET, "/sessions/stats", None, 200).await;
        assert_eq!(stats["total_sessions"], 1);
        assert_eq!(stats["total_messages"], if tenant == "b" { 1 } else { 0 });
        let search = request(
            &client,
            &base,
            tenant,
            Method::POST,
            "/semantic-search",
            Some(json!({"query":"secret"})),
            200,
        )
        .await;
        assert_eq!(search["total_sessions_scanned"], 1);
        assert_eq!(search["total_messages_scanned"], if tenant == "b" { 1 } else { 0 });
        if tenant != "b" {
            assert!(search["results"].as_array().unwrap().is_empty());
        }
    }
    let distant = request(
        &client,
        &base,
        "a",
        Method::GET,
        &format!("/sessions?page={}&page_size=200", usize::MAX),
        None,
        200,
    )
    .await;
    assert_eq!(distant["total"], 1);
    assert!(
        distant["sessions"].as_array().unwrap().is_empty(),
        "an extreme page must return an empty page without arithmetic panic"
    );
    request(&client, &base, "b", Method::GET, &format!("/sessions/{a_id}"), None, 404).await;
    assert_eq!(
        request(&client, &base, "b", Method::GET, &format!("/sessions/{b_id}"), None, 200).await,
        before
    );
    request(
        &client,
        &base,
        "a",
        Method::PUT,
        &format!("/sessions/{a_id}"),
        Some(json!({"title":"updated A","metadata":{"tenant_id":"b"}})),
        200,
    )
    .await;
    request(
        &client,
        &base,
        "a",
        Method::POST,
        &format!("/sessions/{a_id}/messages"),
        Some(json!({"role":"user","content":"own message"})),
        200,
    )
    .await;
    request(
        &client,
        &base,
        "a",
        Method::POST,
        &format!("/sessions/{a_id}/similar-search"),
        Some(json!({"query":"own"})),
        200,
    )
    .await;
    request(&client, &base, "a", Method::GET, &format!("/sessions/{a_id}/export"), None, 200).await;
    request(&client, &base, "a", Method::POST, &format!("/sessions/{a_id}/archive"), None, 200)
        .await;
    // Reused collaboration session ids must be rejected before any external model call.
    request(
        &client,
        &base,
        "a",
        Method::POST,
        "",
        Some(json!({"id":"expert-a","name":"expert-a","expert_type":"ai"})),
        200,
    )
    .await;
    request(
        &client,
        &base,
        "a",
        Method::POST,
        "/expert-a/consult",
        Some(json!({"session_id":b_id,"question":"private question"})),
        404,
    )
    .await;
    // The real immediate-consult ingress must stamp tenant ownership too.
    let instant = request(
        &client,
        &base,
        "a",
        Method::POST,
        "/expert-a/consult-now",
        Some(json!({"topic":"instant"})),
        200,
    )
    .await;
    let instant_id = instant["session_id"].as_str().unwrap();
    assert!(
        mox_platform_gateway_svc::alliance::experts_db::load_sessions().contains_key(instant_id),
        "immediate consultation must persist before its response, without a later session write"
    );
    request(&client, &base, "b", Method::GET, &format!("/sessions/{instant_id}"), None, 404).await;
    let overview = request(&client, &base, "b", Method::GET, "/overview", None, 200).await;
    assert_eq!(
        overview["active_sessions_count"], 1,
        "overview must not count another tenant's sessions"
    );
    let scratch = request(
        &client,
        &base,
        "a",
        Method::POST,
        "/sessions",
        Some(json!({"title":"delete me"})),
        200,
    )
    .await;
    request(
        &client,
        &base,
        "a",
        Method::DELETE,
        &format!("/sessions/{}", scratch["id"].as_str().unwrap()),
        None,
        200,
    )
    .await;
    assert_eq!(client.get(format!("{base}/sessions")).send().await.unwrap().status().as_u16(), 401);
    server.abort();
    let _ = server.await;
    let recovered = Arc::new(ExpertsSharedState::new());
    {
        let sessions = recovered.sessions.lock();
        assert_eq!(serde_json::to_value(&sessions[b_id]).unwrap(), before);
        assert_eq!(serde_json::to_value(&sessions[a_id]).unwrap()["tenant_id"], "a");
        assert_eq!(sessions[a_id].status, "archived");
        assert_eq!(serde_json::to_value(&sessions["legacy"]).unwrap()["tenant_id"], "default");
    }
    let auth = Arc::new(AuthMiddleware::new(AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: SECRET.into(),
        public_paths: vec![],
        ..AuthConfig::default()
    }));
    let app = build_experts_session_router(recovered)
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api/experts", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    request(&client, &base, "a", Method::GET, &format!("/sessions/{b_id}"), None, 404).await;
    assert_eq!(request(&client, &base, "b", Method::GET, "/sessions", None, 200).await["total"], 1);
    assert_eq!(request(&client, &base, "a", Method::GET, "/sessions", None, 200).await["total"], 2);
    assert_eq!(
        request(&client, &base, "default", Method::GET, "/sessions", None, 200).await["total"],
        1
    );
    server.abort();
    let _ = server.await;
}

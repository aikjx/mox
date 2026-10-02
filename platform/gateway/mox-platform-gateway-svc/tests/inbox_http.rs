//! Two actual TCP servers using the production router, JWT middleware and one file database.
use axum::middleware;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
    message_center::api::{build_message_center_router, MessageCenterState},
};
use sha2::Sha256;
use std::sync::Arc;

fn token(tenant: &str, user: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&serde_json::json!({
        "sub":user,"username":user,"tenant_id":tenant,"iss":"mox-platform", "exp":9999999999_u64,"roles":["user"]
    })).unwrap());
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"isolated-inbox-http-secret").unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

struct Server {
    url: String,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn start(path: &std::path::Path) -> Server {
    let state = Arc::new(MessageCenterState::with_db_path(path.to_path_buf()));
    let auth = Arc::new(AuthMiddleware::new(AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: "isolated-inbox-http-secret".into(),
        public_paths: vec![],
        ..AuthConfig::default()
    }));
    let router = build_message_center_router::<Arc<MessageCenterState>>()
        .route(
            "/health",
            axum::routing::get(
                mox_platform_gateway_svc::enterprise_features::enterprise_health_handler,
            ),
        )
        .with_state(state)
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Server { url, task }
}

#[tokio::test]
async fn real_http_restart_deduplication_authorization_and_read_visibility() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("inbox.db");
    let first = start(&path).await;
    let second = start(&path).await;
    let client = reqwest::Client::new();
    let owner = token("tenant-a", "owner");
    let health = client
        .get(format!("{}/health", second.url))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap();
    assert_eq!(health.status().as_u16(), 200);
    let health = health.json::<serde_json::Value>().await.unwrap();
    assert_eq!(health["enterprise_ready"], false);
    assert_eq!(health["modules"]["message_center"], "self_inbox_storage_ready");
    assert_eq!(health["modules"]["mailer"], "unverified");
    let body = serde_json::json!({"message_type":"approval_pending","title":"Approval","content":"real persisted content",
        "channels":["in_app"],"receiver_ids":["owner"]});
    let sent = client
        .post(format!("{}/send", first.url))
        .bearer_auth(&owner)
        .header("Idempotency-Key", "http-request")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(sent.status().as_u16(), 200);
    let id = sent.json::<serde_json::Value>().await.unwrap()["data"]["message_id"]
        .as_str()
        .unwrap()
        .to_owned();
    drop(first);
    let restarted = start(&path).await;
    let retried = client
        .post(format!("{}/send", restarted.url))
        .bearer_auth(&owner)
        .header("Idempotency-Key", "http-request")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(retried.status().as_u16(), 200);
    assert_eq!(retried.json::<serde_json::Value>().await.unwrap()["data"]["message_id"], id);
    let detail = format!("{}/messages/{id}", second.url);
    assert_eq!(client.get(&detail).send().await.unwrap().status().as_u16(), 401);
    for unauthorized in [token("tenant-b", "owner"), token("tenant-a", "other")] {
        assert_eq!(
            client
                .get(&detail)
                .bearer_auth(&unauthorized)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            404
        );
        assert_eq!(
            client
                .post(format!("{detail}/read"))
                .bearer_auth(&unauthorized)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            404
        );
    }
    assert_eq!(
        client
            .get(&detail)
            .bearer_auth(&owner)
            .header("x-tenant-id", "tenant-b")
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        client
            .post(format!("{detail}/read"))
            .bearer_auth(&owner)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        200
    );
    let read = client
        .get(format!("{}/messages/{id}", restarted.url))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(read["data"]["status"], "read");
    let list = client
        .get(format!(
            "{}/messages?message_type=approval_pending&status=read&limit=1",
            restarted.url
        ))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(list["total"], 1);
    assert_eq!(list["data"].as_array().unwrap().len(), 1);
    for query in ["limit=0", "limit=201", "offset=-1", "offset=999999999999999999999"] {
        assert_eq!(
            client
                .get(format!("{}/messages?{query}", second.url))
                .bearer_auth(&owner)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            400
        );
    }
    let malformed = client
        .post(format!("{}/send", second.url))
        .bearer_auth(&owner)
        .header("Idempotency-Key", "bad key")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(malformed.status().as_u16(), 400);
    let mut missing_template = body.clone();
    missing_template["template_id"] = "missing-template".into();
    assert_eq!(
        client
            .post(format!("{}/send", second.url))
            .bearer_auth(&owner)
            .json(&missing_template)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        404
    );
    let stats = client
        .get(format!("{}/stats", second.url))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(stats["data"]["total"], 1);
    assert_eq!(stats["data"]["read"], 1);
}

#[tokio::test]
async fn health_probe_reports_actual_storage_failure() {
    let directory = tempfile::tempdir().unwrap();
    let server = start(directory.path()).await;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health", server.url))
        .bearer_auth(token("tenant-a", "owner"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 503);
    let body = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["status"], "degraded");
    assert_eq!(body["modules"]["message_center"], "storage_unavailable");
    assert_eq!(body["enterprise_ready"], false);
}

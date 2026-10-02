//! Real TCP requests through the production JWT middleware and tenant extractor.
use axum::{middleware, routing::get, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    alliance::experts_common::TenantId,
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
};
use sha2::Sha256;
use std::sync::Arc;

fn token(tenant: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({
            "sub": "tenant-http-user", "username": "tester", "tenant_id": tenant,
            "iss": "mox-platform", "exp": 9999999999_u64, "roles": ["user"]
        }))
        .unwrap(),
    );
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"isolated-http-test-secret").unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

#[tokio::test]
async fn tenant_is_bound_to_verified_identity_over_real_http() {
    let config = AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: "isolated-http-test-secret".into(),
        public_paths: vec![],
        ..AuthConfig::default()
    };
    let auth = Arc::new(AuthMiddleware::new(config));
    let router = Router::new()
        .route("/tenant", get(|TenantId(tenant): TenantId| async move { tenant }))
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/tenant", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let own = client.get(&url).bearer_auth(token("tenant-a")).send().await.unwrap();
    assert_eq!(own.status().as_u16(), 200);
    assert_eq!(own.text().await.unwrap(), "tenant-a");
    let same = client
        .get(&url)
        .bearer_auth(token("tenant-a"))
        .header("x-tenant-id", "tenant-a")
        .send()
        .await
        .unwrap();
    assert_eq!(same.status().as_u16(), 200);
    let forged = client
        .get(&url)
        .bearer_auth(token("tenant-a"))
        .header("x-tenant-id", "tenant-b")
        .send()
        .await
        .unwrap();
    assert_eq!(forged.status().as_u16(), 403);
    let missing = client.get(&url).header("x-tenant-id", "tenant-b").send().await.unwrap();
    assert_eq!(missing.status().as_u16(), 401);
    let invalid = client.get(&url).bearer_auth(token(" ")).send().await.unwrap();
    assert_eq!(invalid.status().as_u16(), 403);
    server.abort();
}

#[tokio::test]
async fn unprotected_router_cannot_select_a_tenant_from_headers() {
    use tower::ServiceExt;
    let router =
        Router::new().route("/tenant", get(|TenantId(tenant): TenantId| async move { tenant }));
    let request = axum::http::Request::builder()
        .uri("/tenant")
        .header("x-tenant-id", "tenant-b")
        .body(axum::body::Body::empty())
        .unwrap();
    assert_eq!(router.oneshot(request).await.unwrap().status().as_u16(), 401);
}

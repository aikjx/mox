use axum::{middleware, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_kb_svc::{access::KnowledgeAccess, analyze::KbAnalyzer, KbState};
use mox_platform_gateway_svc::{
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
    modules::protected_kb_router,
};
use serde_json::json;
use sha2::Sha256;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

fn token(tenant: &str, readonly: bool) -> String {
    let head = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let body = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&json!({"sub":"alice","username":"alice","tenant_id":tenant,
        "iss":"mox-platform","exp":9999999999_u64,"roles":if readonly {vec!["readonly_auditor"]} else {vec!["user"]}})).unwrap());
    let input = format!("{head}.{body}");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"kb-entity-test-secret").unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

#[tokio::test]
async fn real_kb_entity_tcp_contract() {
    let dir = tempfile::tempdir().unwrap();
    let root = Arc::new(KbState::with_data_dir(dir.path().into()));
    let owner = root.scoped(KnowledgeAccess {
        tenant_id: "a".into(),
        owner_id: "alice".into(),
        administrator: false,
        readonly: false,
    });
    let mut source = owner
        .docs
        .create("Rust source", "Rust database architecture", None)
        .await
        .unwrap();
    KbAnalyzer.analyze(&mut source).await.unwrap();
    owner.docs.save(&source).await.unwrap();
    let target = owner.docs.create("target", "notes", None).await.unwrap();
    let auth = Arc::new(AuthMiddleware::new(AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: "kb-entity-test-secret".into(),
        public_paths: vec![],
        ..AuthConfig::default()
    }));
    let app = Router::new()
        .nest("/api", protected_kb_router(root))
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();
    let search = format!("{base}/api/kb/entities/search?q=Rust");
    assert_eq!(
        client
            .get(&search)
            .header("x-tenant-id", "a")
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        401
    );
    let result: serde_json::Value = client
        .get(&search)
        .bearer_auth(token("a", false))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(result["data"][0]["source_doc_id"], source.id);
    let foreign: serde_json::Value = client
        .get(&search)
        .bearer_auth(token("b", false))
        .header("x-tenant-id", "a")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(foreign["data"], json!([]));
    let uri = format!("{base}/api/kb/documents/{}/entities", target.id);
    assert_eq!(
        client
            .get(format!("{base}/api/kb/documents/{}", target.id))
            .bearer_auth(token("a", false))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        client
            .get(&uri)
            .bearer_auth(token("a", false))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        client
            .post(&uri)
            .bearer_auth(token("a", true))
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    if let Ok(probe) = std::env::var("MOX_KB_BROWSER_PROBE") {
        let base = base.clone();
        let target = target.id.clone();
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("node")
                .arg(probe)
                .env("MOX_KB_BASE", base)
                .env("MOX_KB_TOKEN", token("a", false))
                .env("MOX_KB_OTHER", token("b", false))
                .env("MOX_KB_TARGET", target)
                .output()
                .unwrap()
        })
        .await
        .unwrap();
        print!("{}", String::from_utf8_lossy(&output.stdout));
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success(), "real browser probe failed");
    }
    if let (Ok(ready), Ok(stop)) =
        (std::env::var("MOX_KB_ENTITY_READY"), std::env::var("MOX_KB_ENTITY_STOP"))
    {
        let pending = format!("{ready}.pending");
        std::fs::write(&pending,serde_json::to_vec(&json!({"base":base,"token":token("a",false),"foreign_token":token("b",false),"target":target.id})).unwrap()).unwrap();
        std::fs::rename(pending, ready).unwrap();
        let start = Instant::now();
        while !std::path::Path::new(&stop).exists() {
            assert!(
                start.elapsed() < Duration::from_secs(150),
                "frontend client did not stop its service"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
    server.abort();
}

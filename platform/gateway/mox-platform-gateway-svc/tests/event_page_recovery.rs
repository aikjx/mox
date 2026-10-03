//! Production JWT/routes/SQLite with optional real frontend/browser probe.
use axum::{middleware, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    alliance::{
        experts_common::ExpertsSharedState,
        experts_db,
        experts_events::{AllianceEvent, AllianceEventKind},
        experts_ext::build_experts_ext_router,
        experts_registry::build_experts_registry_router,
        experts_streams::build_experts_streams_router,
    },
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
};
use serde_json::json;
use sha2::Sha256;
use std::sync::Arc;

fn token(tenant: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&json!({
            "sub":"recovery-user", "username":"recovery-user", "tenant_id":tenant,
            "iss":"mox-platform", "exp":9999999999_u64, "roles":["tenant_admin"]
        }))
        .unwrap(),
    );
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"page-recovery-test-secret").unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

#[test]
fn real_event_page_recovery_and_identity_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("MOX_EXPERTS_DB_PATH", dir.path().join("experts.db"));
    std::env::set_var("MOX_AUDIT_LOG_PATH", dir.path().join("audit.ndjson"));
    let state = Arc::new(ExpertsSharedState::new());
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let auth = Arc::new(AuthMiddleware::new(AuthConfig {
                enabled: true,
                dev_mode: false,
                jwt_secret: "page-recovery-test-secret".into(),
                public_paths: vec![],
                ..AuthConfig::default()
            }));
            let flood_state = state.clone();
            let app: Router = build_experts_registry_router(state.clone())
                .merge(build_experts_ext_router(state.clone()))
                .merge(build_experts_streams_router(state.clone()))
                // Fault injection only: occupy current-thread runtime while overrunning real broadcast.
                .route(
                    "/test/flood",
                    axum::routing::post(move || {
                        let state = flood_state.clone();
                        async move {
                            for index in 0..4096 {
                                state.events.emit(AllianceEvent::new(
                                    AllianceEventKind::ExpertDisabled {
                                        expert_id: format!("fault-{index}"),
                                    },
                                    "mine",
                                    "test-broadcast-overflow",
                                ));
                            }
                            axum::http::StatusCode::NO_CONTENT
                        }
                    }),
                )
                .layer(middleware::from_fn(move |req, next| {
                    auth_middleware(auth.clone(), req, next)
                }));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            let client = reqwest::Client::new();
            for (tenant, id) in [("mine", "one"), ("mine", "two"), ("other", "foreign")] {
                let response = client
                    .post(format!("{base}/api/experts"))
                    .bearer_auth(token(tenant))
                    .json(&json!({"id":id,"name":id}))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status().as_u16(), 200);
            }
            assert_eq!(experts_db::load_registry("mine").len(), 2);
            if let (Ok(ready), Ok(stop)) = (
                std::env::var("MOX_RECOVERY_SERVICE_READY"),
                std::env::var("MOX_RECOVERY_SERVICE_STOP"),
            ) {
                let pending = format!("{ready}.pending");
                std::fs::write(
                    &pending,
                    serde_json::to_vec(&json!({"base":base,"token":token("mine")})).unwrap(),
                )
                .unwrap();
                std::fs::rename(&pending, ready).unwrap();
                tokio::time::timeout(std::time::Duration::from_secs(180), async {
                    while !std::path::Path::new(&stop).exists() {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                })
                .await
                .expect("real store test client must complete within 180 seconds");
            }
            for variable in ["MOX_RECOVERY_PROBE", "MOX_RECOVERY_BROWSER_PROBE"] {
                if let Ok(probe) = std::env::var(variable) {
                    let base = base.clone();
                    let primary = token("mine");
                    let other = token("other");
                    let output = tokio::task::spawn_blocking(move || {
                        std::process::Command::new("node")
                            .arg(probe)
                            .env("MOX_PROBE_BASE", base)
                            .env("MOX_PROBE_TOKEN", primary)
                            .env("MOX_PROBE_OTHER_TOKEN", other)
                            .output()
                            .unwrap()
                    })
                    .await
                    .unwrap();
                    assert!(
                        output.status.success(),
                        "{}: {}\n{}",
                        variable,
                        String::from_utf8_lossy(&output.stderr),
                        String::from_utf8_lossy(&output.stdout)
                    );
                    println!("{}", String::from_utf8_lossy(&output.stdout));
                }
            }
            assert_eq!(experts_db::load_registry("other").len(), 1);
            server.abort();
        });
}

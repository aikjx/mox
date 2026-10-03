//! Real authenticated HTTP, SQLite and business handlers; no event consumer.
use axum::{middleware, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    alliance::{
        experts_common::ExpertsSharedState,
        experts_db,
        experts_events::{AllianceEvent, AllianceEventKind},
        experts_registry::build_experts_registry_router,
        experts_streams::build_experts_streams_router,
    },
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
};
use serde_json::json;
use sha2::Sha256;
use std::{sync::Arc, time::Duration};

fn token(tenant: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&json!({
            "sub":"resume-user", "username":"resume-user", "tenant_id":tenant,
            "iss":"mox-platform", "exp":9999999999_u64, "roles":["tenant_admin"]
        }))
        .unwrap(),
    );
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"resume-test-secret").unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

async fn next_frame(response: &mut reqwest::Response) -> String {
    let mut text = String::new();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !text.contains("\n\n") {
            let chunk = response.chunk().await.unwrap().expect("stream must remain open");
            text.push_str(std::str::from_utf8(&chunk).unwrap());
        }
    })
    .await
    .expect("a business frame must arrive");
    text
}

#[test]
fn reconnect_replays_only_own_durable_events_then_continues_live_with_bounded_errors() {
    let directory = tempfile::tempdir().unwrap();
    std::env::set_var("MOX_EXPERTS_DB_PATH", directory.path().join("experts.db"));
    std::env::set_var("MOX_AUDIT_LOG_PATH", directory.path().join("audit.ndjson"));
    let state = Arc::new(ExpertsSharedState::new());
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let auth = Arc::new(AuthMiddleware::new(AuthConfig {
                enabled: true,
                dev_mode: false,
                jwt_secret: "resume-test-secret".into(),
                public_paths: vec![],
                ..AuthConfig::default()
            }));
            let app: Router = build_experts_registry_router(state.clone())
                .merge(build_experts_streams_router(state.clone()))
                .layer(middleware::from_fn(move |req, next| {
                    auth_middleware(auth.clone(), req, next)
                }));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            let client = reqwest::Client::new();
            let stream_url = format!("{base}/api/alliance/events/stream");
            for (tenant, id) in [("mine", "one"), ("other", "foreign"), ("mine", "two")] {
                let response = client
                    .post(format!("{base}/api/experts"))
                    .bearer_auth(token(tenant))
                    .json(&json!({"id":id,"name":id}))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status().as_u16(), 200);
            }
            let mine = experts_db::load_event_log_by_tenant("mine");
            let foreign = experts_db::load_event_log_by_tenant("other")[0].event_id.clone();
            if let Ok(probe) = std::env::var("MOX_EVENT_RESUME_PROBE") {
                let probe_url = stream_url.clone();
                let probe_token = token("mine");
                let cursor = mine[0].event_id.clone();
                let expected = mine[1].event_id.clone();
                let output = tokio::task::spawn_blocking(move || {
                    std::process::Command::new("node")
                        .arg(probe)
                        .env("MOX_PROBE_URL", probe_url)
                        .env("MOX_PROBE_TOKEN", probe_token)
                        .env("MOX_PROBE_CURSOR", cursor)
                        .env("MOX_PROBE_EXPECTED", expected)
                        .output()
                        .unwrap()
                })
                .await
                .unwrap();
                assert!(
                    output.status.success(),
                    "frontend probe: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            for cursor in ["evt-missing", foreign.as_str()] {
                let response = client
                    .get(&stream_url)
                    .bearer_auth(token("mine"))
                    .header("Last-Event-ID", cursor)
                    .send()
                    .await
                    .unwrap();
                assert_eq!(
                    response.status().as_u16(),
                    410,
                    "unknown and foreign cursors have identical rejection"
                );
            }
            let response = client
                .get(&stream_url)
                .bearer_auth(token("mine"))
                .header("Last-Event-ID", "x".repeat(257))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), 400);
            let response = client
                .get(&stream_url)
                .header("Last-Event-ID", &mine[0].event_id)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), 401);
            let mut replay = client
                .get(&stream_url)
                .bearer_auth(token("mine"))
                .header("Last-Event-ID", &mine[0].event_id)
                .send()
                .await
                .unwrap();
            assert_eq!(replay.status().as_u16(), 200);
            let frame = next_frame(&mut replay).await;
            assert!(frame.contains(&format!("id: {}", mine[1].event_id)));
            assert!(frame.contains("event: ExpertRegistered"));
            assert!(!frame.contains(&mine[0].event_id));
            assert!(!frame.contains(&foreign));
            // Same-ID overlap between snapshot replay and broadcast must not duplicate.
            let old: AllianceEvent = serde_json::from_value(mine[1].payload.clone()).unwrap();
            state.events.emit(old);
            let response = client
                .post(format!("{base}/api/experts"))
                .bearer_auth(token("mine"))
                .json(&json!({"id":"three","name":"three"}))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), 200);
            let live = experts_db::load_event_log_by_tenant("mine");
            let frame = next_frame(&mut replay).await;
            assert!(frame.contains(&format!("id: {}", live[2].event_id)));
            assert!(!frame.contains(&mine[1].event_id));
            drop(replay);
            // Real SQL fixtures verify finite replay and corrupt payload failure, never mock responses.
            let conn = experts_db::open_experts_db().unwrap();
            for index in 0..201 {
                if index == 200 {
                    let response = client
                        .get(&stream_url)
                        .bearer_auth(token("mine"))
                        .header("Last-Event-ID", &live[2].event_id)
                        .send()
                        .await
                        .unwrap();
                    assert_eq!(
                        response.status().as_u16(),
                        200,
                        "exactly 200 history events are accepted"
                    );
                }
                let event = AllianceEvent::new(
                    AllianceEventKind::ExpertDisabled { expert_id: format!("fixture-{index}") },
                    "mine",
                    "resume-boundary",
                );
                experts_db::insert_event_log_conn(
                    &conn,
                    "mine",
                    &event.id,
                    event.kind.type_name(),
                    &event.source,
                    &event.occurred_at,
                    "",
                    &serde_json::to_value(&event).unwrap(),
                )
                .unwrap();
            }
            let response = client
                .get(&stream_url)
                .bearer_auth(token("mine"))
                .header("Last-Event-ID", &mine[0].event_id)
                .send()
                .await
                .unwrap();
            assert_eq!(
                response.status().as_u16(),
                409,
                "oversized replay requires authoritative refresh"
            );
            let newest =
                experts_db::load_event_log_by_tenant("mine").last().unwrap().event_id.clone();
            conn.execute("UPDATE alliance_event_log SET payload='{}' WHERE event_id=?1", [&newest])
                .unwrap();
            let before = experts_db::load_event_log_by_tenant("mine");
            let cursor = before[before.len() - 2].event_id.clone();
            let response = client
                .get(&stream_url)
                .bearer_auth(token("mine"))
                .header("Last-Event-ID", cursor)
                .send()
                .await
                .unwrap();
            assert_eq!(
                response.status().as_u16(),
                503,
                "corruption must not silently drop history"
            );
            std::env::set_var("MOX_EXPERTS_DB_PATH", directory.path());
            let response = client
                .get(&stream_url)
                .bearer_auth(token("mine"))
                .header("Last-Event-ID", &mine[0].event_id)
                .send()
                .await
                .unwrap();
            assert_eq!(
                response.status().as_u16(),
                503,
                "real database open failure must not become empty history"
            );
            std::env::set_var("MOX_EXPERTS_DB_PATH", directory.path().join("experts.db"));
            for (tenant, count, size) in [("total-size", 3, 800_000), ("single-size", 1, 1_048_576)]
            {
                let cursor = AllianceEvent::new(
                    AllianceEventKind::ExpertDisabled { expert_id: "size-cursor".into() },
                    tenant,
                    "size-boundary",
                );
                experts_db::insert_event_log_conn(
                    &conn,
                    tenant,
                    &cursor.id,
                    cursor.kind.type_name(),
                    &cursor.source,
                    &cursor.occurred_at,
                    "",
                    &serde_json::to_value(&cursor).unwrap(),
                )
                .unwrap();
                for index in 0..count {
                    let event = AllianceEvent::new(
                        AllianceEventKind::ExpertRegistered {
                            expert_id: index.to_string(),
                            name: "x".repeat(size),
                        },
                        tenant,
                        "size-boundary",
                    );
                    experts_db::insert_event_log_conn(
                        &conn,
                        tenant,
                        &event.id,
                        event.kind.type_name(),
                        &event.source,
                        &event.occurred_at,
                        "",
                        &serde_json::to_value(&event).unwrap(),
                    )
                    .unwrap();
                }
                let response = client
                    .get(&stream_url)
                    .bearer_auth(token(tenant))
                    .header("Last-Event-ID", cursor.id)
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status().as_u16(), 409, "payload byte limits require refresh");
            }
            server.abort();
        });
}

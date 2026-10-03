//! Real receiver latency and SQLite contention; no mocked IO or external provider claims.
use axum::{
    middleware,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    alliance::{
        experts_common::ExpertsSharedState,
        experts_db::open_experts_db,
        experts_events::{AllianceEvent, AllianceEventKind},
        experts_streams::build_experts_streams_router,
    },
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
};
use serde_json::{json, Value};
use sha2::Sha256;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

fn admin_token() -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&json!({
            "sub":"performance-user","username":"test","tenant_id":"fast", "roles":["tenant_admin"],
            "iss":"mox-platform","exp":9999999999_u64
        }))
        .unwrap(),
    );
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"performance-test-secret").unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn event(tenant: &str) -> AllianceEvent {
    AllianceEvent::new(
        AllianceEventKind::ExpertRegistered { expert_id: "e".into(), name: "actual".into() },
        tenant,
        "performance-test",
    )
}

#[tokio::test(flavor = "current_thread")]
async fn slow_targets_are_isolated_and_database_wait_does_not_block_runtime() {
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("MOX_EXPERTS_DB_PATH", temp.path().join("performance.db"));
    std::env::set_var("MOX_AUDIT_LOG_PATH", temp.path().join("audit.ndjson"));
    std::env::set_var("MOX_WEBHOOK_CONCURRENCY", "2");
    let receiver = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", receiver.local_addr().unwrap());
    std::env::set_var("MOX_WEBHOOK_ALLOWED_ORIGINS", &origin);
    let (slow_tx, mut slow_rx) = tokio::sync::mpsc::channel::<()>(16);
    let (fast_tx, mut fast_rx) = tokio::sync::mpsc::channel::<Value>(16);
    let release = Arc::new(tokio::sync::Notify::new());
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let receiver_app = Router::new()
        .route(
            "/slow",
            post({
                let release = release.clone();
                let active = active.clone();
                let peak = peak.clone();
                move || {
                    let tx = slow_tx.clone();
                    let release = release.clone();
                    let active = active.clone();
                    let peak = peak.clone();
                    async move {
                        let waiting = release.notified();
                        tokio::pin!(waiting);
                        waiting.as_mut().enable();
                        let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                        peak.fetch_max(count, Ordering::SeqCst);
                        tx.send(()).await.unwrap();
                        waiting.await;
                        active.fetch_sub(1, Ordering::SeqCst);
                        "accepted"
                    }
                }
            }),
        )
        .route(
            "/fast",
            post(move |Json(body): Json<Value>| {
                let tx = fast_tx.clone();
                async move {
                    tx.send(body).await.unwrap();
                    "accepted"
                }
            }),
        );
    let receiver_server =
        tokio::spawn(async move { axum::serve(receiver, receiver_app).await.unwrap() });
    let state = Arc::new(ExpertsSharedState::new());
    state.events.register_webhook("slow", format!("{origin}/slow"), vec![]).unwrap();
    state.events.register_webhook("fast", format!("{origin}/fast"), vec![]).unwrap();
    let auth = Arc::new(AuthMiddleware::new(AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: "performance-test-secret".into(),
        public_paths: vec![],
        ..AuthConfig::default()
    }));
    let app = build_experts_streams_router(state.clone())
        .route("/probe", get(|| async { "responsive" }))
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();
    tokio::time::timeout(Duration::from_secs(2), async {
        while state.events.receiver_count() < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut timings = Vec::new();
    for _ in 0..30 {
        state.events.emit(event("slow"));
        tokio::time::timeout(Duration::from_secs(2), slow_rx.recv())
            .await
            .unwrap()
            .unwrap();
        let started = Instant::now();
        state.events.emit(event("fast"));
        let fast = tokio::time::timeout(Duration::from_millis(500), fast_rx.recv())
            .await
            .expect("slow target blocked an unrelated tenant for more than 500ms")
            .unwrap();
        assert_eq!(fast["tenant"], "fast");
        timings.push(started.elapsed().as_micros());
        release.notify_waiters();
        while active.load(Ordering::SeqCst) > 0 {
            tokio::task::yield_now().await;
        }
    }
    // Fill both delivery slots and retain queued real events without unlimited task spawning.
    for _ in 0..6 {
        state.events.emit(event("slow"));
    }
    tokio::time::timeout(Duration::from_secs(2), slow_rx.recv())
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), slow_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(100), slow_rx.recv()).await.is_err());
    assert_eq!(peak.load(Ordering::SeqCst), 2, "configured concurrency must be an actual bound");
    for _ in 0..2 {
        release.notify_waiters();
        for _ in 0..2 {
            tokio::time::timeout(Duration::from_secs(2), slow_rx.recv())
                .await
                .unwrap()
                .unwrap();
        }
    }
    release.notify_waiters();
    while active.load(Ordering::SeqCst) > 0 {
        tokio::task::yield_now().await;
    }
    // An actual second SQLite connection holds a write lock on another OS thread.
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let (unlock_tx, unlock_rx) = std::sync::mpsc::channel();
    let locker = std::thread::spawn(move || {
        let conn = open_experts_db().unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        locked_tx.send(()).unwrap();
        let _ = unlock_rx.recv_timeout(Duration::from_secs(2));
        conn.execute_batch("ROLLBACK").unwrap();
    });
    locked_rx.recv().unwrap();
    let write_client = client.clone();
    let write_url = format!("{gateway}/api/alliance/events/webhooks");
    let input = json!({"url":format!("{origin}/fast"),"event_types":[]});
    let write = tokio::spawn(async move {
        write_client
            .post(write_url)
            .bearer_auth(admin_token())
            .json(&input)
            .send()
            .await
            .unwrap()
    });
    let started = Instant::now();
    // Force the event log consumer and target lookup to overlap the same real lock wait.
    state.events.emit(event("fast"));
    tokio::time::sleep(Duration::from_millis(50)).await;
    let response = client
        .get(format!("{gateway}/probe"))
        .bearer_auth(admin_token())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let responsive_ms = started.elapsed().as_millis();
    assert!(
        responsive_ms < 500,
        "SQLite write wait froze current-thread runtime: {responsive_ms}ms"
    );
    assert!(!write.is_finished(), "test must overlap the real SQLite lock wait");
    unlock_tx.send(()).unwrap();
    assert_eq!(write.await.unwrap().status().as_u16(), 200);
    locker.join().unwrap();
    println!(
        "PERFORMANCE_JSON={}",
        json!({"fast_delivery_microseconds":timings,"slow_target_released_after_fast":true,
        "runtime_probe_ms_under_sqlite_lock":responsive_ms,"observed_peak":peak.load(Ordering::SeqCst),"configured_concurrency":2,
        "scope":"single host current-thread Tokio, real TCP and SQLite, not production SLO"})
    );
    server.abort();
    receiver_server.abort();
}

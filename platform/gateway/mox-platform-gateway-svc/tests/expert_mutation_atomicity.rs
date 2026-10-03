//! Real HTTP/JWT/SQLite fault injection; no event-log consumer is running.
use axum::{middleware, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    alliance::{
        experts_common::ExpertsSharedState, experts_db,
        experts_registry::build_experts_registry_router,
    },
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
};
use serde_json::{json, Value};
use sha2::Sha256;
use std::sync::Arc;

fn token(tenant: &str, role: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&json!({
            "sub":"atomic-user", "username":"atomic-user", "tenant_id":tenant,
            "iss":"mox-platform", "exp":9999999999_u64, "roles":[role]
        }))
        .unwrap(),
    );
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"expert-atomic-test-secret").unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

#[test]
fn expert_mutations_commit_rows_and_events_together_or_preserve_previous_state() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("MOX_EXPERTS_DB_PATH", dir.path().join("experts.db"));
    std::env::set_var("MOX_AUDIT_LOG_PATH", dir.path().join("audit.ndjson"));
    // Construct outside Tokio: prove durability without the best-effort consumer.
    let state = Arc::new(ExpertsSharedState::new());
    let mut events = state.events.subscribe();
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let auth = Arc::new(AuthMiddleware::new(AuthConfig {
            enabled:true, dev_mode:false, jwt_secret:"expert-atomic-test-secret".into(),
            public_paths:vec![], ..AuthConfig::default()
        }));
        let app: Router = build_experts_registry_router(state.clone()).layer(
            middleware::from_fn(move |req,next| auth_middleware(auth.clone(),req,next)));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/api/experts",listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener,app).await.unwrap() });
        let client = reqwest::Client::new();
        let admin = token("atomic", "tenant_admin");
        let conn = experts_db::open_experts_db().unwrap();
        conn.execute_batch("CREATE TRIGGER reject_expert BEFORE INSERT ON experts BEGIN SELECT RAISE(ABORT,'injected row failure'); END;").unwrap();
        let response = client.post(&base).bearer_auth(&admin).json(&json!({"id":"exp-one","name":"Original"})).send().await.unwrap();
        assert_eq!(response.status().as_u16(),503,"storage failure must not return success");
        assert!(experts_db::load_registry("atomic").is_empty());
        assert!(state.registry.lock().get("atomic").is_none_or(|r| r.is_empty()));
        assert!(events.try_recv().is_err());
        conn.execute_batch("DROP TRIGGER reject_expert; CREATE TRIGGER reject_event BEFORE INSERT ON alliance_event_log BEGIN SELECT RAISE(ABORT,'injected event failure'); END;").unwrap();
        let response = client.post(&base).bearer_auth(&admin).json(&json!({"id":"exp-one","name":"Original"})).send().await.unwrap();
        assert_eq!(response.status().as_u16(),503,"event failure must roll back expert insert");
        assert!(experts_db::load_registry("atomic").is_empty());
        assert!(events.try_recv().is_err());
        conn.execute_batch("DROP TRIGGER reject_event;").unwrap();
        conn.execute_batch("CREATE TRIGGER ignore_event BEFORE INSERT ON alliance_event_log BEGIN SELECT RAISE(IGNORE); END;").unwrap();
        let response = client.post(&base).bearer_auth(&admin).json(&json!({"id":"exp-one","name":"Original"})).send().await.unwrap();
        assert_eq!(response.status().as_u16(),503,"silently ignored event must also roll back expert");
        assert!(experts_db::load_registry("atomic").is_empty());
        assert!(events.try_recv().is_err());
        conn.execute_batch("DROP TRIGGER ignore_event;").unwrap();

        let response = client.post(&base).bearer_auth(&admin).json(&json!({"id":"exp-one","name":"Original"})).send().await.unwrap();
        assert_eq!(response.status().as_u16(),200);
        let registered = events.try_recv().unwrap();
        let rows = experts_db::load_event_log_by_tenant("atomic");
        assert_eq!(rows.len(),1,"event is durable before HTTP success, without consumer");
        assert_eq!(rows[0].event_id,registered.id);
        assert!(experts_db::load_event_log_by_tenant("other").is_empty());
        // A separate connection writes a row absent from the in-memory projection.
        let independent = experts_db::open_experts_db().unwrap();
        let mut external = experts_db::load_registry("atomic")["exp-one"].clone();
        external.id = "external".into();
        independent.execute("INSERT INTO experts SELECT tenant_id,'external',name,title,organization,expert_type,status,enabled,avg_rating,created_at,updated_at,?1 FROM experts WHERE tenant_id='atomic' AND id='exp-one'",[serde_json::to_string(&external).unwrap()]).unwrap();
        conn.execute_batch("CREATE TRIGGER reject_update BEFORE UPDATE ON experts BEGIN SELECT RAISE(ABORT,'injected update failure'); END;").unwrap();
        let response = client.put(format!("{base}/exp-one")).bearer_auth(&admin).json(&json!({"name":"Changed"})).send().await.unwrap();
        assert_eq!(response.status().as_u16(),503);
        assert_eq!(state.registry.lock()["atomic"]["exp-one"].name,"Original");
        assert_eq!(experts_db::load_registry("atomic")["exp-one"].name,"Original");
        let response = client.delete(format!("{base}/exp-one")).bearer_auth(&admin).send().await.unwrap();
        assert_eq!(response.status().as_u16(),503);
        assert!(state.registry.lock()["atomic"]["exp-one"].enabled);
        assert!(events.try_recv().is_err());
        conn.execute_batch("DROP TRIGGER reject_update;").unwrap();
        let response = client.put(format!("{base}/exp-one")).bearer_auth(&admin).json(&json!({"name":"Changed"})).send().await.unwrap();
        assert_eq!(response.status().as_u16(),200);
        let external: i64 = conn.query_row("SELECT COUNT(*) FROM experts WHERE tenant_id='atomic' AND id='external'",[],|r|r.get(0)).unwrap();
        assert_eq!(external,1,"single-row mutation preserves independent writes");
        conn.execute_batch("CREATE TRIGGER reject_event BEFORE INSERT ON alliance_event_log BEGIN SELECT RAISE(ABORT,'injected event failure'); END;").unwrap();
        let response = client.delete(format!("{base}/exp-one")).bearer_auth(&admin).send().await.unwrap();
        assert_eq!(response.status().as_u16(),503);
        assert!(experts_db::load_registry("atomic")["exp-one"].enabled);
        assert!(state.registry.lock()["atomic"]["exp-one"].enabled);
        assert!(events.try_recv().is_err());
        conn.execute_batch("DROP TRIGGER reject_event;").unwrap();
        let response = client.delete(format!("{base}/exp-one")).bearer_auth(&admin).send().await.unwrap();
        assert_eq!(response.status().as_u16(),200);
        let disabled = events.try_recv().unwrap();
        assert!(!experts_db::load_registry("atomic")["exp-one"].enabled);
        assert_eq!(experts_db::load_event_log_by_tenant("atomic")[1].event_id,disabled.id);
        let response = client.delete(format!("{base}/exp-one")).bearer_auth(token("other","tenant_admin")).send().await.unwrap();
        assert_eq!(response.status().as_u16(),404);
        let response = client.post(&base).bearer_auth(token("atomic","user")).json(&json!({"name":"Denied"})).send().await.unwrap();
        assert_eq!(response.status().as_u16(),403);
        let response: Value = client.get(format!("{base}/exp-one")).bearer_auth(&admin).send().await.unwrap().json().await.unwrap();
        assert_eq!(response["code"],404);
        server.abort();
    });
}

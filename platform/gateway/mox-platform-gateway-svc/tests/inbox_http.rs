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

#[tokio::test]
async fn message_text_limits_reject_without_writes_and_accept_unicode_boundaries() {
    use mox_platform_iam_core::IamRepository;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("inbox.db");
    let iam = Arc::new(IamRepository::new(Arc::new(parking_lot::Mutex::new(
        rusqlite::Connection::open(directory.path().join("iam.db")).unwrap(),
    ))));
    iam.init_schema().unwrap();
    let tenant = iam.create_tenant("text", "Text", None, None).unwrap().tenant_id;
    let user = iam
        .create_user(&tenant, "user", "user", None, None, None, false)
        .unwrap()
        .user_id;
    let server = start_state(Arc::new(MessageCenterState::with_db_path(path).with_iam(iam))).await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({"message_type":"custom","title":"real","content":"text","channels":["in_app"],"receiver_ids":[user]});
    for (field, value) in [
        ("title", serde_json::json!(" ")),
        ("title", serde_json::json!("😀".repeat(201))),
        ("content", serde_json::json!(" \n ")),
        ("content", serde_json::json!("汉".repeat(10001))),
        ("receiver_ids", serde_json::json!([""])),
        ("receiver_ids", serde_json::json!(["x".repeat(129)])),
    ] {
        let mut invalid = body.clone();
        invalid[field] = value;
        let response = client
            .post(format!("{}/send", server.url))
            .bearer_auth(token(&tenant, &user))
            .json(&invalid)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400, "field {field}");
    }
    let list = client
        .get(format!("{}/messages", server.url))
        .bearer_auth(token(&tenant, &user))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(list["code"], 0);
    assert_eq!(list["total"], 0);
    let mut valid = body;
    valid["title"] = serde_json::json!("😀".repeat(200));
    valid["content"] = serde_json::json!("汉".repeat(10000));
    let mut ids = Vec::new();
    for _ in 0..2 {
        let response = client
            .post(format!("{}/send", server.url))
            .bearer_auth(token(&tenant, &user))
            .header("Idempotency-Key", "unicode-limit")
            .json(&valid)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        ids.push(response.json::<serde_json::Value>().await.unwrap()["data"]["message_id"].clone());
    }
    assert_eq!(ids[0], ids[1]);
    let list = client
        .get(format!("{}/messages", server.url))
        .bearer_auth(token(&tenant, &user))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(list["code"], 0);
    assert_eq!(list["total"], 1);
}

#[tokio::test]
async fn audit_query_is_filtered_bounded_and_authorized_in_actual_storage() {
    use mox_platform_iam_core::IamRepository;
    use serde_json::{json, Value};
    let dir = tempfile::tempdir().unwrap();
    let conn = Arc::new(parking_lot::Mutex::new(
        rusqlite::Connection::open(dir.path().join("iam.db")).unwrap(),
    ));
    let iam = Arc::new(IamRepository::new(conn.clone()));
    iam.init_schema().unwrap();
    let tenant = iam.create_tenant("audit", "Audit", None, None).unwrap().tenant_id;
    let foreign = iam.create_tenant("audit-other", "Other", None, None).unwrap().tenant_id;
    let admin = iam.create_user(&tenant, "a", "admin", None, None, None, true).unwrap().user_id;
    let ordinary = iam
        .create_user(&tenant, "o", "ordinary", None, None, None, false)
        .unwrap()
        .user_id;
    {
        let mut connection = conn.lock();
        let tx = connection.transaction().unwrap();
        for i in 0..130 {
            tx.execute("INSERT INTO audit_log(log_id,tenant_id,action,user_id,status_code,created_at,curr_hash,snapshot_before,snapshot_after) VALUES (?1,?2,?3,?4,?5,'2026-10-01T00:00:00Z','private-hash','private-before','private-after')",rusqlite::params![format!("audit-{i:03}"),tenant,if i%2==0 {"iam.api_key.create"} else {"iam.role.permissions.replace"},if i%3==0 {&admin} else {&ordinary},if i%5==0 {None} else {Some(200)}]).unwrap();
        }
        tx.execute("INSERT INTO audit_log(log_id,tenant_id,action,created_at,curr_hash) VALUES ('foreign',?1,'foreign','2026-10-01T00:00:00Z','private-hash')",[&foreign]).unwrap();
        tx.commit().unwrap();
    }
    let server = start_system(iam.clone(), &dir.path().join("inbox.db")).await;
    let client = reqwest::Client::new();
    let url = format!("{}/api/security/audit-log", server.url);
    assert_eq!(client.get(&url).send().await.unwrap().status(), 401);
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(token(&tenant, &ordinary))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let mut ids = Vec::new();
    for page in 1..=7 {
        let response = client
            .get(&url)
            .bearer_auth(token(&tenant, &admin))
            .query(&[("page", page), ("page_size", 20)])
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let data: Value = response.json::<Value>().await.unwrap()["data"].clone();
        assert_eq!(data["total"], 130);
        let items = data["items"].as_array().unwrap();
        assert_eq!(items.len(), if page == 7 { 10 } else { 20 });
        if page == 1 {
            assert!(items[0]["statusCode"].is_null());
        }
        ids.extend(items.iter().map(|r| r["id"].as_str().unwrap().to_owned()));
        let text = data.to_string();
        assert!(!text.contains("foreign") && !text.contains("private-"));
    }
    assert_eq!(ids, (0..130).map(|i| format!("audit-{i:03}")).collect::<Vec<_>>());
    let filters = [
        ("action", "iam.api_key.create"),
        ("actor", admin.as_str()),
        ("since", "2026-10-01T08:00:00+08:00"),
        ("until", "2026-10-01T00:00:00Z"),
    ];
    let filtered: Value = client
        .get(&url)
        .bearer_auth(token(&tenant, &admin))
        .query(&filters)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(filtered["data"]["total"], 22);
    let injection: Value = client
        .get(&url)
        .bearer_auth(token(&tenant, &admin))
        .query(&[("action", "' OR 1=1 --")])
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(injection["data"]["total"], 0);
    for query in [
        "page=0",
        "page_size=101",
        "page=-1",
        "since=bad",
        "since=2026-10-02T00:00:00Z&until=2026-10-01T00:00:00Z",
    ] {
        assert_eq!(
            client
                .get(format!("{url}?{query}"))
                .bearer_auth(token(&tenant, &admin))
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
    }
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(token(&tenant, &admin))
            .query(&[("tenant_id", &foreign)])
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    conn.lock()
        .execute(
            "UPDATE audit_log SET created_at='2026-10-01T23:00:00+14:00' WHERE log_id='audit-001'",
            [],
        )
        .unwrap();
    conn.lock()
        .execute(
            "UPDATE audit_log SET created_at='2026-10-01T10:00:00Z' WHERE log_id='audit-002'",
            [],
        )
        .unwrap();
    let time_order: Value = client
        .get(&url)
        .bearer_auth(token(&tenant, &admin))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(time_order["data"]["items"][0]["id"], "audit-002");
    assert_eq!(time_order["data"]["items"][1]["id"], "audit-001");
    let plan: String = conn.lock().query_row("EXPLAIN QUERY PLAN SELECT log_id FROM audit_log WHERE tenant_id=?1 ORDER BY julianday(created_at) DESC,log_id ASC LIMIT 20",[&tenant],|r|r.get(3)).unwrap();
    assert!(plan.contains("idx_audit_tenant_time"), "{plan}");
    let empty: Value = client
        .get(format!("{url}?page=4294967295&page_size=100"))
        .bearer_auth(token(&tenant, &admin))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(empty["data"]["items"], json!([]));
    assert_eq!(empty["data"]["total"], 130);
    conn.lock()
        .execute_batch("ALTER TABLE audit_log RENAME TO unavailable_audit_log")
        .unwrap();
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(token(&tenant, &admin))
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    conn.lock()
        .execute_batch("ALTER TABLE unavailable_audit_log RENAME TO audit_log")
        .unwrap();
    conn.lock()
        .execute("UPDATE iam_user SET is_superuser=0 WHERE user_id=?1", [&admin])
        .unwrap();
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(token(&tenant, &admin))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    server.task.abort();
}

#[tokio::test]
async fn key_pages_are_bounded_tenant_scoped_and_report_configuration_state() {
    use mox_platform_iam_core::IamRepository;
    use serde_json::{json, Value};
    let dir = tempfile::tempdir().unwrap();
    let conn = Arc::new(parking_lot::Mutex::new(
        rusqlite::Connection::open(dir.path().join("iam.db")).unwrap(),
    ));
    let iam = Arc::new(IamRepository::new(conn.clone()));
    iam.init_schema().unwrap();
    let tenant = iam.create_tenant("paged", "Paged", None, None).unwrap().tenant_id;
    let foreign = iam.create_tenant("foreign-page", "Foreign", None, None).unwrap().tenant_id;
    let admin = iam
        .create_user(&tenant, "admin", "admin", None, None, None, true)
        .unwrap()
        .user_id;
    let disabled = iam
        .create_user(&tenant, "disabled", "disabled", None, None, None, false)
        .unwrap()
        .user_id;
    conn.lock()
        .execute("UPDATE iam_user SET user_status='disabled' WHERE user_id=?1", [&disabled])
        .unwrap();
    for i in 0..130 {
        let key = iam
            .create_api_key(
                &tenant,
                &format!("key-{i:03}"),
                &format!("actual-fixture-key-{i}"),
                Some(&admin),
                None,
            )
            .unwrap();
        conn.lock().execute("UPDATE sys_api_key SET key_id=?1,created_at='2026-10-01T00:00:00Z' WHERE key_id=?2",rusqlite::params![format!("k{i:03}"),key.key_id]).unwrap();
    }
    iam.create_api_key(&foreign, "not-visible", "foreign-key", None, None).unwrap();
    conn.lock().execute_batch("UPDATE sys_api_key SET expires_at='2000-01-01T00:00:00Z' WHERE key_id='k120'; UPDATE sys_api_key SET expires_at='invalid' WHERE key_id='k121'; UPDATE sys_api_key SET scopes='admin' WHERE key_id='k122'; UPDATE sys_api_key SET user_id='missing' WHERE key_id='k124'; UPDATE sys_api_key SET status='revoked' WHERE key_id='k125';").unwrap();
    conn.lock()
        .execute("UPDATE sys_api_key SET user_id=?1 WHERE key_id='k123'", [&disabled])
        .unwrap();
    let server = start_system(iam.clone(), &dir.path().join("inbox.db")).await;
    let client = reqwest::Client::new();
    let base = format!("{}/api/security/api-keys", server.url);
    let mut ids = Vec::new();
    for page in 1..=7 {
        let response = client
            .get(&base)
            .bearer_auth(token(&tenant, &admin))
            .query(&[("page", page), ("page_size", 20)])
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let data: Value = response.json::<Value>().await.unwrap()["data"].clone();
        assert_eq!(data["total"], 130);
        assert_eq!(data["page"], page);
        let items = data["items"].as_array().unwrap();
        assert_eq!(items.len(), if page == 7 { 10 } else { 20 });
        ids.extend(items.iter().map(|r| r["id"].as_str().unwrap().to_owned()));
        if page == 7 {
            for (offset, state) in [
                "expired",
                "invalid_expiry",
                "unsupported_scope",
                "inactive_user",
                "missing_identity",
                "revoked_or_inactive",
            ]
            .iter()
            .enumerate()
            {
                assert_eq!(items[offset]["eligibility"], *state);
            }
            assert_eq!(items[6]["eligibility"], "eligible");
        }
        let serialized = data.to_string();
        assert!(
            !serialized.contains("actual-fixture-key")
                && !serialized.contains("sha256:")
                && !serialized.contains("not-visible")
        );
    }
    assert_eq!(ids, (0..130).map(|i| format!("k{i:03}")).collect::<Vec<_>>());
    for query in
        ["page=0", "page_size=0", "page_size=101", "page=-1", "page_size=bad", "page=4294967296"]
    {
        assert_eq!(
            client
                .get(format!("{base}?{query}"))
                .bearer_auth(token(&tenant, &admin))
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
    }
    let empty = client
        .get(format!("{base}?page=4294967295&page_size=100"))
        .bearer_auth(token(&tenant, &admin))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(empty["data"]["total"], 130);
    assert_eq!(empty["data"]["items"], json!([]));
    assert!(iam.authenticate_api_key("actual-fixture-key-120").unwrap().is_none());
    assert!(iam.authenticate_api_key("actual-fixture-key-121").unwrap().is_none());
    let plan: String = conn.lock().query_row("EXPLAIN QUERY PLAN SELECT key_id FROM sys_api_key WHERE tenant_id=?1 ORDER BY created_at DESC,key_id ASC LIMIT 20",[&tenant],|r|r.get(3)).unwrap();
    assert!(plan.contains("idx_sys_api_key_tenant_created"), "{plan}");
    conn.lock()
        .execute("UPDATE iam_tenant SET tenant_status='disabled' WHERE tenant_id=?1", [&tenant])
        .unwrap();
    assert_eq!(
        client
            .get(&base)
            .bearer_auth(token(&tenant, &admin))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    server.task.abort();
}

#[tokio::test]
async fn persistent_api_keys_use_actual_identity_and_transactional_revocation() {
    use mox_platform_iam_core::{api_keys::KeyCommand, IamRepository};
    use serde_json::json;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("iam.db");
    let conn = Arc::new(parking_lot::Mutex::new(rusqlite::Connection::open(&db).unwrap()));
    let iam = Arc::new(IamRepository::new(conn.clone()));
    iam.init_schema().unwrap();
    let tenant = iam.create_tenant("key-owner", "Owner", None, None).unwrap().tenant_id;
    let foreign = iam.create_tenant("key-foreign", "Foreign", None, None).unwrap().tenant_id;
    let admin = iam
        .create_user(&tenant, "a", "key-admin", None, None, None, true)
        .unwrap()
        .user_id;
    let other = iam
        .create_user(&foreign, "a", "foreign-admin", None, None, None, true)
        .unwrap()
        .user_id;
    let ordinary = iam
        .create_user(&tenant, "o", "ordinary", None, None, None, false)
        .unwrap()
        .user_id;
    let inbox = dir.path().join("inbox.db");
    let first = start_system(iam.clone(), &inbox).await;
    let client = reqwest::Client::new();
    let create_url = format!("{}/api/security/api-keys", first.url);
    assert_eq!(
        client
            .post(&create_url)
            .json(&json!({"name":"no-login"}))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .post(&create_url)
            .bearer_auth(token(&tenant, &ordinary))
            .json(&json!({"name":"ordinary"}))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        client
            .post(&create_url)
            .bearer_auth(token(&tenant, &admin))
            .json(&json!({"name":"fake-scope","permissions":["admin"]}))
            .send()
            .await
            .unwrap()
            .status(),
        422
    );
    let response = client
        .post(&create_url)
        .bearer_auth(token(&tenant, &admin))
        .json(&json!({"name":"real-client"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let value: serde_json::Value = response.json().await.unwrap();
    let id = value["data"]["id"].as_str().unwrap();
    let key = value["data"]["api_key"].as_str().unwrap();
    assert_eq!(value["data"]["user_id"], admin);
    let stored: String = conn
        .lock()
        .query_row("SELECT api_key FROM sys_api_key WHERE key_id=?1", [id], |r| r.get(0))
        .unwrap();
    assert!(stored.starts_with("sha256:"));
    assert_ne!(stored, key);
    assert!(iam.authenticate_api_key(&stored).unwrap().is_none());
    let list = client
        .get(&create_url)
        .bearer_auth(token(&tenant, &admin))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!list.contains(key) && !list.contains(&stored));
    assert_eq!(
        client
            .delete(format!("{create_url}/{id}"))
            .bearer_auth(token(&foreign, &other))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        client
            .post(format!("{}/api/security/validate", first.url))
            .bearer_auth(token(&foreign, &other))
            .json(&json!({"api_key":key}))
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap()["data"]["valid"],
        false
    );
    first.task.abort();
    let reopened = Arc::new(IamRepository::new(Arc::new(parking_lot::Mutex::new(
        rusqlite::Connection::open(&db).unwrap(),
    ))));
    let second = start_system(reopened.clone(), &inbox).await;
    let notifications = format!("{}/api/notifications", second.url);
    assert_eq!(
        client
            .get(&notifications)
            .header("X-API-Key", key)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    assert_eq!(reopened.authenticate_api_key(key).unwrap().unwrap().tenant_id, tenant);
    let delivered = client.post(format!("{}/api/enterprise/message/send",second.url))
        .header("X-API-Key",key).json(&json!({"message_type":"task","title":"Machine delivery","content":"Persisted actual user","channels":["in_app"],"receiver_ids":[admin]})).send().await.unwrap();
    assert_eq!(delivered.status(), 200);
    let list: serde_json::Value = client
        .get(&notifications)
        .header("X-API-Key", key)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list["data"]["total"], 1);
    conn.lock()
        .execute_batch("ALTER TABLE sys_api_key RENAME TO temporarily_unavailable_keys")
        .unwrap();
    assert_eq!(
        client
            .get(&notifications)
            .header("X-API-Key", key)
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    conn.lock()
        .execute_batch("ALTER TABLE temporarily_unavailable_keys RENAME TO sys_api_key")
        .unwrap();
    assert_eq!(
        client
            .get(format!("{}/api/system/iam/permissions", second.url))
            .header("X-API-Key", key)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        client
            .get(&notifications)
            .header("X-API-Key", key)
            .header("X-Tenant-Id", &foreign)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    conn.lock().execute_batch("CREATE TRIGGER reject_key_audit BEFORE INSERT ON audit_log WHEN NEW.action LIKE 'iam.api_key.%' BEGIN SELECT RAISE(ABORT,'audit unavailable'); END;").unwrap();
    let url = format!("{}/api/security/api-keys", second.url);
    assert_eq!(
        client
            .delete(format!("{url}/{id}"))
            .bearer_auth(token(&tenant, &admin))
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    assert_eq!(
        client
            .get(&notifications)
            .header("X-API-Key", key)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(token(&tenant, &admin))
            .json(&json!({"name":"rolled-back"}))
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    assert_eq!(
        conn.lock()
            .query_row("SELECT COUNT(*) FROM sys_api_key", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    conn.lock().execute_batch("DROP TRIGGER reject_key_audit").unwrap();
    iam.administer_api_keys(&tenant, &admin, KeyCommand::Revoke(id.to_owned()))
        .unwrap();
    assert_eq!(
        client
            .get(&notifications)
            .header("X-API-Key", key)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    // Legacy plaintext is upgraded only after valid, same-tenant principal validation.
    let legacy = iam
        .create_api_key(&tenant, "legacy", "actual-legacy-key", Some(&admin), None)
        .unwrap();
    let duplicate = iam
        .create_api_key(&foreign, "duplicate", "actual-legacy-key", Some(&other), None)
        .unwrap();
    assert!(iam.authenticate_api_key("actual-legacy-key").unwrap().is_none());
    conn.lock()
        .execute("DELETE FROM sys_api_key WHERE key_id=?1", [&duplicate.key_id])
        .unwrap();
    assert!(iam
        .authenticate_api_key_in_tenant("actual-legacy-key", &foreign)
        .unwrap()
        .is_none());
    assert_eq!(iam.get_api_key(&legacy.key_id).unwrap().unwrap().api_key, "actual-legacy-key");
    assert!(iam.authenticate_api_key("actual-legacy-key").unwrap().is_some());
    assert!(iam.get_api_key(&legacy.key_id).unwrap().unwrap().api_key.starts_with("sha256:"));
    for expiry in ["2000-01-01T00:00:00Z", "invalid"] {
        conn.lock()
            .execute(
                "UPDATE sys_api_key SET expires_at=?1 WHERE key_id=?2",
                rusqlite::params![expiry, legacy.key_id],
            )
            .unwrap();
        assert!(iam.authenticate_api_key("actual-legacy-key").unwrap().is_none());
    }
    conn.lock()
        .execute(
            "UPDATE sys_api_key SET expires_at=NULL,scopes='admin' WHERE key_id=?1",
            [&legacy.key_id],
        )
        .unwrap();
    assert!(iam.authenticate_api_key("actual-legacy-key").unwrap().is_none());
    conn.lock()
        .execute("UPDATE sys_api_key SET scopes=NULL WHERE key_id=?1", [&legacy.key_id])
        .unwrap();
    conn.lock()
        .execute("UPDATE iam_user SET user_status='disabled' WHERE user_id=?1", [&admin])
        .unwrap();
    assert!(iam.authenticate_api_key("actual-legacy-key").unwrap().is_none());
    conn.lock()
        .execute(
            "UPDATE iam_user SET user_status='active',is_superuser=0 WHERE user_id=?1",
            [&admin],
        )
        .unwrap();
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(token(&tenant, &admin))
            .json(&json!({"name":"revoked-admin"}))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    conn.lock()
        .execute("UPDATE iam_tenant SET tenant_status='disabled' WHERE tenant_id=?1", [&tenant])
        .unwrap();
    assert!(iam.authenticate_api_key("actual-legacy-key").unwrap().is_none());
    second.task.abort();
}

async fn start_system(
    iam: Arc<mox_platform_iam_core::IamRepository>,
    inbox: &std::path::Path,
) -> Server {
    use mox_platform_gateway_svc::{
        actuator::{LogStore, RuntimeMetrics},
        config::GatewayConfig,
        enterprise_features::EnterpriseState,
        o11y::{MetricsCollector, ObservabilityConfig},
        rate_limit::RateLimiter,
        GatewayState,
    };
    let config = GatewayConfig {
        auth: AuthConfig {
            enabled: true,
            dev_mode: false,
            jwt_secret: "isolated-inbox-http-secret".into(),
            public_paths: vec!["/api/auth/login".into()],
            ..AuthConfig::default()
        },
        ..GatewayConfig::default()
    };
    let auth = Arc::new(AuthMiddleware::new(config.auth.clone()).with_iam(iam.clone()));
    let state = GatewayState {
        rate_limiter: Arc::new(RateLimiter::new(config.rate_limit.clone())),
        metrics: Arc::new(MetricsCollector::new(ObservabilityConfig {
            metrics_enabled: true,
            tracing_enabled: false,
            logging_enabled: true,
        })),
        logs: LogStore::new(128),
        runtime: Arc::new(RuntimeMetrics::new()),
        process_engine: Arc::new(mox_flow_unified_process_core::ProcessEngine::new()),
        enterprise: Arc::new(EnterpriseState::with_message_center(
            MessageCenterState::with_db_path(inbox.to_path_buf()).with_iam(iam.clone()),
        )),
        iam: iam.clone(),
        auth: auth.clone(),
        config: Arc::new(config),
    };
    let enterprise =
        mox_platform_gateway_svc::enterprise_features::build_enterprise_router_for_gateway(&state);
    let notifications = mox_platform_gateway_svc::notification::build_notification_router(
        Arc::new(mox_platform_gateway_svc::notification::NotificationState::with_inbox(
            state.enterprise.message_center.clone(),
        )),
    );
    let router = mox_platform_gateway_svc::system::build_system_router(iam)
        .merge(mox_platform_gateway_svc::system::build_security_router())
        .nest("/api/enterprise", enterprise)
        .merge(mox_platform_gateway_svc::rbac::build_rbac_router())
        .with_state(state)
        .merge(notifications)
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Server { url, task }
}

#[tokio::test]
async fn legacy_role_writes_require_actual_admin_and_login_remains_available() {
    use mox_platform_iam_core::IamRepository;
    let dir = tempfile::tempdir().unwrap();
    let iam = Arc::new(IamRepository::new(Arc::new(parking_lot::Mutex::new(
        rusqlite::Connection::open(dir.path().join("iam.db")).unwrap(),
    ))));
    iam.init_schema().unwrap();
    let tenant = iam.create_tenant("owned", "Owned", None, None).unwrap().tenant_id;
    let foreign = iam.create_tenant("foreign", "Foreign", None, None).unwrap().tenant_id;
    let admin = iam
        .create_user(
            &tenant,
            "a",
            "admin",
            None,
            Some(&mox_platform_gateway_svc::password_hash::hash_new("actual-test-password")),
            None,
            true,
        )
        .unwrap()
        .user_id;
    let ordinary = iam
        .create_user(&tenant, "o", "ordinary", None, None, None, false)
        .unwrap()
        .user_id;
    let role = iam
        .create_role(&tenant, "owned", "Owned role", None, None, None, "active", None)
        .unwrap()
        .role_id;
    let other = iam
        .create_role(&foreign, "foreign", "Foreign role", None, None, None, "active", None)
        .unwrap()
        .role_id;
    let server = start_system(iam.clone(), &dir.path().join("inbox.db")).await;
    let client = reqwest::Client::new();
    let login=client.post(format!("{}/api/auth/login",server.url)).json(&serde_json::json!({"tenant_id":tenant,"username":"admin","password":"actual-test-password"})).send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    assert_eq!(login["code"], 0, "{login}");
    let principal = login["data"]["access_token"]
        .as_str()
        .or_else(|| login["data"]["accessToken"].as_str())
        .unwrap();
    let roles = client
        .get(format!("{}/api/system/role", server.url))
        .bearer_auth(principal)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(roles["data"].as_array().unwrap().len(), 1);
    assert_eq!(roles["data"][0]["id"], role);
    let current = client
        .get(format!("{}/rbac/v1/current", server.url))
        .bearer_auth(principal)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(current["data"]["tenant_id"], tenant);
    let denied = client
        .get(format!("{}/api/system/permissions?user_id={ordinary}", server.url))
        .bearer_auth(principal)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(denied["code"], 403);
    assert_eq!(
        client
            .put(format!("{}/api/system/user/{ordinary}/roles", server.url))
            .bearer_auth(token(&tenant, &ordinary))
            .json(&serde_json::json!({"roleIds":[role]}))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        client
            .put(format!("{}/api/system/role/{other}", server.url))
            .bearer_auth(principal)
            .json(&serde_json::json!({"name":"attack"}))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        404
    );
    assert_eq!(
        client
            .get(format!("{}/api/system/role?tenant_id={foreign}", server.url))
            .bearer_auth(principal)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    let endpoint = format!("{}/api/system/user/{ordinary}/roles", server.url);
    let assigned = client
        .put(&endpoint)
        .bearer_auth(principal)
        .json(&serde_json::json!({"roleIds":[role]}))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(assigned["code"], 0);
    assert!(iam.set_user_roles(&tenant, &ordinary, &[other]).is_err());
    assert_eq!(iam.get_user_roles(&tenant, &ordinary).unwrap().len(), 1);
    iam.conn
        .lock()
        .execute("UPDATE iam_user SET is_superuser=0 WHERE user_id=?1", [admin])
        .unwrap();
    assert_eq!(
        client
            .put(&endpoint)
            .bearer_auth(principal)
            .json(&serde_json::json!({"roleIds":[]}))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        403
    );
}

#[tokio::test]
async fn notification_adapter_reads_same_inbox_and_bulk_read_is_scoped_atomic() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("inbox.db");
    let server = start(&path).await;
    let client = reqwest::Client::new();
    let owner = token("tenant-a", "owner");
    let mut owned = Vec::new();
    for (tenant, user, kind) in [
        ("tenant-a", "owner", "task"),
        ("tenant-a", "owner", "alert"),
        ("tenant-a", "other", "system"),
        ("tenant-b", "owner", "task"),
    ] {
        let response = client.post(format!("{}/send",server.url)).bearer_auth(token(tenant,user))
            .json(&serde_json::json!({"message_type":kind,"title":"actual","content":"persisted","channels":["in_app"],"receiver_ids":[user]})).send().await.unwrap();
        assert_eq!(response.status().as_u16(), 200);
        let id = response.json::<serde_json::Value>().await.unwrap()["data"]["message_id"]
            .as_str()
            .unwrap()
            .to_owned();
        if tenant == "tenant-a" && user == "owner" {
            owned.push(id);
        }
    }
    let endpoint = format!("{}/api/notifications", server.url);
    assert_eq!(client.get(&endpoint).send().await.unwrap().status().as_u16(), 401);
    let counts = client
        .get(format!("{endpoint}/unread-count"))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(counts["data"]["total"], 2);
    assert_eq!(counts["data"]["by_type"]["task"], 1);
    let list = client
        .get(format!("{endpoint}?page=1&page_size=1"))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(list["data"]["total"], 2);
    assert_eq!(list["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        client
            .put(format!("{endpoint}/{}/read", owned[0]))
            .bearer_auth(token("tenant-a", "other"))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        404
    );
    assert_eq!(
        client
            .get(format!("{endpoint}?page=9223372036854775807&page_size=100"))
            .bearer_auth(&owner)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        400
    );
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_bulk BEFORE UPDATE ON inbox_messages BEGIN SELECT RAISE(ABORT,'reject bulk'); END;").unwrap();
    assert_eq!(
        client
            .put(format!("{endpoint}/read-all"))
            .bearer_auth(&owner)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        503
    );
    let store = MessageCenterState::with_db_path(path);
    assert_eq!(store.repository.stats("tenant-a", "owner").unwrap().sent, 2);
    assert!(store
        .repository
        .receipt("tenant-a", "owner", &owned[0])
        .unwrap()
        .unwrap()
        .read_at
        .is_none());
    conn.execute_batch("DROP TRIGGER reject_bulk;").unwrap();
    let bulk = client
        .put(format!("{endpoint}/read-all"))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(bulk["data"]["updated_count"], 2);
    assert_eq!(store.repository.stats("tenant-a", "owner").unwrap().read, 2);
    assert_eq!(store.repository.stats("tenant-a", "other").unwrap().sent, 1);
    assert_eq!(store.repository.stats("tenant-b", "owner").unwrap().sent, 1);
    let first_read = store
        .repository
        .receipt("tenant-a", "owner", &owned[0])
        .unwrap()
        .unwrap()
        .read_at;
    assert_eq!(store.repository.mark_all_read("tenant-a", "owner").unwrap(), 0);
    assert_eq!(
        store
            .repository
            .receipt("tenant-a", "owner", &owned[0])
            .unwrap()
            .unwrap()
            .read_at,
        first_read
    );
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn start(path: &std::path::Path) -> Server {
    let state = Arc::new(MessageCenterState::with_db_path(path.to_path_buf()));
    start_state(state).await
}
async fn start_state(state: Arc<MessageCenterState>) -> Server {
    let permissions = match state.iam.clone() {
        Some(iam) => mox_platform_gateway_svc::system::iam_permissions::router::<
            Arc<mox_platform_iam_core::IamRepository>,
        >()
        .with_state(iam),
        None => axum::Router::new(),
    };
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
        .with_state(state.clone())
        .merge(mox_platform_gateway_svc::notification::build_notification_router(Arc::new(
            mox_platform_gateway_svc::notification::NotificationState::with_inbox(state),
        )))
        .merge(permissions)
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Server { url, task }
}

#[tokio::test]
async fn permission_management_authorizes_delivery_and_rolls_back_audit_failures() {
    use mox_platform_iam_core::IamRepository;
    let dir = tempfile::tempdir().unwrap();
    let iam_path = dir.path().join("iam.db");
    let iam = Arc::new(IamRepository::new(Arc::new(parking_lot::Mutex::new(
        rusqlite::Connection::open(&iam_path).unwrap(),
    ))));
    iam.init_schema().unwrap();
    let tenant = iam.create_tenant("actual", "Actual", None, None).unwrap().tenant_id;
    let admin = iam
        .create_user(&tenant, "admin", "admin", None, None, None, true)
        .unwrap()
        .user_id;
    let sender = iam
        .create_user(&tenant, "sender", "sender", None, None, None, false)
        .unwrap()
        .user_id;
    let recipient = iam
        .create_user(&tenant, "recipient", "recipient", None, None, None, false)
        .unwrap()
        .user_id;
    let role = iam
        .create_role(&tenant, "sender", "Sender", None, None, None, "active", None)
        .unwrap()
        .role_id;
    iam.set_user_roles(&tenant, &sender, std::slice::from_ref(&role)).unwrap();
    let inbox = dir.path().join("inbox.db");
    let server = start_state(Arc::new(
        MessageCenterState::with_db_path(inbox.clone()).with_iam(iam.clone()),
    ))
    .await;
    let client = reqwest::Client::new();
    let principal = token(&tenant, &admin);
    let register = format!("{}/api/system/iam/permissions/message-send", server.url);
    assert_eq!(client.post(&register).send().await.unwrap().status().as_u16(), 401);
    assert_eq!(
        client
            .post(&register)
            .bearer_auth(token(&tenant, &sender))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    let id = client
        .post(&register)
        .bearer_auth(&principal)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap()["data"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        client
            .post(&register)
            .bearer_auth(&principal)
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap()["data"]["id"],
        id
    );
    let endpoint = format!("{}/api/system/iam/roles/{role}/permissions", server.url);
    let replace = |version: i64, ids: Vec<String>| {
        client
            .put(&endpoint)
            .bearer_auth(&principal)
            .json(&serde_json::json!({"version":version,"permission_ids":ids}))
    };
    let deliver = || {
        client.post(format!("{}/send",server.url)).bearer_auth(token(&tenant,&sender)).json(&serde_json::json!({"message_type":"task","title":"Authorized","content":"actual","channels":["in_app"],"receiver_ids":[recipient]}))
    };
    assert_eq!(deliver().send().await.unwrap().status().as_u16(), 403);
    assert_eq!(
        replace(1, vec!["nonexistent-permission".into()])
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        400
    );
    assert_eq!(replace(1, vec![id.clone()]).send().await.unwrap().status().as_u16(), 200);
    assert_eq!(deliver().send().await.unwrap().status().as_u16(), 200);
    assert_eq!(iam.get_user_permissions(&tenant, &sender).unwrap(), vec!["message:send"]);
    assert_eq!(replace(1, vec![]).send().await.unwrap().status().as_u16(), 409);
    assert_eq!(replace(2, vec![]).send().await.unwrap().status().as_u16(), 200);
    assert!(iam.get_user_permissions(&tenant, &sender).unwrap().is_empty());
    assert_eq!(deliver().send().await.unwrap().status().as_u16(), 403);
    iam.conn.lock().execute_batch("CREATE TRIGGER reject_permission_audit BEFORE INSERT ON audit_log BEGIN SELECT RAISE(ABORT,'actual audit failure'); END;").unwrap();
    assert_eq!(replace(3, vec![id.clone()]).send().await.unwrap().status().as_u16(), 503);
    assert_eq!(deliver().send().await.unwrap().status().as_u16(), 403);
    let snapshot = client
        .get(&endpoint)
        .bearer_auth(&principal)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(snapshot["data"]["version"], 3);
    assert_eq!(snapshot["data"]["permission_ids"], serde_json::json!([]));
    iam.conn.lock().execute_batch("DROP TRIGGER reject_permission_audit;").unwrap();
    let other = Arc::new(IamRepository::new(Arc::new(parking_lot::Mutex::new(
        rusqlite::Connection::open(&iam_path).unwrap(),
    ))));
    let second =
        start_state(Arc::new(MessageCenterState::with_db_path(inbox).with_iam(other))).await;
    let concurrent = client
        .put(format!("{}/api/system/iam/roles/{role}/permissions", second.url))
        .bearer_auth(&principal)
        .json(&serde_json::json!({"version":3,"permission_ids":[id]}));
    let (first, last) = tokio::join!(replace(3, vec![id.clone()]).send(), concurrent.send());
    let mut statuses = vec![first.unwrap().status().as_u16(), last.unwrap().status().as_u16()];
    statuses.sort();
    assert_eq!(statuses, vec![200, 409]);
    let restored = client
        .get(format!("{}/api/system/iam/roles/{role}/permissions", second.url))
        .bearer_auth(&principal)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(restored["data"]["version"], 4);
    assert_eq!(restored["data"]["permission_ids"], serde_json::json!([id]));
    assert_eq!(
        iam.conn
            .lock()
            .query_row("SELECT COUNT(*) FROM audit_log WHERE tenant_id=?1", [&tenant], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        client
            .get(format!("{}/api/system/iam/roles/{role}/permissions", second.url))
            .bearer_auth(token("other-tenant", &admin))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    iam.conn
        .lock()
        .execute("UPDATE iam_user SET is_superuser=0 WHERE user_id=?1", [admin])
        .unwrap();
    assert_eq!(
        client
            .get(&endpoint)
            .bearer_auth(&principal)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        403
    );
}

#[tokio::test]
async fn iam_delivery_checks_fresh_permissions_and_independent_recipient_state() {
    use mox_platform_iam_core::IamRepository;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("inbox.db");
    let iam = Arc::new(IamRepository::new(Arc::new(parking_lot::Mutex::new(
        rusqlite::Connection::open(directory.path().join("iam.db")).unwrap(),
    ))));
    iam.init_schema().unwrap();
    iam.conn.lock().execute_batch("INSERT INTO iam_tenant (tenant_id,tenant_code,tenant_name,tenant_status,created_at,updated_at) VALUES ('a','a','a','active','now','now'),('b','b','b','active','now','now');
        INSERT INTO iam_user (user_id,tenant_id,user_code,username,user_status,created_at,updated_at) VALUES ('sender','a','s','s','active','now','now'),('sender2','a','s2','s2','active','now','now'),('one','a','1','1','active','now','now'),('two','a','2','2','active','now','now'),('foreign','b','f','f','active','now','now');
        INSERT INTO iam_role (role_id,tenant_id,role_code,role_name,role_type,data_scope,created_at,updated_at) VALUES ('r','a','r','r','custom','self','now','now');
        INSERT INTO iam_permission (perm_id,tenant_id,perm_code,perm_name,resource_id,resource_type,perm_action,created_at,updated_at) VALUES ('p','a','message:send','send','message','api','send','now','now');
        INSERT INTO iam_role_permission (rp_id,tenant_id,role_id,perm_id,created_at) VALUES ('rp','a','r','p','now');").unwrap();
    let server =
        start_state(Arc::new(MessageCenterState::with_db_path(path.clone()).with_iam(iam.clone())))
            .await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({"message_type":"task","title":"real delivery","content":"persisted","channels":["in_app"],"receiver_ids":["two","one","one"]});
    let send = |sender: &str, body: serde_json::Value| {
        client
            .post(format!("{}/send", server.url))
            .bearer_auth(token("a", sender))
            .header("Idempotency-Key", "batch-1")
            .json(&body)
    };
    assert_eq!(send("sender", body.clone()).send().await.unwrap().status().as_u16(), 403);
    iam.conn.lock().execute_batch("INSERT INTO iam_user_role (ur_id,tenant_id,user_id,role_id,created_at) VALUES ('ur','a','sender','r','now'),('ur2','a','sender2','r','now');").unwrap();
    let second_iam = Arc::new(IamRepository::new(Arc::new(parking_lot::Mutex::new(
        rusqlite::Connection::open(directory.path().join("iam.db")).unwrap(),
    ))));
    let second =
        start_state(Arc::new(MessageCenterState::with_db_path(path.clone()).with_iam(second_iam)))
            .await;
    let mut jobs = Vec::new();
    for index in 0..12 {
        let url = if index % 2 == 0 { server.url.clone() } else { second.url.clone() };
        let client = client.clone();
        let body = body.clone();
        jobs.push(tokio::spawn(async move {
            let response = client
                .post(format!("{url}/send"))
                .bearer_auth(token("a", "sender"))
                .header("Idempotency-Key", "batch-1")
                .json(&body)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), 200);
            response.json::<serde_json::Value>().await.unwrap()["data"]["message_id"]
                .as_str()
                .unwrap()
                .to_owned()
        }));
    }
    let mut ids = Vec::new();
    for job in jobs {
        ids.push(job.await.unwrap());
    }
    assert!(ids.iter().all(|id| id == &ids[0]));
    let id = ids[0].clone();
    let response = send("sender", body.clone())
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(response["data"]["message_id"], id);
    let mut reordered = body.clone();
    reordered["receiver_ids"] = serde_json::json!(["one", "two"]);
    assert_eq!(
        send("sender", reordered)
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap()["data"]["message_id"],
        id
    );
    let mut changed = body.clone();
    changed["content"] = serde_json::json!("changed");
    assert_eq!(send("sender", changed).send().await.unwrap().status().as_u16(), 409);
    let mut oversized = body.clone();
    oversized["receiver_ids"] = serde_json::json!(vec!["one"; 101]);
    assert_eq!(send("sender", oversized).send().await.unwrap().status().as_u16(), 400);
    let response = send("sender2", body.clone())
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_ne!(response["data"]["message_id"], id);
    assert_eq!(
        client
            .post(format!("{}/messages/{id}/read", server.url))
            .bearer_auth(token("a", "one"))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        200
    );
    let store = mox_platform_gateway_svc::message_center::repository::InboxRepository::new(path);
    assert!(store.receipt("a", "one", &id).unwrap().unwrap().read_at.is_some());
    assert!(store.receipt("a", "two", &id).unwrap().unwrap().read_at.is_none());
    assert!(store.get("a", "sender", &id).unwrap().is_none());
    assert_eq!(store.get("a", "one", &id).unwrap().unwrap().receiver_ids, vec!["one"]);
    iam.conn.lock().execute_batch("DELETE FROM iam_role_permission;").unwrap();
    assert_eq!(send("sender", body.clone()).send().await.unwrap().status().as_u16(), 403);
    iam.conn.lock().execute_batch("INSERT INTO iam_role_permission (rp_id,tenant_id,role_id,perm_id,created_at) VALUES ('rp','a','r','p','now'); UPDATE iam_user SET user_status='disabled' WHERE user_id='two';").unwrap();
    assert_eq!(send("sender", body.clone()).send().await.unwrap().status().as_u16(), 403);
    let mut invalid = body;
    invalid["receiver_ids"] = serde_json::json!(["one", "foreign"]);
    assert_eq!(send("sender", invalid).send().await.unwrap().status().as_u16(), 403);
    assert_eq!(
        client
            .get(format!("{}/api/notifications", server.url))
            .bearer_auth(token("a", "two"))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        client
            .get(format!("{}/messages/{id}", server.url))
            .bearer_auth(token("a", "two"))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        403
    );
    // A failure on the second actual recipient rolls back the first recipient and dedupe key.
    iam.conn
        .lock()
        .execute_batch("UPDATE iam_user SET user_status='active' WHERE user_id='two';")
        .unwrap();
    let conn = rusqlite::Connection::open(directory.path().join("inbox.db")).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_second BEFORE INSERT ON inbox_receipts WHEN NEW.receiver='two' BEGIN SELECT RAISE(ABORT,'actual batch failure'); END;").unwrap();
    let body = serde_json::json!({"message_type":"task","title":"batch rollback","content":"persisted","channels":["in_app"],"receiver_ids":["one","two"]});
    let send_batch = || {
        client
            .post(format!("{}/send", server.url))
            .bearer_auth(token("a", "sender"))
            .header("Idempotency-Key", "batch-rollback")
            .json(&body)
    };
    assert_eq!(send_batch().send().await.unwrap().status().as_u16(), 503);
    assert_eq!(store.stats("a", "one").unwrap().total, 2);
    conn.execute_batch("DROP TRIGGER reject_second;").unwrap();
    assert_eq!(send_batch().send().await.unwrap().status().as_u16(), 200);
    assert_eq!(store.stats("a", "one").unwrap().total, 3);
    // Active inherited permissions work; cyclic inheritance terminates and inactive parents deny.
    iam.conn.lock().execute_batch("INSERT INTO iam_role (role_id,tenant_id,role_code,role_name,role_type,data_scope,created_at,updated_at) VALUES ('child','a','child','child','custom','self','now','now');
        UPDATE iam_user_role SET role_id='child' WHERE user_id='sender';
        INSERT INTO iam_role_inherit (ri_id,tenant_id,parent_role_id,child_role_id,created_at) VALUES ('i1','a','r','child','now'),('i2','a','child','r','now');").unwrap();
    assert_eq!(send_batch().send().await.unwrap().status().as_u16(), 200);
    iam.conn
        .lock()
        .execute_batch("UPDATE iam_role SET status='disabled' WHERE role_id='r';")
        .unwrap();
    assert_eq!(send_batch().send().await.unwrap().status().as_u16(), 403);
    iam.conn.lock().execute_batch("UPDATE iam_role SET status='active' WHERE role_id='r'; UPDATE iam_permission SET status='disabled' WHERE perm_id='p';").unwrap();
    assert_eq!(send_batch().send().await.unwrap().status().as_u16(), 403);
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

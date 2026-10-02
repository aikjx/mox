//! Real file-backed transactions; no substituted business service.
use axum::{extract::State, http::HeaderMap, Json};
use mox_platform_api::UserInfo;
use mox_platform_gateway_svc::{
    alliance::experts_common::TenantId,
    auth::ApiAuth,
    message_center::{
        api::{send_message_handler, MessageCenterState},
        repository::StoreError,
        MessageStatus, SendMessageRequest,
    },
};
use std::sync::Arc;

fn identity(tenant: &str, user: &str) -> UserInfo {
    UserInfo {
        id: user.into(),
        tenant_id: tenant.into(),
        username: user.into(),
        enabled: true,
        roles: vec!["user".into()],
        email: String::new(),
        created_at: String::new(),
    }
}
fn request(user: &str, content: &str) -> SendMessageRequest {
    serde_json::from_value(
        serde_json::json!({"message_type":"approval_pending", "title":"Approval", "content":content,
        "channels":["in_app"], "receiver_ids":[user], "extra_data":{"nested":{"z":1,"a":2}}}),
    )
    .unwrap()
}
async fn send(
    state: Arc<MessageCenterState>,
    tenant: &str,
    user: &str,
    key: &str,
    content: &str,
) -> axum::response::Response {
    let mut headers = HeaderMap::new();
    headers.insert("idempotency-key", key.parse().unwrap());
    send_message_handler(
        State(state),
        TenantId(tenant.into()),
        ApiAuth(identity(tenant, user)),
        headers,
        Json(request(user, content)),
    )
    .await
}
async fn id(response: axum::response::Response) -> String {
    assert_eq!(response.status().as_u16(), 200);
    let bytes = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
    serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["data"]["message_id"]
        .as_str()
        .unwrap()
        .into()
}

#[tokio::test]
async fn restart_and_multiple_instances_share_committed_inbox_and_receipts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inbox.db");
    let first = Arc::new(MessageCenterState::with_db_path(path.clone()));
    let second = MessageCenterState::with_db_path(path.clone());
    let message_id = id(send(first.clone(), "a", "u", "request-1", "actual").await).await;
    drop(first);
    assert_eq!(second.repository.get("a", "u", &message_id).unwrap().unwrap().content, "actual");
    assert!(second.repository.get("b", "u", &message_id).unwrap().is_none());
    assert!(second.repository.get("a", "other", &message_id).unwrap().is_none());
    assert!(!second.repository.mark_read("b", "u", &message_id).unwrap());
    assert!(second.repository.mark_read("a", "u", &message_id).unwrap());
    let restarted = MessageCenterState::with_db_path(path);
    assert!(matches!(
        restarted.repository.get("a", "u", &message_id).unwrap().unwrap().status,
        MessageStatus::Read
    ));
    let receipt = restarted.repository.receipt("a", "u", &message_id).unwrap().unwrap();
    assert!(matches!(receipt.status, MessageStatus::Read));
    assert!(receipt.read_at.is_some());
    restarted.repository.mark_read("a", "u", &message_id).unwrap();
    assert_eq!(
        restarted.repository.receipt("a", "u", &message_id).unwrap().unwrap().read_at,
        receipt.read_at
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_retries_commit_once_and_conflicting_payload_is_rejected() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    let dir = tempfile::tempdir().unwrap();
    let mut tasks = Vec::new();
    for _ in 0..12 {
        let state = Arc::new(MessageCenterState::with_db_path(dir.path().join("inbox.db")));
        tasks.push(tokio::spawn(async move {
            id(send(state, "a", "u", "same-key", "actual").await).await
        }));
    }
    let mut ids = Vec::new();
    for task in tasks {
        ids.push(task.await.unwrap());
    }
    assert!(ids.iter().all(|value| value == &ids[0]));
    let state = Arc::new(MessageCenterState::with_db_path(dir.path().join("inbox.db")));
    assert_eq!(state.repository.stats("a", "u").unwrap().total, 1);
    assert_eq!(send(state.clone(), "a", "u", "same-key", "changed").await.status().as_u16(), 409);
    // Identical keys belong to distinct trusted tenant/user scopes.
    id(send(state.clone(), "b", "u", "same-key", "actual").await).await;
    id(send(state.clone(), "a", "other", "same-key", "actual").await).await;
    assert_eq!(state.repository.stats("a", "u").unwrap().total, 1);
}

#[tokio::test]
async fn receipt_write_failure_rolls_back_message_and_retry_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inbox.db");
    let state = Arc::new(MessageCenterState::with_db_path(path.clone()));
    state.repository.stats("a", "u").unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_receipt BEFORE INSERT ON inbox_receipts BEGIN SELECT RAISE(ABORT, 'receipt rejected'); END;").unwrap();
    assert_eq!(send(state.clone(), "a", "u", "retry", "actual").await.status().as_u16(), 503);
    assert_eq!(state.repository.stats("a", "u").unwrap().total, 0);
    conn.execute_batch("DROP TRIGGER reject_receipt;").unwrap();
    id(send(state.clone(), "a", "u", "retry", "actual").await).await;
    assert_eq!(state.repository.stats("a", "u").unwrap().total, 1);
}

#[tokio::test]
async fn unreadable_storage_is_not_acknowledged_or_replaced_by_memory() {
    let dir = tempfile::tempdir().unwrap();
    // A directory cannot be opened as a SQLite database.
    let state = Arc::new(MessageCenterState::with_db_path(dir.path().to_path_buf()));
    assert_eq!(send(state.clone(), "a", "u", "request", "actual").await.status().as_u16(), 503);
    assert!(matches!(state.repository.stats("a", "u"), Err(StoreError::Unavailable(_))));
}

#[tokio::test]
async fn pagination_and_snake_case_filters_use_persisted_scope() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(MessageCenterState::with_db_path(dir.path().join("inbox.db")));
    for index in 0..3 {
        id(send(state.clone(), "a", "u", &format!("key-{index}"), "actual").await).await;
    }
    let (first, total) = state
        .repository
        .list("a", "u", 1, 0, Some("approval_pending"), Some("sent"))
        .unwrap();
    let (second, _) = state
        .repository
        .list("a", "u", 1, 1, Some("approval_pending"), Some("sent"))
        .unwrap();
    assert_eq!(total, 3);
    assert_eq!(first.len(), 1);
    assert_ne!(first[0].message_id, second[0].message_id);
    assert_eq!(state.repository.list("b", "u", 50, 0, None, None).unwrap().1, 0);
}

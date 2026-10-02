use axum::{
    extract::{Path, State},
    Json,
};
use mox_platform_api::UserInfo;
use mox_platform_gateway_svc::{
    alliance::experts_common::TenantId,
    auth::ApiAuth,
    integration::{
        api::{create_connector_handler, trigger_sync_handler, IntegrationState},
        model::{CreateConnectorRequest, TriggerSyncRequest},
    },
    message_center::{
        api::{send_message_handler, MessageCenterState},
        SendMessageRequest,
    },
};
use std::sync::Arc;

fn identity() -> UserInfo {
    UserInfo {
        id: "inbox-owner".into(),
        tenant_id: "tenant-a".into(),
        username: "owner".into(),
        enabled: true,
        roles: vec!["user".into()],
        email: String::new(),
        created_at: String::new(),
    }
}

#[tokio::test]
async fn unsupported_message_channels_never_create_sent_records() {
    let state = Arc::new(MessageCenterState::new());
    let request: SendMessageRequest = serde_json::from_value(serde_json::json!({
        "message_type":"system", "title":"test", "content":"test",
        "channels":["email"], "receiver_emails":["recipient@example.com"]
    }))
    .unwrap();
    let response = send_message_handler(
        State(state.clone()),
        TenantId("tenant-a".into()),
        ApiAuth(identity()),
        Json(request),
    )
    .await;
    assert_eq!(response.status().as_u16(), 501);
    assert!(state.messages.read().await.is_empty());
    assert!(state.send_records.read().await.is_empty());
}

#[tokio::test]
async fn actual_self_inbox_records_verified_sender_and_receiver() {
    let state = Arc::new(MessageCenterState::new());
    let request = serde_json::from_value(serde_json::json!({
        "message_type":"system", "title":"test", "content":"test",
        "channels":["in_app"], "receiver_ids":["inbox-owner"]
    }))
    .unwrap();
    let response = send_message_handler(
        State(state.clone()),
        TenantId("tenant-a".into()),
        ApiAuth(identity()),
        Json(request),
    )
    .await;
    assert_eq!(response.status().as_u16(), 200);
    let messages = state.messages.read().await;
    let message = messages.values().next().unwrap();
    assert_eq!(message.tenant_id, "tenant-a");
    assert_eq!(message.sender_id, "inbox-owner");
    assert_eq!(state.send_records.read().await[0].receiver_id, "inbox-owner");
}

#[tokio::test]
async fn missing_sync_transaction_never_creates_a_fake_task() {
    let state = Arc::new(IntegrationState::new());
    let create: CreateConnectorRequest = serde_json::from_value(serde_json::json!({
        "name":"actual-source", "connector_type":"generic_rest", "connection":{}, "auth":{}
    }))
    .unwrap();
    let _ = create_connector_handler(State(state.clone()), Json(create)).await;
    let id = state.connectors.read().await.keys().next().unwrap().clone();
    let request: TriggerSyncRequest =
        serde_json::from_value(serde_json::json!({"entity":"users"})).unwrap();
    let response = trigger_sync_handler(State(state.clone()), Path(id), Json(request)).await;
    assert_eq!(response.status().as_u16(), 501);
    assert!(state.sync_tasks.read().await.is_empty());
}

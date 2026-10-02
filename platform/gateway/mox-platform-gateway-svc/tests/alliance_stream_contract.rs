use std::sync::Arc;

use axum::extract::State;
use futures::StreamExt;
use mox_platform_gateway_svc::alliance::{
    experts_common::{ExpertsSharedState, TenantId},
    experts_events::{AllianceEvent, AllianceEventKind, EventBus},
    experts_streams::alliance_events_stream,
};

// Real broadcast and production SSE body. No service replacement or network mock.
#[tokio::test]
async fn lag_requires_refresh_without_disclosing_other_tenant_counts() {
    let directory = tempfile::tempdir().unwrap();
    std::env::set_var("MOX_EXPERTS_DB_PATH", directory.path().join("events.db"));
    let mut state = ExpertsSharedState::new();
    state.events = Arc::new(EventBus::new(8));
    let state = Arc::new(state);
    let response = alliance_events_stream(State(state.clone()), TenantId("mine".into())).await;
    for index in 0..32 {
        state.events.emit(AllianceEvent::new(
            AllianceEventKind::ExpertDisabled { expert_id: format!("other-{index}") },
            "other",
            "contract-test",
        ));
    }
    let expected = AllianceEvent::new(
        AllianceEventKind::ExpertDisabled { expert_id: "mine-expert".into() },
        "mine",
        "contract-test",
    );
    state.events.emit(expected.clone());
    let mut body = response.into_body().into_data_stream();
    let gap = tokio::time::timeout(std::time::Duration::from_secs(2), body.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let gap = std::str::from_utf8(&gap).unwrap();
    assert!(gap.contains("event: StreamGap"));
    assert!(gap.contains(r#"{"reason":"lagged","action":"refresh"}"#));
    assert!(!gap.contains("other"));
    assert!(!gap.contains("count"));
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), body.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let event = std::str::from_utf8(&event).unwrap();
    assert!(event.contains(&format!("id: {}", expected.id)));
    assert!(event.contains("mine-expert"));
    assert!(!event.contains("other-"));
}

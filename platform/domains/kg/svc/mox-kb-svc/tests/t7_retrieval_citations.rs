use axum::{body::Body, http::Request};
use mox_kb_svc::{handlers::build_kb_router_with_state, KbState};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
async fn search(state: &KbState, body: Value) -> (u16, Value) {
    let req = Request::builder()
        .method("POST")
        .uri("/kb/search")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = build_kb_router_with_state(Arc::new(state.clone())).oneshot(req).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn unicode_snippet_reconstructs_exactly_from_pinned_version() {
    let dir = tempfile::tempdir().unwrap();
    let state = KbState::with_data_dir(dir.path().into());
    let doc = state.docs.create("title", "İ中🙂查询 source", None).await.unwrap();
    let (status, result) = search(&state, json!({"query":"查询"})).await;
    assert_eq!(status, 200);
    let hit = &result["data"]["results"][0];
    let citation = &hit["citation"];
    assert_eq!(citation["document_id"], doc.id);
    assert_eq!(citation["version"], "v1");
    assert_eq!(citation["field"], "content");
    let start = citation["start_char"].as_u64().unwrap() as usize;
    let end = citation["end_char"].as_u64().unwrap() as usize;
    let reconstructed: String = doc.content.chars().skip(start).take(end - start).collect();
    assert_eq!(hit["snippet"], reconstructed);
    state.docs.update(&doc.id, &json!({"content":"new content"})).await.unwrap();
    let changed = state.docs.get(&doc.id).await.unwrap();
    let snapshot = changed.versions.iter().find(|v| v.version == "v1").unwrap();
    let source: String = snapshot.content.chars().skip(start).take(end - start).collect();
    assert_eq!(hit["snippet"], source);
}
#[tokio::test]
async fn tag_hits_use_versioned_title_instead_of_unversioned_summary() {
    let dir = tempfile::tempdir().unwrap();
    let state = KbState::with_data_dir(dir.path().into());
    let mut doc = state.docs.create("source title", "body", None).await.unwrap();
    doc.tags = vec!["topic".into()];
    doc.summary = "unversioned generated summary".into();
    state.docs.save(&doc).await.unwrap();
    let (_, result) = search(&state, json!({"query":"topic"})).await;
    let hit = &result["data"]["results"][0];
    assert_eq!(hit["snippet"], "source title");
    assert_eq!(hit["citation"]["field"], "title");
}
#[tokio::test]
async fn search_rejects_blank_queries_and_unbounded_limits() {
    let dir = tempfile::tempdir().unwrap();
    let state = KbState::with_data_dir(dir.path().into());
    for body in [
        json!({"query":"  "}),
        json!({"query":"q","limit":0}),
        json!({"query":"q","limit":101}),
        json!({"query":"q".repeat(513)}),
    ] {
        assert_eq!(search(&state, body).await.0, 400);
    }
}

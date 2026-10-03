use axum::{body::Body, http::Request};
use mox_kb_svc::{
    access::KnowledgeAccess, handlers::build_kb_router_with_state, model::KbEntity, KbState,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn scope(root: &KbState, tenant: &str, user: &str) -> KbState {
    root.scoped(KnowledgeAccess {
        tenant_id: tenant.into(),
        owner_id: user.into(),
        administrator: false,
        readonly: false,
    })
}
async fn call(state: &KbState, method: &str, uri: &str, body: Value) -> (u16, Value) {
    let response = build_kb_router_with_state(Arc::new(state.clone()))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({"message":String::from_utf8_lossy(&bytes)})),
    )
}

#[tokio::test]
async fn entity_search_and_associations_use_current_acl_revision_and_persist() {
    let dir = tempfile::tempdir().unwrap();
    let root = KbState::with_data_dir(dir.path().into());
    let alice = scope(&root, "a", "alice");
    let bob = scope(&root, "a", "bob");
    let foreign = scope(&root, "b", "alice");
    let mut source = alice.docs.create("source", "Rust", None).await.unwrap();
    source.entities.push(KbEntity {
        id: "rust".into(),
        name: "Rust".into(),
        entity_type: "tech".into(),
        frequency: 1,
        snippet: "Rust".into(),
    });
    alice.docs.save(&source).await.unwrap();
    let target = bob.docs.create("target", "notes", None).await.unwrap();
    let uri = format!("/kb/documents/{}/entities", target.id);
    let search = "/kb/entities/search?q=Rust&type=tech&limit=1";
    assert_eq!(
        call(&alice, "GET", search, Value::Null).await.1["data"][0]["source_doc_id"],
        source.id
    );
    assert_eq!(call(&bob, "GET", search, Value::Null).await.1["data"], json!([]));
    assert_eq!(call(&foreign, "GET", search, Value::Null).await.1["data"], json!([]));
    let request = json!({"entity_id":"rust", "source_doc_id":source.id, "source_version":"v1", "source_acl_revision":0, "expected_current_version":"v1", "expected_acl_revision":0, "expected_links_revision":0});
    assert_eq!(call(&bob, "POST", &uri, request.clone()).await.0, 404);
    alice.docs.change_reader(&source.id, "bob", true, 0).await.unwrap();
    assert_eq!(call(&bob, "POST", &uri, request.clone()).await.0, 409);
    let mut request = request;
    request["source_acl_revision"] = json!(1);
    assert_eq!(call(&bob, "POST", &uri, request.clone()).await.0, 200);
    assert_eq!(call(&bob, "POST", &uri, request.clone()).await.0, 409);
    assert_eq!(call(&foreign, "GET", &uri, Value::Null).await.0, 404);
    let reopened = KbState::with_data_dir(dir.path().into());
    let restored = scope(&reopened, "a", "bob");
    let (_, result) = call(&restored, "GET", &uri, Value::Null).await;
    assert_eq!(result["data"]["links_revision"], 1);
    assert_eq!(result["data"]["linked_entities"][0]["name"], "Rust");
    // Revoke through the same state; no stale permission cache or snapshot exposure.
    scope(&reopened, "a", "alice")
        .docs
        .change_reader(&source.id, "bob", false, 1)
        .await
        .unwrap();
    assert_eq!(
        call(&restored, "GET", &uri, Value::Null).await.1["data"]["linked_entities"],
        json!([])
    );
    // An owner may remove a now-invisible reference without accessing its source.
    request["expected_links_revision"] = json!(1);
    assert_eq!(call(&restored, "DELETE", &uri, request.clone()).await.0, 200);
    assert_eq!(call(&restored, "GET", &uri, Value::Null).await.1["data"]["links_revision"], 2);
    assert_eq!(call(&restored, "POST", &uri, json!({"entity_id":"rust"})).await.0, 422);
}

#[tokio::test]
async fn storage_failure_and_concurrent_writes_cannot_report_lost_success() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let root = KbState::with_data_dir(dir.path().into());
    let alice = scope(&root, "a", "alice");
    let mut source = alice.docs.create("source", "Rust", None).await.unwrap();
    mox_kb_svc::analyze::KbAnalyzer.analyze(&mut source).await.unwrap();
    alice.docs.save(&source).await.unwrap();
    let entity = &source.entities[0];
    let target = alice.docs.create("target", "notes", None).await.unwrap();
    let uri = format!("/kb/documents/{}/entities", target.id);
    let request = json!({"entity_id":entity.id,"source_doc_id":source.id,"source_version":"v1","source_acl_revision":0,
        "expected_current_version":"v1","expected_acl_revision":0,"expected_links_revision":0});
    // Block exactly the association metadata destination, leaving source/target reads usable.
    let hash =
        format!("{:x}", Sha256::digest(format!("kb/entity-links/{}.json", target.id).as_bytes()));
    let path = dir.path().join("objects").join(&hash[..2]).join(format!("{hash}.obj"));
    std::fs::create_dir_all(&path).unwrap();
    assert_eq!(call(&alice, "POST", &uri, request.clone()).await.0, 503);
    std::fs::remove_dir(&path).unwrap();
    assert_eq!(call(&alice, "GET", &uri, Value::Null).await.1["data"]["links_revision"], 0);
    let (first, second) = tokio::join!(
        call(&alice, "POST", &uri, request.clone()),
        call(&alice, "POST", &uri, request.clone())
    );
    let mut statuses = [first.0, second.0];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    let reader = KnowledgeAccess {
        tenant_id: "a".into(),
        owner_id: "alice".into(),
        administrator: false,
        readonly: true,
    };
    let readonly = root.scoped(reader.clone());
    assert_eq!(call(&readonly, "POST", &uri, request.clone()).await.0, 403);
    // Current source content version invalidates both old search tokens and old references.
    alice
        .docs
        .update(&source.id, &json!({"content":"changed", "expected_current_version":"v1"}))
        .await
        .unwrap();
    request_assert_stale(&alice, &uri, request).await;
}
async fn request_assert_stale(state: &KbState, uri: &str, mut request: Value) {
    request["expected_links_revision"] = json!(1);
    assert_eq!(call(state, "POST", uri, request).await.0, 409);
    assert_eq!(call(state, "GET", uri, Value::Null).await.1["data"]["linked_entities"], json!([]));
    assert_eq!(call(state, "GET", "/kb/entities/search?limit=101", Value::Null).await.0, 400);
}

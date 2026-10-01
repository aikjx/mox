use axum::{body::Body, http::Request};
use mox_kb_svc::{access::KnowledgeAccess, handlers::build_kb_router_with_state, KbState};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
fn access(tenant: &str, owner: &str, admin: bool) -> KnowledgeAccess {
    KnowledgeAccess {
        tenant_id: tenant.into(),
        owner_id: owner.into(),
        administrator: admin,
        readonly: false,
    }
}
async fn call(state: &KbState, method: &str, uri: &str, body: Value) -> (u16, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = build_kb_router_with_state(Arc::new(state.clone())).oneshot(req).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn tenant_and_owner_filter_every_resource_outlet_and_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let root = KbState::with_data_dir(dir.path().into());
    let alice = root.scoped(access("tenant-a", "alice", false));
    let bob = root.scoped(access("tenant-a", "bob", false));
    let foreign = root.scoped(access("tenant-b", "alice", true));
    let admin = root.scoped(access("tenant-a", "admin", true));
    let legacy = root.docs.create("legacy secret", "secret", None).await.unwrap();
    let (_, created) = call(&alice,"POST","/kb/documents",json!({"title":"Alice secret","content":"Rust secret","tags":["private-a"],"tenant_id":"tenant-b","owner_id":"bob"})).await;
    let id = created["data"]["id"].as_str().unwrap();
    let doc = alice.docs.get(id).await.unwrap();
    assert_eq!(doc.access.as_ref().unwrap().tenant_id, "tenant-a");
    assert_eq!(doc.access.as_ref().unwrap().owner_id, "alice");
    for hidden in [&bob, &foreign] {
        for suffix in ["", "/versions", "/entities", "/history"] {
            assert_eq!(
                call(hidden, "GET", &format!("/kb/documents/{id}{suffix}"), Value::Null).await.0,
                404
            );
        }
        for (method, suffix, body) in [
            ("PUT", "", json!({"title":"stolen"})),
            ("DELETE", "", Value::Null),
            ("POST", "/graph-link", Value::Null),
        ] {
            assert_eq!(
                call(hidden, method, &format!("/kb/documents/{id}{suffix}"), body).await.0,
                404
            );
        }
        let (_, list) = call(hidden, "GET", "/kb/documents", Value::Null).await;
        assert_eq!(list["data"]["total"], 0);
        let (_, tags) = call(hidden, "GET", "/kb/tags", Value::Null).await;
        assert_eq!(tags["data"], json!([]));
    }
    assert_eq!(call(&admin, "GET", &format!("/kb/documents/{id}"), Value::Null).await.0, 200);
    assert_eq!(
        call(&admin, "GET", &format!("/kb/documents/{}", legacy.id), Value::Null)
            .await
            .0,
        404
    );
    assert_eq!(
        call(&alice, "POST", &format!("/kb/documents/{id}/graph-link"), Value::Null)
            .await
            .0,
        200
    );
    let (_, result) =
        call(&foreign, "POST", "/kb/search", json!({"query":"secret","limit":1})).await;
    assert_eq!(result["data"]["total"], 0);
    assert_eq!(result["data"]["graph_hits"], json!([]));
    let (_, stats) = call(&foreign, "GET", "/kb/stats", Value::Null).await;
    assert_eq!(stats["data"]["graph_nodes"], 0);
    let reopened = KbState::with_data_dir(dir.path().into());
    let owner = reopened.scoped(access("tenant-a", "alice", false));
    let (_, stats) = call(&owner, "GET", "/kb/stats", Value::Null).await;
    assert!(stats["data"]["graph_nodes"].as_u64().unwrap() > 0);
    assert!(reopened.scoped(access("tenant-b", "alice", true)).docs.get(id).await.is_err());
}
#[tokio::test]
async fn readonly_cannot_mutate_or_create_and_acl_cannot_be_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let root = KbState::with_data_dir(dir.path().into());
    let owner = root.scoped(access("a", "owner", false));
    let mut doc = owner.docs.create("title", "content", None).await.unwrap();
    let mut reader = access("a", "owner", false);
    reader.readonly = true;
    let readonly = root.scoped(reader);
    assert!(readonly.docs.get(&doc.id).await.is_ok());
    assert!(readonly.docs.create("bad", "", None).await.is_err());
    assert!(readonly.docs.update(&doc.id, &json!({"title":"bad"})).await.is_err());
    assert!(readonly.docs.delete(&doc.id).await.is_err());
    assert!(readonly.docs.save(&doc).await.is_err());
    let graph_before = root.graph.node_count();
    let request = Request::builder()
        .method("POST")
        .uri(format!("/kb/documents/{}/graph-link", doc.id))
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        build_kb_router_with_state(Arc::new(readonly.clone()))
            .oneshot(request)
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(root.graph.node_count(), graph_before);
    doc.access = Some(access("b", "owner", false));
    assert!(owner.docs.save(&doc).await.is_err());
    assert_eq!(owner.docs.get(&doc.id).await.unwrap().title, "title");
}

#[tokio::test]
async fn shared_reader_is_readonly_and_revocation_removes_all_outlets_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let root = KbState::with_data_dir(dir.path().into());
    let owner = root.scoped(access("a", "owner", false));
    let reader = root.scoped(access("a", "reader", false));
    let foreign = root.scoped(access("b", "reader", false));
    let mut doc = owner.docs.create("Rust secret", "Rust architecture", None).await.unwrap();
    doc.tags = vec!["shared-tag".into()];
    owner.docs.save(&doc).await.unwrap();
    let path = format!("/kb/documents/{}", doc.id);
    assert_eq!(
        call(&owner, "POST", &format!("{path}/versions"), json!({"note":"snapshot"}))
            .await
            .0,
        200
    );
    assert_eq!(call(&owner, "POST", &format!("{path}/graph-link"), Value::Null).await.0, 200);
    let stale = owner.docs.get(&doc.id).await.unwrap();
    assert_eq!(call(&reader, "GET", &path, Value::Null).await.0, 404);
    let grant = json!({"user_id":"reader","expected_acl_revision":0});
    let (status, result) = call(&owner, "POST", &format!("{path}/shares"), grant.clone()).await;
    assert_eq!(status, 200);
    assert_eq!(result["data"]["acl_revision"], 1);
    assert_eq!(call(&owner, "POST", &format!("{path}/shares"), grant).await.0, 409);
    assert_eq!(
        call(
            &owner,
            "POST",
            &format!("{path}/shares"),
            json!({"user_id":"reader","expected_acl_revision":1})
        )
        .await
        .1["data"]["acl_revision"],
        1
    );
    assert!(owner.docs.save(&stale).await.is_err());
    for suffix in ["", "/versions", "/entities", "/history"] {
        assert_eq!(call(&reader, "GET", &format!("{path}{suffix}"), Value::Null).await.0, 200);
        assert_eq!(call(&foreign, "GET", &format!("{path}{suffix}"), Value::Null).await.0, 404);
    }
    let (_, hits) = call(&reader, "POST", "/kb/search", json!({"query":"Rust"})).await;
    assert_eq!(hits["data"]["total"], 1);
    assert!(!hits["data"]["graph_hits"].as_array().unwrap().is_empty());
    let reader_tags = call(&reader, "GET", "/kb/tags", Value::Null).await.1["data"].clone();
    let owner_tags = call(&owner, "GET", "/kb/tags", Value::Null).await.1["data"].clone();
    assert!(!reader_tags.as_array().unwrap().is_empty());
    assert_eq!(reader_tags, owner_tags);
    let before = root.graph.snapshot();
    for (method, suffix, body) in [
        ("POST", "/graph-link", Value::Null),
        ("DELETE", "/graph-link", Value::Null),
        ("POST", "/analyze", Value::Null),
        ("POST", "/versions", json!({"note":"bad"})),
        ("POST", "/shares", json!({"user_id":"other","expected_acl_revision":1})),
        ("PUT", "", json!({"title":"bad"})),
        ("DELETE", "", Value::Null),
    ] {
        assert_eq!(call(&reader, method, &format!("{path}{suffix}"), body).await.0, 404);
    }
    assert_eq!(root.graph.node_count(), before.node_count);
    assert_eq!(root.graph.edge_count(), before.edge_count);
    let (_, batch) = call(&reader, "POST", "/kb/batch-analyze", json!({"ids":[doc.id]})).await;
    assert_eq!(batch["data"]["analyzed"], 0);
    assert_eq!(owner.docs.get(&doc.id).await.unwrap().status, "linked");
    let reopened = KbState::with_data_dir(dir.path().into());
    assert!(reopened.scoped(access("a", "reader", false)).docs.get(&doc.id).await.is_ok());
    assert_eq!(
        call(
            &owner,
            "DELETE",
            &format!("{path}/shares/reader?expected_acl_revision=0"),
            Value::Null
        )
        .await
        .0,
        409
    );
    let (status, result) = call(
        &owner,
        "DELETE",
        &format!("{path}/shares/reader?expected_acl_revision=1"),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(result["data"]["acl_revision"], 2);
    assert_eq!(call(&reader, "GET", &path, Value::Null).await.0, 404);
    let (_, hits) = call(&reader, "POST", "/kb/search", json!({"query":"Rust"})).await;
    assert_eq!(hits["data"]["total"], 0);
    assert_eq!(hits["data"]["graph_hits"], json!([]));
    assert_eq!(call(&reader, "GET", "/kb/documents", Value::Null).await.1["data"]["total"], 0);
    assert_eq!(call(&reader, "GET", "/kb/stats", Value::Null).await.1["data"]["graph_nodes"], 0);
    let reopened = KbState::with_data_dir(dir.path().into());
    assert!(reopened.scoped(access("a", "reader", false)).docs.get(&doc.id).await.is_err());
    assert_eq!(call(&reader, "GET", "/kb/tags", Value::Null).await.1["data"], json!([]));
    assert_eq!(
        call(&owner, "POST", &format!("{path}/versions/revert"), json!({"version":"v1"}))
            .await
            .0,
        200
    );
    let current = owner.docs.get(&doc.id).await.unwrap();
    assert!(current.readers.is_empty());
    assert_eq!(current.acl_revision, 2);
    assert!(reader.docs.get(&doc.id).await.is_err());
    assert!(owner.docs.get(&doc.id).await.is_ok());
}
#[tokio::test]
async fn concurrent_acl_updates_do_not_lose_grants() {
    let dir = tempfile::tempdir().unwrap();
    let root = KbState::with_data_dir(dir.path().into());
    let owner = root.scoped(access("a", "owner", false));
    let doc = owner.docs.create("title", "content", None).await.unwrap();
    let (first, second) = tokio::join!(
        owner.docs.change_reader(&doc.id, "one", true, 0),
        owner.docs.change_reader(&doc.id, "two", true, 0)
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert_eq!(owner.docs.get(&doc.id).await.unwrap().readers.len(), 1);
    let missing = if first.is_ok() { "two" } else { "one" };
    owner.docs.change_reader(&doc.id, missing, true, 1).await.unwrap();
    assert_eq!(owner.docs.get(&doc.id).await.unwrap().readers, vec!["one", "two"]);
    assert!(owner.docs.change_reader(&doc.id, " bad ", true, 2).await.is_err());
}

#[tokio::test]
async fn conditional_edits_archive_once_and_reject_lost_updates() {
    let dir = tempfile::tempdir().unwrap();
    let root = KbState::with_data_dir(dir.path().into());
    let owner = root.scoped(access("a", "owner", false));
    let doc = owner.docs.create("title", "old", None).await.unwrap();
    let path = format!("/kb/documents/{}", doc.id);
    let (status, result) = call(
        &owner,
        "PUT",
        &path,
        json!({"content":"new","expected_current_version":"v1","version_note":"reason"}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(result["data"]["document"]["current_version"], "v2");
    let updated = owner.docs.get(&doc.id).await.unwrap();
    assert_eq!(updated.versions.len(), 1);
    assert_eq!(updated.versions[0].content, "old");
    assert_eq!(updated.versions[0].note, "reason");
    assert_eq!(
        call(&owner, "PUT", &path, json!({"content":"lost","expected_current_version":"v1"}))
            .await
            .0,
        409
    );
    assert_eq!(owner.docs.get(&doc.id).await.unwrap(), updated);
}
#[tokio::test]
async fn late_processing_cannot_overwrite_changed_source_or_new_acl() {
    use mox_kb_svc::document::DocumentWriteError;
    let dir = tempfile::tempdir().unwrap();
    let root = KbState::with_data_dir(dir.path().into());
    let owner = root.scoped(access("a", "owner", false));
    let original = owner.docs.create("title", "old", None).await.unwrap();
    let mut processed = original.clone();
    processed.summary = "late summary".into();
    owner.docs.update(&original.id, &json!({"content":"current"})).await.unwrap();
    assert!(matches!(
        owner.docs.save_if_unchanged(&original, &processed).await,
        Err(DocumentWriteError::Conflict)
    ));
    let current = owner.docs.get(&original.id).await.unwrap();
    let mut processed = current.clone();
    processed.summary = "late summary".into();
    owner.docs.change_reader(&original.id, "reader", true, 0).await.unwrap();
    assert!(matches!(
        owner.docs.save_if_unchanged(&current, &processed).await,
        Err(DocumentWriteError::Conflict)
    ));
    let latest = owner.docs.get(&original.id).await.unwrap();
    assert_eq!(latest.content, "current");
    assert!(latest.summary.is_empty());
    assert_eq!(latest.readers, vec!["reader"]);
    let mut processed = latest.clone();
    processed.summary = "current summary".into();
    owner.docs.save_if_unchanged(&latest, &processed).await.unwrap();
    assert_eq!(owner.docs.get(&original.id).await.unwrap().summary, "current summary");
}

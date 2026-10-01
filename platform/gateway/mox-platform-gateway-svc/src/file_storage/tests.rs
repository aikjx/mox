use super::api::{build_file_storage_router, FileStorageState};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use mox_platform_api::UserInfo;
use std::sync::Arc;
use tower::ServiceExt;
fn user(tenant: &str) -> UserInfo {
    UserInfo {
        id: "user-1".into(),
        username: "u".into(),
        email: String::new(),
        tenant_id: tenant.into(),
        roles: vec!["normal_user".into()],
        enabled: true,
        created_at: String::new(),
    }
}
fn router(state: Arc<FileStorageState>) -> Router {
    build_file_storage_router::<Arc<FileStorageState>>().with_state(state)
}
fn request(method: &str, path: &str, actor: UserInfo) -> Request<Body> {
    let mut req = Request::builder().method(method).uri(path).body(Body::empty()).unwrap();
    req.extensions_mut().insert(actor);
    req
}
async fn json_response(app: Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
async fn upload(
    app: Router,
    actor: UserInfo,
    name: &str,
    content: &str,
) -> (StatusCode, serde_json::Value) {
    let body=format!("--BOUNDARY\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: text/plain\r\n\r\n{content}\r\n--BOUNDARY--\r\n");
    let mut req = Request::builder()
        .method("POST")
        .uri("/upload")
        .header("content-type", "multipart/form-data; boundary=BOUNDARY")
        .body(Body::from(body))
        .unwrap();
    req.extensions_mut().insert(actor);
    json_response(app, req).await
}
#[tokio::test]
async fn file_storage_requires_identity() {
    let temp = tempfile::tempdir().unwrap();
    let app = router(Arc::new(FileStorageState::open(temp.path()).unwrap()));
    for path in ["/", "/stats", "/categories", "/storage-types", "/missing", "/missing/download"] {
        assert_eq!(
            app.clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
#[tokio::test]
async fn production_store_does_not_seed_sample_files() {
    let temp = tempfile::tempdir().unwrap();
    let app = router(Arc::new(FileStorageState::open(temp.path()).unwrap()));
    let (_, body) = json_response(app, request("GET", "/", user("tenant"))).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
}
#[tokio::test]
async fn file_lifecycle_survives_restart_and_checks_content() {
    let temp = tempfile::tempdir().unwrap();
    let app = router(Arc::new(FileStorageState::open(temp.path()).unwrap()));
    let (status, body) = upload(app.clone(), user("tenant"), "合同.txt", "real content").await;
    assert_eq!(status, StatusCode::OK);
    let file = &body["data"]["files"][0];
    let id = file["file_id"].as_str().unwrap();
    assert_eq!(file["tenant_id"], "tenant");
    assert_eq!(file["uploaded_by"], "user-1");
    assert!(file["md5"].is_null());
    assert_eq!(file["sha256"].as_str().unwrap().len(), 64);
    let response = app
        .clone()
        .oneshot(request("GET", &format!("/{id}/download"), user("tenant")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .contains("%E5%90%88"));
    assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap().as_ref(), b"real content");
    assert_eq!(
        json_response(app.clone(), request("DELETE", &format!("/{id}"), user("tenant")))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        json_response(app.clone(), request("GET", &format!("/{id}/download"), user("tenant")))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    drop(app);
    let app = router(Arc::new(FileStorageState::open(temp.path()).unwrap()));
    let (_, body) =
        json_response(app.clone(), request("GET", &format!("/{id}"), user("tenant"))).await;
    assert_eq!(body["data"]["status"], "deleted");
    assert_eq!(body["data"]["download_count"], 1);
    assert_eq!(
        json_response(app.clone(), request("POST", &format!("/{id}/restore"), user("tenant")))
            .await
            .0,
        StatusCode::OK
    );
    let blob = temp.path().join(file["file_path"].as_str().unwrap());
    std::fs::write(&blob, b"FAKE content").unwrap();
    assert_eq!(
        json_response(app.clone(), request("GET", &format!("/{id}/download"), user("tenant")))
            .await
            .0,
        StatusCode::CONFLICT
    );
    std::fs::remove_file(&blob).unwrap();
    assert_eq!(
        json_response(app.clone(), request("GET", &format!("/{id}/download"), user("tenant")))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (_, body) = json_response(app, request("GET", &format!("/{id}"), user("tenant"))).await;
    assert_eq!(body["data"]["download_count"], 1);
}
#[tokio::test]
async fn tenant_owner_and_readonly_policy_applies_to_every_resource_operation() {
    let temp = tempfile::tempdir().unwrap();
    let app = router(Arc::new(FileStorageState::open(temp.path()).unwrap()));
    let (_, body) = upload(app.clone(), user("tenant"), "../unsafe.txt", "content").await;
    let file = &body["data"]["files"][0];
    let id = file["file_id"].as_str().unwrap();
    assert!(!file["file_path"].as_str().unwrap().contains(".."));
    let mut other_owner = user("tenant");
    other_owner.id = "user-2".into();
    for actor in [user("other-tenant"), other_owner] {
        for (method, path) in [
            ("GET", format!("/{id}")),
            ("GET", format!("/{id}/download")),
            ("DELETE", format!("/{id}")),
            ("POST", format!("/{id}/restore")),
        ] {
            assert_eq!(
                json_response(app.clone(), request(method, &path, actor.clone())).await.0,
                StatusCode::NOT_FOUND
            );
        }
        let (_, body) = json_response(app.clone(), request("GET", "/", actor.clone())).await;
        assert_eq!(body["data"], serde_json::json!([]));
        let (_, body) = json_response(app.clone(), request("GET", "/stats", actor)).await;
        assert_eq!(body["data"]["total_files"], 0);
    }
    let mut readonly = user("tenant");
    readonly.roles = vec!["readonly_auditor".into()];
    assert_eq!(
        upload(app.clone(), readonly.clone(), "file.txt", "x").await.0,
        StatusCode::FORBIDDEN
    );
    for method in ["DELETE", "POST"] {
        let path = if method == "POST" { format!("/{id}/restore") } else { format!("/{id}") };
        assert_eq!(
            json_response(app.clone(), request(method, &path, readonly.clone())).await.0,
            StatusCode::FORBIDDEN
        );
    }
    let mut admin = user("tenant");
    admin.id = "admin".into();
    admin.roles = vec!["tenant_admin".into()];
    assert_eq!(
        json_response(app.clone(), request("GET", &format!("/{id}"), admin)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        json_response(app.clone(), request("GET", "/?tenant_id=other", user("tenant")))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let mut disabled = user("tenant");
    disabled.enabled = false;
    assert_eq!(json_response(app, request("GET", "/", disabled)).await.0, StatusCode::FORBIDDEN);
}
#[tokio::test]
async fn malformed_batch_does_not_publish_partial_files() {
    let temp = tempfile::tempdir().unwrap();
    let app = router(Arc::new(FileStorageState::open(temp.path()).unwrap()));
    let body="--B\r\nContent-Disposition: form-data; name=\"file\"; filename=\"valid.txt\"\r\n\r\ngood\r\n--B\r\nContent-Disposition: form-data; name=\"file\"; filename=\"bad.txt\"\r\n\r\ntruncated";
    let mut req = Request::builder()
        .method("POST")
        .uri("/upload")
        .header("content-type", "multipart/form-data; boundary=B")
        .body(Body::from(body))
        .unwrap();
    req.extensions_mut().insert(user("tenant"));
    assert_eq!(json_response(app.clone(), req).await.0, StatusCode::BAD_REQUEST);
    let (_, body) = json_response(app, request("GET", "/", user("tenant"))).await;
    assert_eq!(body["data"], serde_json::json!([]));
}

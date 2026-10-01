use super::*;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use tower::ServiceExt;
async fn call(
    app: &Router,
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.map(|value| value.to_string()).unwrap_or_default()))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn document_api_lists_real_data_and_guards_version_updates() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("kb.sqlite3");
    let module = KbModule {
        manager: Arc::new(KbManager::new(Box::new(SqliteKbStore::open(&path).unwrap()))),
    };
    let app = module.routes(&ServerConfig::default()).await;
    let (status, body) = call(
        &app,
        "POST",
        "/api/v1/kb/documents",
        Some(json!({"title":"企业规范","content":"first","author":"tester"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = body["document"]["id"].as_str().unwrap();
    let (_, body) = call(&app, "GET", "/api/v1/kb/documents?page_size=1", None).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["documents"][0]["id"], id);
    let url = format!("/api/v1/kb/documents/{id}");
    let (status, body) = call(
        &app,
        "PUT",
        &url,
        Some(json!({"title":"第二版","content":"second","expected_version":1})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["document"]["version"], 2);
    assert_eq!(
        call(
            &app,
            "PUT",
            &url,
            Some(json!({"title":"stale","content":"stale","expected_version":1}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    drop(app);
    drop(module);
    let module = KbModule {
        manager: Arc::new(KbManager::new(Box::new(SqliteKbStore::open(&path).unwrap()))),
    };
    let app = module.routes(&ServerConfig::default()).await;
    let (_, body) = call(&app, "GET", &format!("{url}/versions"), None).await;
    assert_eq!(body["versions"].as_array().unwrap().len(), 2);
    assert_eq!(body["versions"][0]["content_snapshot"], "first");
    assert_eq!(call(&app, "DELETE", &url, None).await.0, StatusCode::OK);
    assert_eq!(call(&app, "GET", &url, None).await.0, StatusCode::NOT_FOUND);
    let (_, body) = call(&app, "GET", "/api/v1/kb/documents", None).await;
    assert_eq!(body["total"], 0);
}
#[tokio::test]
async fn invalid_input_and_unknown_documents_return_real_http_errors() {
    let temp = tempfile::tempdir().unwrap();
    let module = KbModule {
        manager: Arc::new(KbManager::new(Box::new(
            SqliteKbStore::open(temp.path().join("kb.db")).unwrap(),
        ))),
    };
    let app = module.routes(&ServerConfig::default()).await;
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/kb/documents",
            Some(json!({"title":"","content":"x","author":"tester"}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/kb/documents/missing", None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "DELETE", "/api/v1/kb/documents/missing", None).await.0,
        StatusCode::NOT_FOUND
    );
    assert!(module.ready_checks().await.iter().all(|(_, ready)| *ready));
}

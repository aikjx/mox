// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! `http` feature 的存储 HTTP 面回归：统一 ApiResponse 信封、错误码归一化
//! （400/404）、属性类型往返、邻居投影。纯内存引擎，无需 libclang。

#![cfg(feature = "http")]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use mox_kg_storage_svc::http_api::build_storage_router;
use mox_kg_storage_svc::storage_server::StorageServer;

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    let req = match body {
        Some(j) => builder.body(Body::from(j.to_string())).expect("req"),
        None => builder.body(Body::empty()).expect("req"),
    };
    let resp = app.clone().oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn router() -> axum::Router {
    let srv = StorageServer::start_cluster(16, &[], None).expect("mem cluster");
    build_storage_router(Arc::new(srv))
}

#[tokio::test]
async fn vertex_put_read_delete_roundtrip() {
    let app = router();
    let (s, j) = call(
        &app,
        "POST",
        "/vertex",
        Some(json!({"vid":"u:1","tag":"user","props":{"name":"alice","age":30,"score":4.5,"ok":true}})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["code"], 0);
    assert!(j["data"]["shard"].as_u64().is_some());

    let (s, j) = call(&app, "GET", "/vertex/u%3A1", None).await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["data"]["tag"], "user");
    assert_eq!(j["data"]["props"]["name"], "alice");
    assert_eq!(j["data"]["props"]["age"], 30);
    assert!((j["data"]["props"]["score"].as_f64().unwrap() - 4.5).abs() < 1e-9);
    assert_eq!(j["data"]["props"]["ok"], true);

    let (s, j) = call(&app, "DELETE", "/vertex/u%3A1", None).await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["data"]["removed"], true);

    let (s, _) = call(&app, "GET", "/vertex/u%3A1", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn read_missing_vid_is_404_envelope() {
    let app = router();
    let (s, j) = call(&app, "GET", "/vertex/nope", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(j["code"], 404);
    assert!(j["msg"].as_str().unwrap().contains("nope"));
}

#[tokio::test]
async fn edge_and_neighbors_projection() {
    let app = router();
    for vid in ["a", "b"] {
        let (s, _) = call(
            &app,
            "POST",
            "/vertex",
            Some(json!({"vid":vid,"tag":"node","props":{}})),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
    }
    let (s, j) = call(
        &app,
        "POST",
        "/edge",
        Some(json!({"src":"a","dst":"b","etype":"link","rank":0,"weight":2.5,"props":{"k":"v"}})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{j}");

    let (s, j) = call(&app, "GET", "/neighbors/a?dir=out", None).await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["data"]["count"], 1);
    assert_eq!(j["data"]["neighbors"][0]["neighbor_vid"], "b");
    assert_eq!(j["data"]["neighbors"][0]["etype"], "link");
    assert_eq!(j["data"]["neighbors"][0]["props"]["k"], "v");

    let (s, j) = call(&app, "GET", "/neighbors/a?dir=sideways", None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(j["code"], 400);
}

#[tokio::test]
async fn bytes_prop_hex_roundtrip() {
    let app = router();
    let (s, _) = call(
        &app,
        "POST",
        "/vertex",
        Some(json!({"vid":"bin","tag":"blob","props":{"data":{"$hex":"00ff10"}}})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_s, j) = call(&app, "GET", "/vertex/bin", None).await;
    assert_eq!(j["data"]["props"]["data"]["$hex"], "00ff10");
}

#[tokio::test]
async fn stats_reports_durability_mode() {
    let app = router();
    let (s, j) = call(&app, "GET", "/stats", None).await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["data"]["shard_count"], 16);
    assert_eq!(j["data"]["engine_rocksdb_persist"], cfg!(feature = "persist-rocksdb"));
    assert_eq!(j["data"]["wal_sync_before_ack"], true); // 默认 durable
}

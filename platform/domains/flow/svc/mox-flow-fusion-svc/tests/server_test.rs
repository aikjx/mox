// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 企业级 REST 服务层集成测试
//!
//! 采用 `tower::ServiceExt::oneshot` 直接驱动 `build_router` 返回的 `Router`，
//! 无需真实网络即可验证全部端点的鉴权、溯源与守恒闸门行为。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use mox_flow_fusion_svc::config::Config;
use mox_flow_fusion_svc::server::{build_router, new_state};
use tower::ServiceExt;

fn app_with(cfg: Config) -> axum::Router {
    let state = new_state(cfg);
    build_router(state)
}

fn app() -> axum::Router {
    app_with(Config::default())
}

#[tokio::test]
async fn health_is_ok() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn version_is_ok() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/api/version")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn synthesize_runs_full_pipeline() {
    let body = serde_json::json!({
        "requirement": "抓取销售数据生成月度经营分析报告",
        "slider_s": 0.5
    })
    .to_string();
    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/synthesize")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v["req_id"].is_string(), "应返回 req_id");
    assert!(v["docs_dir"].is_string(), "应返回 docs_dir");
    assert!(v["kappa"].is_number(), "应返回 κ");
}

#[tokio::test]
async fn registry_query_after_synthesize() {
    // 注意：合成与查询必须复用同一个 app 实例（注册表在内存中，按 app 隔离）
    let app = app();

    // 先合成，得到需求 id
    let body = serde_json::json!({
        "requirement": "把客服对话整理成工单并派发",
        "slider_s": 0.7
    })
    .to_string();
    let synth = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/synthesize")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(synth.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let req_id = v["req_id"].as_str().unwrap().to_string();

    // 按需求 id 查询绑定（同一 app 实例，命中刚才注册的绑定）
    let q = format!("/api/v1/registry/by-requirement?req={req_id}");
    let resp = app
        .clone()
        .oneshot(Request::builder().uri(&q).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 注册表统计可用
    let stats = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/registry/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stats.status(), StatusCode::OK);
}

#[tokio::test]
async fn gate_endpoint_returns_report() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/gate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v["passed"].is_boolean(), "闸门报告应含 passed");
    assert!(v["conservation"].is_object());
    assert!(v["binding"].is_object());
    assert!(v["governance"].is_object());
}

#[tokio::test]
async fn auth_enforced_when_token_set() {
    let cfg = Config {
        auth_token: Some("s3cr3t".into()),
        ..Config::default()
    };
    let app = app_with(cfg);

    // 无 token → 401
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/gate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    // 带 token → 200
    let ok = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/gate")
                .header("authorization", "Bearer s3cr3t")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);

    // health 不受鉴权保护
    let h = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(h.status(), StatusCode::OK);
}

#[tokio::test]
async fn docs_list_and_get() {
    // 先合成以触发 PT-DOC 导出（写到默认 docs_dir=data/fusion_docs）
    let body = serde_json::json!({
        "requirement": "生成一份上线发布检查清单",
        "slider_s": 0.5
    })
    .to_string();
    let _ = app()
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/synthesize")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let list = app()
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/docs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(list.into_body(), usize::MAX)
        .await
        .unwrap();
    let docs: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        docs.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "应至少导出一份 PT-DOC"
    );
}

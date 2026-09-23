// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! registry-svc HTTP 层集成测试
//!
//! 通过 `AppState::for_test()`（纯内存）+ `create_router` 构建真实路由，
//! 用 `tower::ServiceExt::oneshot` 发起真实 HTTP 请求，验证注册中心完整链路：
//! HTTP 入口 → 路由 → 注册中心存储 → 响应 JSON。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use mox_alliance_registry_svc::{AppState, routes::create_router};

/// 发送 JSON 请求，返回 (状态码, 响应体字节)
async fn send(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, Vec<u8>) {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    let req = match body {
        Some(json) => builder
            .body(Body::from(json.to_string()))
            .expect("build request"),
        None => builder.body(Body::empty()).expect("build request"),
    };
    let resp = app.clone().oneshot(req).await.expect("oneshot 应成功");
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes()
        .to_vec();
    (status, bytes)
}

#[tokio::test]
async fn registry_health_ok() {
    let app = create_router(AppState::for_test());
    let (status, bytes) = send(&app, "GET", "/api/registry/health", None).await;
    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "mox-alliance-registry");
}

#[tokio::test]
async fn register_then_get_then_heartbeat_flow() {
    let app = create_router(AppState::for_test());

    // 1) 注册实例
    let (s, b) = send(
        &app,
        "POST",
        "/api/registry/experts",
        Some(serde_json::json!({
            "name": "expert-code",
            "version": "1.0.0",
            "endpoint": "http://127.0.0.1:9100",
            "health_check_url": "http://127.0.0.1:9100/health",
            "capabilities": ["programming", "code"],
            "domain": "code",
            "weight": 1.0,
            "lease_seconds": 15
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "注册应 200，body={}", String::from_utf8_lossy(&b));
    let inst: serde_json::Value = serde_json::from_slice(&b).unwrap();
    let id = inst["id"].as_str().unwrap().to_string();
    assert!(!id.is_empty());
    assert_eq!(inst["status"], "active");

    // 2) 详情
    let (s2, b2) = send(&app, "GET", &format!("/api/registry/experts/{id}"), None).await;
    assert_eq!(s2, StatusCode::OK);
    let detail: serde_json::Value = serde_json::from_slice(&b2).unwrap();
    assert_eq!(detail["name"], "expert-code");

    // 3) 心跳续约并上报负载
    let (s3, b3) = send(
        &app,
        "POST",
        &format!("/api/registry/experts/{id}/heartbeat"),
        Some(serde_json::json!({ "load_current": 3 })),
    )
    .await;
    assert_eq!(s3, StatusCode::OK, "心跳应 200，body={}", String::from_utf8_lossy(&b3));
    let hb: serde_json::Value = serde_json::from_slice(&b3).unwrap();
    assert_eq!(hb["load_current"], 3);

    // 4) 不带 body 的心跳也应 200
    let (s4, _) = send(
        &app,
        "POST",
        &format!("/api/registry/experts/{id}/heartbeat"),
        None,
    )
    .await;
    assert_eq!(s4, StatusCode::OK);
}

#[tokio::test]
async fn discover_filters_by_capability_and_domain() {
    let app = create_router(AppState::for_test());

    // 注册两个不同领域实例
    for (name, domain, caps) in [
        ("expert-code", "code", vec!["programming", "code"]),
        ("expert-math", "math", vec!["mathematics", "reasoning"]),
    ] {
        send(
            &app,
            "POST",
            "/api/registry/experts",
            Some(serde_json::json!({
                "name": name, "endpoint": "http://127.0.0.1:9200",
                "domain": domain, "capabilities": caps
            })),
        )
        .await;
    }

    // 按 domain 过滤
    let (s, b) = send(&app, "GET", "/api/registry/experts?domain=math", None).await;
    assert_eq!(s, StatusCode::OK);
    let list: serde_json::Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(list["total"], 1);
    assert_eq!(list["instances"][0]["name"], "expert-math");

    // 按 capability 过滤
    let (s2, b2) = send(
        &app,
        "GET",
        "/api/registry/experts?capability=programming",
        None,
    )
    .await;
    let list2: serde_json::Value = serde_json::from_slice(&b2).unwrap();
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(list2["total"], 1);
    assert_eq!(list2["instances"][0]["name"], "expert-code");
}

#[tokio::test]
async fn deregister_then_get_returns_not_found() {
    let app = create_router(AppState::for_test());

    let (_, b) = send(
        &app,
        "POST",
        "/api/registry/experts",
        Some(serde_json::json!({
            "name": "expert-law", "endpoint": "http://127.0.0.1:9300", "domain": "law"
        })),
    )
    .await;
    let id = serde_json::from_slice::<serde_json::Value>(&b).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // 注销
    let (s, _) = send(&app, "DELETE", &format!("/api/registry/experts/{id}"), None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    // 再查应为 404
    let (s2, _) = send(&app, "GET", &format!("/api/registry/experts/{id}"), None).await;
    assert_eq!(s2, StatusCode::NOT_FOUND);

    // 重复注销也应 404
    let (s3, _) = send(&app, "DELETE", &format!("/api/registry/experts/{id}"), None).await;
    assert_eq!(s3, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn register_without_endpoint_rejected() {
    let app = create_router(AppState::for_test());
    // endpoint 为空串 → 处理器校验返回 400
    let (s, _) = send(
        &app,
        "POST",
        "/api/registry/experts",
        Some(serde_json::json!({ "name": "broken", "endpoint": "" })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // 完全缺少 endpoint 必填字段 → axum JSON 提取器返回 422，同样被拒绝
    let (s2, _) = send(
        &app,
        "POST",
        "/api/registry/experts",
        Some(serde_json::json!({ "name": "broken" })),
    )
    .await;
    assert!(s2.is_client_error(), "缺必填字段应 4xx，实际 {s2}");
}

#[tokio::test]
async fn heartbeat_unknown_instance_returns_not_found() {
    let app = create_router(AppState::for_test());
    let (s, _) = send(
        &app,
        "POST",
        "/api/registry/experts/does-not-exist/heartbeat",
        Some(serde_json::json!({})),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn legacy_directory_still_works() {
    // 旧版 /api/v1/experts CRUD 向后兼容
    let app = create_router(AppState::for_test());
    let (s, b) = send(
        &app,
        "POST",
        "/api/v1/experts",
        Some(serde_json::json!({ "name": "张专家" })),
    )
    .await;
    assert!(s.is_success(), "旧版创建应成功，body={}", String::from_utf8_lossy(&b));

    let (s2, b2) = send(&app, "GET", "/api/v1/experts", None).await;
    assert_eq!(s2, StatusCode::OK);
    let list: serde_json::Value = serde_json::from_slice(&b2).unwrap();
    assert!(list["total"].as_u64().unwrap() >= 1);
}

// ─── 分级心跳聚合（10:1:1）─────────────────────────────────────────────────

async fn register_named(app: &axum::Router, id: &str) -> String {
    let (s, b) = send(
        app,
        "POST",
        "/api/registry/experts",
        Some(serde_json::json!({
            "id": id, "name": format!("expert-{id}"),
            "endpoint": format!("http://127.0.0.1:9500/{id}")
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "注册 {id} 应 200");
    let inst: serde_json::Value = serde_json::from_slice(&b).unwrap();
    inst["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn aggregated_heartbeat_renews_registered_members() {
    let state = AppState::for_test();
    let app = create_router(state.clone());
    let a = register_named(&app, "agg-a").await;
    let b = register_named(&app, "agg-b").await;

    // 模拟 rack 代理：登记两个成员并凑满 fan_in=2 触发摘要发布
    use mox_alliance_registry_core::{MemberStatus, NodeBeat, RackAggregator, RackConfig};
    let mut rack = RackAggregator::new(RackConfig {
        rack_id: "r-agg".into(),
        cell_id: "c-agg".into(),
        fan_in: 2,
        ..Default::default()
    });
    rack.register(&a, Some(4), MemberStatus::Active);
    let digest = rack
        .heartbeat(&NodeBeat {
            member_id: b.clone(),
            load_current: Some(9),
            status: None,
        })
        .expect("2 条心跳应触发 rack 摘要");

    let renewal = mox_alliance_registry_core::AggregatedRenewal::from_digest(&digest);
    let (s, body) = send(
        &app,
        "POST",
        "/api/registry/aggregated-heartbeat",
        Some(serde_json::json!(&renewal)),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "聚合续约应 200，body={}", String::from_utf8_lossy(&body));
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp["renewed"], 2);
    assert_eq!(resp["unknown"], 0);
    assert_eq!(resp["group_id"], "r-agg");

    let ia = state.registry.get(&a).unwrap();
    assert_eq!(ia.load_current, 4);
    let ib = state.registry.get(&b).unwrap();
    assert_eq!(ib.load_current, 9);
    assert_eq!(ib.status, mox_alliance_registry_svc::models::InstanceStatus::Active);
}

#[tokio::test]
async fn aggregated_heartbeat_unknown_not_renewed_and_empty_is_400() {
    let state = AppState::for_test();
    let app = create_router(state.clone());

    // 全部未知成员：renewed=0，实例表不受影响
    let renewal = mox_alliance_registry_core::AggregatedRenewal {
        group_id: "r-x".into(),
        seq: 1,
        reported_at: chrono::Utc::now(),
        members: std::collections::BTreeMap::from([(
            "ghost".to_string(),
            mox_alliance_registry_core::Renewal { load_current: Some(1), status: mox_alliance_registry_core::MemberStatus::Active },
        )]),
        unknown_members: vec![],
        health: mox_alliance_registry_core::AggregationHealth::Healthy,
        member_count: 1,
        unhealthy_count: 0,
        load_total: 1,
    };
    let (s, b) = send(
        &app,
        "POST",
        "/api/registry/aggregated-heartbeat",
        Some(serde_json::json!(&renewal)),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let resp: serde_json::Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(resp["renewed"], 0);
    assert_eq!(resp["unknown"], 1);
    assert_eq!(state.registry.count(), 0, "聚合续约不得隐式注册");

    // 空载荷 → 400
    let (s2, _) = send(
        &app,
        "POST",
        "/api/registry/aggregated-heartbeat",
        Some(serde_json::json!({
            "group_id": "r-empty", "seq": 1,
            "reported_at": chrono::Utc::now().to_rfc3339(),
            "members": {}, "unknown_members": [],
            "health": "healthy", "member_count": 0,
            "unhealthy_count": 0, "load_total": 0
        })),
    )
    .await;
    assert_eq!(s2, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn aggregated_heartbeat_prunes_deregistered_members() {
    let state = AppState::for_test();
    let app = create_router(state.clone());
    let keep = register_named(&app, "keep").await;
    let gone = register_named(&app, "gone").await;

    // rack 聚合器中 "gone" 已消失（显式注销沿聚合链传播）
    use mox_alliance_registry_core::{MemberStatus, RackAggregator, RackConfig};
    let mut rack = RackAggregator::new(RackConfig {
        rack_id: "r-p".into(),
        cell_id: "c-p".into(),
        fan_in: 1,
        ..Default::default()
    });
    rack.register(&keep, None, MemberStatus::Active);
    rack.register(&gone, None, MemberStatus::Active);
    rack.forget_member(&gone);
    let digest = rack.flush();
    assert_eq!(digest.unknown_members, vec![gone.clone()]);

    let renewal = mox_alliance_registry_core::AggregatedRenewal::from_digest(&digest);
    let (s, b) = send(
        &app,
        "POST",
        "/api/registry/aggregated-heartbeat",
        Some(serde_json::json!(&renewal)),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "body={}", String::from_utf8_lossy(&b));
    let resp: serde_json::Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(resp["pruned"], 1);
    assert!(state.registry.get(&gone).is_none(), "unknown 成员应被摘除");
    assert!(state.registry.get(&keep).is_some());
}

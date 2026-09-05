// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 联盟领域服务远程接入集成测试
//!
//! 用 mock 调度器/执行器（模拟 scheduler-svc :3100 / executor-svc :3200 的
//! 真实 HTTP 契约）端到端驱动网关 `/api/alliance/*` 路由，验证：
//! 1. 归一化正确性（枚举映射 / 时间戳秒精度 / 字段对齐 / 响应信封）
//! 2. 远程模式传输失败返回错误，禁止切换数据源
//! 3. 未配置 URL / mode=off 全本地（默认行为不变）

use axum::extract::Path;
use axum::{Json, Router};
use axum::routing::{get, post};
use mox_platform_gateway_svc::alliance::build_alliance_router_with;
use mox_platform_gateway_svc::alliance_remote::RemoteAllianceClient;
use serde_json::{json, Value};

/// 在临时端口上启动路由，返回基址
async fn spawn(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{}", addr)
}

fn mock_task_with_id(id: &str) -> Value {
    json!({
        "task_id": id,
        "title": "远程任务",
        "description": "远程描述",
        "status": "running",
        "priority": "high",
        "progress": 0.5,
        "mode": "parallel",
        "created_at": "2026-09-04T08:00:00.123456789Z",
        "started_at": "2026-09-04T08:00:01Z",
        "completed_at": null,
        "duration_ms": null,
    })
}

/// 模拟 scheduler-svc（:3100 契约）
fn mock_scheduler() -> Router {
    Router::new()
        .route(
            "/tasks",
            post(|| async {
                Json(json!({
                    "task_id": "11111111-1111-1111-1111-111111111111",
                    "title": "远程任务",
                    "status": "pending",
                    "created_at": "2026-09-04T08:00:00.123456789Z",
                }))
            })
            .get(|| async {
                Json(json!({
                    "tasks": [mock_task_with_id("22222222-2222-2222-2222-222222222222")],
                    "total": 1, "page": 1, "page_size": 20,
                }))
            }),
        )
        .route(
            "/tasks/:id",
            get(|Path(id): Path<String>| async move { Json(mock_task_with_id(&id)) })
                .post(|| async { Json(json!({"success": true, "message": "OK"})) }),
        )
        .route(
            "/experts/search",
            post(|| async {
                Json(json!({
                    "experts": [{
                        "expert_id": "expert-architecture",
                        "name": "架构设计专家",
                        "description": "架构设计",
                        "domains": ["architecture"],
                        "status": "active",
                    }],
                    "total": 3,
                }))
            }),
        )
}

/// 模拟 executor-svc（:3200 契约）
fn mock_executor() -> Router {
    Router::new()
        .route(
            "/tasks/:id/status",
            get(|Path(id): Path<String>| async move {
                Json(json!({
                    "task_id": id, "status": "running", "progress": 0.5,
                    "total_nodes": 4, "completed_nodes": 2, "running_nodes": 1,
                    "failed_nodes": 0, "pending_nodes": 1,
                    "skipped_nodes": 0, "cancelled_nodes": 0,
                }))
            }),
        )
        .route(
            "/tasks/:id/nodes",
            get(|Path(_): Path<String>| async move {
                Json(json!({
                    "nodes": [
                        { "node_id": "n1", "name": "需求分析", "expert_id": "e1",
                          "status": "completed", "dependencies": [],
                          "started_at": null, "completed_at": null,
                          "duration_ms": null, "error_message": null },
                        { "node_id": "n2", "name": "架构设计", "expert_id": "e2",
                          "status": "ready", "dependencies": ["n1"],
                          "started_at": null, "completed_at": null,
                          "duration_ms": null, "error_message": null }
                    ],
                    "total": 2,
                }))
            }),
        )
        .route(
            "/tasks/:id/nodes/:nid",
            get(|Path((_id, nid)): Path<(String, String)>| async move {
                Json(json!({
                    "node_id": nid, "name": "节点", "expert_id": "e1",
                    "status": "ready", "dependencies": [],
                    "started_at": null, "completed_at": null,
                    "duration_ms": null, "error_message": null,
                }))
            })
            .post(|| async { Json(json!({"success": true, "message": "OK"})) }),
        )
        .route(
            "/tasks/:id/result",
            get(|| async { Json(json!({"summary": "融合结论", "confidence": 0.9})) }),
        )
}

/// 网关（远程已配置）+ mock 服务全拓扑，返回网关基址
async fn setup_remote() -> String {
    let sched = spawn(mock_scheduler()).await;
    let exec = spawn(mock_executor()).await;
    let remote = RemoteAllianceClient::explicit(Some(sched), Some(exec));
    spawn(build_alliance_router_with(remote)).await
}

async fn body_of(url: &str) -> Value {
    reqwest::get(url).await.unwrap().json::<Value>().await.unwrap()
}

// ====================================================================
// 调度器端点归一化
// ====================================================================

#[tokio::test]
async fn test_remote_create_task_normalized() {
    let gw = setup_remote().await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/alliance/tasks", gw))
        .json(&json!({
            "title": "t", "description": "d",
            "priority": "high", "mode": "parallel", "fusion_strategy": "weighted",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<Value>().await.unwrap();
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["data"]["task_id"], "11111111-1111-1111-1111-111111111111");
    // 时间戳归一化为秒精度
    assert_eq!(body["data"]["data"]["created_at"], "2026-09-04T08:00:00Z");
    // 枚举归一化：parallel → expert_alliance
    assert_eq!(body["data"]["params"]["mode"], "expert_alliance");
    assert_eq!(body["data"]["params"]["fusion_strategy"], "weighted_voting");
}

#[tokio::test]
async fn test_remote_list_and_detail_normalized() {
    let gw = setup_remote().await;
    let list = body_of(&format!("{}/api/alliance/tasks", gw)).await;
    assert_eq!(list["code"], 0);
    assert_eq!(list["data"]["data"]["total"], 1);
    assert_eq!(list["data"]["data"]["tasks"][0]["mode"], "expert_alliance");
    assert_eq!(list["data"]["data"]["tasks"][0]["created_at"], "2026-09-04T08:00:00Z");

    let detail = body_of(&format!(
        "{}/api/alliance/tasks/22222222-2222-2222-2222-222222222222",
        gw
    ))
    .await;
    assert_eq!(detail["code"], 0);
    assert_eq!(detail["data"]["data"]["status"], "running");
    assert_eq!(detail["data"]["data"]["priority"], "high");
}

#[tokio::test]
async fn test_remote_task_action_normalized_message() {
    let gw = setup_remote().await;
    let resp = reqwest::Client::new()
        .post(format!(
            "{}/api/alliance/tasks/22222222-2222-2222-2222-222222222222/pause",
            gw
        ))
        .send()
        .await
        .unwrap();
    let body = resp.json::<Value>().await.unwrap();
    assert_eq!(body["code"], 0);
    // 归一化为本地中文文案（远程仅返回通用 "OK"）
    assert_eq!(body["data"]["data"]["message"], "任务 22222222-2222-2222-2222-222222222222 已暂停");
    assert_eq!(body["data"]["data"]["success"], true);
}

#[tokio::test]
async fn test_remote_expert_search_status_mapping() {
    let gw = setup_remote().await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/alliance/experts/search", gw))
        .json(&json!({"query": "架构", "domains": [], "limit": 5}))
        .send()
        .await
        .unwrap();
    let body = resp.json::<Value>().await.unwrap();
    assert_eq!(body["code"], 0);
    // active → online（对齐本地展示）
    assert_eq!(body["data"]["data"]["experts"][0]["status"], "online");
    // 对齐本地语义：total = 匹配数（非远程可用总数 3）
    assert_eq!(body["data"]["data"]["total"], 1);
}

// ====================================================================
// 执行器端点归一化
// ====================================================================

#[tokio::test]
async fn test_remote_execution_status_normalized() {
    let gw = setup_remote().await;
    let body = body_of(&format!(
        "{}/api/alliance/tasks/22222222-2222-2222-2222-222222222222/execution-status",
        gw
    ))
    .await;
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["data"]["total_nodes"], 4);
    assert_eq!(body["data"]["data"]["completed_nodes"], 2);
    assert_eq!(body["data"]["data"]["running_nodes"], 1);
    assert_eq!(body["data"]["data"]["progress"], 0.5);
}

#[tokio::test]
async fn test_remote_nodes_ready_mapped_to_pending() {
    let gw = setup_remote().await;
    let body = body_of(&format!(
        "{}/api/alliance/tasks/22222222-2222-2222-2222-222222222222/nodes",
        gw
    ))
    .await;
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["data"]["total"], 2);
    assert_eq!(body["data"]["data"]["nodes"][0]["status"], "completed");
    // ready → pending（对齐本地节点状态枚举）
    assert_eq!(body["data"]["data"]["nodes"][1]["status"], "pending");
}

#[tokio::test]
async fn test_remote_node_detail_and_skip() {
    let gw = setup_remote().await;
    let detail = body_of(&format!(
        "{}/api/alliance/tasks/22222222-2222-2222-2222-222222222222/nodes/n2",
        gw
    ))
    .await;
    assert_eq!(detail["data"]["data"]["node_id"], "n2");

    let resp = reqwest::Client::new()
        .post(format!(
            "{}/api/alliance/tasks/22222222-2222-2222-2222-222222222222/nodes/n2",
            gw
        ))
        .send()
        .await
        .unwrap();
    let body = resp.json::<Value>().await.unwrap();
    assert_eq!(body["data"]["data"]["message"], "节点 n2 已跳过");
}

#[tokio::test]
async fn test_remote_dag_fusion_and_status_poll() {
    let gw = setup_remote().await;
    let tid = "22222222-2222-2222-2222-222222222222";

    let dag = body_of(&format!("{}/api/alliance/tasks/{}/dag", gw, tid)).await;
    assert_eq!(dag["code"], 0, "dag body: {}", dag);
    assert_eq!(
        dag["data"]["data"]["stats"]["completed"],
        1,
        "dag body: {}",
        dag
    );
    assert_eq!(dag["data"]["data"]["stats"]["pending"], 1);
    assert_eq!(dag["data"]["data"]["edges"][0]["source"], "n1");
    assert_eq!(dag["data"]["data"]["nodes"][0]["position"]["x"], 100);
    assert_eq!(dag["data"]["data"]["nodes"][1]["position"]["x"], 350);

    let fusion = body_of(&format!("{}/api/alliance/tasks/{}/fusion-result", gw, tid)).await;
    assert_eq!(fusion["code"], 0);
    assert_eq!(fusion["data"]["data"]["fusion_result"]["summary"], "融合结论");

    let poll = body_of(&format!("{}/api/alliance/tasks/{}/status", gw, tid)).await;
    assert_eq!(poll["code"], 0);
    assert_eq!(poll["data"]["data"]["progress"], 0.5);
    assert_eq!(poll["data"]["data"]["total_nodes"], 4);
    assert_eq!(poll["data"]["data"]["status"], "running");
}

// ====================================================================
// 降级与禁用语义
// ====================================================================

#[tokio::test]
async fn configured_remote_failure_does_not_create_local_task() {
    let remote = RemoteAllianceClient::explicit(
        Some("http://127.0.0.1:1".into()), Some("http://127.0.0.1:1".into()),
    );
    let gw = spawn(build_alliance_router_with(remote)).await;
    let response = reqwest::Client::new().post(format!("{}/api/alliance/tasks", gw))
        .json(&json!({"title":"must not appear locally", "description":"d"}))
        .send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(response["code"], 503);
    let list = body_of(&format!("{}/api/alliance/tasks", gw)).await;
    assert_eq!(list["code"], 503, "remote outage must not look like an empty local list");
}

#[tokio::test]
async fn test_disabled_when_no_urls_configured() {
    // 未配置任何 URL → None → 全本地（默认行为不变）
    assert!(RemoteAllianceClient::explicit(None, None).is_none());

    let gw = spawn(build_alliance_router_with(None)).await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/alliance/tasks", gw))
        .json(&json!({"title": "t", "description": "d"}))
        .send()
        .await
        .unwrap();
    let body = resp.json::<Value>().await.unwrap();
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["data"]["status"], "pending");
}

#[tokio::test]
async fn readiness_requires_configured_real_execution() {
    let scheduler = spawn(Router::new().route("/health", get(|| async { Json(json!({"status":"healthy"})) }))).await;
    for configured in [false, true] {
        let executor = spawn(Router::new().route("/health", get(move || async move {
            Json(json!({"status":"healthy","execution_ready":configured}))
        }))).await;
        let gw = spawn(build_alliance_router_with(RemoteAllianceClient::explicit(Some(scheduler.clone()), Some(executor)))).await;
        let response = body_of(&format!("{gw}/api/alliance/runtime")).await;
        assert_eq!(response["data"]["execution_ready"], configured);
    }
    let gw = spawn(build_alliance_router_with(None)).await;
    let response = body_of(&format!("{gw}/api/alliance/runtime")).await;
    assert_eq!(response["data"]["execution_ready"], false);
}

#[tokio::test]
async fn result_service_failure_is_not_pending_success() {
    let executor = spawn(Router::new().route("/tasks/:id/result", get(|| async {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(json!({"message":"unavailable"})))
    }))).await;
    let gw = spawn(build_alliance_router_with(RemoteAllianceClient::explicit(Some(executor.clone()), Some(executor)))).await;
    let response = body_of(&format!("{gw}/api/alliance/tasks/22222222-2222-2222-2222-222222222222/fusion-result")).await;
    assert_eq!(response["code"], 503);
}

#[tokio::test]
async fn logs_are_remote_node_snapshots() {
    let gw = setup_remote().await;
    let response = body_of(&format!("{gw}/api/alliance/tasks/22222222-2222-2222-2222-222222222222/logs")).await;
    assert_eq!(response["data"]["data"]["source"], "executor_snapshot");
    assert!(!response["data"]["data"]["logs"].as_array().unwrap().is_empty());
}

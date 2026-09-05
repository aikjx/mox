// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! executor-svc HTTP 层集成测试。
//!
//! 通过 `ExecutorServer::build_app()`（Mock 执行器模式）构建真实路由，
//! 用 `tower::ServiceExt::oneshot` 验证完整执行链路：
//! 提交执行 → 状态查询 → 节点列表 → 取消执行 → 健康检查。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use mox_alliance_api::dto::SuccessResponse;
use mox_alliance_executor_proto::types::ExecutorConfig;
use mox_alliance_executor_svc::{ExecutorMode, ExecutorServer};

async fn build_test_app() -> axum::Router {
    let server = ExecutorServer::new(
        ExecutorConfig::default(),
        "127.0.0.1:3200".parse().unwrap(),
    )
    .with_mode(ExecutorMode::Mock);
    server.build_app().await.expect("build_app 应成功")
}

#[tokio::test]
async fn unconfigured_model_rejects_execution_without_creating_state() {
    use mox_alliance_executor_core::{DagEngineImpl, MockExecutorConfig, MockNodeExecutor};
    use mox_alliance_executor_svc::{app_state::ExecutorAppState, routes::build_router};
    let config = ExecutorConfig::default();
    let engine = DagEngineImpl::spawn(config.clone(), std::sync::Arc::new(MockNodeExecutor::new(MockExecutorConfig::default())));
    let mut state = ExecutorAppState::new(config, engine);
    state.execution_ready = false;
    state.execution_mode = "llm";
    let app = build_router(state);
    let body = make_submit_json();
    let task_id = body["task"]["task_id"].as_str().unwrap().to_owned();
    let (status, bytes) = send(&app, "POST", "/internal/executions", Some(body), None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["code"], "MODEL_NOT_CONFIGURED");
    assert_eq!(send(&app, "GET", &format!("/tasks/{task_id}/status"), None, None).await.0, StatusCode::NOT_FOUND);
    let (_, bytes) = send(&app, "GET", "/health", None, None).await;
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["execution_ready"], false);
}

/// 发送 JSON 请求并返回 (status, json body bytes)
async fn send(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
    tenant: Option<Uuid>,
) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = tenant {
        builder = builder.header("X-Tenant-Id", t.to_string());
    }
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

/// 构造最小可执行的提交请求 JSON（2 节点并行 DAG）
fn make_submit_json() -> serde_json::Value {
    let task_id = Uuid::new_v4();
    let tenant = Uuid::nil();
    let user = Uuid::new_v4();
    serde_json::json!({
        "task": {
            "task_id": task_id,
            "tenant_id": tenant,
            "user_id": user,
            "title": "集成测试任务",
            "description": "验证执行器全链路",
            "task_type": "test",
            "status": "pending",
            "priority": "normal",
            "progress": 0.0,
            "current_node_id": null,
            "mode": "parallel",
            "fusion_strategy": "weighted",
            "created_at": "2026-08-31T00:00:00Z",
            "started_at": null,
            "completed_at": null,
            "duration_ms": null
        },
        "plan": {
            "task_id": task_id,
            "mode": "parallel",
            "fusion_strategy": "weighted",
            "nodes": [
                {
                    "node_id": "n1",
                    "task_id": task_id,
                    "expert_id": "exp-1",
                    "module_id": "expert-code",
                    "name": "代码专家",
                    "description": "生成代码",
                    "status": "pending",
                    "retry_count": 0,
                    "dependencies": [],
                    "input_refs": [],
                    "output_ref": null,
                    "started_at": null,
                    "completed_at": null,
                    "duration_ms": null
                },
                {
                    "node_id": "n2",
                    "task_id": task_id,
                    "expert_id": "exp-2",
                    "module_id": "expert-math",
                    "name": "数学专家",
                    "description": "数学验证",
                    "status": "pending",
                    "retry_count": 0,
                    "dependencies": [],
                    "input_refs": [],
                    "output_ref": null,
                    "started_at": null,
                    "completed_at": null,
                    "duration_ms": null
                }
            ],
            "version": 1,
            "created_at": "2026-08-31T00:00:00Z"
        },
        "options": {
            "max_retries": 0,
            "node_timeout_ms": 5000,
            "fail_fast": false
        }
    })
}

#[tokio::test]
async fn health_check_ok() {
    let app = build_test_app().await;
    let (status, _) = send(&app, "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn submit_execution_and_query_status() {
    let app = build_test_app().await;
    let submit_json = make_submit_json();
    let task_id = submit_json["task"]["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 1) 提交执行
    let (status, bytes) = send(&app, "POST", "/internal/executions", Some(submit_json.clone()), Some(Uuid::nil())).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "提交执行应 200，body={}",
        String::from_utf8_lossy(&bytes)
    );
    let resp: SuccessResponse = serde_json::from_slice(&bytes).expect("解析 SuccessResponse");
    assert!(resp.success);

    // 2) 等待 Mock 节点执行完成（delay 50ms，并行 2 节点）
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // 3) 查询执行状态
    let (status2, bytes2) = send(
        &app,
        "GET",
        &format!("/tasks/{task_id}/status"),
        None,
        Some(Uuid::nil()),
    )
    .await;
    assert_eq!(status2, StatusCode::OK);
    let exec: mox_alliance_executor_proto::ExecutionStatus =
        serde_json::from_slice(&bytes2).expect("解析 ExecutionStatus");
    assert_eq!(exec.task_id.to_string(), task_id);
    assert_eq!(exec.total_nodes, 2);
    assert_eq!(
        exec.completed_nodes, 2,
        "Mock 执行器应完成全部节点，actual={:?}",
        exec
    );
}

#[tokio::test]
async fn list_nodes_after_submit() {
    let app = build_test_app().await;
    let submit_json = make_submit_json();
    let task_id = submit_json["task"]["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, _) = send(&app, "POST", "/internal/executions", Some(submit_json), Some(Uuid::nil())).await;
    assert!(status.is_success());

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let (status2, bytes2) = send(&app, "GET", &format!("/tasks/{task_id}/nodes"), None, Some(Uuid::nil())).await;
    assert_eq!(status2, StatusCode::OK);
    let nodes: serde_json::Value = serde_json::from_slice(&bytes2).expect("解析节点列表");
    assert_eq!(nodes["total"].as_u64(), Some(2), "应有 2 个节点");
}

#[tokio::test]
async fn cancel_execution_ok() {
    let app = build_test_app().await;
    let submit_json = make_submit_json();
    let task_id = submit_json["task"]["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, _) = send(&app, "POST", "/internal/executions", Some(submit_json), Some(Uuid::nil())).await;
    assert!(status.is_success());

    // 立即取消（Mock 有 50ms delay，可能仍在执行中）
    let (status2, bytes2) = send(
        &app,
        "POST",
        &format!("/tasks/{task_id}/cancel"),
        Some(serde_json::json!({
            "tenant_id": "00000000-0000-0000-0000-000000000000",
            "reason": "测试取消"
        })),
        Some(Uuid::nil()),
    )
    .await;
    assert!(
        status2.is_success() || status2 == StatusCode::NOT_FOUND,
        "取消应 2xx 或 404（任务已终态），实际 {} body={}",
        status2,
        String::from_utf8_lossy(&bytes2)
    );
}

#[tokio::test]
async fn unknown_task_status_returns_not_found() {
    let app = build_test_app().await;
    let (status, _) = send(
        &app,
        "GET",
        &format!("/tasks/{}/status", Uuid::new_v4()),
        None,
        Some(Uuid::nil()),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! scheduler-svc HTTP 层集成测试。
//!
//! 通过 `SchedulerServer::build_app()` 构建真实路由（注入进程内 stub 执行引擎），
//! 再用 `tower::ServiceExt::oneshot` 发起真实 HTTP 请求，验证完整链路：
//! HTTP 入口 → 租户贯通 → 调度器（专家匹配+计划生成）→ 响应 DTO。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use mox_alliance_api::dto::{
    CreateTaskResponse, TaskAction, TaskActionRequest, TaskDetailResponse, TaskListResponse,
};
use mox_alliance_common_proto::{AllianceError, AllianceResult, CollaborationPlan, Node, Task};
use mox_alliance_executor_proto::{DagEngine, ExecutionOptions, ExecutionStatus, ExecutorConfig};
use mox_alliance_scheduler_core::InMemoryTaskRepository;
use mox_alliance_scheduler_proto::types::SchedulerConfig;
use mox_alliance_scheduler_svc::{SchedulerMode, SchedulerServer};

/// 进程内 stub 执行引擎（真实 HTTP 链路测试用，返回确定性结果）
#[derive(Default)]
struct StubEngine {
    config: ExecutorConfig,
}

#[async_trait::async_trait]
impl DagEngine for StubEngine {
    async fn start_execution(
        &self,
        _task: &Task,
        _plan: CollaborationPlan,
        _options: ExecutionOptions,
    ) -> AllianceResult<()> {
        Ok(())
    }
    async fn pause_execution(&self, _task_id: Uuid, _tenant_id: Uuid) -> AllianceResult<()> {
        Ok(())
    }
    async fn resume_execution(&self, _task_id: Uuid, _tenant_id: Uuid) -> AllianceResult<()> {
        Ok(())
    }
    async fn cancel_execution(
        &self,
        _task_id: Uuid,
        _tenant_id: Uuid,
        _reason: Option<String>,
    ) -> AllianceResult<()> {
        Ok(())
    }
    async fn get_execution_status(
        &self,
        task_id: Uuid,
        _tenant_id: Uuid,
    ) -> AllianceResult<ExecutionStatus> {
        Ok(ExecutionStatus {
            task_id,
            total_nodes: 0,
            completed_nodes: 0,
            running_nodes: 0,
            failed_nodes: 0,
            pending_nodes: 0,
            skipped_nodes: 0,
            cancelled_nodes: 0,
            progress: 0.0,
            started_at: None,
            estimated_remaining_ms: None,
        })
    }
    async fn get_nodes(&self, _task_id: Uuid, _tenant_id: Uuid) -> AllianceResult<Vec<Node>> {
        Ok(vec![])
    }
    async fn get_node(
        &self,
        _task_id: Uuid,
        _node_id: &str,
        _tenant_id: Uuid,
    ) -> AllianceResult<Node> {
        Err(AllianceError::not_found("node", "n/a"))
    }
    async fn skip_node(
        &self,
        _task_id: Uuid,
        _node_id: &str,
        _tenant_id: Uuid,
        _reason: Option<String>,
    ) -> AllianceResult<()> {
        Ok(())
    }
    fn config(&self) -> &ExecutorConfig {
        &self.config
    }
}

async fn build_test_app() -> axum::Router {
    let server = SchedulerServer::new(
        SchedulerConfig::default(),
        "127.0.0.1:3100".parse().unwrap(),
    )
    .with_mode(SchedulerMode::Embedded)
    .with_embedded_engine(Arc::new(StubEngine::default()))
    .with_task_repository(Arc::new(InMemoryTaskRepository::new()));

    server.build_app().await.expect("build_app 应成功")
}

/// 发送 JSON 请求并返回 (status, json body bytes)
async fn send(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
    tenant_id: Option<Uuid>,
) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = tenant_id {
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

#[tokio::test]
async fn health_check_ok() {
    let app = build_test_app().await;
    let (status, _) = send(&app, "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn create_then_get_task_full_flow() {
    let app = build_test_app().await;
    let tenant = Uuid::new_v4();

    // 1) 创建任务
    let (status, bytes) = send(
        &app,
        "POST",
        "/tasks",
        Some(serde_json::json!({
            "title": "数据分析",
            "description": "分析季度销售数据并给出建议",
            "task_type": "analysis",
            "mode": "parallel",
            "fusion_strategy": "weighted"
        })),
        Some(tenant),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "创建任务应 200，body={}", String::from_utf8_lossy(&bytes));
    let created: CreateTaskResponse = serde_json::from_slice(&bytes).expect("解析 CreateTaskResponse");
    assert!(!created.task_id.is_nil());

    // 2) 查询任务详情
    let (status2, bytes2) = send(
        &app,
        "GET",
        &format!("/tasks/{}", created.task_id),
        None,
        Some(tenant),
    )
    .await;
    assert_eq!(status2, StatusCode::OK);
    let detail: TaskDetailResponse = serde_json::from_slice(&bytes2).expect("解析 TaskDetailResponse");
    assert_eq!(detail.task_id, created.task_id);
    assert_eq!(detail.title, "数据分析");
}

#[tokio::test]
async fn list_tasks_returns_created() {
    let app = build_test_app().await;
    let tenant = Uuid::new_v4();

    let (s, b) = send(
        &app,
        "POST",
        "/tasks",
        Some(serde_json::json!({
            "title": "t1", "description": "d1"
        })),
        Some(tenant),
    )
    .await;
    assert!(s.is_success(), "创建失败 {}", String::from_utf8_lossy(&b));

    let (status, bytes) = send(&app, "GET", "/tasks", None, Some(tenant)).await;
    assert_eq!(status, StatusCode::OK);
    let list: TaskListResponse = serde_json::from_slice(&bytes).expect("解析 TaskListResponse");
    assert_eq!(list.total, 1);
    assert_eq!(list.tasks[0].title, "t1");
}

#[tokio::test]
async fn cancel_task_action_ok() {
    let app = build_test_app().await;
    let tenant = Uuid::new_v4();

    let (s, b) = send(
        &app,
        "POST",
        "/tasks",
        Some(serde_json::json!({"title": "t", "description": "d"})),
        Some(tenant),
    )
    .await;
    assert!(s.is_success());
    let created: CreateTaskResponse = serde_json::from_slice(&b).unwrap();

    let (status, bytes) = send(
        &app,
        "POST",
        &format!("/tasks/{}", created.task_id),
        Some(serde_json::to_value(TaskActionRequest {
            action: TaskAction::Cancel,
            reason: Some("用户取消".to_string()),
        })
        .unwrap()),
        Some(tenant),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "取消应 200，body={}", String::from_utf8_lossy(&bytes));
}

#[tokio::test]
async fn search_experts_returns_builtin() {
    let app = build_test_app().await;

    let (status, bytes) = send(
        &app,
        "POST",
        "/experts/search",
        Some(serde_json::json!({
            "query": "数据分析",
            "domains": [],
            "limit": 10
        })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let resp: mox_alliance_api::dto::ExpertSearchResponse =
        serde_json::from_slice(&bytes).expect("解析 ExpertSearchResponse");
    // 内置领域专家来自 config-core 模块配置，应能命中数据分析相关专家
    assert!(resp.total > 0, "应返回内置专家");
}

#[tokio::test]
async fn unknown_task_returns_not_found() {
    let app = build_test_app().await;
    let tenant = Uuid::new_v4();
    let (status, bytes) = send(
        &app,
        "GET",
        &format!("/tasks/{}", Uuid::new_v4()),
        None,
        Some(tenant),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let err: mox_alliance_api::dto::ErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(err.error_code, 2000, "TaskNotFound 应映射为 2000");
}

#[tokio::test]
async fn invalid_json_returns_client_error() {
    let app = build_test_app().await;
    let (status, _) = send(
        &app,
        "POST",
        "/tasks",
        Some(serde_json::json!({"foo": "bar"})),
        None,
    )
    .await;
    // 缺少 title/description → 应被拒绝（4xx），而非 500
    assert!(
        status.is_client_error(),
        "缺少必填字段应返回 4xx，实际 {}",
        status
    );
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use mox_project_graph_svc::dto::*;
use mox_project_graph_svc::{AppState, router};
use serde_json::json;

// ─── ApiResponse 测试 ────────────────────────────────────────────────────────

#[test]
fn api_response_ok_constructor() {
    let resp = ApiResponse::ok(42);
    assert_eq!(resp.code, 0);
    assert_eq!(resp.message, "ok");
    assert_eq!(resp.data, Some(42));
}

#[test]
fn api_response_error_constructor() {
    let resp: ApiResponse<()> = ApiResponse::error(500, "内部错误");
    assert_eq!(resp.code, 500);
    assert_eq!(resp.message, "内部错误");
    assert!(resp.data.is_none());
}

#[test]
fn api_response_serialization() {
    let resp = ApiResponse::ok(json!({"key": "value"}));
    let json_val = serde_json::to_value(&resp).unwrap();
    assert_eq!(json_val["code"], 0);
    assert_eq!(json_val["message"], "ok");
    assert_eq!(json_val["data"]["key"], "value");

    // error 响应不包含 data 字段
    let err: ApiResponse<()> = ApiResponse::error(404, "not found");
    let json_val = serde_json::to_value(&err).unwrap();
    assert_eq!(json_val["code"], 404);
    assert!(!json_val.as_object().unwrap().contains_key("data"));
}

// ─── AppState 测试 ───────────────────────────────────────────────────────────

#[tokio::test]
async fn app_state_new_and_default() {
    let state = AppState::new();
    // 验证 engine 可用：新项目列表为空
    let projects = state.engine.list_projects().await;
    assert!(projects.is_empty());

    let state2 = AppState::default();
    let projects2 = state2.engine.list_projects().await;
    assert!(projects2.is_empty());
}

#[tokio::test]
async fn app_state_is_clone() {
    let state = AppState::new();
    let cloned = state.clone();
    // 克隆后共享同一个 engine (Arc)
    let projects = cloned.engine.list_projects().await;
    assert!(projects.is_empty());
}

// ─── Router 构建测试 ─────────────────────────────────────────────────────────

#[test]
fn router_builds_successfully() {
    let state = AppState::new();
    let _r = router(state);
    // 只要不 panic 就算通过
}

// ─── 项目 DTO 测试 ───────────────────────────────────────────────────────────

#[test]
fn create_project_request_defaults() {
    let json = json!({
        "name": "新项目",
        "code": "NP"
    });
    let req: CreateProjectRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.name, "新项目");
    assert_eq!(req.code, "NP");
    assert_eq!(req.status, "planning"); // 默认值
    assert_eq!(req.priority, "P2"); // 默认值
    assert!(req.description.is_none());
    assert!(req.tags.is_none());
}

#[test]
fn project_response_fields() {
    let resp = ProjectResponse {
        id: "project:test".to_string(),
        name: "测试项目".to_string(),
        code: "TEST".to_string(),
        description: Some("描述".to_string()),
        status: "in_progress".to_string(),
        status_label: "进行中".to_string(),
        priority: "P1".to_string(),
        priority_label: "高".to_string(),
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-12-31".to_string()),
        owner_id: Some("owner-1".to_string()),
        progress: 0.5,
        tags: vec!["tag1".to_string()],
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-02T00:00:00Z".to_string(),
    };
    assert_eq!(resp.id, "project:test");
    assert_eq!(resp.status_label, "进行中");
    assert_eq!(resp.priority_label, "高");
    assert_eq!(resp.progress, 0.5);
}

#[test]
fn update_project_request_optional_fields() {
    let json = json!({
        "name": "更新名称"
    });
    let req: UpdateProjectRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.name, Some("更新名称".to_string()));
    assert!(req.description.is_none());
    assert!(req.status.is_none());
    assert!(req.priority.is_none());
}

// ─── 需求 DTO 测试 ───────────────────────────────────────────────────────────

#[test]
fn create_requirement_request_defaults() {
    let json = json!({
        "title": "新需求"
    });
    let req: CreateRequirementRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.title, "新需求");
    assert_eq!(req.status, "pending_review"); // 默认
    assert_eq!(req.priority, "P2"); // 默认
    assert_eq!(req.requirement_type, "功能需求"); // 默认
    assert!(req.description.is_none());
}

#[test]
fn requirement_response_serialization() {
    let resp = RequirementResponse {
        id: "req:123".to_string(),
        title: "登录需求".to_string(),
        description: Some("用户登录功能".to_string()),
        status: "in_development".to_string(),
        status_label: "开发中".to_string(),
        priority: "P0".to_string(),
        priority_label: "紧急".to_string(),
        requirement_type: "功能需求".to_string(),
        source: Some("客户".to_string()),
        story_points: Some(8),
        acceptance_criteria: Some("登录成功".to_string()),
        created_by: Some("pm".to_string()),
        tags: vec!["auth".to_string()],
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-02T00:00:00Z".to_string(),
    };
    let json_val = serde_json::to_value(&resp).unwrap();
    assert_eq!(json_val["id"], "req:123");
    assert_eq!(json_val["title"], "登录需求");
    assert_eq!(json_val["story_points"], 8);
    assert_eq!(json_val["status_label"], "开发中");
}

// ─── 任务 DTO 测试 ───────────────────────────────────────────────────────────

#[test]
fn create_task_request_defaults() {
    let json = json!({
        "title": "新任务"
    });
    let req: CreateTaskRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.title, "新任务");
    assert_eq!(req.status, "todo"); // 默认
    assert_eq!(req.priority, "P2"); // 默认
    assert_eq!(req.task_type, "开发"); // 默认
    assert!(req.estimate_hours.is_none());
    assert!(req.parent_id.is_none());
}

#[test]
fn task_response_fields() {
    let resp = TaskResponse {
        id: "task:abc".to_string(),
        title: "实现接口".to_string(),
        description: Some("后端 API".to_string()),
        status: "in_progress".to_string(),
        status_label: "进行中".to_string(),
        priority: "P0".to_string(),
        priority_label: "紧急".to_string(),
        task_type: "开发".to_string(),
        estimate_hours: Some(8.0),
        actual_hours: Some(3.5),
        due_date: Some("2026-09-01".to_string()),
        assignee_id: Some("person:1".to_string()),
        tags: vec!["backend".to_string()],
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-02T00:00:00Z".to_string(),
    };
    assert_eq!(resp.estimate_hours, Some(8.0));
    assert_eq!(resp.actual_hours, Some(3.5));
    assert_eq!(resp.status_label, "进行中");
}

// ─── 人员 DTO 测试 ───────────────────────────────────────────────────────────

#[test]
fn create_person_request() {
    let json = json!({
        "name": "张三",
        "email": "zhangsan@example.com",
        "role": "开发工程师"
    });
    let req: CreatePersonRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.name, "张三");
    assert_eq!(req.email, Some("zhangsan@example.com".to_string()));
    assert_eq!(req.role, Some("开发工程师".to_string()));
    assert!(req.avatar.is_none());
}

#[test]
fn person_workload_response() {
    let resp = PersonWorkloadResponse {
        person_id: "person:zhangsan".to_string(),
        person_name: "张三".to_string(),
        total_tasks: 5,
        todo: 2,
        in_progress: 2,
        completed: 1,
        blocked: 0,
        p0_count: 1,
        p1_count: 2,
        total_estimate_hours: 40.0,
        total_actual_hours: 25.5,
    };
    assert_eq!(resp.total_tasks, 5);
    assert_eq!(resp.p0_count + resp.p1_count, 3);
    assert_eq!(resp.total_actual_hours, 25.5);
}

// ─── 里程碑 DTO 测试 ─────────────────────────────────────────────────────────

#[test]
fn create_milestone_request() {
    let json = json!({
        "name": "V1.0 发布",
        "target_date": "2026-12-31"
    });
    let req: CreateMilestoneRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.name, "V1.0 发布");
    assert_eq!(req.target_date, "2026-12-31");
    assert!(req.description.is_none());
}

#[test]
fn milestone_response_fields() {
    let resp = MilestoneResponse {
        id: "milestone:m1".to_string(),
        name: "M1".to_string(),
        description: Some("首个里程碑".to_string()),
        target_date: "2026-09-30".to_string(),
        is_completed: false,
        completed_date: None,
        progress: 0.3,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-02T00:00:00Z".to_string(),
    };
    assert!(!resp.is_completed);
    assert_eq!(resp.progress, 0.3);
}

// ─── 问题 DTO 测试 ───────────────────────────────────────────────────────────

#[test]
fn create_issue_request_defaults() {
    let json = json!({
        "title": "Bug: 登录失败"
    });
    let req: CreateIssueRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.title, "Bug: 登录失败");
    assert_eq!(req.status, "open"); // 默认
    assert_eq!(req.risk_level, "medium"); // 默认
    assert!(req.reported_by.is_none());
}

#[test]
fn issue_response_fields() {
    let resp = IssueResponse {
        id: "issue:bug1".to_string(),
        title: "登录超时".to_string(),
        description: Some("5 分钟超时".to_string()),
        status: "open".to_string(),
        status_label: "待处理".to_string(),
        risk_level: "high".to_string(),
        risk_label: "高".to_string(),
        reported_by: Some("qa".to_string()),
        assignee_id: None,
        tags: vec!["bug".to_string()],
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-02T00:00:00Z".to_string(),
    };
    assert_eq!(resp.risk_label, "高");
    assert_eq!(resp.status_label, "待处理");
}

// ─── 文档 DTO 测试 ───────────────────────────────────────────────────────────

#[test]
fn create_document_request() {
    let json = json!({
        "title": "PRD 文档",
        "doc_type": "PRD",
        "url": "https://example.com/prd"
    });
    let req: CreateDocumentRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.title, "PRD 文档");
    assert_eq!(req.doc_type, "PRD");
    assert_eq!(req.url, Some("https://example.com/prd".to_string()));
    assert!(req.linked_to.is_none());
}

#[test]
fn document_response() {
    let resp = DocumentResponse {
        id: "doc:prd1".to_string(),
        title: "产品需求文档".to_string(),
        doc_type: "PRD".to_string(),
        url: Some("https://example.com".to_string()),
        content: None,
        author: Some("PM".to_string()),
        created_at: "2026-01-01T00:00:00Z".to_string(),
    };
    assert_eq!(resp.doc_type, "PRD");
    assert_eq!(resp.author, Some("PM".to_string()));
}

// ─── 统计 DTO 测试 ───────────────────────────────────────────────────────────

#[test]
fn project_stats_response() {
    use std::collections::HashMap;
    let mut req_by_status = HashMap::new();
    req_by_status.insert("released".to_string(), 3);
    req_by_status.insert("in_development".to_string(), 2);

    let mut task_by_status = HashMap::new();
    task_by_status.insert("completed".to_string(), 5);
    task_by_status.insert("todo".to_string(), 3);

    let resp = ProjectStatsResponse {
        project_id: "project:stats".to_string(),
        requirement_count: 5,
        task_count: 8,
        issue_count: 2,
        member_count: 4,
        progress: 0.65,
        requirements_by_status: req_by_status,
        tasks_by_status: task_by_status,
    };
    assert_eq!(resp.requirement_count, 5);
    assert_eq!(resp.task_count, 8);
    assert_eq!(resp.progress, 0.65);
    assert_eq!(resp.requirements_by_status.get("released"), Some(&3));
}

#[test]
fn impact_analysis_and_critical_path_responses() {
    let impact = ImpactAnalysisResponse {
        entity_id: "task:1".to_string(),
        affected_count: 3,
        affected_ids: vec!["task:2".into(), "task:3".into(), "project:p1".into()],
    };
    assert_eq!(impact.affected_count, 3);
    assert_eq!(impact.affected_ids.len(), 3);

    let cp = CriticalPathResponse {
        project_id: "project:cp".to_string(),
        path: vec!["t1".into(), "t2".into(), "t3".into()],
        length: 3,
    };
    assert_eq!(cp.length, 3);
    assert_eq!(cp.path.len(), 3);
}

// ─── 依赖和分配 DTO 测试 ─────────────────────────────────────────────────────

#[test]
fn dependency_and_blocker_requests() {
    let dep: AddDependencyRequest = serde_json::from_value(json!({
        "from_id": "task:a",
        "to_id": "task:b"
    })).unwrap();
    assert_eq!(dep.from_id, "task:a");
    assert_eq!(dep.to_id, "task:b");

    let blk: AddBlockerRequest = serde_json::from_value(json!({
        "blocker_id": "task:x",
        "blocked_id": "task:y"
    })).unwrap();
    assert_eq!(blk.blocker_id, "task:x");
    assert_eq!(blk.blocked_id, "task:y");
}

#[test]
fn assign_task_request() {
    let req: AssignTaskRequest = serde_json::from_value(json!({
        "task_id": "task:123",
        "person_id": "person:zhangsan"
    })).unwrap();
    assert_eq!(req.task_id, "task:123");
    assert_eq!(req.person_id, "person:zhangsan");
}

// ─── 遍历 DTO 测试 ───────────────────────────────────────────────────────────

#[test]
fn traverse_request_defaults() {
    let json = json!({
        "start_id": "project:test"
    });
    let req: TraverseRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.start_id, "project:test");
    assert_eq!(req.direction, "out"); // 默认
    assert_eq!(req.max_depth, 3); // 默认
    assert!(req.edge_types.is_none());
}

#[test]
fn traverse_response() {
    let resp = TraverseResponse {
        start_id: "project:p1".to_string(),
        vertices: vec![json!({"id": "p1", "type": "project"})],
        edges: vec![json!({"id": "e1", "type": "contains"})],
        total: 1,
    };
    assert_eq!(resp.total, 1);
    assert_eq!(resp.vertices.len(), 1);
    assert_eq!(resp.edges.len(), 1);
}

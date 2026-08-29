// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 应用状态 + 路由 + Handler
//!
//! ## API 列表
//! ```text
//! 项目：
//!   POST   /api/v1/projects                 创建项目
//!   GET    /api/v1/projects                 项目列表
//!   GET    /api/v1/projects/:id             项目详情
//!   PUT    /api/v1/projects/:id             更新项目
//!   GET    /api/v1/projects/:id/stats       项目统计
//!   GET    /api/v1/projects/:id/critical-path  关键路径
//!
//! 需求：
//!   POST   /api/v1/projects/:id/requirements  创建需求
//!   GET    /api/v1/projects/:id/requirements  需求列表
//!   GET    /api/v1/requirements/:id           需求详情
//!   PUT    /api/v1/requirements/:id           更新需求
//!   GET    /api/v1/requirements/:id/tasks     需求下的任务
//!
//! 任务：
//!   POST   /api/v1/tasks                      创建任务
//!   GET    /api/v1/tasks/:id                  任务详情
//!   PUT    /api/v1/tasks/:id                  更新任务
//!   POST   /api/v1/tasks/:id/assign           分配任务
//!
//! 人员：
//!   POST   /api/v1/persons                    创建人员
//!   GET    /api/v1/persons/:id                人员详情
//!   GET    /api/v1/persons/:id/tasks          人员任务
//!   GET    /api/v1/persons/:id/workload       人员负载
//!
//! 里程碑：
//!   POST   /api/v1/projects/:id/milestones    创建里程碑
//!   GET    /api/v1/projects/:id/milestones    里程碑列表
//!
//! 问题/风险：
//!   POST   /api/v1/projects/:id/issues        创建问题
//!   GET    /api/v1/projects/:id/issues        问题列表
//!
//! 文档：
//!   POST   /api/v1/documents                  创建文档
//!
//! 关系操作：
//!   POST   /api/v1/dependencies               添加依赖
//!   POST   /api/v1/blockers                   添加阻塞
//!
//! 图谱操作：
//!   POST   /api/v1/graph/traverse             图谱遍历
//!   GET    /api/v1/graph/impact/:id           影响分析
//!
//! 健康：
//!   GET    /health
//! ```

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post, put};
use axum::Router;
use mox_kg_core::TraverseDirection;
use serde_json::json;

use mox_project_graph_core::{
    IssueProps, IssueStatus, MilestoneProps, PersonProps, Priority,
    ProjectGraphEngine, ProjectProps, ProjectStatus,
    RequirementProps, RequirementStatus, RiskLevel, TaskProps, TaskStatus,
    DocumentProps,
};

use crate::dto::*;

// ─── 应用状态 ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<ProjectGraphEngine>,
}

impl AppState {
    pub fn new() -> Self {
        Self { engine: Arc::new(ProjectGraphEngine::new()) }
    }
}

impl Default for AppState { fn default() -> Self { Self::new() } }

// ─── 路由构建 ────────────────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router {
    Router::new()
        // 项目
        .route("/api/v1/projects", post(handle_create_project))
        .route("/api/v1/projects", get(handle_list_projects))
        .route("/api/v1/projects/:id", get(handle_get_project))
        .route("/api/v1/projects/:id", put(handle_update_project))
        .route("/api/v1/projects/:id/stats", get(handle_project_stats))
        .route("/api/v1/projects/:id/critical-path", get(handle_critical_path))
        // 需求
        .route("/api/v1/projects/:id/requirements", post(handle_create_requirement))
        .route("/api/v1/projects/:id/requirements", get(handle_list_requirements))
        .route("/api/v1/requirements/:id", get(handle_get_requirement))
        .route("/api/v1/requirements/:id", put(handle_update_requirement))
        .route("/api/v1/requirements/:id/tasks", get(handle_requirement_tasks))
        // 任务
        .route("/api/v1/tasks", post(handle_create_task))
        .route("/api/v1/tasks/:id", get(handle_get_task))
        .route("/api/v1/tasks/:id", put(handle_update_task))
        .route("/api/v1/tasks/:id/assign", post(handle_assign_task))
        // 人员
        .route("/api/v1/persons", post(handle_create_person))
        .route("/api/v1/persons/:id", get(handle_get_person))
        .route("/api/v1/persons/:id/tasks", get(handle_person_tasks))
        .route("/api/v1/persons/:id/workload", get(handle_person_workload))
        // 里程碑
        .route("/api/v1/projects/:id/milestones", post(handle_create_milestone))
        .route("/api/v1/projects/:id/milestones", get(handle_list_milestones))
        // 问题
        .route("/api/v1/projects/:id/issues", post(handle_create_issue))
        .route("/api/v1/projects/:id/issues", get(handle_list_issues))
        // 文档
        .route("/api/v1/documents", post(handle_create_document))
        // 关系
        .route("/api/v1/dependencies", post(handle_add_dependency))
        .route("/api/v1/blockers", post(handle_add_blocker))
        // 图谱
        .route("/api/v1/graph/traverse", post(handle_traverse))
        .route("/api/v1/graph/impact/:id", get(handle_impact_analysis))
        // 健康
        .route("/health", get(handle_health))
        .with_state(state)
}

// ─── 项目 Handler ────────────────────────────────────────────────────────────

async fn handle_create_project(
    State(state): State<AppState>,
    Json(req): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    let props = ProjectProps {
        name: req.name.clone(),
        code: req.code.clone(),
        description: req.description.clone(),
        status: parse_project_status(&req.status),
        priority: parse_priority(&req.priority),
        start_date: req.start_date.clone(),
        end_date: req.end_date.clone(),
        owner_id: req.owner_id.clone(),
        progress: 0.0,
        tags: req.tags.unwrap_or_default(),
        metadata: None,
    };
    let id = state.engine.create_project(props).await;
    if let Some((v, p)) = state.engine.get_project(&id).await {
        (StatusCode::CREATED, Json(ApiResponse::ok(project_to_dto(&v, &p))))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(500, "创建失败")))
    }
}

async fn handle_list_projects(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let projects = state.engine.list_projects().await;
    let resp: Vec<ProjectResponse> = projects
        .iter()
        .map(|(v, p)| project_to_dto(v, p))
        .collect();
    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

async fn handle_get_project(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.engine.get_project(&id).await {
        Some((v, p)) => (StatusCode::OK, Json(ApiResponse::ok(project_to_dto(&v, &p)))),
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "项目不存在"))),
    }
}

async fn handle_update_project(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateProjectRequest>,
) -> impl IntoResponse {
    match state.engine.get_project(&id).await {
        Some((_, mut props)) => {
            if let Some(name) = req.name { props.name = name; }
            if let Some(desc) = req.description { props.description = Some(desc); }
            if let Some(s) = req.status { props.status = parse_project_status(&s); }
            if let Some(p) = req.priority { props.priority = parse_priority(&p); }
            if req.start_date.is_some() { props.start_date = req.start_date; }
            if req.end_date.is_some() { props.end_date = req.end_date; }
            if req.owner_id.is_some() { props.owner_id = req.owner_id; }
            if let Some(tags) = req.tags { props.tags = tags; }
            state.engine.update_project(&id, props).await;
            if let Some((v, p)) = state.engine.get_project(&id).await {
                (StatusCode::OK, Json(ApiResponse::ok(project_to_dto(&v, &p))))
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(500, "更新失败")))
            }
        }
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "项目不存在"))),
    }
}

async fn handle_project_stats(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let stats = state.engine.project_stats(&id).await;
    let resp = ProjectStatsResponse {
        project_id: stats.project_id,
        requirement_count: stats.requirement_count,
        task_count: stats.task_count,
        issue_count: stats.issue_count,
        member_count: stats.member_count,
        progress: stats.progress,
        requirements_by_status: stats.requirements_by_status,
        tasks_by_status: stats.tasks_by_status,
    };
    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

async fn handle_critical_path(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let path = state.engine.critical_path(&id).await;
    let resp = CriticalPathResponse {
        project_id: id,
        length: path.len(),
        path,
    };
    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

// ─── 需求 Handler ────────────────────────────────────────────────────────────

async fn handle_create_requirement(
    Path(project_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<CreateRequirementRequest>,
) -> impl IntoResponse {
    let props = RequirementProps {
        title: req.title.clone(),
        description: req.description.clone(),
        status: parse_requirement_status(&req.status),
        priority: parse_priority(&req.priority),
        requirement_type: req.requirement_type.clone(),
        source: req.source.clone(),
        story_points: req.story_points,
        acceptance_criteria: req.acceptance_criteria.clone(),
        created_by: req.created_by.clone(),
        tags: req.tags.unwrap_or_default(),
        metadata: None,
    };
    let id = state.engine.create_requirement(&project_id, props).await;
    if let Some((v, p)) = state.engine.get_requirement(&id).await {
        (StatusCode::CREATED, Json(ApiResponse::ok(requirement_to_dto(&v, &p))))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(500, "创建失败")))
    }
}

async fn handle_list_requirements(
    Path(project_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let reqs = state.engine.list_requirements(&project_id).await;
    let resp: Vec<RequirementResponse> = reqs
        .iter()
        .map(|(v, p)| requirement_to_dto(v, p))
        .collect();
    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

async fn handle_get_requirement(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.engine.get_requirement(&id).await {
        Some((v, p)) => (StatusCode::OK, Json(ApiResponse::ok(requirement_to_dto(&v, &p)))),
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "需求不存在"))),
    }
}

async fn handle_update_requirement(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateRequirementRequest>,
) -> impl IntoResponse {
    match state.engine.get_requirement(&id).await {
        Some((_, mut props)) => {
            if let Some(t) = req.title { props.title = t; }
            if let Some(d) = req.description { props.description = Some(d); }
            if let Some(s) = req.status { props.status = parse_requirement_status(&s); }
            if let Some(p) = req.priority { props.priority = parse_priority(&p); }
            if let Some(t) = req.requirement_type { props.requirement_type = t; }
            if req.source.is_some() { props.source = req.source; }
            if req.story_points.is_some() { props.story_points = req.story_points; }
            if req.acceptance_criteria.is_some() { props.acceptance_criteria = req.acceptance_criteria; }
            if let Some(t) = req.tags { props.tags = t; }
            state.engine.update_requirement(&id, props).await;
            // 触发进度重算
            if let Some(pid) = state.engine.find_project_of_entity(&id).await {
                state.engine.recalc_project_progress(&pid).await;
            }
            if let Some((v, p)) = state.engine.get_requirement(&id).await {
                (StatusCode::OK, Json(ApiResponse::ok(requirement_to_dto(&v, &p))))
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(500, "更新失败")))
            }
        }
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "需求不存在"))),
    }
}

async fn handle_requirement_tasks(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let tasks = state.engine.list_tasks_of_requirement(&id).await;
    let resp: Vec<TaskResponse> = tasks
        .iter()
        .map(|(v, p)| task_to_dto(v, p))
        .collect();
    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

// ─── 任务 Handler ────────────────────────────────────────────────────────────

async fn handle_create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    let parent_id = match req.parent_id {
        Some(ref pid) => pid.clone(),
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::error(400, "缺少 parent_id")));
        }
    };
    let parent_type = req.parent_type.as_deref().unwrap_or("requirement");

    let props = TaskProps {
        title: req.title.clone(),
        description: req.description.clone(),
        status: parse_task_status(&req.status),
        priority: parse_priority(&req.priority),
        task_type: req.task_type.clone(),
        estimate_hours: req.estimate_hours,
        actual_hours: req.actual_hours,
        due_date: req.due_date.clone(),
        assignee_id: req.assignee_id.clone(),
        tags: req.tags.unwrap_or_default(),
        metadata: None,
    };

    let id = state.engine.create_task(&parent_id, parent_type, props).await;

    // 如果指定了负责人，建立分配关系
    if let Some(ref aid) = req.assignee_id {
        state.engine.assign_task(&id, aid).await;
    }

    if let Some((v, p)) = state.engine.get_task(&id).await {
        (StatusCode::CREATED, Json(ApiResponse::ok(task_to_dto(&v, &p))))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(500, "创建失败")))
    }
}

async fn handle_get_task(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.engine.get_task(&id).await {
        Some((v, p)) => (StatusCode::OK, Json(ApiResponse::ok(task_to_dto(&v, &p)))),
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "任务不存在"))),
    }
}

async fn handle_update_task(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateTaskRequest>,
) -> impl IntoResponse {
    match state.engine.get_task(&id).await {
        Some((_, mut props)) => {
            if let Some(t) = req.title { props.title = t; }
            if let Some(d) = req.description { props.description = Some(d); }
            if let Some(s) = req.status { props.status = parse_task_status(&s); }
            if let Some(p) = req.priority { props.priority = parse_priority(&p); }
            if let Some(t) = req.task_type { props.task_type = t; }
            if req.estimate_hours.is_some() { props.estimate_hours = req.estimate_hours; }
            if req.actual_hours.is_some() { props.actual_hours = req.actual_hours; }
            if req.due_date.is_some() { props.due_date = req.due_date; }
            if req.assignee_id.is_some() {
                props.assignee_id = req.assignee_id.clone();
            }
            if let Some(t) = req.tags { props.tags = t; }
            state.engine.update_task(&id, props).await;
            if let Some((v, p)) = state.engine.get_task(&id).await {
                (StatusCode::OK, Json(ApiResponse::ok(task_to_dto(&v, &p))))
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(500, "更新失败")))
            }
        }
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "任务不存在"))),
    }
}

async fn handle_assign_task(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<AssignTaskRequest>,
) -> impl IntoResponse {
    state.engine.assign_task(&id, &req.person_id).await;
    (StatusCode::OK, Json(ApiResponse::ok(true)))
}

// ─── 人员 Handler ────────────────────────────────────────────────────────────

async fn handle_create_person(
    State(state): State<AppState>,
    Json(req): Json<CreatePersonRequest>,
) -> impl IntoResponse {
    let props = PersonProps {
        name: req.name.clone(),
        email: req.email.clone(),
        role: req.role.clone(),
        avatar: req.avatar.clone(),
        department: req.department.clone(),
        metadata: None,
    };
    let id = state.engine.create_person(props).await;
    if let Some((v, p)) = state.engine.get_person(&id).await {
        (StatusCode::CREATED, Json(ApiResponse::ok(person_to_dto(&v, &p))))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(500, "创建失败")))
    }
}

async fn handle_get_person(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.engine.get_person(&id).await {
        Some((v, p)) => (StatusCode::OK, Json(ApiResponse::ok(person_to_dto(&v, &p)))),
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "人员不存在"))),
    }
}

async fn handle_person_tasks(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let tasks = state.engine.list_person_tasks(&id, None).await;
    let resp: Vec<TaskResponse> = tasks
        .iter()
        .map(|(v, p)| task_to_dto(v, p))
        .collect();
    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

async fn handle_person_workload(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let wl = state.engine.person_workload(&id).await;
    let name = state.engine.get_person(&id).await
        .map(|(_, p)| p.name)
        .unwrap_or_else(|| id.clone());
    let resp = PersonWorkloadResponse {
        person_id: wl.person_id,
        person_name: name,
        total_tasks: wl.total_tasks,
        todo: wl.todo,
        in_progress: wl.in_progress,
        completed: wl.completed,
        blocked: wl.blocked,
        p0_count: wl.p0_count,
        p1_count: wl.p1_count,
        total_estimate_hours: wl.total_estimate_hours,
        total_actual_hours: wl.total_actual_hours,
    };
    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

// ─── 里程碑 Handler ──────────────────────────────────────────────────────────

async fn handle_create_milestone(
    Path(project_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<CreateMilestoneRequest>,
) -> impl IntoResponse {
    let props = MilestoneProps {
        name: req.name.clone(),
        description: req.description.clone(),
        target_date: req.target_date,
        is_completed: false,
        completed_date: None,
        progress: 0.0,
        metadata: None,
    };
    let id = state.engine.create_milestone(&project_id, props).await;
    if let Some((v, p)) = state.engine.get_milestone(&id).await {
        (StatusCode::CREATED, Json(ApiResponse::ok(milestone_to_dto(&v, &p))))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(500, "创建失败")))
    }
}

async fn handle_list_milestones(
    Path(project_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use mox_project_graph_core::edge_types;
    // P1: 直接从 contains 边里找 milestone 类型的节点
    let edges = state.engine.get_out_edges_for_svc(&project_id, Some(edge_types::CONTAINS)).await;
    let mut result = Vec::new();
    for e in edges {
        if e.target.starts_with("milestone:") {
            if let Some((v, p)) = state.engine.get_milestone(&e.target).await {
                result.push(milestone_to_dto(&v, &p));
            }
        }
    }
    (StatusCode::OK, Json(ApiResponse::ok(result)))
}

// ─── 问题 Handler ────────────────────────────────────────────────────────────

async fn handle_create_issue(
    Path(project_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<CreateIssueRequest>,
) -> impl IntoResponse {
    let props = IssueProps {
        title: req.title.clone(),
        description: req.description.clone(),
        status: parse_issue_status(&req.status),
        risk_level: parse_risk_level(&req.risk_level),
        reported_by: req.reported_by.clone(),
        assignee_id: req.assignee_id.clone(),
        tags: req.tags.unwrap_or_default(),
        metadata: None,
    };
    let id = state.engine.create_issue(&project_id, props).await;
    if let Some(ref related) = req.related_to {
        state.engine.link_issue_to(&id, related).await;
    }
    // P1: 简化返回
    (StatusCode::CREATED, Json(ApiResponse::ok(json!({"id": id}))))
}

async fn handle_list_issues(
    Path(project_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use mox_project_graph_core::edge_types;
    let edges = state.engine.get_in_edges_for_svc(&project_id, Some(edge_types::BELONGS_TO)).await;
    let mut result = Vec::new();
    for e in edges {
        if e.source.starts_with("issue:") {
            // P2: 完整 IssueResponse
            result.push(serde_json::json!({"id": e.source}));
        }
    }
    (StatusCode::OK, Json(ApiResponse::ok(result)))
}

// ─── 文档 Handler ────────────────────────────────────────────────────────────

async fn handle_create_document(
    State(state): State<AppState>,
    Json(req): Json<CreateDocumentRequest>,
) -> impl IntoResponse {
    let props = DocumentProps {
        title: req.title.clone(),
        doc_type: req.doc_type.clone(),
        url: req.url.clone(),
        content: req.content.clone(),
        author: req.author.clone(),
        metadata: None,
    };
    let id = state.engine.create_document(props).await;
    if let Some(ref target) = req.linked_to {
        state.engine.link_document_to(&id, target).await;
    }
    (StatusCode::CREATED, Json(ApiResponse::ok(json!({"id": id, "title": req.title}))))
}

// ─── 关系 Handler ────────────────────────────────────────────────────────────

async fn handle_add_dependency(
    State(state): State<AppState>,
    Json(req): Json<AddDependencyRequest>,
) -> impl IntoResponse {
    state.engine.add_dependency(&req.from_id, &req.to_id).await;
    (StatusCode::OK, Json(ApiResponse::ok(true)))
}

async fn handle_add_blocker(
    State(state): State<AppState>,
    Json(req): Json<AddBlockerRequest>,
) -> impl IntoResponse {
    state.engine.add_blocker(&req.blocker_id, &req.blocked_id).await;
    (StatusCode::OK, Json(ApiResponse::ok(true)))
}

// ─── 图谱 Handler ────────────────────────────────────────────────────────────

async fn handle_traverse(
    State(state): State<AppState>,
    Json(req): Json<TraverseRequest>,
) -> impl IntoResponse {
    let direction = match req.direction.as_str() {
        "in" => TraverseDirection::In,
        "both" => TraverseDirection::Both,
        _ => TraverseDirection::Out,
    };
    let result = state.engine.traverse(
        &req.start_id,
        direction,
        req.edge_types,
        req.max_depth,
    ).await;

    let vertices: Vec<serde_json::Value> = result.vertices
        .iter()
        .map(|v| serde_json::json!({
            "id": v.id,
            "type": v.vertex_type,
            "properties": v.properties,
        }))
        .collect();
    let edges: Vec<serde_json::Value> = result.edges
        .iter()
        .map(|e| serde_json::json!({
            "id": e.id,
            "type": e.edge_type,
            "source": e.source,
            "target": e.target,
            "properties": e.properties,
        }))
        .collect();

    let resp = TraverseResponse {
        start_id: req.start_id,
        vertices,
        edges,
        total: result.total,
    };
    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

async fn handle_impact_analysis(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let affected = state.engine.analyze_impact(&id).await;
    let resp = ImpactAnalysisResponse {
        entity_id: id,
        affected_count: affected.len(),
        affected_ids: affected,
    };
    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

// ─── 健康检查 ────────────────────────────────────────────────────────────────

async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok", "service": "mox-project-graph-svc" })))
}

// ─── DTO 转换辅助 ────────────────────────────────────────────────────────────

fn project_to_dto(v: &mox_kg_core::Vertex, p: &ProjectProps) -> ProjectResponse {
    ProjectResponse {
        id: v.id.clone(),
        name: p.name.clone(),
        code: p.code.clone(),
        description: p.description.clone(),
        status: format!("{:?}", p.status).to_lowercase(),
        status_label: p.status.label().into(),
        priority: format!("{:?}", p.priority).to_uppercase(),
        priority_label: p.priority.label().into(),
        start_date: p.start_date.clone(),
        end_date: p.end_date.clone(),
        owner_id: p.owner_id.clone(),
        progress: p.progress,
        tags: p.tags.clone(),
        created_at: v.created_at.clone(),
        updated_at: v.updated_at.clone(),
    }
}

fn requirement_to_dto(v: &mox_kg_core::Vertex, p: &RequirementProps) -> RequirementResponse {
    RequirementResponse {
        id: v.id.clone(),
        title: p.title.clone(),
        description: p.description.clone(),
        status: format!("{:?}", p.status).to_lowercase(),
        status_label: p.status.label().into(),
        priority: format!("{:?}", p.priority).to_uppercase(),
        priority_label: p.priority.label().into(),
        requirement_type: p.requirement_type.clone(),
        source: p.source.clone(),
        story_points: p.story_points,
        acceptance_criteria: p.acceptance_criteria.clone(),
        created_by: p.created_by.clone(),
        tags: p.tags.clone(),
        created_at: v.created_at.clone(),
        updated_at: v.updated_at.clone(),
    }
}

fn task_to_dto(v: &mox_kg_core::Vertex, p: &TaskProps) -> TaskResponse {
    TaskResponse {
        id: v.id.clone(),
        title: p.title.clone(),
        description: p.description.clone(),
        status: format!("{:?}", p.status).to_lowercase(),
        status_label: p.status.label().into(),
        priority: format!("{:?}", p.priority).to_uppercase(),
        priority_label: p.priority.label().into(),
        task_type: p.task_type.clone(),
        estimate_hours: p.estimate_hours,
        actual_hours: p.actual_hours,
        due_date: p.due_date.clone(),
        assignee_id: p.assignee_id.clone(),
        tags: p.tags.clone(),
        created_at: v.created_at.clone(),
        updated_at: v.updated_at.clone(),
    }
}

fn person_to_dto(v: &mox_kg_core::Vertex, p: &PersonProps) -> PersonResponse {
    PersonResponse {
        id: v.id.clone(),
        name: p.name.clone(),
        email: p.email.clone(),
        role: p.role.clone(),
        avatar: p.avatar.clone(),
        department: p.department.clone(),
        created_at: v.created_at.clone(),
    }
}

fn milestone_to_dto(v: &mox_kg_core::Vertex, p: &MilestoneProps) -> MilestoneResponse {
    MilestoneResponse {
        id: v.id.clone(),
        name: p.name.clone(),
        description: p.description.clone(),
        target_date: p.target_date.clone(),
        is_completed: p.is_completed,
        completed_date: p.completed_date.clone(),
        progress: p.progress,
        created_at: v.created_at.clone(),
        updated_at: v.updated_at.clone(),
    }
}

// ─── 解析辅助 ────────────────────────────────────────────────────────────────

fn parse_project_status(s: &str) -> ProjectStatus {
    match s.to_lowercase().as_str() {
        "planning" | "规划中" => ProjectStatus::Planning,
        "in_progress" | "进行中" | "active" => ProjectStatus::InProgress,
        "paused" | "已暂停" => ProjectStatus::Paused,
        "completed" | "已完成" | "done" => ProjectStatus::Completed,
        "cancelled" | "已取消" | "canceled" => ProjectStatus::Cancelled,
        _ => ProjectStatus::Planning,
    }
}

fn parse_requirement_status(s: &str) -> RequirementStatus {
    match s.to_lowercase().as_str() {
        "pending_review" | "待评审" => RequirementStatus::PendingReview,
        "confirmed" | "已确认" => RequirementStatus::Confirmed,
        "in_development" | "开发中" => RequirementStatus::InDevelopment,
        "in_testing" | "测试中" => RequirementStatus::InTesting,
        "released" | "已上线" | "done" => RequirementStatus::Released,
        "rejected" | "已拒绝" => RequirementStatus::Rejected,
        _ => RequirementStatus::PendingReview,
    }
}

fn parse_task_status(s: &str) -> TaskStatus {
    match s.to_lowercase().as_str() {
        "todo" | "待办" | "pending" => TaskStatus::Todo,
        "in_progress" | "进行中" | "doing" => TaskStatus::InProgress,
        "completed" | "已完成" | "done" => TaskStatus::Completed,
        "blocked" | "已阻塞" | "阻塞" => TaskStatus::Blocked,
        "cancelled" | "已取消" => TaskStatus::Cancelled,
        _ => TaskStatus::Todo,
    }
}

fn parse_priority(s: &str) -> Priority {
    match s.to_uppercase().as_str() {
        "P0" | "紧急" | "critical" => Priority::P0,
        "P1" | "高" | "high" => Priority::P1,
        "P2" | "中" | "medium" | "normal" => Priority::P2,
        "P3" | "低" | "low" => Priority::P3,
        _ => Priority::P2,
    }
}

fn parse_issue_status(s: &str) -> IssueStatus {
    match s.to_lowercase().as_str() {
        "open" | "待处理" => IssueStatus::Open,
        "investigating" | "处理中" => IssueStatus::Investigating,
        "resolved" | "已解决" => IssueStatus::Resolved,
        "closed" | "已关闭" => IssueStatus::Closed,
        _ => IssueStatus::Open,
    }
}

fn parse_risk_level(s: &str) -> RiskLevel {
    match s.to_lowercase().as_str() {
        "low" | "低" => RiskLevel::Low,
        "medium" | "中" | "normal" => RiskLevel::Medium,
        "high" | "高" => RiskLevel::High,
        "critical" | "紧急" => RiskLevel::Critical,
        _ => RiskLevel::Medium,
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! DTO — 请求 / 响应结构体

use serde::{Deserialize, Serialize};

// ─── 通用响应 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { code: 0, message: "ok".into(), data: Some(data) }
    }
    pub fn error(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), data: None }
    }
}

// ─── 项目 DTO ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    #[serde(default = "default_project_status")]
    pub status: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub owner_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

fn default_project_status() -> String { "planning".into() }
fn default_priority() -> String { "P2".into() }

#[derive(Debug, Clone, Serialize)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub status: String,
    pub status_label: String,
    pub priority: String,
    pub priority_label: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub owner_id: Option<String>,
    pub progress: f32,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub owner_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

// ─── 需求 DTO ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRequirementRequest {
    pub title: String,
    pub description: Option<String>,
    #[serde(default = "default_req_status")]
    pub status: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_req_type")]
    pub requirement_type: String,
    pub source: Option<String>,
    pub story_points: Option<u32>,
    pub acceptance_criteria: Option<String>,
    pub created_by: Option<String>,
    pub tags: Option<Vec<String>>,
}

fn default_req_status() -> String { "pending_review".into() }
fn default_req_type() -> String { "功能需求".into() }

#[derive(Debug, Clone, Serialize)]
pub struct RequirementResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub status_label: String,
    pub priority: String,
    pub priority_label: String,
    pub requirement_type: String,
    pub source: Option<String>,
    pub story_points: Option<u32>,
    pub acceptance_criteria: Option<String>,
    pub created_by: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRequirementRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub requirement_type: Option<String>,
    pub source: Option<String>,
    pub story_points: Option<u32>,
    pub acceptance_criteria: Option<String>,
    pub tags: Option<Vec<String>>,
}

// ─── 任务 DTO ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    #[serde(default = "default_task_status")]
    pub status: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_task_type")]
    pub task_type: String,
    pub estimate_hours: Option<f32>,
    pub actual_hours: Option<f32>,
    pub due_date: Option<String>,
    pub assignee_id: Option<String>,
    pub tags: Option<Vec<String>>,
    /// 父节点类型：requirement / project
    pub parent_type: Option<String>,
    /// 父节点 ID
    pub parent_id: Option<String>,
}

fn default_task_status() -> String { "todo".into() }
fn default_task_type() -> String { "开发".into() }

#[derive(Debug, Clone, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub status_label: String,
    pub priority: String,
    pub priority_label: String,
    pub task_type: String,
    pub estimate_hours: Option<f32>,
    pub actual_hours: Option<f32>,
    pub due_date: Option<String>,
    pub assignee_id: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub task_type: Option<String>,
    pub estimate_hours: Option<f32>,
    pub actual_hours: Option<f32>,
    pub due_date: Option<String>,
    pub assignee_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

// ─── 人员 DTO ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePersonRequest {
    pub name: String,
    pub email: Option<String>,
    pub role: Option<String>,
    pub avatar: Option<String>,
    pub department: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonResponse {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub role: Option<String>,
    pub avatar: Option<String>,
    pub department: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonWorkloadResponse {
    pub person_id: String,
    pub person_name: String,
    pub total_tasks: usize,
    pub todo: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub blocked: usize,
    pub p0_count: usize,
    pub p1_count: usize,
    pub total_estimate_hours: f32,
    pub total_actual_hours: f32,
}

// ─── 里程碑 DTO ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMilestoneRequest {
    pub name: String,
    pub description: Option<String>,
    pub target_date: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MilestoneResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub target_date: String,
    pub is_completed: bool,
    pub completed_date: Option<String>,
    pub progress: f32,
    pub created_at: String,
    pub updated_at: String,
}

// ─── 问题 DTO ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub description: Option<String>,
    #[serde(default = "default_issue_status")]
    pub status: String,
    #[serde(default = "default_risk")]
    pub risk_level: String,
    pub reported_by: Option<String>,
    pub assignee_id: Option<String>,
    pub tags: Option<Vec<String>>,
    /// 关联的需求/任务 ID
    pub related_to: Option<String>,
}

fn default_issue_status() -> String { "open".into() }
fn default_risk() -> String { "medium".into() }

#[derive(Debug, Clone, Serialize)]
pub struct IssueResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub status_label: String,
    pub risk_level: String,
    pub risk_label: String,
    pub reported_by: Option<String>,
    pub assignee_id: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ─── 文档 DTO ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub doc_type: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    /// 关联的实体 ID（项目/需求）
    pub linked_to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentResponse {
    pub id: String,
    pub title: String,
    pub doc_type: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub created_at: String,
}

// ─── 统计 DTO ────────────────────────────────────────────────────────────────

use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatsResponse {
    pub project_id: String,
    pub requirement_count: usize,
    pub task_count: usize,
    pub issue_count: usize,
    pub member_count: usize,
    pub progress: f32,
    pub requirements_by_status: HashMap<String, usize>,
    pub tasks_by_status: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactAnalysisResponse {
    pub entity_id: String,
    pub affected_count: usize,
    pub affected_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CriticalPathResponse {
    pub project_id: String,
    pub path: Vec<String>,
    pub length: usize,
}

// ─── 依赖 DTO ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct AddDependencyRequest {
    /// 依赖方 ID（需要等对方完成）
    pub from_id: String,
    /// 被依赖方 ID
    pub to_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddBlockerRequest {
    /// 阻塞方 ID
    pub blocker_id: String,
    /// 被阻塞方 ID
    pub blocked_id: String,
}

// ─── 分配 DTO ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct AssignTaskRequest {
    pub task_id: String,
    pub person_id: String,
}

// ─── 遍历 DTO ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct TraverseRequest {
    pub start_id: String,
    #[serde(default = "default_direction")]
    pub direction: String,
    pub edge_types: Option<Vec<String>>,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

fn default_direction() -> String { "out".into() }
fn default_max_depth() -> usize { 3 }

#[derive(Debug, Clone, Serialize)]
pub struct TraverseResponse {
    pub start_id: String,
    pub vertices: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
    pub total: usize,
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! API DTO（Data Transfer Objects）
//!
//! 定义 HTTP API 的请求和响应结构。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mox_alliance_common_proto::{
    AllianceMode, FusionStrategy, TaskPriority, TaskStatus,
};

// ─── Task API ────────────────────────────────────────────────────────────────

/// 创建任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub task_type: Option<String>,
    #[serde(default)]
    pub priority: Option<TaskPriority>,
    #[serde(default)]
    pub mode: Option<AllianceMode>,
    #[serde(default)]
    pub fusion_strategy: Option<FusionStrategy>,
}

/// 创建任务响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskResponse {
    pub task_id: Uuid,
    pub title: String,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 任务详情响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetailResponse {
    pub task_id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub progress: f32,
    pub mode: AllianceMode,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<i64>,
}

/// 任务列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskDetailResponse>,
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
}

/// 任务操作请求（暂停/恢复/取消）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskActionRequest {
    pub action: TaskAction,
    #[serde(default)]
    pub reason: Option<String>,
}

/// 任务操作类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskAction {
    Pause,
    Resume,
    Cancel,
}

// ─── Execution API ───────────────────────────────────────────────────────────

/// 执行状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatusResponse {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub progress: f32,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub running_nodes: usize,
    pub failed_nodes: usize,
    pub pending_nodes: usize,
}

/// 节点详情响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDetailResponse {
    pub node_id: String,
    pub name: String,
    pub expert_id: String,
    pub status: mox_alliance_common_proto::NodeStatus,
    pub dependencies: Vec<String>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
}

/// 节点列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeListResponse {
    pub nodes: Vec<NodeDetailResponse>,
    pub total: usize,
}

// ─── Expert API ─────────────────────────────────────────────────────────────

/// 专家搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertSearchRequest {
    pub query: String,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

/// 专家搜索响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertSearchResponse {
    pub experts: Vec<ExpertSummary>,
    pub total: usize,
}

/// 专家摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertSummary {
    pub expert_id: String,
    pub name: String,
    pub description: String,
    pub domains: Vec<String>,
    pub status: mox_alliance_common_proto::ExpertStatus,
}

// ─── 通用响应 ────────────────────────────────────────────────────────────────

/// 通用成功响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

impl Default for SuccessResponse {
    fn default() -> Self {
        Self {
            success: true,
            message: "OK".to_string(),
        }
    }
}

/// 通用错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error_code: u32,
    pub message: String,
}

impl ErrorResponse {
    pub fn new(code: u32, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error_code: code,
            message: message.into(),
        }
    }
}

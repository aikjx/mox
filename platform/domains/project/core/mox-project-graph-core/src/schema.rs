// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 项目需求知识图谱 · Schema 定义
//!
//! ## 实体类型（8 类）
//! - `project`      项目
//! - `requirement`  需求
//! - `task`         任务
//! - `milestone`    里程碑
//! - `person`       人员
//! - `issue`        问题/风险
//! - `document`     文档
//! - `tag`          标签
//!
//! ## 关系类型（12 类）
//! - `contains`        项目 -[包含]-> 需求/任务/里程碑
//! - `decomposes_into` 需求 -[拆解为]-> 任务
//! - `assigned_to`     任务/需求 -[分配给]-> 人员
//! - `depends_on`      需求/任务 -[依赖]-> 需求/任务
//! - `blocks`          任务 -[阻塞]-> 任务
//! - `tracks`          里程碑 -[跟踪]-> 需求/任务
//! - `reported_by`     问题 -[由..报告]-> 人员
//! - `related_to`      问题 -[关联]-> 需求/任务
//! - `describes`       文档 -[描述]-> 需求/项目
//! - `tagged_with`     任意实体 -[打标签]-> 标签
//! - `manages`         人员 -[管理]-> 项目
//! - `belongs_to`      任务/需求 -[属于]-> 项目
//!
//! ## 状态枚举
//! - 项目状态：规划中 / 进行中 / 暂停 / 已完成 / 已取消
//! - 需求状态：待评审 / 已确认 / 开发中 / 测试中 / 已上线 / 已拒绝
//! - 任务状态：待办 / 进行中 / 已完成 / 已阻塞 / 已取消
//! - 优先级：P0 / P1 / P2 / P3
//! - 风险等级：低 / 中 / 高 / 紧急

use serde::{Deserialize, Serialize};

// ─── 实体类型常量 ────────────────────────────────────────────────────────────

pub mod entity_types {
    pub const PROJECT: &str = "project";
    pub const REQUIREMENT: &str = "requirement";
    pub const TASK: &str = "task";
    pub const MILESTONE: &str = "milestone";
    pub const PERSON: &str = "person";
    pub const ISSUE: &str = "issue";
    pub const DOCUMENT: &str = "document";
    pub const TAG: &str = "tag";
}

// ─── 关系类型常量 ────────────────────────────────────────────────────────────

pub mod edge_types {
    pub const CONTAINS: &str = "contains";
    pub const DECOMPOSES_INTO: &str = "decomposes_into";
    pub const ASSIGNED_TO: &str = "assigned_to";
    pub const DEPENDS_ON: &str = "depends_on";
    pub const BLOCKS: &str = "blocks";
    pub const TRACKS: &str = "tracks";
    pub const REPORTED_BY: &str = "reported_by";
    pub const RELATED_TO: &str = "related_to";
    pub const DESCRIBES: &str = "describes";
    pub const TAGGED_WITH: &str = "tagged_with";
    pub const MANAGES: &str = "manages";
    pub const BELONGS_TO: &str = "belongs_to";
}

// ─── 状态枚举 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Planning,
    InProgress,
    Paused,
    Completed,
    Cancelled,
}

impl ProjectStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Planning => "规划中",
            Self::InProgress => "进行中",
            Self::Paused => "已暂停",
            Self::Completed => "已完成",
            Self::Cancelled => "已取消",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    PendingReview,
    Confirmed,
    InDevelopment,
    InTesting,
    Released,
    Rejected,
}

impl RequirementStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PendingReview => "待评审",
            Self::Confirmed => "已确认",
            Self::InDevelopment => "开发中",
            Self::InTesting => "测试中",
            Self::Released => "已上线",
            Self::Rejected => "已拒绝",
        }
    }

    pub fn progress_weight(&self) -> f32 {
        match self {
            Self::PendingReview => 0.0,
            Self::Confirmed => 0.1,
            Self::InDevelopment => 0.4,
            Self::InTesting => 0.7,
            Self::Released => 1.0,
            Self::Rejected => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Completed,
    Blocked,
    Cancelled,
}

impl TaskStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Todo => "待办",
            Self::InProgress => "进行中",
            Self::Completed => "已完成",
            Self::Blocked => "已阻塞",
            Self::Cancelled => "已取消",
        }
    }

    pub fn progress_weight(&self) -> f32 {
        match self {
            Self::Todo => 0.0,
            Self::InProgress => 0.5,
            Self::Completed => 1.0,
            Self::Blocked => 0.0,
            Self::Cancelled => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

impl Priority {
    pub fn label(&self) -> &'static str {
        match self {
            Self::P0 => "紧急",
            Self::P1 => "高",
            Self::P2 => "中",
            Self::P3 => "低",
        }
    }

    pub fn weight(&self) -> u32 {
        match self {
            Self::P0 => 4,
            Self::P1 => 3,
            Self::P2 => 2,
            Self::P3 => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Low => "低",
            Self::Medium => "中",
            Self::High => "高",
            Self::Critical => "紧急",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    Open,
    Investigating,
    Resolved,
    Closed,
}

impl IssueStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Open => "待处理",
            Self::Investigating => "处理中",
            Self::Resolved => "已解决",
            Self::Closed => "已关闭",
        }
    }
}

// ─── 强类型实体属性 ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProps {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub priority: Priority,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub owner_id: Option<String>,
    pub progress: f32,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementProps {
    pub title: String,
    pub description: Option<String>,
    pub status: RequirementStatus,
    pub priority: Priority,
    pub requirement_type: String, // 功能需求 / 非功能需求 / 优化 / Bug
    pub source: Option<String>,   // 来源：客户/内部/市场
    pub story_points: Option<u32>,
    pub acceptance_criteria: Option<String>,
    pub created_by: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProps {
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: Priority,
    pub task_type: String, // 开发 / 测试 / 设计 / 调研 / 运维
    pub estimate_hours: Option<f32>,
    pub actual_hours: Option<f32>,
    pub due_date: Option<String>,
    pub assignee_id: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneProps {
    pub name: String,
    pub description: Option<String>,
    pub target_date: String,
    pub is_completed: bool,
    pub completed_date: Option<String>,
    pub progress: f32,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonProps {
    pub name: String,
    pub email: Option<String>,
    pub role: Option<String>,
    pub avatar: Option<String>,
    pub department: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueProps {
    pub title: String,
    pub description: Option<String>,
    pub status: IssueStatus,
    pub risk_level: RiskLevel,
    pub reported_by: Option<String>,
    pub assignee_id: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentProps {
    pub title: String,
    pub doc_type: String, // PRD / 设计文档 / 测试报告 / 会议纪要
    pub url: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagProps {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
}

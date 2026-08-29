// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # mox-project-graph-core · 项目需求知识图谱核心引擎
//!
//! 基于知识图谱的项目管理核心，将项目、需求、任务、人员、里程碑、问题、文档、标签
//! 建模为图谱节点，通过关系边表达依赖、分配、包含、阻塞等关联。
//!
//! ## 核心能力
//! - 项目 / 需求 / 任务 / 里程碑 / 人员 / 问题 / 文档 / 标签 的 CRUD
//! - 依赖关系管理（需求依赖、任务阻塞）
//! - 项目进度自动计算（加权平均）
//! - 影响范围分析（变更传播链）
//! - 人员负载分析
//! - 关键路径识别
//! - 图谱遍历查询
//!
//! ## 架构
//! ```text
//! schema.rs    实体类型 / 关系类型 / 状态枚举 / 属性结构体
//! engine.rs    项目图谱引擎（领域操作封装）
//! ```

pub mod schema;
pub mod engine;

// ─── 重导出 ──────────────────────────────────────────────────────────────────

pub use schema::{
    entity_types, edge_types,
    ProjectStatus, RequirementStatus, TaskStatus,
    Priority, RiskLevel, IssueStatus,
    ProjectProps, RequirementProps, TaskProps,
    MilestoneProps, PersonProps, IssueProps,
    DocumentProps, TagProps,
};

pub use engine::{
    ProjectGraphEngine, ProjectStats, PersonWorkload,
};

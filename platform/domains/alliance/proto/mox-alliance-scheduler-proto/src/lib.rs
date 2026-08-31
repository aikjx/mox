// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Scheduler Proto — 调度器协议层
//!
//! 联盟调度器的接口契约定义，包括：
//! - 任务调度接口
//! - 专家匹配接口
//! - 协作计划生成接口
//!
//! ## 设计原则
//! - **DIP 依赖倒置**：scheduler-core 依赖本 crate 的 trait 抽象
//! - **SSOT 单一真相源**：调度器的接口契约只有这里一个权威定义
//! - **协议先行**：先定义接口，再实现

pub mod matcher;
pub mod scheduler;
pub mod types;

// ─── 重导出 ────────────────────────────────────────────────────────────────

pub use matcher::{ExpertMatchQuery, ExpertMatchResult, ExpertMatcher, MatchScoreBreakdown, MatchedExpert};
pub use scheduler::{TaskScheduler, TaskSubmitRequest, TaskSubmitResponse};
pub use types::{PlanGenerationRequest, PlanGenerationResponse, SchedulerConfig};

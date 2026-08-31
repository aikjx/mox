// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Scheduler Service — 联盟调度器服务
//!
//! 专家联盟调度器的服务层：
//! - HTTP API（任务管理、专家匹配、计划生成）
//! - 任务队列管理
//! - 与执行器对接（通过 ExecutorBridge）
//!
//! Phase 1：基础 HTTP API + 内存调度器
//! Phase 2：执行器桥接层（支持 HTTP 远程和进程内调用）

pub mod app_state;
pub mod routes;
pub mod server;

pub use app_state::SchedulerAppState;
pub use server::{SchedulerMode, SchedulerServer};

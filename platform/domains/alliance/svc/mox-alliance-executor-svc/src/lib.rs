// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Executor Service — 联盟执行器服务
//!
//! 专家联盟执行器的服务层：
//! - HTTP API（执行状态查询、节点管理、人工干预）
//! - DAG 执行引擎运行
//! - 节点执行调度
//!
//! Phase 1：基础 HTTP API + 内存执行引擎 + Mock 执行器

pub mod app_state;
pub mod routes;
pub mod server;

pub use app_state::ExecutorAppState;
pub use server::ExecutorServer;

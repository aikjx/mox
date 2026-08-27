// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! HTTP + WebSocket Handlers 模块
//!
//! 导出：
//! - `governance` — Dashboard / Audit / Config / Veto / WebSocket handlers
//! - `hitl` — 人机协同（HITL）WebSocket 审批端点
//! - `agent` — AI Agent 引擎任务执行端点

pub mod agent;
pub mod ai_engine;
pub mod governance;
pub mod hitl;

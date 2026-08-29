// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # mox-project-graph-svc · 项目需求知识图谱 HTTP 服务
//!
//! 提供项目/需求/任务/人员/里程碑/问题/文档 的图谱化管理接口。

pub mod dto;
pub mod server;

pub use server::{AppState, router};

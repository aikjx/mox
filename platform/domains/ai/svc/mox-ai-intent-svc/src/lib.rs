// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # mox-ai-intent-svc · AI 对话意图理解服务
//!
//! 提供 HTTP 接口：
//! - 端到端意图理解（分类 + 实体 + 拆解 + Agent匹配 + 协同建议）
//! - 实体提取
//! - 任务拆解
//! - 会话管理
//! - 内置意图定义查询
//!
//! 核心能力来自 [`mox_ai_intent_core`]，本服务仅做 HTTP 封装。

pub mod dto;
pub mod server;

pub use server::{AppState, router};

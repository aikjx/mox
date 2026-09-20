// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家联盟网关内联模块
//!
//! 本目录收拢原网关根目录下 9 个 `experts_*.rs` 模块，承担：
//! - 专家注册 CRUD 与持久化（registry / db）
//! - 协作编排与会话管理（collaboration / orchestration / dispatcher / session）
//! - 图谱关联查询（graph）
//! - 共享状态与扩展点（common / ext）
//!
//! 与 scheduler-svc(3100) / executor-svc(3200) 的关系：
//! 本层是**用户对话与会话层**，负责接收请求、管理会话、转发调度；
//! 纯算法与执行在 svc 层完成。

// 兼容导出：原 alliance.rs 重导出 alliance HTTP SDK
pub use mox_alliance_http_sdk::alliance::*;

pub mod experts_common;
pub mod experts_collaboration;
pub mod experts_db;
pub mod experts_ext;
pub mod experts_session;
pub mod experts_graph;
pub mod experts_registry;
pub mod experts_orchestration;
pub mod experts_dispatcher;
pub mod registry_client;

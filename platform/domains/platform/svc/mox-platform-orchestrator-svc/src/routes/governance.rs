// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # 治理台 API 路由
//!
//! 挂载 `/api/governance/*` 下的全部 REST + WebSocket 端点。
//! 使用 `GovernanceState` 作为状态，独立于主 AppState。
//!
//! ## 路由清单
//!
//! | 方法 | 路径 | Handler | 权限 | 说明 |
//! |------|------|---------|------|------|
//! | GET | /api/governance/dashboard | `dashboard_handler` | `governance:read` | 实时监控面板 |
//! | GET | /api/governance/experts/status | `experts_status_handler` | `governance:read` | 专家执行状态 |
//! | GET | /api/governance/veto/events | `veto_events_handler` | `governance:read` | 否决事件列表 |
//! | GET | /api/governance/audit/logs | `audit_logs_handler` | `governance:audit_read` | 审计日志 |
//! | GET | /api/governance/config/rbac | `get_rbac_config_handler` | `governance:read` | RBAC 配置查询 |
//! | PUT | /api/governance/config/rbac | `update_rbac_config_handler` | `governance:config_write` | RBAC 配置更新 |
//! | GET | /api/governance/config/experts | `get_expert_config_handler` | `governance:read` | 专家权重阈值配置 |
//! | PUT | /api/governance/config/experts | `update_expert_config_handler` | `governance:config_write` | 专家配置更新 |
//! | GET | /api/governance/ws | `governance_ws_handler` | 独立 token 校验 | WebSocket 实时推送 |
//! | POST | /api/governance/assess | `assess_handler` | `governance:assess` | 触发治理评估 |

use crate::handlers::governance as gov;
use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;

/// 返回治理台 API 路由树
///
/// 使用 `Arc<gov::GovernanceState>` 作为状态，独立于主 AppState。
/// 在 main.rs 中通过 `.nest("/api/governance", governance_routes().with_state(state.governance.clone()))` 挂载。
pub fn governance_routes() -> Router<Arc<gov::GovernanceState>> {
    Router::new()
        // ========== 监控面板 ==========
        .route("/dashboard", get(gov::dashboard_handler))
        // ========== 专家状态 ==========
        .route("/experts/status", get(gov::experts_status_handler))
        // ========== 否决事件 ==========
        .route("/veto/events", get(gov::veto_events_handler))
        // ========== 审计日志 ==========
        .route("/audit/logs", get(gov::audit_logs_handler))
        // ========== RBAC 配置 ==========
        .route("/config/rbac", get(gov::get_rbac_config_handler))
        .route("/config/rbac", put(gov::update_rbac_config_handler))
        // ========== 专家配置 ==========
        .route("/config/experts", get(gov::get_expert_config_handler))
        .route("/config/experts", put(gov::update_expert_config_handler))
        // ========== 实时推送 ==========
        .route("/ws", get(gov::governance_ws_handler))
        // ========== 治理评估 ==========
        .route("/assess", post(gov::assess_handler))
}

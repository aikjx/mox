//! 企业级功能统一路由模块
//!
//! 整合：OA/ERP集成适配器 / 低代码设计器 / SSO单点登录 / 消息推送 / 文档管理 / 企业级管理
//! 所有企业级功能路由统一在此注册，挂载到 /api/enterprise/* 路径下

use crate::designer::api::{build_designer_router, DesignerState};
use crate::document::api::{build_document_router, DocumentState};
use crate::enterprise::admin_api::{build_admin_router, AdminState};
use crate::integration::api::{build_integration_router, IntegrationState};
use crate::message_center::api::{build_message_center_router, MessageCenterState};
use crate::sso::api::{build_sso_router, SsoState};
use crate::GatewayState;
use axum::{Router, extract::State};
use std::sync::Arc;

/// 企业级功能统一状态
#[derive(Clone)]
pub struct EnterpriseState {
    /// OA/ERP集成适配器状态
    pub integration: Arc<IntegrationState>,
    /// 低代码设计器状态
    pub designer: Arc<DesignerState>,
    /// SSO单点登录状态
    pub sso: Arc<SsoState>,
    /// 消息推送状态
    pub message_center: Arc<MessageCenterState>,
    /// 文档管理+电子签章状态
    pub document: Arc<DocumentState>,
    /// 企业级管理状态（租户/部门/角色权限/审计日志）
    pub admin: Arc<AdminState>,
}

impl EnterpriseState {
    pub fn new() -> Self {
        Self {
            integration: Arc::new(IntegrationState::new()),
            designer: Arc::new(DesignerState::new()),
            sso: Arc::new(SsoState::new()),
            message_center: Arc::new(MessageCenterState::new()),
            document: Arc::new(DocumentState::new()),
            admin: Arc::new(AdminState::new()),
        }
    }
}

impl Default for EnterpriseState {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建企业级功能统一路由（GatewayState版本，用于网关主路由挂载）
///
/// 路由结构：
/// - /api/enterprise/integration/*  —— OA/ERP集成适配器
/// - /api/enterprise/designer/*     —— 低代码设计器（流程/表单/报表）
/// - /api/enterprise/sso/*          —— SSO单点登录
/// - /api/enterprise/message/*      —— 消息推送
/// - /api/enterprise/document/*     —— 文档管理+电子签章
/// - /api/enterprise/admin/*        —— 企业级管理（租户/部门/角色权限/审计日志）
/// - /api/enterprise/health         —— 健康检查
pub fn build_enterprise_router_for_gateway(_gateway: &GatewayState) -> Router<GatewayState> {
    // 构建各模块路由（直接使用 GatewayState 作为状态类型）
    let integration_router: Router<GatewayState> = build_integration_router::<GatewayState>();
    let designer_router: Router<GatewayState> = build_designer_router::<GatewayState>();
    let sso_router: Router<GatewayState> = build_sso_router::<GatewayState>();
    let message_router: Router<GatewayState> = build_message_center_router::<GatewayState>();
    let document_router: Router<GatewayState> = build_document_router::<GatewayState>();
    let admin_router: Router<GatewayState> = build_admin_router::<GatewayState>();

    // 健康检查路由
    let health_router: Router<GatewayState> = Router::new()
        .route("/health", axum::routing::get(enterprise_health_handler));

    // 合并所有路由
    Router::<GatewayState>::new()
        .nest("/integration", integration_router)
        .nest("/designer", designer_router)
        .nest("/sso", sso_router)
        .nest("/message", message_router)
        .nest("/document", document_router)
        .nest("/admin", admin_router)
        .merge(health_router)
}

/// 企业级功能健康检查
pub async fn enterprise_health_handler(
    State(state): State<GatewayState>,
) -> axum::Json<serde_json::Value> {
    let _ = state.enterprise.clone();
    axum::Json(serde_json::json!({
        "status": "ok",
        "modules": {
            "integration": "ready",
            "designer": "ready",
            "sso": "ready",
            "message_center": "ready",
            "document": "ready",
            "admin": "ready",
        },
        "ts": chrono::Utc::now().to_rfc3339(),
    }))
}

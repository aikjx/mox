//! 企业级功能统一路由模块
//!
//! 整合：OA/ERP集成适配器 / 低代码设计器 / SSO单点登录 / 消息推送 / 文档管理 / 企业级管理
//! 所有企业级功能路由统一在此注册，挂载到 /api/enterprise/* 路径下

use crate::system_config::api::{build_system_config_router, ConfigState};
use crate::dictionary::api::{build_dictionary_router, DictionaryState};
use crate::operation_log::api::{build_operation_log_router, OperationLogState};
use crate::file_storage::api::{build_file_storage_router, FileStorageState};
use crate::organization::api::{build_organization_router, OrganizationState};
use crate::mailer::api::{build_mailer_router, MailerState};
use crate::api_permission::ApiPermissionState;
use crate::api_permission::api::build_api_permission_router;
use crate::batch_operation::TemplateState;
use crate::batch_operation::api::build_batch_operation_router;
use crate::designer::api::{build_designer_router, DesignerState};
use crate::document::api::{build_document_router, DocumentState};
use crate::enterprise::admin_api::{build_admin_router, AdminState};
use crate::integration::api::{build_integration_router, IntegrationState};
use crate::message_center::api::{build_message_center_router, MessageCenterState};
use crate::scheduler::api::{build_scheduler_router, SchedulerState};
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
    /// 定时任务调度状态
    pub scheduler: Arc<SchedulerState>,
    /// 系统配置管理状态
    pub system_config: Arc<ConfigState>,
    pub dictionary: Arc<DictionaryState>,
    pub operation_log: Arc<OperationLogState>,
    pub file_storage: Arc<FileStorageState>,
    pub organization: Arc<OrganizationState>,
    pub mailer: Arc<MailerState>,
    pub api_permission: Arc<ApiPermissionState>,
    pub batch_operation: Arc<TemplateState>,
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
            scheduler: Arc::new(SchedulerState::new()),
            system_config: Arc::new(ConfigState::new()),
            dictionary: Arc::new(DictionaryState::new()),
            operation_log: Arc::new(OperationLogState::new()),
            file_storage: Arc::new(FileStorageState::new()),
            organization: Arc::new(OrganizationState::new()),
            mailer: Arc::new(MailerState::new()),
            api_permission: Arc::new(ApiPermissionState::new()),
            batch_operation: Arc::new(TemplateState::new()),
        }
    }

    /// 异步初始化：自动注册API端点和标准权限点
    pub async fn init(&self) {
        use crate::api_permission::middleware::{auto_register_endpoints, init_standard_permissions};
        init_standard_permissions(&self.api_permission).await;
        auto_register_endpoints(&self.api_permission).await;
        println!("[企业级平台] API权限中心初始化完成");
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
    let scheduler_router: Router<GatewayState> = build_scheduler_router::<GatewayState>();
    let config_router: Router<GatewayState> = build_system_config_router::<GatewayState>();
    let dictionary_router: Router<GatewayState> = build_dictionary_router::<GatewayState>();
    let mailer_router: Router<GatewayState> = build_mailer_router::<GatewayState>();
    let organization_router: Router<GatewayState> = build_organization_router::<GatewayState>();
    let file_storage_router: Router<GatewayState> = build_file_storage_router::<GatewayState>();
    let operation_log_router: Router<GatewayState> = build_operation_log_router::<GatewayState>();

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
        .nest("/scheduler", scheduler_router)
        .nest("/config", config_router)
        .nest("/dictionary", dictionary_router)
        .nest("/mailer", mailer_router)
        .nest("/org", organization_router)
        .nest("/files", file_storage_router)
        .nest("/operation-logs", operation_log_router)
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
            "scheduler": "ready",
            "system_config": "ready",
            "dictionary": "ready",
            "operation_log": "ready",
            "file_storage": "ready",
            "organization": "ready",
            "mailer": "ready",
        },
        "ts": chrono::Utc::now().to_rfc3339(),
    }))
}

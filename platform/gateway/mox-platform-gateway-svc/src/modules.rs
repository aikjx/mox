// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 网关模块装配层（企业级布局归一化）
//!
//! 本模块是网关「布局归一化」的单一落点，承担三件事：
//!
//! 1. **状态注册中心**（`ModuleStates`）
//!    跨多个路由模块共享的状态实例统一在此构造一次，再按依赖顺序注入各域路由，
//!    杜绝「同一份状态被不同模块各建一份」造成的数据分裂
//!    （典型反例：专家收藏集合曾同时存在于共享态与广场扩展域）。
//!
//! 2. **装配单一入口**（`build_module_routers`）
//!    全部进程内业务域路由在此集中装配，并统一挂载鉴权层；
//!    `lib.rs` 只保留中间件分层与进程生命周期，不再散落状态创建与 merge 调用。
//!
//! 3. **状态类型升级归一**（`upgrade`）
//!    axum 0.7 中自包含 `Router<()>` 需手动升级为 `Router<GatewayState>`，
//!    此前该转换在装配处重复 17 次，现统一由 `upgrade` 承担。
//!
//! ## 装配顺序约定
//! 先领域外挂（KG/AI/KB）→ 联盟域 → 系统/安全域 → 兜底反代 → 各业务域 → 专家联盟域。
//! 兜底反代 `business_proxy` 必须排在具体域之后：axum 按路由具体度匹配，
//! 具体路由优先命中，未命中的 `/api/*` 才落入代理。

use std::sync::Arc;

use axum::{
    Router,
    extract::Request,
    middleware::{Next, from_fn},
};

use crate::{
    GatewayState,
    actuator::{LogStore, RuntimeMetrics},
    alliance::{self, experts_collaboration, experts_common, experts_dispatcher, experts_ext,
        experts_graph, experts_orchestration, experts_registry, experts_session, experts_streams},
    cloud::CloudState,
    dialogue_sediment, misc,
    monitor, notification, projects_ext, proxy, system, workspace,
};
use crate::auth::{AuthMiddleware, auth_middleware};

/// 模块状态注册中心
///
/// 网关全部业务域状态在此统一构造、统一持有、按依赖顺序注入各域路由，
/// 生命周期与网关进程一致。
///
/// 归一化前：各模块在 `build_*_router()` 内部自行 `new` 状态，创建时机分散、
/// 无法统一观测与托管，跨模块共享状态还存在「各建一份」的数据分裂风险。
/// 归一化后：状态在注册中心一次性构造，路由构建器只接收状态、不负责创建。
#[derive(Clone)]
pub struct ModuleStates {
    /// 专家联盟全域共享状态
    ///
    /// 被注册中心 / 智能协作 / 会话 / 调度 / 图谱 / 编排 / 广场扩展 共 7 个路由模块共用，
    /// 保证注册表、会话、图谱、收藏等数据的唯一真源。
    pub experts: Arc<experts_common::ExpertsSharedState>,
    /// 监控域状态（告警规则 + 运行时指标/在线日志缓冲引用）
    pub monitor: Arc<monitor::MonitorState>,
    /// 工作区域状态（操作历史）
    pub workspace: Arc<workspace::WorkspaceState>,
    /// 项目扩展域状态（项目文件 + 收藏）
    pub projects: Arc<projects_ext::ProjectsState>,
    /// 杂项域状态（任务 / 项目列表）
    pub misc: Arc<misc::MiscState>,
    /// 知识库扩展域状态（文档-实体关联）
    /// 通知域状态（通知列表）
    pub notification: Arc<notification::NotificationState>,
    /// 知识库域共享状态（mox-kb-svc 唯一真源；KB 路由与对话沉淀共用）
    pub kb: Arc<mox_kb_svc::KbState>,
    /// 云盘共享状态（本地对象存储根；Cloud 路由与对话沉淀共用）
    pub cloud: CloudState,
    /// 对话沉淀状态（对话核心内容 → 知识图谱 / 云盘 / 知识库）
    pub sediment: dialogue_sediment::SedimentState,
}

impl ModuleStates {
    /// 构造注册中心
    ///
    /// - `runtime` / `logs`：来自 `GatewayState` 的运行时指标与在线日志缓冲，供监控域复用；
    ///   注册中心仅持有 `Arc` 引用，监控域与 Actuator 管理面共享同一份数据。
    /// - 各域状态构造时读取各自的 JSON 持久化文件；专家共享状态还会完成
    ///   启动期 JSON→SQLite 一次性迁移、内置专家种子化、能力图谱首次构建（均幂等）。
    pub fn new(runtime: Arc<RuntimeMetrics>, logs: Arc<LogStore>, iam: Arc<mox_platform_iam_core::IamRepository>) -> Self {
        Self::with_inbox(
            runtime,
            logs,
            iam,
            Arc::new(crate::message_center::api::MessageCenterState::new()),
        )
    }

    pub fn with_inbox(
        runtime: Arc<RuntimeMetrics>,
        logs: Arc<LogStore>,
        iam: Arc<mox_platform_iam_core::IamRepository>,
        inbox: Arc<crate::message_center::api::MessageCenterState>,
    ) -> Self {
        let experts = Arc::new(experts_common::ExpertsSharedState::new());
        let kb = Arc::new(mox_kb_svc::KbState::from_env());
        let cloud = CloudState::new();
        Self {
            experts: experts.clone(),
            monitor: Arc::new(monitor::MonitorState::new(runtime, logs, iam, experts.clone())),
            workspace: Arc::new(workspace::WorkspaceState::new()),
            projects: Arc::new(projects_ext::ProjectsState::new()),
            misc: Arc::new(misc::MiscState::new()),
            notification: Arc::new(notification::NotificationState::with_inbox(inbox)),
            kb: kb.clone(),
            cloud: cloud.clone(),
            sediment: dialogue_sediment::SedimentState::new(kb, cloud, experts.clone()),
        }
    }
}

/// 状态类型升级：自包含 `Router<()>` → `Router<S>`
///
/// axum 0.7 无 `From<Router<()>> for Router<S>`，需以空元组填充原状态位后重填目标状态。
/// 统一由此函数承担，消除装配处的重复类型转换。
pub fn upgrade<S>(router: Router<()>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.with_state(())
}

/// 装配全部进程内业务域路由，并统一挂载鉴权层
///
/// 返回的 `Router<GatewayState>` 即「受保护路由组」，
/// 由 `lib.rs` 与公开的 Actuator/L0 端点合并后再套限流、CORS、可观测中间件。
pub fn build_module_routers(states: &ModuleStates, gateway: &GatewayState) -> Router<GatewayState> {
    let experts = states.experts.clone();

    let modules = Router::<GatewayState>::new()
        // —— 领域外挂：KG + AI 引擎（mox-kg-service-svc 自包含） ——
        .merge(upgrade(crate::http_adapter::build_kg_ai_router()))
        // —— 知识库域（mox-kb-svc） ——
        // 前缀归一化（RC-1）：内部注册 /kb/*，对外统一暴露 /api/kb/*。
        // 采用 nest 包装而非直接 merge，避免破坏 mox-kb-svc 自身的 /kb/* 集成测试。
        .merge(upgrade(Router::new().nest(
            "/api",
            protected_kb_router(states.kb.clone()),
        )))
        // —— 联盟任务域（远程优先 + 本地降级）——
        .merge(upgrade(alliance::build_alliance_router()))
        // —— 系统管理 + 安全域（IAM SQLite 真实链路，已回收为受保护路由）——
        .merge(system::build_system_router(gateway.iam.clone()))
        .merge(system::build_security_router())
        // —— 业务域兜底反代（必须排在具体域之后）——
        
        // —— RBAC 域（L1 /rbac/v1/* · IAM 真实仓储：角色/权限/当前用户）——
        .merge(crate::rbac::build_rbac_router())
        // —— Voice 域（L7 /voice/v1/* · 桥接 melody2score :8012）——
        .merge(upgrade(crate::voice::build_voice_router()))
        // —— Melody 域（L7 /melody/v1/* · melody2score 转谱桥接，复用 Voice 上游）——
        .merge(upgrade(crate::melody::build_melody_router()))
        // —— Cloud 域（L5 /cloud/v1/* · 本地磁盘对象存储，S3 兼容语义；注册中心统一持有）——
        .merge(upgrade(crate::cloud::build_cloud_router_with_state(states.cloud.clone())))
        // —— 存储管理面（/api/storage/* · /api/modules · StorageBackend 抽象：local 恒在 + s3 env 门控）——
        .merge(upgrade({
            let registry = Arc::new(crate::admin_storage::StorageRegistry::new(states.cloud.clone()));
            crate::admin_storage::build_storage_admin_router(registry)
        }))
        // —— LLM 网关管理面·只读族（/api/llm/providers|presets|health|routing · 实时投影 MOX_LLM_* env）——
        .merge(upgrade(crate::admin_llm::build_llm_admin_router()))
        .merge(upgrade(proxy::build_proxy_router()))
        // —— 对话沉淀域（/api/alliance/sediment · 对话核心内容 → 知识图谱/云盘/知识库）——
        .merge(upgrade(dialogue_sediment::build_sediment_router(states.sediment.clone())))
        // —— 通用业务域（状态由注册中心注入，路由构建器不负责创建） ——
        .merge(upgrade(monitor::build_monitor_router(
            states.monitor.clone(),
        )))
        .merge(upgrade(workspace::build_workspace_router(
            states.workspace.clone(),
        )))
        .merge(upgrade(projects_ext::build_projects_ext_router(
            states.projects.clone(),
        )))
        // —— 专家联盟域（7 模块共用同一份共享状态）——
        .merge(upgrade(experts_ext::build_experts_ext_router(
            experts.clone(),
        )))
        .merge(upgrade(experts_registry::build_experts_registry_router(
            experts.clone(),
        )))
        .merge(upgrade(experts_collaboration::build_experts_collaboration_router(
            experts.clone(),
        )))
        .merge(upgrade(experts_session::build_experts_session_router(
            experts.clone(),
        )))
        .merge(upgrade(experts_dispatcher::build_experts_dispatcher_router(
            experts.clone(),
        )))
        .merge(upgrade(experts_graph::build_experts_graph_router(
            experts.clone(),
        )))
        .merge(upgrade(experts_orchestration::build_experts_orchestration_router(
            experts.clone(),
        )))
        // —— T4 事件总线对外出口：SSE 事件帧 + webhook 订阅（按租户，与既有日志流独立）——
        .merge(upgrade(experts_streams::build_experts_streams_router(
            experts.clone(),
        )))
        // —— 杂项与通知 ——
        .merge(upgrade(misc::build_misc_router(states.misc.clone())))
        .merge(upgrade(notification::build_notification_router(
            states.notification.clone(),
        )))
        // —— 企业级功能域（/api/enterprise/*：OA集成/低代码设计器/SSO/消息/文档）——
        .merge(Router::new().nest(
            "/api/enterprise",
            crate::enterprise_features::build_enterprise_router_for_gateway(gateway),
        ))
        // —— P1 企业级缺口闭合：审计日志 CSV 导出 + 调度器状态总览 ——
        // /api/audit/export?start=&end=&user=&action=  真实落盘 .runtime/audit-exports/ 并流式返回
        .route(
            "/api/audit/export",
            axum::routing::get(crate::enterprise::admin_api::export_audit_logs_handler),
        )
        // /api/scheduler/status  任务列表 + 下次执行时间 + 上次执行结果
        .route(
            "/api/scheduler/status",
            axum::routing::get(crate::scheduler::api::scheduler_status_handler),
        );

    // 受保护路由统一鉴权：JWT Bearer 或 X-API-Key
    let auth_state: Arc<AuthMiddleware> = gateway.auth.clone();
    modules.route_layer(from_fn(move |request: Request, next: Next| {
        let auth = auth_state.clone();
        async move { auth_middleware(auth, request, next).await }
    }))
}

/// Auth middleware runs outside this adapter. Only its trusted extension supplies identity.
pub fn protected_kb_router(state: Arc<mox_kb_svc::KbState>) -> Router {
    use axum::response::IntoResponse;
    use tower::ServiceExt;
    mox_kb_svc::handlers::build_kb_router_with_state(state.clone()).layer(from_fn(
        move |request: Request, _next: Next| {
            let state = state.clone();
            async move {
                let Some(user) = request.extensions().get::<mox_platform_api::UserInfo>() else {
                    return axum::http::StatusCode::UNAUTHORIZED.into_response();
                };
                let access = mox_kb_svc::access::KnowledgeAccess {
                    tenant_id: user.tenant_id.clone(),
                    owner_id: user.id.clone(),
                    administrator: user
                        .roles
                        .iter()
                        .any(|r| matches!(r.as_str(), "tenant_admin" | "super_admin")),
                    readonly: user.roles.iter().any(|r| r == "readonly_auditor"),
                };
                if !user.enabled || !access.valid() {
                    return axum::http::StatusCode::FORBIDDEN.into_response();
                }
                if !access.allows_request(request.method().as_str(), request.uri().path()) {
                    return axum::http::StatusCode::FORBIDDEN.into_response();
                }
                // Restore the global projection before creating the scoped view; otherwise the first
                // caller would initialize the shared OnceCell with only their own documents.
                if state.ensure_projection_ready().await.is_err() {
                    return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
                // The scoped router must match a fresh request: retaining axum's private
                // URL parameter extension duplicates :id during the second dispatch.
                // Carry trusted identity and OriginalUri explicitly; headers/body/version
                // stay intact, while route-local extraction metadata is rebuilt below.
                let trusted_user = user.clone();
                let original_uri = request.extensions().get::<axum::extract::OriginalUri>().cloned();
                let (mut parts, body) = request.into_parts();
                parts.extensions = Default::default();
                parts.extensions.insert(trusted_user);
                if let Some(uri) = original_uri { parts.extensions.insert(uri); }
                let request = Request::from_parts(parts, body);
                match mox_kb_svc::handlers::build_kb_router_with_state(Arc::new(
                    state.scoped(access),
                ))
                .oneshot(request)
                .await
                {
                    Ok(response) => response,
                    Err(never) => match never {},
                }
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn protected_kb_requires_trusted_identity_and_denies_readonly_writes() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use tower::ServiceExt;
        let dir = tempfile::tempdir().unwrap();
        let app =
            protected_kb_router(Arc::new(mox_kb_svc::KbState::with_data_dir(dir.path().into())));
        let request = Request::builder()
            .uri("/kb/documents")
            .header("x-tenant-id", "forged")
            .body(Body::empty())
            .unwrap();
        assert_eq!(app.clone().oneshot(request).await.unwrap().status(), StatusCode::UNAUTHORIZED);
        let user = mox_platform_api::UserInfo {
            id: "reader".into(),
            username: "reader".into(),
            email: String::new(),
            tenant_id: "a".into(),
            roles: vec!["readonly_auditor".into()],
            enabled: true,
            created_at: String::new(),
        };
        for (method, path, status) in [
            ("GET", "/kb/documents", StatusCode::OK),
            ("POST", "/kb/search", StatusCode::OK),
            ("POST", "/kb/documents", StatusCode::FORBIDDEN),
            ("POST", "/kb/batch-analyze", StatusCode::FORBIDDEN),
        ] {
            let mut request = Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"query":"test"}"#))
                .unwrap();
            request.extensions_mut().insert(user.clone());
            assert_eq!(app.clone().oneshot(request).await.unwrap().status(), status);
        }
        let mut disabled = user;
        disabled.enabled = false;
        let mut request = Request::builder().uri("/kb/documents").body(Body::empty()).unwrap();
        request.extensions_mut().insert(disabled);
        assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::FORBIDDEN);
    }

    /// 构造测试用注册中心（运行时指标与日志缓冲用最小容量实例，不触碰生产数据）
    fn test_states() -> ModuleStates {
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory sqlite");
        let iam = mox_platform_iam_core::IamRepository::new(Arc::new(parking_lot::Mutex::new(conn)));
        let _ = iam.init_schema();
        ModuleStates::new(
            Arc::new(RuntimeMetrics::new()),
            LogStore::new(16),
            Arc::new(iam),
        )
    }

    /// 专家共享态在注册中心克隆后仍指向同一实例（唯一真源）
    #[test]
    fn test_module_states_shares_single_experts_state() {
        let states = test_states();
        let cloned = states.clone();
        assert!(Arc::ptr_eq(&states.experts, &cloned.experts));
    }

    /// 注册中心纳管全部 7 套域状态：任一状态在注册中心克隆后都不产生副本
    #[test]
    fn test_module_states_owns_all_domain_states() {
        let states = test_states();
        let cloned = states.clone();
        assert!(Arc::ptr_eq(&states.monitor, &cloned.monitor));
        assert!(Arc::ptr_eq(&states.workspace, &cloned.workspace));
        assert!(Arc::ptr_eq(&states.projects, &cloned.projects));
        assert!(Arc::ptr_eq(&states.misc, &cloned.misc));
        assert!(Arc::ptr_eq(&states.notification, &cloned.notification));
        assert!(Arc::ptr_eq(&states.kb, &cloned.kb));
    }

    /// 对话沉淀与 KB 路由共享同一 KbState（唯一真源）
    #[test]
    fn test_sediment_shares_kb_state() {
        let states = test_states();
        let cloned = states.clone();
        assert!(Arc::ptr_eq(&states.sediment.kb, &states.kb));
        assert!(Arc::ptr_eq(&cloned.sediment.kb, &cloned.kb));
    }

    /// 状态类型升级后仍可正常 merge（类型层面归一）
    #[test]
    fn test_upgrade_produces_mergeable_router() {
        let empty: Router<()> = Router::new();
        let upgraded: Router<GatewayState> = upgrade(empty);
        let _merged: Router<GatewayState> = Router::new().merge(upgraded);
    }
}

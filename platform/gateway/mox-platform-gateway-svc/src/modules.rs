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
    alliance, experts_collaboration, experts_common, experts_dispatcher, experts_ext,
    experts_graph, experts_orchestration, experts_registry, experts_session, kb_ext, misc,
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
    pub kb_ext: Arc<kb_ext::KbExtState>,
    /// 通知域状态（通知列表）
    pub notification: Arc<notification::NotificationState>,
}

impl ModuleStates {
    /// 构造注册中心
    ///
    /// - `runtime` / `logs`：来自 `GatewayState` 的运行时指标与在线日志缓冲，供监控域复用；
    ///   注册中心仅持有 `Arc` 引用，监控域与 Actuator 管理面共享同一份数据。
    /// - 各域状态构造时读取各自的 JSON 持久化文件；专家共享状态还会完成
    ///   启动期 JSON→SQLite 一次性迁移、内置专家种子化、能力图谱首次构建（均幂等）。
    pub fn new(runtime: Arc<RuntimeMetrics>, logs: Arc<LogStore>) -> Self {
        Self {
            experts: Arc::new(experts_common::ExpertsSharedState::new()),
            monitor: Arc::new(monitor::MonitorState::new(runtime, logs)),
            workspace: Arc::new(workspace::WorkspaceState::new()),
            projects: Arc::new(projects_ext::ProjectsState::new()),
            misc: Arc::new(misc::MiscState::new()),
            kb_ext: Arc::new(kb_ext::KbExtState::new()),
            notification: Arc::new(notification::NotificationState::new()),
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
pub fn build_module_routers(
    states: &ModuleStates,
    gateway: &GatewayState,
) -> Router<GatewayState> {
    let experts = states.experts.clone();

    let modules = Router::<GatewayState>::new()
        // —— 领域外挂：KG + AI 引擎（mox-kg-service-svc 自包含） ——
        .merge(upgrade(crate::http_adapter::build_kg_ai_router()))
        // —— 知识库域（mox-kb-svc） ——
        // 前缀归一化（RC-1）：内部注册 /kb/*，对外统一暴露 /api/kb/*。
        // 采用 nest 包装而非直接 merge，避免破坏 mox-kb-svc 自身的 /kb/* 集成测试。
        .merge(upgrade(Router::new().nest(
            "/api",
            mox_kb_svc::handlers::build_kb_router(),
        )))
        // —— 联盟任务域（远程优先 + 本地降级）——
        .merge(upgrade(alliance::build_alliance_router()))
        // —— 系统管理 + 安全域（IAM SQLite 真实链路，已回收为受保护路由）——
        .merge(system::build_system_router())
        .merge(system::build_security_router())
        // —— 业务域兜底反代（必须排在具体域之后）——
        .merge(upgrade(proxy::build_proxy_router()))
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
            experts,
        )))
        // —— 杂项与通知 ——
        .merge(upgrade(misc::build_misc_router(states.misc.clone())))
        .merge(upgrade(kb_ext::build_kb_ext_router(states.kb_ext.clone())))
        .merge(upgrade(notification::build_notification_router(
            states.notification.clone(),
        )));

    // 受保护路由统一鉴权：JWT Bearer 或 X-API-Key
    let auth_state: Arc<AuthMiddleware> = gateway.auth.clone();
    modules.route_layer(from_fn(move |request: Request, next: Next| {
        let auth = auth_state.clone();
        async move { auth_middleware(auth, request, next).await }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用注册中心（运行时指标与日志缓冲用最小容量实例，不触碰生产数据）
    fn test_states() -> ModuleStates {
        ModuleStates::new(
            Arc::new(RuntimeMetrics::new()),
            LogStore::new(16),
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
        assert!(Arc::ptr_eq(&states.kb_ext, &cloned.kb_ext));
        assert!(Arc::ptr_eq(&states.notification, &cloned.notification));
    }

    /// 状态类型升级后仍可正常 merge（类型层面归一）
    #[test]
    fn test_upgrade_produces_mergeable_router() {
        let empty: Router<()> = Router::new();
        let upgraded: Router<GatewayState> = upgrade(empty);
        let _merged: Router<GatewayState> = Router::new().merge(upgraded);
    }
}

// Copyright (c) 2026 璇玑 RelGraph · mox 模块化系统架构归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 平台门面 (Platform Facade)
//!
//! 统一平台的总入口，对外暴露六大归一化体系的统一 API，
//! 并负责跨模块协同编排。

use parking_lot::RwLock;

use crate::audit::AuditLogger;
use crate::config_center::UnifiedConfigCenter;
use crate::cross_orchestrator::{
    CrossOrchestrator, OrchestrationContext, OrchestrationResult,
};
use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, RateLimitManager};
use crate::error::{PlatformError, PlatformResult};
use crate::event_bus::{EventBus, EventType, PlatformEvent};
use crate::platform_lifecycle::PlatformLifecycle;
use crate::platform_status::PlatformStatusMonitor;
use crate::types::*;

/// 平台门面 - 统一平台总入口
pub struct PlatformFacade {
    /// 生命周期管理
    lifecycle: PlatformLifecycle,
    /// 状态监控
    status_monitor: PlatformStatusMonitor,
    /// 跨系统编排引擎
    orchestrator: RwLock<CrossOrchestrator>,
    /// 统一事件总线
    event_bus: EventBus,
    /// 统一配置中心
    config_center: UnifiedConfigCenter,
    /// 审计日志
    audit_logger: AuditLogger,
    /// 限流管理器
    rate_limiter: RateLimitManager,
    /// 平台熔断器
    circuit_breaker: CircuitBreaker,
    /// 是否已初始化
    initialized: RwLock<bool>,
}

impl PlatformFacade {
    /// 创建平台实例
    pub fn new() -> Self {
        Self {
            lifecycle: PlatformLifecycle::new(),
            status_monitor: PlatformStatusMonitor::new(),
            orchestrator: RwLock::new(CrossOrchestrator::new()),
            event_bus: EventBus::new(),
            config_center: UnifiedConfigCenter::new(),
            audit_logger: AuditLogger::new(),
            rate_limiter: RateLimitManager::new(1000.0, 2000.0, 500.0, 100.0),
            circuit_breaker: CircuitBreaker::new(CircuitBreakerConfig::default()),
            initialized: RwLock::new(false),
        }
    }

    /// 启动平台（注册标准模块并初始化）
    pub fn bootstrap(&self) -> PlatformResult<()> {
        if *self.initialized.read() {
            return Err(PlatformError::InitError(
                "platform already bootstrapped".to_string(),
            ));
        }

        // 1. 注册标准模块
        self.lifecycle.register_standard_modules();

        // 2. 注册监控
        for module in self.lifecycle.get_modules_by_system(NormalizationSystem::Architecture) {
            self.status_monitor.register_module(&module.id);
        }
        for module in self.lifecycle.get_modules_by_system(NormalizationSystem::Permission) {
            self.status_monitor.register_module(&module.id);
        }
        for module in self.lifecycle.get_modules_by_system(NormalizationSystem::Lowcode) {
            self.status_monitor.register_module(&module.id);
        }
        for module in self.lifecycle.get_modules_by_system(NormalizationSystem::ProcessAlgo) {
            self.status_monitor.register_module(&module.id);
        }
        for module in self.lifecycle.get_modules_by_system(NormalizationSystem::Frontend) {
            self.status_monitor.register_module(&module.id);
        }
        for module in self.lifecycle.get_modules_by_system(NormalizationSystem::AiAssistant) {
            self.status_monitor.register_module(&module.id);
        }

        // 3. 初始化所有模块
        self.lifecycle.initialize()?;

        // 4. 注册内置事件联动规则
        self.event_bus.register_builtin_subscribers();

        *self.initialized.write() = true;

        Ok(())
    }

    /// 检查平台是否就绪
    pub fn is_ready(&self) -> bool {
        *self.initialized.read() && self.lifecycle.is_initialized()
    }

    /// 获取平台健康度
    pub fn health(&self) -> PlatformHealth {
        self.lifecycle.get_health()
    }

    /// 获取平台指标
    pub fn metrics(&self) -> crate::platform_status::PlatformMetrics {
        self.status_monitor.get_metrics()
    }

    /// 获取六大归一化体系概览
    pub fn systems_overview(&self) -> Vec<SystemOverview> {
        let mut result = Vec::new();

        for system in NormalizationSystem::all() {
            let modules = self.lifecycle.get_modules_by_system(system);
            let healthy = modules.iter().filter(|m| m.status.is_healthy()).count();

            result.push(SystemOverview {
                system,
                name: system.name().to_string(),
                description: system.description().to_string(),
                module_count: modules.len(),
                healthy_count: healthy,
                status: if healthy == modules.len() {
                    PlatformModuleStatus::Running
                } else if healthy > 0 {
                    PlatformModuleStatus::Degraded
                } else {
                    PlatformModuleStatus::Failed
                },
            });
        }

        result
    }

    /// 获取生命周期管理引用
    pub fn lifecycle(&self) -> &PlatformLifecycle {
        &self.lifecycle
    }

    /// 获取状态监控引用
    pub fn status_monitor(&self) -> &PlatformStatusMonitor {
        &self.status_monitor
    }

    /// 执行跨系统编排
    pub fn orchestrate(
        &self,
        template_id: &str,
        context: OrchestrationContext,
    ) -> PlatformResult<OrchestrationResult> {
        if !self.is_ready() {
            return Err(PlatformError::NotInitialized);
        }
        let orchestrator = self.orchestrator.read();
        orchestrator.execute(template_id, context)
    }

    /// 获取可用编排模板列表
    pub fn orchestration_templates(&self) -> Vec<String> {
        let orchestrator = self.orchestrator.read();
        orchestrator
            .list_templates()
            .iter()
            .map(|t| t.id.clone())
            .collect()
    }

    /// 获取事件总线引用
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// 获取配置中心引用
    pub fn config(&self) -> &UnifiedConfigCenter {
        &self.config_center
    }

    /// 获取审计日志引用
    pub fn audit(&self) -> &AuditLogger {
        &self.audit_logger
    }

    /// 获取限流管理器引用
    pub fn rate_limiter(&self) -> &RateLimitManager {
        &self.rate_limiter
    }

    /// 获取熔断器引用
    pub fn circuit_breaker(&self) -> &CircuitBreaker {
        &self.circuit_breaker
    }

    /// 发布平台事件
    pub fn publish_event(&self, event: PlatformEvent) -> Vec<crate::event_bus::EventHandleResult> {
        self.event_bus.publish(event)
    }

    /// 关闭平台
    pub fn shutdown(&self) -> PlatformResult<()> {
        self.lifecycle.shutdown()?;
        *self.initialized.write() = false;
        Ok(())
    }
}

/// 归一化体系概览
pub struct SystemOverview {
    /// 体系标识
    pub system: NormalizationSystem,
    /// 名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 模块数
    pub module_count: usize,
    /// 健康模块数
    pub healthy_count: usize,
    /// 状态
    pub status: PlatformModuleStatus,
}

impl Default for PlatformFacade {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_and_shutdown() {
        let platform = PlatformFacade::new();
        assert!(!platform.is_ready());

        platform.bootstrap().unwrap();
        assert!(platform.is_ready());

        let health = platform.health();
        assert_eq!(health.overall_status, PlatformModuleStatus::Running);
        assert_eq!(health.healthy_count, 9);

        platform.shutdown().unwrap();
        assert!(!platform.is_ready());
    }

    #[test]
    fn test_systems_overview() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let overview = platform.systems_overview();
        assert_eq!(overview.len(), 6);

        // 架构归一化应有 3 个模块
        let arch = overview
            .iter()
            .find(|o| o.system == NormalizationSystem::Architecture)
            .unwrap();
        assert_eq!(arch.module_count, 3);

        // AI 应有 1 个模块
        let ai = overview
            .iter()
            .find(|o| o.system == NormalizationSystem::AiAssistant)
            .unwrap();
        assert_eq!(ai.module_count, 1);

        // 所有体系都应该是 Running
        for o in &overview {
            assert_eq!(o.status, PlatformModuleStatus::Running);
        }
    }

    #[test]
    fn test_double_bootstrap_fails() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();
        assert!(platform.bootstrap().is_err());
    }

    #[test]
    fn test_metrics() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let metrics = platform.metrics();
        assert_eq!(metrics.total_requests, 0);
    }

    #[test]
    fn test_lifecycle_access() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        assert_eq!(platform.lifecycle().module_count(), 9);
    }

    #[test]
    fn test_orchestration_templates() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let templates = platform.orchestration_templates();
        assert!(templates.contains(&"ai-business-request".to_string()));
        assert!(templates.contains(&"ai-query-only".to_string()));
        assert!(templates.contains(&"algo-analysis".to_string()));
        assert_eq!(templates.len(), 3);
    }

    #[test]
    fn test_orchestrate_ai_business_request() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let ctx = OrchestrationContext {
            tenant_id: "tenant-001".to_string(),
            user_id: "user-001".to_string(),
            original_request: "我想申请请假回家".to_string(),
            variables: std::collections::HashMap::new(),
            current_step: 0,
        };

        let result = platform.orchestrate("ai-business-request", ctx).unwrap();
        assert!(result.success);
        assert_eq!(result.total_steps, 6);
        assert_eq!(result.completed_steps, 6);

        // 验证六大体系全部被调用
        let mut systems_used = std::collections::HashSet::new();
        for step in &result.steps {
            systems_used.insert(step.step_type.system());
        }
        // 6步覆盖了6个体系
        assert_eq!(systems_used.len(), 6);
    }

    #[test]
    fn test_orchestrate_without_init_fails() {
        let platform = PlatformFacade::new();
        let ctx = OrchestrationContext::default();
        let result = platform.orchestrate("ai-query-only", ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_event_bus_builtin_subscribers() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        assert_eq!(platform.event_bus().total_subscribers(), 5);
        assert_eq!(platform.event_bus().published_count(), 0);
    }

    #[test]
    fn test_publish_event_via_facade() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let event = PlatformEvent::new(
            EventType::IntentRecognized,
            NormalizationSystem::AiAssistant,
            "t1",
            serde_json::json!({"intent": "test"}),
        );

        let results = platform.publish_event(event);
        assert_eq!(results.len(), 1); // perm-auto-check
        assert!(results[0].success);
        assert_eq!(platform.event_bus().published_count(), 1);
    }

    #[test]
    fn test_full_event_chain_via_facade() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let corr_id = "facade-test-001";

        // 发布意图事件 → 触发权限校验订阅
        let e1 = PlatformEvent::new(
            EventType::IntentRecognized,
            NormalizationSystem::AiAssistant,
            "t1",
            serde_json::json!({"intent": "leave"}),
        )
        .with_correlation_id(corr_id);
        let r1 = platform.publish_event(e1);
        assert_eq!(r1.len(), 1);

        // 发布权限通过事件 → 触发表单生成订阅
        let e2 = PlatformEvent::new(
            EventType::PermissionChecked,
            NormalizationSystem::Permission,
            "t1",
            serde_json::json!({"allowed": true}),
        )
        .with_correlation_id(corr_id);
        let r2 = platform.publish_event(e2);
        assert_eq!(r2.len(), 1);
        assert!(r2[0].success);

        // 发布流程完成事件 → 触发前端通知 + 架构同步
        let e3 = PlatformEvent::new(
            EventType::ProcessCompleted,
            NormalizationSystem::ProcessAlgo,
            "t1",
            serde_json::json!({"status": "approved"}),
        )
        .with_correlation_id(corr_id);
        let r3 = platform.publish_event(e3);
        assert_eq!(r3.len(), 2);

        // 事件链完整
        let chain = platform.event_bus().query_by_correlation(corr_id);
        assert_eq!(chain.len(), 3);
    }

    #[test]
    fn test_config_center_access() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        assert_eq!(platform.config().schema_count(), 13);
        assert_eq!(platform.config().config_count(), 13);
    }

    #[test]
    fn test_config_get_and_set_via_facade() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        // 读取默认值
        let theme = platform.config().get_global("frontend.theme").unwrap();
        assert_eq!(theme, serde_json::json!("light"));

        // 修改配置
        platform
            .config()
            .set_global("frontend.theme", serde_json::json!("dark"))
            .unwrap();

        // 验证修改
        let theme = platform.config().get_global("frontend.theme").unwrap();
        assert_eq!(theme, serde_json::json!("dark"));
    }

    #[test]
    fn test_config_validation_via_facade() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        // 无效的主题值
        let result = platform
            .config()
            .set_global("frontend.theme", serde_json::json!("invalid"));
        assert!(result.is_err());
    }

    #[test]
    fn test_config_per_system_schemas() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        use crate::types::NormalizationSystem;
        let arch_schemas = platform.config().list_schemas_by_system(NormalizationSystem::Architecture);
        assert_eq!(arch_schemas.len(), 2); // default_protocol + request_timeout_ms
    }
}

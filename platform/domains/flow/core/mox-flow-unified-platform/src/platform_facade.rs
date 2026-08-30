// Copyright (c) 2026 璇玑 RelGraph · 全维归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 平台门面 (Platform Facade)
//!
//! 统一平台的总入口，对外暴露六大归一化体系的统一 API，
//! 并负责跨模块协同编排。

use parking_lot::RwLock;

use crate::error::{PlatformError, PlatformResult};
use crate::platform_lifecycle::PlatformLifecycle;
use crate::platform_status::PlatformStatusMonitor;
use crate::types::*;

/// 平台门面 - 统一平台总入口
pub struct PlatformFacade {
    /// 生命周期管理
    lifecycle: PlatformLifecycle,
    /// 状态监控
    status_monitor: PlatformStatusMonitor,
    /// 是否已初始化
    initialized: RwLock<bool>,
}

impl PlatformFacade {
    /// 创建平台实例
    pub fn new() -> Self {
        Self {
            lifecycle: PlatformLifecycle::new(),
            status_monitor: PlatformStatusMonitor::new(),
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
}

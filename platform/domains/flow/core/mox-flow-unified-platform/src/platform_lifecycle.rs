// Copyright (c) 2026 璇玑 RelGraph · 全维归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 平台生命周期管理
//!
//! 负责平台的启动、初始化、健康检查和优雅关闭

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{PlatformError, PlatformResult};
use crate::types::*;

/// 平台生命周期管理器
pub struct PlatformLifecycle {
    /// 平台模块表
    modules: RwLock<HashMap<String, PlatformModule>>,
    /// 是否已初始化
    initialized: AtomicBool,
    /// 启动时间戳
    started_at: AtomicU64,
    /// 总请求数
    total_requests: AtomicU64,
}

impl PlatformLifecycle {
    /// 创建生命周期管理器
    pub fn new() -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
            initialized: AtomicBool::new(false),
            started_at: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
        }
    }

    /// 注册模块
    pub fn register_module(&self, module: PlatformModule) -> PlatformResult<PlatformModule> {
        if self.modules.read().contains_key(&module.id) {
            return Err(PlatformError::ModuleAlreadyExists(format!(
                "module '{}' already exists",
                module.name
            )));
        }
        self.modules
            .write()
            .insert(module.id.clone(), module.clone());
        Ok(module)
    }

    /// 注册六大归一化体系的标准模块
    pub fn register_standard_modules(&self) {
        let modules = vec![
            // 第一层：基础架构
            PlatformModule::new(
                "unified-storage",
                NormalizationSystem::Architecture,
                "0.1.0",
                10,
            ),
            PlatformModule::new(
                "unified-meta",
                NormalizationSystem::Architecture,
                "0.1.0",
                20,
            ),
            // 第二层：架构归一
            PlatformModule::new(
                "unified-arch",
                NormalizationSystem::Architecture,
                "0.1.0",
                30,
            ),
            // 第三层：权限
            PlatformModule::new(
                "unified-perm",
                NormalizationSystem::Permission,
                "0.1.0",
                40,
            ),
            // 第四层：算法联盟
            PlatformModule::new(
                "algo-alliance",
                NormalizationSystem::ProcessAlgo,
                "0.1.0",
                50,
            ),
            // 第五层：流程算法归一
            PlatformModule::new(
                "unified-process",
                NormalizationSystem::ProcessAlgo,
                "0.1.0",
                60,
            ),
            // 第六层：低代码
            PlatformModule::new(
                "lowcode-core",
                NormalizationSystem::Lowcode,
                "0.1.0",
                70,
            ),
            // 第七层：前端归一
            PlatformModule::new(
                "unified-frontend",
                NormalizationSystem::Frontend,
                "0.1.0",
                80,
            ),
            // 第八层：AI助手
            PlatformModule::new(
                "ai-assistant",
                NormalizationSystem::AiAssistant,
                "0.1.0",
                90,
            ),
        ];

        for module in modules {
            let _ = self.register_module(module);
        }
    }

    /// 初始化平台（按顺序启动所有模块）
    pub fn initialize(&self) -> PlatformResult<()> {
        if self.initialized.load(Ordering::Relaxed) {
            return Err(PlatformError::InitError(
                "platform already initialized".to_string(),
            ));
        }

        // 获取所有模块并按启动顺序排序
        let mut module_ids: Vec<(String, u32)> = {
            let modules = self.modules.read();
            modules
                .values()
                .map(|m| (m.id.clone(), m.startup_order))
                .collect()
        };
        module_ids.sort_by_key(|(_, order)| *order);

        // 按顺序初始化
        for (module_id, _) in &module_ids {
            self.set_module_status(module_id, PlatformModuleStatus::Initializing)?;
            // 模拟初始化
            self.set_module_status(module_id, PlatformModuleStatus::Running)?;
        }

        self.initialized.store(true, Ordering::Relaxed);
        self.started_at
            .store(Self::now_ms(), Ordering::Relaxed);

        Ok(())
    }

    /// 检查平台是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    /// 设置模块状态
    fn set_module_status(
        &self,
        module_id: &str,
        status: PlatformModuleStatus,
    ) -> PlatformResult<()> {
        let mut modules = self.modules.write();
        let module = modules
            .get_mut(module_id)
            .ok_or_else(|| PlatformError::ModuleNotFound(module_id.to_string()))?;
        module.status = status;
        Ok(())
    }

    /// 获取模块状态
    pub fn get_module_status(&self, module_id: &str) -> Option<PlatformModuleStatus> {
        self.modules
            .read()
            .get(module_id)
            .map(|m| m.status)
    }

    /// 获取平台健康度
    pub fn get_health(&self) -> PlatformHealth {
        let modules = self.modules.read();
        let mut module_statuses = HashMap::new();
        let mut healthy_count = 0u32;
        let total_count = modules.len() as u32;

        for (id, module) in modules.iter() {
            module_statuses.insert(id.clone(), module.status);
            if module.status.is_healthy() {
                healthy_count += 1;
            }
        }

        let overall_status = if healthy_count == total_count {
            PlatformModuleStatus::Running
        } else if healthy_count > 0 {
            PlatformModuleStatus::Degraded
        } else {
            PlatformModuleStatus::Failed
        };

        let started = self.started_at.load(Ordering::Relaxed);
        let now = Self::now_ms();
        let uptime_seconds = if started > 0 {
            (now - started) / 1000
        } else {
            0
        };

        PlatformHealth {
            overall_status,
            healthy_count,
            total_count,
            module_statuses,
            started_at: started,
            uptime_seconds,
            active_users: 0,
            today_requests: self.total_requests.load(Ordering::Relaxed),
        }
    }

    /// 记录请求
    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// 优雅关闭
    pub fn shutdown(&self) -> PlatformResult<()> {
        if !self.initialized.load(Ordering::Relaxed) {
            return Err(PlatformError::NotInitialized);
        }

        // 逆序关闭
        let mut module_ids: Vec<(String, u32)> = {
            let modules = self.modules.read();
            modules
                .values()
                .map(|m| (m.id.clone(), m.startup_order))
                .collect()
        };
        module_ids.sort_by_key(|(_, order)| std::cmp::Reverse(*order));

        for (module_id, _) in &module_ids {
            self.set_module_status(module_id, PlatformModuleStatus::Shutdown)?;
        }

        self.initialized.store(false, Ordering::Relaxed);
        Ok(())
    }

    /// 获取模块数量
    pub fn module_count(&self) -> usize {
        self.modules.read().len()
    }

    /// 按系统获取模块
    pub fn get_modules_by_system(&self, system: NormalizationSystem) -> Vec<PlatformModule> {
        self.modules
            .read()
            .values()
            .filter(|m| m.system == system)
            .cloned()
            .collect()
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

impl Default for PlatformLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_standard_modules() {
        let lifecycle = PlatformLifecycle::new();
        lifecycle.register_standard_modules();
        assert_eq!(lifecycle.module_count(), 9);
    }

    #[test]
    fn test_initialize_and_shutdown() {
        let lifecycle = PlatformLifecycle::new();
        lifecycle.register_standard_modules();

        assert!(!lifecycle.is_initialized());
        lifecycle.initialize().unwrap();
        assert!(lifecycle.is_initialized());

        let health = lifecycle.get_health();
        assert_eq!(health.overall_status, PlatformModuleStatus::Running);
        assert_eq!(health.healthy_count, 9);
        assert_eq!(health.health_score(), 1.0);

        lifecycle.shutdown().unwrap();
        assert!(!lifecycle.is_initialized());
    }

    #[test]
    fn test_record_request() {
        let lifecycle = PlatformLifecycle::new();
        lifecycle.register_standard_modules();
        lifecycle.initialize().unwrap();

        lifecycle.record_request();
        lifecycle.record_request();

        let health = lifecycle.get_health();
        assert_eq!(health.today_requests, 2);
    }

    #[test]
    fn test_get_modules_by_system() {
        let lifecycle = PlatformLifecycle::new();
        lifecycle.register_standard_modules();

        let arch_modules = lifecycle.get_modules_by_system(NormalizationSystem::Architecture);
        assert_eq!(arch_modules.len(), 3); // storage + meta + arch

        let ai_modules = lifecycle.get_modules_by_system(NormalizationSystem::AiAssistant);
        assert_eq!(ai_modules.len(), 1);
    }

    #[test]
    fn test_double_init_fails() {
        let lifecycle = PlatformLifecycle::new();
        lifecycle.register_standard_modules();
        lifecycle.initialize().unwrap();
        assert!(lifecycle.initialize().is_err());
    }

    #[test]
    fn test_shutdown_without_init_fails() {
        let lifecycle = PlatformLifecycle::new();
        assert!(lifecycle.shutdown().is_err());
    }
}

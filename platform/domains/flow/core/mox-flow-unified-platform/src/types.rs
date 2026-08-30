// Copyright (c) 2026 璇玑 RelGraph · 全维归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 平台核心类型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 六大归一化体系标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizationSystem {
    /// 架构归一化
    Architecture,
    /// 权限管理归一化
    Permission,
    /// 低代码平台
    Lowcode,
    /// 流程算法归一化
    ProcessAlgo,
    /// 前端功能归一化
    Frontend,
    /// AI对话全维自动化
    AiAssistant,
}

impl NormalizationSystem {
    pub fn name(&self) -> &'static str {
        match self {
            NormalizationSystem::Architecture => "架构归一化",
            NormalizationSystem::Permission => "权限管理归一化",
            NormalizationSystem::Lowcode => "低代码平台",
            NormalizationSystem::ProcessAlgo => "流程算法归一化",
            NormalizationSystem::Frontend => "前端功能归一化",
            NormalizationSystem::AiAssistant => "AI对话全维自动化",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            NormalizationSystem::Architecture => "统一接入协议+统一模型+第三方系统对接标准",
            NormalizationSystem::Permission => "RBAC+ABAC混合+多租户+数据权限+SSO",
            NormalizationSystem::Lowcode => "元数据驱动+表单引擎+页面引擎+脚本扩展",
            NormalizationSystem::ProcessAlgo => "算法联盟+流程编排+专家系统深度融合",
            NormalizationSystem::Frontend => "统一组件库+统一设计系统+功能模块重整",
            NormalizationSystem::AiAssistant => "意图理解+任务分解+多Agent协同+自主执行",
        }
    }

    /// 获取所有六大体系
    pub fn all() -> [NormalizationSystem; 6] {
        [
            NormalizationSystem::Architecture,
            NormalizationSystem::Permission,
            NormalizationSystem::Lowcode,
            NormalizationSystem::ProcessAlgo,
            NormalizationSystem::Frontend,
            NormalizationSystem::AiAssistant,
        ]
    }
}

/// 平台模块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformModule {
    /// 模块 ID
    pub id: String,
    /// 模块名称
    pub name: String,
    /// 所属归一化体系
    pub system: NormalizationSystem,
    /// 模块版本
    pub version: String,
    /// 描述
    pub description: String,
    /// 状态
    pub status: PlatformModuleStatus,
    /// 依赖模块
    pub dependencies: Vec<String>,
    /// 启动顺序
    pub startup_order: u32,
}

/// 模块状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformModuleStatus {
    /// 未初始化
    Uninitialized,
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 降级运行
    Degraded,
    /// 暂停
    Suspended,
    /// 故障
    Failed,
    /// 已关闭
    Shutdown,
}

impl PlatformModuleStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, PlatformModuleStatus::Running)
    }

    pub fn is_available(&self) -> bool {
        matches!(
            self,
            PlatformModuleStatus::Running | PlatformModuleStatus::Degraded
        )
    }
}

/// 平台健康度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformHealth {
    /// 整体状态
    pub overall_status: PlatformModuleStatus,
    /// 健康模块数
    pub healthy_count: u32,
    /// 总模块数
    pub total_count: u32,
    /// 各模块状态
    pub module_statuses: HashMap<String, PlatformModuleStatus>,
    /// 启动时间
    pub started_at: u64,
    /// 运行时长（秒）
    pub uptime_seconds: u64,
    /// 活跃用户数
    pub active_users: u64,
    /// 今日请求数
    pub today_requests: u64,
}

impl PlatformHealth {
    /// 健康度百分比
    pub fn health_score(&self) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        self.healthy_count as f64 / self.total_count as f64
    }
}

impl PlatformModule {
    /// 创建模块
    pub fn new(
        name: &str,
        system: NormalizationSystem,
        version: &str,
        startup_order: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            system,
            version: version.to_string(),
            description: system.description().to_string(),
            status: PlatformModuleStatus::Uninitialized,
            dependencies: Vec::new(),
            startup_order,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_six_systems() {
        let all = NormalizationSystem::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_system_names() {
        assert_eq!(NormalizationSystem::Architecture.name(), "架构归一化");
        assert_eq!(NormalizationSystem::Permission.name(), "权限管理归一化");
        assert_eq!(NormalizationSystem::Lowcode.name(), "低代码平台");
        assert_eq!(NormalizationSystem::ProcessAlgo.name(), "流程算法归一化");
        assert_eq!(NormalizationSystem::Frontend.name(), "前端功能归一化");
        assert_eq!(NormalizationSystem::AiAssistant.name(), "AI对话全维自动化");
    }

    #[test]
    fn test_module_status_healthy() {
        assert!(PlatformModuleStatus::Running.is_healthy());
        assert!(!PlatformModuleStatus::Degraded.is_healthy());
        assert!(!PlatformModuleStatus::Failed.is_healthy());
    }

    #[test]
    fn test_module_status_available() {
        assert!(PlatformModuleStatus::Running.is_available());
        assert!(PlatformModuleStatus::Degraded.is_available());
        assert!(!PlatformModuleStatus::Failed.is_available());
    }

    #[test]
    fn test_platform_health_score() {
        let mut health = PlatformHealth {
            overall_status: PlatformModuleStatus::Running,
            healthy_count: 5,
            total_count: 6,
            module_statuses: HashMap::new(),
            started_at: 0,
            uptime_seconds: 3600,
            active_users: 100,
            today_requests: 10000,
        };
        assert!((health.health_score() - 5.0 / 6.0).abs() < f64::EPSILON);

        health.healthy_count = 6;
        assert_eq!(health.health_score(), 1.0);
    }

    #[test]
    fn test_platform_module_creation() {
        let module =
            PlatformModule::new("test-module", NormalizationSystem::Lowcode, "1.0.0", 10);
        assert_eq!(module.name, "test-module");
        assert_eq!(module.system, NormalizationSystem::Lowcode);
        assert_eq!(module.version, "1.0.0");
        assert_eq!(module.startup_order, 10);
        assert_eq!(module.status, PlatformModuleStatus::Uninitialized);
    }
}

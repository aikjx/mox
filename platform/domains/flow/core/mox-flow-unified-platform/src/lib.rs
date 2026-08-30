// Copyright (c) 2026 璇玑 RelGraph · 全维归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 全维归一化统一平台核心
//!
//! 六大归一化体系的统一入口：
//! 1. 架构归一化 - 统一接入协议 + 统一模型 + 第三方系统对接标准
//! 2. 权限管理归一化 - RBAC+ABAC 混合 + 多租户 + 数据权限 + SSO
//! 3. 低代码平台 - 元数据驱动 + 表单引擎 + 页面引擎 + 脚本扩展
//! 4. 流程算法归一化 - 算法联盟 + 流程编排 + 专家系统深度融合
//! 5. 前端功能归一化 - 统一组件库 + 统一设计系统 + 功能模块重整
//! 6. AI 对话全维自动化 - 意图理解 + 任务分解 + 多Agent协同 + 自主执行
//!
//! 本模块提供：
//! - 平台级统一 API 门面 (PlatformFacade)
//! - 跨模块协同编排
//! - 平台生命周期管理
//! - 平台状态监控

pub mod error;
pub mod types;
pub mod platform_facade;
pub mod platform_lifecycle;
pub mod platform_status;

// ========== 六大归一化体系重导出 ==========

// 1. 架构归一化
pub use mox_flow_unified_arch_core as arch;

// 2. 权限管理归一化
pub use mox_flow_unified_perm_core as perm;

// 3. 低代码平台
pub use mox_flow_lowcode_core as lowcode;

// 4. 流程算法归一化
pub use mox_flow_unified_process_core as process;
pub use mox_flow_algo_alliance_core as algo;

// 5. 前端功能归一化
pub use mox_flow_unified_frontend_core as frontend;

// 6. AI 对话全维自动化
pub use mox_flow_ai_assistant_core as ai;

// 基础支撑
pub use mox_flow_unified_storage_core as storage;
pub use mox_flow_unified_meta_core as meta;

// 平台级导出
pub use error::{PlatformError, PlatformResult};
pub use types::{
    PlatformModule, PlatformModuleStatus, PlatformHealth,
    NormalizationSystem,
};
pub use platform_facade::PlatformFacade;
pub use platform_lifecycle::PlatformLifecycle;
pub use platform_status::PlatformStatusMonitor;

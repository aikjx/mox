// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 七位业务专家 + 七位开发专家 = 共十四专家
//!
//! P2 架构解耦 · 阶段 4：
//! 本模块为骨架占位，14 位具体专家的完整实现将在后续迭代中迁移。
//! 当前提供 `all_experts()` 函数，返回空向量，确保管线可编译。

// ---- 业务七维（分析流程图）----
pub mod algorithm;
pub mod business;
pub mod data;
pub mod observability;
pub mod permission;
pub mod resource;
pub mod security;

// ---- 开发七维（分析代码IR）----
pub mod architecture;
pub mod code_quality;
pub mod documentation;
pub mod maintainability;
pub mod performance;
pub mod security_code;
pub mod testing;

use crate::expert::Expert;
use std::boxed::Box;

/// 构建业务七专家（分析流程图）
///
/// TODO(P2 阶段 4 后续迭代)：迁移完整的业务七专家实现
pub fn business_experts() -> Vec<Box<dyn Expert>> {
    vec![
        Box::new(business::BusinessExpert),
        Box::new(algorithm::AlgorithmExpert),
        Box::new(permission::PermissionExpert),
        Box::new(resource::ResourceExpert),
        Box::new(security::SecurityExpert),
        Box::new(data::DataExpert),
        Box::new(observability::ObservabilityExpert),
    ]
}

/// 构建开发七专家（分析代码IR）
///
/// TODO(P2 阶段 4 后续迭代)：迁移完整的开发七专家实现
pub fn development_experts() -> Vec<Box<dyn Expert>> {
    vec![
        Box::new(architecture::ArchitectureExpert),
        Box::new(security_code::SecurityCodeExpert),
        Box::new(code_quality::CodeQualityExpert),
        Box::new(performance::PerformanceExpert),
        Box::new(testing::TestingExpert),
        Box::new(documentation::DocumentationExpert),
        Box::new(maintainability::MaintainabilityExpert),
    ]
}

/// 构建全部十四专家（业务 + 开发）
pub fn all_experts() -> Vec<Box<dyn Expert>> {
    let mut experts = business_experts();
    experts.extend(development_experts());
    experts
}

//! 七位业务专家 + 七位开发专家 = 共十四专家

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
pub mod security_code;
pub mod code_quality;
pub mod performance;
pub mod testing;
pub mod documentation;
pub mod maintainability;

use crate::expert::Expert;
use std::boxed::Box;

/// 构建业务七专家（分析流程图）
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

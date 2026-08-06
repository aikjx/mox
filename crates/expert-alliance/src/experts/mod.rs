//! 七位专家实现

pub mod algorithm;
pub mod business;
pub mod data;
pub mod observability;
pub mod permission;
pub mod resource;
pub mod security;

use crate::expert::Expert;
use std::boxed::Box;

/// 构建全部专家（顺序无关，并行派发）
pub fn all_experts() -> Vec<Box<dyn Expert>> {
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

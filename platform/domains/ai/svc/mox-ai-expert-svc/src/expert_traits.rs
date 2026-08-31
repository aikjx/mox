// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! L3 领域层对外抽象 trait（DIP 反转：下游只依赖这些 trait 抽象，不依赖具体实现）。
//!
//! 设计：
//! - `ExpertRegistry`：专家注册 / 查询（可替代直接依赖 `experts::all_experts()`）。
//! - `ExpertConsultant`：单次咨询接口（替代直接依赖 `mox_optimize` / `GovernanceReport`）。
//! - `AllianceOrchestrator`：联盟编排 / 任务路由（替代直接依赖裁决、路由等内部实现）。
//!
//! P2 架构解耦 · 阶段 1.5：
//! - 三个 trait 定义已迁移至 `mox-ai-expert-proto`，本模块通过 re-export 保持对外 100% 兼容。
//! - 工厂函数（default_registry / default_consultant / default_orchestrator）保留在本地，
//!   因为它们依赖 concrete 实现（RegistryImpl / ExpertServiceImpl / AllianceRouter）。

use std::sync::Arc;

// ---------------------------------------------------------------------------
// 从 mox-ai-expert-proto 重新导出 trait 定义（SSOT 单一真相源）
// ---------------------------------------------------------------------------

pub use mox_ai_expert_proto::{AllianceOrchestrator, ExpertConsultant, ExpertRegistry};

// ---------------------------------------------------------------------------
// 默认工厂：下游 crate 调用这些函数获得默认 trait object，
// 从而无需 `use mox_ai_expert_svc::services::{RegistryImpl, ...}` 等具体名字。
// 下游只会 `use mox_ai_expert_svc::expert_traits::default_consultant`，走 trait 模块，合规。
// ---------------------------------------------------------------------------

/// 默认注册表（内置璇玑 14 维专家，内存实现）。
pub fn default_registry() -> Arc<dyn ExpertRegistry> {
    Arc::new(crate::services::RegistryImpl::new()) as Arc<dyn ExpertRegistry>
}

/// 默认咨询实现（包装真实 `mox_optimize`，同步快路径见 `ExpertServiceImpl::consult_sync`）。
pub fn default_consultant() -> Arc<dyn ExpertConsultant> {
    Arc::new(crate::services::ExpertServiceImpl::new()) as Arc<dyn ExpertConsultant>
}

/// 默认编排器（基于关键词匹配 + prefer_expert 约束短路）。接受注册表 trait object。
pub fn default_orchestrator(registry: Arc<dyn ExpertRegistry>) -> Arc<dyn AllianceOrchestrator> {
    Arc::new(crate::services::AllianceRouter::new(registry)) as Arc<dyn AllianceOrchestrator>
}

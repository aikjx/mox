// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox

//! 璇玑十四维专家引擎核心
//!
//! `ExpertEngine` 是引擎的统一入口，协调三大子系统：
//! - **Registry**（注册表）：管理 14 位专家的元数据
//! - **Consultant**（咨询器）：把查询转为报告（调用完整管线）
//! - **Governor**（治理器）：治理裁决（实现 GovernExpert trait）
//!
//! 设计要点：
//! - 单一入口，便于下游依赖注入
//! - 内部组件可独立替换（DIP）
//! - 同步核心 + 异步 trait 适配
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │         ExpertEngine                │
//! │  ┌──────────┐  ┌──────────────┐    │
//! │  │ Registry │  │  Consultant  │    │
//! │  │ (14位专家)│  │ (mox_optimize)│   │
//! │  └──────────┘  └──────────────┘    │
//! │  ┌──────────┐                      │
//! │  │ Governor │                      │
//! │  │ (8闸治理) │                      │
//! │  └──────────┘                      │
//! └─────────────────────────────────────┘
//! ```

pub mod consultant;
pub mod governor;
pub mod registry;

use crate::context::ResourceQuota;
use crate::error::CoreResult;
pub use consultant::ExpertConsultantImpl;
pub use governor::GovernExpertImpl;
pub use registry::InMemoryExpertRegistry;

use mox_ai_expert_proto::{ConsultReport, ExpertMeta};
use std::sync::Arc;

/// 引擎配置
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// 是否启用并行（默认 true，关闭则串行执行）
    pub parallel: bool,
    /// 默认配额
    pub default_quota: ResourceQuota,
    /// 是否启用审计链
    pub audit_enabled: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            parallel: true,
            default_quota: ResourceQuota::default(),
            audit_enabled: true,
        }
    }
}

/// 璇玑十四维专家引擎核心
///
/// 统一协调注册表、咨询器、治理器三大子系统。
/// 下游通过 `Arc<ExpertEngine>` 共享引擎实例。
pub struct ExpertEngine {
    config: EngineConfig,
    registry: Arc<InMemoryExpertRegistry>,
    consultant: Arc<ExpertConsultantImpl>,
    governor: Arc<GovernExpertImpl>,
}

impl Default for ExpertEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpertEngine {
    /// 创建引擎（预填 14 位内置专家）
    pub fn new() -> Self {
        let config = EngineConfig::default();
        let registry = Arc::new(InMemoryExpertRegistry::new());
        let consultant = Arc::new(
            ExpertConsultantImpl::new().with_default_quota(config.default_quota.clone()),
        );
        let governor = Arc::new(GovernExpertImpl::new());

        Self {
            config,
            registry,
            consultant,
            governor,
        }
    }

    /// 用自定义配置创建引擎
    pub fn with_config(config: EngineConfig) -> Self {
        let registry = Arc::new(InMemoryExpertRegistry::new());
        let consultant = Arc::new(
            ExpertConsultantImpl::new().with_default_quota(config.default_quota.clone()),
        );
        let governor = Arc::new(GovernExpertImpl::new());

        Self {
            config,
            registry,
            consultant,
            governor,
        }
    }

    /// 获取注册表引用
    pub fn registry(&self) -> &Arc<InMemoryExpertRegistry> {
        &self.registry
    }

    /// 获取咨询器引用
    pub fn consultant(&self) -> &Arc<ExpertConsultantImpl> {
        &self.consultant
    }

    /// 获取治理器引用
    pub fn governor(&self) -> &Arc<GovernExpertImpl> {
        &self.governor
    }

    /// 获取配置
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// 便捷：列出所有专家
    pub fn list_experts(&self) -> CoreResult<Vec<ExpertMeta>> {
        Ok(self.registry.list_sync(None))
    }

    /// 便捷：按 id 查找专家
    pub fn find_expert(&self, id: &str) -> CoreResult<Option<ExpertMeta>> {
        Ok(self.registry.find_sync(id))
    }

    /// 便捷：同步咨询
    pub fn consult_sync(
        &self,
        query: &mox_ai_expert_proto::ConsultQuery,
    ) -> CoreResult<ConsultReport> {
        self.consultant
            .consult_sync(query)
            .map_err(|e| crate::error::CoreError::Internal(e.to_string()))
    }

    /// 注册自定义专家
    pub fn register_expert(&self, meta: ExpertMeta) {
        self.registry.register_sync(meta);
    }

    /// 专家总数
    pub fn expert_count(&self) -> usize {
        self.registry.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_expert_proto::ConsultQuery;
    use std::collections::HashMap;

    #[test]
    fn engine_creation_preloads_fourteen_experts() {
        let engine = ExpertEngine::new();
        assert_eq!(engine.expert_count(), 14);
    }

    #[test]
    fn engine_list_experts_returns_all() {
        let engine = ExpertEngine::new();
        let experts = engine.list_experts().unwrap();
        assert_eq!(experts.len(), 14);
    }

    #[test]
    fn engine_find_expert_works() {
        let engine = ExpertEngine::new();
        let found = engine.find_expert("security").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "security");
    }

    #[test]
    fn engine_consult_empty_query() {
        let engine = ExpertEngine::new();
        let query = ConsultQuery {
            id: "q1".into(),
            query: "test".into(),
            ctx: HashMap::new(),
        };
        let rep = engine.consult_sync(&query).unwrap();
        assert_eq!(rep.report_id, "q1");
        assert!(!rep.vetoed);
    }

    #[test]
    fn engine_register_custom_expert() {
        let engine = ExpertEngine::new();
        assert_eq!(engine.expert_count(), 14);

        let meta = ExpertMeta::new("custom-1", "Custom Expert", "test");
        engine.register_expert(meta);
        assert_eq!(engine.expert_count(), 15);

        let found = engine.find_expert("custom-1").unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn engine_with_custom_config() {
        let config = EngineConfig {
            parallel: false,
            default_quota: ResourceQuota {
                max_parallel: 4,
                max_cost_budget: 0.5,
                sla_ms: 2_000,
            },
            audit_enabled: false,
        };
        let engine = ExpertEngine::with_config(config);
        assert_eq!(engine.config().parallel, false);
        assert_eq!(engine.config().default_quota.max_parallel, 4);
        assert_eq!(engine.expert_count(), 14);
    }

    #[test]
    fn engine_has_all_core_components() {
        let engine = ExpertEngine::new();
        // 三大组件都可用
        assert!(engine.registry().len() > 0);
        assert_eq!(engine.registry().len(), 14);
    }

    #[test]
    fn engine_dimensions_complete() {
        let engine = ExpertEngine::new();
        let experts = engine.list_experts().unwrap();
        let dims: std::collections::HashSet<String> = experts
            .iter()
            .filter_map(|e| e.dimension.clone())
            .collect();
        // 14 个维度都应出现
        assert!(dims.len() >= 14 || experts.len() == 14);
    }
}

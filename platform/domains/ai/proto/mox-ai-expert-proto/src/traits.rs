// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 领域抽象 trait（DIP 反转：下游只依赖这些 trait 抽象，不依赖具体实现）
//!
//! ## 设计
//! - `ExpertRegistry`：专家注册 / 查询（可替代直接依赖内部专家列表）
//! - `ExpertConsultant`：单次咨询接口（替代直接依赖 mox_optimize）
//! - `AllianceOrchestrator`：联盟编排 / 任务路由（替代直接依赖裁决、路由等内部实现）
//!
//! 所有 trait 都要求 `Send + Sync`，使用 `async_trait` 以便 `Arc<dyn Trait>`
//! 在多线程环境中共享。
//!
//! 下游改依赖 `Arc<dyn ExpertConsultant>` 等后，即可通过 Mock 实现
//! 脱离 mox-expert concrete 做独立测试，完成 DIP。

use crate::types::{ConsultQuery, ConsultReport, ExpertMeta, RoutingDecision, TaskSpec};
use crate::error::ExpertResult;
use async_trait::async_trait;
use std::sync::Arc;

// ============================================================================
// ExpertRegistry — 专家注册 / 列表 / 查找
// ============================================================================

/// 专家注册表抽象。对外暴露「注册 / 按域列出 / 按 id 查找」三个原语。
///
/// 实现可以是：
/// - 真实内置注册表（从 14 位专家映射）
/// - 外部配置文件驱动的静态注册表（如 experts.json）
/// - 或 Mock（测试用）
#[async_trait]
pub trait ExpertRegistry: Send + Sync {
    /// 注册一位专家到注册表中。若 id 重复则实现可覆盖或返回 Err。
    async fn register(&self, expert: &ExpertMeta) -> ExpertResult<()>;

    /// 按 domain 列出专家；`domain = None` 表示列出全部。
    async fn list(&self, domain: Option<&str>) -> ExpertResult<Vec<ExpertMeta>>;

    /// 按 id 查找专家；不存在时返回 `Ok(None)`。
    async fn find(&self, id: &str) -> ExpertResult<Option<ExpertMeta>>;
}

// ============================================================================
// ExpertConsultant — 咨询接口
// ============================================================================

/// 专家咨询抽象：把 `ConsultQuery` 变成 `ConsultReport`。
///
/// 真实实现：把 ConsultQuery 映射到 FlowGraph + GovernContext，调用优化引擎，
/// 再把内部报告归一化为 `ConsultReport`。
///
/// 下游改为持有 `Arc<dyn ExpertConsultant>`，即可在测试时替换为 MockExpert，
/// 不再需要引入完整引擎构建开销。
#[async_trait]
pub trait ExpertConsultant: Send + Sync {
    /// 异步咨询
    async fn consult(&self, query: &ConsultQuery) -> ExpertResult<ConsultReport>;

    /// 同步便捷：对同步调用者包装 consult()。
    ///
    /// 默认实现：创建当前线程 tokio runtime 并 block_on 异步 consult()。
    /// 具体实现可覆写，使用原生同步 consult_sync 以省去 runtime 开销。
    fn consult_blocking(&self, query: &ConsultQuery) -> ExpertResult<ConsultReport> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| mox_error::MoxError::internal(
                mox_error::ErrorDomain::Ai, 11, 099,
                format!("启动 tokio runtime 失败: {}", e)
            ))?;
        rt.block_on(self.consult(query))
    }
}

// ============================================================================
// AllianceOrchestrator — 联盟编排 / 任务路由
// ============================================================================

/// 联盟编排器抽象：给定任务规格（TaskSpec），返回最合适的专家路由决策。
///
/// 真实实现：按 scenario + constraints 匹配 experts，得到 `RoutingDecision`。
/// Mock 实现：直接返回固定 expert_id + confidence=1.0，便于测试下游逻辑。
#[async_trait]
pub trait AllianceOrchestrator: Send + Sync {
    async fn route(&self, task: &TaskSpec) -> ExpertResult<RoutingDecision>;
}

// ============================================================================
// 工厂函数 — 获取默认 trait object
// ============================================================================
//
// 下游调用这些函数获得默认 trait object，从而无需 use 具体 struct 名字。
// 下游只会 `use mox_ai_expert_proto::traits::default_consultant`，走 trait 模块。
//
// 注意：具体实现放在 mox-ai-expert-core 中，通过 feature flag 或外部装配注入。
// 这里只提供 trait 对象的类型别名和工厂签名。

/// 注册表 trait 对象类型别名
pub type RegistryRef = Arc<dyn ExpertRegistry>;

/// 咨询器 trait 对象类型别名
pub type ConsultantRef = Arc<dyn ExpertConsultant>;

/// 编排器 trait 对象类型别名
pub type OrchestratorRef = Arc<dyn AllianceOrchestrator>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ExpertMeta;

    /// Mock 注册表：验证 trait 可独立实现（无需 concrete 依赖）
    struct MockRegistry {
        experts: std::sync::Mutex<Vec<ExpertMeta>>,
    }

    #[async_trait]
    impl ExpertRegistry for MockRegistry {
        async fn register(&self, expert: &ExpertMeta) -> ExpertResult<()> {
            self.experts.lock().unwrap().push(expert.clone());
            Ok(())
        }
        async fn list(&self, _domain: Option<&str>) -> ExpertResult<Vec<ExpertMeta>> {
            Ok(self.experts.lock().unwrap().clone())
        }
        async fn find(&self, id: &str) -> ExpertResult<Option<ExpertMeta>> {
            Ok(self
                .experts
                .lock()
                .unwrap()
                .iter()
                .find(|e| e.id == id)
                .cloned())
        }
    }

    #[tokio::test]
    async fn mock_registry_works_without_concrete() {
        let reg = MockRegistry {
            experts: std::sync::Mutex::new(Vec::new()),
        };
        let meta = ExpertMeta::new("mock", "Mock", "test");
        reg.register(&meta).await.unwrap();
        let found = reg.find("mock").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Mock");
    }
}

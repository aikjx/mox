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
//! 所有 trait 都要求 `Send + Sync`，并使用 `async_trait` 以便 `Arc<dyn Trait>`
//! 在多线程环境中共享。
//!
//! 下游（hermes-flow-bridge / business-catalog）改依赖 `Arc<dyn ExpertConsultant>` 等后，
//! 即可通过 Mock 实现脱离 mox-expert concrete 做独立测试，完成 DIP。
//!
//! 注意：trait 方法的 Result 类型目前为 `anyhow::Result`（与 expert-svc 原始签名保持一致），
//! 统一错误类型（ExpertResult）将在后续阶段替换，当前阶段不引入破坏性变更。

use crate::types::*;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// ExpertRegistry：专家注册 / 列表 / 查找
// ---------------------------------------------------------------------------

/// 专家注册表抽象。对外暴露「注册 / 按域列出 / 按 id 查找」三个原语。
///
/// 实现可以是：
/// - 真实 mox-expert 内置注册表（把 `experts::all_experts()` 转为 `ExpertMeta` 返回）
/// - 外部配置文件驱动的静态注册表（如 experts.json）
/// - 或 Mock（测试用）
#[async_trait]
pub trait ExpertRegistry: Send + Sync {
    /// 注册一位专家到注册表中。若 id 重复则实现可覆盖或返回 Err。
    async fn register(&self, expert: &ExpertMeta) -> Result<()>;

    /// 按 domain 列出专家；`domain = None` 表示列出全部。
    async fn list(&self, domain: Option<&str>) -> Result<Vec<ExpertMeta>>;

    /// 按 id 查找专家；不存在时返回 `Ok(None)`。
    async fn find(&self, id: &str) -> Result<Option<ExpertMeta>>;
}

// ---------------------------------------------------------------------------
// ExpertConsultant：咨询接口（把输入请求转换为报告输出）
// ---------------------------------------------------------------------------

/// 专家咨询抽象：把 `ConsultQuery` 变成 `ConsultReport`。
///
/// 真实实现：把 ConsultQuery 映射到 FlowGraph + GovernContext，调用 `mox_optimize`，
/// 再把 `GovernanceReport` 归一化为 `ConsultReport`。
/// 下游（hermes-flow-bridge / business-catalog）改为持有 `Arc<dyn ExpertConsultant>`，
/// 即可在测试时替换为 MockExpert，不再需要引入 mox-expert 的 full engine 构建开销。
#[async_trait]
pub trait ExpertConsultant: Send + Sync {
    async fn consult(&self, query: &ConsultQuery) -> Result<ConsultReport>;

    /// 同步便捷：对同步调用者（如 std 线程、同步测试）包装 consult()。
    ///
    /// 默认实现：创建当前线程 tokio runtime 并 block_on 异步 consult()。
    /// 具体实现（如 `ExpertServiceImpl`）可覆写，使用原生同步 consult_sync
    /// 以省去 runtime 开销。
    fn consult_blocking(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        // 创建一个最小的 current-thread runtime（一次性开销），用于桥接同步调用。
        // 真实生产环境建议直接用异步 consult() 共享 runtime。
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| anyhow::anyhow!("启动 tokio runtime 失败: {}", e))?;
        rt.block_on(self.consult(query))
    }
}

// ---------------------------------------------------------------------------
// AllianceOrchestrator：联盟编排 / 任务路由
// ---------------------------------------------------------------------------

/// 联盟编排器抽象：给定任务规格（TaskSpec），返回最合适的专家路由决策。
///
/// 真实实现：按 scenario + constraints 匹配 experts，得到 `RoutingDecision`。
/// Mock 实现：直接返回固定 expert_id + confidence=1.0，便于测试下游逻辑。
#[async_trait]
pub trait AllianceOrchestrator: Send + Sync {
    async fn route(&self, task: &TaskSpec) -> Result<RoutingDecision>;
}

// ---------------------------------------------------------------------------
// trait 对象类型别名（下游便捷使用）
// ---------------------------------------------------------------------------

/// 注册表 trait 对象类型别名
pub type RegistryRef = Arc<dyn ExpertRegistry>;

/// 咨询器 trait 对象类型别名
pub type ConsultantRef = Arc<dyn ExpertConsultant>;

/// 编排器 trait 对象类型别名
pub type OrchestratorRef = Arc<dyn AllianceOrchestrator>;

// ---------------------------------------------------------------------------
// 测试：验证 trait 可独立实现（无需 concrete 依赖）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock 注册表：验证 trait 可独立实现（无需 concrete 依赖）
    struct MockRegistry {
        experts: std::sync::Mutex<Vec<ExpertMeta>>,
    }

    #[async_trait]
    impl ExpertRegistry for MockRegistry {
        async fn register(&self, expert: &ExpertMeta) -> Result<()> {
            self.experts.lock().unwrap().push(expert.clone());
            Ok(())
        }
        async fn list(&self, _domain: Option<&str>) -> Result<Vec<ExpertMeta>> {
            Ok(self.experts.lock().unwrap().clone())
        }
        async fn find(&self, id: &str) -> Result<Option<ExpertMeta>> {
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

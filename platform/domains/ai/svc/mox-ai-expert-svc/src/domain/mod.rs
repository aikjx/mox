// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 治理领域对外抽象 trait（DIP 层）。
//!
//! 下游（hermes / business-catalog / mox-expert L3 服务）仅依赖本文件 trait 抽象，
//! 不直接 `use crate::context::GovernContext` 或 `crate::govern::govern` 具体实现。
//!
//! - `GovernContext`：治理运行上下文抽象（租户 / 主体 / 配额 / 合规标志只读）
//! - `GovernExpert`：治理专家抽象（给定流程图 + 上下文 → 治理裁决）
//!
//! 通过把 GovernContext + GovernExpert 抽成 trait，下游可在测试中用
//! `MockGovernContext + MockGovernExpert` 脱离真实引擎做轻量 DIP 验证。
//!
//! P2 架构解耦 · 阶段 1.5：
//! - `GovernContext` trait / `GovernLevel` / `GovernVerdict` / `MinimalGovernContext`
//!   已迁移至 `mox-ai-expert-proto`，本模块通过 re-export 保持对外 100% 兼容。
//! - `GovernExpert` trait 保留本地定义（使用 `&FlowGraph` 签名），
//!   proto 版本使用 `&dyn std::any::Any` 实现更彻底的 DIP，
//!   将在后续阶段统一迁移。

use async_trait::async_trait;
use mox_ai_flow_svc::model::FlowGraph;

// ---------------------------------------------------------------------------
// 从 mox-ai-expert-proto 重新导出（SSOT 单一真相源）
// ---------------------------------------------------------------------------

pub use mox_ai_expert_proto::{
    GovernContext, GovernLevel, GovernVerdict, MinimalGovernContext,
};

// ============================================================================
// GovernExpert：治理专家抽象（FlowGraph + GovernContext → GovernVerdict）
//
// 注意：保留本地 FlowGraph 版本以保持向后兼容。
// proto 版本使用 &dyn std::any::Any 以实现更彻底的 DIP（不依赖 FlowGraph 具体类型），
// 将在后续阶段统一迁移。
// ============================================================================

/// 治理专家：把 FlowGraph 与 GovernContext 映射为 GovernVerdict。
///
/// - 真实实现：调用 `crate::govern::govern` + 七维专家进行严格裁决。
/// - Mock 实现：直接返回 `Pass(1.0)` 或根据简单规则返回 `Block`。
///
/// trait 方法命名 `govern()` 与 concrete `crate::govern::govern` 函数同名，
/// 但对外通过 trait 调用，避免下游反向依赖 crate 内部模块。
#[async_trait]
pub trait GovernExpert: Send + Sync {
    async fn govern(&self, graph: &FlowGraph, ctx: &dyn GovernContext) -> GovernVerdict;

    /// 同步便捷：当前线程 block_on govern()。适合同步下游 / 测试。
    fn govern_blocking(&self, graph: &FlowGraph, ctx: &dyn GovernContext) -> GovernVerdict {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("govern_blocking: tokio runtime 构造失败");
        rt.block_on(self.govern(graph, ctx))
    }
}

// ============================================================================
// MockGovernExpert：永远 Pass 的 Mock（用于 DIP 证据；脱离真实引擎）
//
// 注意：保留本地版本，使用 FlowGraph 签名的 GovernExpert trait。
// proto 版本的 MockGovernExpert 使用 &dyn Any 签名，将在后续阶段统一迁移。
// ============================================================================

pub struct MockGovernExpert {
    pub forced_level: GovernLevel,
    pub fixed_score: f64,
}

impl Default for MockGovernExpert {
    fn default() -> Self {
        Self {
            forced_level: GovernLevel::Pass,
            fixed_score: 1.0,
        }
    }
}

#[async_trait]
impl GovernExpert for MockGovernExpert {
    async fn govern(&self, _graph: &FlowGraph, _ctx: &dyn GovernContext) -> GovernVerdict {
        GovernVerdict {
            level: self.forced_level.clone(),
            score: self.fixed_score,
            reasons: vec!["[MockGovernExpert] DIP 证据：无 concrete govern 调用".into()],
            gate_id: "mock-gate".into(),
        }
    }

    fn govern_blocking(&self, _graph: &FlowGraph, _ctx: &dyn GovernContext) -> GovernVerdict {
        GovernVerdict {
            level: self.forced_level.clone(),
            score: self.fixed_score,
            reasons: vec!["[MockGovernExpert] sync DIP 证据（无需 tokio）".into()],
            gate_id: "mock-gate-sync".into(),
        }
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

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

use async_trait::async_trait;
use mox_ai_flow_svc::model::FlowGraph;
use std::fmt;

// ============================================================================
// GovernVerdict：治理裁决（共享投影值类型，可脱离 concrete 使用）
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernLevel {
    Pass,
    Warn,
    Block,
}

#[derive(Debug, Clone)]
pub struct GovernVerdict {
    pub level: GovernLevel,
    pub score: f64,
    pub reasons: Vec<String>,
    pub gate_id: String,
}

impl Default for GovernVerdict {
    fn default() -> Self {
        Self {
            level: GovernLevel::Pass,
            score: 1.0,
            reasons: Vec::new(),
            gate_id: "default".into(),
        }
    }
}

impl fmt::Display for GovernVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:?}] score={:.3} gate={} reasons={}",
            self.level,
            self.score,
            self.gate_id,
            self.reasons.len()
        )
    }
}

// ============================================================================
// GovernContext：治理上下文只读抽象
// ============================================================================

/// 治理运行上下文：对外暴露只读视图，不依赖具体 GovernContext 结构体字段布局。
pub trait GovernContext: Send + Sync {
    /// 租户 id（多租户隔离）。
    fn tenant(&self) -> &str;
    /// 命名空间（租户内次级隔离）。
    fn namespace(&self) -> &str;
    /// 执行主体/提交者 id。
    fn principal(&self) -> &str;
    /// 主体角色列表（用于权限级别治理）。
    fn roles(&self) -> &[String];
    /// 是否强合规租户（政务 / 金融 → 更严格 Block 阈值）。
    fn is_regulated(&self) -> bool;
    /// 并行度上限（专家治理时约束算子并发）。
    fn max_parallel(&self) -> u32;
    /// 总费用预算（治理时否决超限方案）。
    fn cost_budget(&self) -> f64;
    /// SLA 耗时上限（毫秒）。
    fn sla_ms(&self) -> u64;
}

// ============================================================================
// GovernExpert：治理专家抽象（FlowGraph + GovernContext → GovernVerdict）
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
            .expect("govern_blocking: tokio mox_platform_orchestrator_svc 构造失败");
        rt.block_on(self.govern(graph, ctx))
    }
}

// ============================================================================
// 工具：MinimalContext（最小 GovernContext 实现；用于测试 / 简易场景；纯 std）
// ============================================================================

pub struct MinimalGovernContext {
    pub tenant: String,
    pub namespace: String,
    pub principal: String,
    pub roles: Vec<String>,
    pub regulated: bool,
    pub max_parallel: u32,
    pub cost_budget: f64,
    pub sla_ms: u64,
}

impl Default for MinimalGovernContext {
    fn default() -> Self {
        Self {
            tenant: "default-tenant".into(),
            namespace: "default".into(),
            principal: "anon".into(),
            roles: vec!["viewer".into()],
            regulated: false,
            max_parallel: 8,
            cost_budget: 100.0,
            sla_ms: 50_000,
        }
    }
}

impl GovernContext for MinimalGovernContext {
    fn tenant(&self) -> &str {
        &self.tenant
    }
    fn namespace(&self) -> &str {
        &self.namespace
    }
    fn principal(&self) -> &str {
        &self.principal
    }
    fn roles(&self) -> &[String] {
        &self.roles
    }
    fn is_regulated(&self) -> bool {
        self.regulated
    }
    fn max_parallel(&self) -> u32 {
        self.max_parallel
    }
    fn cost_budget(&self) -> f64 {
        self.cost_budget
    }
    fn sla_ms(&self) -> u64 {
        self.sla_ms
    }
}

// ============================================================================
// MockGovernExpert：永远 Pass 的 Mock（用于 DIP 证据；脱离真实引擎）
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

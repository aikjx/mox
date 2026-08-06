//! 开发专家联盟 · 全维处理工具流程图
//!
//! 七位领域专家在归一化 IR 上并行诊断 → 裁决器按「权限/安全优先」全维归一 →
//! flow-ai 引擎做已验证的最优求解 → 治理层把关后出码。
//! 兼容 MCP / Skills / Loops / 大模型。

pub mod context;
pub mod expert;
pub mod experts;
pub mod govern;
pub mod ir;
pub mod pipeline;
pub mod programming;
pub mod executor;
pub mod reconcile;
pub mod server;
pub mod verify;
/// 多场景 Benchmark：用真实引擎量化核心收益（产品页可复用证据）
pub mod bench;

pub use context::{CompatibilityRegistry, GovernContext, LoopGuard, LoopPolicy, McpTool, Principal, ResourceQuota, SkillRef, Tenant};
pub use expert::{dispatch, Constraint, Expert, ExpertOpinion, Risk, Suggestion};
pub use govern::{apply_rules, govern, AuditChain, AuditEvent, FlowStatus, GateResult};
pub use ir::{auto_dimension, Dimension, DimensionTag, DimensionedFlow};
pub use pipeline::{alliance_optimize, GovernanceReport};
pub use reconcile::{reconcile, ReconciledPlan, ReconcileConflict};
pub use verify::{verify, AlgoVerification, Check};

/// 便捷重导出 flow-ai 的公共类型
pub mod flow {
    pub use flow_ai::model::*;
    pub use flow_ai::pipeline::{optimize, OptimizeConfig, OptimizationReport};
    pub use flow_ai::schedule::{route_models, ModelTier, Schedule};
    pub use flow_ai::topology::{TopologyGraph, EntityKind};
}

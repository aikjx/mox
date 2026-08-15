//! 璇玑 · 全维处理工具流程图
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
/// 外部审计 Sink：Syslog / S3(WORM) / Kafka，满足 SOC2/GDPR 合规要求
pub mod audit;
/// RBAC 引擎：资源级权限控制，多角色继承链，跨租户隔离
pub mod rbac;
/// 流程 YAML 外部化：业务人员用 YAML 增删改流程，无需写 Rust 代码
pub mod flow_loader;
/// 插件化运行时：参考 DeepSeek Harness "Everything is a Plugin" 范式的共享上下文与瀑布扩展点
pub mod harness;

pub use context::{CompatibilityRegistry, GovernContext, LoopGuard, LoopPolicy, McpTool, Principal, ResourceQuota, SkillRef, Tenant};
pub use expert::{dispatch, Constraint, Expert, ExpertOpinion, Risk, Suggestion};
pub use govern::{apply_rules, govern, AuditChain, AuditEvent, FlowStatus, GateResult};
pub use ir::{auto_dimension, Dimension, DimensionTag, DimensionedFlow};
pub use pipeline::{alliance_optimize, GovernanceReport};
pub use reconcile::{reconcile, ReconciledPlan, ReconcileConflict};
pub use verify::{verify, AlgoVerification, Check};
pub use audit::{
    AuditContext, AuditSink, AuditError,
    ExtAuditEvent, AuditAction, AuditOutcome,
    AuditSeverity, AuditActor, AuditResource,
    SyslogSink, S3Sink, NatsSink, RabbitMqSink, NoopSink, MultiSink, FlushPolicy,
};
pub use rbac::{
    check, check_with_audit, PermissionCheck, PermissionResult,
    RbacPolicy, Permission, RbacError,
};
pub use rbac::check::Resource;
pub use flow_loader::{
    FlowLoader, FlowLoadError, FlowDef, NodeDef, YamlEdgeDef,
    ValidationError, YamlFlowLoader,
};
pub use harness::{
    HarnessCtx, HarnessProfile, Plugin, PluginMeta, ExpertPlugin,
    WaterfallEvent, WaterfallState, ModelAdapterConfig,
    expert_plugins, run_experts,
};

/// 便捷重导出 flow-ai 的公共类型
pub mod flow {
    pub use flow_ai::model::*;
    pub use flow_ai::pipeline::{optimize, OptimizeConfig, OptimizationReport};
    pub use flow_ai::schedule::{route_models, ModelTier, Schedule};
    pub use flow_ai::topology::{TopologyGraph, EntityKind};
}

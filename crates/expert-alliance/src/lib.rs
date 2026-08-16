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
/// 敏感度判定单一权威源（SSOT）：根治 P1 三处分叉
pub mod sensitivity;
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

// ===================== 全维归一化常量（SSOT） =====================
//
// 维度优先级、冲突升级门槛、归一化阈值等"魔法数字"此前散落在 expert.rs / reconcile.rs
// 等多处，违反单一权威源原则，易产生维护漂移。此处集中定义，全局引用。

/// 维度优先级（数值越大越优先）。权限/安全必须压过性能/成本。
/// 与 `Dimension::priority()` 保持一致，是 `priority()` 的单一数据源。
pub const DIM_PRIORITY: &[(Dimension, i32)] = &[
    (Dimension::Permission, 100),
    (Dimension::Security,   100),
    (Dimension::Resource,   70),
    (Dimension::Data,       60),
    (Dimension::Algorithm,  50),
    (Dimension::Business,   40),
    (Dimension::Observability, 30),
    // ---- 开发七维（与业务七维同尺度，跨层不互盖）----
    (Dimension::Architecture,    100),
    (Dimension::SecurityCode,    100),
    (Dimension::CodeQuality,      70),
    (Dimension::Performance,      60),
    (Dimension::Testing,          50),
    (Dimension::Documentation,    40),
    (Dimension::Maintainability,  30),
];

/// 维度激活门槛：归一化置信度低于该值则该维度不计入裁决。
/// 可观测维度门槛略低（噪声大），业务维度略高（需更确信）。
pub const DIM_THRESHOLD: &[(Dimension, f64)] = &[
    (Dimension::Permission,  0.5),
    (Dimension::Security,    0.5),
    (Dimension::Resource,    0.5),
    (Dimension::Data,        0.5),
    (Dimension::Algorithm,   0.5),
    (Dimension::Business,    0.6),
    (Dimension::Observability, 0.4),
    // ---- 开发七维 ----
    (Dimension::Architecture,    0.5),
    (Dimension::SecurityCode,    0.5),
    (Dimension::CodeQuality,     0.5),
    (Dimension::Performance,     0.5),
    (Dimension::Testing,         0.5),
    (Dimension::Documentation,   0.6),
    (Dimension::Maintainability, 0.5),
];

/// 冲突升级门槛：同类别约束且优先级差 < 该值才判为 escalated（Blocking）。
/// 优先级差 ≥ 该值视为高优先维度合法压过低优先维度，不升级。
pub const CONFLICT_ESCALATE_PRIORITY_GAP: i32 = 1;

/// 归一化默认可调权重（用于裁决器多目标折中，数值越大权重越高）。
pub const NORMALIZATION_WEIGHTS: &[(Dimension, f64)] = &[
    (Dimension::Permission, 1.0),
    (Dimension::Security,   1.0),
    (Dimension::Resource,   0.8),
    (Dimension::Data,       0.8),
    (Dimension::Algorithm,  0.7),
    (Dimension::Business,   0.6),
    (Dimension::Observability,0.5),
    // ---- 开发七维 ----
    (Dimension::Architecture,    1.0),
    (Dimension::SecurityCode,    1.0),
    (Dimension::CodeQuality,     0.8),
    (Dimension::Performance,     0.8),
    (Dimension::Testing,         0.7),
    (Dimension::Documentation,   0.6),
    (Dimension::Maintainability, 0.5),
];

/// 便捷查询：取维度优先级（缺省 0）。
pub fn dim_priority(dim: Dimension) -> i32 {
    DIM_PRIORITY.iter().find(|(d, _)| *d == dim).map(|(_, p)| *p).unwrap_or(0)
}

/// 便捷查询：取维度激活门槛（缺省 0.5）。
pub fn dim_threshold(dim: Dimension) -> f64 {
    DIM_THRESHOLD.iter().find(|(d, _)| *d == dim).map(|(_, t)| *t).unwrap_or(0.5)
}

/// 便捷重导出 flow-ai 的公共类型
pub mod flow {
    pub use flow_ai::model::*;
    pub use flow_ai::pipeline::{optimize, OptimizeConfig, OptimizationReport};
    pub use flow_ai::schedule::{route_models, ModelTier, Schedule};
    pub use flow_ai::topology::{TopologyGraph, EntityKind};
}

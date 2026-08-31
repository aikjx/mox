// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 璇玑 · 全维处理工具流程图
//!
//! 七位领域专家在归一化 IR 上并行诊断 → 裁决器按「权限/安全优先」全维归一 →
//! flow-ai 引擎做已验证的最优求解 → 治理层把关后出码。
//! 兼容 MCP / Skills / Loops / 大模型。

pub const CRATE_ID: &str = "50bb6200-04c5-5e4c-8354-4c6e1b230024";
pub const ENGINE_NAME: &str = "mox::mox_expert";
pub const CRATE_META: mox_platform_foundation::CrateMeta = mox_platform_foundation::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: mox_platform_foundation::AisLayer::L4Services,
    owner: "mox-core",
};

/// 外部审计 Sink：Syslog / S3(WORM) / Kafka，满足 SOC2/GDPR 合规要求
pub mod audit;
/// 专家联盟 6 阶段全维分析管线（新增，enterprise/18 BP-05 Rust 单一真相化）
pub mod alliance;
/// 多场景 Benchmark：用真实引擎量化核心收益（产品页可复用证据）
pub mod bench;
pub mod context;
pub mod executor;
pub mod expert;
pub mod experts;
/// 流程 YAML 外部化：业务人员用 YAML 增删改流程，无需写 Rust 代码
pub mod flow_loader;
pub mod govern;
/// 插件化运行时：参考 DeepSeek Harness "Everything is a Plugin" 范式的共享上下文与瀑布扩展点
pub mod harness;
pub mod ir;
pub mod llm;
pub mod pipeline;
pub mod programming;
/// RBAC 引擎：资源级权限控制，多角色继承链，跨租户隔离
pub mod rbac;
pub mod reconcile;
/// 敏感度判定单一权威源（SSOT）：根治 P1 三处分叉
pub mod sensitivity;
pub mod server;
/// 租户策略分层 + 治理 8 闸门全量门禁（I-06 / G3·G6·G8 补全）
pub mod tenant_policy;
pub mod verify;

/// L3 对外领域抽象：GovernExpert / GovernContext trait + MinimalGovernContext / MockGovernExpert
/// （Govern* 从 concrete context/govern 模块解耦到 domain trait，实现 DIP）。
pub mod domain;

// ============================================================================
// L3 领域层对外抽象（DIP 反转：下游只依赖 expert_traits::* 与 types::* ，
// 不再依赖下面的 concrete struct 名字）。
// ============================================================================

/// L3 对外抽象 trait：`ExpertRegistry` / `ExpertConsultant` / `AllianceOrchestrator`
pub mod expert_traits;
/// 对外 trait 的 concrete 实现：`RegistryImpl` / `ExpertServiceImpl` / `AllianceRouter`
/// （下游不直接 use 这些名字，除非构建阶段装配依赖注入）。
pub mod services;
/// 共享数据类型：三个 trait 的入参/出参统一使用本模块。
pub mod types;

pub use audit::{
    AuditAction, AuditActor, AuditContext, AuditError, AuditOutcome, AuditResource, AuditSeverity,
    AuditSink, ExtAuditEvent, FlushPolicy, MultiSink, NoopSink, S3Sink, SyslogSink,
};
pub use context::{
    CompatibilityRegistry, GovernContext, LoopGuard, LoopPolicy, McpTool, Principal, ResourceQuota,
    SkillRef, Tenant,
};
pub use expert::{dispatch, Constraint, Expert, ExpertOpinion, Risk, Suggestion};
pub use flow_loader::{
    FlowDef, FlowLoadError, FlowLoader, NodeDef, ValidationError, YamlEdgeDef, YamlFlowLoader,
};
pub use govern::{apply_rules, govern, AuditChain, AuditEvent, FlowStatus, GateResult};
pub use harness::{
    expert_plugins, run_experts, ExpertPlugin, HarnessCtx, HarnessProfile, ModelAdapterConfig,
    Plugin, PluginMeta, WaterfallEvent, WaterfallState,
};
pub use ir::{auto_dimension, Dimension, DimensionTag, DimensionedFlow};
pub use pipeline::{mox_optimize, GovernanceReport};
pub use rbac::check::Resource;
pub use rbac::{
    check, check_with_audit, Permission, PermissionCheck, PermissionResult, RbacError, RbacPolicy,
};
pub use reconcile::{reconcile, ReconcileConflict, ReconciledPlan};
pub use verify::{verify, AlgoVerification, Check};

// ===================== 全维归一化常量（SSOT） =====================
//
// P2 架构解耦 · 阶段 1.5：维度优先级、冲突升级门槛、归一化阈值等 SSOT 常量
// 已迁移至 `mox-ai-expert-proto`，此处通过 re-export 保持对外 100% 兼容。
// 维度优先级、冲突升级门槛、归一化阈值等"魔法数字"此前散落在 expert.rs / reconcile.rs
// 等多处，违反单一权威源原则，易产生维护漂移。此处集中定义，全局引用。

/// 维度优先级（数值越大越优先）。权限/安全必须压过性能/成本。
/// 与 `Dimension::priority()` 保持一致，是 `priority()` 的单一数据源。
/// 【大白话·权限功能归一化】"多个专家吵起来时听谁的"：当权限、安全、性能、成本等维度
/// 给同一件事给出互相打架的建议，按这张表排座次——Permission(权限)和 Security(安全)
/// 并列最高(100)，谁都压不过它；性能(60)、成本、体验只能往后排。同时把开发侧七维
/// (架构/代码安全/代码质量…)和业务七维放在同一把尺子上，避免两层各算各的、互相覆盖。
/// 这就是"权限功能归一化"在裁决阶段落地的核心：不是再造一套权限系统，而是让权限/安全
/// 在所有维度的归一打分里天然占最高权重。
pub use mox_ai_expert_proto::DIM_PRIORITY;

/// 维度激活门槛：归一化置信度低于该值则该维度不计入裁决。
/// 可观测维度门槛略低（噪声大），业务维度略高（需更确信）。
/// 【大白话】"没把握就不插嘴"：每个维度的建议都带一个自信度(0~1)。如果某维度自己都
/// 只有 0.4 的把握(低于门槛 0.5)，这次裁决就不带它玩，免得噪声大的维度乱带节奏。
/// 可观测类噪声大所以门槛放低，业务类影响大所以门槛略高。
pub use mox_ai_expert_proto::DIM_THRESHOLD;

/// 冲突升级门槛：同类别约束且优先级差 < 该值才判为 escalated（Blocking）。
/// 优先级差 ≥ 该值视为高优先维度合法压过低优先维度，不升级。
pub use mox_ai_expert_proto::CONFLICT_ESCALATE_PRIORITY_GAP;

/// 归一化默认可调权重（用于裁决器多目标折中，数值越大权重越高）。
pub use mox_ai_expert_proto::NORMALIZATION_WEIGHTS;

/// 便捷查询：取维度优先级（缺省 0）。
pub use mox_ai_expert_proto::dim_priority;

/// 便捷查询：取维度激活门槛（缺省 0.5）。
pub use mox_ai_expert_proto::dim_threshold;

/// 便捷查询：取维度归一化权重（缺省 0.5）。
/// P2 阶段 1.5 新增：从 mox-ai-expert-proto 引入，补充 SSOT 常量体系。
pub use mox_ai_expert_proto::dim_weight;

/// 便捷重导出 flow-ai 的公共类型
pub mod flow {
    pub use mox_ai_flow_svc::model::*;
    pub use mox_ai_flow_svc::pipeline::{optimize, OptimizationReport, OptimizeConfig};
    pub use mox_ai_flow_svc::schedule::{route_models, ModelTier, Schedule};
    pub use mox_ai_flow_svc::topology::{EntityKind, TopologyGraph};
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox

//! # Mox AI Expert Core — 璇玑十四维专家引擎核心
//!
//! P2 架构解耦 · 阶段 4：将专家引擎核心从 `mox-ai-expert-svc` 独立为 core crate。
//!
//! ## 架构定位
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │  mox-ai-expert-svc  (L4 服务层：HTTP/RPC/鉴权/审计)  │
//! └─────────────┬───────────────────────────────────────┘
//!               │ 依赖
//!               ▼
//! ┌─────────────────────────────────────────────────────┐
//! │  mox-ai-expert-core  (L5 领域核心：引擎实现)        │
//! │  ┌─────────────────────────────────────────────┐    │
//! │  │  ExpertEngine（统一入口）                   │    │
//! │  │  - Registry（14位专家注册）                 │    │
//! │  │  - Consultant（咨询 / mox_optimize）       │    │
//! │  │  - Governor（治理裁决 / 8 闸门）           │    │
//! │  └─────────────────────────────────────────────┘    │
//! │  - 归一化 IR（CodeIR / DimensionedFlow）            │
//! │  - 裁决器（按维度优先级归一合并）                    │
//! │  - 璇玑验证器（最高权限，5 项守恒不变量）            │
//! │  - 14 维归一化 / 并行调度 / 敏感度 SSOT             │
//! └─────────────┬───────────────────────────────────────┘
//!               │ 依赖
//!               ▼
//! ┌─────────────────────────────────────────────────────┐
//! │  mox-ai-expert-proto  (L5 协议层：类型 + trait 抽象)│
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## 设计原则
//!
//! - **DIP 依赖倒置**：core 依赖 proto 的 trait 抽象，实现 proto 中定义的 trait
//! - **SSOT 单一真相源**：领域类型统一使用 proto 定义，core 不重复定义
//! - **对内完整，对外收敛**：内部引擎完整可用，对外只暴露 proto trait 实现
//! - **可独立编译测试**：core crate 不依赖服务层（HTTP/RPC 等）
//!
//! ## 模块结构
//!
//! ### 引擎核心（对外主入口）
//! - [`engine`] — [`ExpertEngine`] 统一入口 + 注册表/咨询器/治理器
//!   - [`engine::InMemoryExpertRegistry`] — 内存专家注册表（实现 `ExpertRegistry`）
//!   - [`engine::ExpertConsultantImpl`] — 专家咨询器（实现 `ExpertConsultant`）
//!   - [`engine::GovernExpertImpl`] — 治理裁决器（实现 `GovernExpert`）
//!
//! ### IR 与归一化
//! - [`ir`] — 归一化 IR 模型（CodeIR / DimensionedFlow / auto_dimension）
//! - [`normalize`] — 14 维归一化（加权健康分 / 风险扣减 / 否决处理）
//!
//! ### 调度与裁决
//! - [`dispatch`] — 专家并行调度（rayon 真并行 + 结果收集）
//! - [`reconcile`] — 归一化裁决器（维度优先级仲裁 + 冲突升级）
//!
//! ### 专家集合
//! - [`expert`] — 内部 Expert trait + 类型转换工具
//! - [`experts`] — 14 位具体专家（业务七维 + 开发七维）
//!
//! ### 治理与验证
//! - [`govern`] — 治理闸门（FlowStatus / GateResult / govern 函数）
//! - [`verify`] — 璇玑验证器（最高权限，守恒不变量）
//! - [`tenant_policy`] — 租户策略 + 治理 8 闸门
//!
//! ### 基础设施
//! - [`context`] — 治理上下文（Tenant / Principal / GovernContext）
//! - [`error`] — 核心错误类型（CoreError）
//! - [`sensitivity`] — 敏感度判定 SSOT
//! - [`pipeline`] — mox 模块化系统架构处理管线（mox_optimize）

// ─── 引擎核心（对外主入口） ─────────────────────────────────────────────────

pub mod engine;

// ─── IR 与归一化 ────────────────────────────────────────────────────────────

pub mod ir;
pub mod normalize;

// ─── 调度与裁决 ────────────────────────────────────────────────────────────

pub mod dispatch;
pub mod reconcile;

// ─── 专家集合 ──────────────────────────────────────────────────────────────

pub mod expert;
pub mod experts;

// ─── 治理与验证 ────────────────────────────────────────────────────────────

pub mod govern;
pub mod verify;
pub mod tenant_policy;

// ─── 基础设施 ──────────────────────────────────────────────────────────────

pub mod context;
pub mod error;
pub mod sensitivity;
pub mod pipeline;

// ─── 重导出：引擎核心类型（最常用） ─────────────────────────────────────────

pub use engine::{EngineConfig, ExpertEngine};
pub use engine::{ExpertConsultantImpl, GovernExpertImpl, InMemoryExpertRegistry};

// ─── 重导出：核心错误 ──────────────────────────────────────────────────────

pub use error::{CoreError, CoreResult};

// ─── 重导出：IR 类型 ───────────────────────────────────────────────────────

pub use ir::{CodeIR, CodeUnit, DimensionedFlow};

// ─── 重导出：裁决类型 ──────────────────────────────────────────────────────

pub use reconcile::{ReconcileConflict, ReconciledPlan};

// ─── 重导出：治理类型 ──────────────────────────────────────────────────────

pub use govern::{FlowStatus, GateResult};

// ─── 重导出：管线类型 ──────────────────────────────────────────────────────

pub use pipeline::GovernanceReport;

// ─── 重导出：验证类型 ──────────────────────────────────────────────────────

pub use verify::AlgoVerification;

// ─── 重导出：上下文类型 ────────────────────────────────────────────────────

pub use context::{
    Capability, CompatibilityRegistry, ExpertContext, GovernContext, LoopGuard, LoopPolicy,
    McpTool, Policy, Principal, ResourceQuota, SkillRef, Tenant,
};

// ─── Crate 元数据 ──────────────────────────────────────────────────────────

pub const CRATE_ID: &str = "a1b2c3d4-0004-5e4c-8354-expertcore0001";
pub const ENGINE_NAME: &str = "mox::ai::expert::core";
pub const CRATE_META: mox_platform_foundation::CrateMeta =
    mox_platform_foundation::CrateMeta {
        id: CRATE_ID,
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        layer: mox_platform_foundation::AisLayer::L5Domain,
        owner: "mox-core",
    };

/// 便捷预导入（内部使用，外部应通过 proto::prelude）
pub mod prelude {
    pub use super::*;
}

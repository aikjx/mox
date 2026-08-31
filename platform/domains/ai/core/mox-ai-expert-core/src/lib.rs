// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

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
//! │  - 归一化 IR（CodeIR / DimensionedFlow）            │
//! │  - 十四位专家（业务七维 + 开发七维）                 │
//! │  - 裁决器（按维度优先级归一合并）                    │
//! │  - 璇玑验证器（最高权限，5 项守恒不变量）            │
//! │  - 治理闸门（8 闸全量门禁）                         │
//! │  - 核心管线（mox_optimize）                         │
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
//! - [`ir`](crate::ir) — 归一化 IR 模型（CodeIR / DimensionedFlow / auto_dimension）
//! - [`expert`](crate::expert) — 内部 Expert trait + 并行派发 dispatch
//! - [`experts`](crate::experts) — 十四位具体专家实现（TODO：后续迭代补全）
//! - [`reconcile`](crate::reconcile) — 归一化裁决器
//! - [`verify`](crate::verify) — 璇玑验证器（TODO：后续迭代补全）
//! - [`govern`](crate::govern) — 治理闸门（TODO：后续迭代补全）
//! - [`pipeline`](crate::pipeline) — mox_optimize 核心管线（TODO：后续迭代补全）
//! - [`sensitivity`](crate::sensitivity) — 敏感度判定 SSOT
//! - [`context`](crate::context) — 治理上下文（Tenant / Principal / GovernContext）
//! - [`tenant_policy`](crate::tenant_policy) — 租户策略 + 治理 8 闸门（TODO：后续迭代补全）
//! - [`services`](crate::services) — proto trait 的 concrete 实现（DIP 适配层）

// ─── 内部模块（引擎核心，不对外公开） ────────────────────────────────────────

pub mod context;
pub mod expert;
pub mod experts;
pub mod govern;
pub mod ir;
pub mod pipeline;
pub mod reconcile;
pub mod services;
pub mod sensitivity;
pub mod tenant_policy;
pub mod verify;

// ─── proto trait 实现（对外暴露的 DIP 适配层） ────────────────────────────────

pub use services::{ConcreteGovernExpert, ExpertServiceImpl, RegistryImpl};

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

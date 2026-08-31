// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox AI Expert Proto — 璇玑专家服务协议层
//!
//! 本 crate 是璇玑专家领域的 **单一真相源（SSOT）协议定义**，所有下游 crate
//! 只依赖本 crate 的类型与 trait 抽象，不依赖具体实现。
//!
//! ## 设计原则
//! - **DIP 依赖倒置**：下游依赖抽象，不依赖 concrete 实现
//! - **SSOT 单一真相源**：每个领域概念只有一个权威定义
//! - **零具体实现**：本 crate 只定义类型、trait 和常量，不含任何业务逻辑
//!
//! ## 模块结构
//! - [`types`] — 领域值类型（Dimension, ExpertMeta, ConsultQuery...）
//! - [`traits`] — 领域抽象 trait（ExpertRegistry, ExpertConsultant...）
//! - [`domain`] — 治理领域抽象（GovernContext, GovernExpert, GovernVerdict）
//! - [`error`] — 统一错误类型（基于 mox-error 的专家域错误码）
//! - [`events`] — 领域事件协议
//! - [`constants`] — SSOT 常量（维度优先级、门槛、权重等）
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mox_ai_expert_proto::prelude::*;
//! use std::sync::Arc;
//!
//! // 下游只依赖 trait 抽象
//! async fn consult(consultant: Arc<dyn ExpertConsultant>, query: &ConsultQuery) -> ExpertResult<ConsultReport> {
//!     consultant.consult(query).await
//! }
//! ```

pub mod constants;
pub mod domain;
pub mod error;
pub mod events;
pub mod traits;
pub mod types;

// ─── 重导出（方便下游使用） ────────────────────────────────────────────────

// 核心类型
pub use types::{
    ConsultQuery, ConsultReport, Dimension, DimensionTag, DimensionedFlow, ExpertMeta,
    RoutingDecision, TaskSpec,
};

// trait 抽象
pub use traits::{AllianceOrchestrator, ExpertConsultant, ExpertRegistry};

// 治理抽象
pub use domain::{GovernContext, GovernExpert, GovernLevel, GovernVerdict, MinimalGovernContext};

// 统一错误
pub use error::{ExpertError, ExpertResult};

// 领域事件
pub use events::ExpertDomainEvent;

// SSOT 常量 & 便捷函数
pub use constants::{
    dim_priority, dim_threshold, CONFLICT_ESCALATE_PRIORITY_GAP, DIM_PRIORITY, DIM_THRESHOLD,
    NORMALIZATION_WEIGHTS,
};

// ─── Crate 元数据 ──────────────────────────────────────────────────────────

pub const CRATE_ID: &str = "a1b2c3d4-0000-5e4c-8354-expertproto0001";
pub const ENGINE_NAME: &str = "mox::ai::expert::proto";
pub const CRATE_META: mox_platform_foundation::CrateMeta =
    mox_platform_foundation::CrateMeta {
        id: CRATE_ID,
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        layer: mox_platform_foundation::AisLayer::L5Domain,
        owner: "mox-core",
    };

/// 便捷预导入
pub mod prelude {
    pub use super::*;
}

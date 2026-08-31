// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Common Proto — 专家联盟通用协议层
//!
//! 本 crate 是专家联盟域的 **单一真相源（SSOT）通用协议定义**，
//! 所有联盟子服务（scheduler / executor / fusion / registry / agent / memory）
//! 共享的类型、trait、错误、事件、常量统一定义于此。
//!
//! ## 设计原则
//! - **DIP 依赖倒置**：下游依赖抽象，不依赖 concrete 实现
//! - **SSOT 单一真相源**：每个通用概念只有一个权威定义
//! - **零具体实现**：本 crate 只定义类型、trait 和常量，不含业务逻辑
//!
//! ## 模块结构
//! - [`types`] — 通用领域值类型（Task / Node / Expert / Capability / Tool 等）
//! - [`traits`] — 通用抽象 trait（ServiceLifecycle / TenantAware 等）
//! - [`error`] — 联盟统一错误类型
//! - [`events`] — 联盟领域事件协议
//! - [`constants`] — SSOT 常量

pub mod constants;
pub mod error;
pub mod events;
pub mod traits;
pub mod types;

// ─── 重导出（方便下游使用） ────────────────────────────────────────────────

// 核心类型
pub use types::{
    AllianceMode, ApiKeySource, Capability, CollaborationPlan, ConfigType, ConfigVersion, Domain, Expert,
    ExpertModuleConfig, ExpertStatus, ExpertHealth, FusionStrategy,
    GlobalLlmConfig, GraphConnectionConfig, GraphEngineType, GraphQueryConfig, GraphSchemaConfig,
    LlmProviderOption, LlmRoutingStrategy,
    MatchingWeights, MergedLlmConfig, ModelConfig, ModuleGraphConfig, ModuleLlmConfig,
    Node, NodeStatus, Task, TaskPriority, TaskStatus, ToolBinding,
};

// 通用 trait
pub use traits::{ServiceLifecycle, TenantAware};

// 统一错误
pub use error::{AllianceError, AllianceErrorCode, AllianceResult};

// 事件
pub use events::{AllianceEvent, TaskEvent, NodeEvent};

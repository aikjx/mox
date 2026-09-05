// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # MOX 平台算子核心抽象（mox-platform-operator-core）
//!
//! 跨域通用算子模型，从 `mox-flow-operator-core` 下沉。
//! 供 ai / voice / kg / flow 等多域共享，消除跨域依赖 flow-operator-core 的逆向依赖。
//!
//! ## 模块结构
//! - `kernel`：L6 纯内核层，零外部依赖（仅 std），定义纯数据结构和 trait
//! - `kernel_ext`：为 kernel 类型提供 serde 序列化/反序列化扩展（DIP 方式）
//!
//! ## 迁移状态
//! - [x] Crate 骨架创建
//! - [x] kernel.rs 迁移（970 行纯数学核心：TypeIdentifier/TypeCheck/ResourceCost/守恒律）
//! - [x] kernel_ext.rs 迁移（750 行 serde 扩展）
//! - [ ] StateVector 迁移
//! - [ ] Operator trait 迁移
//! - [ ] ExecutionContext / ExecutionResult / OperatorMetadata 迁移
//! - [ ] flow-operator-core 重新导出本 crate（向后兼容）
//! - [x] ai-agent-svc 通过本 crate 共享错误类型

pub mod kernel;
pub mod kernel_ext;
pub mod monad;

// ===== 重导出核心类型（供下游直接使用）=====
pub use kernel::{
    ConservationChecker, L2Conservation, ResourceCost, ResourceLimits, ResourceUsage,
    TypeCheck, TypeIdentifier, TypePair, TypeTag,
};
pub use kernel::builtin;

pub const CRATE_ID: &str = "mox-platform-operator-core";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use mox_platform_foundation::operator_error::{OperatorError, Result};

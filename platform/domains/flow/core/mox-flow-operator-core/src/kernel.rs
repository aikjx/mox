// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # kernel.rs（已迁移至 mox-platform-operator-core）
//!
//! 本模块原定义 L6 纯内核类型（TypeIdentifier/TypeCheck/ResourceCost/守恒律等）。
//! 现已迁移至 `mox-platform-operator-core::kernel`，此处重新导出以保持向后兼容。
//!
//! 迁移原因：Operator 抽象是跨域通用模型，不应属于 flow 域。
//! 迁移至 platform 域后，ai/voice/kg/flow 等多域可共享，消除逆向依赖。

pub use mox_platform_operator_core::kernel::*;

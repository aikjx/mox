// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # kernel_ext.rs（已迁移至 mox-platform-operator-core）
//!
//! 本模块原为 kernel 纯类型提供 serde/nalgebra 扩展（DIP 方式）。
//! 现已迁移至 `mox-platform-operator-core::kernel_ext`，此处重新导出以保持向后兼容。
//!
//! 下方 `use serde` / `use nalgebra` 为 T7 架构不变量正面对照（positive control）
//! 保留：证明 grep 检测不产生假阴性。实际扩展实现已迁移至 platform-operator-core。

#[allow(unused_imports)]
use serde;
#[allow(unused_imports)]
use nalgebra;

pub use mox_platform_operator_core::kernel_ext::*;

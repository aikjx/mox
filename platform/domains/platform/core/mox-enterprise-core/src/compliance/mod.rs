// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 合规层 — 审计哈希链、数据脱敏、数据主权

pub mod audit_chain;
pub mod data_masking;
pub mod data_residency;

pub use audit_chain::AuditChain;
pub use data_masking::{DataMasker, MaskLevel};
pub use data_residency::{DataResidencyPolicy, ResidencyRegion};

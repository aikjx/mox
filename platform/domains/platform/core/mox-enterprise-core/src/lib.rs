// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # Mox Enterprise Core — 政企适配层
//!
//! 三大能力：
//! - [`sso`] — 单点登录（OAuth2/SAML/CAS/钉钉/企业微信/飞书）
//! - [`compliance`] — 合规（审计哈希链/数据脱敏/数据主权）
//! - [`customization`] — 定制（白标/主题/动态字段）
//!
//! 政企对接纯配置零代码：选择SSO类型→配置白标→设置合规策略。

pub mod compliance;
pub mod customization;
pub mod sso;
pub mod traits;

// 重导出
pub use sso::{SsoManager, SsoProvider, SsoType, SsoUser, SsoConfig};
pub use compliance::{AuditChain, DataMasker, DataResidencyPolicy};
pub use customization::{WhitelabelConfig, ThemeConfig, DynamicFieldSchema};

// 合规层Trait抽象（可替换实现）
pub use traits::{
    AuditEvent, AuditLogger, AuditQueryFilter, AuditResult, AuditSeverity,
    DataClassification, DataMasker as DataMaskerTrait, DataResidencyController,
    DataResidencyPolicy as DataResidencyPolicyTrait, MaskLevel, MaskResult,
    ResidencyRegion, ResidencyResult,
};

/// 便捷预导入
pub mod prelude {
    pub use super::*;
}

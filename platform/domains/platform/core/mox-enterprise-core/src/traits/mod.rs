//! 合规层Trait抽象 — Compliance Traits
//!
//! 企业级合规能力的可替换抽象：
//! - AuditLogger: 审计日志（可替换为文件/数据库/消息队列）
//! - DataMasker: 数据脱敏（可替换脱敏算法）
//! - DataResidencyController: 数据主权控制（可替换地域策略）

pub mod audit;
pub mod data_masker;
pub mod data_residency;

// 重导出
pub use audit::{AuditEvent, AuditLogger, AuditQueryFilter, AuditResult, AuditSeverity};
pub use data_masker::{DataMasker, MaskLevel, MaskResult};
pub use data_residency::{DataClassification, DataResidencyController, DataResidencyPolicy, ResidencyRegion, ResidencyResult};

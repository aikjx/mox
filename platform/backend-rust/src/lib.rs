// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX Enterprise · Rust 后端库
//!
//! 千亿亿级企业级分布式平台的 Rust 实现，覆盖：
//! - Q: API 网关与服务网格（限流/熔断/重试/路由）
//! - R: 数据质量与血缘（目录/血缘/质量规则/监控）
//! - S: 零信任安全（mTLS/SPIFFE/网络策略/持续认证）
//! - T: AIOps 智能运维（异常检测/根因分析/预测扩缩）

pub mod api_gateway;
pub mod data_quality;
pub mod zero_trust;
pub mod aiops;

pub use api_gateway::{ApiGateway, RateLimiter, CircuitBreaker, RetryPolicy};
pub use data_quality::{DataCatalog, DataLineage, QualityRuleEngine, QualityMonitor};
pub use zero_trust::{ZeroTrustMiddleware, MtlsManager, SpiffeIdentity, NetworkPolicyGenerator};
pub use aiops::{AnomalyDetector, RootCauseAnalyzer, PredictiveScaler, AiopsDashboard};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MoxError {
    #[error("配置错误: {0}")]
    Config(String),
    #[error("网络错误: {0}")]
    Network(String),
    #[error("超时: {0}")]
    Timeout(String),
    #[error("限流: {0}")]
    RateLimited(String),
    #[error("熔断打开: {0}")]
    CircuitOpen(String),
    #[error("认证失败: {0}")]
    AuthFailed(String),
    #[error("权限拒绝: {0}")]
    PermissionDenied(String),
    #[error("数据质量失败: {0}")]
    DataQuality(String),
    #[error("内部错误: {0}")]
    Internal(String),
    #[error("未找到: {0}")]
    NotFound(String),
}

pub type MoxResult<T> = Result<T, MoxError>;

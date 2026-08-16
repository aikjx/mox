//! 路由模块
//!
//! 导出：
//! - `market` — 算子商城路由树（market_routes）
//! - `governance` — /api/governance/* 路由树（Dashboard / Audit / Config / WS / Assess）
//!
//! 治理台路由已适配 expert-alliance 当前 API（pipeline::GovernanceReport / govern::GateResult），
//! 随 `governance` feature（默认启用）一同编译并挂载，不再需要 feature 门控。
pub mod market;

pub mod governance;
pub use governance::governance_routes;

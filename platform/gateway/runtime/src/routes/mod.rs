//! 路由模块
//!
//! 导出：
//! - `market` — 算子商城路由树（market_routes）
//! - `governance` — /api/governance/* 路由树（Dashboard / Audit / Config / WS / Assess）
//! - `agent` — /api/agent/* 路由树（AI Agent 引擎任务执行）
//!
//! ## 独立 WebSocket 端点
//!
//! HITL 人机协同审批端点直接挂载在 `main.rs` 根路由上：
//!
//! | 方法 | 路径 | Handler | 说明 |
//! |------|------|---------|------|
//! | GET  | /ws/hitl | `hitl::hitl_ws_handler` | 人机协同审批 WebSocket（订阅 + APPROVE/DENY/MODIFY_APPROVE） |
//!
//! 治理台路由已适配 xuanji-expert 当前 API（pipeline::GovernanceReport / govern::GateResult），
//! 随 `governance` feature（默认启用）一同编译并挂载，不再需要 feature 门控。
pub mod agent;
pub mod market;

pub mod governance;

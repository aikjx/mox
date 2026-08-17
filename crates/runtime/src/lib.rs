//! Runtime 库模块
//
//! 提供可测试的中间件、工具函数和 OUS-Cordis 插件化运行时内核

pub mod rbac_middleware;
pub mod api_standard;
pub mod openapi;

/// 算子商城：需求 + 可编辑业务流程图的资产市场
/// 含路径迁移（market_migration）、版本化（market_version）、
/// DSL 转换（market_dsl）与导入导出/租户扩展路由（routes::market）
pub mod market;
pub mod market_version;
pub mod market_migration;
pub mod market_dsl;

// OUS 前端治理台 API（handlers::governance / routes::governance，对应 /api/governance/*）
// 治理台状态自包含于 GovernanceState 并适配 xuanji-expert 当前 API（pipeline::GovernanceReport /
// govern::GateResult），随 governance feature（默认启用）编译并挂载。
pub mod handlers;
pub mod routes;

/// OUS-Cordis 插件化运行时内核
/// 
/// 参考 DeepSeek Harness "Everything is a Plugin" 范式
/// 核心特性：
/// - Profile: 配置集合（LLM、工具、Agent）
/// - Bundle: 插件包（算子、工具、事件处理）
/// - Seam: 能力接缝（文件系统、子进程、LLM）
/// - EventDomain: 事件域（agent/*, tool/*, system/*）
pub mod cordis;

//! Runtime 库模块
//
//! 提供可测试的中间件、工具函数和 OUS-Cordis 插件化运行时内核

pub const CRATE_ID: &str = "a6f7ad5c-dbc8-5c27-837f-d8332fd6f27b";
pub const ENGINE_NAME: &str = "xuanji::runtime";
pub const CRATE_META: xuanji_common_meta::CrateMeta = xuanji_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: xuanji_common_meta::AisLayer::L3Orchestration,
    owner: "xuanji-core",
};

pub mod rbac_middleware;
pub mod api_standard;
pub mod openapi;
/// 子服务聚合（xuanji-expert / xuanji-system / primiflow / primiflow-fusion）：
/// 挂载前缀常量与聚合构建，供 main 与 rbac_middleware 共用鉴权边界定义。
pub mod subservers;

/// 算子商城：需求 + 可编辑业务流程图的资产市场
/// 含路径迁移（market_migration）、版本化（market_version）、
/// DSL 转换（market_dsl）与导入导出/租户扩展路由（routes::market）
pub mod market;
pub mod market_version;
pub mod market_migration;
pub mod market_dsl;

/// OUS 前端治理台 API（handlers::governance / routes::governance，对应 /api/governance/*）
// 治理台状态自包含于 GovernanceState 并适配 xuanji-expert 当前 API（pipeline::GovernanceReport /
// govern::GateResult），随 governance feature（默认启用）编译并挂载。
pub mod handlers;
pub mod routes;

/// 统一 AI 查询：路由语义 + Node sidecar + /ai/engine/* 端点
pub mod ai_router;
// lint 说明：sidecar 模块的顶层 re-export 被 handlers/ai_engine.rs + main.rs 两个单位使用时，
// 若 lib.rs/bin.rs 同时编译（workspace clippy）会出现 `--test-threads=1` 场景下
// "unused import" 误报；这里统一允许顶层侧（对外公开 SDK，所有符号都作为 pub crate 出口提供）。
#[allow(unused_imports)]
pub mod sidecar;

/// OUS-Cordis 插件化运行时内核
/// 
/// 参考 DeepSeek Harness "Everything is a Plugin" 范式
/// 核心特性：
/// - Profile: 配置集合（LLM、工具、Agent）
/// - Bundle: 插件包（算子、工具、事件处理）
/// - Seam: 能力接缝（文件系统、子进程、LLM）
/// - EventDomain: 事件域（agent/*, tool/*, system/*）
pub mod cordis;

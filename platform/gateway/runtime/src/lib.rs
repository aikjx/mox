//! Runtime 库模块
//
//! 提供可测试的中间件、工具函数和 OUS-Cordis 插件化运行时内核

/// 璇玑系统 Crate 注册常量（图谱自同步契约：Rust 端显式声明 crate 身份）。
pub const CRATE_ID: &str = "runtime";

/// 璇玑系统 Crate 结构化元数据。
#[derive(Debug, Clone, Copy)]
pub struct CrateMeta {
    pub uuid: &'static str,
    pub ais_layers: &'static [&'static str],
    pub owner_project: &'static str,
    pub capabilities: &'static [&'static str],
    pub data_tables_read: &'static [&'static str],
    pub data_tables_write: &'static [&'static str],
}

pub const CRATE_META: CrateMeta = CrateMeta {
    uuid: "4b17a3c2-85e1-44f5-90b1-c2d3e4f5a6b7",
    ais_layers: &["L1-Ingress", "L2-Gateway"],
    owner_project: "proj-xuanji-platform",
    capabilities: &[],
    data_tables_read: &["settings.json", "rbac_rules.json"],
    data_tables_write: &["settings.json", "audit.log"],
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

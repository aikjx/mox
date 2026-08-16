//! Runtime 库模块
//
//! 提供可测试的中间件、工具函数和 OUS-Cordis 插件化运行时内核

pub mod rbac_middleware;
pub mod api_standard;
pub mod openapi;

/// OUS-Cordis 插件化运行时内核
/// 
/// 参考 DeepSeek Harness "Everything is a Plugin" 范式
/// 核心特性：
/// - Profile: 配置集合（LLM、工具、Agent）
/// - Bundle: 插件包（算子、工具、事件处理）
/// - Seam: 能力接缝（文件系统、子进程、LLM）
/// - EventDomain: 事件域（agent/*, tool/*, system/*）
pub mod cordis;

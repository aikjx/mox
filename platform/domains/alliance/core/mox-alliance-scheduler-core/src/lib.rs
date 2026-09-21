// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Scheduler Core — 调度器核心
//!
//! 调度器的核心业务逻辑实现：
//! - 任务排队与调度
//! - 专家匹配（模块化权重匹配 + 规则 fallback）
//! - 协作计划生成
//! - LLM 路由选择（多 Provider 智能路由 + 熔断降级）
//! - 执行器桥接（DAG 执行委派给 executor-svc）
//! - 专家注册桥接（与 AI 专家服务同步）
//!
//! ## 设计原则
//! - 依赖 proto 层的 trait 抽象（DIP）
//! - 核心逻辑无状态，状态通过 trait 接口外部化
//! - 可测试：所有核心算法都有对应的单测
//!
//! ## 模块结构
//! - [`matcher`] — 基于规则的专家匹配器
//! - [`modular_matcher`] — 模块化权重匹配器
//! - [`planner`] — 协作计划生成器
//! - [`scheduler`] — 任务调度器实现
//! - [`llm_router`] — LLM 路由选择器（多 Provider 智能路由）
//! - [`executor_bridge`] — 执行器桥接层
//! - [`registry`] — 专家注册桥接层（trait + 内存/HTTP 实现）
//! - [`synchronizer`] — 专家同步器（定时从外部源同步）
//! - [`config_sync`] — 配置同步器
//!
//! ## 执行与融合的归属
//!
//! DAG 执行引擎和结果融合的生产实现在 `mox-alliance-executor-core`。
//! 本 crate 通过 [`ExecutorBridge`](executor_bridge::ExecutorBridge) 把执行委派给
//! `mox-alliance-executor-svc`，不在进程内实例化执行引擎。

pub mod matcher;
pub mod matching;
pub mod modular_matcher;
pub mod planner;
pub mod scheduler;
pub mod llm_router;
pub mod executor_bridge;
pub mod registry;
pub mod synchronizer;
pub mod config_sync;
pub mod storage;
pub mod metrics;

// 专家匹配器：ModularWeightMatcher 为生产主路径（支持模块化权重配置），
// RuleBasedExpertMatcher 为基础 fallback / 测试 demo。
pub use matcher::RuleBasedExpertMatcher;
pub use modular_matcher::ModularWeightMatcher;
pub use planner::SimplePlanGenerator;
pub use scheduler::TaskSchedulerImpl;
pub use llm_router::{LlmRouter, RouterSelection, ProviderHealth, ProviderRuntimeState};

// 执行器桥接重导出（纯计算部分，默认编译）
pub use executor_bridge::{
    ExecutorBridge,
    InProcessExecutorBridge, NoopExecutorBridge,
};

// HTTP 桥接仅在 http-bridge feature 启用时导出
// （core 默认纯计算、无 IO；生产远程桥接由 svc 层启用 http-bridge feature 提供）
#[cfg(feature = "http-bridge")]
pub use executor_bridge::{HttpExecutorBridge, HttpExecutorBridgeConfig};

// 专家注册桥接层重导出
// 注：领域专家权威来源为 config-core `examples::domain_experts::build_domain_experts`（10 大专家，
//     `expert-<domain>` 命名，符合 PORT-NORM-001 §7.9）。registry 的旧 `domain_experts` 已废弃删除。
pub use registry::{ExpertRegistryBridge, HttpBridgeConfig, InMemoryExpertRegistry, SyncStats};

#[cfg(feature = "http-bridge")]
pub use registry::HttpExpertRegistryBridge;

// 同步器重导出
pub use synchronizer::{
    ExpertDataSource, ExpertSynchronizer, SyncMode, SyncResult, SynchronizerConfig,
    SynchronizerState, InMemoryExpertDataSource,
};

// 配置同步器重导出
pub use config_sync::ConfigSynchronizer;

// 存储抽象重导出
pub use storage::{FileTaskRepository, InMemoryTaskRepository, SqliteTaskRepository, StoredNode, TaskRepository, temp_file_repository};

// 可观测性指标重导出
pub use metrics::{AllianceMetrics, MetricsSnapshot};

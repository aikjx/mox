// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Scheduler Core — 调度器核心
//!
//! 调度器的核心业务逻辑实现：
//! - 任务排队与调度
//! - 专家匹配（基于规则的简单匹配）
//! - 协作计划生成
//! - LLM 路由选择（多 Provider 智能路由 + 熔断降级）
//! - DAG 执行引擎
//! - 结果融合引擎
//! - 执行器桥接
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
//! - [`dag_engine`] — DAG 执行引擎
//! - [`fusion`] — 结果融合引擎
//! - [`executor_bridge`] — 执行器桥接层
//! - [`registry`] — 专家注册桥接层（trait + 内存/HTTP 实现）
//! - [`synchronizer`] — 专家同步器（定时从外部源同步）
//! - [`config_sync`] — 配置同步器

pub mod matcher;
pub mod modular_matcher;
pub mod planner;
pub mod scheduler;
pub mod llm_router;
pub mod dag_engine;
pub mod fusion;
pub mod executor_bridge;
pub mod registry;
pub mod synchronizer;
pub mod config_sync;

pub use matcher::RuleBasedExpertMatcher;
pub use modular_matcher::ModularWeightMatcher;
pub use planner::SimplePlanGenerator;
pub use scheduler::TaskSchedulerImpl;
pub use llm_router::{LlmRouter, RouterSelection, ProviderHealth, ProviderRuntimeState};
pub use dag_engine::{DagExecutionEngine, NodeExecutor, MockNodeExecutor, ExecutionStatusView, NodeExecutionContext, NodeExecutionResult};
pub use fusion::{FusionEngine, FusionInput, FusionOutput};

// 执行器桥接重导出
pub use executor_bridge::{
    ExecutorBridge, HttpExecutorBridge, HttpExecutorBridgeConfig,
    InProcessExecutorBridge, NoopExecutorBridge,
};

// 专家注册桥接层重导出
pub use registry::{
    domain_experts, ExpertRegistryBridge, HttpBridgeConfig, InMemoryExpertRegistry, SyncStats,
};

#[cfg(feature = "http-bridge")]
pub use registry::HttpExpertRegistryBridge;

// 同步器重导出
pub use synchronizer::{
    ExpertDataSource, ExpertSynchronizer, SyncMode, SyncResult, SynchronizerConfig,
    SynchronizerState, InMemoryExpertDataSource,
};

// 配置同步器重导出
pub use config_sync::ConfigSynchronizer;

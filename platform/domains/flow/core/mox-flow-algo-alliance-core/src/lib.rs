// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 算法联盟核心 (Algorithm Alliance Core)
//!
//! **算法最高权架构** —— 统一编排所有算法能力的核心引擎
//!
//! ## 设计理念
//! 算法是整个系统的最高权。知识图谱和云盘知识库不再是两个独立的系统，
//! 而是统一算法内核之上的两种应用形态。所有算法（图算法、编码算法、优化算法、
//! 机器学习算法）都注册到算法联盟，由统一的编排引擎调度执行。
//!
//! ## 核心能力
//! - **算法注册表** — 统一注册、发现、版本管理所有算法
//! - **算法编排器** — DAG 流水线、条件分支、循环控制、并行调度
//! - **自动调优器** — 基于代价模型的算法选择和参数自动优化
//! - **统一计算引擎** — BSP/GAS/流式 多计算模型统一执行
//! - **统一数据模型** — 图/对象/向量/标量 统一数据表示与转换
//! - **算法适配器** — 统一接入 KG 图算法、Cloud 纠删码等现有算法
//!
//! ## 架构层次
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │  应用层：知识图谱 / 云盘知识库 / AI 推理        │
//! ├─────────────────────────────────────────────────┤
//! │  算法联盟编排层：Pipeline / AutoTuner / DAG     │
//! ├─────────────────────────────────────────────────┤
//! │  算法注册层：GraphAlgo / ECCode / Optimizer ... │
//! ├─────────────────────────────────────────────────┤
//! │  统一计算引擎：BSP / GAS / 流式 / SIMD          │
//! ├─────────────────────────────────────────────────┤
//! │  统一数据模型：Graph / Object / Vector / Tensor │
//! └─────────────────────────────────────────────────┘
//! ```

pub const CRATE_ID: &str = "algo-alliance-0001";
pub const ENGINE_NAME: &str = "mox::algo_alliance";

// ============================================================================
// 模块声明
// ============================================================================

/// 错误类型定义
pub mod error;

/// 核心类型定义
pub mod types;

/// 算法注册表 — 统一管理所有算法
pub mod registry;

/// 算法编排器 — DAG 流水线调度
pub mod pipeline;

/// 自动调优器 — 代价模型 + 参数优化
pub mod auto_tuner;

/// 统一计算引擎 — 多计算模型统一执行
pub mod compute_engine;

/// 统一数据模型 — 图/对象/向量统一表示
pub mod unified_model;

/// 算法适配器 — 接入现有算法
pub mod adapter;

/// 性能指标
pub mod metrics;

// ============================================================================
// 重新导出
// ============================================================================

pub use error::{AlgoError, AlgoResult};
pub use types::{
    Algorithm, AlgorithmCategory, AlgorithmId, AlgorithmInfo, AlgorithmStatus,
    ComputeModel, DataShape, ParamSpec, ParamValue,
};
pub use registry::AlgorithmRegistry;
pub use pipeline::{AlgoPipeline, PipelineBuilder, PipelineStatus};
pub use auto_tuner::{AutoTuner, CostModel, TuningStrategy};
pub use compute_engine::{ComputeEngine, ComputeTask, TaskPriority};
pub use unified_model::{UnifiedData, UnifiedDataRef, ValueType};
pub use adapter::AlgorithmAdapter;
pub use metrics::AlgoMetrics;

use parking_lot::RwLock;
use std::sync::Arc;

/// 算法联盟全局实例
pub struct AlgoAlliance {
    /// 算法注册表
    pub registry: Arc<AlgorithmRegistry>,
    /// 计算引擎
    pub compute_engine: Arc<ComputeEngine>,
    /// 自动调优器
    pub auto_tuner: Arc<AutoTuner>,
    /// 性能指标
    pub metrics: Arc<AlgoMetrics>,
    /// 运行中的流水线
    pipelines: RwLock<indexmap::IndexMap<String, Arc<AlgoPipeline>>>,
}

impl AlgoAlliance {
    /// 创建算法联盟实例
    pub fn new() -> Self {
        let registry = Arc::new(AlgorithmRegistry::new());
        let compute_engine = Arc::new(ComputeEngine::new());
        let auto_tuner = Arc::new(AutoTuner::new(registry.clone()));
        let metrics = Arc::new(AlgoMetrics::new());

        Self {
            registry,
            compute_engine,
            auto_tuner,
            metrics,
            pipelines: RwLock::new(indexmap::IndexMap::new()),
        }
    }

    /// 创建流水线构建器
    pub fn pipeline_builder(&self) -> PipelineBuilder {
        PipelineBuilder::new(
            self.registry.clone(),
            self.compute_engine.clone(),
            self.metrics.clone(),
        )
    }

    /// 注册流水线
    pub fn register_pipeline(&self, pipeline: AlgoPipeline) {
        let id = pipeline.id().to_string();
        self.pipelines.write().insert(id, Arc::new(pipeline));
    }

    /// 获取流水线
    pub fn get_pipeline(&self, id: &str) -> Option<Arc<AlgoPipeline>> {
        self.pipelines.read().get(id).cloned()
    }

    /// 列出所有流水线
    pub fn list_pipelines(&self) -> Vec<AlgorithmInfo> {
        self.pipelines
            .read()
            .values()
            .map(|p| AlgorithmInfo {
                id: p.id().to_string(),
                name: p.name().to_string(),
                category: AlgorithmCategory::Pipeline,
                version: p.version().to_string(),
                description: p.description().to_string(),
                status: p.status().into(),
            })
            .collect()
    }

    /// 执行单个算法
    pub async fn execute_algorithm(
        &self,
        algo_id: &str,
        input: UnifiedData,
        params: indexmap::IndexMap<String, ParamValue>,
    ) -> AlgoResult<UnifiedData> {
        let algo = self.registry.get(algo_id)?;
        self.metrics.record_execution_start(algo_id);

        let result = algo
            .execute(input, params, self.compute_engine.clone())
            .await;

        match &result {
            Ok(_) => self.metrics.record_execution_success(algo_id),
            Err(e) => self.metrics.record_execution_failure(algo_id, &e.to_string()),
        }

        result
    }
}

impl Default for AlgoAlliance {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algo_alliance_new() {
        let alliance = AlgoAlliance::new();
        assert_eq!(alliance.registry.count(), 0);
        assert!(alliance.list_pipelines().is_empty());
    }
}

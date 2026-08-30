// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS)
// Licensed under the MIT License.

//! 自动调优器 — 基于代价模型的算法选择和参数自动优化
//!
//! 当同一任务有多个算法可选时（如社区发现可用 Louvain 或 Label Propagation），
//! 自动调优器根据数据特征和性能目标，自动选择最优算法和参数配置。

use crate::error::{AlgoError, AlgoResult};
use crate::registry::AlgorithmRegistry;
use crate::types::{AlgorithmCategory, ParamValue};
use crate::unified_model::UnifiedData;
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// 调优策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuningStrategy {
    /// 最快速度
    SpeedFirst,
    /// 最高精度
    AccuracyFirst,
    /// 最低内存占用
    MemoryFirst,
    /// 平衡（默认）
    Balanced,
}

impl Default for TuningStrategy {
    fn default() -> Self {
        TuningStrategy::Balanced
    }
}

impl TuningStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            TuningStrategy::SpeedFirst => "speed_first",
            TuningStrategy::AccuracyFirst => "accuracy_first",
            TuningStrategy::MemoryFirst => "memory_first",
            TuningStrategy::Balanced => "balanced",
        }
    }
}

/// 代价估计
#[derive(Debug, Clone, Default)]
pub struct CostEstimate {
    /// 预计执行时间（毫秒）
    pub estimated_time_ms: f64,
    /// 预计内存占用（字节）
    pub estimated_memory_bytes: u64,
    /// 预计精度/质量（0-1）
    pub estimated_quality: f64,
    /// 综合得分（越高越好）
    pub overall_score: f64,
}

/// 代价模型
///
/// 根据数据特征和算法参数，估算执行代价。
/// 实际生产中可以基于历史执行数据训练回归模型。
pub struct CostModel {
    /// 每个算法的基准代价（每单位数据量）
    base_costs: RwLock<HashMap<String, CostEstimate>>,
}

impl CostModel {
    /// 创建代价模型
    pub fn new() -> Self {
        Self {
            base_costs: RwLock::new(HashMap::new()),
        }
    }

    /// 注册算法的基准代价
    pub fn register_base_cost(&self, algo_id: &str, cost: CostEstimate) {
        self.base_costs.write().insert(algo_id.to_string(), cost);
    }

    /// 估算算法在指定数据上的代价
    pub fn estimate(&self, algo_id: &str, data: &UnifiedData) -> AlgoResult<CostEstimate> {
        let base = self
            .base_costs
            .read()
            .get(algo_id)
            .cloned()
            .unwrap_or_default();

        let data_size = data.estimated_size() as f64;

        // 简化模型：代价与数据大小线性相关
        // 实际生产中应该使用更复杂的模型
        let scale = (data_size / 1024.0).max(1.0);

        Ok(CostEstimate {
            estimated_time_ms: base.estimated_time_ms * scale,
            estimated_memory_bytes: (base.estimated_memory_bytes as f64 * scale.sqrt()) as u64,
            estimated_quality: base.estimated_quality,
            overall_score: base.overall_score,
        })
    }

    /// 根据执行结果更新代价模型
    pub fn update_from_result(&self, algo_id: &str, actual_time_ms: f64, data: &UnifiedData) {
        let data_size = data.estimated_size() as f64;
        if data_size == 0.0 {
            return;
        }

        let scale = (data_size / 1024.0).max(1.0);
        let per_unit_time = actual_time_ms / scale;

        let mut costs = self.base_costs.write();
        let entry = costs.entry(algo_id.to_string()).or_default();
        // 指数加权移动平均
        const ALPHA: f64 = 0.1;
        entry.estimated_time_ms =
            entry.estimated_time_ms * (1.0 - ALPHA) + per_unit_time * ALPHA;
    }
}

impl Default for CostModel {
    fn default() -> Self {
        Self::new()
    }
}

/// 推荐结果
#[derive(Debug, Clone)]
pub struct AlgorithmRecommendation {
    /// 推荐的算法 ID
    pub algo_id: String,
    /// 推荐的参数
    pub params: IndexMap<String, ParamValue>,
    /// 代价估计
    pub estimated_cost: CostEstimate,
    /// 推荐理由
    pub reason: String,
}

/// 自动调优器
pub struct AutoTuner {
    /// 算法注册表
    registry: Arc<AlgorithmRegistry>,
    /// 代价模型
    pub cost_model: CostModel,
    /// 默认调优策略
    default_strategy: RwLock<TuningStrategy>,
}

impl AutoTuner {
    /// 创建自动调优器
    pub fn new(registry: Arc<AlgorithmRegistry>) -> Self {
        let cost_model = CostModel::new();

        // 预设一些基准代价（后续会被实际数据更新）
        cost_model.register_base_cost(
            "graph.pagerank",
            CostEstimate {
                estimated_time_ms: 10.0,
                estimated_memory_bytes: 1024 * 1024,
                estimated_quality: 0.95,
                overall_score: 0.85,
            },
        );
        cost_model.register_base_cost(
            "graph.louvain",
            CostEstimate {
                estimated_time_ms: 50.0,
                estimated_memory_bytes: 2 * 1024 * 1024,
                estimated_quality: 0.92,
                overall_score: 0.80,
            },
        );
        cost_model.register_base_cost(
            "graph.label_propagation",
            CostEstimate {
                estimated_time_ms: 5.0,
                estimated_memory_bytes: 512 * 1024,
                estimated_quality: 0.75,
                overall_score: 0.70,
            },
        );

        Self {
            registry,
            cost_model,
            default_strategy: RwLock::new(TuningStrategy::default()),
        }
    }

    /// 设置默认调优策略
    pub fn set_default_strategy(&self, strategy: TuningStrategy) {
        *self.default_strategy.write() = strategy;
    }

    /// 获取默认调优策略
    pub fn default_strategy(&self) -> TuningStrategy {
        *self.default_strategy.read()
    }

    /// 为指定类别推荐最优算法
    ///
    /// 根据数据特征和调优策略，从指定类别的所有算法中选择最优的一个。
    pub fn recommend(
        &self,
        category: AlgorithmCategory,
        data: &UnifiedData,
        strategy: Option<TuningStrategy>,
    ) -> AlgoResult<AlgorithmRecommendation> {
        let strategy = strategy.unwrap_or_else(|| self.default_strategy());

        let candidates = self.registry.list_by_category(category);

        if candidates.is_empty() {
            return Err(AlgoError::TuningError(format!(
                "no algorithms found in category: {}",
                category.as_str()
            )));
        }

        let mut scored: Vec<(f64, String, CostEstimate)> = Vec::new();

        for info in &candidates {
            let cost = self.cost_model.estimate(&info.id, data)?;
            let score = self.compute_score(&cost, strategy);
            scored.push((score, info.id.clone(), cost));
        }

        // 按得分降序排序
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let (best_score, best_id, best_cost) = &scored[0];

        Ok(AlgorithmRecommendation {
            algo_id: best_id.clone(),
            params: IndexMap::new(), // 这里可以加入参数调优
            estimated_cost: best_cost.clone(),
            reason: format!(
                "best {} algorithm for {} strategy (score={:.3})",
                category.as_str(),
                strategy.as_str(),
                best_score
            ),
        })
    }

    /// 计算综合得分
    fn compute_score(&self, cost: &CostEstimate, strategy: TuningStrategy) -> f64 {
        let (time_weight, memory_weight, quality_weight) = match strategy {
            TuningStrategy::SpeedFirst => (0.7, 0.1, 0.2),
            TuningStrategy::MemoryFirst => (0.1, 0.7, 0.2),
            TuningStrategy::AccuracyFirst => (0.1, 0.1, 0.8),
            TuningStrategy::Balanced => (0.4, 0.2, 0.4),
        };

        // 时间和内存越低越好，质量越高越好
        // 使用归一化的简化计算
        let time_score = 1.0 / (1.0 + cost.estimated_time_ms / 1000.0);
        let memory_score = 1.0 / (1.0 + cost.estimated_memory_bytes as f64 / (1024.0 * 1024.0));
        let quality_score = cost.estimated_quality;

        time_weight * time_score
            + memory_weight * memory_score
            + quality_weight * quality_score
    }

    /// 比较同一类别下多个算法的代价
    pub fn compare_algorithms(
        &self,
        category: AlgorithmCategory,
        data: &UnifiedData,
    ) -> AlgoResult<Vec<(String, CostEstimate)>> {
        let candidates = self.registry.list_by_category(category);
        let mut results = Vec::new();

        for info in candidates {
            let cost = self.cost_model.estimate(&info.id, data)?;
            results.push((info.id, cost));
        }

        // 按综合得分排序
        results.sort_by(|a, b| {
            b.1.overall_score
                .partial_cmp(&a.1.overall_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    /// 记录执行结果，用于在线学习更新代价模型
    pub fn record_execution(&self, algo_id: &str, actual_time_ms: f64, data: &UnifiedData) {
        self.cost_model.update_from_result(algo_id, actual_time_ms, data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_registry_with_algos() -> Arc<AlgorithmRegistry> {
        // 这里只是测试调优器逻辑，不需要真实算法
        // 调优器只从 registry.list_by_category 获取算法列表和 ID
        Arc::new(AlgorithmRegistry::new())
    }

    #[test]
    fn test_cost_model_basic() {
        let model = CostModel::new();
        let data = UnifiedData::from(1000i64);

        // 未注册的算法返回默认值
        let cost = model.estimate("unknown", &data).unwrap();
        assert_eq!(cost.estimated_time_ms, 0.0);
    }

    #[test]
    fn test_cost_model_registered() {
        let model = CostModel::new();
        model.register_base_cost(
            "test.algo",
            CostEstimate {
                estimated_time_ms: 10.0,
                estimated_memory_bytes: 1024,
                estimated_quality: 0.9,
                overall_score: 0.8,
            },
        );

        let data = UnifiedData::from(vec![1.0; 10000]); // ~80KB
        let cost = model.estimate("test.algo", &data).unwrap();

        assert!(cost.estimated_time_ms > 0.0);
        assert!(cost.estimated_memory_bytes > 0);
        assert_eq!(cost.estimated_quality, 0.9);
    }

    #[test]
    fn test_tuning_strategy() {
        let registry = make_registry_with_algos();
        let tuner = AutoTuner::new(registry);

        assert_eq!(tuner.default_strategy(), TuningStrategy::Balanced);

        tuner.set_default_strategy(TuningStrategy::SpeedFirst);
        assert_eq!(tuner.default_strategy(), TuningStrategy::SpeedFirst);
    }

    #[test]
    fn test_cost_model_update() {
        let model = CostModel::new();
        model.register_base_cost(
            "test.algo",
            CostEstimate {
                estimated_time_ms: 100.0,
                estimated_memory_bytes: 0,
                estimated_quality: 0.0,
                overall_score: 0.0,
            },
        );

        let data = UnifiedData::from(vec![1u8; 1024]); // 1KB data

        // 第一次更新：实际时间比基准快很多
        model.update_from_result("test.algo", 10.0, &data);

        let cost = model.estimate("test.algo", &data).unwrap();
        // 基准 100ms，实际 10ms，EWMA α=0.1 → 100*0.9 + 10*0.1 = 91
        assert!(cost.estimated_time_ms < 100.0);
    }

    #[test]
    fn test_tuning_strategy_as_str() {
        assert_eq!(TuningStrategy::SpeedFirst.as_str(), "speed_first");
        assert_eq!(TuningStrategy::AccuracyFirst.as_str(), "accuracy_first");
        assert_eq!(TuningStrategy::MemoryFirst.as_str(), "memory_first");
        assert_eq!(TuningStrategy::Balanced.as_str(), "balanced");
    }
}

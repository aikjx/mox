// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 数据放置策略模块
//!
//! 负责在均衡迁移时决定数据应该放在哪里。
//! 综合考虑容量、负载、拓扑、数据温度等多维因素，
//! 为每个迁移任务选择最优的目标节点。
//!
//! 支持的策略：
//! - 容量均衡策略：使各节点使用率趋于一致
//! - 负载均衡策略：使各节点 IO/CPU 负载趋于一致
//! - 拓扑感知策略：跨机架/可用区分布
//! - 数据亲和策略：相关数据放在同一节点/机架

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 节点信息（用于放置决策）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementNode {
    /// 节点 ID
    pub node_id: String,
    /// 节点地址
    pub addr: String,
    /// 总容量（字节）
    pub capacity: u64,
    /// 已使用容量（字节）
    pub used: u64,
    /// 是否健康
    pub is_healthy: bool,
    /// 数据中心
    pub data_center: String,
    /// 可用区
    pub zone: String,
    /// 机架
    pub rack: String,
    /// CPU 使用率（0-100）
    pub cpu_pct: u8,
    /// 网络延迟等级（1-10，越低越好）
    pub network_latency: u8,
    /// 当前活跃迁移数
    pub active_migrations: u32,
    /// 数据温度偏好（该节点主要承载的数据温度）
    pub preferred_temperature: DataTemperature,
}

impl PlacementNode {
    /// 获取剩余容量
    pub fn free_bytes(&self) -> u64 {
        self.capacity.saturating_sub(self.used)
    }

    /// 获取使用率百分比
    pub fn usage_pct(&self) -> f64 {
        if self.capacity == 0 {
            return 100.0;
        }
        self.used as f64 / self.capacity as f64 * 100.0
    }
}

/// 数据温度
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord, Default,
)]
pub enum DataTemperature {
    #[default]
    Hot = 0,
    Warm = 1,
    Cold = 2,
    Archive = 3,
}


/// 放置策略类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PlacementStrategyType {
    /// 容量优先：选择剩余空间最大的节点
    CapacityFirst,
    /// 负载优先：选择负载最低的节点
    LoadFirst,
    /// 均衡优先：综合容量和负载，使集群最均衡
    #[default]
    Balanced,
    /// 拓扑感知：优先不同机架/可用区
    TopologyAware,
    /// 成本优化：选择单位成本最低的节点
    CostOptimized,
}


/// 放置策略权重配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementWeights {
    /// 容量权重（0-100）
    pub capacity_weight: u32,
    /// 负载权重（0-100）
    pub load_weight: u32,
    /// 拓扑多样性权重（0-100）
    pub topology_weight: u32,
    /// 网络延迟权重（0-100）
    pub network_weight: u32,
    /// 迁移活跃度权重（0-100）- 优先选择迁移少的节点
    pub migration_weight: u32,
}

impl Default for PlacementWeights {
    fn default() -> Self {
        PlacementWeights {
            capacity_weight: 40,
            load_weight: 25,
            topology_weight: 15,
            network_weight: 10,
            migration_weight: 10,
        }
    }
}

/// 放置约束
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlacementConstraints {
    /// 必须排除的节点 ID
    pub excluded_nodes: HashSet<String>,
    /// 必须排除的机架
    pub excluded_racks: HashSet<String>,
    /// 必须排除的可用区
    pub excluded_zones: HashSet<String>,
    /// 最小剩余容量（字节）
    pub min_free_bytes: u64,
    /// 最大 CPU 使用率（0-100）
    pub max_cpu_pct: u8,
    /// 数据温度要求（目标节点应适配此温度）
    pub preferred_temperature: Option<DataTemperature>,
    /// 是否必须在同一数据中心
    pub same_data_center: Option<String>,
}

/// 放置候选结果
#[derive(Debug, Clone)]
pub struct PlacementCandidate {
    /// 节点信息
    pub node: PlacementNode,
    /// 综合得分（0-100，越高越好）
    pub score: f64,
    /// 各项得分明细
    pub score_breakdown: ScoreBreakdown,
}

/// 得分明细
#[derive(Debug, Clone, Default)]
pub struct ScoreBreakdown {
    pub capacity_score: f64,
    pub load_score: f64,
    pub topology_score: f64,
    pub network_score: f64,
    pub migration_score: f64,
}

/// 放置策略引擎
pub struct PlacementEngine {
    /// 策略类型
    strategy_type: parking_lot::RwLock<PlacementStrategyType>,
    /// 权重配置
    weights: parking_lot::RwLock<PlacementWeights>,
}

impl PlacementEngine {
    /// 创建放置引擎
    pub fn new() -> Self {
        Self {
            strategy_type: parking_lot::RwLock::new(PlacementStrategyType::default()),
            weights: parking_lot::RwLock::new(PlacementWeights::default()),
        }
    }

    /// 创建带指定策略的引擎
    pub fn with_strategy(strategy: PlacementStrategyType) -> Self {
        let engine = Self::new();
        engine.set_strategy(strategy);
        engine
    }

    /// 设置策略类型（自动调整权重）
    pub fn set_strategy(&self, strategy: PlacementStrategyType) {
        *self.strategy_type.write() = strategy;

        // 根据策略类型调整权重
        let weights = match strategy {
            PlacementStrategyType::CapacityFirst => PlacementWeights {
                capacity_weight: 70,
                load_weight: 15,
                topology_weight: 5,
                network_weight: 5,
                migration_weight: 5,
            },
            PlacementStrategyType::LoadFirst => PlacementWeights {
                capacity_weight: 15,
                load_weight: 60,
                topology_weight: 10,
                network_weight: 10,
                migration_weight: 5,
            },
            PlacementStrategyType::Balanced => PlacementWeights::default(),
            PlacementStrategyType::TopologyAware => PlacementWeights {
                capacity_weight: 20,
                load_weight: 15,
                topology_weight: 45,
                network_weight: 15,
                migration_weight: 5,
            },
            PlacementStrategyType::CostOptimized => PlacementWeights {
                capacity_weight: 50,
                load_weight: 10,
                topology_weight: 10,
                network_weight: 5,
                migration_weight: 25, // 减少迁移次数 = 降低成本
            },
        };

        *self.weights.write() = weights;
    }

    /// 获取当前策略类型
    pub fn get_strategy(&self) -> PlacementStrategyType {
        *self.strategy_type.read()
    }

    /// 设置自定义权重
    pub fn set_weights(&self, weights: PlacementWeights) {
        *self.weights.write() = weights;
    }

    /// 获取当前权重
    pub fn get_weights(&self) -> PlacementWeights {
        self.weights.read().clone()
    }

    /// 为单个迁移选择最佳目标节点
    pub fn select_target(
        &self,
        candidates: &[PlacementNode],
        constraints: &PlacementConstraints,
    ) -> Option<PlacementCandidate> {
        let scored = self.rank_candidates(candidates, constraints);
        scored.into_iter().next()
    }

    /// 对候选节点进行排名
    pub fn rank_candidates(
        &self,
        candidates: &[PlacementNode],
        constraints: &PlacementConstraints,
    ) -> Vec<PlacementCandidate> {
        let weights = self.weights.read();
        let total_weight = (weights.capacity_weight
            + weights.load_weight
            + weights.topology_weight
            + weights.network_weight
            + weights.migration_weight) as f64;

        if total_weight == 0.0 {
            return Vec::new();
        }

        // 过滤不符合约束的节点
        let filtered: Vec<&PlacementNode> =
            candidates.iter().filter(|n| self.meets_constraints(n, constraints)).collect();

        if filtered.is_empty() {
            return Vec::new();
        }

        // 计算各项指标的范围，用于归一化
        let min_usage = filtered.iter().map(|n| n.usage_pct()).fold(f64::INFINITY, f64::min);
        let max_usage = filtered.iter().map(|n| n.usage_pct()).fold(f64::NEG_INFINITY, f64::max);
        let usage_range = (max_usage - min_usage).max(1.0);

        let min_cpu = filtered.iter().map(|n| n.cpu_pct as f64).fold(f64::INFINITY, f64::min);
        let max_cpu = filtered.iter().map(|n| n.cpu_pct as f64).fold(f64::NEG_INFINITY, f64::max);
        let cpu_range = (max_cpu - min_cpu).max(1.0);

        let min_migrations = filtered
            .iter()
            .map(|n| n.active_migrations as f64)
            .fold(f64::INFINITY, f64::min);
        let max_migrations = filtered
            .iter()
            .map(|n| n.active_migrations as f64)
            .fold(f64::NEG_INFINITY, f64::max);
        let migration_range = (max_migrations - min_migrations).max(1.0);

        // 计算拓扑多样性（有多少不同的 rack/zone/dc）
        let unique_racks: HashSet<&str> = filtered.iter().map(|n| n.rack.as_str()).collect();
        let unique_zones: HashSet<&str> = filtered.iter().map(|n| n.zone.as_str()).collect();
        let unique_dcs: HashSet<&str> = filtered.iter().map(|n| n.data_center.as_str()).collect();
        let _ = (unique_racks, unique_zones, unique_dcs); // 暂时保留，后续可能用

        // 计算每个节点的得分
        let mut scored: Vec<PlacementCandidate> = filtered
            .iter()
            .map(|node| {
                // 容量得分：使用率越低得分越高
                let capacity_score = if usage_range > 0.0 {
                    (1.0 - (node.usage_pct() - min_usage) / usage_range) * 100.0
                } else {
                    50.0
                };

                // 负载得分：CPU 越低得分越高
                let load_score = if cpu_range > 0.0 {
                    (1.0 - (node.cpu_pct as f64 - min_cpu) / cpu_range) * 100.0
                } else {
                    50.0
                };

                // 拓扑得分：节点在拓扑中的多样性（简化：延迟等级越低越好）
                let topology_score = (10 - node.network_latency.min(9)) as f64 * 10.0;

                // 网络得分：延迟越低越好
                let network_score = (10 - node.network_latency.min(9)) as f64 * 10.0;

                // 迁移活跃度得分：活跃迁移越少越好
                let migration_score = if migration_range > 0.0 {
                    (1.0 - (node.active_migrations as f64 - min_migrations) / migration_range)
                        * 100.0
                } else {
                    50.0
                };

                let total_score = (capacity_score * weights.capacity_weight as f64
                    + load_score * weights.load_weight as f64
                    + topology_score * weights.topology_weight as f64
                    + network_score * weights.network_weight as f64
                    + migration_score * weights.migration_weight as f64)
                    / total_weight;

                PlacementCandidate {
                    node: (*node).clone(),
                    score: total_score.clamp(0.0, 100.0),
                    score_breakdown: ScoreBreakdown {
                        capacity_score,
                        load_score,
                        topology_score,
                        network_score,
                        migration_score,
                    },
                }
            })
            .collect();

        // 按得分降序排列
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        scored
    }

    /// 检查节点是否满足约束
    fn meets_constraints(&self, node: &PlacementNode, constraints: &PlacementConstraints) -> bool {
        // 健康检查
        if !node.is_healthy {
            return false;
        }

        // 排除节点
        if constraints.excluded_nodes.contains(&node.node_id) {
            return false;
        }

        // 排除机架
        if constraints.excluded_racks.contains(&node.rack) {
            return false;
        }

        // 排除可用区
        if constraints.excluded_zones.contains(&node.zone) {
            return false;
        }

        // 最小剩余容量
        if node.free_bytes() < constraints.min_free_bytes {
            return false;
        }

        // 最大 CPU 使用率
        if constraints.max_cpu_pct > 0 && node.cpu_pct > constraints.max_cpu_pct {
            return false;
        }

        // 数据温度偏好
        if let Some(pref_temp) = constraints.preferred_temperature {
            // 如果节点有明确的温度偏好且不匹配，降低优先级（但不排除）
            // 这里简化为只排除完全不匹配的情况
            if node.preferred_temperature as u8 > pref_temp as u8 + 1 {
                return false;
            }
        }

        // 同一数据中心要求
        if let Some(ref dc) = constraints.same_data_center {
            if &node.data_center != dc {
                return false;
            }
        }

        true
    }

    /// 为副本集选择 N 个节点（考虑拓扑分布）
    pub fn select_replica_nodes(
        &self,
        candidates: &[PlacementNode],
        count: usize,
        existing_nodes: &[String],
    ) -> Vec<PlacementCandidate> {
        if candidates.len() < count {
            return Vec::new();
        }

        let mut constraints = PlacementConstraints::default();
        for id in existing_nodes {
            constraints.excluded_nodes.insert(id.clone());
        }

        let ranked = self.rank_candidates(candidates, &constraints);

        // 从排名中选择 count 个，尽量分散在不同机架
        let mut selected = Vec::with_capacity(count);
        let mut used_racks = HashSet::new();
        let mut used_zones = HashSet::new();
        let mut used_ids = HashSet::new();

        for id in existing_nodes {
            used_ids.insert(id.clone());
            // 从候选中找已有节点的机架和可用区
            if let Some(node) = candidates.iter().find(|c| &c.node_id == id) {
                used_racks.insert(node.rack.clone());
                used_zones.insert(node.zone.clone());
            }
        }

        // 第一轮：选不同可用区的
        let mut remaining = count;
        for candidate in &ranked {
            if remaining == 0 {
                break;
            }
            if used_ids.contains(&candidate.node.node_id) {
                continue;
            }
            if !used_zones.contains(&candidate.node.zone) {
                used_zones.insert(candidate.node.zone.clone());
                used_racks.insert(candidate.node.rack.clone());
                used_ids.insert(candidate.node.node_id.clone());
                selected.push(candidate.clone());
                remaining -= 1;
            }
        }

        // 第二轮：选不同机架的
        if remaining > 0 {
            for candidate in &ranked {
                if remaining == 0 {
                    break;
                }
                if used_ids.contains(&candidate.node.node_id) {
                    continue;
                }
                if !used_racks.contains(&candidate.node.rack) {
                    used_racks.insert(candidate.node.rack.clone());
                    used_ids.insert(candidate.node.node_id.clone());
                    selected.push(candidate.clone());
                    remaining -= 1;
                }
            }
        }

        // 第三轮：补充剩余
        if remaining > 0 {
            for candidate in &ranked {
                if remaining == 0 {
                    break;
                }
                if used_ids.contains(&candidate.node.node_id) {
                    continue;
                }
                used_ids.insert(candidate.node.node_id.clone());
                selected.push(candidate.clone());
                remaining -= 1;
            }
        }

        selected
    }

    /// 计算集群均衡度（0-100，越高越均衡）
    pub fn compute_cluster_balance(&self, nodes: &[PlacementNode]) -> f64 {
        if nodes.is_empty() {
            return 0.0;
        }
        if nodes.len() == 1 {
            return 100.0;
        }

        let healthy_nodes: Vec<&PlacementNode> = nodes.iter().filter(|n| n.is_healthy).collect();
        if healthy_nodes.is_empty() {
            return 0.0;
        }

        // 计算使用率的标准差
        let usages: Vec<f64> = healthy_nodes.iter().map(|n| n.usage_pct()).collect();

        let mean = usages.iter().sum::<f64>() / usages.len() as f64;
        if mean == 0.0 {
            return 100.0;
        }

        let variance = usages.iter().map(|u| (u - mean).powi(2)).sum::<f64>() / usages.len() as f64;
        let std_dev = variance.sqrt();

        // 变异系数 = 标准差 / 均值
        let cv = std_dev / mean;

        // 转换为 0-100 的均衡度分数：cv 越小越均衡
        // 假设 cv=0 时得 100 分，cv=0.5 时得 0 分
        let balance_score = (1.0 - cv / 0.5) * 100.0;
        balance_score.clamp(0.0, 100.0)
    }

    /// 预测迁移后的均衡度改善
    pub fn predict_balance_improvement(
        &self,
        nodes: &[PlacementNode],
        source_id: &str,
        target_id: &str,
        bytes: u64,
    ) -> f64 {
        let before = self.compute_cluster_balance(nodes);

        // 模拟迁移后的节点状态
        let simulated: Vec<PlacementNode> = nodes
            .iter()
            .map(|n| {
                let mut n2 = n.clone();
                if n.node_id == source_id {
                    n2.used = n2.used.saturating_sub(bytes);
                }
                if n.node_id == target_id {
                    n2.used = (n2.used + bytes).min(n2.capacity);
                }
                n2
            })
            .collect();

        let after = self.compute_cluster_balance(&simulated);
        (after - before).max(0.0)
    }
}

impl Default for PlacementEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, capacity: u64, used: u64, rack: &str, zone: &str) -> PlacementNode {
        PlacementNode {
            node_id: id.to_string(),
            addr: format!("127.0.0.1:{}", 8000 + id.len() as u16),
            capacity,
            used,
            is_healthy: true,
            data_center: "dc1".to_string(),
            zone: zone.to_string(),
            rack: rack.to_string(),
            cpu_pct: ((used as f64 / capacity as f64) * 50.0) as u8,
            network_latency: 2,
            active_migrations: 0,
            preferred_temperature: DataTemperature::Hot,
        }
    }

    #[test]
    fn test_placement_engine_default() {
        let engine = PlacementEngine::new();
        assert_eq!(engine.get_strategy(), PlacementStrategyType::Balanced);
    }

    #[test]
    fn test_set_strategy_changes_weights() {
        let engine = PlacementEngine::new();

        engine.set_strategy(PlacementStrategyType::CapacityFirst);
        let w = engine.get_weights();
        assert_eq!(w.capacity_weight, 70);

        engine.set_strategy(PlacementStrategyType::LoadFirst);
        let w = engine.get_weights();
        assert_eq!(w.load_weight, 60);

        engine.set_strategy(PlacementStrategyType::TopologyAware);
        let w = engine.get_weights();
        assert_eq!(w.topology_weight, 45);
    }

    #[test]
    fn test_select_target_basic() {
        let engine = PlacementEngine::with_strategy(PlacementStrategyType::CapacityFirst);

        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"), // 90% used
            make_node("n2", 1000, 100, "r2", "z1"), // 10% used
            make_node("n3", 1000, 500, "r3", "z2"), // 50% used
        ];

        let constraints = PlacementConstraints::default();
        let target = engine.select_target(&nodes, &constraints).unwrap();

        // 容量优先策略应该选剩余最多的（n2）
        assert_eq!(target.node.node_id, "n2");
        assert!(target.score > 0.0 && target.score <= 100.0);
    }

    #[test]
    fn test_select_target_with_exclusion() {
        let engine = PlacementEngine::with_strategy(PlacementStrategyType::CapacityFirst);

        let nodes = vec![
            make_node("n1", 1000, 100, "r1", "z1"),
            make_node("n2", 1000, 200, "r2", "z1"),
            make_node("n3", 1000, 300, "r3", "z2"),
        ];

        let mut constraints = PlacementConstraints::default();
        constraints.excluded_nodes.insert("n1".to_string());

        let target = engine.select_target(&nodes, &constraints).unwrap();
        assert_eq!(target.node.node_id, "n2"); // 排除 n1 后 n2 最空
    }

    #[test]
    fn test_select_target_min_free() {
        let engine = PlacementEngine::new();

        let nodes = vec![
            make_node("n1", 1000, 850, "r1", "z1"), // 150 free
            make_node("n2", 1000, 990, "r2", "z1"), // 10 free
        ];

        let constraints = PlacementConstraints { min_free_bytes: 100, ..Default::default() }; // 需要至少 100 bytes 空闲

        let target = engine.select_target(&nodes, &constraints).unwrap();
        assert_eq!(target.node.node_id, "n1");
    }

    #[test]
    fn test_select_target_no_healthy() {
        let engine = PlacementEngine::new();

        let mut n1 = make_node("n1", 1000, 500, "r1", "z1");
        n1.is_healthy = false;
        let nodes = vec![n1];

        let constraints = PlacementConstraints::default();
        let target = engine.select_target(&nodes, &constraints);
        assert!(target.is_none());
    }

    #[test]
    fn test_rank_candidates_order() {
        let engine = PlacementEngine::with_strategy(PlacementStrategyType::CapacityFirst);

        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"),
            make_node("n2", 1000, 100, "r2", "z1"),
            make_node("n3", 1000, 500, "r3", "z2"),
        ];

        let constraints = PlacementConstraints::default();
        let ranked = engine.rank_candidates(&nodes, &constraints);

        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].node.node_id, "n2"); // 最空的排第一
        assert_eq!(ranked[1].node.node_id, "n3");
        assert_eq!(ranked[2].node.node_id, "n1");
        assert!(ranked[0].score >= ranked[1].score);
        assert!(ranked[1].score >= ranked[2].score);
    }

    #[test]
    fn test_select_replica_nodes_topology() {
        let engine = PlacementEngine::with_strategy(PlacementStrategyType::TopologyAware);

        let nodes = vec![
            make_node("n1", 1000, 500, "r1", "z1"),
            make_node("n2", 1000, 500, "r2", "z1"),
            make_node("n3", 1000, 500, "r3", "z2"),
            make_node("n4", 1000, 500, "r4", "z2"),
            make_node("n5", 1000, 500, "r5", "z3"),
        ];

        let selected = engine.select_replica_nodes(&nodes, 3, &[]);
        assert_eq!(selected.len(), 3);

        // 应该尽量分布在不同 zone
        let zones: HashSet<&str> = selected.iter().map(|s| s.node.zone.as_str()).collect();
        assert!(zones.len() >= 2); // 至少 2 个不同 zone
    }

    #[test]
    fn test_cluster_balance_perfect() {
        let engine = PlacementEngine::new();

        let nodes = vec![
            make_node("n1", 1000, 500, "r1", "z1"),
            make_node("n2", 1000, 500, "r2", "z1"),
            make_node("n3", 1000, 500, "r3", "z2"),
        ];

        let balance = engine.compute_cluster_balance(&nodes);
        assert!(balance > 95.0); // 完全均衡应该接近 100
    }

    #[test]
    fn test_cluster_balance_unbalanced() {
        let engine = PlacementEngine::new();

        let nodes = vec![
            make_node("n1", 1000, 100, "r1", "z1"), // 10%
            make_node("n2", 1000, 900, "r2", "z1"), // 90%
        ];

        let balance = engine.compute_cluster_balance(&nodes);
        assert!(balance < 50.0); // 严重不均衡
    }

    #[test]
    fn test_predict_balance_improvement() {
        let engine = PlacementEngine::new();

        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"), // 90%
            make_node("n2", 1000, 100, "r2", "z1"), // 10%
        ];

        let improvement = engine.predict_balance_improvement(&nodes, "n1", "n2", 400);
        assert!(improvement > 0.0); // 迁移后应该更均衡
    }

    #[test]
    fn test_placement_node_free_bytes() {
        let node = make_node("n1", 1000, 300, "r1", "z1");
        assert_eq!(node.free_bytes(), 700);
    }

    #[test]
    fn test_placement_node_usage_pct() {
        let node = make_node("n1", 1000, 250, "r1", "z1");
        assert!((node.usage_pct() - 25.0).abs() < 0.1);
    }

    #[test]
    fn test_score_breakdown() {
        let engine = PlacementEngine::new();

        let nodes = vec![make_node("n1", 1000, 500, "r1", "z1")];
        let constraints = PlacementConstraints::default();
        let result = engine.rank_candidates(&nodes, &constraints);

        assert_eq!(result.len(), 1);
        let bd = &result[0].score_breakdown;
        assert!(bd.capacity_score >= 0.0 && bd.capacity_score <= 100.0);
        assert!(bd.load_score >= 0.0 && bd.load_score <= 100.0);
        assert!(bd.topology_score >= 0.0 && bd.topology_score <= 100.0);
    }

    #[test]
    fn test_constraints_excluded_racks() {
        let engine = PlacementEngine::new();

        let nodes =
            vec![make_node("n1", 1000, 100, "r1", "z1"), make_node("n2", 1000, 200, "r2", "z1")];

        let mut constraints = PlacementConstraints::default();
        constraints.excluded_racks.insert("r1".to_string());

        let target = engine.select_target(&nodes, &constraints).unwrap();
        assert_eq!(target.node.node_id, "n2");
    }

    #[test]
    fn test_constraints_same_dc() {
        let engine = PlacementEngine::new();

        let mut n1 = make_node("n1", 1000, 100, "r1", "z1");
        n1.data_center = "dc1".to_string();
        let mut n2 = make_node("n2", 1000, 200, "r2", "z2");
        n2.data_center = "dc2".to_string();

        let nodes = vec![n1, n2];
        let constraints = PlacementConstraints {
            same_data_center: Some("dc1".to_string()),
            ..Default::default()
        };

        let target = engine.select_target(&nodes, &constraints).unwrap();
        assert_eq!(target.node.node_id, "n1");
    }

    #[test]
    fn test_data_temperature_ordering() {
        assert!(DataTemperature::Hot < DataTemperature::Warm);
        assert!(DataTemperature::Warm < DataTemperature::Cold);
        assert!(DataTemperature::Cold < DataTemperature::Archive);
    }

    #[test]
    fn test_placement_weights_default() {
        let w = PlacementWeights::default();
        assert_eq!(w.capacity_weight, 40);
        assert_eq!(w.load_weight, 25);
        assert_eq!(w.topology_weight, 15);
        assert_eq!(w.network_weight, 10);
        assert_eq!(w.migration_weight, 10);
    }

    #[test]
    fn test_custom_weights() {
        let engine = PlacementEngine::new();
        engine.set_weights(PlacementWeights {
            capacity_weight: 100,
            load_weight: 0,
            topology_weight: 0,
            network_weight: 0,
            migration_weight: 0,
        });

        let w = engine.get_weights();
        assert_eq!(w.capacity_weight, 100);
    }

    #[test]
    fn test_single_node_balance() {
        let engine = PlacementEngine::new();
        let nodes = vec![make_node("n1", 1000, 500, "r1", "z1")];
        assert_eq!(engine.compute_cluster_balance(&nodes), 100.0);
    }

    #[test]
    fn test_empty_nodes_balance() {
        let engine = PlacementEngine::new();
        let nodes: Vec<PlacementNode> = Vec::new();
        assert_eq!(engine.compute_cluster_balance(&nodes), 0.0);
    }
}

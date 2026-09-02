// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 分布式调度器模块
//!
//! 负责 Volume 节点的容量感知调度、副本放置策略、
//! 数据均衡、故障检测与自动恢复、卷在线迁移等功能。
//!
//! 设计参考 SeaweedFS 的 Master 调度架构，结合 Rack/AZ 感知
//! 和加权评分机制，支持千亿级文件的分布式调度。

use crate::error::{MasterError, MasterResult};
use crate::volume_allocator::VolumeInfo;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// 调度权重配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerWeights {
    /// 剩余容量权重（0-100）
    pub capacity_weight: u32,
    /// IO 负载权重（0-100）
    pub io_load_weight: u32,
    /// 网络延迟权重（0-100）
    pub network_weight: u32,
}

impl Default for SchedulerWeights {
    fn default() -> Self {
        SchedulerWeights {
            capacity_weight: 50,
            io_load_weight: 30,
            network_weight: 20,
        }
    }
}

/// 副本放置策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlacementStrategy {
    /// 跨机架：副本分布在不同机架
    RackAware,
    /// 跨可用区：副本分布在不同可用区
    ZoneAware,
    /// 反亲和：同一副本集的副本尽量分散
    AntiAffinity,
    /// 随机放置（简化模式）
    Random,
}

impl Default for PlacementStrategy {
    fn default() -> Self {
        PlacementStrategy::RackAware
    }
}

/// 节点拓扑信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTopology {
    /// 节点 ID
    pub node_id: String,
    /// 所属数据中心
    pub data_center: String,
    /// 所属可用区
    pub zone: String,
    /// 所属机架
    pub rack: String,
    /// 网络延迟等级（数值越低越好，1-10）
    pub network_latency_level: u8,
}

/// 节点负载信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeLoad {
    /// CPU 使用率（0-100）
    pub cpu_pct: u8,
    /// 内存使用率（0-100）
    pub memory_pct: u8,
    /// 磁盘 IOPS
    pub iops: u64,
    /// 网络吞吐（bytes/s）
    pub network_bps: u64,
    /// 活跃连接数
    pub active_connections: u32,
}

impl Default for NodeLoad {
    fn default() -> Self {
        NodeLoad {
            cpu_pct: 0,
            memory_pct: 0,
            iops: 0,
            network_bps: 0,
            active_connections: 0,
        }
    }
}

/// 调度用的节点完整信息
#[derive(Debug, Clone)]
pub struct SchedulerNode {
    /// 基础卷信息
    pub volume: VolumeInfo,
    /// 拓扑信息
    pub topology: NodeTopology,
    /// 负载信息
    pub load: NodeLoad,
    /// 综合得分（越高越优）
    pub score: f64,
}

/// 卷迁移任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMigrationTask {
    /// 任务 ID
    pub task_id: String,
    /// 源卷 ID
    pub source_volume_id: String,
    /// 目标卷 ID
    pub target_volume_id: String,
    /// 目标卷地址
    pub target_addr: String,
    /// 迁移的副本集 ID
    pub replica_set_id: String,
    /// 迁移大小（字节）
    pub size_bytes: u64,
    /// 已迁移字节数
    pub migrated_bytes: u64,
    /// 迁移状态
    pub status: MigrationStatus,
    /// 创建时间（ms）
    pub created_at_ms: u64,
    /// 开始时间（ms）
    pub started_at_ms: Option<u64>,
    /// 完成时间（ms）
    pub completed_at_ms: Option<u64>,
    /// 失败原因
    pub error: Option<String>,
    /// 限速（bytes/s），0 表示不限速
    pub bandwidth_limit_bps: u64,
}

/// 迁移状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// 等待执行
    Pending,
    /// 正在执行
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
    /// 暂停
    Paused,
}

/// 均衡计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancePlan {
    /// 计划 ID
    pub plan_id: String,
    /// 生成时间
    pub created_at_ms: u64,
    /// 需要迁移的任务列表
    pub migrations: Vec<VolumeMigrationTask>,
    /// 预计总迁移量
    pub total_bytes: u64,
    /// 预计改善程度（0-100，越高越好）
    pub estimated_improvement: u8,
}

/// 故障恢复计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlan {
    /// 故障节点 ID 列表
    pub failed_nodes: Vec<String>,
    /// 需要重建的副本数
    pub replicas_to_rebuild: u64,
    /// 预计影响的卷数
    pub affected_volumes: u64,
    /// 重建任务列表
    pub rebuild_tasks: Vec<RebuildTask>,
}

/// 重建任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildTask {
    /// 任务 ID
    pub task_id: String,
    /// 副本集 ID
    pub replica_set_id: String,
    /// 丢失的副本所在节点
    pub lost_node: String,
    /// 重建到的目标节点
    pub target_node: String,
    /// 目标地址
    pub target_addr: String,
    /// 数据大小
    pub size_bytes: u64,
    /// 优先级（0-10，越高越紧急）
    pub priority: u8,
}

/// 数据温度（用于冷热分层调度）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum DataTemperature {
    /// 热数据：高频访问
    Hot,
    /// 温数据：中频访问
    Warm,
    /// 冷数据：低频访问
    Cold,
    /// 归档数据：几乎不访问
    Archive,
}

impl Default for DataTemperature {
    fn default() -> Self {
        DataTemperature::Hot
    }
}

/// 分布式调度器
///
/// 核心能力：
/// - 容量感知调度：基于剩余容量/IO负载/网络延迟的加权评分
/// - 副本放置策略：跨机架/跨可用区/反亲和
/// - 数据均衡器：冷热迁移、容量均衡
/// - 故障检测与自动恢复：心跳超时 -> 标记不健康 -> 重建副本
/// - 卷在线迁移：不中断服务的迁移
pub struct DistributedScheduler {
    /// 调度权重
    weights: parking_lot::RwLock<SchedulerWeights>,
    /// 放置策略
    placement_strategy: parking_lot::RwLock<PlacementStrategy>,
    /// 节点拓扑信息（node_id -> topology）
    topology: parking_lot::RwLock<HashMap<String, NodeTopology>>,
    /// 节点负载信息（node_id -> load）
    node_loads: parking_lot::RwLock<HashMap<String, NodeLoad>>,
    /// 迁移任务队列
    migration_tasks: parking_lot::Mutex<Vec<VolumeMigrationTask>>,
    /// 活跃迁移数限制
    max_concurrent_migrations: parking_lot::Mutex<usize>,
    /// 调度统计
    stats: Arc<SchedulerStats>,
    /// 心跳超时时间（ms）
    heartbeat_timeout_ms: u64,
}

/// 调度器统计
#[derive(Debug, Default)]
pub struct SchedulerStats {
    /// 总调度次数
    pub scheduling_total: parking_lot::Mutex<u64>,
    /// 成功调度次数
    pub scheduling_success: parking_lot::Mutex<u64>,
    /// 均衡计划生成数
    pub rebalance_plans: parking_lot::Mutex<u64>,
    /// 已完成迁移数
    pub migrations_completed: parking_lot::Mutex<u64>,
    /// 故障恢复次数
    pub recoveries_total: parking_lot::Mutex<u64>,
    /// 重建副本数
    pub replicas_rebuilt: parking_lot::Mutex<u64>,
}

impl SchedulerStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert(
            "scheduler_scheduling_total".into(),
            *self.scheduling_total.lock(),
        );
        m.insert(
            "scheduler_scheduling_success".into(),
            *self.scheduling_success.lock(),
        );
        m.insert(
            "scheduler_rebalance_plans".into(),
            *self.rebalance_plans.lock(),
        );
        m.insert(
            "scheduler_migrations_completed".into(),
            *self.migrations_completed.lock(),
        );
        m.insert(
            "scheduler_recoveries_total".into(),
            *self.recoveries_total.lock(),
        );
        m.insert(
            "scheduler_replicas_rebuilt".into(),
            *self.replicas_rebuilt.lock(),
        );
        m
    }
}

impl DistributedScheduler {
    /// 创建新的分布式调度器
    pub fn new(heartbeat_timeout_ms: u64) -> Self {
        Self {
            weights: parking_lot::RwLock::new(SchedulerWeights::default()),
            placement_strategy: parking_lot::RwLock::new(PlacementStrategy::default()),
            topology: parking_lot::RwLock::new(HashMap::new()),
            node_loads: parking_lot::RwLock::new(HashMap::new()),
            migration_tasks: parking_lot::Mutex::new(Vec::new()),
            max_concurrent_migrations: parking_lot::Mutex::new(4),
            stats: Arc::new(SchedulerStats::new()),
            heartbeat_timeout_ms,
        }
    }

    /// 设置调度权重
    pub fn set_weights(&self, weights: SchedulerWeights) {
        *self.weights.write() = weights;
    }

    /// 获取当前权重
    pub fn get_weights(&self) -> SchedulerWeights {
        self.weights.read().clone()
    }

    /// 设置放置策略
    pub fn set_placement_strategy(&self, strategy: PlacementStrategy) {
        *self.placement_strategy.write() = strategy;
    }

    /// 获取当前放置策略
    pub fn get_placement_strategy(&self) -> PlacementStrategy {
        *self.placement_strategy.read()
    }

    /// 注册节点拓扑信息
    pub fn register_topology(&self, topology: NodeTopology) {
        self.topology
            .write()
            .insert(topology.node_id.clone(), topology);
    }

    /// 更新节点负载
    pub fn update_node_load(&self, node_id: &str, load: NodeLoad) {
        self.node_loads
            .write()
            .insert(node_id.to_string(), load);
    }

    /// 获取统计信息
    pub fn stats(&self) -> Arc<SchedulerStats> {
        self.stats.clone()
    }

    /// 获取节点拓扑信息的读写锁引用（供外部查询/测试使用）
    pub fn topology(&self) -> &parking_lot::RwLock<HashMap<String, NodeTopology>> {
        &self.topology
    }

    // -----------------------------------------------------------------------
    // 容量感知调度
    // -----------------------------------------------------------------------

    /// 计算节点的综合调度得分（0-100，越高越优）
    pub fn compute_node_score(&self, node: &VolumeInfo) -> f64 {
        let weights = self.weights.read();

        // 容量得分：剩余容量比例，越高越好
        let capacity_ratio = if node.capacity > 0 {
            1.0 - (node.used as f64 / node.capacity as f64)
        } else {
            0.0
        };
        let capacity_score = capacity_ratio * 100.0;

        // IO 负载得分：CPU 越低越好
        let load = self
            .node_loads
            .read()
            .get(&node.id)
            .cloned()
            .unwrap_or_default();
        let io_score = 100.0 - load.cpu_pct as f64;

        // 网络得分：延迟等级越低越好
        let topo = self.topology.read().get(&node.id).cloned();
        let network_score = match topo {
            Some(t) => (10 - t.network_latency_level.min(9)) as f64 * 10.0,
            None => 50.0, // 未知则给中等分
        };

        // 加权求和
        let total_weight =
            (weights.capacity_weight + weights.io_load_weight + weights.network_weight) as f64;
        if total_weight == 0.0 {
            return capacity_score;
        }

        let weighted = (capacity_score * weights.capacity_weight as f64
            + io_score * weights.io_load_weight as f64
            + network_score * weights.network_weight as f64)
            / total_weight;

        weighted.max(0.0).min(100.0)
    }

    /// 从候选节点中选择最佳的 N 个（考虑放置策略）
    pub fn select_best_nodes(
        &self,
        candidates: &[VolumeInfo],
        count: usize,
        existing_nodes: &[String],
    ) -> MasterResult<Vec<VolumeInfo>> {
        *self.stats.scheduling_total.lock() += 1;

        if candidates.len() < count {
            return Err(MasterError::NoCapacity(format!(
                "need {} nodes, only {} available",
                count,
                candidates.len()
            )));
        }

        let strategy = *self.placement_strategy.read();

        // 计算所有候选节点的得分
        let mut scored: Vec<(f64, VolumeInfo)> = candidates
            .iter()
            .map(|v| (self.compute_node_score(v), v.clone()))
            .collect();

        // 按得分降序排列
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // 根据放置策略选择节点
        let selected = match strategy {
            PlacementStrategy::RackAware => {
                self.select_rack_aware(&scored, count, existing_nodes)?
            }
            PlacementStrategy::ZoneAware => {
                self.select_zone_aware(&scored, count, existing_nodes)?
            }
            PlacementStrategy::AntiAffinity => {
                self.select_anti_affinity(&scored, count, existing_nodes)?
            }
            PlacementStrategy::Random => {
                // 随机选 count 个（从高分到低分前 2*count 中随机）
                self.select_random(&scored, count)?
            }
        };

        *self.stats.scheduling_success.lock() += 1;
        Ok(selected)
    }

    /// 跨机架选择：尽量选不同机架的节点
    fn select_rack_aware(
        &self,
        scored: &[(f64, VolumeInfo)],
        count: usize,
        existing: &[String],
    ) -> MasterResult<Vec<VolumeInfo>> {
        let topo = self.topology.read();
        let mut selected = Vec::with_capacity(count);
        let mut used_racks = HashSet::new();
        let mut selected_ids = HashSet::new();

        // 已有节点占用的机架
        for id in existing {
            if let Some(t) = topo.get(id) {
                used_racks.insert(t.rack.clone());
            }
            selected_ids.insert(id.clone());
        }

        // 第一轮：优先选不同机架
        for (_, node) in scored {
            if selected.len() >= count {
                break;
            }
            if selected_ids.contains(&node.id) {
                continue;
            }
            if !node.is_alive {
                continue;
            }
            let rack = topo
                .get(&node.id)
                .map(|t| t.rack.clone())
                .unwrap_or_else(|| format!("default-{}", node.id));
            if !used_racks.contains(&rack) {
                used_racks.insert(rack);
                selected_ids.insert(node.id.clone());
                selected.push(node.clone());
            }
        }

        // 第二轮：如果还不够，从高分依次补充
        if selected.len() < count {
            for (_, node) in scored {
                if selected.len() >= count {
                    break;
                }
                if selected_ids.contains(&node.id) {
                    continue;
                }
                if !node.is_alive {
                    continue;
                }
                selected_ids.insert(node.id.clone());
                selected.push(node.clone());
            }
        }

        if selected.len() < count {
            return Err(MasterError::NoCapacity(format!(
                "rack-aware placement: need {} nodes, only {} qualified",
                count,
                selected.len()
            )));
        }

        Ok(selected)
    }

    /// 跨可用区选择：尽量选不同可用区的节点
    fn select_zone_aware(
        &self,
        scored: &[(f64, VolumeInfo)],
        count: usize,
        existing: &[String],
    ) -> MasterResult<Vec<VolumeInfo>> {
        let topo = self.topology.read();
        let mut selected = Vec::with_capacity(count);
        let mut used_zones = HashSet::new();
        let mut selected_ids = HashSet::new();

        for id in existing {
            if let Some(t) = topo.get(id) {
                used_zones.insert(t.zone.clone());
            }
            selected_ids.insert(id.clone());
        }

        // 第一轮：优先选不同 AZ
        for (_, node) in scored {
            if selected.len() >= count {
                break;
            }
            if selected_ids.contains(&node.id) {
                continue;
            }
            if !node.is_alive {
                continue;
            }
            let zone = topo
                .get(&node.id)
                .map(|t| t.zone.clone())
                .unwrap_or_else(|| "default".to_string());
            if !used_zones.contains(&zone) {
                used_zones.insert(zone);
                selected_ids.insert(node.id.clone());
                selected.push(node.clone());
            }
        }

        // 第二轮：补充
        if selected.len() < count {
            for (_, node) in scored {
                if selected.len() >= count {
                    break;
                }
                if selected_ids.contains(&node.id) {
                    continue;
                }
                if !node.is_alive {
                    continue;
                }
                selected_ids.insert(node.id.clone());
                selected.push(node.clone());
            }
        }

        if selected.len() < count {
            return Err(MasterError::NoCapacity(format!(
                "zone-aware placement: need {} nodes, only {} qualified",
                count,
                selected.len()
            )));
        }

        Ok(selected)
    }

    /// 反亲和选择：副本集的副本尽量分散在不同故障域
    fn select_anti_affinity(
        &self,
        scored: &[(f64, VolumeInfo)],
        count: usize,
        existing: &[String],
    ) -> MasterResult<Vec<VolumeInfo>> {
        // 反亲和策略：优先不同 DC，其次不同 AZ，其次不同 Rack
        // 组合键：dc:zone:rack，越多样越好
        let topo = self.topology.read();
        let mut selected = Vec::with_capacity(count);
        let mut used_keys = HashSet::new();
        let mut selected_ids = HashSet::new();

        for id in existing {
            if let Some(t) = topo.get(id) {
                used_keys.insert(format!("{}:{}:{}", t.data_center, t.zone, t.rack));
            }
            selected_ids.insert(id.clone());
        }

        // 按多样性选择：先选 DC 不同的，再选 AZ 不同的，最后选 Rack 不同的
        let mut remaining = count;

        // Level 1: 不同 DC
        for (_, node) in scored {
            if remaining == 0 {
                break;
            }
            if selected_ids.contains(&node.id) || !node.is_alive {
                continue;
            }
            let dc = topo
                .get(&node.id)
                .map(|t| t.data_center.clone())
                .unwrap_or_default();
            let is_new_dc = !used_keys.iter().any(|k| k.starts_with(&format!("{}:", dc)));
            if is_new_dc {
                let key = format!(
                    "{}:{}:{}",
                    topo.get(&node.id).map(|t| t.data_center.as_str()).unwrap_or(""),
                    topo.get(&node.id).map(|t| t.zone.as_str()).unwrap_or(""),
                    topo.get(&node.id).map(|t| t.rack.as_str()).unwrap_or("")
                );
                used_keys.insert(key);
                selected_ids.insert(node.id.clone());
                selected.push(node.clone());
                remaining -= 1;
            }
        }

        // Level 2: 不同 AZ
        if remaining > 0 {
            for (_, node) in scored {
                if remaining == 0 {
                    break;
                }
                if selected_ids.contains(&node.id) || !node.is_alive {
                    continue;
                }
                if let Some(t) = topo.get(&node.id) {
                    let is_new_az = !used_keys
                        .iter()
                        .any(|k| k.ends_with(&format!(":{}:{}", t.zone, t.rack)));
                    if is_new_az {
                        let key = format!("{}:{}:{}", t.data_center, t.zone, t.rack);
                        used_keys.insert(key);
                        selected_ids.insert(node.id.clone());
                        selected.push(node.clone());
                        remaining -= 1;
                    }
                }
            }
        }

        // Level 3: 补充剩余
        if remaining > 0 {
            for (_, node) in scored {
                if remaining == 0 {
                    break;
                }
                if selected_ids.contains(&node.id) || !node.is_alive {
                    continue;
                }
                selected_ids.insert(node.id.clone());
                selected.push(node.clone());
                remaining -= 1;
            }
        }

        if selected.len() < count {
            return Err(MasterError::NoCapacity(format!(
                "anti-affinity placement: need {} nodes, only {} qualified",
                count,
                selected.len()
            )));
        }

        Ok(selected)
    }

    /// 随机选择（从前 2*count 个高分节点中随机）
    fn select_random(
        &self,
        scored: &[(f64, VolumeInfo)],
        count: usize,
    ) -> MasterResult<Vec<VolumeInfo>> {
        use rand::seq::SliceRandom;

        let pool_size = (count * 2).min(scored.len());
        let pool: Vec<&(f64, VolumeInfo)> = scored.iter().take(pool_size).collect();

        let mut rng = rand::thread_rng();
        let mut indices: Vec<usize> = (0..pool.len()).collect();
        indices.shuffle(&mut rng);

        let mut selected = Vec::with_capacity(count);
        for &idx in indices.iter().take(count) {
            selected.push(pool[idx].1.clone());
        }

        Ok(selected)
    }

    // -----------------------------------------------------------------------
    // 数据均衡器（Rebalance）
    // -----------------------------------------------------------------------

    /// 生成均衡计划
    ///
    /// 分析当前集群容量分布，找出不均衡的节点，
    /// 生成迁移计划将数据从高负载节点迁移到低负载节点。
    pub fn generate_rebalance_plan(
        &self,
        nodes: &[VolumeInfo],
        threshold_pct: u8,
    ) -> RebalancePlan {
        *self.stats.rebalance_plans.lock() += 1;

        if nodes.is_empty() {
            return RebalancePlan {
                plan_id: generate_plan_id(),
                created_at_ms: now_ms(),
                migrations: Vec::new(),
                total_bytes: 0,
                estimated_improvement: 0,
            };
        }

        // 计算平均使用率
        let total_capacity: u64 = nodes.iter().map(|n| n.capacity).sum();
        let total_used: u64 = nodes.iter().map(|n| n.used).sum();
        let avg_usage_pct = if total_capacity > 0 {
            (total_used as f64 / total_capacity as f64 * 100.0) as u8
        } else {
            0
        };

        // 分类：高于阈值的为源候选，低于平均值的为目标候选
        let mut sources: Vec<&VolumeInfo> = Vec::new();
        let mut targets: Vec<&VolumeInfo> = Vec::new();

        for node in nodes {
            if !node.is_alive || node.capacity == 0 {
                continue;
            }
            let usage_pct = (node.used as f64 / node.capacity as f64 * 100.0) as u8;
            if usage_pct > avg_usage_pct.saturating_add(threshold_pct) {
                sources.push(node);
            } else if usage_pct < avg_usage_pct.saturating_sub(threshold_pct) {
                targets.push(node);
            }
        }

        let mut migrations = Vec::new();
        let mut total_bytes = 0u64;

        // 简化：为每个源节点生成一个迁移任务到目标节点
        for src in &sources {
            if targets.is_empty() {
                break;
            }
            let src_usage = src.used as f64 / src.capacity as f64;
            let avg_ratio = avg_usage_pct as f64 / 100.0;
            let bytes_to_move =
                ((src_usage - avg_ratio) * src.capacity as f64).max(0.0) as u64;

            if bytes_to_move < 1024 * 1024 {
                continue; // 小于 1MB 不迁移
            }

            // 找一个得分最高的目标节点
            let best_target = targets
                .iter()
                .max_by(|a, b| {
                    let sa = self.compute_node_score(a);
                    let sb = self.compute_node_score(b);
                    sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();

            let actual_bytes = bytes_to_move
                .min(best_target.capacity.saturating_sub(best_target.used));

            if actual_bytes == 0 {
                continue;
            }

            migrations.push(VolumeMigrationTask {
                task_id: generate_migration_id(),
                source_volume_id: src.id.clone(),
                target_volume_id: best_target.id.clone(),
                target_addr: best_target.addr.clone(),
                replica_set_id: format!("rebalance-{}-{}", src.id, best_target.id),
                size_bytes: actual_bytes,
                migrated_bytes: 0,
                status: MigrationStatus::Pending,
                created_at_ms: now_ms(),
                started_at_ms: None,
                completed_at_ms: None,
                error: None,
                bandwidth_limit_bps: 0,
            });

            total_bytes += actual_bytes;
        }

        // 估算改善程度
        let current_std = self.compute_usage_stddev(nodes);
        let estimated_improvement = if current_std > 0.0 {
            // 根据当前使用率标准差映射改善程度（0-100）
            // 标准差最大约 0.5（极端 0/1 分布），据此线性映射
            let improvement = (current_std / 0.5 * 100.0).min(100.0);
            improvement as u8
        } else {
            0
        };

        RebalancePlan {
            plan_id: generate_plan_id(),
            created_at_ms: now_ms(),
            migrations,
            total_bytes,
            estimated_improvement,
        }
    }

    /// 计算节点使用率的标准差
    fn compute_usage_stddev(&self, nodes: &[VolumeInfo]) -> f64 {
        if nodes.is_empty() {
            return 0.0;
        }
        let usages: Vec<f64> = nodes
            .iter()
            .filter(|n| n.capacity > 0 && n.is_alive)
            .map(|n| n.used as f64 / n.capacity as f64)
            .collect();

        if usages.is_empty() {
            return 0.0;
        }

        let mean: f64 = usages.iter().sum::<f64>() / usages.len() as f64;
        let variance: f64 = usages
            .iter()
            .map(|u| (u - mean).powi(2))
            .sum::<f64>()
            / usages.len() as f64;

        variance.sqrt()
    }

    // -----------------------------------------------------------------------
    // 故障检测与自动恢复
    // -----------------------------------------------------------------------

    /// 检测故障节点并生成恢复计划
    ///
    /// 根据心跳超时检测不健康节点，为受影响的副本集生成重建任务。
    pub fn detect_and_plan_recovery(
        &self,
        nodes: &[VolumeInfo],
        replica_map: &HashMap<String, Vec<String>>, // set_id -> [volume_id]
        last_heartbeats: &HashMap<String, u64>,     // volume_id -> last_hb_ms
    ) -> RecoveryPlan {
        let now = now_ms();
        let mut failed_nodes = Vec::new();
        let mut affected_sets = HashSet::new();

        // 检测故障节点
        for node in nodes {
            let last_hb = *last_heartbeats.get(&node.id).unwrap_or(&0);
            let timed_out = now.saturating_sub(last_hb) > self.heartbeat_timeout_ms;
            if !node.is_alive || timed_out {
                failed_nodes.push(node.id.clone());
            }
        }

        // 找出受影响的副本集
        for node_id in &failed_nodes {
            for (set_id, volumes) in replica_map {
                if volumes.contains(node_id) {
                    affected_sets.insert(set_id.clone());
                }
            }
        }

        // 生成重建任务
        let mut rebuild_tasks = Vec::new();
        let alive_nodes: Vec<&VolumeInfo> = nodes
            .iter()
            .filter(|n| {
                n.is_alive
                    && now.saturating_sub(*last_heartbeats.get(&n.id).unwrap_or(&0))
                        <= self.heartbeat_timeout_ms
            })
            .collect();

        for set_id in &affected_sets {
            let volumes = replica_map.get(set_id).cloned().unwrap_or_default();
            // 找丢失的副本
            let lost_count = volumes
                .iter()
                .filter(|v| failed_nodes.contains(v))
                .count();

            for lost_node in volumes.iter().filter(|v| failed_nodes.contains(v)) {
                // 找一个不在该副本集中的健康节点作为重建目标
                let candidates: Vec<VolumeInfo> = alive_nodes
                    .iter()
                    .filter(|n| !volumes.contains(&n.id))
                    .map(|n| (*n).clone())
                    .collect();

                if let Ok(selected) = self.select_best_nodes(&candidates, 1, &volumes) {
                    if let Some(target) = selected.first() {
                        rebuild_tasks.push(RebuildTask {
                            task_id: format!("rebuild-{}-{}", set_id, lost_node),
                            replica_set_id: set_id.clone(),
                            lost_node: lost_node.clone(),
                            target_node: target.id.clone(),
                            target_addr: target.addr.clone(),
                            size_bytes: 0, // 需要从现有副本获取实际大小
                            priority: if lost_count > 1 { 10 } else { 7 },
                        });
                    }
                }
            }
        }

        *self.stats.recoveries_total.lock() += 1;
        *self.stats.replicas_rebuilt.lock() += rebuild_tasks.len() as u64;

        RecoveryPlan {
            failed_nodes,
            replicas_to_rebuild: rebuild_tasks.len() as u64,
            affected_volumes: affected_sets.len() as u64,
            rebuild_tasks,
        }
    }

    /// 简化版故障恢复计划生成：仅根据节点 is_alive 状态检测故障
    ///
    /// 适用于不需要副本映射和心跳时间戳的简化场景。
    pub fn generate_recovery_plan(&self, nodes: &[VolumeInfo]) -> RecoveryPlan {
        let now = now_ms();
        let replica_map: HashMap<String, Vec<String>> = HashMap::new();
        let last_heartbeats: HashMap<String, u64> = nodes
            .iter()
            .map(|n| (n.id.clone(), if n.is_alive { now } else { 0 }))
            .collect();
        self.detect_and_plan_recovery(nodes, &replica_map, &last_heartbeats)
    }

    // -----------------------------------------------------------------------
    // 卷迁移管理
    // -----------------------------------------------------------------------

    /// 提交迁移任务
    pub fn submit_migration(&self, task: VolumeMigrationTask) {
        self.migration_tasks.lock().push(task);
    }

    /// 获取待执行的迁移任务（考虑并发限制）
    pub fn get_pending_migrations(&self, max_count: usize) -> Vec<VolumeMigrationTask> {
        let tasks = self.migration_tasks.lock();
        let running_count = tasks
            .iter()
            .filter(|t| t.status == MigrationStatus::Running)
            .count();
        let max_concurrent = *self.max_concurrent_migrations.lock();
        let available = max_concurrent.saturating_sub(running_count).min(max_count);

        tasks
            .iter()
            .filter(|t| t.status == MigrationStatus::Pending)
            .take(available)
            .cloned()
            .collect()
    }

    /// 更新迁移任务状态
    pub fn update_migration_status(
        &self,
        task_id: &str,
        status: MigrationStatus,
        migrated_bytes: u64,
        error: Option<String>,
    ) -> MasterResult<()> {
        let mut tasks = self.migration_tasks.lock();
        let now = now_ms();

        for task in tasks.iter_mut() {
            if task.task_id == task_id {
                if status == MigrationStatus::Running && task.started_at_ms.is_none() {
                    task.started_at_ms = Some(now);
                }
                if matches!(status, MigrationStatus::Completed | MigrationStatus::Failed) {
                    task.completed_at_ms = Some(now);
                    if status == MigrationStatus::Completed {
                        *self.stats.migrations_completed.lock() += 1;
                    }
                }
                task.status = status;
                task.migrated_bytes = migrated_bytes;
                task.error = error;
                return Ok(());
            }
        }

        Err(MasterError::Internal(format!(
            "migration task {} not found",
            task_id
        )))
    }

    /// 获取所有迁移任务
    pub fn list_migrations(&self) -> Vec<VolumeMigrationTask> {
        self.migration_tasks.lock().clone()
    }

    /// 设置最大并发迁移数
    pub fn set_max_concurrent_migrations(&self, max: usize) {
        *self.max_concurrent_migrations.lock() = max;
    }

    // -----------------------------------------------------------------------
    // 在线卷迁移
    // -----------------------------------------------------------------------

    /// 计划在线卷迁移（不中断服务）
    ///
    /// 迁移流程：
    /// 1. 在目标节点创建新副本
    /// 2. 后台同步历史数据
    /// 3. 增量同步新写入数据
    /// 4. 切换流量到新副本
    /// 5. 移除旧副本
    pub fn plan_online_migration(
        &self,
        set_id: &str,
        from_volume: &VolumeInfo,
        available_nodes: &[VolumeInfo],
        existing_replicas: &[String],
    ) -> MasterResult<VolumeMigrationTask> {
        // 选择目标节点
        let candidates: Vec<VolumeInfo> = available_nodes
            .iter()
            .filter(|n| n.is_alive && !existing_replicas.contains(&n.id))
            .cloned()
            .collect();

        let targets = self.select_best_nodes(&candidates, 1, existing_replicas)?;
        let target = targets
            .into_iter()
            .next()
            .ok_or_else(|| MasterError::NoCapacity("no target node available".into()))?;

        Ok(VolumeMigrationTask {
            task_id: generate_migration_id(),
            source_volume_id: from_volume.id.clone(),
            target_volume_id: target.id,
            target_addr: target.addr,
            replica_set_id: set_id.to_string(),
            size_bytes: from_volume.used,
            migrated_bytes: 0,
            status: MigrationStatus::Pending,
            created_at_ms: now_ms(),
            started_at_ms: None,
            completed_at_ms: None,
            error: None,
            bandwidth_limit_bps: 0,
        })
    }
}

impl Default for DistributedScheduler {
    fn default() -> Self {
        Self::new(1500)
    }
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn generate_migration_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!(
        "mig-{:08x}{:08x}",
        rng.gen::<u32>(),
        rng.gen::<u32>()
    )
}

fn generate_plan_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!(
        "plan-{:08x}{:08x}",
        rng.gen::<u32>(),
        rng.gen::<u32>()
    )
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_volume(id: &str, capacity: u64, used: u64, alive: bool) -> VolumeInfo {
        VolumeInfo {
            id: id.to_string(),
            addr: format!("127.0.0.1:{}", 8000 + id.len() as u16),
            capacity,
            used,
            is_alive: alive,
        }
    }

    fn make_scheduler() -> DistributedScheduler {
        DistributedScheduler::new(1500)
    }

    #[test]
    fn test_compute_node_score() {
        let scheduler = make_scheduler();
        let node = make_volume("v1", 1000, 200, true);
        let score = scheduler.compute_node_score(&node);
        assert!(score >= 0.0 && score <= 100.0);
    }

    #[test]
    fn test_score_capacity_sensitivity() {
        let scheduler = make_scheduler();
        let empty = make_volume("v1", 1000, 0, true);
        let full = make_volume("v2", 1000, 999, true);
        let score_empty = scheduler.compute_node_score(&empty);
        let score_full = scheduler.compute_node_score(&full);
        assert!(score_empty > score_full);
    }

    #[test]
    fn test_select_best_nodes_random() {
        let scheduler = make_scheduler();
        scheduler.set_placement_strategy(PlacementStrategy::Random);

        let nodes: Vec<VolumeInfo> = (0..10)
            .map(|i| make_volume(&format!("v{}", i), 1000, i * 100, true))
            .collect();

        let selected = scheduler.select_best_nodes(&nodes, 3, &[]).unwrap();
        assert_eq!(selected.len(), 3);
    }

    #[test]
    fn test_select_best_nodes_rack_aware() {
        let scheduler = make_scheduler();
        scheduler.set_placement_strategy(PlacementStrategy::RackAware);

        // 注册拓扑
        for i in 0..6 {
            scheduler.register_topology(NodeTopology {
                node_id: format!("v{}", i),
                data_center: "dc1".to_string(),
                zone: "zone1".to_string(),
                rack: format!("rack{}", i / 2), // 每 2 个节点一个机架
                network_latency_level: 1,
            });
        }

        let nodes: Vec<VolumeInfo> = (0..6)
            .map(|i| make_volume(&format!("v{}", i), 1000, i * 100, true))
            .collect();

        let selected = scheduler.select_best_nodes(&nodes, 3, &[]).unwrap();
        assert_eq!(selected.len(), 3);

        // 验证是否来自不同机架
        let topo = scheduler.topology.read();
        let racks: HashSet<String> = selected
            .iter()
            .map(|n| topo.get(&n.id).unwrap().rack.clone())
            .collect();
        assert_eq!(racks.len(), 3);
    }

    #[test]
    fn test_select_best_nodes_zone_aware() {
        let scheduler = make_scheduler();
        scheduler.set_placement_strategy(PlacementStrategy::ZoneAware);

        for i in 0..6 {
            scheduler.register_topology(NodeTopology {
                node_id: format!("v{}", i),
                data_center: "dc1".to_string(),
                zone: format!("zone{}", i / 3),
                rack: format!("rack{}", i),
                network_latency_level: 1,
            });
        }

        let nodes: Vec<VolumeInfo> = (0..6)
            .map(|i| make_volume(&format!("v{}", i), 1000, i * 100, true))
            .collect();

        let selected = scheduler.select_best_nodes(&nodes, 2, &[]).unwrap();
        assert_eq!(selected.len(), 2);

        let topo = scheduler.topology.read();
        let zones: HashSet<String> = selected
            .iter()
            .map(|n| topo.get(&n.id).unwrap().zone.clone())
            .collect();
        assert_eq!(zones.len(), 2);
    }

    #[test]
    fn test_select_insufficient_nodes() {
        let scheduler = make_scheduler();
        let nodes: Vec<VolumeInfo> = (0..2)
            .map(|i| make_volume(&format!("v{}", i), 1000, 0, true))
            .collect();

        let result = scheduler.select_best_nodes(&nodes, 5, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_weights_config() {
        let scheduler = make_scheduler();
        let w = scheduler.get_weights();
        assert_eq!(w.capacity_weight, 50);
        assert_eq!(w.io_load_weight, 30);
        assert_eq!(w.network_weight, 20);

        scheduler.set_weights(SchedulerWeights {
            capacity_weight: 60,
            io_load_weight: 20,
            network_weight: 20,
        });
        let w2 = scheduler.get_weights();
        assert_eq!(w2.capacity_weight, 60);
    }

    #[test]
    fn test_rebalance_plan_empty() {
        let scheduler = make_scheduler();
        let plan = scheduler.generate_rebalance_plan(&[], 10);
        assert!(plan.migrations.is_empty());
        assert_eq!(plan.total_bytes, 0);
    }

    #[test]
    fn test_rebalance_plan_unbalanced() {
        let scheduler = make_scheduler();
        let mut nodes = Vec::new();
        let capacity = 100 * 1024 * 1024 * 1024; // 100 GB
        // 一个几乎满的节点 (90% used = 90GB)
        nodes.push(make_volume("v1", capacity, 90 * 1024 * 1024 * 1024, true));
        // 一个几乎空的节点 (10% used = 10GB)
        nodes.push(make_volume("v2", capacity, 10 * 1024 * 1024 * 1024, true));

        let plan = scheduler.generate_rebalance_plan(&nodes, 10);
        // 两个节点差异很大（80%差，阈值10%），应该有迁移计划
        assert!(plan.total_bytes > 0);
        assert!(!plan.migrations.is_empty());
    }

    #[test]
    fn test_rebalance_plan_balanced() {
        let scheduler = make_scheduler();
        let nodes: Vec<VolumeInfo> = (0..5)
            .map(|i| make_volume(&format!("v{}", i), 1000, 500, true))
            .collect();

        let plan = scheduler.generate_rebalance_plan(&nodes, 10);
        // 均衡的集群应该没有迁移
        assert!(plan.migrations.is_empty());
    }

    #[test]
    fn test_migration_lifecycle() {
        let scheduler = make_scheduler();

        let task = VolumeMigrationTask {
            task_id: "mig-test-1".to_string(),
            source_volume_id: "v1".to_string(),
            target_volume_id: "v2".to_string(),
            target_addr: "127.0.0.1:8002".to_string(),
            replica_set_id: "set-1".to_string(),
            size_bytes: 1024 * 1024,
            migrated_bytes: 0,
            status: MigrationStatus::Pending,
            created_at_ms: now_ms(),
            started_at_ms: None,
            completed_at_ms: None,
            error: None,
            bandwidth_limit_bps: 0,
        };

        scheduler.submit_migration(task);
        assert_eq!(scheduler.list_migrations().len(), 1);

        // 开始迁移
        scheduler
            .update_migration_status("mig-test-1", MigrationStatus::Running, 0, None)
            .unwrap();

        let tasks = scheduler.list_migrations();
        assert_eq!(tasks[0].status, MigrationStatus::Running);
        assert!(tasks[0].started_at_ms.is_some());

        // 完成迁移
        scheduler
            .update_migration_status(
                "mig-test-1",
                MigrationStatus::Completed,
                1024 * 1024,
                None,
            )
            .unwrap();

        let tasks = scheduler.list_migrations();
        assert_eq!(tasks[0].status, MigrationStatus::Completed);
        assert!(tasks[0].completed_at_ms.is_some());
        assert_eq!(tasks[0].migrated_bytes, 1024 * 1024);
    }

    #[test]
    fn test_migration_not_found() {
        let scheduler = make_scheduler();
        let result = scheduler.update_migration_status(
            "nonexistent",
            MigrationStatus::Running,
            0,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_max_concurrent_migrations() {
        let scheduler = make_scheduler();
        scheduler.set_max_concurrent_migrations(2);

        for i in 0..5 {
            scheduler.submit_migration(VolumeMigrationTask {
                task_id: format!("mig-{}", i),
                source_volume_id: format!("v{}", i),
                target_volume_id: format!("v{}", i + 10),
                target_addr: format!("127.0.0.1:{}", 9000 + i),
                replica_set_id: format!("set-{}", i),
                size_bytes: 1024,
                migrated_bytes: 0,
                status: MigrationStatus::Pending,
                created_at_ms: now_ms(),
                started_at_ms: None,
                completed_at_ms: None,
                error: None,
                bandwidth_limit_bps: 0,
            });
        }

        let pending = scheduler.get_pending_migrations(10);
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_detect_recovery() {
        let scheduler = make_scheduler();
        let nodes = vec![
            make_volume("v1", 1000, 500, true),
            make_volume("v2", 1000, 500, true),
            make_volume("v3", 1000, 500, false), // 不健康
        ];

        let mut replica_map = HashMap::new();
        replica_map.insert("set-1".to_string(), vec!["v1".to_string(), "v2".to_string(), "v3".to_string()]);

        let mut last_hb = HashMap::new();
        last_hb.insert("v1".to_string(), now_ms());
        last_hb.insert("v2".to_string(), now_ms());
        last_hb.insert("v3".to_string(), 0); // 超时

        let plan = scheduler.detect_and_plan_recovery(&nodes, &replica_map, &last_hb);
        assert!(plan.failed_nodes.len() >= 1);
        assert!(plan.affected_volumes >= 1);
    }

    #[test]
    fn test_placement_strategy_default() {
        let s = PlacementStrategy::default();
        assert_eq!(s, PlacementStrategy::RackAware);
    }

    #[test]
    fn test_data_temperature_default() {
        let t = DataTemperature::default();
        assert_eq!(t, DataTemperature::Hot);
    }

    #[test]
    fn test_migration_status_values() {
        let statuses = vec![
            MigrationStatus::Pending,
            MigrationStatus::Running,
            MigrationStatus::Completed,
            MigrationStatus::Failed,
            MigrationStatus::Cancelled,
            MigrationStatus::Paused,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let back: MigrationStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn test_scheduler_stats() {
        let scheduler = make_scheduler();
        let stats = scheduler.stats();
        let snap = stats.snapshot();
        assert!(snap.contains_key("scheduler_scheduling_total"));
        assert!(snap.contains_key("scheduler_migrations_completed"));
        assert!(snap.contains_key("scheduler_replicas_rebuilt"));
    }

    #[test]
    fn test_node_load_default() {
        let load = NodeLoad::default();
        assert_eq!(load.cpu_pct, 0);
        assert_eq!(load.iops, 0);
    }

    #[test]
    fn test_update_node_load() {
        let scheduler = make_scheduler();
        scheduler.update_node_load(
            "v1",
            NodeLoad {
                cpu_pct: 50,
                memory_pct: 60,
                iops: 1000,
                network_bps: 1000000,
                active_connections: 10,
            },
        );
        let loads = scheduler.node_loads.read();
        let load = loads.get("v1").unwrap();
        assert_eq!(load.cpu_pct, 50);
        assert_eq!(load.iops, 1000);
    }

    #[test]
    fn test_topology_registration() {
        let scheduler = make_scheduler();
        scheduler.register_topology(NodeTopology {
            node_id: "v1".to_string(),
            data_center: "dc1".to_string(),
            zone: "zone-a".to_string(),
            rack: "rack-1".to_string(),
            network_latency_level: 2,
        });
        let topo = scheduler.topology.read();
        let t = topo.get("v1").unwrap();
        assert_eq!(t.data_center, "dc1");
        assert_eq!(t.zone, "zone-a");
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 均衡控制器模块
//!
//! 作为数据均衡服务的核心协调者，负责：
//! - 监控集群容量/负载分布
//! - 生成均衡计划（决定哪些数据需要迁移、迁到哪里）
//! - 调度迁移任务执行（限速、并发控制）
//! - 迁移验证（CRC 校验、数据一致性）
//! - 迁移状态跟踪与进度报告
//!
//! 设计参考 SeaweedFS 的 Volume 均衡器和 Ceph 的 CRUSH 算法思想，
//! 支持千亿级文件规模的分布式数据均衡。

use crate::migration_task::{
    MigrationPhase, MigrationStatus, MigrationTask, MigrationTaskManager,
    MigrationType, VerificationMethod,
};
use crate::placement_strategy::{
    DataTemperature, PlacementConstraints, PlacementEngine, PlacementNode, PlacementStrategyType,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// 均衡控制器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceConfig {
    /// 是否启用自动均衡
    pub auto_rebalance_enabled: bool,
    /// 均衡阈值（使用率标准差百分比，超过则触发均衡）
    pub balance_threshold_pct: f64,
    /// 每次均衡最大迁移字节数
    pub max_migration_bytes_per_round: u64,
    /// 每次均衡最大迁移任务数
    pub max_migrations_per_round: usize,
    /// 最大并发迁移数
    pub max_concurrent_migrations: usize,
    /// 全局带宽限制（bytes/s）
    pub global_bandwidth_limit_bps: u64,
    /// 均衡间隔（秒）
    pub rebalance_interval_sec: u64,
    /// 低峰期起始小时（0-23）
    pub off_peak_start_hour: u8,
    /// 低峰期结束小时（0-23）
    pub off_peak_end_hour: u8,
    /// 低峰期带宽倍数（相对于正常带宽）
    pub off_peak_bandwidth_multiplier: f64,
    /// 是否启用迁移后校验
    pub verify_after_migration: bool,
    /// 校验方法
    pub verification_method: VerificationMethod,
    /// 放置策略
    pub placement_strategy: PlacementStrategyType,
}

impl Default for RebalanceConfig {
    fn default() -> Self {
        RebalanceConfig {
            auto_rebalance_enabled: true,
            balance_threshold_pct: 10.0,
            max_migration_bytes_per_round: 100 * 1024 * 1024 * 1024, // 100GB
            max_migrations_per_round: 50,
            max_concurrent_migrations: 4,
            global_bandwidth_limit_bps: 100 * 1024 * 1024, // 100MB/s
            rebalance_interval_sec: 300, // 5 分钟
            off_peak_start_hour: 2,
            off_peak_end_hour: 6,
            off_peak_bandwidth_multiplier: 2.0,
            verify_after_migration: true,
            verification_method: VerificationMethod::Crc32c,
            placement_strategy: PlacementStrategyType::Balanced,
        }
    }
}

/// 均衡计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancePlan {
    /// 计划 ID
    pub plan_id: String,
    /// 生成时间（ms）
    pub created_at_ms: u64,
    /// 计划中的迁移任务
    pub migrations: Vec<MigrationTask>,
    /// 计划总迁移量（字节）
    pub total_bytes: u64,
    /// 均衡前均衡度（0-100）
    pub balance_before: f64,
    /// 预计均衡后均衡度（0-100）
    pub estimated_balance_after: f64,
    /// 预计改善程度
    pub estimated_improvement: f64,
    /// 触发原因
    pub trigger_reason: String,
}

/// 均衡运行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebalanceState {
    /// 空闲
    Idle,
    /// 正在生成计划
    Planning,
    /// 正在执行迁移
    Running,
    /// 正在验证
    Verifying,
    /// 已暂停
    Paused,
    /// 已停止
    Stopped,
}

/// 均衡控制器
pub struct RebalanceController {
    /// 配置
    config: parking_lot::RwLock<RebalanceConfig>,
    /// 放置引擎
    placement_engine: PlacementEngine,
    /// 迁移任务管理器
    task_manager: MigrationTaskManager,
    /// 当前状态
    state: parking_lot::RwLock<RebalanceState>,
    /// 当前均衡计划
    current_plan: parking_lot::RwLock<Option<RebalancePlan>>,
    /// 统计信息
    stats: Arc<RebalanceStats>,
    /// 节点信息缓存
    nodes: parking_lot::RwLock<Vec<PlacementNode>>,
}

/// 均衡统计
#[derive(Debug, Default)]
pub struct RebalanceStats {
    /// 已完成的均衡轮次
    pub rebalance_rounds: parking_lot::Mutex<u64>,
    /// 生成的计划数
    pub plans_generated: parking_lot::Mutex<u64>,
    /// 总迁移字节数
    pub total_migrated_bytes: parking_lot::Mutex<u64>,
    /// 均衡改善总次数（每次均衡后更均衡的计数）
    pub improvements_total: parking_lot::Mutex<u64>,
    /// 失败的均衡轮次
    pub failed_rounds: parking_lot::Mutex<u64>,
    /// 当前集群均衡度（最近一次计算）
    pub current_balance_score: parking_lot::Mutex<f64>,
}

impl RebalanceStats {
    pub fn snapshot(&self) -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert(
            "rebalance_rounds_total".into(),
            *self.rebalance_rounds.lock(),
        );
        m.insert(
            "rebalance_plans_generated".into(),
            *self.plans_generated.lock(),
        );
        m.insert(
            "rebalance_total_migrated_bytes".into(),
            *self.total_migrated_bytes.lock(),
        );
        m.insert(
            "rebalance_improvements_total".into(),
            *self.improvements_total.lock(),
        );
        m.insert(
            "rebalance_failed_rounds".into(),
            *self.failed_rounds.lock(),
        );
        m.insert(
            "rebalance_current_score".into(),
            self.current_balance_score.lock().round() as u64,
        );
        m
    }
}

impl RebalanceController {
    /// 创建均衡控制器
    pub fn new(config: RebalanceConfig) -> Self {
        let placement_engine = PlacementEngine::with_strategy(config.placement_strategy);
        let task_manager = MigrationTaskManager::new(config.max_concurrent_migrations);
        task_manager.set_global_bandwidth_limit(config.global_bandwidth_limit_bps);

        Self {
            config: parking_lot::RwLock::new(config),
            placement_engine,
            task_manager,
            state: parking_lot::RwLock::new(RebalanceState::Idle),
            current_plan: parking_lot::RwLock::new(None),
            stats: Arc::new(RebalanceStats::default()),
            nodes: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// 获取配置
    pub fn get_config(&self) -> RebalanceConfig {
        self.config.read().clone()
    }

    /// 更新配置
    pub fn update_config(&self, config: RebalanceConfig) {
        self.placement_engine.set_strategy(config.placement_strategy);
        self.task_manager
            .set_max_concurrent(config.max_concurrent_migrations);
        self.task_manager
            .set_global_bandwidth_limit(config.global_bandwidth_limit_bps);
        *self.config.write() = config;
    }

    /// 获取当前状态
    pub fn get_state(&self) -> RebalanceState {
        *self.state.read()
    }

    /// 获取统计信息
    pub fn stats(&self) -> Arc<RebalanceStats> {
        self.stats.clone()
    }

    /// 获取迁移任务管理器引用
    pub fn task_manager(&self) -> &MigrationTaskManager {
        &self.task_manager
    }

    /// 获取放置引擎引用
    pub fn placement_engine(&self) -> &PlacementEngine {
        &self.placement_engine
    }

    // -----------------------------------------------------------------------
    // 节点管理
    // -----------------------------------------------------------------------

    /// 更新节点信息
    pub fn update_nodes(&self, nodes: Vec<PlacementNode>) {
        *self.nodes.write() = nodes;
    }

    /// 获取当前节点列表
    pub fn get_nodes(&self) -> Vec<PlacementNode> {
        self.nodes.read().clone()
    }

    // -----------------------------------------------------------------------
    // 均衡计划生成
    // -----------------------------------------------------------------------

    /// 检查是否需要均衡
    pub fn needs_rebalance(&self) -> bool {
        let config = self.config.read();
        if !config.auto_rebalance_enabled {
            return false;
        }

        let nodes = self.nodes.read();
        if nodes.is_empty() {
            return false;
        }

        let balance = self.placement_engine.compute_cluster_balance(&nodes);
        *self.stats.current_balance_score.lock() = balance;

        // 均衡度低于阈值则需要均衡
        // balance_threshold_pct 表示可接受的不均衡程度
        (100.0 - balance) > config.balance_threshold_pct
    }

    /// 生成均衡计划
    pub fn generate_plan(&self) -> Option<RebalancePlan> {
        let config = self.config.read();
        let nodes = self.nodes.read().clone();

        if nodes.len() < 2 {
            return None; // 至少需要 2 个节点才能均衡
        }

        let balance_before = self.placement_engine.compute_cluster_balance(&nodes);
        *self.stats.current_balance_score.lock() = balance_before;

        // 找出高负载节点（源）和低负载节点（目标）
        let avg_usage = self.compute_average_usage(&nodes);

        let mut sources: Vec<&PlacementNode> = Vec::new();
        let mut targets: Vec<&PlacementNode> = Vec::new();

        for node in &nodes {
            if !node.is_healthy {
                continue;
            }
            let usage = node.usage_pct();
            if usage > avg_usage + config.balance_threshold_pct {
                sources.push(node);
            } else if usage < avg_usage - config.balance_threshold_pct {
                targets.push(node);
            }
        }

        if sources.is_empty() || targets.is_empty() {
            return None;
        }

        // 按使用率降序排列源节点
        sources.sort_by(|a, b| {
            b.usage_pct()
                .partial_cmp(&a.usage_pct())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut migrations = Vec::new();
        let mut total_bytes = 0u64;
        let mut estimated_improvement_sum = 0.0f64;
        let mut plan_count = 0;

        // 为每个源节点生成迁移任务
        for src in &sources {
            if plan_count >= config.max_migrations_per_round {
                break;
            }
            if total_bytes >= config.max_migration_bytes_per_round {
                break;
            }

            // 计算需要迁出的字节数（使源节点降到平均值）
            let avg_bytes = (avg_usage / 100.0 * src.capacity as f64) as u64;
            let bytes_to_move = src.used.saturating_sub(avg_bytes);

            if bytes_to_move < 1024 * 1024 {
                continue; // 小于 1MB 不迁移
            }

            // 选择目标节点
            let constraints = PlacementConstraints {
                excluded_nodes: HashSet::from([src.node_id.clone()]),
                excluded_racks: HashSet::from([src.rack.clone()]),
                excluded_zones: HashSet::new(),
                min_free_bytes: bytes_to_move,
                max_cpu_pct: 90,
                preferred_temperature: Some(src.preferred_temperature),
                same_data_center: Some(src.data_center.clone()),
            };

            let candidate_list: Vec<PlacementNode> = targets
                .iter()
                .filter(|t| t.free_bytes() >= bytes_to_move)
                .map(|t| (*t).clone())
                .collect();

            if let Some(target) = self.placement_engine.select_target(&candidate_list, &constraints) {
                let actual_bytes = bytes_to_move.min(target.node.free_bytes());

                if actual_bytes == 0 {
                    continue;
                }

                // 预测改善
                let improvement = self.placement_engine.predict_balance_improvement(
                    &nodes,
                    &src.node_id,
                    &target.node.node_id,
                    actual_bytes,
                );

                let task_id = generate_rebalance_task_id();
                let task = MigrationTask {
                    task_id,
                    migration_type: MigrationType::CapacityRebalance,
                    source_node_id: src.node_id.clone(),
                    source_addr: src.addr.clone(),
                    target_node_id: target.node.node_id.clone(),
                    target_addr: target.node.addr.clone(),
                    object_id: format!("rebalance-{}-{}", src.node_id, target.node.node_id),
                    total_bytes: actual_bytes,
                    migrated_bytes: 0,
                    verified_bytes: 0,
                    priority: 4,
                    status: MigrationStatus::Pending,
                    phase: MigrationPhase::Init,
                    created_at_ms: now_ms(),
                    started_at_ms: None,
                    completed_at_ms: None,
                    bandwidth_limit_bps: 0,
                    retry_count: 0,
                    max_retries: 3,
                    last_error: None,
                    checkpoint: None,
                    delete_source_after: true,
                    verify_after_migration: config.verify_after_migration,
                    verification_method: config.verification_method,
                };

                migrations.push(task);
                total_bytes += actual_bytes;
                estimated_improvement_sum += improvement;
                plan_count += 1;
            }
        }

        if migrations.is_empty() {
            return None;
        }

        let estimated_balance_after = balance_before + estimated_improvement_sum;

        let plan = RebalancePlan {
            plan_id: generate_plan_id(),
            created_at_ms: now_ms(),
            migrations,
            total_bytes,
            balance_before,
            estimated_balance_after: estimated_balance_after.min(100.0),
            estimated_improvement: estimated_improvement_sum,
            trigger_reason: format!(
                "balance {} < threshold {}%",
                balance_before as u8,
                config.balance_threshold_pct as u8
            ),
        };

        *self.stats.plans_generated.lock() += 1;

        Some(plan)
    }

    /// 计算平均使用率
    fn compute_average_usage(&self, nodes: &[PlacementNode]) -> f64 {
        let healthy_nodes: Vec<&PlacementNode> =
            nodes.iter().filter(|n| n.is_healthy).collect();

        if healthy_nodes.is_empty() {
            return 0.0;
        }

        let total_capacity: u64 = healthy_nodes.iter().map(|n| n.capacity).sum();
        let total_used: u64 = healthy_nodes.iter().map(|n| n.used).sum();

        if total_capacity == 0 {
            return 0.0;
        }

        total_used as f64 / total_capacity as f64 * 100.0
    }

    // -----------------------------------------------------------------------
    // 均衡执行
    // -----------------------------------------------------------------------

    /// 执行一次均衡
    pub fn run_rebalance(&self) -> MasterRebalanceResult {
        let config = self.config.read().clone();
        drop(config);

        // 生成计划
        *self.state.write() = RebalanceState::Planning;
        let plan = match self.generate_plan() {
            Some(p) => p,
            None => {
                *self.state.write() = RebalanceState::Idle;
                return MasterRebalanceResult {
                    plan_id: None,
                    migrations_scheduled: 0,
                    total_bytes: 0,
                    balance_before: self.placement_engine.compute_cluster_balance(&self.nodes.read()),
                    balance_after: 0.0,
                };
            }
        };

        let plan_id = plan.plan_id.clone();
        let total_bytes = plan.total_bytes;
        let count = plan.migrations.len();
        let balance_before = plan.balance_before;

        *self.current_plan.write() = Some(plan.clone());

        // 提交所有迁移任务
        *self.state.write() = RebalanceState::Running;
        for task in plan.migrations {
            self.task_manager.submit_task(task);
        }

        *self.stats.rebalance_rounds.lock() += 1;

        // 注意：这里只是提交任务，实际执行由外部驱动（如 tick 循环）
        // 真实场景中，controller 启动后台协程执行迁移

        MasterRebalanceResult {
            plan_id: Some(plan_id),
            migrations_scheduled: count as u32,
            total_bytes,
            balance_before,
            balance_after: 0.0, // 实际均衡度需在完成后计算
        }
    }

    /// 驱动迁移执行（应周期性调用）
    /// 返回本轮处理的任务数
    pub fn tick(&self) -> usize {
        let mut processed = 0;

        // 启动新任务
        while let Some(task) = self.task_manager.get_next_task() {
            // 实际项目中这里会启动真实的迁移流程
            // 简化实现：直接标记完成（模拟）
            self.task_manager
                .report_progress(&task.task_id, task.total_bytes, MigrationPhase::Verification);

            let verified = if task.verify_after_migration {
                task.total_bytes
            } else {
                0
            };
            self.task_manager.complete_task(&task.task_id, verified);

            *self.stats.total_migrated_bytes.lock() += task.total_bytes;
            *self.stats.improvements_total.lock() += 1;
            processed += 1;
        }

        // 检查是否所有任务都完成了
        let pending = self.task_manager.list_pending().len();
        let running = self.task_manager.list_running().len();
        if pending == 0 && running == 0 && *self.state.read() == RebalanceState::Running {
            *self.state.write() = RebalanceState::Idle;

            // 更新当前均衡度
            let nodes = self.nodes.read();
            let balance = self.placement_engine.compute_cluster_balance(&nodes);
            *self.stats.current_balance_score.lock() = balance;
        }

        processed
    }

    /// 暂停均衡
    pub fn pause(&self) {
        let state = *self.state.read();
        if state == RebalanceState::Running {
            *self.state.write() = RebalanceState::Paused;
            // 暂停所有运行中的任务
            for task in self.task_manager.list_running() {
                self.task_manager.pause_task(&task.task_id);
            }
        }
    }

    /// 恢复均衡
    pub fn resume(&self) {
        if *self.state.read() == RebalanceState::Paused {
            *self.state.write() = RebalanceState::Running;
            // 恢复所有暂停的任务
            for task in self.task_manager.list_running() {
                if task.status == MigrationStatus::Paused {
                    self.task_manager.resume_task(&task.task_id);
                }
            }
        }
    }

    /// 停止均衡（取消所有待执行任务）
    pub fn stop(&self) {
        // 取消所有 pending 任务
        for task in self.task_manager.list_pending() {
            self.task_manager.cancel_task(&task.task_id);
        }
        // 取消 running 任务
        for task in self.task_manager.list_running() {
            self.task_manager.cancel_task(&task.task_id);
        }
        *self.state.write() = RebalanceState::Stopped;
        *self.current_plan.write() = None;
    }

    // -----------------------------------------------------------------------
    // 故障恢复均衡
    // -----------------------------------------------------------------------

    /// 为故障节点生成恢复计划
    pub fn generate_recovery_plan(
        &self,
        failed_node_ids: &[String],
        replica_map: &HashMap<String, Vec<String>>, // set_id -> [node_id]
    ) -> Vec<MigrationTask> {
        let nodes = self.nodes.read();
        let mut tasks = Vec::new();

        let failed_set: HashSet<&str> = failed_node_ids.iter().map(|s| s.as_str()).collect();

        // 找出受影响的副本集
        for (set_id, replicas) in replica_map {
            let lost_count = replicas.iter().filter(|r| failed_set.contains(r.as_str())).count();
            if lost_count == 0 {
                continue;
            }

            // 为每个丢失的副本找重建目标
            for _lost_node in replicas.iter().filter(|r| failed_set.contains(r.as_str())) {
                // 选择目标节点（排除已有副本的节点）
                let constraints = PlacementConstraints {
                    excluded_nodes: replicas.iter().cloned().collect(),
                    excluded_racks: HashSet::new(),
                    excluded_zones: HashSet::new(),
                    min_free_bytes: 0, // 待填充
                    max_cpu_pct: 80,
                    preferred_temperature: Some(DataTemperature::Hot),
                    same_data_center: None,
                };

                let candidate_list: Vec<PlacementNode> = nodes
                    .iter()
                    .filter(|n| n.is_healthy && !replicas.contains(&n.node_id))
                    .cloned()
                    .collect();

                if let Some(target) =
                    self.placement_engine.select_target(&candidate_list, &constraints)
                {
                    let task = MigrationTask {
                        task_id: generate_recovery_task_id(),
                        migration_type: MigrationType::FailureRecovery,
                        source_node_id: String::new(), // 恢复任务源是其他副本
                        source_addr: String::new(),
                        target_node_id: target.node.node_id.clone(),
                        target_addr: target.node.addr.clone(),
                        object_id: set_id.clone(),
                        total_bytes: 0, // 需要从现有副本获取大小
                        migrated_bytes: 0,
                        verified_bytes: 0,
                        priority: MigrationType::FailureRecovery.default_priority(),
                        status: MigrationStatus::Pending,
                        phase: MigrationPhase::Init,
                        created_at_ms: now_ms(),
                        started_at_ms: None,
                        completed_at_ms: None,
                        bandwidth_limit_bps: 0,
                        retry_count: 0,
                        max_retries: 5,
                        last_error: None,
                        checkpoint: None,
                        delete_source_after: false,
                        verify_after_migration: true,
                        verification_method: VerificationMethod::Sha256,
                    };
                    tasks.push(task);
                }
            }
        }

        // 按优先级排序（高优先级在前）
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));

        tasks
    }

    // -----------------------------------------------------------------------
    // 进度查询
    // -----------------------------------------------------------------------

    /// 获取当前均衡进度（0-100）
    pub fn get_progress_pct(&self) -> f64 {
        let plan = match self.current_plan.read().as_ref() {
            Some(p) => p.clone(),
            None => return 0.0,
        };

        if plan.total_bytes == 0 {
            return 100.0;
        }

        let mut completed_bytes = 0u64;

        // 统计已完成任务的字节数
        for task in self.task_manager.list_completed(1000) {
            if task.migration_type == MigrationType::CapacityRebalance
                && task.status == MigrationStatus::Completed
            {
                completed_bytes += task.total_bytes;
            }
        }

        // 统计运行中任务的已迁移字节
        for task in self.task_manager.list_running() {
            completed_bytes += task.migrated_bytes;
        }

        (completed_bytes as f64 / plan.total_bytes as f64 * 100.0).min(100.0)
    }

    /// 获取当前均衡计划
    pub fn get_current_plan(&self) -> Option<RebalancePlan> {
        self.current_plan.read().as_ref().cloned()
    }

    // -----------------------------------------------------------------------
    // 带宽管理
    // -----------------------------------------------------------------------

    /// 检查当前是否在低峰期
    pub fn is_off_peak(&self) -> bool {
        let config = self.config.read();
        let now = now_ms();
        let hour = (now / 1000 / 60 / 60) % 24;

        if config.off_peak_start_hour <= config.off_peak_end_hour {
            hour >= config.off_peak_start_hour as u64
                && hour < config.off_peak_end_hour as u64
        } else {
            hour >= config.off_peak_start_hour as u64
                || hour < config.off_peak_end_hour as u64
        }
    }

    /// 根据时间段调整带宽
    pub fn adjust_bandwidth_for_time(&self) {
        let config = self.config.read();
        let base_limit = config.global_bandwidth_limit_bps;

        let effective_limit = if self.is_off_peak() {
            (base_limit as f64 * config.off_peak_bandwidth_multiplier) as u64
        } else {
            base_limit
        };

        self.task_manager.set_global_bandwidth_limit(effective_limit);
    }

    /// 获取有效带宽限制（考虑低峰期）
    pub fn effective_bandwidth_bps(&self) -> u64 {
        self.task_manager.get_global_bandwidth_limit()
    }

    // -----------------------------------------------------------------------
    // 综合指标
    // -----------------------------------------------------------------------

    /// 获取完整的均衡状态摘要
    pub fn get_status_summary(&self) -> RebalanceStatusSummary {
        let nodes = self.nodes.read();
        let balance = self.placement_engine.compute_cluster_balance(&nodes);

        RebalanceStatusSummary {
            state: *self.state.read(),
            balance_score: balance,
            pending_migrations: self.task_manager.list_pending().len() as u32,
            running_migrations: self.task_manager.list_running().len() as u32,
            total_migrated_bytes: *self.stats.total_migrated_bytes.lock(),
            current_plan_id: self.current_plan.read().as_ref().map(|p| p.plan_id.clone()),
            progress_pct: self.get_progress_pct(),
            effective_bandwidth_bps: self.effective_bandwidth_bps(),
            is_off_peak: self.is_off_peak(),
            node_count: nodes.len() as u32,
            healthy_node_count: nodes.iter().filter(|n| n.is_healthy).count() as u32,
        }
    }
}

impl Default for RebalanceController {
    fn default() -> Self {
        Self::new(RebalanceConfig::default())
    }
}

/// 均衡执行结果
#[derive(Debug, Clone)]
pub struct MasterRebalanceResult {
    pub plan_id: Option<String>,
    pub migrations_scheduled: u32,
    pub total_bytes: u64,
    pub balance_before: f64,
    pub balance_after: f64,
}

/// 均衡状态摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceStatusSummary {
    /// 当前状态
    pub state: RebalanceState,
    /// 当前均衡度（0-100）
    pub balance_score: f64,
    /// 待执行迁移数
    pub pending_migrations: u32,
    /// 运行中迁移数
    pub running_migrations: u32,
    /// 累计迁移字节数
    pub total_migrated_bytes: u64,
    /// 当前计划 ID
    pub current_plan_id: Option<String>,
    /// 当前进度（0-100）
    pub progress_pct: f64,
    /// 有效带宽（bytes/s）
    pub effective_bandwidth_bps: u64,
    /// 是否在低峰期
    pub is_off_peak: bool,
    /// 总节点数
    pub node_count: u32,
    /// 健康节点数
    pub healthy_node_count: u32,
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

fn generate_rebalance_task_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!(
        "rebal-{:08x}{:08x}",
        rng.gen::<u32>(),
        rng.gen::<u32>()
    )
}

fn generate_recovery_task_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!(
        "recov-{:08x}{:08x}",
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
            cpu_pct: 30,
            network_latency: 2,
            active_migrations: 0,
            preferred_temperature: DataTemperature::Hot,
        }
    }

    fn make_controller() -> RebalanceController {
        let config = RebalanceConfig {
            balance_threshold_pct: 5.0,
            max_migrations_per_round: 10,
            ..RebalanceConfig::default()
        };
        RebalanceController::new(config)
    }

    #[test]
    fn test_controller_default_state() {
        let ctrl = make_controller();
        assert_eq!(ctrl.get_state(), RebalanceState::Idle);
        assert!(ctrl.get_current_plan().is_none());
    }

    #[test]
    fn test_needs_rebalance_balanced() {
        let ctrl = make_controller();
        let nodes = vec![
            make_node("n1", 1000, 500, "r1", "z1"),
            make_node("n2", 1000, 500, "r2", "z1"),
            make_node("n3", 1000, 500, "r3", "z2"),
        ];
        ctrl.update_nodes(nodes);

        assert!(!ctrl.needs_rebalance());
    }

    #[test]
    fn test_needs_rebalance_unbalanced() {
        let ctrl = make_controller();
        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"), // 90%
            make_node("n2", 1000, 100, "r2", "z1"), // 10%
        ];
        ctrl.update_nodes(nodes);

        assert!(ctrl.needs_rebalance());
    }

    #[test]
    fn test_generate_plan() {
        let ctrl = make_controller();
        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"),
            make_node("n2", 1000, 100, "r2", "z1"),
        ];
        ctrl.update_nodes(nodes);

        let plan = ctrl.generate_plan();
        assert!(plan.is_some());

        let plan = plan.unwrap();
        assert!(!plan.migrations.is_empty());
        assert!(plan.total_bytes > 0);
        assert!(plan.estimated_improvement > 0.0);
        assert!(plan.balance_before < 100.0);
    }

    #[test]
    fn test_generate_plan_single_node() {
        let ctrl = make_controller();
        let nodes = vec![make_node("n1", 1000, 500, "r1", "z1")];
        ctrl.update_nodes(nodes);

        let plan = ctrl.generate_plan();
        assert!(plan.is_none()); // 单节点无法均衡
    }

    #[test]
    fn test_run_rebalance() {
        let ctrl = make_controller();
        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"),
            make_node("n2", 1000, 100, "r2", "z1"),
        ];
        ctrl.update_nodes(nodes);

        let result = ctrl.run_rebalance();
        assert!(result.plan_id.is_some());
        assert!(result.migrations_scheduled > 0);
        assert!(result.total_bytes > 0);
        assert_eq!(ctrl.get_state(), RebalanceState::Running);
    }

    #[test]
    fn test_tick_processes_tasks() {
        let ctrl = make_controller();
        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"),
            make_node("n2", 1000, 100, "r2", "z1"),
        ];
        ctrl.update_nodes(nodes);

        ctrl.run_rebalance();
        assert_eq!(ctrl.get_state(), RebalanceState::Running);

        // tick 驱动执行
        let processed = ctrl.tick();
        assert!(processed > 0);

        // 任务完成后应该回到 Idle
        assert_eq!(ctrl.get_state(), RebalanceState::Idle);
    }

    #[test]
    fn test_pause_and_resume() {
        let ctrl = make_controller();
        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"),
            make_node("n2", 1000, 100, "r2", "z1"),
        ];
        ctrl.update_nodes(nodes);

        ctrl.run_rebalance();
        assert_eq!(ctrl.get_state(), RebalanceState::Running);

        ctrl.pause();
        assert_eq!(ctrl.get_state(), RebalanceState::Paused);

        ctrl.resume();
        assert_eq!(ctrl.get_state(), RebalanceState::Running);
    }

    #[test]
    fn test_stop() {
        let ctrl = make_controller();
        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"),
            make_node("n2", 1000, 100, "r2", "z1"),
        ];
        ctrl.update_nodes(nodes);

        ctrl.run_rebalance();
        ctrl.stop();

        assert_eq!(ctrl.get_state(), RebalanceState::Stopped);
        assert!(ctrl.get_current_plan().is_none());
        assert_eq!(ctrl.task_manager().list_pending().len(), 0);
    }

    #[test]
    fn test_generate_recovery_plan() {
        let ctrl = make_controller();
        let nodes = vec![
            make_node("n1", 1000, 500, "r1", "z1"),
            make_node("n2", 1000, 500, "r2", "z1"),
            make_node("n3", 1000, 500, "r3", "z2"),
        ];
        ctrl.update_nodes(nodes);

        let mut replica_map = HashMap::new();
        replica_map.insert(
            "set-1".to_string(),
            vec!["n1".to_string(), "n2".to_string(), "n3".to_string()],
        );

        let tasks = ctrl.generate_recovery_plan(&["n1".to_string()], &replica_map);
        // 三副本丢一个，应该能恢复
        assert!(!tasks.is_empty());
        assert_eq!(tasks[0].migration_type, MigrationType::FailureRecovery);
        assert_eq!(tasks[0].priority, 10);
    }

    #[test]
    fn test_get_status_summary() {
        let ctrl = make_controller();
        let nodes = vec![
            make_node("n1", 1000, 500, "r1", "z1"),
            make_node("n2", 1000, 500, "r2", "z1"),
        ];
        ctrl.update_nodes(nodes);

        let summary = ctrl.get_status_summary();
        assert_eq!(summary.state, RebalanceState::Idle);
        assert_eq!(summary.node_count, 2);
        assert_eq!(summary.healthy_node_count, 2);
        assert!(summary.balance_score > 90.0);
    }

    #[test]
    fn test_update_config() {
        let ctrl = make_controller();
        let mut config = ctrl.get_config();
        config.max_concurrent_migrations = 10;
        config.global_bandwidth_limit_bps = 200 * 1024 * 1024;
        config.placement_strategy = PlacementStrategyType::CapacityFirst;

        ctrl.update_config(config);

        let new_config = ctrl.get_config();
        assert_eq!(new_config.max_concurrent_migrations, 10);
        assert_eq!(
            ctrl.task_manager().get_max_concurrent(),
            10
        );
    }

    #[test]
    fn test_stats_snapshot() {
        let ctrl = make_controller();
        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"),
            make_node("n2", 1000, 100, "r2", "z1"),
        ];
        ctrl.update_nodes(nodes);

        ctrl.run_rebalance();
        ctrl.tick();

        let snap = ctrl.stats().snapshot();
        assert!(snap["rebalance_rounds_total"] >= 1);
        assert!(snap["rebalance_plans_generated"] >= 1);
        assert!(snap["rebalance_total_migrated_bytes"] > 0);
        assert!(snap["rebalance_improvements_total"] >= 1);
    }

    #[test]
    fn test_progress_pct() {
        let ctrl = make_controller();
        // 没有计划时进度为 0
        assert_eq!(ctrl.get_progress_pct(), 0.0);
    }

    #[test]
    fn test_rebalance_config_default() {
        let config = RebalanceConfig::default();
        assert!(config.auto_rebalance_enabled);
        assert_eq!(config.balance_threshold_pct, 10.0);
        assert_eq!(config.max_concurrent_migrations, 4);
        assert_eq!(config.off_peak_start_hour, 2);
        assert_eq!(config.off_peak_end_hour, 6);
    }

    #[test]
    fn test_adjust_bandwidth() {
        let ctrl = make_controller();
        // 这个测试验证函数不崩溃即可
        // 实际是否为低峰期取决于当前时间
        ctrl.adjust_bandwidth_for_time();
        assert!(ctrl.effective_bandwidth_bps() > 0);
    }

    #[test]
    fn test_disabled_auto_rebalance() {
        let ctrl = make_controller();
        let mut config = ctrl.get_config();
        config.auto_rebalance_enabled = false;
        ctrl.update_config(config);

        let nodes = vec![
            make_node("n1", 1000, 900, "r1", "z1"),
            make_node("n2", 1000, 100, "r2", "z1"),
        ];
        ctrl.update_nodes(nodes);

        assert!(!ctrl.needs_rebalance());
    }

    #[test]
    fn test_get_nodes() {
        let ctrl = make_controller();
        let nodes = vec![make_node("n1", 1000, 500, "r1", "z1")];
        ctrl.update_nodes(nodes.clone());

        let got = ctrl.get_nodes();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].node_id, "n1");
    }
}

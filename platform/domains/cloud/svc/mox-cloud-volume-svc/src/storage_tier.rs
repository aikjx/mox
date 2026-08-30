// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 分层存储引擎
//!
//! 实现热/温/冷三层存储架构，参考 JuiceFS 的分层存储设计：
//! - 热层（Hot）：本地 SSD/NVMe，低延迟，高 IOPS
//! - 温层（Warm）：本地 HDD 或网络存储，大容量，中等性能
//! - 冷层（Cold）：对象存储/归档，低成本，低性能
//!
//! 支持自动分层策略，基于访问频率、时间、大小等维度，
//! 配合带宽限制和业务低峰期调度，实现数据在层间的平滑迁移。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// 存储层类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum StorageLayer {
    /// 热层：本地 SSD/NVMe，低延迟高 IOPS
    Hot = 0,
    /// 温层：本地 HDD 或网络存储，大容量
    Warm = 1,
    /// 冷层：对象存储/归档，低成本
    Cold = 2,
}

impl StorageLayer {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "hot" => Some(StorageLayer::Hot),
            "warm" => Some(StorageLayer::Warm),
            "cold" => Some(StorageLayer::Cold),
            _ => None,
        }
    }

    /// 层级名称
    pub fn name(&self) -> &'static str {
        match self {
            StorageLayer::Hot => "hot",
            StorageLayer::Warm => "warm",
            StorageLayer::Cold => "cold",
        }
    }

    /// 是否比另一个层更"热"（数值更小）
    pub fn is_hotter_than(&self, other: StorageLayer) -> bool {
        (*self as u8) < (other as u8)
    }

    /// 是否比另一个层更"冷"（数值更大）
    pub fn is_colder_than(&self, other: StorageLayer) -> bool {
        (*self as u8) > (other as u8)
    }
}

impl Default for StorageLayer {
    fn default() -> Self {
        StorageLayer::Hot
    }
}

impl std::fmt::Display for StorageLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// 存储层配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageLayerConfig {
    /// 层类型
    pub layer: StorageLayer,
    /// 层名称/描述
    pub name: String,
    /// 总容量（字节）
    pub total_capacity: u64,
    /// 高水位线（使用率百分比，超过则触发迁出）
    pub high_watermark_pct: u8,
    /// 低水位线（使用率百分比，迁出到低于此值）
    pub low_watermark_pct: u8,
    /// 最大 IOPS（0 表示不限制）
    pub max_iops: u64,
    /// 最大带宽（bytes/s，0 表示不限制）
    pub max_bandwidth_bps: u64,
    /// 平均读取延迟（微秒）
    pub avg_read_latency_us: u32,
    /// 平均写入延迟（微秒）
    pub avg_write_latency_us: u32,
    /// 每 GB 成本（元/月，用于成本优化计算）
    pub cost_per_gb_per_month: f64,
    /// 后端路径/端点
    pub backend_path: String,
}

impl StorageLayerConfig {
    /// 获取高水位对应的字节数
    pub fn high_watermark_bytes(&self) -> u64 {
        self.total_capacity * self.high_watermark_pct as u64 / 100
    }

    /// 获取低水位对应的字节数
    pub fn low_watermark_bytes(&self) -> u64 {
        self.total_capacity * self.low_watermark_pct as u64 / 100
    }
}

/// 对象访问统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectAccessStats {
    /// 对象 ID
    pub object_id: String,
    /// 对象大小（字节）
    pub size_bytes: u64,
    /// 当前所在层
    pub current_layer: StorageLayer,
    /// 创建时间（ms）
    pub created_at_ms: u64,
    /// 最后访问时间（ms）
    pub last_access_ms: u64,
    /// 最后修改时间（ms）
    pub last_modified_ms: u64,
    /// 总访问次数
    pub access_count: u64,
    /// 最近 24 小时访问次数
    pub access_count_24h: u64,
    /// 最近 7 天访问次数
    pub access_count_7d: u64,
    /// 最近 30 天访问次数
    pub access_count_30d: u64,
    /// 读取字节总数
    pub read_bytes_total: u64,
    /// 写入字节总数
    pub write_bytes_total: u64,
}

impl ObjectAccessStats {
    /// 创建新的访问统计
    pub fn new(object_id: String, size_bytes: u64, layer: StorageLayer) -> Self {
        let now = now_ms();
        Self {
            object_id,
            size_bytes,
            current_layer: layer,
            created_at_ms: now,
            last_access_ms: now,
            last_modified_ms: now,
            access_count: 0,
            access_count_24h: 0,
            access_count_7d: 0,
            access_count_30d: 0,
            read_bytes_total: 0,
            write_bytes_total: 0,
        }
    }

    /// 记录一次读访问
    pub fn record_read(&mut self, bytes: u64) {
        let now = now_ms();
        self.last_access_ms = now;
        self.access_count += 1;
        self.access_count_24h += 1;
        self.access_count_7d += 1;
        self.access_count_30d += 1;
        self.read_bytes_total += bytes;
    }

    /// 记录一次写访问
    pub fn record_write(&mut self, bytes: u64) {
        let now = now_ms();
        self.last_access_ms = now;
        self.last_modified_ms = now;
        self.access_count += 1;
        self.access_count_24h += 1;
        self.access_count_7d += 1;
        self.access_count_30d += 1;
        self.write_bytes_total += bytes;
    }

    /// 计算"热度"分数（0-100，越高越热）
    pub fn heat_score(&self) -> f64 {
        let now = now_ms();

        // 基于最近访问时间的分数：最近访问过的分数高
        let days_since_access = (now.saturating_sub(self.last_access_ms) as f64)
            / (1000.0 * 60.0 * 60.0 * 24.0);
        let recency_score = if days_since_access <= 0.0 {
            100.0
        } else {
            (100.0 / (1.0 + days_since_access.ln())).max(0.0)
        };

        // 基于访问频率的分数
        let freq_score = if self.size_bytes > 0 {
            // 标准化：每 MB 每天访问次数
            let mb = (self.size_bytes as f64 / (1024.0 * 1024.0)).max(1.0);
            let access_per_mb_per_day = self.access_count_7d as f64 / 7.0 / mb;
            (access_per_mb_per_day * 10.0).min(100.0)
        } else {
            0.0
        };

        // 基于最近 24h 活跃度的分数（权重更高）
        let recent_active_score = if self.access_count_24h > 0 {
            (self.access_count_24h as f64 * 5.0).min(100.0)
        } else {
            0.0
        };

        // 加权求和
        let score = recency_score * 0.4 + freq_score * 0.3 + recent_active_score * 0.3;
        score.max(0.0).min(100.0)
    }
}

/// 分层策略类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TieringPolicyType {
    /// 基于访问频率
    AccessFrequency,
    /// 基于年龄（创建时间）
    AgeBased,
    /// 基于大小
    SizeBased,
    /// 综合策略（频率+年龄+大小）
    Combined,
    /// 用户指定（手动分层）
    Manual,
}

impl Default for TieringPolicyType {
    fn default() -> Self {
        TieringPolicyType::Combined
    }
}

/// 分层策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieringPolicyConfig {
    /// 策略类型
    pub policy_type: TieringPolicyType,
    /// 热层 -> 温层阈值（天）
    pub hot_to_warm_days: u32,
    /// 温层 -> 冷层阈值（天）
    pub warm_to_cold_days: u32,
    /// 热度阈值：低于此值从热层降级到温层（0-100）
    pub heat_threshold_hot_to_warm: u8,
    /// 热度阈值：低于此值从温层降级到冷层（0-100）
    pub heat_threshold_warm_to_cold: u8,
    /// 热度阈值：高于此值从温层升级到热层（0-100）
    pub heat_threshold_warm_to_hot: u8,
    /// 大对象阈值（字节），超过此大小的对象直接进入温层
    pub large_object_threshold: u64,
    /// 小对象阈值（字节），小于此大小的对象保持在热层
    pub small_object_threshold: u64,
    /// 是否在读取时自动升级到热层
    pub promote_on_read: bool,
    /// 读取时升级的最小访问次数
    pub promote_min_access_count: u32,
}

impl Default for TieringPolicyConfig {
    fn default() -> Self {
        TieringPolicyConfig {
            policy_type: TieringPolicyType::Combined,
            hot_to_warm_days: 30,
            warm_to_cold_days: 90,
            heat_threshold_hot_to_warm: 30,
            heat_threshold_warm_to_cold: 10,
            heat_threshold_warm_to_hot: 60,
            large_object_threshold: 100 * 1024 * 1024, // 100MB
            small_object_threshold: 64 * 1024,          // 64KB
            promote_on_read: true,
            promote_min_access_count: 3,
        }
    }
}

/// 分层迁移任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierMigrationTask {
    /// 任务 ID
    pub task_id: String,
    /// 对象 ID
    pub object_id: String,
    /// 源层
    pub source_layer: StorageLayer,
    /// 目标层
    pub target_layer: StorageLayer,
    /// 对象大小
    pub size_bytes: u64,
    /// 已迁移字节
    pub migrated_bytes: u64,
    /// 迁移状态
    pub status: MigrationStatus,
    /// 创建时间
    pub created_at_ms: u64,
    /// 开始时间
    pub started_at_ms: Option<u64>,
    /// 完成时间
    pub completed_at_ms: Option<u64>,
    /// 优先级（越高越先执行）
    pub priority: u8,
    /// 失败原因
    pub error: Option<String>,
}

/// 迁移状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 迁移调度窗口（低峰期配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationScheduleWindow {
    /// 窗口开始时间（小时，0-23）
    pub start_hour: u8,
    /// 窗口结束时间（小时，0-23）
    pub end_hour: u8,
    /// 窗口内最大带宽（bytes/s）
    pub max_bandwidth_bps: u64,
    /// 是否只在工作日执行
    pub weekdays_only: bool,
}

impl Default for MigrationScheduleWindow {
    fn default() -> Self {
        MigrationScheduleWindow {
            start_hour: 2,    // 凌晨 2 点
            end_hour: 6,      // 到早上 6 点
            max_bandwidth_bps: 100 * 1024 * 1024, // 100MB/s
            weekdays_only: false,
        }
    }
}

/// 分层存储引擎
///
/// 管理热/温/冷三层存储，提供自动分层、数据迁移调度、
/// 访问统计追踪等功能。
pub struct StorageTierEngine {
    /// 各层配置
    layer_configs: parking_lot::RwLock<HashMap<StorageLayer, StorageLayerConfig>>,
    /// 各层已使用容量
    layer_used: parking_lot::Mutex<HashMap<StorageLayer, u64>>,
    /// 分层策略配置
    policy: parking_lot::RwLock<TieringPolicyConfig>,
    /// 对象访问统计
    access_stats: parking_lot::Mutex<HashMap<String, ObjectAccessStats>>,
    /// 迁移任务队列
    migration_tasks: parking_lot::Mutex<VecDeque<TierMigrationTask>>,
    /// 迁移调度窗口
    schedule_window: parking_lot::RwLock<MigrationScheduleWindow>,
    /// 最大并发迁移数（预留配置，用于未来并发控制）
    #[allow(dead_code)]
    max_concurrent_migrations: parking_lot::Mutex<usize>,
    /// 统计信息
    stats: Arc<TierStats>,
}

/// 分层存储统计
#[derive(Debug, Default)]
pub struct TierStats {
    /// 热层对象数
    pub hot_objects: parking_lot::Mutex<u64>,
    /// 温层对象数
    pub warm_objects: parking_lot::Mutex<u64>,
    /// 冷层对象数
    pub cold_objects: parking_lot::Mutex<u64>,
    /// 已完成的降级迁移数（热->温, 温->冷）
    pub tier_down_total: parking_lot::Mutex<u64>,
    /// 已完成的升级迁移数（冷->温, 温->热）
    pub tier_up_total: parking_lot::Mutex<u64>,
    /// 迁移字节总数
    pub migration_bytes_total: parking_lot::Mutex<u64>,
    /// 失败迁移数
    pub migrations_failed: parking_lot::Mutex<u64>,
}

impl TierStats {
    pub fn snapshot(&self) -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert("tier_hot_objects".into(), *self.hot_objects.lock());
        m.insert("tier_warm_objects".into(), *self.warm_objects.lock());
        m.insert("tier_cold_objects".into(), *self.cold_objects.lock());
        m.insert("tier_down_total".into(), *self.tier_down_total.lock());
        m.insert("tier_up_total".into(), *self.tier_up_total.lock());
        m.insert(
            "tier_migration_bytes_total".into(),
            *self.migration_bytes_total.lock(),
        );
        m.insert(
            "tier_migrations_failed".into(),
            *self.migrations_failed.lock(),
        );
        m
    }
}

impl StorageTierEngine {
    /// 创建分层存储引擎
    pub fn new() -> Self {
        let mut layer_configs = HashMap::new();
        // 默认三层配置
        layer_configs.insert(
            StorageLayer::Hot,
            StorageLayerConfig {
                layer: StorageLayer::Hot,
                name: "Hot Tier (SSD/NVMe)".to_string(),
                total_capacity: 1024 * 1024 * 1024 * 1024, // 1TB
                high_watermark_pct: 80,
                low_watermark_pct: 70,
                max_iops: 100000,
                max_bandwidth_bps: 10 * 1024 * 1024 * 1024, // 10GB/s
                avg_read_latency_us: 100,
                avg_write_latency_us: 200,
                cost_per_gb_per_month: 0.5,
                backend_path: "/data/hot".to_string(),
            },
        );
        layer_configs.insert(
            StorageLayer::Warm,
            StorageLayerConfig {
                layer: StorageLayer::Warm,
                name: "Warm Tier (HDD/Network)".to_string(),
                total_capacity: 10 * 1024 * 1024 * 1024 * 1024, // 10TB
                high_watermark_pct: 85,
                low_watermark_pct: 75,
                max_iops: 1000,
                max_bandwidth_bps: 1 * 1024 * 1024 * 1024, // 1GB/s
                avg_read_latency_us: 5000,
                avg_write_latency_us: 10000,
                cost_per_gb_per_month: 0.15,
                backend_path: "/data/warm".to_string(),
            },
        );
        layer_configs.insert(
            StorageLayer::Cold,
            StorageLayerConfig {
                layer: StorageLayer::Cold,
                name: "Cold Tier (Object/Archive)".to_string(),
                total_capacity: 100 * 1024 * 1024 * 1024 * 1024, // 100TB
                high_watermark_pct: 90,
                low_watermark_pct: 80,
                max_iops: 100,
                max_bandwidth_bps: 500 * 1024 * 1024, // 500MB/s
                avg_read_latency_us: 50000,  // 50ms
                avg_write_latency_us: 100000, // 100ms
                cost_per_gb_per_month: 0.03,
                backend_path: "s3://cold-storage".to_string(),
            },
        );

        let mut layer_used = HashMap::new();
        layer_used.insert(StorageLayer::Hot, 0);
        layer_used.insert(StorageLayer::Warm, 0);
        layer_used.insert(StorageLayer::Cold, 0);

        Self {
            layer_configs: parking_lot::RwLock::new(layer_configs),
            layer_used: parking_lot::Mutex::new(layer_used),
            policy: parking_lot::RwLock::new(TieringPolicyConfig::default()),
            access_stats: parking_lot::Mutex::new(HashMap::new()),
            migration_tasks: parking_lot::Mutex::new(VecDeque::new()),
            schedule_window: parking_lot::RwLock::new(MigrationScheduleWindow::default()),
            max_concurrent_migrations: parking_lot::Mutex::new(4),
            stats: Arc::new(TierStats::default()),
        }
    }

    /// 获取分层策略
    pub fn get_policy(&self) -> TieringPolicyConfig {
        self.policy.read().clone()
    }

    /// 设置分层策略
    pub fn set_policy(&self, policy: TieringPolicyConfig) {
        *self.policy.write() = policy;
    }

    /// 获取某层配置
    pub fn get_layer_config(&self, layer: StorageLayer) -> Option<StorageLayerConfig> {
        self.layer_configs.read().get(&layer).cloned()
    }

    /// 设置某层配置
    pub fn set_layer_config(&self, config: StorageLayerConfig) {
        self.layer_configs.write().insert(config.layer, config);
    }

    /// 获取某层已用容量
    pub fn get_layer_used(&self, layer: StorageLayer) -> u64 {
        *self.layer_used.lock().get(&layer).unwrap_or(&0)
    }

    /// 获取统计信息
    pub fn stats(&self) -> Arc<TierStats> {
        self.stats.clone()
    }

    /// 获取调度窗口配置
    pub fn get_schedule_window(&self) -> MigrationScheduleWindow {
        self.schedule_window.read().clone()
    }

    /// 设置调度窗口
    pub fn set_schedule_window(&self, window: MigrationScheduleWindow) {
        *self.schedule_window.write() = window;
    }

    // -----------------------------------------------------------------------
    // 对象管理
    // -----------------------------------------------------------------------

    /// 注册新对象（写入时调用）
    pub fn register_object(&self, object_id: &str, size: u64) -> StorageLayer {
        let policy = self.policy.read();
        let layer = self.determine_initial_layer(size, &policy);

        let stats = ObjectAccessStats::new(object_id.to_string(), size, layer);
        self.access_stats.lock().insert(object_id.to_string(), stats);

        // 更新层使用量
        *self.layer_used.lock().entry(layer).or_insert(0) += size;

        // 更新对象计数
        match layer {
            StorageLayer::Hot => *self.stats.hot_objects.lock() += 1,
            StorageLayer::Warm => *self.stats.warm_objects.lock() += 1,
            StorageLayer::Cold => *self.stats.cold_objects.lock() += 1,
        };

        layer
    }

    /// 确定对象的初始存储层
    fn determine_initial_layer(&self, size: u64, policy: &TieringPolicyConfig) -> StorageLayer {
        match policy.policy_type {
            TieringPolicyType::SizeBased => {
                if size >= policy.large_object_threshold {
                    StorageLayer::Warm
                } else {
                    StorageLayer::Hot
                }
            }
            _ => {
                // 默认：小对象保持热层，大对象直接进温层
                if size >= policy.large_object_threshold {
                    StorageLayer::Warm
                } else {
                    StorageLayer::Hot
                }
            }
        }
    }

    /// 记录对象读访问
    pub fn record_read(&self, object_id: &str, bytes: u64) -> Option<StorageLayer> {
        let mut stats_map = self.access_stats.lock();
        let stats = stats_map.get_mut(object_id)?;
        stats.record_read(bytes);

        let policy = self.policy.read();
        let current_layer = stats.current_layer;

        // 检查是否需要升级
        if policy.promote_on_read && current_layer != StorageLayer::Hot {
            if stats.access_count_24h >= policy.promote_min_access_count as u64 {
                let heat = stats.heat_score();
                let should_promote = match current_layer {
                    StorageLayer::Warm => heat >= policy.heat_threshold_warm_to_hot as f64,
                    StorageLayer::Cold => heat >= policy.heat_threshold_warm_to_cold as f64, // 先升到温
                    StorageLayer::Hot => false,
                };

                if should_promote {
                    let target = match current_layer {
                        StorageLayer::Warm => StorageLayer::Hot,
                        StorageLayer::Cold => StorageLayer::Warm,
                        StorageLayer::Hot => StorageLayer::Hot,
                    };

                    // 提交升级任务
                    self.queue_migration(object_id, current_layer, target, stats.size_bytes, 10);
                }
            }
        }

        Some(current_layer)
    }

    /// 记录对象写访问
    pub fn record_write(&self, object_id: &str, bytes: u64) {
        let mut stats_map = self.access_stats.lock();
        if let Some(stats) = stats_map.get_mut(object_id) {
            stats.record_write(bytes);
        }
    }

    /// 获取对象访问统计
    pub fn get_object_stats(&self, object_id: &str) -> Option<ObjectAccessStats> {
        self.access_stats.lock().get(object_id).cloned()
    }

    /// 删除对象
    pub fn remove_object(&self, object_id: &str) {
        let stats = {
            let mut map = self.access_stats.lock();
            map.remove(object_id)
        };

        if let Some(s) = stats {
            // 从层使用量中扣除
            let mut used = self.layer_used.lock();
            if let Some(u) = used.get_mut(&s.current_layer) {
                *u = u.saturating_sub(s.size_bytes);
            }
            drop(used);

            // 更新对象计数
            match s.current_layer {
                StorageLayer::Hot => {
                    let mut c = self.stats.hot_objects.lock();
                    *c = c.saturating_sub(1);
                }
                StorageLayer::Warm => {
                    let mut c = self.stats.warm_objects.lock();
                    *c = c.saturating_sub(1);
                }
                StorageLayer::Cold => {
                    let mut c = self.stats.cold_objects.lock();
                    *c = c.saturating_sub(1);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 分层决策
    // -----------------------------------------------------------------------

    /// 评估对象是否需要迁移
    ///
    /// 返回需要迁移的目标层（None 表示不需要迁移）
    pub fn evaluate_tiering(&self, object_id: &str) -> Option<StorageLayer> {
        let stats = self.access_stats.lock().get(object_id)?.clone();
        let policy = self.policy.read();
        let now = now_ms();

        let age_days = (now.saturating_sub(stats.created_at_ms) as f64)
            / (1000.0 * 60.0 * 60.0 * 24.0);
        let heat = stats.heat_score();
        let current = stats.current_layer;

        match policy.policy_type {
            TieringPolicyType::AgeBased => {
                if current == StorageLayer::Hot && age_days >= policy.hot_to_warm_days as f64 {
                    Some(StorageLayer::Warm)
                } else if current == StorageLayer::Warm
                    && age_days >= policy.warm_to_cold_days as f64
                {
                    Some(StorageLayer::Cold)
                } else {
                    None
                }
            }
            TieringPolicyType::AccessFrequency => {
                if current == StorageLayer::Hot
                    && heat < policy.heat_threshold_hot_to_warm as f64
                {
                    Some(StorageLayer::Warm)
                } else if current == StorageLayer::Warm
                    && heat < policy.heat_threshold_warm_to_cold as f64
                {
                    Some(StorageLayer::Cold)
                } else if current == StorageLayer::Warm
                    && heat >= policy.heat_threshold_warm_to_hot as f64
                {
                    Some(StorageLayer::Hot)
                } else {
                    None
                }
            }
            TieringPolicyType::SizeBased => {
                if current == StorageLayer::Hot
                    && stats.size_bytes >= policy.large_object_threshold
                {
                    Some(StorageLayer::Warm)
                } else if current == StorageLayer::Warm
                    && stats.size_bytes < policy.small_object_threshold
                {
                    Some(StorageLayer::Hot)
                } else {
                    None
                }
            }
            TieringPolicyType::Combined => {
                // 综合策略：降级用热度+年龄，升级用热度+访问次数
                let should_demote_to_warm = current == StorageLayer::Hot
                    && heat < policy.heat_threshold_hot_to_warm as f64
                    && age_days >= 7.0; // 至少 7 天

                let should_demote_to_cold = current == StorageLayer::Warm
                    && heat < policy.heat_threshold_warm_to_cold as f64
                    && age_days >= policy.warm_to_cold_days as f64;

                let should_promote_to_hot = current == StorageLayer::Warm
                    && heat >= policy.heat_threshold_warm_to_hot as f64
                    && stats.access_count_24h >= policy.promote_min_access_count as u64;

                if should_demote_to_cold {
                    Some(StorageLayer::Cold)
                } else if should_demote_to_warm {
                    Some(StorageLayer::Warm)
                } else if should_promote_to_hot {
                    Some(StorageLayer::Hot)
                } else {
                    None
                }
            }
            TieringPolicyType::Manual => None,
        }
    }

    /// 扫描所有对象，生成分层迁移计划
    pub fn generate_tiering_plan(&self, max_tasks: usize) -> Vec<TierMigrationTask> {
        let objects: Vec<ObjectAccessStats> =
            self.access_stats.lock().values().cloned().collect();

        let mut tasks = Vec::new();

        for stats in objects {
            if let Some(target) = self.evaluate_tiering(&stats.object_id) {
                if target == stats.current_layer {
                    continue;
                }

                // 检查目标层是否有足够空间
                let target_used = self.get_layer_used(target);
                let target_config = match self.get_layer_config(target) {
                    Some(c) => c,
                    None => continue,
                };
                if target_used + stats.size_bytes > target_config.total_capacity {
                    continue; // 空间不足，跳过
                }

                let priority = if target.is_hotter_than(stats.current_layer) {
                    5 // 升级优先级中等
                } else {
                    3 // 降级优先级较低
                };

                tasks.push(TierMigrationTask {
                    task_id: generate_tier_task_id(),
                    object_id: stats.object_id,
                    source_layer: stats.current_layer,
                    target_layer: target,
                    size_bytes: stats.size_bytes,
                    migrated_bytes: 0,
                    status: MigrationStatus::Pending,
                    created_at_ms: now_ms(),
                    started_at_ms: None,
                    completed_at_ms: None,
                    priority,
                    error: None,
                });

                if tasks.len() >= max_tasks {
                    break;
                }
            }
        }

        // 按优先级降序排列
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        tasks
    }

    // -----------------------------------------------------------------------
    // 迁移管理
    // -----------------------------------------------------------------------

    /// 将迁移任务加入队列
    fn queue_migration(
        &self,
        object_id: &str,
        source: StorageLayer,
        target: StorageLayer,
        size: u64,
        priority: u8,
    ) {
        let task = TierMigrationTask {
            task_id: generate_tier_task_id(),
            object_id: object_id.to_string(),
            source_layer: source,
            target_layer: target,
            size_bytes: size,
            migrated_bytes: 0,
            status: MigrationStatus::Pending,
            created_at_ms: now_ms(),
            started_at_ms: None,
            completed_at_ms: None,
            priority,
            error: None,
        };
        self.migration_tasks.lock().push_back(task);
    }

    /// 获取下一个待执行的迁移任务
    pub fn get_next_migration(&self) -> Option<TierMigrationTask> {
        let mut tasks = self.migration_tasks.lock();
        // 按优先级找最高的 pending 任务
        let mut best_idx = None;
        let mut best_priority = 0u8;
        for (i, task) in tasks.iter().enumerate() {
            if task.status == MigrationStatus::Pending && task.priority > best_priority {
                best_priority = task.priority;
                best_idx = Some(i);
            }
        }

        if let Some(idx) = best_idx {
            let mut task = tasks.remove(idx).unwrap();
            task.status = MigrationStatus::Running;
            task.started_at_ms = Some(now_ms());
            // 放回队列尾部（状态更新）
            tasks.push_back(task.clone());
            Some(task)
        } else {
            None
        }
    }

    /// 完成迁移任务
    pub fn complete_migration(&self, task_id: &str, success: bool, error: Option<String>) {
        let mut tasks = self.migration_tasks.lock();
        let now = now_ms();

        for task in tasks.iter_mut() {
            if task.task_id == task_id {
                if success {
                    task.status = MigrationStatus::Completed;
                    task.completed_at_ms = Some(now);
                    task.migrated_bytes = task.size_bytes;

                    // 更新层使用量和对象位置
                    let mut stats_map = self.access_stats.lock();
                    if let Some(stats) = stats_map.get_mut(&task.object_id) {
                        let old_layer = stats.current_layer;
                        stats.current_layer = task.target_layer;

                        let mut used = self.layer_used.lock();
                        if let Some(u) = used.get_mut(&old_layer) {
                            *u = u.saturating_sub(task.size_bytes);
                        }
                        *used.entry(task.target_layer).or_insert(0) += task.size_bytes;
                        drop(used);

                        // 更新对象计数
                        match old_layer {
                            StorageLayer::Hot => {
                                let mut c = self.stats.hot_objects.lock();
                                *c = c.saturating_sub(1);
                            }
                            StorageLayer::Warm => {
                                let mut c = self.stats.warm_objects.lock();
                                *c = c.saturating_sub(1);
                            }
                            StorageLayer::Cold => {
                                let mut c = self.stats.cold_objects.lock();
                                *c = c.saturating_sub(1);
                            }
                        }
                        match task.target_layer {
                            StorageLayer::Hot => {
                                *self.stats.hot_objects.lock() += 1;
                            }
                            StorageLayer::Warm => {
                                *self.stats.warm_objects.lock() += 1;
                            }
                            StorageLayer::Cold => {
                                *self.stats.cold_objects.lock() += 1;
                            }
                        }

                        // 更新迁移统计
                        if task.target_layer.is_hotter_than(old_layer) {
                            *self.stats.tier_up_total.lock() += 1;
                        } else {
                            *self.stats.tier_down_total.lock() += 1;
                        }
                        *self.stats.migration_bytes_total.lock() += task.size_bytes;
                    }
                } else {
                    task.status = MigrationStatus::Failed;
                    task.completed_at_ms = Some(now);
                    task.error = error;
                    *self.stats.migrations_failed.lock() += 1;
                }
                return;
            }
        }
    }

    /// 获取迁移任务数
    pub fn pending_migration_count(&self) -> usize {
        self.migration_tasks
            .lock()
            .iter()
            .filter(|t| t.status == MigrationStatus::Pending)
            .count()
    }

    /// 清理已完成/失败的任务
    pub fn cleanup_completed_tasks(&self) -> usize {
        let mut tasks = self.migration_tasks.lock();
        let before = tasks.len();
        tasks.retain(|t| {
            t.status == MigrationStatus::Pending || t.status == MigrationStatus::Running
        });
        before - tasks.len()
    }

    // -----------------------------------------------------------------------
    // 容量管理
    // -----------------------------------------------------------------------

    /// 检查热层是否超过高水位，生成降温计划
    pub fn check_hot_tier_capacity(&self) -> Vec<TierMigrationTask> {
        let hot_config = match self.get_layer_config(StorageLayer::Hot) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let hot_used = self.get_layer_used(StorageLayer::Hot);
        if hot_used < hot_config.high_watermark_bytes() {
            return Vec::new(); // 未超过高水位
        }

        // 需要迁移到低于低水位
        let need_to_free = hot_used.saturating_sub(hot_config.low_watermark_bytes());
        self.generate_eviction_plan(StorageLayer::Hot, need_to_free)
    }

    /// 生成驱逐计划（从某层移出数据到更冷的层）
    fn generate_eviction_plan(
        &self,
        from_layer: StorageLayer,
        target_free_bytes: u64,
    ) -> Vec<TierMigrationTask> {
        let target_layer = match from_layer {
            StorageLayer::Hot => StorageLayer::Warm,
            StorageLayer::Warm => StorageLayer::Cold,
            StorageLayer::Cold => return Vec::new(), // 冷层无法再降级
        };

        // 获取该层的所有对象，按热度从低到高排序
        let mut objects: Vec<ObjectAccessStats> = self
            .access_stats
            .lock()
            .values()
            .filter(|s| s.current_layer == from_layer)
            .cloned()
            .collect();

        objects.sort_by(|a, b| {
            a.heat_score()
                .partial_cmp(&b.heat_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut tasks = Vec::new();
        let mut freed = 0u64;

        for obj in objects {
            if freed >= target_free_bytes {
                break;
            }
            tasks.push(TierMigrationTask {
                task_id: generate_tier_task_id(),
                object_id: obj.object_id,
                source_layer: from_layer,
                target_layer,
                size_bytes: obj.size_bytes,
                migrated_bytes: 0,
                status: MigrationStatus::Pending,
                created_at_ms: now_ms(),
                started_at_ms: None,
                completed_at_ms: None,
                priority: 8, // 容量紧张时优先级高
                error: None,
            });
            freed += obj.size_bytes;
        }

        tasks
    }

    /// 检查当前时间是否在迁移调度窗口内
    pub fn is_in_schedule_window(&self) -> bool {
        let window = self.schedule_window.read();
        let now = now_ms();
        let hours = (now / 1000 / 60 / 60) % 24;

        if window.start_hour <= window.end_hour {
            hours >= window.start_hour as u64 && hours < window.end_hour as u64
        } else {
            // 跨午夜，比如 22:00 - 6:00
            hours >= window.start_hour as u64 || hours < window.end_hour as u64
        }
    }

    /// 计算总存储成本（元/月）
    pub fn total_monthly_cost(&self) -> f64 {
        let mut total = 0.0;
        let configs = self.layer_configs.read();
        let used = self.layer_used.lock();

        for (layer, config) in configs.iter() {
            let bytes = used.get(layer).copied().unwrap_or(0);
            let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            total += gb * config.cost_per_gb_per_month;
        }

        total
    }
}

impl Default for StorageTierEngine {
    fn default() -> Self {
        Self::new()
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

fn generate_tier_task_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!(
        "tier-{:08x}{:08x}",
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

    fn make_engine() -> StorageTierEngine {
        StorageTierEngine::new()
    }

    #[test]
    fn test_storage_layer_ordering() {
        assert!(StorageLayer::Hot < StorageLayer::Warm);
        assert!(StorageLayer::Warm < StorageLayer::Cold);
        assert!(StorageLayer::Hot.is_hotter_than(StorageLayer::Warm));
        assert!(StorageLayer::Warm.is_hotter_than(StorageLayer::Cold));
        assert!(StorageLayer::Cold.is_colder_than(StorageLayer::Hot));
    }

    #[test]
    fn test_storage_layer_from_str() {
        assert_eq!(StorageLayer::from_str("hot"), Some(StorageLayer::Hot));
        assert_eq!(StorageLayer::from_str("warm"), Some(StorageLayer::Warm));
        assert_eq!(StorageLayer::from_str("cold"), Some(StorageLayer::Cold));
        assert_eq!(StorageLayer::from_str("invalid"), None);
    }

    #[test]
    fn test_storage_layer_display() {
        assert_eq!(StorageLayer::Hot.to_string(), "hot");
        assert_eq!(StorageLayer::Warm.to_string(), "warm");
        assert_eq!(StorageLayer::Cold.to_string(), "cold");
    }

    #[test]
    fn test_register_object() {
        let engine = make_engine();
        let layer = engine.register_object("obj-1", 1024);
        assert_eq!(layer, StorageLayer::Hot);

        let stats = engine.get_object_stats("obj-1").unwrap();
        assert_eq!(stats.size_bytes, 1024);
        assert_eq!(stats.current_layer, StorageLayer::Hot);
        assert_eq!(*engine.stats.hot_objects.lock(), 1);
    }

    #[test]
    fn test_register_large_object() {
        let engine = make_engine();
        // 大于 100MB 的对象直接进温层
        let large_size = 200 * 1024 * 1024;
        let layer = engine.register_object("obj-large", large_size);
        assert_eq!(layer, StorageLayer::Warm);
        assert_eq!(*engine.stats.warm_objects.lock(), 1);
    }

    #[test]
    fn test_record_read() {
        let engine = make_engine();
        engine.register_object("obj-1", 1024);

        let layer = engine.record_read("obj-1", 512).unwrap();
        assert_eq!(layer, StorageLayer::Hot);

        let stats = engine.get_object_stats("obj-1").unwrap();
        assert_eq!(stats.access_count, 1);
        assert_eq!(stats.read_bytes_total, 512);
    }

    #[test]
    fn test_record_write() {
        let engine = make_engine();
        engine.register_object("obj-1", 1024);

        engine.record_write("obj-1", 1024);

        let stats = engine.get_object_stats("obj-1").unwrap();
        assert_eq!(stats.access_count, 1);
        assert_eq!(stats.write_bytes_total, 1024);
    }

    #[test]
    fn test_heat_score() {
        let mut stats = ObjectAccessStats::new("obj-1".to_string(), 1024, StorageLayer::Hot);

        // 刚创建的对象应该有一定热度
        let score1 = stats.heat_score();
        assert!(score1 >= 0.0 && score1 <= 100.0);

        // 增加访问次数后热度上升
        for _ in 0..100 {
            stats.record_read(512);
        }
        let score2 = stats.heat_score();
        assert!(score2 >= score1);
    }

    #[test]
    fn test_evaluate_tiering_age_based() {
        let engine = make_engine();
        engine.set_policy(TieringPolicyConfig {
            policy_type: TieringPolicyType::AgeBased,
            hot_to_warm_days: 1,
            warm_to_cold_days: 7,
            ..TieringPolicyConfig::default()
        });

        // 注册一个"老"对象（手动修改创建时间）
        engine.register_object("old-obj", 1024);
        {
            let mut stats_map = engine.access_stats.lock();
            if let Some(s) = stats_map.get_mut("old-obj") {
                s.created_at_ms = 0; // 很久以前
                s.last_access_ms = 0;
            }
        }

        let target = engine.evaluate_tiering("old-obj");
        // 应该降级到温层
        assert_eq!(target, Some(StorageLayer::Warm));
    }

    #[test]
    fn test_evaluate_tiering_access_frequency() {
        let engine = make_engine();
        engine.set_policy(TieringPolicyConfig {
            policy_type: TieringPolicyType::AccessFrequency,
            heat_threshold_hot_to_warm: 50,
            heat_threshold_warm_to_cold: 20,
            heat_threshold_warm_to_hot: 70,
            ..TieringPolicyConfig::default()
        });

        engine.register_object("cold-obj", 1024);
        {
            let mut stats_map = engine.access_stats.lock();
            if let Some(s) = stats_map.get_mut("cold-obj") {
                s.last_access_ms = 0; // 很久没访问
                s.access_count = 1;
            }
        }

        let target = engine.evaluate_tiering("cold-obj");
        assert!(target.is_some()); // 应该被降级
    }

    #[test]
    fn test_migration_lifecycle() {
        let engine = make_engine();
        engine.register_object("obj-1", 1024 * 1024); // 1MB

        // 手动添加一个降级任务
        engine.queue_migration("obj-1", StorageLayer::Hot, StorageLayer::Warm, 1024 * 1024, 5);

        assert_eq!(engine.pending_migration_count(), 1);

        // 获取并执行任务
        let task = engine.get_next_migration().unwrap();
        assert_eq!(task.status, MigrationStatus::Running);
        assert!(task.started_at_ms.is_some());

        // 完成任务
        engine.complete_migration(&task.task_id, true, None);

        // 验证对象已迁移
        let stats = engine.get_object_stats("obj-1").unwrap();
        assert_eq!(stats.current_layer, StorageLayer::Warm);

        // 验证层使用量更新
        assert_eq!(engine.get_layer_used(StorageLayer::Hot), 0);
        assert_eq!(engine.get_layer_used(StorageLayer::Warm), 1024 * 1024);

        // 验证统计
        assert_eq!(*engine.stats.tier_down_total.lock(), 1);
    }

    #[test]
    fn test_migration_failed() {
        let engine = make_engine();
        engine.register_object("obj-1", 1024);

        engine.queue_migration("obj-1", StorageLayer::Hot, StorageLayer::Warm, 1024, 5);
        let task = engine.get_next_migration().unwrap();

        engine.complete_migration(&task.task_id, false, Some("disk error".into()));

        assert_eq!(*engine.stats.migrations_failed.lock(), 1);
        // 对象应该还在热层
        let stats = engine.get_object_stats("obj-1").unwrap();
        assert_eq!(stats.current_layer, StorageLayer::Hot);
    }

    #[test]
    fn test_remove_object() {
        let engine = make_engine();
        engine.register_object("obj-1", 1024);
        assert_eq!(*engine.stats.hot_objects.lock(), 1);

        engine.remove_object("obj-1");
        assert_eq!(*engine.stats.hot_objects.lock(), 0);
        assert!(engine.get_object_stats("obj-1").is_none());
    }

    #[test]
    fn test_generate_tiering_plan() {
        let engine = make_engine();
        engine.set_policy(TieringPolicyConfig {
            policy_type: TieringPolicyType::AgeBased,
            hot_to_warm_days: 1,
            warm_to_cold_days: 7,
            ..TieringPolicyConfig::default()
        });

        // 创建一些对象
        for i in 0..10 {
            let oid = format!("obj-{}", i);
            engine.register_object(&oid, 1024 * (i as u64 + 1));
            // 让一半对象"变老"
            if i < 5 {
                let mut stats_map = engine.access_stats.lock();
                if let Some(s) = stats_map.get_mut(&oid) {
                    s.created_at_ms = 0;
                    s.last_access_ms = 0;
                }
            }
        }

        let plan = engine.generate_tiering_plan(100);
        assert!(!plan.is_empty());
    }

    #[test]
    fn test_hot_tier_capacity_check() {
        let engine = make_engine();

        // 配置小容量热层
        engine.set_layer_config(StorageLayerConfig {
            layer: StorageLayer::Hot,
            name: "Small Hot".to_string(),
            total_capacity: 1024 * 1024, // 1MB
            high_watermark_pct: 80,
            low_watermark_pct: 50,
            max_iops: 100,
            max_bandwidth_bps: 1000,
            avg_read_latency_us: 100,
            avg_write_latency_us: 200,
            cost_per_gb_per_month: 0.5,
            backend_path: "/data/hot".to_string(),
        });

        // 注册对象填满热层
        for i in 0..10 {
            let oid = format!("obj-{}", i);
            engine.register_object(&oid, 200 * 1024); // 每个 200KB
        }

        let plan = engine.check_hot_tier_capacity();
        assert!(!plan.is_empty());
        // 应该是从热层迁到温层
        assert!(plan.iter().all(|t| t.source_layer == StorageLayer::Hot));
        assert!(plan.iter().all(|t| t.target_layer == StorageLayer::Warm));
    }

    #[test]
    fn test_total_monthly_cost() {
        let engine = make_engine();
        engine.register_object("obj-1", 1024 * 1024 * 1024); // 1GB

        let cost = engine.total_monthly_cost();
        assert!(cost > 0.0);
    }

    #[test]
    fn test_tier_stats_snapshot() {
        let engine = make_engine();
        engine.register_object("obj-1", 1024);
        let snap = engine.stats().snapshot();
        assert!(snap.contains_key("tier_hot_objects"));
        assert!(snap.contains_key("tier_warm_objects"));
        assert!(snap.contains_key("tier_down_total"));
        assert!(snap.contains_key("tier_migration_bytes_total"));
    }

    #[test]
    fn test_cleanup_completed_tasks() {
        let engine = make_engine();
        engine.register_object("obj-1", 1024);
        engine.queue_migration("obj-1", StorageLayer::Hot, StorageLayer::Warm, 1024, 5);

        let task = engine.get_next_migration().unwrap();
        engine.complete_migration(&task.task_id, true, None);

        let cleaned = engine.cleanup_completed_tasks();
        assert_eq!(cleaned, 1);
    }

    #[test]
    fn test_layer_config_watermarks() {
        let config = StorageLayerConfig {
            layer: StorageLayer::Hot,
            name: "test".to_string(),
            total_capacity: 1000,
            high_watermark_pct: 80,
            low_watermark_pct: 50,
            max_iops: 0,
            max_bandwidth_bps: 0,
            avg_read_latency_us: 0,
            avg_write_latency_us: 0,
            cost_per_gb_per_month: 0.0,
            backend_path: String::new(),
        };

        assert_eq!(config.high_watermark_bytes(), 800);
        assert_eq!(config.low_watermark_bytes(), 500);
    }

    #[test]
    fn test_tiering_policy_default() {
        let policy = TieringPolicyConfig::default();
        assert_eq!(policy.policy_type, TieringPolicyType::Combined);
        assert_eq!(policy.hot_to_warm_days, 30);
        assert_eq!(policy.warm_to_cold_days, 90);
        assert!(policy.promote_on_read);
    }

    #[test]
    fn test_set_and_get_policy() {
        let engine = make_engine();
        let mut policy = engine.get_policy();
        policy.hot_to_warm_days = 60;
        engine.set_policy(policy.clone());

        let got = engine.get_policy();
        assert_eq!(got.hot_to_warm_days, 60);
    }

    #[test]
    fn test_schedule_window_default() {
        let window = MigrationScheduleWindow::default();
        assert_eq!(window.start_hour, 2);
        assert_eq!(window.end_hour, 6);
    }
}

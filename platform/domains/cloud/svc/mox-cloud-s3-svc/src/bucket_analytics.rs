// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 存储桶分析与指标模块
//!
//! 提供存储桶级别的统计分析能力，参考 AWS S3 Storage Metrics / CloudWatch metrics。
//!
//! # 功能特性
//!
//! * **存储桶级别统计**：对象总数、总大小、请求次数、流量统计
//! * **存储类分析**：标准/低频/归档各存储类的对象数与容量占比
//! * **访问模式分析**：热点/温点/冷点对象分布，基于访问频率自动分类
//! * **时间维度聚合**：每日/每周/每月统计数据聚合与持久化
//! * **存储成本估算**：基于存储类用量和单价计算月度成本估算
//!
//! # 设计说明
//!
//! 采用内存聚合 + 时间窗口快照的设计。每次对象操作（PUT/GET/DELETE）都会
//! 更新实时计数器，定期（默认每小时）将当前状态快照保存到历史记录中，
//! 支持按日/周/月维度的趋势分析。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{S3Error, S3Result};
use crate::lifecycle::StorageClass;

// ---------------- 类型定义 ----------------

/// 访问热度分级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessTier {
    /// 热点：近7天访问次数 >= 100
    Hot,
    /// 温点：近7天访问次数 1~99
    Warm,
    /// 冷点：近7天无访问
    Cold,
}

impl AccessTier {
    /// 根据近7天访问次数计算热度分级
    pub fn from_access_count(count: u64) -> Self {
        if count >= 100 {
            AccessTier::Hot
        } else if count > 0 {
            AccessTier::Warm
        } else {
            AccessTier::Cold
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AccessTier::Hot => "hot",
            AccessTier::Warm => "warm",
            AccessTier::Cold => "cold",
        }
    }
}

/// 存储桶实时统计指标
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BucketMetrics {
    /// 对象总数
    pub object_count: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// GET 请求次数（累计）
    pub get_requests: u64,
    /// PUT 请求次数（累计）
    pub put_requests: u64,
    /// DELETE 请求次数（累计）
    pub delete_requests: u64,
    /// 下载流量（字节，累计）
    pub download_bytes: u64,
    /// 上传流量（字节，累计）
    pub upload_bytes: u64,
    /// 按存储类统计的对象数
    pub objects_by_class: BTreeMap<String, u64>,
    /// 按存储类统计的字节数
    pub bytes_by_class: BTreeMap<String, u64>,
    /// 按访问热度统计的对象数
    pub objects_by_tier: BTreeMap<String, u64>,
    /// 按访问热度统计的字节数
    pub bytes_by_tier: BTreeMap<String, u64>,
}

/// 单条时间点统计快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// 快照时间戳（秒）
    pub timestamp_sec: u64,
    /// 当时的指标数据
    pub metrics: BucketMetrics,
}

/// 聚合周期
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationPeriod {
    Daily,
    Weekly,
    Monthly,
}

impl AggregationPeriod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "daily" | "day" => Some(AggregationPeriod::Daily),
            "weekly" | "week" => Some(AggregationPeriod::Weekly),
            "monthly" | "month" => Some(AggregationPeriod::Monthly),
            _ => None,
        }
    }

    /// 周期内包含的秒数（近似值）
    pub fn seconds(&self) -> u64 {
        match self {
            AggregationPeriod::Daily => 86400,
            AggregationPeriod::Weekly => 7 * 86400,
            AggregationPeriod::Monthly => 30 * 86400,
        }
    }
}

/// 存储成本配置（每 GB 每月价格，单位：分）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// 标准存储单价（分/GB/月）
    pub standard_cents_per_gb: f64,
    /// 低频存储单价
    pub infrequent_cents_per_gb: f64,
    /// 归档存储单价
    pub archive_cents_per_gb: f64,
    /// 冷归档单价
    pub glacier_cents_per_gb: f64,
    /// GET 请求单价（分/万次）
    pub get_cents_per_10k: f64,
    /// PUT 请求单价
    pub put_cents_per_10k: f64,
    /// 下载流量单价（分/GB）
    pub download_cents_per_gb: f64,
}

impl Default for CostConfig {
    fn default() -> Self {
        // 参考 AWS S3 标准定价（近似值）
        CostConfig {
            standard_cents_per_gb: 2.3,
            infrequent_cents_per_gb: 1.2,
            archive_cents_per_gb: 0.4,
            glacier_cents_per_gb: 0.1,
            get_cents_per_10k: 0.4,
            put_cents_per_10k: 5.0,
            download_cents_per_gb: 9.0,
        }
    }
}

/// 成本估算结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostEstimate {
    /// 存储成本（分/月）
    pub storage_cost_cents: f64,
    /// 请求成本（分/月）
    pub request_cost_cents: f64,
    /// 流量成本（分/月）
    pub transfer_cost_cents: f64,
    /// 总成本（分/月）
    pub total_cost_cents: f64,
    /// 按存储类明细
    pub storage_detail: BTreeMap<String, f64>,
}

// ---------------- 对象访问追踪 ----------------

/// 对象访问记录（用于热度分析）
#[derive(Debug, Clone)]
struct ObjectAccessRecord {
    /// 最近7天访问次数（滚动窗口）
    access_count_7d: u64,
    /// 最后访问时间戳（秒）
    last_access_sec: u64,
    /// 对象大小（字节）
    size: u64,
    /// 存储类
    storage_class: StorageClass,
}

// ---------------- 分析管理器 ----------------

/// 存储桶分析管理器
///
/// 负责维护所有存储桶的实时指标、访问记录和历史快照。
#[derive(Debug)]
pub struct AnalyticsManager {
    /// 存储桶实时指标：bucket_name -> metrics
    metrics: parking_lot::Mutex<BTreeMap<String, BucketMetrics>>,
    /// 对象访问记录：bucket_name -> (key -> record)
    access_records: parking_lot::Mutex<BTreeMap<String, BTreeMap<String, ObjectAccessRecord>>>,
    /// 历史快照：bucket_name -> [snapshots]（按时间排序，保留最近90天）
    snapshots: parking_lot::Mutex<BTreeMap<String, Vec<MetricsSnapshot>>>,
    /// 成本配置
    cost_config: parking_lot::RwLock<CostConfig>,
    /// 最大历史快照数量
    max_snapshots: usize,
}

impl Default for AnalyticsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyticsManager {
    /// 创建新的分析管理器
    pub fn new() -> Self {
        Self {
            metrics: parking_lot::Mutex::new(BTreeMap::new()),
            access_records: parking_lot::Mutex::new(BTreeMap::new()),
            snapshots: parking_lot::Mutex::new(BTreeMap::new()),
            cost_config: parking_lot::RwLock::new(CostConfig::default()),
            max_snapshots: 2160, // 90天 * 24小时 = 2160 条每小时快照
        }
    }

    /// 设置成本配置
    pub fn set_cost_config(&self, config: CostConfig) {
        *self.cost_config.write() = config;
    }

    /// 获取当前成本配置
    pub fn get_cost_config(&self) -> CostConfig {
        self.cost_config.read().clone()
    }

    // ---- 对象操作钩子：每次对象操作时调用以更新指标 ----

    /// 记录对象 PUT（创建或覆盖）
    pub fn record_put(&self, bucket: &str, key: &str, size: u64, class: StorageClass) {
        let now = now_secs();

        // 更新实时指标
        let mut metrics_map = self.metrics.lock();
        let m = metrics_map.entry(bucket.to_string()).or_default();
        m.put_requests += 1;
        m.upload_bytes += size;

        // 更新访问记录
        let mut records_map = self.access_records.lock();
        let bucket_records = records_map.entry(bucket.to_string()).or_default();

        if let Some(existing) = bucket_records.get(key) {
            // 覆盖：先减去旧大小
            let old_size = existing.size;
            let old_class = existing.storage_class;
            m.object_count = m.object_count.saturating_sub(0); // 数量不变
            m.total_bytes = m.total_bytes.saturating_sub(old_size) + size;

            // 旧存储类统计
            let old_class_key = old_class.as_str().to_string();
            *m.bytes_by_class.entry(old_class_key.clone()).or_insert(0) =
                m.bytes_by_class.get(&old_class_key).copied().unwrap_or(0).saturating_sub(old_size);
            *m.objects_by_class.entry(old_class_key).or_insert(0) =
                m.objects_by_class.get(&old_class.as_str().to_string()).copied().unwrap_or(0).saturating_sub(1);
        } else {
            // 新增
            m.object_count += 1;
            m.total_bytes += size;
        }

        // 插入/更新访问记录
        let record = ObjectAccessRecord {
            access_count_7d: 0, // PUT 不计入访问热度
            last_access_sec: now,
            size,
            storage_class: class,
        };
        bucket_records.insert(key.to_string(), record);

        // 新存储类统计
        let class_key = class.as_str().to_string();
        *m.objects_by_class.entry(class_key.clone()).or_insert(0) += 1;
        *m.bytes_by_class.entry(class_key).or_insert(0) += size;

        // 更新热度统计（简化：新增对象默认冷点）
        let tier_key = AccessTier::Cold.as_str().to_string();
        if !bucket_records.contains_key(key) || matches!(
            bucket_records.get(key).map(|r| AccessTier::from_access_count(r.access_count_7d)),
            Some(AccessTier::Cold)
        ) {
            // 新对象或本来就是冷点，更新
            // 这里简化处理：实际应根据之前状态调整
        }
        drop(metrics_map);
        drop(records_map);

        // 重新计算热度分布
        self.recalc_tier_distribution(bucket);
    }

    /// 记录对象 GET
    pub fn record_get(&self, bucket: &str, key: &str, size: u64) {
        let now = now_secs();

        // 更新实时指标
        let mut metrics_map = self.metrics.lock();
        let m = metrics_map.entry(bucket.to_string()).or_default();
        m.get_requests += 1;
        m.download_bytes += size;
        drop(metrics_map);

        // 更新访问记录
        let mut records_map = self.access_records.lock();
        if let Some(bucket_records) = records_map.get_mut(bucket) {
            if let Some(record) = bucket_records.get_mut(key) {
                record.access_count_7d += 1;
                record.last_access_sec = now;
            }
        }
        drop(records_map);

        // 更新热度分布
        self.recalc_tier_distribution(bucket);
    }

    /// 记录对象 DELETE
    pub fn record_delete(&self, bucket: &str, key: &str) {
        // 更新访问记录并获取删除对象的信息
        let mut records_map = self.access_records.lock();
        let deleted = if let Some(bucket_records) = records_map.get_mut(bucket) {
            bucket_records.remove(key)
        } else {
            None
        };
        drop(records_map);

        // 更新实时指标
        let mut metrics_map = self.metrics.lock();
        let m = metrics_map.entry(bucket.to_string()).or_default();
        m.delete_requests += 1;

        if let Some(record) = deleted {
            m.object_count = m.object_count.saturating_sub(1);
            m.total_bytes = m.total_bytes.saturating_sub(record.size);

            // 存储类统计
            let class_key = record.storage_class.as_str().to_string();
            *m.objects_by_class.entry(class_key.clone()).or_insert(0) =
                m.objects_by_class.get(&class_key).copied().unwrap_or(0).saturating_sub(1);
            *m.bytes_by_class.entry(class_key).or_insert(0) =
                m.bytes_by_class.get(&record.storage_class.as_str().to_string()).copied().unwrap_or(0).saturating_sub(record.size);
        }
        drop(metrics_map);

        // 重新计算热度分布
        self.recalc_tier_distribution(bucket);
    }

    /// 重新计算存储桶的热度分布
    fn recalc_tier_distribution(&self, bucket: &str) {
        let records_map = self.access_records.lock();
        let bucket_records = match records_map.get(bucket) {
            Some(r) => r,
            None => return,
        };

        let mut hot_objects = 0u64;
        let mut hot_bytes = 0u64;
        let mut warm_objects = 0u64;
        let mut warm_bytes = 0u64;
        let mut cold_objects = 0u64;
        let mut cold_bytes = 0u64;

        for record in bucket_records.values() {
            match AccessTier::from_access_count(record.access_count_7d) {
                AccessTier::Hot => {
                    hot_objects += 1;
                    hot_bytes += record.size;
                }
                AccessTier::Warm => {
                    warm_objects += 1;
                    warm_bytes += record.size;
                }
                AccessTier::Cold => {
                    cold_objects += 1;
                    cold_bytes += record.size;
                }
            }
        }
        drop(records_map);

        let mut metrics_map = self.metrics.lock();
        if let Some(m) = metrics_map.get_mut(bucket) {
            m.objects_by_tier.insert("hot".into(), hot_objects);
            m.objects_by_tier.insert("warm".into(), warm_objects);
            m.objects_by_tier.insert("cold".into(), cold_objects);
            m.bytes_by_tier.insert("hot".into(), hot_bytes);
            m.bytes_by_tier.insert("warm".into(), warm_bytes);
            m.bytes_by_tier.insert("cold".into(), cold_bytes);
        }
    }

    // ---- 查询接口 ----

    /// 获取存储桶实时指标
    pub fn get_metrics(&self, bucket: &str) -> S3Result<BucketMetrics> {
        let metrics_map = self.metrics.lock();
        metrics_map
            .get(bucket)
            .cloned()
            .ok_or(S3Error::NoSuchBucket)
    }

    /// 创建当前指标快照
    pub fn take_snapshot(&self, bucket: &str) -> S3Result<MetricsSnapshot> {
        let metrics = self.get_metrics(bucket)?;
        let snapshot = MetricsSnapshot {
            timestamp_sec: now_secs(),
            metrics,
        };

        let mut snapshots_map = self.snapshots.lock();
        let bucket_snapshots = snapshots_map.entry(bucket.to_string()).or_default();
        bucket_snapshots.push(snapshot.clone());

        // 限制快照数量
        if bucket_snapshots.len() > self.max_snapshots {
            let drain_count = bucket_snapshots.len() - self.max_snapshots;
            bucket_snapshots.drain(0..drain_count);
        }

        Ok(snapshot)
    }

    /// 获取指定时间范围内的快照
    pub fn get_snapshots_in_range(
        &self,
        bucket: &str,
        start_sec: u64,
        end_sec: u64,
    ) -> S3Result<Vec<MetricsSnapshot>> {
        let snapshots_map = self.snapshots.lock();
        let bucket_snapshots = snapshots_map
            .get(bucket)
            .ok_or(S3Error::NoSuchBucket)?;

        let result: Vec<MetricsSnapshot> = bucket_snapshots
            .iter()
            .filter(|s| s.timestamp_sec >= start_sec && s.timestamp_sec <= end_sec)
            .cloned()
            .collect();

        Ok(result)
    }

    /// 按周期聚合统计
    pub fn aggregate_by_period(
        &self,
        bucket: &str,
        period: AggregationPeriod,
    ) -> S3Result<Vec<MetricsSnapshot>> {
        let snapshots_map = self.snapshots.lock();
        let bucket_snapshots = snapshots_map
            .get(bucket)
            .ok_or(S3Error::NoSuchBucket)?;

        if bucket_snapshots.is_empty() {
            return Ok(Vec::new());
        }

        let period_secs = period.seconds();
        let mut aggregated: BTreeMap<u64, MetricsSnapshot> = BTreeMap::new();

        for snap in bucket_snapshots {
            let period_start = (snap.timestamp_sec / period_secs) * period_secs;
            // 每个周期取最后一个快照作为该周期的代表值
            aggregated.insert(period_start, snap.clone());
        }

        Ok(aggregated.into_values().collect())
    }

    /// 估算存储桶月度成本
    pub fn estimate_monthly_cost(&self, bucket: &str) -> S3Result<CostEstimate> {
        let metrics = self.get_metrics(bucket)?;
        let config = self.cost_config.read().clone();

        let mut estimate = CostEstimate::default();
        let mut storage_detail = BTreeMap::new();

        // 存储成本
        let gb = |bytes: u64| bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        let standard_bytes = metrics
            .bytes_by_class
            .get("HOT")
            .copied()
            .unwrap_or(0);
        let standard_cost = gb(standard_bytes) * config.standard_cents_per_gb;
        storage_detail.insert("standard".into(), standard_cost);
        estimate.storage_cost_cents += standard_cost;

        let infrequent_bytes = metrics
            .bytes_by_class
            .get("WARM")
            .copied()
            .unwrap_or(0);
        let infrequent_cost = gb(infrequent_bytes) * config.infrequent_cents_per_gb;
        storage_detail.insert("infrequent".into(), infrequent_cost);
        estimate.storage_cost_cents += infrequent_cost;

        let archive_bytes = metrics
            .bytes_by_class
            .get("COLD")
            .copied()
            .unwrap_or(0);
        let archive_cost = gb(archive_bytes) * config.archive_cents_per_gb;
        storage_detail.insert("archive".into(), archive_cost);
        estimate.storage_cost_cents += archive_cost;

        let glacier_bytes = metrics
            .bytes_by_class
            .get("GLACIER")
            .copied()
            .unwrap_or(0);
        let glacier_cost = gb(glacier_bytes) * config.glacier_cents_per_gb;
        storage_detail.insert("glacier".into(), glacier_cost);
        estimate.storage_cost_cents += glacier_cost;

        // 请求成本（按当前累计估算月度，简化为线性外推）
        let get_cost = (metrics.get_requests as f64 / 10000.0) * config.get_cents_per_10k;
        let put_cost = (metrics.put_requests as f64 / 10000.0) * config.put_cents_per_10k;
        estimate.request_cost_cents = get_cost + put_cost;

        // 流量成本
        estimate.transfer_cost_cents = gb(metrics.download_bytes) * config.download_cents_per_gb;

        estimate.storage_detail = storage_detail;
        estimate.total_cost_cents =
            estimate.storage_cost_cents + estimate.request_cost_cents + estimate.transfer_cost_cents;

        Ok(estimate)
    }

    /// 列出所有有统计数据的存储桶
    pub fn list_buckets(&self) -> Vec<String> {
        self.metrics.lock().keys().cloned().collect()
    }

    /// 重置指定存储桶的所有统计数据
    pub fn reset_bucket(&self, bucket: &str) {
        self.metrics.lock().remove(bucket);
        self.access_records.lock().remove(bucket);
        self.snapshots.lock().remove(bucket);
    }
}

// ---------------- 辅助函数 ----------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------- 共享类型别名 ----------------

/// 共享的分析管理器引用
pub type SharedAnalytics = Arc<AnalyticsManager>;

// ---------------- 单元测试 ----------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_tier_from_count() {
        assert_eq!(AccessTier::from_access_count(0), AccessTier::Cold);
        assert_eq!(AccessTier::from_access_count(1), AccessTier::Warm);
        assert_eq!(AccessTier::from_access_count(50), AccessTier::Warm);
        assert_eq!(AccessTier::from_access_count(99), AccessTier::Warm);
        assert_eq!(AccessTier::from_access_count(100), AccessTier::Hot);
        assert_eq!(AccessTier::from_access_count(1000), AccessTier::Hot);
    }

    #[test]
    fn test_aggregation_period_from_str() {
        assert_eq!(
            AggregationPeriod::from_str("daily"),
            Some(AggregationPeriod::Daily)
        );
        assert_eq!(
            AggregationPeriod::from_str("weekly"),
            Some(AggregationPeriod::Weekly)
        );
        assert_eq!(
            AggregationPeriod::from_str("monthly"),
            Some(AggregationPeriod::Monthly)
        );
        assert_eq!(AggregationPeriod::from_str("invalid"), None);
    }

    #[test]
    fn test_analytics_manager_basic() {
        let mgr = AnalyticsManager::new();

        // 初始状态：无存储桶
        assert!(mgr.get_metrics("test-bucket").is_err());

        // PUT 一个对象
        mgr.record_put("test-bucket", "file1.txt", 1024, StorageClass::Hot);

        // 验证指标
        let metrics = mgr.get_metrics("test-bucket").unwrap();
        assert_eq!(metrics.object_count, 1);
        assert_eq!(metrics.total_bytes, 1024);
        assert_eq!(metrics.put_requests, 1);
        assert_eq!(metrics.upload_bytes, 1024);
        assert_eq!(
            metrics.objects_by_class.get("HOT").copied().unwrap_or(0),
            1
        );
        assert_eq!(
            metrics.bytes_by_class.get("HOT").copied().unwrap_or(0),
            1024
        );
    }

    #[test]
    fn test_analytics_get_and_delete() {
        let mgr = AnalyticsManager::new();

        mgr.record_put("test-bucket", "file1.txt", 2048, StorageClass::Hot);
        mgr.record_get("test-bucket", "file1.txt", 2048);
        mgr.record_get("test-bucket", "file1.txt", 2048);

        let metrics = mgr.get_metrics("test-bucket").unwrap();
        assert_eq!(metrics.get_requests, 2);
        assert_eq!(metrics.download_bytes, 4096);

        // 删除对象
        mgr.record_delete("test-bucket", "file1.txt");

        let metrics = mgr.get_metrics("test-bucket").unwrap();
        assert_eq!(metrics.object_count, 0);
        assert_eq!(metrics.total_bytes, 0);
        assert_eq!(metrics.delete_requests, 1);
    }

    #[test]
    fn test_analytics_multiple_storage_classes() {
        let mgr = AnalyticsManager::new();

        mgr.record_put("bucket", "hot.txt", 100, StorageClass::Hot);
        mgr.record_put("bucket", "warm.txt", 200, StorageClass::Warm);
        mgr.record_put("bucket", "cold.txt", 300, StorageClass::Cold);
        mgr.record_put("bucket", "glacier.txt", 400, StorageClass::Glacier);

        let metrics = mgr.get_metrics("bucket").unwrap();
        assert_eq!(metrics.object_count, 4);
        assert_eq!(metrics.total_bytes, 1000);
        assert_eq!(
            metrics.objects_by_class.get("HOT").copied().unwrap_or(0),
            1
        );
        assert_eq!(
            metrics.objects_by_class.get("WARM").copied().unwrap_or(0),
            1
        );
        assert_eq!(
            metrics.objects_by_class.get("COLD").copied().unwrap_or(0),
            1
        );
        assert_eq!(
            metrics.objects_by_class.get("GLACIER").copied().unwrap_or(0),
            1
        );
    }

    #[test]
    fn test_analytics_snapshot() {
        let mgr = AnalyticsManager::new();

        mgr.record_put("bucket", "file.txt", 512, StorageClass::Hot);
        let snap = mgr.take_snapshot("bucket").unwrap();

        assert_eq!(snap.metrics.object_count, 1);
        assert_eq!(snap.metrics.total_bytes, 512);
        assert!(snap.timestamp_sec > 0);

        // 获取快照列表
        let snaps = mgr.get_snapshots_in_range("bucket", 0, u64::MAX).unwrap();
        assert_eq!(snaps.len(), 1);
    }

    #[test]
    fn test_cost_estimate() {
        let mgr = AnalyticsManager::new();

        mgr.record_put("bucket", "bigfile.bin", 1024 * 1024 * 1024, StorageClass::Hot); // 1GB
        mgr.record_get("bucket", "bigfile.bin", 1024 * 1024 * 1024);

        let cost = mgr.estimate_monthly_cost("bucket").unwrap();
        assert!(cost.storage_cost_cents > 0.0);
        assert!(cost.total_cost_cents > cost.storage_cost_cents);
        assert!(cost.storage_detail.contains_key("standard"));
    }

    #[test]
    fn test_tier_distribution() {
        let mgr = AnalyticsManager::new();

        mgr.record_put("bucket", "cold1.txt", 100, StorageClass::Hot);
        mgr.record_put("bucket", "warm1.txt", 200, StorageClass::Hot);
        mgr.record_put("bucket", "hot1.txt", 300, StorageClass::Hot);

        // 让 warm1 有几次访问
        for _ in 0..5 {
            mgr.record_get("bucket", "warm1.txt", 200);
        }

        // 让 hot1 有很多访问
        for _ in 0..150 {
            mgr.record_get("bucket", "hot1.txt", 300);
        }

        let metrics = mgr.get_metrics("bucket").unwrap();
        let hot_count = metrics.objects_by_tier.get("hot").copied().unwrap_or(0);
        let warm_count = metrics.objects_by_tier.get("warm").copied().unwrap_or(0);
        let cold_count = metrics.objects_by_tier.get("cold").copied().unwrap_or(0);

        assert_eq!(hot_count, 1, "应该有1个热点对象");
        assert_eq!(warm_count, 1, "应该有1个温点对象");
        assert_eq!(cold_count, 1, "应该有1个冷点对象");
    }

    #[test]
    fn test_list_and_reset_buckets() {
        let mgr = AnalyticsManager::new();

        mgr.record_put("bucket-a", "f1.txt", 100, StorageClass::Hot);
        mgr.record_put("bucket-b", "f2.txt", 200, StorageClass::Hot);

        let buckets = mgr.list_buckets();
        assert_eq!(buckets.len(), 2);
        assert!(buckets.contains(&"bucket-a".to_string()));
        assert!(buckets.contains(&"bucket-b".to_string()));

        mgr.reset_bucket("bucket-a");
        let buckets = mgr.list_buckets();
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0], "bucket-b");
        assert!(mgr.get_metrics("bucket-a").is_err());
    }

    #[test]
    fn test_overwrite_object() {
        let mgr = AnalyticsManager::new();

        mgr.record_put("bucket", "file.txt", 100, StorageClass::Hot);
        let m1 = mgr.get_metrics("bucket").unwrap();
        assert_eq!(m1.object_count, 1);
        assert_eq!(m1.total_bytes, 100);

        // 覆盖为更大的文件
        mgr.record_put("bucket", "file.txt", 500, StorageClass::Hot);
        let m2 = mgr.get_metrics("bucket").unwrap();
        assert_eq!(m2.object_count, 1); // 数量不变
        assert_eq!(m2.total_bytes, 500); // 大小更新
        assert_eq!(m2.put_requests, 2); // PUT次数+1
    }

    #[test]
    fn test_cost_config_custom() {
        let mgr = AnalyticsManager::new();
        let mut config = CostConfig::default();
        config.standard_cents_per_gb = 5.0; // 自定义价格
        mgr.set_cost_config(config.clone());

        let got = mgr.get_cost_config();
        assert!((got.standard_cents_per_gb - 5.0).abs() < f64::EPSILON);
    }
}

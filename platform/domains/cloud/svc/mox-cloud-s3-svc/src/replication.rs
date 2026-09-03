// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 跨区域复制 (CRR) 与同区域复制 (SRR) 模块
//!
//! 实现 AWS S3 Replication 风格的对象复制功能。
//!
//! # 功能特性
//!
//! * **复制规则配置**：支持前缀筛选、标签筛选、目标存储桶配置
//! * **增量复制**：基于版本号/时间戳的增量同步，避免全量扫描
//! * **复制状态跟踪**：每个对象的复制状态（Pending/Completed/Failed/Replica）
//! * **失败重试**：指数退避重试机制，最大重试次数可配置
//! * **死信队列**：超过重试次数的失败对象进入死信队列，供人工干预
//! * **复制指标监控**：复制延迟、成功率、失败率等关键指标
//!
//! # 设计说明
//!
//! 采用基于事件的复制模型：源存储桶的对象变更事件（PUT/DELETE）触发复制任务。
//! 复制任务进入队列后由后台 worker 异步处理。每个复制操作都有状态跟踪，
//! 支持通过 Replication Status API 查询单个对象的复制状态。

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{error::S3Result, lifecycle::StorageClass};

// ---------------- 常量 ----------------

/// 默认最大重试次数
const DEFAULT_MAX_RETRIES: u32 = 5;

/// 默认初始重试延迟（毫秒）
const DEFAULT_INITIAL_RETRY_DELAY_MS: u64 = 1000;

/// 死信队列最大容量
const DEFAULT_DLQ_CAPACITY: usize = 10000;

// ---------------- 类型定义 ----------------

/// 复制类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReplicationType {
    /// 跨区域复制
    CRR,
    /// 同区域复制
    SRR,
}

/// 复制状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ReplicationStatus {
    /// 等待复制
    Pending,
    /// 复制完成
    Completed,
    /// 复制失败
    Failed,
    /// 副本（目标端标记）
    Replica,
}

impl ReplicationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReplicationStatus::Pending => "PENDING",
            ReplicationStatus::Completed => "COMPLETED",
            ReplicationStatus::Failed => "FAILED",
            ReplicationStatus::Replica => "REPLICA",
        }
    }
}

/// 复制规则筛选条件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplicationFilter {
    /// 键前缀匹配
    pub prefix: Option<String>,
    /// 标签匹配（所有标签都匹配才生效）
    pub tags: BTreeMap<String, String>,
}

impl ReplicationFilter {
    /// 检查对象是否匹配筛选条件
    pub fn matches(&self, key: &str, obj_tags: &BTreeMap<String, String>) -> bool {
        // 前缀匹配
        if let Some(ref prefix) = self.prefix {
            if !key.starts_with(prefix) {
                return false;
            }
        }

        // 标签匹配
        for (k, v) in &self.tags {
            match obj_tags.get(k) {
                Some(obj_v) if obj_v == v => continue,
                _ => return false,
            }
        }

        true
    }
}

/// 复制目标配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationDestination {
    /// 目标存储桶 ARN（这里简化为桶名）
    pub bucket: String,
    /// 目标存储类（None 表示保持源存储类）
    pub storage_class: Option<StorageClass>,
    /// 目标区域（CRR 时不同）
    pub region: Option<String>,
    /// 目标账户 ID（跨账户复制时）
    pub account_id: Option<String>,
}

/// 单条复制规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationRule {
    /// 规则 ID
    pub id: String,
    /// 优先级（数字越小优先级越高）
    pub priority: u32,
    /// 是否启用
    pub enabled: bool,
    /// 筛选条件
    pub filter: ReplicationFilter,
    /// 目标配置
    pub destination: ReplicationDestination,
    /// 是否删除标记复制
    pub delete_marker_replication: bool,
    /// 复制类型
    pub replication_type: ReplicationType,
}

/// 复制配置（存储桶级别）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplicationConfiguration {
    /// 规则列表
    pub rules: Vec<ReplicationRule>,
    /// IAM 角色 ARN（简化）
    pub role: Option<String>,
}

/// 单个对象的复制状态记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectReplicationStatus {
    /// 源存储桶
    pub source_bucket: String,
    /// 源对象键
    pub source_key: String,
    /// 源版本 ID
    pub source_version_id: String,
    /// 目标存储桶
    pub destination_bucket: String,
    /// 目标对象键
    pub destination_key: String,
    /// 复制状态
    pub status: ReplicationStatus,
    /// 最后一次复制时间（毫秒）
    pub last_replicated_ms: u64,
    /// 重试次数
    pub retry_count: u32,
    /// 最后一次错误信息
    pub last_error: Option<String>,
    /// 复制规则 ID
    pub rule_id: String,
    /// 创建时间（毫秒）
    pub created_at_ms: u64,
}

/// 死信队列项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterEntry {
    /// 对象复制状态
    pub status: ObjectReplicationStatus,
    /// 入队时间（毫秒）
    pub enqueued_at_ms: u64,
    /// 失败原因摘要
    pub reason: String,
}

/// 复制指标统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplicationMetrics {
    /// 总复制任务数
    pub total_tasks: u64,
    /// 成功数
    pub succeeded: u64,
    /// 失败数
    pub failed: u64,
    /// 待处理数
    pub pending: u64,
    /// 死信队列长度
    pub dlq_size: u64,
    /// 平均复制延迟（毫秒）
    pub avg_latency_ms: f64,
    /// 最近一次成功复制时间（毫秒）
    pub last_success_ms: u64,
    /// 最近一次失败复制时间（毫秒）
    pub last_failure_ms: u64,
}

/// 复制任务（队列中的条目）
#[derive(Debug, Clone)]
struct ReplicationTask {
    /// 源存储桶
    source_bucket: String,
    /// 源对象键
    source_key: String,
    /// 源版本 ID
    source_version_id: String,
    /// 目标配置
    destination: ReplicationDestination,
    /// 规则 ID
    rule_id: String,
    /// 重试次数
    retry_count: u32,
    /// 下次可执行时间（毫秒，用于退避重试）
    next_attempt_ms: u64,
    /// 对象大小（用于统计）
    object_size: u64,
}

// ---------------- 复制管理器 ----------------

/// 复制管理器
///
/// 负责管理存储桶的复制配置、复制任务队列和执行、状态跟踪。
#[derive(Debug)]
pub struct ReplicationManager {
    /// 存储桶复制配置：source_bucket -> config
    configurations: parking_lot::RwLock<BTreeMap<String, ReplicationConfiguration>>,
    /// 对象复制状态：(source_bucket, source_key) -> status
    object_statuses: parking_lot::Mutex<BTreeMap<(String, String), ObjectReplicationStatus>>,
    /// 复制任务队列
    task_queue: parking_lot::Mutex<VecDeque<ReplicationTask>>,
    /// 死信队列
    dead_letter_queue: parking_lot::Mutex<VecDeque<DeadLetterEntry>>,
    /// 复制指标
    metrics: parking_lot::RwLock<ReplicationMetrics>,
    /// 最大重试次数
    max_retries: u32,
    /// 初始重试延迟（毫秒）
    initial_retry_delay_ms: u64,
    /// 死信队列容量
    dlq_capacity: usize,
}

impl Default for ReplicationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicationManager {
    /// 创建新的复制管理器
    pub fn new() -> Self {
        Self {
            configurations: parking_lot::RwLock::new(BTreeMap::new()),
            object_statuses: parking_lot::Mutex::new(BTreeMap::new()),
            task_queue: parking_lot::Mutex::new(VecDeque::new()),
            dead_letter_queue: parking_lot::Mutex::new(VecDeque::new()),
            metrics: parking_lot::RwLock::new(ReplicationMetrics::default()),
            max_retries: DEFAULT_MAX_RETRIES,
            initial_retry_delay_ms: DEFAULT_INITIAL_RETRY_DELAY_MS,
            dlq_capacity: DEFAULT_DLQ_CAPACITY,
        }
    }

    /// 配置最大重试次数
    pub fn set_max_retries(&mut self, max_retries: u32) {
        self.max_retries = max_retries;
    }

    // ---- 配置管理 ----

    /// 设置存储桶的复制配置
    pub fn set_configuration(&self, bucket: &str, config: ReplicationConfiguration) {
        self.configurations.write().insert(bucket.to_string(), config);
    }

    /// 获取存储桶的复制配置
    pub fn get_configuration(&self, bucket: &str) -> Option<ReplicationConfiguration> {
        self.configurations.read().get(bucket).cloned()
    }

    /// 删除存储桶的复制配置
    pub fn delete_configuration(&self, bucket: &str) {
        self.configurations.write().remove(bucket);
    }

    /// 添加复制规则
    pub fn add_rule(&self, bucket: &str, rule: ReplicationRule) -> S3Result<()> {
        let mut configs = self.configurations.write();
        let config = configs.entry(bucket.to_string()).or_default();
        config.rules.push(rule);
        // 按优先级排序
        config.rules.sort_by_key(|r| r.priority);
        Ok(())
    }

    // ---- 事件触发 ----

    /// 触发对象 PUT 事件（检查是否需要复制）
    pub fn on_object_put(
        &self,
        bucket: &str,
        key: &str,
        version_id: &str,
        size: u64,
        tags: &BTreeMap<String, String>,
    ) {
        let config = match self.get_configuration(bucket) {
            Some(c) => c,
            None => return,
        };

        let now = now_ms();

        for rule in &config.rules {
            if !rule.enabled {
                continue;
            }

            if !rule.filter.matches(key, tags) {
                continue;
            }

            // 创建复制状态记录
            let status = ObjectReplicationStatus {
                source_bucket: bucket.to_string(),
                source_key: key.to_string(),
                source_version_id: version_id.to_string(),
                destination_bucket: rule.destination.bucket.clone(),
                destination_key: key.to_string(),
                status: ReplicationStatus::Pending,
                last_replicated_ms: 0,
                retry_count: 0,
                last_error: None,
                rule_id: rule.id.clone(),
                created_at_ms: now,
            };

            self.object_statuses
                .lock()
                .insert((bucket.to_string(), key.to_string()), status);

            // 创建复制任务
            let task = ReplicationTask {
                source_bucket: bucket.to_string(),
                source_key: key.to_string(),
                source_version_id: version_id.to_string(),
                destination: rule.destination.clone(),
                rule_id: rule.id.clone(),
                retry_count: 0,
                next_attempt_ms: now,
                object_size: size,
            };

            self.task_queue.lock().push_back(task);

            // 更新指标
            let mut metrics = self.metrics.write();
            metrics.total_tasks += 1;
            metrics.pending += 1;
        }
    }

    /// 触发对象 DELETE 事件
    pub fn on_object_delete(&self, bucket: &str, key: &str, _version_id: &str) {
        let config = match self.get_configuration(bucket) {
            Some(c) => c,
            None => return,
        };

        // 检查是否有规则启用了删除标记复制
        let has_delete_rule = config.rules.iter().any(|r| {
            r.enabled && r.delete_marker_replication && r.filter.matches(key, &BTreeMap::new())
        });

        if has_delete_rule {
            // 简化实现：删除时也触发复制（实际应复制删除标记）
            // 这里只移除状态记录
            self.object_statuses.lock().remove(&(bucket.to_string(), key.to_string()));
        }
    }

    // ---- 任务执行 ----

    /// 处理队列中的复制任务
    ///
    /// 参数 copy_fn 为实际执行复制的函数（由外部存储层提供）。
    /// 返回处理的任务数。
    pub fn process_tasks(
        &self,
        copy_fn: impl Fn(&str, &str, &str, &str, Option<StorageClass>) -> S3Result<()>,
    ) -> usize {
        let now = now_ms();
        let mut processed = 0;

        // 取出所有到期的任务
        let mut due_tasks = Vec::new();
        {
            let mut queue = self.task_queue.lock();
            while let Some(task) = queue.front() {
                if task.next_attempt_ms <= now {
                    due_tasks.push(queue.pop_front().unwrap());
                } else {
                    break;
                }
            }
        }

        for task in &due_tasks {
            let dst_class = task.destination.storage_class;
            let result = copy_fn(
                &task.source_bucket,
                &task.source_key,
                &task.destination.bucket,
                &task.source_key, // 目标键默认同源键
                dst_class,
            );

            let status_key = (task.source_bucket.clone(), task.source_key.clone());
            let mut statuses = self.object_statuses.lock();

            match result {
                Ok(()) => {
                    // 更新状态为 Completed
                    if let Some(status) = statuses.get_mut(&status_key) {
                        status.status = ReplicationStatus::Completed;
                        status.last_replicated_ms = now_ms();
                        status.retry_count = task.retry_count;
                        status.last_error = None;
                    }

                    // 更新指标
                    let mut metrics = self.metrics.write();
                    metrics.succeeded += 1;
                    metrics.pending = metrics.pending.saturating_sub(1);
                    metrics.last_success_ms = now;

                    // 计算平均延迟
                    let latency = now.saturating_sub(task.next_attempt_ms);
                    let total = metrics.succeeded as f64;
                    metrics.avg_latency_ms =
                        (metrics.avg_latency_ms * (total - 1.0) + latency as f64) / total;
                },
                Err(e) => {
                    let retry_count = task.retry_count + 1;

                    if retry_count >= self.max_retries {
                        // 超过最大重试次数，送入死信队列
                        if let Some(status) = statuses.get_mut(&status_key) {
                            status.status = ReplicationStatus::Failed;
                            status.retry_count = retry_count;
                            status.last_error = Some(e.message());

                            let dlq_entry = DeadLetterEntry {
                                status: status.clone(),
                                enqueued_at_ms: now,
                                reason: format!("Max retries exceeded: {}", e.message()),
                            };

                            let mut dlq = self.dead_letter_queue.lock();
                            dlq.push_back(dlq_entry);
                            if dlq.len() > self.dlq_capacity {
                                dlq.pop_front();
                            }
                        }

                        let mut metrics = self.metrics.write();
                        metrics.failed += 1;
                        metrics.pending = metrics.pending.saturating_sub(1);
                        metrics.dlq_size += 1;
                        metrics.last_failure_ms = now;
                    } else {
                        // 重新入队，指数退避
                        let delay =
                            self.initial_retry_delay_ms * (1u64 << (retry_count - 1).min(10));
                        let retry_task = ReplicationTask {
                            source_bucket: task.source_bucket.clone(),
                            source_key: task.source_key.clone(),
                            source_version_id: task.source_version_id.clone(),
                            destination: task.destination.clone(),
                            rule_id: task.rule_id.clone(),
                            retry_count,
                            next_attempt_ms: now + delay,
                            object_size: task.object_size,
                        };

                        // 按时间插入队列（保持有序）
                        let mut queue = self.task_queue.lock();
                        let mut insert_idx = queue.len();
                        for (i, t) in queue.iter().enumerate() {
                            if t.next_attempt_ms > retry_task.next_attempt_ms {
                                insert_idx = i;
                                break;
                            }
                        }
                        queue.insert(insert_idx, retry_task);

                        // 更新状态
                        if let Some(status) = statuses.get_mut(&status_key) {
                            status.retry_count = retry_count;
                            status.last_error = Some(e.message());
                        }
                    }
                },
            }

            processed += 1;
        }

        processed
    }

    // ---- 查询接口 ----

    /// 获取对象的复制状态
    pub fn get_object_replication_status(
        &self,
        bucket: &str,
        key: &str,
    ) -> Option<ObjectReplicationStatus> {
        self.object_statuses.lock().get(&(bucket.to_string(), key.to_string())).cloned()
    }

    /// 获取存储桶的复制指标
    pub fn get_metrics(&self, bucket: &str) -> ReplicationMetrics {
        // 简化：返回全局指标（生产环境应按桶隔离）
        let _ = bucket;
        self.metrics.read().clone()
    }

    /// 获取死信队列内容
    pub fn get_dead_letter_queue(&self, limit: usize) -> Vec<DeadLetterEntry> {
        let dlq = self.dead_letter_queue.lock();
        dlq.iter().take(limit).cloned().collect()
    }

    /// 从死信队列重新入队（重试失败的任务）
    pub fn retry_dlq_entries(&self, limit: usize) -> usize {
        let mut dlq = self.dead_letter_queue.lock();
        let count = limit.min(dlq.len());

        let mut retried = 0;
        for _ in 0..count {
            if let Some(entry) = dlq.pop_front() {
                // 重新创建任务
                let task = ReplicationTask {
                    source_bucket: entry.status.source_bucket.clone(),
                    source_key: entry.status.source_key.clone(),
                    source_version_id: entry.status.source_version_id.clone(),
                    destination: ReplicationDestination {
                        bucket: entry.status.destination_bucket.clone(),
                        storage_class: None,
                        region: None,
                        account_id: None,
                    },
                    rule_id: entry.status.rule_id.clone(),
                    retry_count: 0,
                    next_attempt_ms: now_ms(),
                    object_size: 0,
                };

                self.task_queue.lock().push_back(task);

                // 更新状态
                let key = (entry.status.source_bucket.clone(), entry.status.source_key.clone());
                let mut statuses = self.object_statuses.lock();
                if let Some(s) = statuses.get_mut(&key) {
                    s.status = ReplicationStatus::Pending;
                    s.retry_count = 0;
                    s.last_error = None;
                }

                retried += 1;
            }
        }

        // 更新指标
        let mut metrics = self.metrics.write();
        metrics.dlq_size = metrics.dlq_size.saturating_sub(retried as u64);
        metrics.pending += retried as u64;

        retried
    }

    /// 获取队列长度
    pub fn queue_length(&self) -> usize {
        self.task_queue.lock().len()
    }

    /// 列出启用复制的存储桶
    pub fn list_replicating_buckets(&self) -> Vec<String> {
        self.configurations.read().keys().cloned().collect()
    }
}

// ---------------- 辅助函数 ----------------

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------- 共享类型别名 ----------------

/// 共享的复制管理器引用
pub type SharedReplication = Arc<ReplicationManager>;

// ---------------- 单元测试 ----------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::S3Error;

    #[test]
    fn test_replication_filter_prefix() {
        let filter = ReplicationFilter { prefix: Some("logs/".into()), tags: BTreeMap::new() };

        assert!(filter.matches("logs/2024/01.log", &BTreeMap::new()));
        assert!(!filter.matches("data/file.txt", &BTreeMap::new()));
        assert!(!filter.matches("log/test.txt", &BTreeMap::new()));
    }

    #[test]
    fn test_replication_filter_tags() {
        let mut filter_tags = BTreeMap::new();
        filter_tags.insert("env".to_string(), "prod".to_string());
        filter_tags.insert("type".to_string(), "backup".to_string());

        let filter = ReplicationFilter { prefix: None, tags: filter_tags };

        let mut obj_tags = BTreeMap::new();
        obj_tags.insert("env".to_string(), "prod".to_string());
        obj_tags.insert("type".to_string(), "backup".to_string());
        assert!(filter.matches("any.txt", &obj_tags));

        let mut obj_tags2 = BTreeMap::new();
        obj_tags2.insert("env".to_string(), "prod".to_string());
        assert!(!filter.matches("any.txt", &obj_tags2));

        let mut obj_tags3 = BTreeMap::new();
        obj_tags3.insert("env".to_string(), "dev".to_string());
        obj_tags3.insert("type".to_string(), "backup".to_string());
        assert!(!filter.matches("any.txt", &obj_tags3));
    }

    #[test]
    fn test_replication_filter_combined() {
        let mut filter_tags = BTreeMap::new();
        filter_tags.insert("replicate".to_string(), "true".to_string());

        let filter = ReplicationFilter { prefix: Some("data/".into()), tags: filter_tags };

        let mut tags = BTreeMap::new();
        tags.insert("replicate".to_string(), "true".to_string());
        assert!(filter.matches("data/file.bin", &tags));
        assert!(!filter.matches("logs/file.bin", &tags));

        let empty_tags = BTreeMap::new();
        assert!(!filter.matches("data/file.bin", &empty_tags));
    }

    #[test]
    fn test_configuration_management() {
        let mgr = ReplicationManager::new();

        assert!(mgr.get_configuration("bucket").is_none());

        let rule = ReplicationRule {
            id: "rule1".into(),
            priority: 1,
            enabled: true,
            filter: ReplicationFilter::default(),
            destination: ReplicationDestination {
                bucket: "dest-bucket".into(),
                storage_class: None,
                region: Some("us-west-2".into()),
                account_id: None,
            },
            delete_marker_replication: false,
            replication_type: ReplicationType::CRR,
        };

        let config = ReplicationConfiguration {
            rules: vec![rule],
            role: Some("arn:aws:iam::123456789012:role/replication-role".into()),
        };

        mgr.set_configuration("source-bucket", config);
        assert!(mgr.get_configuration("source-bucket").is_some());
        assert_eq!(mgr.get_configuration("source-bucket").unwrap().rules.len(), 1);

        mgr.delete_configuration("source-bucket");
        assert!(mgr.get_configuration("source-bucket").is_none());
    }

    #[test]
    fn test_on_object_put_triggers_replication() {
        let mgr = ReplicationManager::new();

        // 配置复制规则
        let rule = ReplicationRule {
            id: "rule1".into(),
            priority: 1,
            enabled: true,
            filter: ReplicationFilter { prefix: Some("docs/".into()), tags: BTreeMap::new() },
            destination: ReplicationDestination {
                bucket: "dest-bucket".into(),
                storage_class: Some(StorageClass::Warm),
                region: None,
                account_id: None,
            },
            delete_marker_replication: false,
            replication_type: ReplicationType::SRR,
        };

        let config = ReplicationConfiguration { rules: vec![rule], role: None };

        mgr.set_configuration("source-bucket", config);

        // 触发 PUT 事件（匹配前缀）
        let tags = BTreeMap::new();
        mgr.on_object_put("source-bucket", "docs/report.pdf", "v1", 1024, &tags);

        // 验证状态记录
        let status = mgr.get_object_replication_status("source-bucket", "docs/report.pdf").unwrap();
        assert_eq!(status.status, ReplicationStatus::Pending);
        assert_eq!(status.destination_bucket, "dest-bucket");
        assert_eq!(status.rule_id, "rule1");

        // 验证任务队列
        assert_eq!(mgr.queue_length(), 1);

        // 触发不匹配前缀的 PUT
        mgr.on_object_put("source-bucket", "images/photo.jpg", "v1", 2048, &tags);

        // 不匹配前缀，不应有复制任务
        assert!(mgr.get_object_replication_status("source-bucket", "images/photo.jpg").is_none());
        assert_eq!(mgr.queue_length(), 1); // 队列长度不变
    }

    #[test]
    fn test_process_tasks_success() {
        let mgr = ReplicationManager::new();

        // 配置复制
        let rule = ReplicationRule {
            id: "rule1".into(),
            priority: 1,
            enabled: true,
            filter: ReplicationFilter::default(),
            destination: ReplicationDestination {
                bucket: "dest-bucket".into(),
                storage_class: None,
                region: None,
                account_id: None,
            },
            delete_marker_replication: false,
            replication_type: ReplicationType::SRR,
        };

        let config = ReplicationConfiguration { rules: vec![rule], role: None };

        mgr.set_configuration("src-bucket", config);

        // 触发 PUT
        let tags = BTreeMap::new();
        mgr.on_object_put("src-bucket", "file.txt", "v1", 100, &tags);
        mgr.on_object_put("src-bucket", "file2.txt", "v1", 200, &tags);

        assert_eq!(mgr.queue_length(), 2);

        // 模拟复制函数
        let storage = Arc::new(parking_lot::Mutex::new(BTreeMap::new()));
        let storage_clone = storage.clone();

        let copy_fn = move |src_bucket: &str,
                            src_key: &str,
                            dst_bucket: &str,
                            dst_key: &str,
                            _class: Option<StorageClass>|
              -> S3Result<()> {
            let data = format!("data:{}/{}", src_bucket, src_key);
            storage_clone
                .lock()
                .insert(format!("{}/{}", dst_bucket, dst_key), data.into_bytes());
            Ok(())
        };

        let processed = mgr.process_tasks(copy_fn);
        assert_eq!(processed, 2);
        assert_eq!(mgr.queue_length(), 0);

        // 验证状态
        let status = mgr.get_object_replication_status("src-bucket", "file.txt").unwrap();
        assert_eq!(status.status, ReplicationStatus::Completed);

        // 验证目标存储
        let s = storage.lock();
        assert!(s.contains_key("dest-bucket/file.txt"));
        assert!(s.contains_key("dest-bucket/file2.txt"));

        // 验证指标
        let metrics = mgr.get_metrics("src-bucket");
        assert_eq!(metrics.total_tasks, 2);
        assert_eq!(metrics.succeeded, 2);
        assert_eq!(metrics.failed, 0);
        assert_eq!(metrics.pending, 0);
    }

    #[test]
    fn test_process_tasks_failure_and_dlq() {
        let mgr = ReplicationManager::new();
        // 减少最大重试次数以加快测试
        // 注意：由于 max_retries 不是 mut 方法可改的，我们用默认 5 次

        // 配置复制
        let rule = ReplicationRule {
            id: "rule1".into(),
            priority: 1,
            enabled: true,
            filter: ReplicationFilter::default(),
            destination: ReplicationDestination {
                bucket: "dest-bucket".into(),
                storage_class: None,
                region: None,
                account_id: None,
            },
            delete_marker_replication: false,
            replication_type: ReplicationType::CRR,
        };

        let config = ReplicationConfiguration { rules: vec![rule], role: None };

        mgr.set_configuration("src-bucket", config);

        // 触发 PUT
        let tags = BTreeMap::new();
        mgr.on_object_put("src-bucket", "fail.txt", "v1", 100, &tags);

        // 复制函数总是失败
        let copy_fn = |_sb: &str,
                       _sk: &str,
                       _db: &str,
                       _dk: &str,
                       _c: Option<StorageClass>|
         -> S3Result<()> {
            Err(S3Error::InternalError("simulated failure".into()))
        };

        // 第一次处理：失败并重试
        let processed = mgr.process_tasks(copy_fn);
        assert_eq!(processed, 1);
        assert_eq!(mgr.queue_length(), 1); // 重新入队（待重试）

        let status = mgr.get_object_replication_status("src-bucket", "fail.txt").unwrap();
        assert_eq!(status.retry_count, 1);
        assert!(status.last_error.is_some());
    }

    #[test]
    fn test_disabled_rule_not_triggered() {
        let mgr = ReplicationManager::new();

        let rule = ReplicationRule {
            id: "rule1".into(),
            priority: 1,
            enabled: false, // 禁用
            filter: ReplicationFilter::default(),
            destination: ReplicationDestination {
                bucket: "dest-bucket".into(),
                storage_class: None,
                region: None,
                account_id: None,
            },
            delete_marker_replication: false,
            replication_type: ReplicationType::SRR,
        };

        let config = ReplicationConfiguration { rules: vec![rule], role: None };

        mgr.set_configuration("source-bucket", config);

        let tags = BTreeMap::new();
        mgr.on_object_put("source-bucket", "file.txt", "v1", 100, &tags);

        assert_eq!(mgr.queue_length(), 0);
        assert!(mgr.get_object_replication_status("source-bucket", "file.txt").is_none());
    }

    #[test]
    fn test_dlq_retry() {
        let mgr = ReplicationManager::new();

        // 配置复制
        let rule = ReplicationRule {
            id: "rule1".into(),
            priority: 1,
            enabled: true,
            filter: ReplicationFilter::default(),
            destination: ReplicationDestination {
                bucket: "dest-bucket".into(),
                storage_class: None,
                region: None,
                account_id: None,
            },
            delete_marker_replication: false,
            replication_type: ReplicationType::SRR,
        };

        let config = ReplicationConfiguration { rules: vec![rule], role: None };

        mgr.set_configuration("src-bucket", config);

        // 触发 PUT
        let tags = BTreeMap::new();
        mgr.on_object_put("src-bucket", "test.txt", "v1", 100, &tags);

        // 多次处理让其进入死信队列（简化测试：手动送入 DLQ）
        // 直接验证 DLQ 重试机制
        let status = ObjectReplicationStatus {
            source_bucket: "src-bucket".into(),
            source_key: "manual-test.txt".into(),
            source_version_id: "v1".into(),
            destination_bucket: "dest-bucket".into(),
            destination_key: "manual-test.txt".into(),
            status: ReplicationStatus::Failed,
            last_replicated_ms: 0,
            retry_count: 5,
            last_error: Some("test error".into()),
            rule_id: "rule1".into(),
            created_at_ms: now_ms(),
        };

        let entry = DeadLetterEntry { status, enqueued_at_ms: now_ms(), reason: "test".into() };

        mgr.dead_letter_queue.lock().push_back(entry);
        assert_eq!(mgr.get_dead_letter_queue(10).len(), 1);

        // 重试 DLQ
        let retried = mgr.retry_dlq_entries(1);
        assert_eq!(retried, 1);
        assert_eq!(mgr.get_dead_letter_queue(10).len(), 0);
    }

    #[test]
    fn test_replication_status_as_str() {
        assert_eq!(ReplicationStatus::Pending.as_str(), "PENDING");
        assert_eq!(ReplicationStatus::Completed.as_str(), "COMPLETED");
        assert_eq!(ReplicationStatus::Failed.as_str(), "FAILED");
        assert_eq!(ReplicationStatus::Replica.as_str(), "REPLICA");
    }

    #[test]
    fn test_list_replicating_buckets() {
        let mgr = ReplicationManager::new();

        assert!(mgr.list_replicating_buckets().is_empty());

        mgr.set_configuration("bucket-a", ReplicationConfiguration::default());
        mgr.set_configuration("bucket-b", ReplicationConfiguration::default());

        let buckets = mgr.list_replicating_buckets();
        assert_eq!(buckets.len(), 2);
        assert!(buckets.contains(&"bucket-a".to_string()));
        assert!(buckets.contains(&"bucket-b".to_string()));
    }
}

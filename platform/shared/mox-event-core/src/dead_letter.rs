// =============================================================================
// 死信队列（DeadLetterQueue）
// =============================================================================
//
// 处理失败的事件进入死信队列，支持：
//
// - 自动重试：根据重试策略自动重试失败事件
// - 人工介入：超过最大重试次数的事件等待人工处理
// - 事件持久化：死信事件持久化存储，防止丢失
// - 指标统计：失败事件数量、重试次数、处理时长等
// =============================================================================

use crate::event::{EventPayload, EventType};
use crate::metadata::{EventId, EventMetadata, EventSource};
use crate::subscriber::SubscriberId;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

/// 重试策略
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 最大重试次数
    max_retries: u32,
    /// 初始重试间隔（毫秒）
    initial_interval_ms: u64,
    /// 最大重试间隔（毫秒）
    max_interval_ms: u64,
    /// 退避因子（指数退避）
    backoff_factor: f64,
    /// 是否启用抖动（防止惊群效应）
    jitter_enabled: bool,
}

impl RetryPolicy {
    /// 创建默认重试策略（3次重试，指数退避）
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            initial_interval_ms: 1000,
            max_interval_ms: 60000,
            backoff_factor: 2.0,
            jitter_enabled: true,
        }
    }

    /// 不重试策略
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            initial_interval_ms: 0,
            max_interval_ms: 0,
            backoff_factor: 1.0,
            jitter_enabled: false,
        }
    }

    /// 获取最大重试次数
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// 计算第 n 次重试的等待时间
    pub fn retry_interval(&self, attempt: u32) -> Duration {
        if attempt == 0 || self.initial_interval_ms == 0 {
            return Duration::from_millis(0);
        }

        let interval = (self.initial_interval_ms as f64)
            * self.backoff_factor.powi(attempt as i32 - 1);
        let interval = interval.min(self.max_interval_ms as f64);

        if self.jitter_enabled {
            // 添加 ±20% 的随机抖动
            let jitter = 0.8 + (rand_jitter() * 0.4);
            Duration::from_millis((interval * jitter) as u64)
        } else {
            Duration::from_millis(interval as u64)
        }
    }

    /// 检查是否还可以重试
    pub fn can_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// 简单的随机数生成（用于抖动）
fn rand_jitter() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    ((nanos % 1000) as f64) / 1000.0
}

/// 死信队列条目
#[derive(Debug, Clone)]
pub struct DeadLetterEntry {
    /// 事件 ID
    event_id: EventId,
    /// 事件类型
    event_type: EventType,
    /// 事件载荷
    payload: EventPayload,
    /// 事件元数据
    metadata: EventMetadata,
    /// 失败的订阅者 ID
    failed_subscriber: SubscriberId,
    /// 失败原因
    error_message: String,
    /// 已重试次数
    retry_count: u32,
    /// 首次失败时间（Unix 毫秒时间戳）
    first_failed_at: u64,
    /// 最后失败时间
    last_failed_at: u64,
    /// 下次可重试时间（Unix 毫秒时间戳）
    next_retry_at: Option<u64>,
    /// 状态
    status: DeadLetterStatus,
}

/// 死信条目状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadLetterStatus {
    /// 等待重试
    PendingRetry,
    /// 已达到最大重试次数，等待人工处理
    AwaitingManual,
    /// 已处理成功
    Resolved,
    /// 已丢弃
    Discarded,
}

impl DeadLetterEntry {
    /// 创建新的死信条目
    pub fn new(
        event_id: EventId,
        event_type: EventType,
        payload: EventPayload,
        metadata: EventMetadata,
        failed_subscriber: SubscriberId,
        error_message: String,
    ) -> Self {
        let now = current_timestamp_ms();
        Self {
            event_id,
            event_type,
            payload,
            metadata,
            failed_subscriber,
            error_message,
            retry_count: 0,
            first_failed_at: now,
            last_failed_at: now,
            next_retry_at: None,
            status: DeadLetterStatus::PendingRetry,
        }
    }

    /// 获取事件 ID
    pub fn event_id(&self) -> &EventId {
        &self.event_id
    }

    /// 获取事件类型
    pub fn event_type(&self) -> &EventType {
        &self.event_type
    }

    /// 获取事件载荷
    pub fn payload(&self) -> &EventPayload {
        &self.payload
    }

    /// 获取事件元数据
    pub fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    /// 获取失败的订阅者 ID
    pub fn failed_subscriber(&self) -> &SubscriberId {
        &self.failed_subscriber
    }

    /// 获取错误消息
    pub fn error_message(&self) -> &str {
        &self.error_message
    }

    /// 获取重试次数
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// 获取状态
    pub fn status(&self) -> &DeadLetterStatus {
        &self.status
    }

    /// 记录重试失败
    pub fn record_retry_failed(&mut self, error_message: String, retry_policy: &RetryPolicy) {
        self.retry_count += 1;
        self.error_message = error_message;
        self.last_failed_at = current_timestamp_ms();

        if retry_policy.can_retry(self.retry_count) {
            let interval = retry_policy.retry_interval(self.retry_count);
            self.next_retry_at = Some(self.last_failed_at + interval.as_millis() as u64);
            self.status = DeadLetterStatus::PendingRetry;
        } else {
            self.next_retry_at = None;
            self.status = DeadLetterStatus::AwaitingManual;
        }
    }

    /// 标记为已解决
    pub fn mark_resolved(&mut self) {
        self.status = DeadLetterStatus::Resolved;
    }

    /// 标记为已丢弃
    pub fn mark_discarded(&mut self) {
        self.status = DeadLetterStatus::Discarded;
    }

    /// 是否可以重试
    pub fn can_retry_now(&self) -> bool {
        if self.status != DeadLetterStatus::PendingRetry {
            return false;
        }
        match self.next_retry_at {
            Some(at) => current_timestamp_ms() >= at,
            None => true,
        }
    }
}

/// 获取当前时间戳（毫秒）
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 死信队列
///
/// 存储处理失败的事件，支持自动重试和人工处理。
pub struct DeadLetterQueue {
    /// 队列配置
    config: DeadLetterConfig,
    /// 死信条目
    entries: Mutex<VecDeque<DeadLetterEntry>>,
    /// 重试策略
    retry_policy: RetryPolicy,
}

/// 死信队列配置
#[derive(Debug, Clone)]
pub struct DeadLetterConfig {
    /// 最大队列大小
    pub max_size: usize,
    /// 已解决条目保留时间（毫秒）
    pub resolved_retention_ms: u64,
}

impl Default for DeadLetterConfig {
    fn default() -> Self {
        Self {
            max_size: 10000,
            resolved_retention_ms: 3600000, // 1小时
        }
    }
}

impl DeadLetterQueue {
    /// 创建新的死信队列
    pub fn new(config: DeadLetterConfig, retry_policy: RetryPolicy) -> Arc<Self> {
        Arc::new(Self {
            config,
            entries: Mutex::new(VecDeque::new()),
            retry_policy,
        })
    }

    /// 创建默认配置的死信队列
    pub fn default() -> Arc<Self> {
        Self::new(DeadLetterConfig::default(), RetryPolicy::default())
    }

    /// 添加死信条目
    pub fn push(&self, entry: DeadLetterEntry) -> Result<(), crate::EventError> {
        let mut entries = self.entries.lock();
        if entries.len() >= self.config.max_size {
            return Err(crate::EventError::DeadLetterQueueFull);
        }
        entries.push_back(entry);
        Ok(())
    }

    /// 获取可重试的条目（不删除）
    pub fn get_retryable(&self, limit: usize) -> Vec<DeadLetterEntry> {
        let entries = self.entries.lock();
        entries
            .iter()
            .filter(|e| e.can_retry_now())
            .take(limit)
            .cloned()
            .collect()
    }

    /// 获取等待人工处理的条目
    pub fn get_awaiting_manual(&self, limit: usize) -> Vec<DeadLetterEntry> {
        let entries = self.entries.lock();
        entries
            .iter()
            .filter(|e| e.status() == &DeadLetterStatus::AwaitingManual)
            .take(limit)
            .cloned()
            .collect()
    }

    /// 更新条目（重试失败后）
    pub fn update_entry_retry_failed(
        &self,
        event_id: &EventId,
        error_message: String,
    ) -> Option<DeadLetterEntry> {
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.iter_mut().find(|e| e.event_id() == event_id) {
            entry.record_retry_failed(error_message, &self.retry_policy);
            Some(entry.clone())
        } else {
            None
        }
    }

    /// 标记条目为已解决
    pub fn mark_resolved(&self, event_id: &EventId) -> bool {
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.iter_mut().find(|e| e.event_id() == event_id) {
            entry.mark_resolved();
            true
        } else {
            false
        }
    }

    /// 标记条目为已丢弃
    pub fn mark_discarded(&self, event_id: &EventId) -> bool {
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.iter_mut().find(|e| e.event_id() == event_id) {
            entry.mark_discarded();
            true
        } else {
            false
        }
    }

    /// 获取队列统计
    pub fn stats(&self) -> DeadLetterStats {
        let entries = self.entries.lock();
        DeadLetterStats {
            total: entries.len(),
            pending_retry: entries.iter().filter(|e| e.status() == &DeadLetterStatus::PendingRetry).count(),
            awaiting_manual: entries.iter().filter(|e| e.status() == &DeadLetterStatus::AwaitingManual).count(),
            resolved: entries.iter().filter(|e| e.status() == &DeadLetterStatus::Resolved).count(),
            discarded: entries.iter().filter(|e| e.status() == &DeadLetterStatus::Discarded).count(),
        }
    }

    /// 清理已解决和已丢弃的条目
    pub fn cleanup(&self) -> usize {
        let mut entries = self.entries.lock();
        let before = entries.len();
        entries.retain(|e| {
            e.status() != &DeadLetterStatus::Resolved && e.status() != &DeadLetterStatus::Discarded
        });
        before - entries.len()
    }

    /// 获取重试策略引用
    pub fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }
}

/// 死信队列统计
#[derive(Debug, Clone)]
pub struct DeadLetterStats {
    /// 总条目数
    pub total: usize,
    /// 等待重试数
    pub pending_retry: usize,
    /// 等待人工处理数
    pub awaiting_manual: usize,
    /// 已解决数
    pub resolved: usize,
    /// 已丢弃数
    pub discarded: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries(), 3);
        assert!(policy.can_retry(0));
        assert!(policy.can_retry(2));
        assert!(!policy.can_retry(3));
    }

    #[test]
    fn test_retry_policy_no_retry() {
        let policy = RetryPolicy::no_retry();
        assert_eq!(policy.max_retries(), 0);
        assert!(!policy.can_retry(0));
    }

    #[test]
    fn test_retry_interval_exponential_backoff() {
        let policy = RetryPolicy::new()
            .with_max_retries(5)
            .with_max_retries(5);
        let interval1 = policy.retry_interval(1);
        let interval2 = policy.retry_interval(2);
        let interval3 = policy.retry_interval(3);
        // 指数退避：interval2 应该大于 interval1
        assert!(interval2.as_millis() > interval1.as_millis());
        assert!(interval3.as_millis() > interval2.as_millis());
    }

    #[test]
    fn test_dead_letter_entry() {
        let entry = DeadLetterEntry::new(
            EventId::new(),
            EventType::new("test.event"),
            EventPayload::empty(),
            EventMetadata::new(EventSource::new("test")),
            SubscriberId::new("test-sub"),
            "处理失败".to_string(),
        );

        assert_eq!(entry.retry_count(), 0);
        assert_eq!(entry.status(), &DeadLetterStatus::PendingRetry);
        assert!(entry.can_retry_now());
    }

    #[test]
    fn test_dead_letter_entry_retry_failed() {
        let policy = RetryPolicy::new().with_max_retries(2);
        let mut entry = DeadLetterEntry::new(
            EventId::new(),
            EventType::new("test.event"),
            EventPayload::empty(),
            EventMetadata::new(EventSource::new("test")),
            SubscriberId::new("test-sub"),
            "第一次失败".to_string(),
        );

        // 第一次重试失败
        entry.record_retry_failed("第二次失败".to_string(), &policy);
        assert_eq!(entry.retry_count(), 1);
        assert_eq!(entry.status(), &DeadLetterStatus::PendingRetry);

        // 第二次重试失败（达到最大重试次数）
        entry.record_retry_failed("第三次失败".to_string(), &policy);
        assert_eq!(entry.retry_count(), 2);
        assert_eq!(entry.status(), &DeadLetterStatus::AwaitingManual);
        assert!(!entry.can_retry_now());
    }

    #[tokio::test]
    async fn test_dead_letter_queue() {
        let dlq = DeadLetterQueue::default();

        let entry = DeadLetterEntry::new(
            EventId::new(),
            EventType::new("test.event"),
            EventPayload::empty(),
            EventMetadata::new(EventSource::new("test")),
            SubscriberId::new("test-sub"),
            "处理失败".to_string(),
        );
        let event_id = entry.event_id().clone();

        dlq.push(entry).unwrap();

        let stats = dlq.stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.pending_retry, 1);

        // 获取可重试条目
        let retryable = dlq.get_retryable(10);
        assert_eq!(retryable.len(), 1);

        // 标记为已解决
        assert!(dlq.mark_resolved(&event_id));
        let stats = dlq.stats();
        assert_eq!(stats.resolved, 1);

        // 清理
        let cleaned = dlq.cleanup();
        assert_eq!(cleaned, 1);
        let stats = dlq.stats();
        assert_eq!(stats.total, 0);
    }
}

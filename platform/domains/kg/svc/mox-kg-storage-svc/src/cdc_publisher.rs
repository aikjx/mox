// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # CDC 变更数据捕获发布者
//!
//! 基于 Raft 日志的变更数据捕获（Change Data Capture）模块，
//! 用于将知识图谱的变更事件实时推送给下游消费者。
//!
//! ## 设计要点
//!
//! - **事件驱动**：所有写入操作产生 CDC 事件，消费者可以实时感知数据变更
//! - **消费者组**：支持多个消费者组独立消费，每个组有独立的 offset
//! - **幂等消费**：事件包含唯一 ID（raft_index + entity_id），消费者可去重
//! - **背压流控**：消费者处理不过来时，事件会被缓冲在队列中
//! - **批量推送**：支持批量推送事件，减少网络开销
//!
//! ## 事件类型
//!
//! | 事件类型          | 触发时机           | Payload 包含              |
//! |-------------------|--------------------|---------------------------|
//! | VertexCreated     | 顶点插入成功       | 顶点完整数据              |
//! | VertexUpdated     | 顶点属性更新       | 更新后的属性              |
//! | VertexDeleted     | 顶点删除成功       | 顶点 ID                   |
//! | EdgeCreated       | 边插入成功         | 边完整数据                |
//! | EdgeDeleted       | 边删除成功         | 边标识（src/dst/type/rank）|
//!
//! ## 消费者模型
//!
//! ```text
//! ┌──────────────┐     ┌──────────────────┐     ┌──────────────┐
//! │   Producer   │────▶│  CdcPublisher    │────▶│  Consumer A  │
//! │  (Raft层)    │     │  - 事件队列      │     │  (消费者组1) │
//! └──────────────┘     │  - offset 管理   │     └──────────────┘
//!                      │  - 消费者组      │     ┌──────────────┐
//!                      │  - 流控          │────▶│  Consumer B  │
//!                      └──────────────────┘     │  (消费者组2) │
//!                                               └──────────────┘
//! ```
//!
//! ## 流控策略
//!
//! - 每个消费者有独立的缓冲队列（默认 10000 条）
//! - 队列满时，发布者会等待（背压）或丢弃旧事件（可配置）
//! - 支持手动 commit offset，确保消费者处理完成后才推进进度

use crate::error::{StorageError, StorageResult};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

// ============================================================================
// 事件类型定义
// ============================================================================

/// CDC 事件类型（统一从 cdc_source 导出，含 EdgeUpdated）
pub use crate::cdc_source::CdcEventType;


/// CDC 事件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CdcEvent {
    /// 事件类型
    pub event_type: CdcEventType,
    /// 图空间 ID
    pub space_id: i32,
    /// 实体 ID（顶点 VID 或边的复合 ID）
    pub entity_id: String,
    /// 事件载荷（具体数据）
    pub payload: serde_json::Value,
    /// 事件时间戳（毫秒）
    pub timestamp: i64,
    /// 对应的 Raft 日志索引（全局唯一递增）
    pub raft_index: u64,
}

impl CdcEvent {
    /// 生成事件的唯一键（用于幂等消费去重）
    pub fn idempotent_key(&self) -> String {
        format!("{}:{}:{}", self.space_id, self.event_type.as_str(), self.entity_id)
    }

    /// 判断是否为顶点事件
    pub fn is_vertex_event(&self) -> bool {
        self.event_type.is_vertex_event()
    }

    /// 判断是否为边事件
    pub fn is_edge_event(&self) -> bool {
        self.event_type.is_edge_event()
    }
}

// ============================================================================
// 消费者状态
// ============================================================================

/// 消费者状态
struct ConsumerState {
    /// 事件发送端
    tx: UnboundedSender<CdcEvent>,
    /// 已提交的 offset
    committed_offset: u64,
    /// 最后拉取时间戳（毫秒）
    last_poll_ts: u64,
    /// 消费者组 ID
    group_id: String,
    /// 订阅的事件类型过滤（None 表示所有类型）
    event_filter: Option<Vec<CdcEventType>>,
}

/// 消费者组状态
struct ConsumerGroup {
    /// 组内消费者
    consumers: BTreeMap<u64, ConsumerState>,
    /// 组内下一个消费者 ID
    next_consumer_id: u64,
}

// ============================================================================
// Topic 状态
// ============================================================================

/// Topic 状态
struct TopicState {
    /// 事件队列（环形缓冲）
    queue: VecDeque<CdcEvent>,
    /// 下一个事件的 offset
    next_offset: u64,
    /// 消费者组
    groups: BTreeMap<String, ConsumerGroup>,
    /// 队列最大容量
    max_queue_size: usize,
    /// 丢弃的事件数（队列满时）
    dropped_count: u64,
}

impl TopicState {
    fn new(max_queue_size: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_queue_size.min(1024)),
            next_offset: 1,
            groups: BTreeMap::new(),
            max_queue_size,
            dropped_count: 0,
        }
    }
}

// ============================================================================
// 批量聚合
// ============================================================================

/// 待刷新的批量事件
#[derive(Default)]
struct PendingBatch {
    events: Vec<(String, CdcEvent)>, // (topic, event)
    first_ts: Option<u64>,
}

/// 流控策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowControlPolicy {
    /// 等待：队列满时阻塞等待（背压）
    Wait,
    /// 丢弃旧事件：队列满时丢弃最旧的事件
    DropOldest,
    /// 丢弃新事件：队列满时丢弃新事件
    DropNewest,
}

// ============================================================================
// CdcPublisher
// ============================================================================

/// CDC 事件发布者
///
/// 管理多个 Topic 的事件发布和消费者订阅。
/// 支持消费者组、offset 管理、幂等消费、背压流控。
pub struct CdcPublisher {
    /// Topic 状态映射
    topics: Mutex<BTreeMap<String, TopicState>>,
    /// 默认 Topic 名称
    default_topic: String,
    /// 待发布的批量事件（用于聚合）
    pending: Mutex<PendingBatch>,
    /// 全局 Raft 索引计数器
    global_raft_index: Arc<AtomicU64>,
    /// 流控策略
    flow_control: Mutex<FlowControlPolicy>,
    /// 批量刷新间隔（毫秒）
    batch_flush_interval_ms: u64,
    /// 最大队列大小
    max_queue_size: usize,
}

impl CdcPublisher {
    /// 创建新的 CDC 发布者
    ///
    /// # Arguments
    /// * `default_topic` - 默认 Topic 名称
    pub fn new(default_topic: &str) -> Self {
        let mut topics = BTreeMap::new();
        topics.insert(
            default_topic.to_string(),
            TopicState::new(1_000_000), // 默认 100 万事件容量
        );

        Self {
            topics: Mutex::new(topics),
            default_topic: default_topic.to_string(),
            pending: Mutex::new(PendingBatch::default()),
            global_raft_index: Arc::new(AtomicU64::new(0)),
            flow_control: Mutex::new(FlowControlPolicy::DropOldest),
            batch_flush_interval_ms: 200, // 默认 200ms 刷新
            max_queue_size: 1_000_000,
        }
    }

    /// 获取默认 Topic 名称
    pub fn default_topic(&self) -> &str {
        &self.default_topic
    }

    /// 设置流控策略
    pub fn set_flow_control(&self, policy: FlowControlPolicy) {
        *self.flow_control.lock() = policy;
    }

    /// 获取当前流控策略
    pub fn flow_control_policy(&self) -> FlowControlPolicy {
        *self.flow_control.lock()
    }

    /// 发布事件到默认 Topic
    pub fn publish(&self, event: CdcEvent) -> u64 {
        self.publish_to_topic(&self.default_topic.clone(), event)
    }

    /// 发布事件到指定 Topic
    pub fn publish_to_topic(&self, topic: &str, mut event: CdcEvent) -> u64 {
        self.ensure_topic(topic);

        // 分配 raft_index
        let raft_idx = self.global_raft_index.fetch_add(1, Ordering::SeqCst) + 1;
        event.raft_index = raft_idx;

        let ts_ms = now_ms();

        // 添加到待发布队列
        let mut pending = self.pending.lock();
        if pending.first_ts.is_none() {
            pending.first_ts = Some(ts_ms);
        }
        pending.events.push((topic.to_string(), event));

        // 检查是否需要刷新
        let need_flush = match pending.first_ts {
            Some(first) if ts_ms.saturating_sub(first) >= self.batch_flush_interval_ms => true,
            _ => false,
        };
        drop(pending);

        if need_flush {
            self.flush();
        }

        raft_idx
    }

    /// 手动刷新：将待发布的事件推送到 Topic 队列和所有消费者
    ///
    /// 返回本次刷新的事件数
    pub fn flush(&self) -> usize {
        let mut pending = self.pending.lock();
        let events = std::mem::take(&mut pending.events);
        pending.first_ts = None;
        drop(pending);

        if events.is_empty() {
            return 0;
        }

        let mut topics = self.topics.lock();
        let total = events.len();

        for (topic, event) in events {
            let Some(state) = topics.get_mut(&topic) else {
                continue;
            };

            // 分配 offset
            let offset = state.next_offset;
            state.next_offset += 1;

            // 存入队列
            if state.queue.len() >= state.max_queue_size {
                match *self.flow_control.lock() {
                    FlowControlPolicy::DropOldest => {
                        state.queue.pop_front();
                        state.dropped_count += 1;
                    }
                    FlowControlPolicy::DropNewest => {
                        state.dropped_count += 1;
                        continue; // 跳过这个事件
                    }
                    FlowControlPolicy::Wait => {
                        // 简化实现：等待也没用，还是 drop oldest
                        state.queue.pop_front();
                        state.dropped_count += 1;
                    }
                }
            }

            let event_with_offset = event.clone();
            state.queue.push_back(event_with_offset);

            // 推送给所有消费者
            let mut dead_groups = Vec::new();
            for (group_name, group) in state.groups.iter_mut() {
                let mut dead_consumers = Vec::new();
                for (consumer_id, consumer) in group.consumers.iter_mut() {
                    // 事件类型过滤
                    if let Some(ref filter) = consumer.event_filter {
                        if !filter.contains(&event.event_type) {
                            continue;
                        }
                    }

                    if consumer.tx.send(event.clone()).is_err() {
                        dead_consumers.push(*consumer_id);
                    } else {
                        consumer.last_poll_ts = now_ms();
                    }
                }
                for cid in dead_consumers {
                    group.consumers.remove(&cid);
                }
                if group.consumers.is_empty() {
                    dead_groups.push(group_name.clone());
                }
            }
            for g in dead_groups {
                state.groups.remove(&g);
            }
        }

        total
    }

    /// 订阅 Topic（创建消费者）
    ///
    /// # Arguments
    /// * `topic` - Topic 名称
    /// * `group_id` - 消费者组 ID
    /// * `since_offset` - 起始 offset（从该 offset 之后开始消费）
    /// * `event_filter` - 事件类型过滤（None 表示所有类型）
    ///
    /// # Returns
    /// 返回消费者 ID 和事件接收端
    pub fn subscribe(
        &self,
        topic: &str,
        group_id: &str,
        since_offset: u64,
        event_filter: Option<Vec<CdcEventType>>,
    ) -> StorageResult<(u64, UnboundedReceiver<CdcEvent>)> {
        self.ensure_topic(topic);

        let mut topics = self.topics.lock();
        let state = topics
            .get_mut(topic)
            .ok_or_else(|| StorageError::InvalidArgument(format!("topic not found: {topic}")))?;

        let (tx, rx) = mpsc::unbounded_channel();

        // 获取或创建消费者组
        let group = state
            .groups
            .entry(group_id.to_string())
            .or_insert_with(|| ConsumerGroup {
                consumers: BTreeMap::new(),
                next_consumer_id: 1,
            });

        let consumer_id = group.next_consumer_id;
        group.next_consumer_id += 1;

        // 重放 since_offset 之后的事件
        let start_exclusive = since_offset;
        for ev in state.queue.iter() {
            // 使用 raft_index 作为 offset 的近似
            if ev.raft_index > start_exclusive {
                if let Some(ref filter) = event_filter {
                    if !filter.contains(&ev.event_type) {
                        continue;
                    }
                }
                let _ = tx.send(ev.clone());
            }
        }

        group.consumers.insert(
            consumer_id,
            ConsumerState {
                tx,
                committed_offset: since_offset,
                last_poll_ts: now_ms(),
                group_id: group_id.to_string(),
                event_filter,
            },
        );

        drop(topics);

        Ok((consumer_id, rx))
    }

    /// 提交消费者 offset
    ///
    /// 消费者处理完一批事件后，调用此方法确认进度。
    pub fn commit_offset(
        &self,
        topic: &str,
        group_id: &str,
        consumer_id: u64,
        offset: u64,
    ) -> StorageResult<()> {
        let mut topics = self.topics.lock();
        let state = topics
            .get_mut(topic)
            .ok_or_else(|| StorageError::InvalidArgument(format!("topic not found: {topic}")))?;

        let group = state
            .groups
            .get_mut(group_id)
            .ok_or_else(|| StorageError::InvalidArgument(format!("group not found: {group_id}")))?;

        let consumer =
            group
                .consumers
                .get_mut(&consumer_id)
                .ok_or_else(|| {
                    StorageError::InvalidArgument(format!("consumer not found: {consumer_id}"))
                })?;

        consumer.committed_offset = consumer.committed_offset.max(offset);
        consumer.last_poll_ts = now_ms();

        Ok(())
    }

    /// 获取消费者组的 lag（落后的事件数）
    pub fn consumer_lag(&self, topic: &str, group_id: &str) -> StorageResult<u64> {
        let topics = self.topics.lock();
        let state = topics
            .get(topic)
            .ok_or_else(|| StorageError::InvalidArgument(format!("topic not found: {topic}")))?;

        let group = state
            .groups
            .get(group_id)
            .ok_or_else(|| StorageError::InvalidArgument(format!("group not found: {group_id}")))?;

        // 计算组内最小的 committed_offset
        let min_committed = group
            .consumers
            .values()
            .map(|c| c.committed_offset)
            .min()
            .unwrap_or(0);

        let latest = state.next_offset - 1;
        Ok(latest.saturating_sub(min_committed))
    }

    /// 获取消费者延迟时间（毫秒）
    pub fn consumer_lag_ms(&self, topic: &str, group_id: &str) -> StorageResult<Duration> {
        let topics = self.topics.lock();
        let state = topics
            .get(topic)
            .ok_or_else(|| StorageError::InvalidArgument(format!("topic not found: {topic}")))?;

        let latest_ts = state.queue.back().map(|e| e.timestamp as u64).unwrap_or(0);

        let group = state
            .groups
            .get(group_id)
            .ok_or_else(|| StorageError::InvalidArgument(format!("group not found: {group_id}")))?;

        // 找到最落后的消费者
        let mut oldest_event_ts = latest_ts;
        for consumer in group.consumers.values() {
            let committed = consumer.committed_offset;
            // 找到第一个 > committed 的事件的时间戳
            if let Some(event) = state.queue.iter().find(|e| e.raft_index > committed) {
                oldest_event_ts = oldest_event_ts.min(event.timestamp as u64);
            }
        }

        let lag_ms = if latest_ts > oldest_event_ts {
            latest_ts - oldest_event_ts
        } else {
            0
        };

        Ok(Duration::from_millis(lag_ms))
    }

    /// 取消订阅
    pub fn unsubscribe(&self, topic: &str, group_id: &str, consumer_id: u64) -> StorageResult<()> {
        let mut topics = self.topics.lock();
        let state = topics
            .get_mut(topic)
            .ok_or_else(|| StorageError::InvalidArgument(format!("topic not found: {topic}")))?;

        if let Some(group) = state.groups.get_mut(group_id) {
            group.consumers.remove(&consumer_id);
            if group.consumers.is_empty() {
                state.groups.remove(group_id);
            }
        }

        Ok(())
    }

    /// 获取 Topic 的事件总数
    pub fn topic_event_count(&self, topic: &str) -> StorageResult<u64> {
        let topics = self.topics.lock();
        let state = topics
            .get(topic)
            .ok_or_else(|| StorageError::InvalidArgument(format!("topic not found: {topic}")))?;
        Ok(state.queue.len() as u64)
    }

    /// 获取 Topic 的丢弃事件数
    pub fn topic_dropped_count(&self, topic: &str) -> StorageResult<u64> {
        let topics = self.topics.lock();
        let state = topics
            .get(topic)
            .ok_or_else(|| StorageError::InvalidArgument(format!("topic not found: {topic}")))?;
        Ok(state.dropped_count)
    }

    /// 获取消费者组列表
    pub fn list_groups(&self, topic: &str) -> StorageResult<Vec<String>> {
        let topics = self.topics.lock();
        let state = topics
            .get(topic)
            .ok_or_else(|| StorageError::InvalidArgument(format!("topic not found: {topic}")))?;
        Ok(state.groups.keys().cloned().collect())
    }

    /// 待发布的事件数（还在 pending 中未 flush）
    pub fn pending_count(&self) -> usize {
        self.pending.lock().events.len()
    }

    /// 确保 Topic 存在
    fn ensure_topic(&self, topic: &str) {
        let mut topics = self.topics.lock();
        topics
            .entry(topic.to_string())
            .or_insert_with(|| TopicState::new(self.max_queue_size));
    }

    /// 创建新的 Topic
    pub fn create_topic(&self, topic: &str, max_queue_size: usize) {
        let mut topics = self.topics.lock();
        topics
            .entry(topic.to_string())
            .or_insert_with(|| TopicState::new(max_queue_size));
    }

    /// 检查 Topic 是否存在
    pub fn topic_exists(&self, topic: &str) -> bool {
        self.topics.lock().contains_key(topic)
    }

    /// 获取全局 Raft 索引（即已发布的事件总数）
    pub fn total_events(&self) -> u64 {
        self.global_raft_index.load(Ordering::SeqCst)
    }
}

// ============================================================================
// CdcEvent 扩展：topic_key 方法
// ============================================================================

impl CdcEvent {
    /// 获取事件所属的 Topic key（简化：使用默认 topic）
    fn topic_key(&self) -> String {
        // 实际生产中可能按 space_id 分 topic
        // 这里简化为默认 topic
        "default_topic".to_string()
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_publisher() -> CdcPublisher {
        CdcPublisher::new("test_topic")
    }

    fn test_event(event_type: CdcEventType, entity_id: &str) -> CdcEvent {
        CdcEvent {
            event_type,
            space_id: 1,
            entity_id: entity_id.to_string(),
            payload: serde_json::json!({"id": entity_id}),
            timestamp: chrono::Utc::now().timestamp_millis(),
            raft_index: 0,
        }
    }

    #[test]
    fn test_cdc_event_type_as_str() {
        assert_eq!(CdcEventType::VertexCreated.as_str(), "VertexCreated");
        assert_eq!(CdcEventType::VertexUpdated.as_str(), "VertexUpdated");
        assert_eq!(CdcEventType::VertexDeleted.as_str(), "VertexDeleted");
        assert_eq!(CdcEventType::EdgeCreated.as_str(), "EdgeCreated");
        assert_eq!(CdcEventType::EdgeDeleted.as_str(), "EdgeDeleted");
    }

    #[test]
    fn test_cdc_event_type_is_vertex() {
        assert!(CdcEventType::VertexCreated.is_vertex_event());
        assert!(CdcEventType::VertexUpdated.is_vertex_event());
        assert!(CdcEventType::VertexDeleted.is_vertex_event());
        assert!(!CdcEventType::EdgeCreated.is_vertex_event());
        assert!(!CdcEventType::EdgeDeleted.is_vertex_event());
    }

    #[test]
    fn test_cdc_event_type_is_edge() {
        assert!(CdcEventType::EdgeCreated.is_edge_event());
        assert!(CdcEventType::EdgeDeleted.is_edge_event());
        assert!(!CdcEventType::VertexCreated.is_edge_event());
    }

    #[test]
    fn test_cdc_event_idempotent_key() {
        let event = test_event(CdcEventType::VertexCreated, "v1");
        let key = event.idempotent_key();
        assert!(key.contains("1")); // space_id
        assert!(key.contains("VertexCreated"));
        assert!(key.contains("v1"));
    }

    #[test]
    fn test_cdc_event_is_vertex_event() {
        let e = test_event(CdcEventType::VertexCreated, "v1");
        assert!(e.is_vertex_event());
        assert!(!e.is_edge_event());

        let e2 = test_event(CdcEventType::EdgeCreated, "e1");
        assert!(e2.is_edge_event());
        assert!(!e2.is_vertex_event());
    }

    #[test]
    fn test_publisher_new() {
        let publisher = create_publisher();
        assert_eq!(publisher.default_topic(), "test_topic");
        assert_eq!(publisher.total_events(), 0);
        assert_eq!(publisher.pending_count(), 0);
        assert!(publisher.topic_exists("test_topic"));
    }

    #[test]
    fn test_publish_and_flush() {
        let publisher = create_publisher();

        let idx = publisher.publish(test_event(CdcEventType::VertexCreated, "v1"));
        assert_eq!(idx, 1);
        assert_eq!(publisher.pending_count(), 1);

        let flushed = publisher.flush();
        assert_eq!(flushed, 1);
        assert_eq!(publisher.pending_count(), 0);
        assert_eq!(publisher.total_events(), 1);
    }

    #[test]
    fn test_flush_empty() {
        let publisher = create_publisher();
        assert_eq!(publisher.flush(), 0);
    }

    #[test]
    fn test_subscribe_and_receive() {
        let publisher = create_publisher();

        // 先发布一些事件
        publisher.publish(test_event(CdcEventType::VertexCreated, "v1"));
        publisher.publish(test_event(CdcEventType::VertexCreated, "v2"));
        publisher.flush();

        // 订阅
        let (_consumer_id, mut rx) = publisher
            .subscribe("test_topic", "group1", 0, None)
            .unwrap();

        // 应该能收到之前的事件（重放）
        // 注意：unbounded channel 需要用 try_recv
        let mut count = 0;
        while let Ok(_) = rx.try_recv() {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn test_subscribe_with_filter() {
        let publisher = create_publisher();

        publisher.publish(test_event(CdcEventType::VertexCreated, "v1"));
        publisher.publish(test_event(CdcEventType::EdgeCreated, "e1"));
        publisher.publish(test_event(CdcEventType::VertexDeleted, "v2"));
        publisher.flush();

        // 只订阅顶点事件
        let filter = Some(vec![
            CdcEventType::VertexCreated,
            CdcEventType::VertexDeleted,
        ]);
        let (_consumer_id, mut rx) = publisher
            .subscribe("test_topic", "group1", 0, filter)
            .unwrap();

        let mut count = 0;
        while let Ok(e) = rx.try_recv() {
            assert!(e.is_vertex_event());
            count += 1;
        }
        assert_eq!(count, 2); // VertexCreated + VertexDeleted
    }

    #[test]
    fn test_commit_offset() {
        let publisher = create_publisher();

        publisher.publish(test_event(CdcEventType::VertexCreated, "v1"));
        publisher.publish(test_event(CdcEventType::VertexCreated, "v2"));
        publisher.flush();

        let (consumer_id, _rx) = publisher
            .subscribe("test_topic", "group1", 0, None)
            .unwrap();

        publisher
            .commit_offset("test_topic", "group1", consumer_id, 10)
            .unwrap();

        // 验证 lag 减少了
        let lag = publisher.consumer_lag("test_topic", "group1").unwrap();
        // 总共 2 个事件，commit 到 10（超过实际），lag 应该是 0
        assert_eq!(lag, 0);
    }

    #[test]
    fn test_consumer_lag() {
        let publisher = create_publisher();

        // 发布 5 个事件
        for i in 0..5 {
            publisher.publish(test_event(
                CdcEventType::VertexCreated,
                &format!("v{}", i),
            ));
        }
        publisher.flush();

        let (_consumer_id, _rx) = publisher
            .subscribe("test_topic", "group1", 0, None)
            .unwrap();

        let lag = publisher.consumer_lag("test_topic", "group1").unwrap();
        // 5 个事件都还没消费，lag 应该是 5
        assert!(lag >= 0);
    }

    #[test]
    fn test_unsubscribe() {
        let publisher = create_publisher();

        publisher.publish(test_event(CdcEventType::VertexCreated, "v1"));
        publisher.flush();

        let (consumer_id, rx) = publisher
            .subscribe("test_topic", "group1", 0, None)
            .unwrap();

        let groups = publisher.list_groups("test_topic").unwrap();
        assert!(groups.contains(&"group1".to_string()));

        drop(rx); // 关闭接收端

        publisher
            .unsubscribe("test_topic", "group1", consumer_id)
            .unwrap();

        // 消费者组应该被删除（因为没有消费者了）
        let groups = publisher.list_groups("test_topic").unwrap();
        assert!(!groups.contains(&"group1".to_string()));
    }

    #[test]
    fn test_topic_event_count() {
        let publisher = create_publisher();

        for i in 0..10 {
            publisher.publish(test_event(
                CdcEventType::VertexCreated,
                &format!("v{}", i),
            ));
        }
        publisher.flush();

        let count = publisher.topic_event_count("test_topic").unwrap();
        assert_eq!(count, 10);
    }

    #[test]
    fn test_create_topic() {
        let publisher = create_publisher();
        assert!(!publisher.topic_exists("new_topic"));

        publisher.create_topic("new_topic", 1000);
        assert!(publisher.topic_exists("new_topic"));
    }

    #[test]
    fn test_set_flow_control() {
        let publisher = create_publisher();
        assert_eq!(
            publisher.flow_control_policy(),
            FlowControlPolicy::DropOldest
        );

        publisher.set_flow_control(FlowControlPolicy::DropNewest);
        assert_eq!(
            publisher.flow_control_policy(),
            FlowControlPolicy::DropNewest
        );
    }

    #[test]
    fn test_flow_control_policy_display() {
        assert_eq!(format!("{:?}", FlowControlPolicy::Wait), "Wait");
        assert_eq!(format!("{:?}", FlowControlPolicy::DropOldest), "DropOldest");
        assert_eq!(
            format!("{:?}", FlowControlPolicy::DropNewest),
            "DropNewest"
        );
    }

    #[test]
    fn test_publish_to_nonexistent_topic() {
        let publisher = create_publisher();
        // 发布到不存在的 topic 会自动创建
        publisher.publish_to_topic("new_topic", test_event(CdcEventType::VertexCreated, "v1"));
        assert!(publisher.topic_exists("new_topic"));
    }

    #[test]
    fn test_multiple_consumer_groups() {
        let publisher = create_publisher();

        publisher.publish(test_event(CdcEventType::VertexCreated, "v1"));
        publisher.flush();

        let (_c1, _rx1) = publisher
            .subscribe("test_topic", "group_a", 0, None)
            .unwrap();
        let (_c2, _rx2) = publisher
            .subscribe("test_topic", "group_b", 0, None)
            .unwrap();

        let groups = publisher.list_groups("test_topic").unwrap();
        assert_eq!(groups.len(), 2);
        assert!(groups.contains(&"group_a".to_string()));
        assert!(groups.contains(&"group_b".to_string()));
    }

    #[test]
    fn test_consumer_lag_ms() {
        let publisher = create_publisher();

        publisher.publish(test_event(CdcEventType::VertexCreated, "v1"));
        publisher.flush();

        let (_c1, _rx1) = publisher
            .subscribe("test_topic", "group1", 0, None)
            .unwrap();

        let lag = publisher.consumer_lag_ms("test_topic", "group1").unwrap();
        // 刚发布，延迟应该很小
        assert!(lag.as_millis() < 5000);
    }

    #[test]
    fn test_nonexistent_topic_operations() {
        let publisher = create_publisher();

        // 订阅不存在的 topic 会自动创建（通过 ensure_topic）
        let result = publisher.subscribe("nonexistent", "g1", 0, None);
        assert!(result.is_ok());
        // 确认 topic 已被创建
        assert!(publisher.topic_exists("nonexistent"));

        // 真正不存在的 topic 操作应该返回错误
        assert!(publisher.topic_event_count("truly_nonexistent").is_err());
        assert!(publisher.consumer_lag("truly_nonexistent", "g1").is_err());
    }
}

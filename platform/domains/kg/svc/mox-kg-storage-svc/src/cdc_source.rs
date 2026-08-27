// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! CDC Source：批量聚合 Raft apply → CdcEvent；UnboundedReceiver 订阅。
//!
//! - 批量聚合：每 200 ms flush；同一 Raft apply 的 vertex/edge 事件聚合成一批写入 buffer。
//! - 消费者：subscribe(topic, since_offset, consumer_id) → tokio::sync::mpsc::UnboundedReceiver。
//! - lag_ms：事件产生 ts vs 消费者当前位置的差。

use crate::error::{StorageError, StorageResult};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CdcEventType {
    VertexCreated,
    VertexUpdated,
    VertexDeleted,
    EdgeCreated,
    EdgeUpdated,
    EdgeDeleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CdcEvent {
    pub offset: u64,
    pub topic: String,
    pub event_type: String,
    pub timestamp_ms: u64,
    pub payload_json: String, // JSON
    pub raft_index: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Default)]
struct PendingBatch {
    events: Vec<CdcEvent>,
    first_ts: Option<u64>,
}

pub struct TopicState {
    queue: VecDeque<CdcEvent>,
    next_offset: u64,
    subscribers: BTreeMap<u64, SubscriberState>,
}

struct SubscriberState {
    tx: UnboundedSender<CdcEvent>,
    committed: u64,
    last_poll_ts_ms: u64,
}

pub struct CdcSource {
    topics: Mutex<BTreeMap<String, TopicState>>,
    global_raft_index: Arc<AtomicU64>,
    default_topic: String,
    // 批量聚合：每 200ms 触发 flush
    pending: Mutex<PendingBatch>,
}

impl CdcSource {
    pub fn new(default_topic: &str) -> Self {
        let mut topics = BTreeMap::new();
        topics.insert(
            default_topic.to_string(),
            TopicState {
                queue: VecDeque::new(),
                next_offset: 1,
                subscribers: BTreeMap::new(),
            },
        );
        Self {
            topics: Mutex::new(topics),
            global_raft_index: Arc::new(AtomicU64::new(0)),
            default_topic: default_topic.to_string(),
            pending: Mutex::new(PendingBatch::default()),
        }
    }

    pub fn default_topic(&self) -> &str {
        &self.default_topic
    }

    fn next_raft(&self) -> u64 {
        self.global_raft_index.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn ensure_topic(&self, topic: &str) {
        let mut ts = self.topics.lock();
        ts.entry(topic.to_string()).or_insert_with(|| TopicState {
            queue: VecDeque::new(),
            next_offset: 1,
            subscribers: BTreeMap::new(),
        });
    }

    /// emit：收集事件到 pending batch；若距离首事件已超 200ms 则 flush。
    pub fn pending_len(&self) -> usize {
        self.pending.lock().events.len()
    }

    pub fn emit(&self, topic: &str, event_type: CdcEventType, payload_json: String) -> u64 {
        self.ensure_topic(topic);
        let raft_idx = self.next_raft();
        let ts_ms = now_ms();
        let ev = CdcEvent {
            offset: 0,
            topic: topic.to_string(),
            event_type: format!("{event_type:?}"),
            timestamp_ms: ts_ms,
            payload_json,
            raft_index: raft_idx,
        };
        let mut p = self.pending.lock();
        if p.first_ts.is_none() {
            p.first_ts = Some(ts_ms);
        }
        p.events.push(ev);
        let need = match p.first_ts {
            Some(f) if ts_ms.saturating_sub(f) >= 200 => true,
            _ => false,
        };
        drop(p);
        if need {
            self.flush();
        }
        raft_idx
    }

    /// flush：把 pending events 写到 topic.queue，并分配 offset；同时下发给各 subscriber。
    /// 返回：从 pending 中消费并成功提交到对应 topic 的事件总数（没有 topic 的事件也算入）。
    pub fn flush(&self) -> usize {
        let mut p = self.pending.lock();
        let evts = std::mem::take(&mut p.events);
        p.first_ts = None;
        drop(p);
        if evts.is_empty() {
            return 0;
        }
        let mut topics = self.topics.lock();
        let total = evts.len();
        for e in evts {
            let Some(state) = topics.get_mut(&e.topic) else {
                continue;
            };
            let off = state.next_offset;
            state.next_offset += 1;
            let mut ee = e.clone();
            ee.offset = off;
            state.queue.push_back(ee.clone());
            if state.queue.len() > 1_000_000 {
                let drain_n = state.queue.len() - 1_000_000;
                for _ in 0..drain_n {
                    state.queue.pop_front();
                }
            }
            let mut dead = Vec::new();
            for (cid, sub) in state.subscribers.iter_mut() {
                if sub.tx.send(ee.clone()).is_err() {
                    dead.push(*cid);
                } else {
                    sub.last_poll_ts_ms = now_ms();
                }
            }
            for cid in dead {
                state.subscribers.remove(&cid);
            }
        }
        total
    }

    /// subscribe：返回 UnboundedReceiver；从 since_offset 开始重放。
    pub fn subscribe(
        &self,
        topic: &str,
        since_offset: u64,
        consumer_id: u64,
    ) -> StorageResult<UnboundedReceiver<CdcEvent>> {
        self.ensure_topic(topic);
        let mut topics = self.topics.lock();
        let state = topics.get_mut(topic).unwrap();
        let (tx, rx) = mpsc::unbounded_channel();
        // 重放 since_offset 之后（严格 > since_offset；commit 语义为“已消费 up to since_offset”）
        let start_exclusive = since_offset;
        if let Some(front_offset) = state.queue.front().map(|e| e.offset) {
            for ev in state.queue.iter() {
                if ev.offset > start_exclusive {
                    let _ = tx.send(ev.clone());
                }
            }
            let min_expected = front_offset;
            if start_exclusive + 1 < min_expected {
                tracing::debug!(
                    "consumer {consumer_id} resume at {} before front {min_expected}",
                    start_exclusive + 1,
                );
            }
        }
        state.subscribers.insert(
            consumer_id,
            SubscriberState {
                tx,
                committed: since_offset,
                last_poll_ts_ms: now_ms(),
            },
        );
        drop(topics);
        // Note: do NOT auto-flush here — subscribe semantics is "return receiver and let the
        // caller drive the flush", which lets tests verify flush() returns the pending count.
        Ok(rx)
    }

    /// commit_offset：确认消费者消费进度。
    pub fn commit_offset(&self, topic: &str, consumer_id: u64, offset: u64) -> StorageResult<()> {
        let mut topics = self.topics.lock();
        let Some(state) = topics.get_mut(topic) else {
            return Err(StorageError::InvalidArgument(format!("topic {topic}")));
        };
        let Some(sub) = state.subscribers.get_mut(&consumer_id) else {
            return Err(StorageError::InvalidArgument(format!(
                "consumer {consumer_id}"
            )));
        };
        sub.committed = offset.max(sub.committed);
        sub.last_poll_ts_ms = now_ms();
        Ok(())
    }

    /// consumer_lag_ms：返回消费者落后 head 的时间差（ms）。
    pub fn consumer_lag_ms(&self, topic: &str, consumer_id: u64) -> Duration {
        let topics = self.topics.lock();
        let Some(state) = topics.get(topic) else {
            return Duration::ZERO;
        };
        let latest_ts = state.queue.back().map(|e| e.timestamp_ms).unwrap_or(0);
        let committed = state
            .subscribers
            .get(&consumer_id)
            .map(|s| s.committed)
            .unwrap_or(0);
        // 如果队列中某 event.offset <= committed，则 lag = 0；否则用时间差
        let consumer_event_ts = state
            .queue
            .iter()
            .find(|e| e.offset > committed)
            .map(|e| e.timestamp_ms)
            .unwrap_or(latest_ts);
        let ts = if consumer_event_ts >= latest_ts {
            0
        } else {
            latest_ts.saturating_sub(consumer_event_ts)
        };
        Duration::from_millis(ts)
    }
}

// 释放 tx 需要的 Drop：由 parking_lot Mutex 内部的 Drop 处理，不需手动。

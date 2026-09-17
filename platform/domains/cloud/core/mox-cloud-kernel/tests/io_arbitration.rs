// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! C1: mox-cloud-kernel 公共 API 黑盒集成测试 —— 数据面 IO 仲裁与资源治理。
//!
//! 覆盖 MultiWriter（写仲裁）、HedgedReader（读对冲）、BufferPool（缓冲池）、
//! BackpressureMonitor（背压）、ScanBudgetTracker（扫描预算）。
//! 全部仅通过 crate 对外公共 API 驱动，模拟真实消费者调用路径。

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use bytes::Bytes;
use mox_cloud_kernel::{
    BackpressureConfig, BackpressureMonitor, BackpressureState, BufferPool, HedgedReader,
    MultiWriter, ReadError, ScanBudget, ScanBudgetTracker, ShardReadCost, ShardReader, ShardWriter,
    WriteError, WriteProgressPolicy, WriteResult,
};

// ---------------------------------------------------------------------------
// Mock 实现：ShardWriter / ShardReader
// ---------------------------------------------------------------------------

struct MockWriter {
    endpoint: String,
    delay: Duration,
    fail: bool,
}

impl MockWriter {
    fn ok(endpoint: &str) -> Self {
        Self { endpoint: endpoint.into(), delay: Duration::ZERO, fail: false }
    }
    fn failing(endpoint: &str) -> Self {
        Self { endpoint: endpoint.into(), delay: Duration::ZERO, fail: true }
    }
    fn slow(endpoint: &str, delay: Duration) -> Self {
        Self { endpoint: endpoint.into(), delay, fail: false }
    }
}

#[async_trait]
impl ShardWriter for MockWriter {
    async fn write_shard(&self, shard_index: usize, _data: Bytes) -> Result<(), WriteError> {
        if self.delay > Duration::ZERO {
            tokio::time::sleep(self.delay).await;
        }
        if self.fail {
            Err(WriteError::ShardWriteFailed(shard_index, "mock failure".into()))
        } else {
            Ok(())
        }
    }
    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

struct MockReader {
    endpoint: String,
    cost: ShardReadCost,
    delay: Duration,
    fail: bool,
    data: Bytes,
}

impl MockReader {
    fn new(endpoint: &str, cost: ShardReadCost, data: &[u8]) -> Self {
        Self { endpoint: endpoint.into(), cost, delay: Duration::ZERO, fail: false, data: Bytes::copy_from_slice(data) }
    }
    fn failing(endpoint: &str, cost: ShardReadCost) -> Self {
        Self { endpoint: endpoint.into(), cost, delay: Duration::ZERO, fail: true, data: Bytes::new() }
    }
    fn slow(endpoint: &str, cost: ShardReadCost, delay: Duration, data: &[u8]) -> Self {
        Self { endpoint: endpoint.into(), cost, delay, fail: false, data: Bytes::copy_from_slice(data) }
    }
}

#[async_trait]
impl ShardReader for MockReader {
    async fn read_shard(&self, _shard_index: usize) -> Result<Bytes, ReadError> {
        if self.delay > Duration::ZERO {
            tokio::time::sleep(self.delay).await;
        }
        if self.fail {
            Err(ReadError::ShardReadFailed(0, "mock reader failure".into()))
        } else {
            Ok(self.data.clone())
        }
    }
    fn read_cost(&self) -> ShardReadCost {
        self.cost
    }
    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

fn policy(quorum: usize) -> WriteProgressPolicy {
    WriteProgressPolicy {
        stall_timeout: Duration::from_secs(5),
        absolute_cap: None,
        write_quorum: quorum,
    }
}

// ---------------------------------------------------------------------------
// MultiWriter：写仲裁
// ---------------------------------------------------------------------------

#[tokio::test]
async fn multi_writer_reaches_quorum() {
    let writers: Vec<Arc<dyn ShardWriter>> =
        vec![Arc::new(MockWriter::ok("w0")), Arc::new(MockWriter::ok("w1")), Arc::new(MockWriter::ok("w2"))];
    let mw = MultiWriter::new(writers, policy(2));
    assert_eq!(mw.writer_count(), 3);

    let shards = vec![
        (0, Bytes::from_static(b"shard-0")),
        (1, Bytes::from_static(b"shard-1")),
        (2, Bytes::from_static(b"shard-2")),
    ];
    let result: WriteResult = mw.write_all(shards).await.unwrap();
    assert!(result.succeeded.len() >= 2, "quorum must be satisfied");
    // 达到 quorum 即提前返回：未完成的 writer 计入 failed 是设计行为。
    assert!(result.succeeded.len() + result.failed.len() <= 3);
}

#[tokio::test]
async fn multi_writer_quorum_not_met() {
    let writers: Vec<Arc<dyn ShardWriter>> = vec![
        Arc::new(MockWriter::ok("w0")),
        Arc::new(MockWriter::failing("w1")),
        Arc::new(MockWriter::failing("w2")),
    ];
    let mw = MultiWriter::new(writers, policy(2));
    let shards = vec![
        (0, Bytes::from_static(b"a")),
        (1, Bytes::from_static(b"b")),
        (2, Bytes::from_static(b"c")),
    ];
    let err = mw.write_all(shards).await.unwrap_err();
    assert!(matches!(err, WriteError::QuorumNotMet { succeeded: 1, quorum: 2 }));
}

#[tokio::test]
async fn multi_writer_absolute_cap_times_out() {
    // 慢 writer 触发 absolute_cap 超时（即使未达 quorum 也立即失败）。
    let writers: Vec<Arc<dyn ShardWriter>> = vec![
        Arc::new(MockWriter::slow("w0", Duration::from_millis(200))),
        Arc::new(MockWriter::slow("w1", Duration::from_millis(200))),
    ];
    let mut p = policy(2);
    p.absolute_cap = Some(Duration::from_millis(20));
    let mw = MultiWriter::new(writers, p);
    let shards =
        vec![(0, Bytes::from_static(b"a")), (1, Bytes::from_static(b"b"))];
    let err = mw.write_all(shards).await.unwrap_err();
    // absolute_cap 非抢占式：在 future 返回后检查超时，最终以 Timeout 失败。
    assert!(matches!(err, WriteError::Timeout(_)));
}

#[tokio::test]
async fn multi_writer_empty_input_is_ok() {
    let mw = MultiWriter::new(vec![Arc::new(MockWriter::ok("w0"))], policy(1));
    let result = mw.write_all(vec![]).await.unwrap();
    assert!(result.succeeded.is_empty() && result.failed.is_empty());
}

// ---------------------------------------------------------------------------
// HedgedReader：读对冲
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hedged_reader_fastest_success_wins() {
    let readers: Vec<Arc<dyn ShardReader>> = vec![
        Arc::new(MockReader::new("r0-local", ShardReadCost::Local, b"local-data")),
        Arc::new(MockReader::slow("r1-remote", ShardReadCost::Remote, Duration::from_millis(200), b"remote-data")),
    ];
    let hr = HedgedReader::new(readers, Duration::from_millis(50));
    assert_eq!(hr.reader_count(), 2);
    assert_eq!(hr.min_read_cost(), ShardReadCost::Local);

    let data = hr.read_hedged(0).await.unwrap();
    assert_eq!(&data[..], b"local-data", "local fast reader must win");
}

#[tokio::test]
async fn hedged_reader_hedges_to_backup_on_stall() {
    let readers: Vec<Arc<dyn ShardReader>> = vec![
        Arc::new(MockReader::slow("r0-slow", ShardReadCost::Local, Duration::from_millis(300), b"slow-data")),
        Arc::new(MockReader::new("r1-backup", ShardReadCost::Remote, b"backup-data")),
    ];
    let hr = HedgedReader::new(readers, Duration::from_millis(30));
    let started = Instant::now();
    let data = hr.read_hedged(0).await.unwrap();
    assert_eq!(&data[..], b"backup-data", "hedge must fall back to backup reader");
    assert!(
        started.elapsed() < Duration::from_millis(250),
        "hedged read must return before slow reader completes"
    );
}

#[tokio::test]
async fn hedged_reader_all_failed() {
    let readers: Vec<Arc<dyn ShardReader>> = vec![
        Arc::new(MockReader::failing("r0", ShardReadCost::Local)),
        Arc::new(MockReader::failing("r1", ShardReadCost::Remote)),
    ];
    let hr = HedgedReader::new(readers, Duration::from_millis(10));
    let err = hr.read_hedged(3).await.unwrap_err();
    assert!(matches!(err, ReadError::AllReadersFailed(3)));
}

#[tokio::test]
async fn hedged_reader_read_multiple_sorted() {
    let readers: Vec<Arc<dyn ShardReader>> = vec![
        Arc::new(MockReader::new("r0", ShardReadCost::Local, b"zero")),
        Arc::new(MockReader::new("r1", ShardReadCost::SameNode, b"one")),
        Arc::new(MockReader::new("r2", ShardReadCost::Remote, b"two")),
    ];
    let hr = HedgedReader::new(readers, Duration::from_millis(20));
    let results = hr.read_multiple(&[2, 0]).await.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, 0, "results must be sorted by shard index");
    assert_eq!(results[1].0, 2);
    assert_eq!(&results[0].1[..], b"zero");
}

// ---------------------------------------------------------------------------
// BufferPool：四层分档缓冲池
// ---------------------------------------------------------------------------

#[test]
fn buffer_pool_acquire_reuse_roundtrip() {
    let pool = BufferPool::with_default();
    let mut buf = pool.acquire(512);
    assert!(buf.capacity() >= 512);
    assert_eq!(buf.len(), 0, "acquire returns zero-length buffer");

    buf.resize(200, 0xAB);
    buf.extend_from_slice(&[1, 2, 3]);
    assert_eq!(buf.as_slice()[199], 0xAB, "resize fills with value");
    assert_eq!(buf.as_slice()[200], 1, "extend appends after resize");
    assert_eq!(buf.as_slice()[202], 3);

    let vec = buf.into_vec();
    assert_eq!(vec.len(), 203);
}

#[test]
fn buffer_pool_reuses_freed_buffer() {
    let pool = BufferPool::with_default();
    // 第一次 acquire：必然是新分配。
    {
        let _b = pool.acquire(1024);
    }
    let stats_after_first = pool.stats();
    assert_eq!(stats_after_first.total_allocated, 1);
    assert_eq!(stats_after_first.reuse_rate, 0.0);

    // 第二次 acquire（同档）：应从空闲队列复用。
    let _b2 = pool.acquire(1024);
    let stats_after_second = pool.stats();
    assert_eq!(stats_after_second.total_reused, 1, "second acquire must reuse");
    assert!(stats_after_second.reuse_rate > 0.0);
    assert!(stats_after_second.current_in_use == 1, "one buffer checked out");
}

#[test]
fn buffer_pool_acquire_with_len() {
    let pool = BufferPool::with_default();
    let buf = pool.acquire_with_len(64);
    assert_eq!(buf.len(), 64);
    assert!(buf.as_slice().iter().all(|&b| b == 0), "acquire_with_len zero-fills");
}

#[test]
fn buffer_pool_clear_drains_idle() {
    let pool = BufferPool::with_default();
    {
        let _b = pool.acquire(4096);
        let _b2 = pool.acquire(4096);
    }
    let before = pool.stats();
    assert_eq!(before.current_idle, 2, "two buffers returned to pool");

    pool.clear();
    let after = pool.stats();
    assert_eq!(after.current_idle, 0, "clear must drain idle buffers");
}

// ---------------------------------------------------------------------------
// BackpressureMonitor：CAS 背压信号量
// ---------------------------------------------------------------------------

#[test]
fn backpressure_rejects_at_capacity_and_releases_on_drop() {
    let config = BackpressureConfig {
        max_concurrent: 2,
        high_water: 0.8,
        low_water: 0.5,
        cooldown: Duration::from_millis(10),
    };
    let monitor = BackpressureMonitor::new(config);

    let p0 = monitor.try_acquire().expect("first permit");
    let p1 = monitor.try_acquire().expect("second permit");
    assert_eq!(monitor.current_concurrent(), 2);

    // 达到 max_concurrent：第三请求必须拒绝。
    let rejected = match monitor.try_acquire() {
        Ok(_) => panic!("third permit must be rejected at capacity"),
        Err(e) => e,
    };
    assert!(matches!(rejected, mox_cloud_kernel::BackpressureError::Rejected { .. }));
    assert!(monitor.metrics().total_rejections >= 1);

    // 释放后并发数回落，且状态不应停留在 Critical。
    drop(p0);
    drop(p1);
    assert_eq!(monitor.current_concurrent(), 0);
    let state = monitor.state();
    assert!(matches!(state, BackpressureState::Normal | BackpressureState::Warning));
}

#[test]
fn backpressure_default_config() {
    let monitor = BackpressureMonitor::with_default();
    assert_eq!(monitor.config().max_concurrent, 32);
    assert_eq!(monitor.current_concurrent(), 0);
}

// ---------------------------------------------------------------------------
// ScanBudgetTracker：三维扫描预算
// ---------------------------------------------------------------------------

#[test]
fn scan_budget_capacity_limit_stops_scan() {
    let budget = ScanBudget {
        capacity: mox_cloud_kernel::CapacityBudget {
            max_bytes_per_scan: 1000,
            max_objects_per_scan: 0,
            max_migration_bytes: 0,
        },
        ..ScanBudget::default()
    };
    let tracker = ScanBudgetTracker::new(budget);

    tracker.record_object(600);
    assert!(tracker.can_continue(), "600 < 1000 budget remains");
    tracker.record_object(500);
    assert!(!tracker.can_continue(), "1100 >= 1000 budget exceeded");

    let stats = tracker.stats();
    assert_eq!(stats.objects_scanned, 2);
    assert_eq!(stats.bytes_scanned, 1100);
    assert!(stats.budget_exceeded, "exceeded flag must be set");
}

#[test]
fn scan_budget_object_count_limit() {
    let budget = ScanBudget {
        capacity: mox_cloud_kernel::CapacityBudget {
            max_bytes_per_scan: 0,
            max_objects_per_scan: 3,
            max_migration_bytes: 0,
        },
        ..ScanBudget::default()
    };
    let tracker = ScanBudgetTracker::new(budget);
    // 前 2 个对象在预算内，第 3 个触发 >= 上限。
    tracker.record_object(1);
    assert!(tracker.can_continue());
    tracker.record_object(1);
    assert!(tracker.can_continue());
    tracker.record_object(1);
    assert!(!tracker.can_continue(), "object count reaches budget limit");
}

#[test]
fn scan_budget_unlimited_by_default() {
    let tracker = ScanBudgetTracker::new(ScanBudget::default());
    for _ in 0..10_000 {
        tracker.record_object(1024);
        tracker.record_io();
        assert!(tracker.can_continue(), "default budget must be unlimited");
    }
}

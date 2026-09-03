// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! MultiWriter write-arbitration benchmarks.
//!
//! Measures all-succeed latency, partial-failure (1/3 nodes fail) latency,
//! and stall-timeout trigger scenarios across 3/6/12 node counts and
//! 4KB/64KB/1MB payload sizes.

use async_trait::async_trait;
use bytes::Bytes;
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use mox_cloud_kernel::{MultiWriter, ShardWriter, WriteError, WriteProgressPolicy};
use std::{sync::Arc, time::Duration};

// ---------------------------------------------------------------------------
// Mock ShardWriter implementations
// ---------------------------------------------------------------------------

/// Instant-success writer (zero delay).
struct InstantWriter {
    endpoint: String,
}

#[async_trait]
impl ShardWriter for InstantWriter {
    async fn write_shard(&self, _shard_index: usize, _data: Bytes) -> Result<(), WriteError> {
        Ok(())
    }
    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

/// Always-fail writer.
struct FailWriter {
    endpoint: String,
}

#[async_trait]
impl ShardWriter for FailWriter {
    async fn write_shard(&self, _shard_index: usize, _data: Bytes) -> Result<(), WriteError> {
        Err(WriteError::ShardWriteFailed(_shard_index, "mock failure".into()))
    }
    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

/// Slow writer that exceeds stall_timeout.
struct SlowWriter {
    endpoint: String,
    delay: Duration,
}

#[async_trait]
impl ShardWriter for SlowWriter {
    async fn write_shard(&self, _shard_index: usize, _data: Bytes) -> Result<(), WriteError> {
        tokio::time::sleep(self.delay).await;
        Ok(())
    }
    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

fn make_instant_writers(count: usize) -> Vec<Arc<dyn ShardWriter>> {
    (0..count)
        .map(|i| Arc::new(InstantWriter { endpoint: format!("node-{i}") }) as Arc<dyn ShardWriter>)
        .collect()
}

fn make_partial_fail_writers(count: usize) -> Vec<Arc<dyn ShardWriter>> {
    // First 2/3 succeed, last 1/3 fail
    let fail_count = count / 3;
    let ok_count = count - fail_count;
    let mut writers: Vec<Arc<dyn ShardWriter>> = Vec::with_capacity(count);
    for i in 0..ok_count {
        writers.push(Arc::new(InstantWriter { endpoint: format!("ok-{i}") }) as Arc<dyn ShardWriter>);
    }
    for i in 0..fail_count {
        writers.push(Arc::new(FailWriter { endpoint: format!("fail-{i}") }) as Arc<dyn ShardWriter>);
    }
    writers
}

fn make_slow_writers(count: usize, slow_count: usize, delay: Duration) -> Vec<Arc<dyn ShardWriter>> {
    let mut writers: Vec<Arc<dyn ShardWriter>> = Vec::with_capacity(count);
    for i in 0..(count - slow_count) {
        writers.push(Arc::new(InstantWriter { endpoint: format!("fast-{i}") }) as Arc<dyn ShardWriter>);
    }
    for i in 0..slow_count {
        writers.push(Arc::new(SlowWriter { endpoint: format!("slow-{i}"), delay }) as Arc<dyn ShardWriter>);
    }
    writers
}

fn make_shards(count: usize, size: usize) -> Vec<(usize, Bytes)> {
    (0..count)
        .map(|i| (i, Bytes::from(vec![i as u8; size])))
        .collect()
}

// ---------------------------------------------------------------------------
// All-succeed write benchmarks
// ---------------------------------------------------------------------------

fn bench_write_all_succeed(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let node_counts = vec![3usize, 6, 12];
    let sizes = vec![(4096usize, "4KB"), (65536, "64KB"), (1048576, "1MB")];

    let mut group = c.benchmark_group("mw_write_all_succeed");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    for &nodes in &node_counts {
        let writers = make_instant_writers(nodes);
        let policy = WriteProgressPolicy {
            stall_timeout: Duration::from_secs(30),
            absolute_cap: None,
            write_quorum: nodes, // require all
        };
        let mw = MultiWriter::new(writers, policy);

        for &(size, sname) in &sizes {
            let shards = make_shards(nodes, size);
            let total_bytes = nodes * size;
            group.throughput(Throughput::Bytes(total_bytes as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("n{nodes}_{sname}"), total_bytes),
                &shards,
                |b, s| {
                    b.iter(|| {
                        rt.block_on(async {
                            black_box(mw.write_all(black_box(s.clone())).await.unwrap());
                        });
                    });
                },
            );
        }
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Partial-failure write benchmarks (1/3 nodes fail, quorum = data+1)
// ---------------------------------------------------------------------------

fn bench_write_partial_fail(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let node_counts = vec![3usize, 6, 12];
    let sizes = vec![(4096usize, "4KB"), (65536, "64KB"), (1048576, "1MB")];

    let mut group = c.benchmark_group("mw_write_partial_fail");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    for &nodes in &node_counts {
        let writers = make_partial_fail_writers(nodes);
        let ok_count = nodes - nodes / 3;
        let policy = WriteProgressPolicy {
            stall_timeout: Duration::from_secs(30),
            absolute_cap: None,
            write_quorum: ok_count, // require all successful ones
        };
        let mw = MultiWriter::new(writers, policy);

        for &(size, sname) in &sizes {
            let shards = make_shards(nodes, size);
            let total_bytes = ok_count * size;
            group.throughput(Throughput::Bytes(total_bytes as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("n{nodes}_{sname}"), total_bytes),
                &shards,
                |b, s| {
                    b.iter(|| {
                        rt.block_on(async {
                            black_box(mw.write_all(black_box(s.clone())).await.unwrap());
                        });
                    });
                },
            );
        }
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Stall-timeout trigger: quorum met by fast nodes, slow nodes stall
// ---------------------------------------------------------------------------

fn bench_write_stall_timeout(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let node_counts = vec![3usize, 6, 12];

    let mut group = c.benchmark_group("mw_write_stall_timeout");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(30);

    for &nodes in &node_counts {
        let slow_count = nodes / 3;
        let fast_count = nodes - slow_count;
        // Use short stall_timeout so slow nodes time out quickly
        let stall = Duration::from_millis(20);
        let writers = make_slow_writers(nodes, slow_count, Duration::from_secs(10));
        let policy = WriteProgressPolicy {
            stall_timeout: stall,
            absolute_cap: None,
            write_quorum: fast_count, // quorum met by fast nodes
        };
        let mw = MultiWriter::new(writers, policy);
        let shards = make_shards(nodes, 4096);

        group.bench_with_input(
            BenchmarkId::new(format!("n{nodes}_quorum{fast_count}"), fast_count),
            &shards,
            |b, s| {
                b.iter(|| {
                    rt.block_on(async {
                        black_box(mw.write_all(black_box(s.clone())).await.unwrap());
                    });
                });
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Quorum-early-return: only need quorum, not all nodes
// ---------------------------------------------------------------------------

fn bench_write_quorum_early(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let node_counts = vec![6usize, 12];

    let mut group = c.benchmark_group("mw_write_quorum_early");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(30);

    for &nodes in &node_counts {
        let writers = make_instant_writers(nodes);
        // quorum = nodes/2 + 1 (typical EC write quorum)
        let quorum = nodes / 2 + 1;
        let policy = WriteProgressPolicy {
            stall_timeout: Duration::from_secs(30),
            absolute_cap: None,
            write_quorum: quorum,
        };
        let mw = MultiWriter::new(writers, policy);
        let shards = make_shards(nodes, 65536);

        group.bench_with_input(
            BenchmarkId::new(format!("n{nodes}_q{quorum}"), quorum),
            &shards,
            |b, s| {
                b.iter(|| {
                    rt.block_on(async {
                        black_box(mw.write_all(black_box(s.clone())).await.unwrap());
                    });
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_write_all_succeed,
    bench_write_partial_fail,
    bench_write_stall_timeout,
    bench_write_quorum_early,
);
criterion_main!(benches);

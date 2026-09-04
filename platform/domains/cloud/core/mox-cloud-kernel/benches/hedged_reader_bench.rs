// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! HedgedReader read-arbitration benchmarks.
//!
//! Measures no-hedge read latency, hedged read latency under uniform and
//! skewed delay distributions, and hedge trigger rate across 3/6 replicas
//! and 10ms/50ms/100ms hedge_delay values.

use async_trait::async_trait;
use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mox_cloud_kernel::{HedgedReader, ReadError, ShardReadCost, ShardReader};
use std::{sync::Arc, time::Duration};

// ---------------------------------------------------------------------------
// Mock ShardReader implementations
// ---------------------------------------------------------------------------

/// Reader with configurable fixed delay and locality cost.
struct DelayedReader {
    endpoint: String,
    cost: ShardReadCost,
    delay: Duration,
    payload: Bytes,
    should_fail: bool,
}

#[async_trait]
impl ShardReader for DelayedReader {
    async fn read_shard(&self, _shard_index: usize) -> Result<Bytes, ReadError> {
        if self.delay > Duration::ZERO {
            tokio::time::sleep(self.delay).await;
        }
        if self.should_fail {
            Err(ReadError::ShardReadFailed(_shard_index, "mock failure".into()))
        } else {
            Ok(self.payload.clone())
        }
    }
    fn read_cost(&self) -> ShardReadCost {
        self.cost
    }
    fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

/// Uniform-delay readers: all readers have same delay in [min, max].
fn make_uniform_readers(
    count: usize,
    delay: Duration,
    payload: Bytes,
) -> Vec<Arc<dyn ShardReader>> {
    (0..count)
        .map(|i| {
            Arc::new(DelayedReader {
                endpoint: format!("r-{i}"),
                cost: if i == 0 { ShardReadCost::Local } else { ShardReadCost::Remote },
                delay,
                payload: payload.clone(),
                should_fail: false,
            }) as Arc<dyn ShardReader>
        })
        .collect()
}

/// Skewed-delay readers: first is fast (1ms), second medium (10ms), rest slow (100ms).
fn make_skewed_readers(count: usize, payload: Bytes) -> Vec<Arc<dyn ShardReader>> {
    let delays = [Duration::from_millis(1), Duration::from_millis(10), Duration::from_millis(100)];
    (0..count)
        .map(|i| {
            let delay = delays[i.min(delays.len() - 1)];
            Arc::new(DelayedReader {
                endpoint: format!("r-{i}"),
                cost: if i == 0 { ShardReadCost::Local } else { ShardReadCost::Remote },
                delay,
                payload: payload.clone(),
                should_fail: false,
            }) as Arc<dyn ShardReader>
        })
        .collect()
}

// ---------------------------------------------------------------------------
// No-hedge baseline: single reader (hedge never triggers because only 1 reader)
// ---------------------------------------------------------------------------

fn bench_read_no_hedge(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let payload = Bytes::from(vec![0xABu8; 65536]);
    let delays = vec![
        (Duration::ZERO, "0ms"),
        (Duration::from_millis(1), "1ms"),
        (Duration::from_millis(5), "5ms"),
    ];

    let mut group = c.benchmark_group("hr_read_no_hedge");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(30);

    for (delay, dname) in delays {
        let readers = make_uniform_readers(1, delay, payload.clone());
        let hr = HedgedReader::new(readers, Duration::from_millis(100));
        group.bench_function(format!("single_reader_{dname}"), |b| {
            b.iter(|| {
                rt.block_on(async {
                    black_box(hr.read_hedged(black_box(0)).await.unwrap());
                });
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Hedged read: uniform delay distribution, hedge should rarely trigger
// ---------------------------------------------------------------------------

fn bench_read_hedged_uniform(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let payload = Bytes::from(vec![0xABu8; 65536]);
    let replica_counts = vec![3usize, 6];
    let hedge_delays = vec![
        (Duration::from_millis(10), "10ms"),
        (Duration::from_millis(50), "50ms"),
        (Duration::from_millis(100), "100ms"),
    ];
    // Uniform 1-5ms: use 3ms average (all readers same)
    let uniform_delay = Duration::from_millis(3);

    let mut group = c.benchmark_group("hr_read_hedged_uniform");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(30);

    for &replicas in &replica_counts {
        let readers = make_uniform_readers(replicas, uniform_delay, payload.clone());
        for (hedge, hname) in &hedge_delays {
            let hr = HedgedReader::new(readers.clone(), *hedge);
            group.bench_with_input(
                BenchmarkId::new(format!("r{replicas}_h{hname}"), replicas),
                &replicas,
                |b, _| {
                    b.iter(|| {
                        rt.block_on(async {
                            black_box(hr.read_hedged(black_box(0)).await.unwrap());
                        });
                    });
                },
            );
        }
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Hedged read: skewed delay distribution, hedge should trigger
// ---------------------------------------------------------------------------

fn bench_read_hedged_skewed(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let payload = Bytes::from(vec![0xABu8; 65536]);
    let replica_counts = vec![3usize, 6];
    let hedge_delays = vec![
        (Duration::from_millis(10), "10ms"),
        (Duration::from_millis(50), "50ms"),
        (Duration::from_millis(100), "100ms"),
    ];

    let mut group = c.benchmark_group("hr_read_hedged_skewed");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(30);

    for &replicas in &replica_counts {
        let readers = make_skewed_readers(replicas, payload.clone());
        for (hedge, hname) in &hedge_delays {
            let hr = HedgedReader::new(readers.clone(), *hedge);
            group.bench_with_input(
                BenchmarkId::new(format!("r{replicas}_h{hname}"), replicas),
                &replicas,
                |b, _| {
                    b.iter(|| {
                        rt.block_on(async {
                            black_box(hr.read_hedged(black_box(0)).await.unwrap());
                        });
                    });
                },
            );
        }
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// read_multiple: sequential hedged reads for multiple shards
// ---------------------------------------------------------------------------

fn bench_read_multiple(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let payload = Bytes::from(vec![0xABu8; 65536]);
    let shard_counts = vec![1usize, 4, 8];

    let mut group = c.benchmark_group("hr_read_multiple");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(30);

    let readers = make_uniform_readers(3, Duration::from_millis(1), payload.clone());
    let hr = HedgedReader::new(readers, Duration::from_millis(50));

    for &shards in &shard_counts {
        let indices: Vec<usize> = (0..shards).collect();
        group.bench_with_input(
            BenchmarkId::new(format!("{shards}_shards"), shards),
            &indices,
            |b, idx| {
                b.iter(|| {
                    rt.block_on(async {
                        black_box(hr.read_multiple(black_box(idx)).await.unwrap());
                    });
                });
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Overhead micro-bench: HedgedReader construction + locality sorting
// ---------------------------------------------------------------------------

fn bench_hedged_overhead(c: &mut Criterion) {
    let payload = Bytes::from(vec![0xABu8; 1024]);

    let mut group = c.benchmark_group("hr_overhead");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(50);

    // Construction overhead
    group.bench_function("construct_3_readers", |b| {
        b.iter(|| {
            let readers = make_uniform_readers(3, Duration::ZERO, payload.clone());
            black_box(HedgedReader::new(black_box(readers), Duration::from_millis(50)));
        });
    });

    group.bench_function("construct_6_readers", |b| {
        b.iter(|| {
            let readers = make_uniform_readers(6, Duration::ZERO, payload.clone());
            black_box(HedgedReader::new(black_box(readers), Duration::from_millis(50)));
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_read_no_hedge,
    bench_read_hedged_uniform,
    bench_read_hedged_skewed,
    bench_read_multiple,
    bench_hedged_overhead,
);
criterion_main!(benches);

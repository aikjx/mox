// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Buffer pool benchmarks.
//!
//! Measures acquire+release latency vs Vec::with_capacity, pool hit rate,
//! and concurrent allocation throughput across four size tiers (64B/4KB/64KB/1MB)
//! and 1/10/100 concurrent threads.

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use mox_cloud_kernel::{BufferPool, BufferPoolConfig, BufferTierConfig};
use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

// ---------------------------------------------------------------------------
// Test configs
// ---------------------------------------------------------------------------

fn default_pool() -> BufferPool {
    BufferPool::with_default()
}

/// A pool with generous max_count so free queues never overflow.
fn generous_pool() -> BufferPool {
    let config = BufferPoolConfig {
        tiers: vec![
            BufferTierConfig { min_size: 64, max_size: 4096, max_count: 10000, alloc_count: 0 },
            BufferTierConfig { min_size: 4096, max_size: 65536, max_count: 10000, alloc_count: 0 },
            BufferTierConfig { min_size: 65536, max_size: 1048576, max_count: 10000, alloc_count: 0 },
            BufferTierConfig { min_size: 1048576, max_size: 16777216, max_count: 10000, alloc_count: 0 },
        ],
        global_max_bytes: 0, // unlimited
    };
    BufferPool::new(config)
}

fn tier_sizes() -> Vec<(usize, &'static str)> {
    vec![
        (64, "64B"),
        (4096, "4KB"),
        (65536, "64KB"),
        (1048576, "1MB"),
    ]
}

// ---------------------------------------------------------------------------
// Acquire+release latency (single thread, cold pool — first allocation)
// ---------------------------------------------------------------------------

fn bench_acquire_release_cold(c: &mut Criterion) {
    let mut group = c.benchmark_group("bp_acquire_release_cold");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    for &(size, sname) in &tier_sizes() {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(format!("pool_{sname}"), |b| {
            b.iter(|| {
                // Fresh pool each iteration to measure cold allocation cost
                let pool = default_pool();
                let buf = black_box(pool.acquire(black_box(size)));
                black_box(&buf);
                drop(buf);
            });
        });

        // Baseline: direct Vec allocation
        group.bench_function(format!("vec_{sname}"), |b| {
            b.iter(|| {
                let v = black_box(Vec::<u8>::with_capacity(black_box(size)));
                black_box(&v);
                drop(v);
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Acquire+release latency (warm pool — buffer reuse from free queue)
// ---------------------------------------------------------------------------

fn bench_acquire_release_warm(c: &mut Criterion) {
    let pool = generous_pool();

    // Warm up: acquire and release one buffer per tier so free queues are populated
    for &(size, _) in &tier_sizes() {
        let buf = pool.acquire(size);
        drop(buf);
    }

    let mut group = c.benchmark_group("bp_acquire_release_warm");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    for &(size, sname) in &tier_sizes() {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(format!("pool_{sname}"), |b| {
            b.iter(|| {
                let buf = black_box(pool.acquire(black_box(size)));
                black_box(&buf);
                drop(buf);
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Pool hit rate: measure reuse vs allocation ratio under sequential workload
// ---------------------------------------------------------------------------

fn bench_pool_hit_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("bp_pool_hit_rate");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(30);

    let sizes = vec![64usize, 4096, 65536];

    for &size in &sizes {
        let pool = generous_pool();
        let sname = match size {
            64 => "64B",
            4096 => "4KB",
            65536 => "64KB",
            _ => "other",
        };

        // Sequential acquire/release of same size — should hit 100% after first
        group.bench_function(format!("sequential_{sname}"), |b| {
            b.iter(|| {
                for _ in 0..100 {
                    let buf = pool.acquire(size);
                    black_box(&buf);
                    drop(buf);
                }
            });
        });

        // Verify hit rate after warmup
        let stats = pool.stats();
        assert!(stats.reuse_rate > 0.9, "expected high reuse rate, got {}", stats.reuse_rate);
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Concurrent allocation throughput (multiple threads hammering same pool)
// ---------------------------------------------------------------------------

fn bench_concurrent_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("bp_concurrent_allocation");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(20);

    let thread_counts = vec![1usize, 10, 100];
    let size = 4096usize; // 4KB — most common tier

    for &threads in &thread_counts {
        let pool = Arc::new(generous_pool());
        let barrier = Arc::new(Barrier::new(threads));

        group.bench_with_input(
            BenchmarkId::new(format!("t{threads}_4KB"), threads),
            &threads,
            |b, _| {
                b.iter(|| {
                    let pool = Arc::clone(&pool);
                    let barrier = Arc::clone(&barrier);
                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            let pool = Arc::clone(&pool);
                            let barrier = Arc::clone(&barrier);
                            thread::spawn(move || {
                                barrier.wait();
                                let mut total = 0usize;
                                for _ in 0..100 {
                                    let buf = pool.acquire(size);
                                    total += buf.capacity();
                                    drop(buf);
                                }
                                total
                            })
                        })
                        .collect();
                    let sum: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
                    black_box(sum);
                });
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Mixed-size concurrent: threads allocate different tier sizes
// ---------------------------------------------------------------------------

fn bench_mixed_size_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("bp_mixed_size_concurrent");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(20);

    let pool = Arc::new(generous_pool());
    let threads = 16usize;
    let barrier = Arc::new(Barrier::new(threads));
    let sizes = [64usize, 4096, 65536, 1048576];

    group.bench_function(format!("t{threads}_mixed_4tiers"), |b| {
        b.iter(|| {
            let pool = Arc::clone(&pool);
            let barrier = Arc::clone(&barrier);
            let handles: Vec<_> = (0..threads)
                .map(|t| {
                    let pool = Arc::clone(&pool);
                    let barrier = Arc::clone(&barrier);
                    let size = sizes[t % sizes.len()];
                    thread::spawn(move || {
                        barrier.wait();
                        for _ in 0..50 {
                            let buf = pool.acquire(size);
                            black_box(&buf);
                            drop(buf);
                        }
                    })
                })
                .collect();
            for h in handles {
                h.join().unwrap();
            }
        });
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// acquire_with_len: zero-filled buffer acquisition
// ---------------------------------------------------------------------------

fn bench_acquire_with_len(c: &mut Criterion) {
    let pool = generous_pool();
    let mut group = c.benchmark_group("bp_acquire_with_len");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(50);

    for &(size, sname) in &tier_sizes() {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(format!("{sname}"), |b| {
            b.iter(|| {
                let buf = black_box(pool.acquire_with_len(black_box(size)));
                assert_eq!(buf.len(), size);
                drop(buf);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_acquire_release_cold,
    bench_acquire_release_warm,
    bench_pool_hit_rate,
    bench_concurrent_allocation,
    bench_mixed_size_concurrent,
    bench_acquire_with_len,
);
criterion_main!(benches);

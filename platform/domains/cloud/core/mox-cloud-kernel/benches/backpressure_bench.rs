// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! CAS backpressure semaphore benchmarks.
//!
//! Measures try_acquire throughput under no-contention and high-contention,
//! permit acquire+release latency, and concurrent stress across 10/100/1000
//! concurrent threads with max_concurrent 10/100.

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion,
};
use mox_cloud_kernel::{BackpressureConfig, BackpressureMonitor};
use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

// ---------------------------------------------------------------------------
// try_acquire: no contention (single thread, capacity large)
// ---------------------------------------------------------------------------

fn bench_try_acquire_no_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("bp_try_acquire_no_contention");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    let configs = vec![
        (10usize, "max10"),
        (100, "max100"),
        (1000, "max1000"),
    ];

    for (max, mname) in &configs {
        let cfg = BackpressureConfig {
            max_concurrent: *max,
            high_water: 0.8,
            low_water: 0.5,
            cooldown: Duration::ZERO, // disable cooldown for pure CAS measurement
        };
        let monitor = BackpressureMonitor::new(cfg);

        group.bench_function(format!("{mname}_acquire_only"), |b| {
            b.iter(|| {
                // Acquire and immediately drop (release) to keep capacity available
                let permit = black_box(monitor.try_acquire()).unwrap();
                black_box(&permit);
                drop(permit);
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// try_acquire: high contention (multiple threads fighting for limited slots)
// ---------------------------------------------------------------------------

fn bench_try_acquire_high_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("bp_try_acquire_high_contention");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(30);

    let thread_counts = vec![10usize, 100];
    let max_concurrent = 10usize;

    for &threads in &thread_counts {
        let cfg = BackpressureConfig {
            max_concurrent,
            high_water: 0.8,
            low_water: 0.5,
            cooldown: Duration::ZERO,
        };
        let monitor = Arc::new(BackpressureMonitor::new(cfg));
        let barrier = Arc::new(Barrier::new(threads));

        group.bench_with_input(
            BenchmarkId::new(format!("t{threads}_max{max_concurrent}"), threads),
            &threads,
            |b, _| {
                b.iter(|| {
                    let monitor = Arc::clone(&monitor);
                    let barrier = Arc::clone(&barrier);
                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            let monitor = Arc::clone(&monitor);
                            let barrier = Arc::clone(&barrier);
                            thread::spawn(move || {
                                barrier.wait();
                                // Each thread tries to acquire 100 times
                                let mut successes = 0usize;
                                for _ in 0..100 {
                                    if let Ok(permit) = monitor.try_acquire() {
                                        successes += 1;
                                        drop(permit);
                                    }
                                }
                                successes
                            })
                        })
                        .collect();
                    let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
                    black_box(total);
                });
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Permit acquire+release latency (single thread, measure round-trip)
// ---------------------------------------------------------------------------

fn bench_permit_acquire_release(c: &mut Criterion) {
    let mut group = c.benchmark_group("bp_permit_acquire_release");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    let cfg = BackpressureConfig {
        max_concurrent: 1000,
        high_water: 0.8,
        low_water: 0.5,
        cooldown: Duration::ZERO,
    };
    let monitor = BackpressureMonitor::new(cfg);

    // Acquire and release in tight loop
    group.bench_function("acquire_release_tight", |b| {
        b.iter(|| {
            let permit = monitor.try_acquire().unwrap();
            black_box(&permit);
            // permit drops here, releasing
        });
    });

    // Acquire, hold briefly (black_box), release
    group.bench_function("acquire_hold_release", |b| {
        b.iter(|| {
            let permit = monitor.try_acquire().unwrap();
            for _ in 0..10 {
                black_box(&permit);
            }
        });
    });

    // Batch: acquire N permits, then release all
    group.bench_function("batch_10_acquire_release", |b| {
        b.iter(|| {
            let mut permits = Vec::with_capacity(10);
            for _ in 0..10 {
                permits.push(monitor.try_acquire().unwrap());
            }
            black_box(&permits);
            // all drop here
        });
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Rejection path: at capacity, measure rejection throughput
// ---------------------------------------------------------------------------

fn bench_rejection_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("bp_rejection");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(50);

    let cfg = BackpressureConfig {
        max_concurrent: 10,
        high_water: 0.8,
        low_water: 0.5,
        cooldown: Duration::ZERO,
    };
    let monitor = BackpressureMonitor::new(cfg);

    // Fill to capacity
    let mut held = Vec::new();
    for _ in 0..10 {
        held.push(monitor.try_acquire().unwrap());
    }

    // Now all acquires should be rejected
    group.bench_function("reject_at_capacity", |b| {
        b.iter(|| {
            let result = black_box(monitor.try_acquire());
            assert!(result.is_err());
        });
    });

    // Release one, acquire one (contention-free slot cycling)
    group.bench_function("slot_cycle_1", |b| {
        b.iter(|| {
            // Release one held permit by replacing it
            let old = held.pop().unwrap();
            drop(old);
            let new = monitor.try_acquire().unwrap();
            held.push(new);
        });
    });

    drop(held);
    group.finish();
}

// ---------------------------------------------------------------------------
// State transition overhead: measure update_state cost
// ---------------------------------------------------------------------------

fn bench_state_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("bp_state_transitions");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(50);

    // With cooldown=0, every acquire/release triggers state evaluation
    let cfg = BackpressureConfig {
        max_concurrent: 100,
        high_water: 0.8,
        low_water: 0.5,
        cooldown: Duration::ZERO,
    };
    let monitor = BackpressureMonitor::new(cfg);

    // Cycle around high-water threshold to trigger Warning <-> Normal transitions
    group.bench_function("cycle_high_water", |b| {
        b.iter(|| {
            // Acquire to 80 (high water = 80)
            let mut permits = Vec::new();
            for _ in 0..80 {
                permits.push(monitor.try_acquire().unwrap());
            }
            // Release all
            drop(permits);
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_try_acquire_no_contention,
    bench_try_acquire_high_contention,
    bench_permit_acquire_release,
    bench_rejection_path,
    bench_state_transitions,
);
criterion_main!(benches);

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Reed-Solomon erasure codec benchmarks.
//!
//! Covers encode throughput, decode (no loss), decode with reconstruction
//! (lost parity), and decode with verification across three (data, parity)
//! profiles and four block sizes.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mox_cloud_kernel::{reed_solomon::PathChoice, EcProfile, ReedSolomonEngine};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Test profiles & payloads
// ---------------------------------------------------------------------------

fn profiles() -> Vec<(u16, u16, &'static str)> {
    vec![(4, 2, "4+2"), (8, 4, "8+4"), (12, 4, "12+4")]
}

fn block_sizes() -> Vec<(usize, &'static str)> {
    vec![(1024, "1KB"), (4096, "4KB"), (65536, "64KB"), (1048576, "1MB")]
}

fn make_payload(size: usize) -> Vec<u8> {
    // Pseudo-random but deterministic payload so SIMD can't trivially win
    // on all-zeros; avoids rand crate overhead in bench setup.
    let mut v = Vec::with_capacity(size);
    let mut acc: u32 = 0x9E37_79B9;
    for _ in 0..size {
        acc = acc.wrapping_mul(2654435761).wrapping_add(acc >> 13);
        v.push(acc as u8);
    }
    v
}

/// Warm up the global matrix cache so cold-cache cost is not measured.
fn warm_matrix_cache() {
    let engine = ReedSolomonEngine::new();
    for &(d, p, _) in &profiles() {
        let profile = EcProfile::with_default_min_size(d, p).unwrap();
        let data = vec![0u8; 256];
        let _ = engine.encode(&profile, &data);
    }
}

// ---------------------------------------------------------------------------
// Encode benchmarks
// ---------------------------------------------------------------------------

fn bench_encode(c: &mut Criterion) {
    warm_matrix_cache();
    let engine = ReedSolomonEngine::new();

    let mut group = c.benchmark_group("rs_encode");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    for &(data, parity, pname) in &profiles() {
        let profile = EcProfile::with_default_min_size(data, parity).unwrap();
        for &(size, sname) in &block_sizes() {
            let payload = make_payload(size);
            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{pname}_{sname}"), size),
                &payload,
                |b, p| {
                    b.iter(|| {
                        black_box(engine.encode(&profile, black_box(p)).unwrap());
                    });
                },
            );
        }
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Decode (no loss) benchmarks — all shards present, no reconstruction needed
// ---------------------------------------------------------------------------

fn bench_decode_no_loss(c: &mut Criterion) {
    warm_matrix_cache();
    let engine = ReedSolomonEngine::new();

    let mut group = c.benchmark_group("rs_decode_no_loss");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    for &(data, parity, pname) in &profiles() {
        let profile = EcProfile::with_default_min_size(data, parity).unwrap();
        for &(size, sname) in &block_sizes() {
            let payload = make_payload(size);
            let shards = engine.encode(&profile, &payload).unwrap();
            let slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{pname}_{sname}"), size),
                &slots,
                |b, s| {
                    b.iter(|| {
                        black_box(
                            engine
                                .decode_reconstruct(&profile, black_box(s), payload.len())
                                .unwrap(),
                        );
                    });
                },
            );
        }
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Decode with reconstruction (lost parity shards)
// ---------------------------------------------------------------------------

fn bench_decode_reconstruct(c: &mut Criterion) {
    warm_matrix_cache();
    let engine = ReedSolomonEngine::new();

    let mut group = c.benchmark_group("rs_decode_reconstruct");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    for &(data, parity, pname) in &profiles() {
        let profile = EcProfile::with_default_min_size(data, parity).unwrap();
        for &(size, sname) in &block_sizes() {
            let payload = make_payload(size);
            let shards = engine.encode(&profile, &payload).unwrap();
            // Drop all parity shards (indices data..total)
            let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
            for slot in slots.iter_mut().skip(data as usize) {
                *slot = None;
            }
            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{pname}_{sname}"), size),
                &slots,
                |b, s| {
                    b.iter(|| {
                        black_box(
                            engine
                                .decode_reconstruct(&profile, black_box(s), payload.len())
                                .unwrap(),
                        );
                    });
                },
            );
        }
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Decode with verification (1 data shard lost, surplus parity available)
// ---------------------------------------------------------------------------

fn bench_decode_with_verification(c: &mut Criterion) {
    warm_matrix_cache();
    let engine = ReedSolomonEngine::new();

    let mut group = c.benchmark_group("rs_decode_verify");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    for &(data, parity, pname) in &profiles() {
        let profile = EcProfile::with_default_min_size(data, parity).unwrap();
        for &(size, sname) in &block_sizes() {
            let payload = make_payload(size);
            let shards = engine.encode(&profile, &payload).unwrap();
            // Drop 1 data shard so present = total-1 > data, verification runs
            let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
            slots[0] = None;
            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{pname}_{sname}"), size),
                &slots,
                |b, s| {
                    b.iter(|| {
                        black_box(
                            engine
                                .decode_with_verification(&profile, black_box(s), payload.len())
                                .unwrap(),
                        );
                    });
                },
            );
        }
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Matrix cache: hot lookup via repeated encode (matrix cached after first call)
// ---------------------------------------------------------------------------

fn bench_matrix_cache(c: &mut Criterion) {
    warm_matrix_cache();
    let engine = ReedSolomonEngine::new();
    let profile_4_2 = EcProfile::with_default_min_size(4, 2).unwrap();
    let profile_12_4 = EcProfile::with_default_min_size(12, 4).unwrap();
    let small_data = vec![0u8; 256];

    let mut group = c.benchmark_group("rs_matrix_cache");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(50);

    // Hot cache: matrix_for(4,2) is already cached from warmup
    group.bench_function("hot_encode_4+2_256B", |b| {
        b.iter(|| {
            black_box(engine.encode(black_box(&profile_4_2), black_box(&small_data)).unwrap());
        });
    });

    group.bench_function("hot_encode_12+4_256B", |b| {
        b.iter(|| {
            black_box(engine.encode(black_box(&profile_12_4), black_box(&small_data)).unwrap());
        });
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// GF(2^8) scalar multiply micro-benchmark via encode with scalar path
// ---------------------------------------------------------------------------

fn bench_gf_mul_scalar(c: &mut Criterion) {
    warm_matrix_cache();
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();

    let mut group = c.benchmark_group("rs_gf_mul");
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(50);

    let size = 65536;
    let payload = make_payload(size);
    group.throughput(Throughput::Bytes(size as u64));

    group.bench_function("encode_scalar_4+2_64KB", |b| {
        b.iter(|| {
            black_box(
                engine
                    .encode_with_path(black_box(&profile), black_box(&payload), PathChoice::Scalar)
                    .unwrap(),
            );
        });
    });

    group.bench_function("encode_auto_4+2_64KB", |b| {
        b.iter(|| {
            black_box(
                engine
                    .encode_with_path(black_box(&profile), black_box(&payload), PathChoice::Auto)
                    .unwrap(),
            );
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_encode,
    bench_decode_no_loss,
    bench_decode_reconstruct,
    bench_decode_with_verification,
    bench_matrix_cache,
    bench_gf_mul_scalar,
);
criterion_main!(benches);

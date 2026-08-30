// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Volume 服务性能基准测试
//!
//! 测试场景：
//! - 小文件IOPS：4KB/16KB/64KB随机读写IOPS
//! - 大文件吞吐量：1MB/4MB/16MB顺序读写吞吐量
//! - 纠删码性能：RS编码/解码吞吐量，不同配置对比
//! - 元数据操作：创建/删除/列出文件的延迟
//! - 批量操作：批量创建/删除文件的吞吐量
//! - 并发性能：不同并发数下的性能变化
//!
//! 性能指标输出：
//! - 吞吐量 (MB/s)
//! - IOPS (operations/sec)
//! - 延迟 (latency: p50/p95/p99)
//! - 每次操作平均耗时 (μs)
//!
//! 说明：本测试使用标准库 Instant 进行计时，兼容 cargo test 运行，
//! 输出格式参考 Criterion 风格。若需更专业的基准测试，可在 benches/
//! 目录下使用 criterion 库创建正式基准。

use bytes::Bytes;
use mox_cloud_volume_svc::{
    CauchyReedSolomon, EcProfile, ReedSolomonEngine, VolumeServer, crc32c_bytes, crc64_ecma,
    encode_and_write, StorageTier,
};
use rand::RngCore;
use std::sync::Arc;
use std::time::{Duration, Instant};

// =========================================================================
// 辅助工具
// =========================================================================

fn random_bytes(n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut v);
    v
}

/// 性能测试结果
struct BenchResult {
    name: String,
    total_ops: u64,
    total_bytes: u64,
    elapsed: Duration,
}

impl BenchResult {
    fn iops(&self) -> f64 {
        self.total_ops as f64 / self.elapsed.as_secs_f64()
    }

    fn throughput_mbps(&self) -> f64 {
        (self.total_bytes as f64 / (1024.0 * 1024.0)) / self.elapsed.as_secs_f64()
    }

    fn avg_latency_us(&self) -> f64 {
        if self.total_ops == 0 {
            return 0.0;
        }
        self.elapsed.as_secs_f64() * 1_000_000.0 / self.total_ops as f64
    }

    fn print(&self) {
        eprintln!(
            "  {:<40} | ops={:>8} | {:.2} ops/s | {:.2} MB/s | {:.2} μs/op",
            self.name,
            self.total_ops,
            self.iops(),
            self.throughput_mbps(),
            self.avg_latency_us()
        );
    }
}

/// 运行基准测试并返回结果
fn bench<F: FnMut()>(name: &str, bytes_per_op: u64, mut f: F, min_ops: u64, min_duration: Duration) -> BenchResult {
    // Warmup
    for _ in 0..(min_ops / 10).max(1) {
        f();
    }

    // Measurement
    let start = Instant::now();
    let mut ops = 0u64;
    while ops < min_ops || start.elapsed() < min_duration {
        f();
        ops += 1;
    }
    let elapsed = start.elapsed();

    BenchResult {
        name: name.to_string(),
        total_ops: ops,
        total_bytes: ops * bytes_per_op,
        elapsed,
    }
}

// =========================================================================
// 模块一：小文件 IOPS (Small File IOPS)
// =========================================================================

/// 测试：4KB 随机写 IOPS
#[test]
fn perf01_01_4kb_random_write_iops() {
    let vs = VolumeServer::new("vol-4kw".to_string(), 10 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(4 * 1024));

    let mut i = 0u64;
    let result = bench(
        "4KB random write",
        4 * 1024,
        || {
            let cid = format!("chunk-4kw-{}", i);
            vs.write_chunk(&cid, data.clone()).unwrap();
            i += 1;
        },
        500,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.iops() > 0.0, "IOPS should be > 0");
    assert!(result.total_ops >= 500);
}

/// 测试：4KB 随机读 IOPS
#[test]
fn perf01_02_4kb_random_read_iops() {
    let vs = VolumeServer::new("vol-4kr".to_string(), 10 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(4 * 1024));

    // Pre-populate
    for i in 0..1000 {
        vs.write_chunk(&format!("chunk-4kr-{}", i), data.clone())
            .unwrap();
    }

    let mut i = 0u64;
    let result = bench(
        "4KB random read",
        4 * 1024,
        || {
            let cid = format!("chunk-4kr-{}", i % 1000);
            let _ = vs.read_chunk(&cid).unwrap();
            i += 1;
        },
        1000,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.iops() > 0.0);
}

/// 测试：16KB 随机写 IOPS
#[test]
fn perf01_03_16kb_random_write_iops() {
    let vs = VolumeServer::new("vol-16kw".to_string(), 10 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(16 * 1024));

    let mut i = 0u64;
    let result = bench(
        "16KB random write",
        16 * 1024,
        || {
            let cid = format!("chunk-16kw-{}", i);
            vs.write_chunk(&cid, data.clone()).unwrap();
            i += 1;
        },
        300,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.iops() > 0.0);
}

/// 测试：16KB 随机读 IOPS
#[test]
fn perf01_04_16kb_random_read_iops() {
    let vs = VolumeServer::new("vol-16kr".to_string(), 10 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(16 * 1024));

    for i in 0..500 {
        vs.write_chunk(&format!("chunk-16kr-{}", i), data.clone())
            .unwrap();
    }

    let mut i = 0u64;
    let result = bench(
        "16KB random read",
        16 * 1024,
        || {
            let cid = format!("chunk-16kr-{}", i % 500);
            let _ = vs.read_chunk(&cid).unwrap();
            i += 1;
        },
        500,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.iops() > 0.0);
}

/// 测试：64KB 随机写 IOPS
#[test]
fn perf01_05_64kb_random_write_iops() {
    let vs = VolumeServer::new("vol-64kw".to_string(), 10 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(64 * 1024));

    let mut i = 0u64;
    let result = bench(
        "64KB random write",
        64 * 1024,
        || {
            let cid = format!("chunk-64kw-{}", i);
            vs.write_chunk(&cid, data.clone()).unwrap();
            i += 1;
        },
        100,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.iops() > 0.0);
}

/// 测试：64KB 随机读 IOPS
#[test]
fn perf01_06_64kb_random_read_iops() {
    let vs = VolumeServer::new("vol-64kr".to_string(), 10 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(64 * 1024));

    for i in 0..200 {
        vs.write_chunk(&format!("chunk-64kr-{}", i), data.clone())
            .unwrap();
    }

    let mut i = 0u64;
    let result = bench(
        "64KB random read",
        64 * 1024,
        || {
            let cid = format!("chunk-64kr-{}", i % 200);
            let _ = vs.read_chunk(&cid).unwrap();
            i += 1;
        },
        200,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.iops() > 0.0);
}

// =========================================================================
// 模块二：大文件吞吐量 (Large File Throughput)
// =========================================================================

/// 测试：1MB 顺序写吞吐量
#[test]
fn perf02_01_1mb_seq_write_throughput() {
    let vs = VolumeServer::new("vol-1mw".to_string(), 100 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(1024 * 1024));

    let mut i = 0u64;
    let result = bench(
        "1MB sequential write",
        1024 * 1024,
        || {
            let cid = format!("chunk-1mw-{}", i);
            vs.write_chunk(&cid, data.clone()).unwrap();
            i += 1;
        },
        50,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：1MB 顺序读吞吐量
#[test]
fn perf02_02_1mb_seq_read_throughput() {
    let vs = VolumeServer::new("vol-1mr".to_string(), 100 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(1024 * 1024));

    for i in 0..50 {
        vs.write_chunk(&format!("chunk-1mr-{}", i), data.clone())
            .unwrap();
    }

    let mut i = 0u64;
    let result = bench(
        "1MB sequential read",
        1024 * 1024,
        || {
            let cid = format!("chunk-1mr-{}", i % 50);
            let _ = vs.read_chunk(&cid).unwrap();
            i += 1;
        },
        50,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：4MB 顺序写吞吐量
#[test]
fn perf02_03_4mb_seq_write_throughput() {
    let vs = VolumeServer::new("vol-4mw".to_string(), 100 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(4 * 1024 * 1024));

    let mut i = 0u64;
    let result = bench(
        "4MB sequential write",
        4 * 1024 * 1024,
        || {
            let cid = format!("chunk-4mw-{}", i);
            vs.write_chunk(&cid, data.clone()).unwrap();
            i += 1;
        },
        20,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：4MB 顺序读吞吐量
#[test]
fn perf02_04_4mb_seq_read_throughput() {
    let vs = VolumeServer::new("vol-4mr".to_string(), 100 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(4 * 1024 * 1024));

    for i in 0..20 {
        vs.write_chunk(&format!("chunk-4mr-{}", i), data.clone())
            .unwrap();
    }

    let mut i = 0u64;
    let result = bench(
        "4MB sequential read",
        4 * 1024 * 1024,
        || {
            let cid = format!("chunk-4mr-{}", i % 20);
            let _ = vs.read_chunk(&cid).unwrap();
            i += 1;
        },
        20,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：16MB 顺序写吞吐量
#[test]
fn perf02_05_16mb_seq_write_throughput() {
    let vs = VolumeServer::new("vol-16mw".to_string(), 500 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(16 * 1024 * 1024));

    let mut i = 0u64;
    let result = bench(
        "16MB sequential write",
        16 * 1024 * 1024,
        || {
            let cid = format!("chunk-16mw-{}", i);
            vs.write_chunk(&cid, data.clone()).unwrap();
            i += 1;
        },
        5,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：16MB 顺序读吞吐量
#[test]
fn perf02_06_16mb_seq_read_throughput() {
    let vs = VolumeServer::new("vol-16mr".to_string(), 500 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(16 * 1024 * 1024));

    for i in 0..5 {
        vs.write_chunk(&format!("chunk-16mr-{}", i), data.clone())
            .unwrap();
    }

    let mut i = 0u64;
    let result = bench(
        "16MB sequential read",
        16 * 1024 * 1024,
        || {
            let cid = format!("chunk-16mr-{}", i % 5);
            let _ = vs.read_chunk(&cid).unwrap();
            i += 1;
        },
        5,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

// =========================================================================
// 模块三：纠删码性能 (Erasure Coding Performance)
// =========================================================================

/// 测试：RS(4+2) 编码吞吐量
#[test]
fn perf03_01_rs_4plus2_encode_throughput() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let data = random_bytes(256 * 1024);

    let mut count = 0u64;
    let result = bench(
        "RS(4+2) encode 256KB",
        256 * 1024,
        || {
            let _ = engine.encode(&profile, &data).unwrap();
            count += 1;
        },
        200,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：RS(4+2) 解码吞吐量
#[test]
fn perf03_02_rs_4plus2_decode_throughput() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let data = random_bytes(256 * 1024);
    let shards = engine.encode(&profile, &data).unwrap();

    // Prepare slots with 2 shards missing
    let mut slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
    slots[1] = None;
    slots[4] = None;

    let mut count = 0u64;
    let result = bench(
        "RS(4+2) decode 256KB (2 lost)",
        256 * 1024,
        || {
            let _ = engine
                .decode_reconstruct(&profile, &slots, data.len())
                .unwrap();
            count += 1;
        },
        100,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：RS(8+4) 编码吞吐量
#[test]
fn perf03_03_rs_8plus4_encode_throughput() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(8, 4).unwrap();
    let data = random_bytes(512 * 1024);

    let mut count = 0u64;
    let result = bench(
        "RS(8+4) encode 512KB",
        512 * 1024,
        || {
            let _ = engine.encode(&profile, &data).unwrap();
            count += 1;
        },
        100,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：RS(12+4) 编码吞吐量
#[test]
fn perf03_04_rs_12plus4_encode_throughput() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(12, 4).unwrap();
    let data = random_bytes(1024 * 1024);

    let mut count = 0u64;
    let result = bench(
        "RS(12+4) encode 1MB",
        1024 * 1024,
        || {
            let _ = engine.encode(&profile, &data).unwrap();
            count += 1;
        },
        50,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：Cauchy RS(4+2) 编码吞吐量
#[test]
fn perf03_05_cauchy_4plus2_encode_throughput() {
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let cauchy = CauchyReedSolomon::new(profile).unwrap();
    let data = random_bytes(256 * 1024);

    let mut count = 0u64;
    let result = bench(
        "Cauchy RS(4+2) encode 256KB",
        256 * 1024,
        || {
            let _ = cauchy.encode(&data).unwrap();
            count += 1;
        },
        200,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：Cauchy RS(4+2) 解码吞吐量
#[test]
fn perf03_06_cauchy_4plus2_decode_throughput() {
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let cauchy = CauchyReedSolomon::new(profile).unwrap();
    let data = random_bytes(256 * 1024);
    let shards = cauchy.encode(&data).unwrap();

    let mut slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
    slots[0] = None;
    slots[5] = None;

    let mut count = 0u64;
    let result = bench(
        "Cauchy RS(4+2) decode 256KB (2 lost)",
        256 * 1024,
        || {
            let _ = cauchy.decode_reconstruct(&slots, data.len()).unwrap();
            count += 1;
        },
        100,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：不同 RS 配置编码吞吐量对比
#[test]
fn perf03_07_rs_config_comparison() {
    let engine = ReedSolomonEngine::new();
    let configs = [(4, 2, 64 * 1024), (6, 3, 128 * 1024), (8, 4, 256 * 1024)];
    let mut results = Vec::new();

    for (data, parity, size) in configs {
        let profile = EcProfile::with_default_min_size(data, parity).unwrap();
        let payload = random_bytes(size);

        let start = Instant::now();
        let mut ops = 0u64;
        while ops < 50 || start.elapsed() < Duration::from_millis(200) {
            let _ = engine.encode(&profile, &payload).unwrap();
            ops += 1;
        }
        let elapsed = start.elapsed();
        let mbps = (ops as f64 * size as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();

        eprintln!(
            "  RS({}+{}) {}KB: {:.2} MB/s ({ops} ops in {:.2}ms)",
            data,
            parity,
            size / 1024,
            mbps,
            elapsed.as_secs_f64() * 1000.0
        );
        results.push((data, parity, mbps));
    }

    assert_eq!(results.len(), 3);
    for (_, _, mbps) in &results {
        assert!(*mbps > 0.0);
    }
}

/// 测试：大对象 RS 编码吞吐量 (16MB)
#[test]
fn perf03_08_large_object_encode_throughput() {
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(6, 3).unwrap();
    let data = random_bytes(16 * 1024 * 1024); // 16MB

    let start = Instant::now();
    let shards = engine.encode(&profile, &data).unwrap();
    let elapsed = start.elapsed();

    let mbps = (data.len() as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();
    eprintln!(
        "  RS(6+3) encode 16MB: {:.2} MB/s, {} shards, {:.2}ms",
        mbps,
        shards.len(),
        elapsed.as_secs_f64() * 1000.0
    );

    assert_eq!(shards.len(), 9);
    assert!(mbps > 0.0);
}

// =========================================================================
// 模块四：元数据操作性能 (Metadata Operation Performance)
// =========================================================================

/// 测试：chunk 创建延迟
#[test]
fn perf04_01_chunk_create_latency() {
    let vs = VolumeServer::new("vol-meta".to_string(), 10 * 1024 * 1024 * 1024);
    let data = Bytes::from_static(b"metadata test data");

    let mut latencies = Vec::with_capacity(1000);
    for i in 0..1000 {
        let cid = format!("meta-chunk-{}", i);
        let start = Instant::now();
        vs.write_chunk(&cid, data.clone()).unwrap();
        latencies.push(start.elapsed());
    }

    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() * 95) / 100];
    let p99 = latencies[(latencies.len() * 99) / 100];

    eprintln!(
        "  Chunk create latency: p50={:.2}μs, p95={:.2}μs, p99={:.2}μs",
        p50.as_secs_f64() * 1_000_000.0,
        p95.as_secs_f64() * 1_000_000.0,
        p99.as_secs_f64() * 1_000_000.0
    );

    assert!(!latencies.is_empty());
}

/// 测试：chunk 读取延迟
#[test]
fn perf04_02_chunk_read_latency() {
    let vs = VolumeServer::new("vol-metard".to_string(), 10 * 1024 * 1024 * 1024);
    let data = Bytes::from_static(b"read latency test");

    for i in 0..1000 {
        vs.write_chunk(&format!("rd-lat-{}", i), data.clone())
            .unwrap();
    }

    let mut latencies = Vec::with_capacity(1000);
    for i in 0..1000 {
        let start = Instant::now();
        let _ = vs.read_chunk(&format!("rd-lat-{}", i)).unwrap();
        latencies.push(start.elapsed());
    }

    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() * 95) / 100];
    let p99 = latencies[(latencies.len() * 99) / 100];

    eprintln!(
        "  Chunk read latency: p50={:.2}μs, p95={:.2}μs, p99={:.2}μs",
        p50.as_secs_f64() * 1_000_000.0,
        p95.as_secs_f64() * 1_000_000.0,
        p99.as_secs_f64() * 1_000_000.0
    );

    assert!(!latencies.is_empty());
}

/// 测试：chunk 删除延迟
#[test]
fn perf04_03_chunk_delete_latency() {
    let vs = VolumeServer::new("vol-metadel".to_string(), 10 * 1024 * 1024 * 1024);
    let data = Bytes::from_static(b"delete latency test");

    for i in 0..1000 {
        vs.write_chunk(&format!("del-lat-{}", i), data.clone())
            .unwrap();
    }

    let mut latencies = Vec::with_capacity(1000);
    for i in 0..1000 {
        let start = Instant::now();
        vs.delete_chunk(&format!("del-lat-{}", i)).unwrap();
        latencies.push(start.elapsed());
    }

    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() * 95) / 100];
    let p99 = latencies[(latencies.len() * 99) / 100];

    eprintln!(
        "  Chunk delete latency: p50={:.2}μs, p95={:.2}μs, p99={:.2}μs",
        p50.as_secs_f64() * 1_000_000.0,
        p95.as_secs_f64() * 1_000_000.0,
        p99.as_secs_f64() * 1_000_000.0
    );

    assert!(!latencies.is_empty());
}

/// 测试：chunk 数量对查询性能的影响
#[test]
fn perf04_04_chunk_count_scaling() {
    let vs = VolumeServer::new("vol-scale".to_string(), 100 * 1024 * 1024 * 1024);
    let data = Bytes::from_static(b"scaling test");

    // 1000 chunks
    for i in 0..1000 {
        vs.write_chunk(&format!("scale-{}", i), data.clone()).unwrap();
    }

    let start = Instant::now();
    for i in 0..100 {
        let _ = vs.read_chunk(&format!("scale-{}", i % 1000)).unwrap();
    }
    let elapsed_1k = start.elapsed();

    eprintln!(
        "  1000 chunks: 100 reads in {:.2}ms ({:.2}μs/read)",
        elapsed_1k.as_secs_f64() * 1000.0,
        elapsed_1k.as_secs_f64() * 1_000_000.0 / 100.0
    );

    assert!(vs.chunk_count() >= 1000);
}

// =========================================================================
// 模块五：批量操作性能 (Batch Operation Performance)
// =========================================================================

/// 测试：批量创建 chunk 吞吐量
#[test]
fn perf05_01_batch_create_throughput() {
    let vs = VolumeServer::new("vol-batch-create".to_string(), 50 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(4096));
    let count = 10_000;

    let start = Instant::now();
    for i in 0..count {
        vs.write_chunk(&format!("batch-{}", i), data.clone()).unwrap();
    }
    let elapsed = start.elapsed();

    let iops = count as f64 / elapsed.as_secs_f64();
    let mbps = (count as f64 * 4096.0 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();

    eprintln!(
        "  Batch create {} chunks: {:.0} ops/s, {:.2} MB/s, {:.2}ms total",
        count,
        iops,
        mbps,
        elapsed.as_secs_f64() * 1000.0
    );

    assert_eq!(vs.chunk_count(), count);
}

/// 测试：批量删除 chunk 吞吐量
#[test]
fn perf05_02_batch_delete_throughput() {
    let vs = VolumeServer::new("vol-batch-del".to_string(), 50 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(4096));
    let count = 10_000;

    for i in 0..count {
        vs.write_chunk(&format!("batch-del-{}", i), data.clone())
            .unwrap();
    }

    let start = Instant::now();
    for i in 0..count {
        vs.delete_chunk(&format!("batch-del-{}", i)).unwrap();
    }
    let elapsed = start.elapsed();

    let iops = count as f64 / elapsed.as_secs_f64();

    eprintln!(
        "  Batch delete {} chunks: {:.0} ops/s, {:.2}ms total",
        count,
        iops,
        elapsed.as_secs_f64() * 1000.0
    );

    assert_eq!(vs.chunk_count(), 0);
}

/// 测试：批量读取吞吐量
#[test]
fn perf05_03_batch_read_throughput() {
    let vs = VolumeServer::new("vol-batch-read".to_string(), 50 * 1024 * 1024 * 1024);
    let data = Bytes::from(random_bytes(4096));
    let count = 10_000;

    for i in 0..count {
        vs.write_chunk(&format!("batch-rd-{}", i), data.clone())
            .unwrap();
    }

    let start = Instant::now();
    let mut total_bytes = 0u64;
    for i in 0..count {
        let d = vs.read_chunk(&format!("batch-rd-{}", i)).unwrap();
        total_bytes += d.len() as u64;
    }
    let elapsed = start.elapsed();

    let iops = count as f64 / elapsed.as_secs_f64();
    let mbps = (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();

    eprintln!(
        "  Batch read {} chunks: {:.0} ops/s, {:.2} MB/s, {:.2}ms total",
        count,
        iops,
        mbps,
        elapsed.as_secs_f64() * 1000.0
    );

    assert_eq!(total_bytes, count as u64 * 4096);
}

// =========================================================================
// 模块六：并发性能 (Concurrency Performance)
// =========================================================================

/// 测试：并发写入 - 4 线程
#[test]
fn perf06_01_concurrent_write_4_threads() {
    let vs = Arc::new(VolumeServer::new(
        "vol-conc-w4".to_string(),
        50 * 1024 * 1024 * 1024,
    ));
    let data = Arc::new(Bytes::from(random_bytes(4096)));
    let per_thread = 2500;

    let start = Instant::now();
    let mut handles = vec![];
    for t in 0..4 {
        let vs = Arc::clone(&vs);
        let data = Arc::clone(&data);
        handles.push(std::thread::spawn(move || {
            for i in 0..per_thread {
                let cid = format!("conc-w4-t{}-{}", t, i);
                vs.write_chunk(&cid, data.clone()).unwrap();
            }
            per_thread
        }));
    }

    let total_ops: u64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    let elapsed = start.elapsed();

    let iops = total_ops as f64 / elapsed.as_secs_f64();
    let mbps = (total_ops as f64 * 4096.0 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();

    eprintln!(
        "  Concurrent write (4 threads, {} ops): {:.0} ops/s, {:.2} MB/s",
        total_ops, iops, mbps
    );

    assert_eq!(vs.chunk_count(), total_ops);
}

/// 测试：并发写入 - 8 线程
#[test]
fn perf06_02_concurrent_write_8_threads() {
    let vs = Arc::new(VolumeServer::new(
        "vol-conc-w8".to_string(),
        50 * 1024 * 1024 * 1024,
    ));
    let data = Arc::new(Bytes::from(random_bytes(4096)));
    let per_thread = 1250;

    let start = Instant::now();
    let mut handles = vec![];
    for t in 0..8 {
        let vs = Arc::clone(&vs);
        let data = Arc::clone(&data);
        handles.push(std::thread::spawn(move || {
            for i in 0..per_thread {
                let cid = format!("conc-w8-t{}-{}", t, i);
                vs.write_chunk(&cid, data.clone()).unwrap();
            }
            per_thread
        }));
    }

    let total_ops: u64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    let elapsed = start.elapsed();

    let iops = total_ops as f64 / elapsed.as_secs_f64();
    let mbps = (total_ops as f64 * 4096.0 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();

    eprintln!(
        "  Concurrent write (8 threads, {} ops): {:.0} ops/s, {:.2} MB/s",
        total_ops, iops, mbps
    );

    assert_eq!(vs.chunk_count(), total_ops);
}

/// 测试：并发读取 - 4 线程
#[test]
fn perf06_03_concurrent_read_4_threads() {
    let vs = Arc::new(VolumeServer::new(
        "vol-conc-r4".to_string(),
        50 * 1024 * 1024 * 1024,
    ));
    let data = Bytes::from(random_bytes(4096));

    // Pre-populate
    for i in 0..1000 {
        vs.write_chunk(&format!("conc-rd-{}", i), data.clone())
            .unwrap();
    }

    let per_thread = 2500;
    let start = Instant::now();
    let mut handles = vec![];
    for t in 0..4 {
        let vs = Arc::clone(&vs);
        handles.push(std::thread::spawn(move || {
            for i in 0..per_thread {
                let cid = format!("conc-rd-{}", (i + t * 100) % 1000);
                let _ = vs.read_chunk(&cid).unwrap();
            }
            per_thread
        }));
    }

    let total_ops: u64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    let elapsed = start.elapsed();

    let iops = total_ops as f64 / elapsed.as_secs_f64();

    eprintln!(
        "  Concurrent read (4 threads, {} ops): {:.0} ops/s",
        total_ops, iops
    );

    assert!(iops > 0.0);
}

/// 测试：并发读取 - 16 线程
#[test]
fn perf06_04_concurrent_read_16_threads() {
    let vs = Arc::new(VolumeServer::new(
        "vol-conc-r16".to_string(),
        50 * 1024 * 1024 * 1024,
    ));
    let data = Bytes::from(random_bytes(4096));

    for i in 0..1000 {
        vs.write_chunk(&format!("conc-rd16-{}", i), data.clone())
            .unwrap();
    }

    let per_thread = 1250;
    let start = Instant::now();
    let mut handles = vec![];
    for t in 0..16 {
        let vs = Arc::clone(&vs);
        handles.push(std::thread::spawn(move || {
            for i in 0..per_thread {
                let cid = format!("conc-rd16-{}", (i + t * 50) % 1000);
                let _ = vs.read_chunk(&cid).unwrap();
            }
            per_thread
        }));
    }

    let total_ops: u64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    let elapsed = start.elapsed();

    let iops = total_ops as f64 / elapsed.as_secs_f64();

    eprintln!(
        "  Concurrent read (16 threads, {} ops): {:.0} ops/s",
        total_ops, iops
    );

    assert!(iops > 0.0);
}

/// 测试：并发扩展性对比
#[test]
fn perf06_05_concurrency_scaling_comparison() {
    let data = Bytes::from(random_bytes(4096));
    let thread_counts = [1, 2, 4, 8];
    let mut results = Vec::new();

    for threads in thread_counts {
        let vs = Arc::new(VolumeServer::new(
            format!("vol-scale-{}", threads),
            20 * 1024 * 1024 * 1024,
        ));

        // Pre-populate for reads
        for i in 0..500 {
            vs.write_chunk(&format!("sc-{}-{}", threads, i), data.clone())
                .unwrap();
        }

        let per_thread = 2000 / threads.max(1);
        let start = Instant::now();
        let mut handles = vec![];
        for t in 0..threads {
            let vs = Arc::clone(&vs);
            let data = data.clone();
            handles.push(std::thread::spawn(move || {
                let mut count = 0u64;
                for i in 0..per_thread {
                    let cid = format!("sc-{}-{}", threads, (i + t * 10) % 500);
                    if vs.read_chunk(&cid).is_ok() {
                        count += 1;
                    }
                    // Also write some
                    let wid = format!("sc-{}-w{}-{}", threads, t, i);
                    if vs.write_chunk(&wid, data.clone()).is_ok() {
                        count += 1;
                    }
                }
                count
            }));
        }

        let total: u64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
        let elapsed = start.elapsed();
        let iops = total as f64 / elapsed.as_secs_f64();

        results.push((threads, iops));
        eprintln!(
            "  {} threads: {:.0} ops/s ({} ops in {:.2}ms)",
            threads,
            iops,
            total,
            elapsed.as_secs_f64() * 1000.0
        );
    }

    assert_eq!(results.len(), 4);
    // 验证所有并发级别都有正的 IOPS
    for (_, iops) in &results {
        assert!(*iops > 0.0);
    }
}

// =========================================================================
// 模块七：校验和性能 (Checksum Performance)
// =========================================================================

/// 测试：CRC32C 计算吞吐量
#[test]
fn perf07_01_crc32c_throughput() {
    let data = random_bytes(1024 * 1024); // 1MB

    let mut count = 0u64;
    let result = bench(
        "CRC32C 1MB",
        1024 * 1024,
        || {
            let _ = crc32c_bytes(&data);
            count += 1;
        },
        100,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：CRC64 ECMA 计算吞吐量
#[test]
fn perf07_02_crc64_ecma_throughput() {
    let data = random_bytes(1024 * 1024);

    let mut count = 0u64;
    let result = bench(
        "CRC64-ECMA 1MB",
        1024 * 1024,
        || {
            let _ = crc64_ecma(&data);
            count += 1;
        },
        100,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

/// 测试：SHA256 计算吞吐量
#[test]
fn perf07_03_sha256_throughput() {
    use mox_cloud_volume_svc::sha256_hex;

    let data = random_bytes(1024 * 1024);

    let mut count = 0u64;
    let result = bench(
        "SHA256 1MB",
        1024 * 1024,
        || {
            let _ = sha256_hex(&data);
            count += 1;
        },
        50,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.throughput_mbps() > 0.0);
}

// =========================================================================
// 模块八：文件系统布局性能 (FS Layout Performance)
// =========================================================================

/// 测试：encode_and_write 文件系统写入性能
#[test]
fn perf08_01_encode_and_write_performance() {
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let data = random_bytes(256 * 1024);

    let mut count = 0u64;
    let result = bench(
        "encode_and_write 256KB (RS 4+2)",
        256 * 1024,
        || {
            let oid = format!("obj-perf-{}", count);
            encode_and_write(mount, "perf-bucket", &oid, &profile, StorageTier::Hot, &data)
                .unwrap();
            count += 1;
        },
        50,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.iops() > 0.0);
}

/// 测试：小尺寸数据（replica 模式）写入性能
#[test]
fn perf08_02_small_object_replica_performance() {
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path();
    let profile = EcProfile::with_default_min_size(3, 2).unwrap();
    let data = random_bytes(1024); // 1KB, 小于默认 64KB 阈值

    let mut count = 0u64;
    let result = bench(
        "encode_and_write 1KB (replica mode)",
        1024,
        || {
            let oid = format!("replica-obj-{}", count);
            encode_and_write(mount, "repl-bucket", &oid, &profile, StorageTier::Hot, &data)
                .unwrap();
            count += 1;
        },
        200,
        Duration::from_millis(500),
    );

    result.print();
    assert!(result.iops() > 0.0);
}

// =========================================================================
// 模块九：综合性能报告 (Performance Summary)
// =========================================================================

/// 测试：完整性能报告 - 输出汇总表
#[test]
fn perf09_01_comprehensive_performance_report() {
    eprintln!();
    eprintln!("{:=<90}", "=");
    eprintln!("  Volume Service Performance Benchmark Report");
    eprintln!("{:=<90}", "=");
    eprintln!();
    eprintln!("  {:<40} | {:>10} | {:>12} | {:>12} | {:>12}",
        "Benchmark", "Ops", "IOPS", "Throughput", "Avg Latency");
    eprintln!("  {:->40}-+-{:->10}-+-{:->12}-+-{:->12}-+-{:->12}",
        "", "", "", "", "");

    // Small file IOPS
    let vs_rw = VolumeServer::new("vol-summary".to_string(), 20 * 1024 * 1024 * 1024);
    let data_4k = Bytes::from(random_bytes(4 * 1024));

    // 4KB write
    let mut i = 0u64;
    let r1 = bench("4KB random write", 4 * 1024, || {
        vs_rw.write_chunk(&format!("sum-w-{}", i), data_4k.clone()).unwrap();
        i += 1;
    }, 200, Duration::from_millis(200));
    print_summary_row(&r1);

    // 4KB read
    let mut i = 0u64;
    let r2 = bench("4KB random read", 4 * 1024, || {
        let _ = vs_rw.read_chunk(&format!("sum-w-{}", i % 200)).unwrap();
        i += 1;
    }, 500, Duration::from_millis(200));
    print_summary_row(&r2);

    // RS encode
    let engine = ReedSolomonEngine::new();
    let profile = EcProfile::with_default_min_size(4, 2).unwrap();
    let data_256k = random_bytes(256 * 1024);
    let mut count = 0u64;
    let r3 = bench("RS(4+2) encode 256KB", 256 * 1024, || {
        let _ = engine.encode(&profile, &data_256k).unwrap();
        count += 1;
    }, 100, Duration::from_millis(200));
    print_summary_row(&r3);

    eprintln!("  {:->40}-+-{:->10}-+-{:->12}-+-{:->12}-+-{:->12}",
        "", "", "", "", "");
    eprintln!("  * All benchmarks run in-memory (no disk I/O for VolumeServer)");
    eprintln!("  * RS benchmarks measure pure CPU encoding/decoding time");
    eprintln!("  * Results vary based on CPU, memory, and system load");
    eprintln!();
}

fn print_summary_row(r: &BenchResult) {
    eprintln!(
        "  {:<40} | {:>10} | {:>10.0} /s | {:>9.2} MB/s | {:>10.2} μs",
        r.name,
        r.total_ops,
        r.iops(),
        r.throughput_mbps(),
        r.avg_latency_us()
    );
}

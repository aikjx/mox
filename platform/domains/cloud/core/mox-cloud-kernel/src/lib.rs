// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Mox Cloud L5 Pure Algorithm Kernel
//!
//! 零业务依赖的纯算法内核，从 mox-cloud-volume-svc / mox-cloud-s3-svc 抽离：
//! - `reed_solomon` — Vandermonde 矩阵 + GF(2^8) Gauss-Jordan 纠删码引擎
//! - `gf256_simd` — GF(2^8) SIMD 加速内核（AVX2/NEON）
//! - `multi_writer` — 多副本写仲裁
//! - `hedged_reader` — Hedged 读仲裁
//! - `backpressure` — CAS 背压信号量
//! - `buffer_pool` — 四层分档缓冲池
//! - `reader_capability` — ReaderCapability trait + pipeline
//! - `profile` — EcProfile 纠删码配置
//! - `metrics` — 数据面指标
//! - `scanner` — 三维扫描预算

pub mod backpressure;
pub mod buffer_pool;
pub mod gf256_simd;
pub mod hedged_reader;
pub mod metrics;
pub mod multi_writer;
pub mod profile;
pub mod reader_capability;
pub mod reed_solomon;
pub mod scanner;

// ── backpressure ──
pub use backpressure::{
    BackpressureConfig, BackpressureError, BackpressureMetrics, BackpressureMonitor,
    BackpressurePermit, BackpressureState,
};

// ── buffer_pool ──
pub use buffer_pool::{
    BufferPool, BufferPoolConfig, BufferPoolStats, BufferTierConfig, BufferTierStats,
    PooledBuffer,
};

// ── gf256_simd ──
pub use gf256_simd::{gf_vec_mul_auto, is_avx2_supported, SIMD_CHUNK};

// ── hedged_reader ──
pub use hedged_reader::{HedgedReader, ReadError, ShardReadCost, ShardReader};

// ── metrics ──
pub use metrics::{
    encode_us_samples_snapshot, observe_encode_us, reset_all, ENCODE_US_COUNT,
    MAX_HISTOGRAM_SAMPLES, REBUILD_COUNT, SHARDS_LOST_TOTAL,
};

// ── multi_writer ──
pub use multi_writer::{MultiWriter, ShardWriter, WriteError, WriteProgressPolicy, WriteResult};

// ── profile ──
pub use profile::{EcProfile, DEFAULT_MIN_OBJ_SIZE};

// ── reader_capability ──
pub use reader_capability::{
    probe_capabilities, ReadCapabilityError, ReaderCapabilitiesSummary, ReaderCapability,
    ReaderPipeline, SimpleReader,
};

// ── reed_solomon ──
pub use reed_solomon::{RSError, RSResult, ReedSolomon2Plus1, ReedSolomonEngine, shard_size_for};

// ── scanner ──
pub use scanner::{
    CapacityBudget, IoBudget, ScanBudget, ScanBudgetTracker, ScanStats, TimeBudget,
};

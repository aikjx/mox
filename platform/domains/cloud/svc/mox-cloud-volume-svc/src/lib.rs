// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Cloud Drive L4 Volume (All self-implemented, no external storage system)
//!
//! Mox Cloud Drive Volume — 数据面 (Data Plane)
//! 负责: chunk 读写、容量控制、自研 RS(2+1 XOR) 纠删码、chunk 重建。
//!
//! 额外提供 AIS 风格的 Reed-Solomon(n+k) over GF(2^8) 完整 EC 引擎
//! (`reed_solomon::ReedSolomonEngine`)，配合 profile / manifest /
//! fs_layout / rebuild / metrics 模块使用。

pub mod backpressure;
pub mod buffer_pool;
pub mod chunk_rebuild;
pub mod config;
pub mod erasure_coding_ext;
pub mod error;
pub mod fs_layout;
pub mod hedged_reader;
pub mod manifest;
pub mod metrics;
pub mod multi_writer;
pub mod profile;
pub mod reader_capability;
pub mod rebuild;
pub mod reed_solomon;
pub mod storage_tier;
pub mod gf256_simd;
pub mod volume_server;

pub use backpressure::{
    BackpressureConfig, BackpressureError, BackpressureMetrics, BackpressureMonitor,
    BackpressurePermit, BackpressureState,
};
pub use buffer_pool::{
    BufferPool, BufferPoolConfig, BufferPoolStats, BufferTierConfig, BufferTierStats,
    PooledBuffer,
};
pub use chunk_rebuild::{InMemoryPeerFetcher, PeerChunkFetcher, RebuildCoordinator};
pub use erasure_coding_ext::{
    CauchyReedSolomon, ChecksumType, IncrementalEncoder, IncrementalUpdate,
    IncrementalUpdateResult, IntegrityChecker, ProgressiveRebuildJob, ProgressiveRebuilder,
    RebuildEngineType, RebuildPriority, RebuildStats, ShardChecksum,
};
pub use error::{VolumeError, VolumeResult};
pub use fs_layout::{ec_object_dir, manifest_path, parse_shard_path, shard_path};
pub use hedged_reader::{HedgedReader, ReadError, ShardReadCost, ShardReader};
pub use manifest::{crc64_ecma, crc64_ecma_update, EcManifest, StorageTier};
pub use metrics::{
    encode_us_samples_snapshot, observe_encode_us, reset_all, REBUILD_COUNT, SHARDS_LOST_TOTAL,
    ENCODE_US_COUNT, MAX_HISTOGRAM_SAMPLES,
};
pub use multi_writer::{MultiWriter, WriteError, WriteProgressPolicy, WriteResult, ShardWriter};
pub use profile::{EcProfile, DEFAULT_MIN_OBJ_SIZE};
pub use reader_capability::{
    probe_capabilities, ReaderCapabilitiesSummary, ReaderCapability, ReadCapabilityError,
    ReaderPipeline, SimpleReader,
};
pub use rebuild::{encode_and_write, RebuildJob};
pub use reed_solomon::{RSError, RSResult, ReedSolomon2Plus1, ReedSolomonEngine, shard_size_for};
pub use storage_tier::{
    MigrationStatus, MigrationScheduleWindow, ObjectAccessStats, StorageLayer,
    StorageLayerConfig, StorageTierEngine, TierMigrationTask, TierStats, TieringPolicyConfig,
    TieringPolicyType,
};
pub use gf256_simd::{gf_vec_mul_auto, is_avx2_supported, SIMD_CHUNK};
pub use volume_server::{crc32c_bytes, sha256_hex, ChunkAck, VolumeId, VolumeServer};
pub use config::{
    ErasureCodingConfig, ReadArbitrationConfig, VolumeFeatureFlags,
    VolumeServiceConfig, WriteArbitrationConfig,
};

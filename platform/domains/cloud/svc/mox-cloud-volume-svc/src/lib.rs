// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Cloud Drive L4 Volume (All self-implemented, no external storage system)
//!
//! Mox Cloud Drive Volume — 数据面 (Data Plane)
//! 负责: chunk 读写、容量控制、自研 RS(2+1 XOR) 纠删码、chunk 重建。
//!
//! 额外提供 AIS 风格的 Reed-Solomon(n+k) over GF(2^8) 完整 EC 引擎
//! (`reed_solomon::ReedSolomonEngine`)，配合 profile / manifest /
//! fs_layout / rebuild / metrics 模块使用。

pub mod chunk_rebuild;
pub mod error;
pub mod fs_layout;
pub mod manifest;
pub mod metrics;
pub mod profile;
pub mod rebuild;
pub mod reed_solomon;
pub mod gf256_simd;
pub mod volume_server;

pub use chunk_rebuild::{InMemoryPeerFetcher, PeerChunkFetcher, RebuildCoordinator};
pub use error::{VolumeError, VolumeResult};
pub use fs_layout::{ec_object_dir, manifest_path, parse_shard_path, shard_path};
pub use manifest::{crc64_ecma, crc64_ecma_update, EcManifest, StorageTier};
pub use metrics::{
    encode_us_samples_snapshot, observe_encode_us, reset_all, REBUILD_COUNT, SHARDS_LOST_TOTAL,
    ENCODE_US_COUNT, MAX_HISTOGRAM_SAMPLES,
};
pub use profile::{EcProfile, DEFAULT_MIN_OBJ_SIZE};
pub use rebuild::{encode_and_write, RebuildJob};
pub use reed_solomon::{RSError, RSResult, ReedSolomon2Plus1, ReedSolomonEngine, shard_size_for};
pub use gf256_simd::{gf_vec_mul_auto, is_avx2_supported, SIMD_CHUNK};
pub use volume_server::{crc32c_bytes, sha256_hex, ChunkAck, VolumeId, VolumeServer};

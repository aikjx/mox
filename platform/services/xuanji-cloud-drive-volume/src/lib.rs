//! Cloud Drive L4 Volume (All self-implemented, no external storage system)
//!
//! Xuanji Cloud Drive Volume — 数据面 (Data Plane)
//! 负责: chunk 读写、容量控制、自研 RS(2+1 XOR) 纠删码、chunk 重建
//!
//! 完全自研。

pub mod chunk_rebuild;
pub mod error;
pub mod reed_solomon;
pub mod volume_server;

pub use chunk_rebuild::{InMemoryPeerFetcher, PeerChunkFetcher, RebuildCoordinator};
pub use error::{VolumeError, VolumeResult};
pub use reed_solomon::{RSError, RSResult, ReedSolomon2Plus1};
pub use volume_server::{crc32c_bytes, sha256_hex, ChunkAck, VolumeId, VolumeServer};

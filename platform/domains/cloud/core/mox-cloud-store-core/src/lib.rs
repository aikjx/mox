// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Cloud Drive L2 Store Engine（存储引擎，全部自研）
//!
//! 基于 [`mox-base-store-core`] 的统一物理口契约（`ObjectStore` / `KvStore` /
//! `ObjectStreamWriter`）实现真实的存储后端与数据面算法：
//!
//! - **FS 后端**：内容寻址（SHA-256 两级散列）去重存储 + 引用计数 GC + 原子写。
//! - **S3 后端**（阶段2）：自研 SigV4 客户端，与 MinIO/COS/OSS 互通；目标空读自动回源。
//! - **版本/快照**：`versions/<fileId>/vN.json` 零拷贝恢复。
//! - **企业级算法**（阶段3，feature `erasure`）：RS 纠删码装饰器、bitrot 检测、
//!   自愈协调、对象数据缓存——复用 `mox-cloud-volume-svc` 的 RS 引擎，不重写。
//!
//! ## 磁盘布局（DATA_DIR）
//! ```text
//! DATA_DIR/
//! ├── objects/<xx>/<keyhash>.json     # 对象元数据（path/content_type/size/sha256）
//! ├── chunks/<xx>/<sha256>            # 内容寻址数据块（去重单元）
//! ├── refs/<xx>/<sha256>.json         # 引用计数索引
//! ├── kv/<keyhash>.json               # KvStore（原子 JSON）
//! ├── mpu/<uploadId>/<partN>          # 分片上传暂存
//! └── versions/<fileId>/vN.json       # 对象版本元数据
//! ```
//! `<xx>` 为哈希前 2 个十六进制字符（两级散列分片，避免单目录膨胀）。
//!
//! ## 依赖方向
//! 域 → mox-base-store-core（契约）← mox-cloud-store-core（实现）
//! 协议层（s3/filer-svc）→ 本 crate 门面类型；本 crate → volume-svc（仅 feature `erasure`）。

pub mod backend;
pub mod dedup;
pub mod fs_backend;
pub mod gc;
pub mod kv_backend;
pub mod stats;
pub mod stream_writer;
pub mod versioning;

// 阶段2：S3 协议层（feature `s3`）
#[cfg(feature = "s3")]
pub mod fallback;
#[cfg(feature = "s3")]
pub mod s3_backend;

// 阶段3：企业级算法（feature `erasure`，复用 volume-svc RS 引擎）
#[cfg(feature = "erasure")]
pub mod bitrot;
#[cfg(feature = "erasure")]
pub mod cache;
#[cfg(feature = "erasure")]
pub mod erasure;
#[cfg(feature = "erasure")]
pub mod heal;
#[cfg(feature = "erasure")]
pub mod snapshot;

// 统一错误/结果类型（re-export 自 base-store-core，协议层免直接依赖）
pub use mox_base_store_core::{StoreError, StoreResult};

pub use backend::{create_backend, BackendKind, StoreBackend, StoreConfig};
pub use dedup::{
    list_chunks, list_object_refs, ChunkRefManager, RebuildReport,
};
pub use fs_backend::{FsObjectStore, KeyPathCodec, ObjectMeta};
pub use gc::{GCReport, GarbageCollector};
pub use kv_backend::FsKvStore;
pub use stats::{collect_store_stats, StoreStats};
pub use stream_writer::{ContentDefinedChunker, FsStreamWriter, StreamResult};
pub use versioning::{VersionInfo, VersionManager};

// 阶段2：S3 协议层导出（feature `s3`）
#[cfg(feature = "s3")]
pub use backend::S3ClientConfig;
#[cfg(feature = "s3")]
pub use fallback::FallbackObjectStore;
#[cfg(feature = "s3")]
pub use s3_backend::{build_s3_backend, S3Client, S3HeadInfo, S3ObjectStore};

// =============== 共享工具 ===============

/// 计算 SHA-256 十六进制摘要（与 volume-svc / 规范一致）
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

/// 两级散列前缀（前 2 个十六进制字符）
pub fn hash_prefix(sha: &str) -> String {
    sha.get(..2).unwrap_or("00").to_string()
}

/// 文件系统路径穿越防护：将逻辑 key 编码为安全的单段文件名
///
/// 不做 URL 编码（会引入 `%2F` 与目录层级歧义），而是直接使用
/// `sha256(key)` 作为文件名——路径只存在于元数据 JSON 中，天然免疫 `../` 穿越。
pub fn key_file_name(key: &str) -> String {
    format!("{}.obj", sha256_hex(key.as_bytes()))
}

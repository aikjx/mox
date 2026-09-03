// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Mox Cloud Drive M3: POSIX Filer crate.
//!
//! 三个元数据后端：
//! - SQLite（内存：`rusqlite`）
//! - Postgres+Citus（内存 BTreeMap，模拟 shard_id = id % 16）
//! - Redis（内存 HashMap + 假 TTL）
//!
//! Filer server 提供元数据后端切换 [`FilerServer::switch_backend`]，
//! FUSE 客户端为 Mox 自研（模拟 mount/ls/write/s3 list，避免跨平台依赖）。
//!
//! ## 增强特性
//! - 目录项缓存（LRU + 负缓存 + 预取）
//! - 文件锁（POSIX fcntl 建议锁 + 死锁检测）
//! - 配额管理（用户/目录/桶三级配额）
//! - 目录快照（COW + 空间回收）
//! - 分布式元数据增强（Citus 分片 + 分布式事务）
//!
//! ## 自研边界
//! 未引入任何第三方 POSIX 网关：FUSE 相关逻辑 100% 由璇玑自研实现。
#![allow(dead_code)]

pub mod dir_entry_cache;
pub mod error;
pub mod file_lock;
pub mod filer_server;
pub mod fuse_client;
pub mod meta_pg_citus;
pub mod meta_redis;
pub mod meta_sqlite;
pub mod meta_trait;
pub mod posix_api;
pub mod quota_manager;
pub mod snapshot_filer;
pub mod store_core_bridge;

pub use dir_entry_cache::{CacheStats, DirEntryCache, SharedDirEntryCache};
pub use error::{FilerError, FilerResult};
pub use file_lock::{
    DeadlockResult, FileLockManager, LockRange, LockRecord, LockStats, LockType,
    SharedFileLockManager, DEFAULT_LOCK_TIMEOUT_MS,
};
pub use filer_server::{FilerServer, InMemoryObjectStorage, ObjectStorage};
pub use fuse_client::FuseClient;
pub use meta_pg_citus::PgCitusMeta;
pub use meta_redis::RedisMeta;
pub use meta_sqlite::SqliteMeta;
pub use meta_trait::{
    Attr, AttrPatch, BatchCreateResult, BatchDeleteResult, BatchReadAttrResult, DirEntry,
    DirListPage, MetaBackend, MetaStorageProvider, S_IFDIR, S_IFLNK, S_IFREG, TxStatus, META_BACKENDS, PJD_CASES_TOTAL,
    PJD_PASS_THRESHOLD,
};
pub use posix_api::Filer;
pub use quota_manager::{
    QuotaAlert, QuotaCheckResult, QuotaLimit, QuotaManager, QuotaStats, QuotaType,
    QuotaUsage, SharedQuotaManager, DEFAULT_GRACE_PERIOD_SECS,
};
pub use snapshot_filer::{
    SharedSnapshotManager, SnapshotInfo, SnapshotManager, SnapshotStatus,
};
pub use store_core_bridge::StoreCoreObjectStorage;

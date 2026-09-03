//! # mox-cloud-domain-traits
//!
//! Mox Cloud **L4/L6 领域契约层** —— 集中定义所有跨 crate 共享的核心 trait，
//! 消除各 svc crate 内部 trait 定义分散的问题。
//!
//! ## 层级定位
//!
//! - **L6 存储后端抽象**：[`StorageBackend`] —— 底层 chunk 级存取
//! - **L4 元数据存储抽象**：[`MetaStorage`] —— 文件/对象元数据存取
//! - **L4 生命周期评估抽象**：[`LifecycleEvaluator`] —— 存储分级与过期策略
//! - **L4 分片读取抽象**：[`ShardReader`] —— 跨节点分片读取与对冲
//! - **L4 分片写入抽象**：[`ShardWriter`] —— 多副本分片写入与法定人数
//!
//! ## 设计约束
//!
//! - **零 svc 依赖**：本 crate 不依赖任何 volume/s3/filer/master/rebalance svc crate，
//!   也不依赖 mox-cloud-kernel，是纯粹的 trait 契约层。
//! - **Object-safe**：所有 trait 均可通过 `dyn Trait` 进行动态分发。
//! - **serde 派生**：所有数据结构体均派生 `Serialize/Deserialize`，用于配置与持久化。

pub mod error;
pub mod lifecycle;
pub mod meta_storage;
pub mod shard_reader;
pub mod shard_writer;
pub mod storage_backend;

// ---------------------------------------------------------------------------
// 统一 re-export
// ---------------------------------------------------------------------------

// 统一顶层错误
pub use error::{CloudError, CloudResult};

// L6 存储后端
pub use storage_backend::{
    BackendCapabilities, BackendType, ChunkId, ChunkInfo, ChunkListPage, ConsistencyModel,
    StorageBackend, StorageError,
};

// L4 元数据存储
pub use meta_storage::{
    ConcurrencyModel, DirEntry, DirListPage, EntryType, MetaError, MetaKey, MetaStorage, MetaValue,
};

// L4 生命周期
pub use lifecycle::{
    LifecycleAction, LifecycleEvaluator, LifecycleThresholds, ObjectLifecycleMeta,
    ReplicationStatus, StorageClass, StorageClassTransition,
};

// L4 分片读取
pub use shard_reader::{
    HedgeConfig, ReadError, ShardLocation, ShardReadCost, ShardReader, StorageTier,
};

// L4 分片写入
pub use shard_writer::{ConcurrencyHint, ShardWriter, WriteError, WriteQuorum, WriteResult};

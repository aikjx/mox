// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Mox Cloud Drive S3 Compatible Service — 100% 自研，零成品存储系统。
//!
//! 兼容 S3 v20060301，38+ API 全自研：
//! - 桶管理：ListBuckets/CreateBucket/DeleteBucket/HeadBucket
//! - 对象：Put/Get/Delete/Head/Copy + ACL + Tagging
//! - 列表：ListObjects V1/V2 + ListObjectVersions
//! - MPU：Create/UploadPart/UploadPartCopy/Complete/Abort + ListParts + ListMultipartUploads
//! - 高级：Versioning、Bucket Policy、Lifecycle、CORS、批量 Delete
//! - 增强：Bucket Analytics（存储桶分析）、Batch Operations（批量操作）、
//!   Replication（CRR/SRR 复制）、Inventory（存储桶清单）
//! - 持久化（阶段2d）：[`persist`] 写 chokepoint 镜像到 `mox-cloud-store-core` 真实后端
//!
//! 签名：SigV4（复用 mox-standards sigv4）
//! ETag：CRC32C + MD5 + MPU concat-MD5（复用 mox-standards etag_crc32c）

pub mod acl;
pub mod bucket_analytics;
pub mod config;
pub mod cors;
pub mod error;
pub mod etag;
pub mod glacier_adapter;
pub mod glacier_http;
pub mod inventory;
pub mod lifecycle;
pub mod mpu;
pub mod object_batch_ops;
pub mod persist;
pub mod policy;
pub mod replication;
pub mod restore_tasks;
pub mod s3_server;
pub mod s3_sigv4;
pub mod scanner {
    pub use mox_cloud_kernel::scanner::*;
}
pub mod sigv4_middleware;
pub mod storage;
pub mod tagging;
pub mod versioning;

pub use bucket_analytics::{
    AccessTier, AggregationPeriod, AnalyticsManager, BucketMetrics, CostConfig, CostEstimate,
    MetricsSnapshot, SharedAnalytics,
};
pub use error::{S3Error, S3Result};
pub use inventory::{
    InventoryConfiguration, InventoryDestination, InventoryEncryption, InventoryFilter,
    InventoryFormat, InventoryFrequency, InventoryJob, InventoryJobStatus, InventoryManager,
    InventoryManifest, InventoryRecord, SharedInventory,
};
pub use lifecycle::{
    replication_status_blocks_lifecycle, CloudLifecycleStats, DeleteAllVersionsPlan,
    HotWarmColdLifecycle, LifecycleObjectMeta, LifecycleThresholds, SharedLifecycle, StorageClass,
    TransitionAction, TransitionPlan,
};
// 注意：lifecycle::ObjectReplicationStatus（枚举，生命周期门控用）与
// replication::ObjectReplicationStatus（结构体，完整复制记录）同名，
// 此处以别名 LifecycleReplicationStatus 导出，避免根命名空间冲突。
// 原始路径仍可通过 mox_cloud_s3_svc::lifecycle::ObjectReplicationStatus 访问。
pub use config::{
    FeatureFlags, InventoryConfig, LifecycleConfig, ReplicationConfig, S3ServiceConfig,
};
pub use lifecycle::ObjectReplicationStatus as LifecycleReplicationStatus;
pub use object_batch_ops::{
    BatchCopyRequest, BatchJob, BatchJobReport, BatchJobStatus, BatchObjectResult,
    BatchOperationManager, BatchOperationType, DeleteError, DeleteObjectIdentifier,
    DeleteObjectsRequest, DeleteObjectsResponse, DeletedObject, RestoreTier, SharedBatchOps,
    MAX_BATCH_OBJECTS,
};
pub use persist::{PersistSink, StoreCorePersist};
pub use replication::{
    DeadLetterEntry, ObjectReplicationStatus, ReplicationConfiguration, ReplicationDestination,
    ReplicationFilter, ReplicationManager, ReplicationMetrics, ReplicationRule, ReplicationStatus,
    ReplicationType, SharedReplication,
};
pub use s3_server::S3Server;
pub use scanner::{CapacityBudget, IoBudget, ScanBudget, ScanBudgetTracker, ScanStats, TimeBudget};
pub use storage::InMemoryStorageBackend;
#[cfg(feature = "rustfs_ecstore_backend")]
pub use storage::RustFsEcstoreBackend;

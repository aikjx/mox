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
//!         Replication（CRR/SRR 复制）、Inventory（存储桶清单）
//!
//! 签名：SigV4（复用 mox-standards sigv4）
//! ETag：CRC32C + MD5 + MPU concat-MD5（复用 mox-standards etag_crc32c）

pub mod acl;
pub mod bucket_analytics;
pub mod cors;
pub mod error;
pub mod etag;
pub mod glacier_adapter;
pub mod glacier_http;
pub mod inventory;
pub mod lifecycle;
pub mod mpu;
pub mod object_batch_ops;
pub mod policy;
pub mod replication;
pub mod restore_tasks;
pub mod s3_sigv4;
pub mod s3_server;
pub mod sigv4_middleware;
pub mod tagging;
pub mod versioning;

pub use bucket_analytics::{
    AccessTier, AggregationPeriod, BucketMetrics, CostConfig, CostEstimate, AnalyticsManager,
    MetricsSnapshot, SharedAnalytics,
};
pub use error::{S3Error, S3Result};
pub use inventory::{
    InventoryConfiguration, InventoryDestination, InventoryEncryption, InventoryFilter,
    InventoryFormat, InventoryFrequency, InventoryJob, InventoryJobStatus, InventoryManager,
    InventoryManifest, InventoryRecord, SharedInventory,
};
pub use lifecycle::{
    CloudLifecycleStats, HotWarmColdLifecycle, LifecycleObjectMeta, LifecycleThresholds,
    SharedLifecycle, StorageClass, TransitionAction, TransitionPlan,
};
pub use object_batch_ops::{
    BatchCopyRequest, BatchJob, BatchJobReport, BatchJobStatus, BatchObjectResult,
    BatchOperationManager, BatchOperationType, DeleteError, DeleteObjectIdentifier,
    DeleteObjectsRequest, DeleteObjectsResponse, DeletedObject, RestoreTier,
    SharedBatchOps, MAX_BATCH_OBJECTS,
};
pub use replication::{
    DeadLetterEntry, ObjectReplicationStatus, ReplicationConfiguration, ReplicationDestination,
    ReplicationFilter, ReplicationManager, ReplicationMetrics, ReplicationRule,
    ReplicationStatus, ReplicationType, SharedReplication,
};
pub use s3_server::S3Server;

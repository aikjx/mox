//! Xuanji Cloud Drive S3 Compatible Service — 100% 自研，零成品存储系统。
//!
//! 兼容 S3 v20060301，34 API 全自研：
//! - 桶管理：ListBuckets/CreateBucket/DeleteBucket/HeadBucket
//! - 对象：Put/Get/Delete/Head/Copy + ACL + Tagging
//! - 列表：ListObjects V1/V2 + ListObjectVersions
//! - MPU：Create/UploadPart/UploadPartCopy/Complete/Abort + ListParts + ListMultipartUploads
//! - 高级：Versioning、Bucket Policy、Lifecycle、CORS、批量 Delete
//!
//! 签名：SigV4（复用 xuanji-standards sigv4）
//! ETag：CRC32C + MD5 + MPU concat-MD5（复用 xuanji-standards etag_crc32c）

pub mod acl;
pub mod cors;
pub mod error;
pub mod etag;
pub mod glacier_adapter;
pub mod lifecycle;
pub mod mpu;
pub mod policy;
pub mod restore_tasks;
pub mod s3_sigv4;
pub mod s3_server;
pub mod sigv4_middleware;
pub mod tagging;
pub mod versioning;

pub use error::{S3Error, S3Result};
pub use lifecycle::{
    CloudLifecycleStats, HotWarmColdLifecycle, LifecycleObjectMeta, LifecycleThresholds,
    SharedLifecycle, StorageClass, TransitionAction, TransitionPlan,
};
pub use s3_server::S3Server;

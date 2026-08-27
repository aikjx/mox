// Copyright (c) 2026 璇玑 mox · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX Cloud Domain API — trait contracts for storage resource management.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CloudApiError {
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("quota exceeded: {0}")]
    QuotaExceeded(String),
    #[error("resource exists: {0}")]
    AlreadyExists(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("internal: {0}")]
    Internal(String),
}

pub type CloudApiResult<T> = Result<T, CloudApiError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceStatus { Pending, Active, Suspended, Deleted, Error }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudVolume {
    pub id: String,
    pub name: String,
    pub tenant_id: String,
    pub size_gb: u64,
    pub used_gb: u64,
    pub status: ResourceStatus,
    pub created_at: String,
}

impl CloudVolume {
    pub fn new(name: impl Into<String>, tenant_id: impl Into<String>, size_gb: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            tenant_id: tenant_id.into(),
            size_gb,
            used_gb: 0,
            status: ResourceStatus::Pending,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Bucket {
    pub id: String,
    pub name: String,
    pub tenant_id: String,
    pub object_count: u64,
    pub used_bytes: u64,
    pub status: ResourceStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileShare {
    pub id: String,
    pub name: String,
    pub tenant_id: String,
    pub capacity_gb: u64,
    pub used_gb: u64,
    pub protocol: String,
    pub status: ResourceStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub tenant_id: String,
    pub max_volumes: u32,
    pub max_buckets: u32,
    pub max_file_shares: u32,
    pub max_total_gb: u64,
    pub used_total_gb: u64,
}

#[async_trait]
pub trait VolumeManager: Send + Sync {
    async fn create(&self, volume: CloudVolume) -> CloudApiResult<CloudVolume>;
    async fn get(&self, id: &str) -> CloudApiResult<Option<CloudVolume>>;
    async fn delete(&self, id: &str) -> CloudApiResult<bool>;
    async fn resize(&self, id: &str, new_size_gb: u64) -> CloudApiResult<CloudVolume>;
    async fn list(&self, tenant_id: &str) -> CloudApiResult<Vec<CloudVolume>>;
}

#[async_trait]
pub trait BucketManager: Send + Sync {
    async fn create(&self, bucket: S3Bucket) -> CloudApiResult<S3Bucket>;
    async fn get(&self, id: &str) -> CloudApiResult<Option<S3Bucket>>;
    async fn delete(&self, id: &str) -> CloudApiResult<bool>;
    async fn list(&self, tenant_id: &str) -> CloudApiResult<Vec<S3Bucket>>;
}

#[async_trait]
pub trait QuotaManager: Send + Sync {
    async fn get_quota(&self, tenant_id: &str) -> CloudApiResult<ResourceQuota>;
    async fn set_quota(&self, quota: ResourceQuota) -> CloudApiResult<()>;
    async fn check_and_reserve(&self, tenant_id: &str, resource_type: &str, size_gb: u64) -> CloudApiResult<bool>;
    async fn release(&self, tenant_id: &str, size_gb: u64) -> CloudApiResult<()>;
}

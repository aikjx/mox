//! 云存储抽象接口
//!
//! 支持多种后端：本地文件系统、S3 兼容、MinIO 等

use async_trait::async_trait;
use crate::error::CloudError;

/// 对象元信息
#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub key: String,
    pub size: u64,
    pub content_type: Option<String>,
    pub last_modified: i64,
    pub etag: Option<String>,
}

/// 对象存储接口
#[async_trait]
pub trait ObjectStorage: Send + Sync {
    /// 上传对象
    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<ObjectMeta, CloudError>;

    /// 获取对象
    async fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Vec<u8>, CloudError>;

    /// 删除对象
    async fn delete_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<(), CloudError>;

    /// 列出对象
    async fn list_objects(
        &self,
        bucket: &str,
        prefix: Option<&str>,
        max_keys: i32,
    ) -> Result<Vec<ObjectMeta>, CloudError>;

    /// 创建存储桶
    async fn create_bucket(&self, bucket: &str) -> Result<(), CloudError>;

    /// 检查桶是否存在
    async fn bucket_exists(&self, bucket: &str) -> Result<bool, CloudError>;
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! S3 读路径 ReaderPipeline 接入层。
//!
//! 将 [`mox_cloud_kernel::reader_capability::ReaderPipeline`] 组合式读管线
//! 接入 S3 GetObject 路径，支持多后端并发取最快（hedged read）。
//!
//! ## 设计
//! - [`StorageBackendReader`]：包装单个 [`StorageBackend`]，实现 [`ReaderCapability`]，
//!   将 `get_chunk(chunk_id)` 适配为 `read_shard(shard_index)`。
//! - [`S3ReaderPipeline`]：持有多个 `StorageBackend`，按 chunk_id 动态构建
//!   `ReaderPipeline` 并调用 `read_first_success` 取最快成功结果。

use async_trait::async_trait;
use bytes::Bytes;
use mox_cloud_domain_traits::{BackendType, ChunkId, StorageBackend};
use mox_cloud_kernel::reader_capability::{
    ReadCapabilityError, ReaderCapability, ReaderPipeline,
};
use mox_cloud_kernel::ShardReadCost;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// StorageBackendReader — 单个 StorageBackend 的 ReaderCapability 适配
// ---------------------------------------------------------------------------

/// 包装单个 [`StorageBackend`]，实现 [`ReaderCapability`] trait。
///
/// 每个实例绑定一个 `chunk_id`，`read_shard` 调用时忽略 `shard_index`
/// （S3 对象读无分片概念），直接读取绑定的 chunk_id。
pub struct StorageBackendReader {
    backend: Arc<dyn StorageBackend>,
    chunk_id: String,
    endpoint_label: String,
}

impl StorageBackendReader {
    /// 创建新的 StorageBackendReader，绑定指定 chunk_id。
    pub fn new(backend: Arc<dyn StorageBackend>, chunk_id: &str) -> Self {
        let endpoint_label = format!("{}-{}", backend.backend_type(), chunk_id);
        Self {
            backend,
            chunk_id: chunk_id.to_string(),
            endpoint_label,
        }
    }

    /// 获取底层 StorageBackend 引用。
    pub fn backend(&self) -> &Arc<dyn StorageBackend> {
        &self.backend
    }

    /// 获取绑定的 chunk_id。
    pub fn chunk_id(&self) -> &str {
        &self.chunk_id
    }
}

#[async_trait]
impl ReaderCapability for StorageBackendReader {
    async fn read_shard(&self, shard_index: usize) -> Result<Bytes, ReadCapabilityError> {
        let cid = ChunkId::new(&self.chunk_id);
        let data = self
            .backend
            .get_chunk(&cid)
            .await
            .map_err(|e| ReadCapabilityError::ReadFailed(shard_index, e.to_string()))?;
        Ok(Bytes::from(data))
    }

    fn read_cost(&self) -> ShardReadCost {
        match self.backend.backend_type() {
            BackendType::LocalFs | BackendType::InMemory => ShardReadCost::Local,
            BackendType::S3Compatible => ShardReadCost::Remote,
            BackendType::RustFsEcstore => ShardReadCost::SameNode,
            BackendType::Other => ShardReadCost::Unknown,
        }
    }

    fn endpoint(&self) -> &str {
        &self.endpoint_label
    }

    /// StorageBackendReader 支持 hedged read（多后端并发取最快）。
    fn supports_hedged_read(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// S3ReaderPipeline — S3 读路径组合式管线封装
// ---------------------------------------------------------------------------

/// S3 读路径 ReaderPipeline 封装。
///
/// 持有多个 [`StorageBackend`]，`read_object(chunk_id)` 时为每个后端创建
/// [`StorageBackendReader`]，构建 `ReaderPipeline` 并调用 `read_first_success`
/// 并发取最快成功结果。
///
/// # 用法
/// ```ignore
/// let pipeline = S3ReaderPipeline::new(vec![backend1, backend2]);
/// let data = pipeline.read_object("obj:bucket:key:").await?;
/// ```
pub struct S3ReaderPipeline {
    backends: Vec<Arc<dyn StorageBackend>>,
}

impl S3ReaderPipeline {
    /// 创建空管线（无后端，read_object 将返回 AllFailed）。
    pub fn empty() -> Self {
        Self { backends: Vec::new() }
    }

    /// 创建包含指定后端列表的管线。
    pub fn new(backends: Vec<Arc<dyn StorageBackend>>) -> Self {
        Self { backends }
    }

    /// Builder：添加一个后端。
    pub fn with_backend(mut self, backend: Arc<dyn StorageBackend>) -> Self {
        self.backends.push(backend);
        self
    }

    /// 获取后端数量。
    pub fn backend_count(&self) -> usize {
        self.backends.len()
    }

    /// 读取指定 chunk_id 的对象数据，并发取最快成功结果。
    ///
    /// 为每个后端创建 [`StorageBackendReader`]，按 locality 成本排序后
    /// 并发发起读请求，第一个 `Ok` 立即返回；全部失败返回
    /// `ReadCapabilityError::AllFailed`。
    pub async fn read_object(&self, chunk_id: &str) -> Result<Vec<u8>, ReadCapabilityError> {
        if self.backends.is_empty() {
            return Err(ReadCapabilityError::AllFailed(0));
        }

        // 为每个后端创建 StorageBackendReader（绑定 chunk_id）
        let readers: Vec<Arc<dyn ReaderCapability>> = self
            .backends
            .iter()
            .map(|b| Arc::new(StorageBackendReader::new(b.clone(), chunk_id)) as Arc<dyn ReaderCapability>)
            .collect();

        // 构建 ReaderPipeline 并并发取最快
        let mut pipeline = ReaderPipeline::new();
        for r in readers {
            pipeline = pipeline.with_reader(r);
        }

        let bytes = pipeline.read_first_success(0).await?;
        Ok(bytes.to_vec())
    }

    /// 顺序读取（按 locality 成本排序后逐个尝试，用于调试或对比）。
    pub async fn read_object_sequential(
        &self,
        chunk_id: &str,
    ) -> Result<Vec<u8>, ReadCapabilityError> {
        if self.backends.is_empty() {
            return Err(ReadCapabilityError::AllFailed(0));
        }

        let readers: Vec<Arc<dyn ReaderCapability>> = self
            .backends
            .iter()
            .map(|b| Arc::new(StorageBackendReader::new(b.clone(), chunk_id)) as Arc<dyn ReaderCapability>)
            .collect();

        let mut pipeline = ReaderPipeline::new();
        for r in readers {
            pipeline = pipeline.with_reader(r);
        }

        let bytes = pipeline.read_sequential(0).await?;
        Ok(bytes.to_vec())
    }
}

impl std::fmt::Debug for S3ReaderPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3ReaderPipeline")
            .field("backend_count", &self.backends.len())
            .field(
                "backend_types",
                &self
                    .backends
                    .iter()
                    .map(|b| b.backend_type().to_string())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mox_cloud_domain_traits::{
        BackendCapabilities, ChunkInfo, ChunkListPage, StorageError,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    /// Mock StorageBackend：可配置延迟、成功率和返回数据。
    struct MockBackend {
        name: String,
        btype: BackendType,
        delay: Duration,
        should_fail: bool,
        payload: Vec<u8>,
        call_count: AtomicUsize,
    }

    impl MockBackend {
        fn new(name: &str, btype: BackendType, delay: Duration, should_fail: bool, payload: Vec<u8>) -> Self {
            Self {
                name: name.to_string(),
                btype,
                delay,
                should_fail,
                payload,
                call_count: AtomicUsize::new(0),
            }
        }
        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl StorageBackend for MockBackend {
        async fn put_chunk(
            &self,
            chunk_id: &ChunkId,
            data: &[u8],
        ) -> Result<ChunkInfo, StorageError> {
            Ok(ChunkInfo {
                chunk_id: chunk_id.clone(),
                size_bytes: data.len() as u64,
                created_at_ms: 0,
                checksum: String::new(),
            })
        }
        async fn get_chunk(&self, _chunk_id: &ChunkId) -> Result<Vec<u8>, StorageError> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            if self.delay > Duration::ZERO {
                tokio::time::sleep(self.delay).await;
            }
            if self.should_fail {
                Err(StorageError::IoError(format!("mock {} failure", self.name)))
            } else {
                Ok(self.payload.clone())
            }
        }
        async fn delete_chunk(&self, _chunk_id: &ChunkId) -> Result<bool, StorageError> {
            Ok(true)
        }
        async fn chunk_exists(&self, _chunk_id: &ChunkId) -> Result<bool, StorageError> {
            Ok(true)
        }
        async fn list_chunks(
            &self,
            _prefix: &str,
            _marker: Option<&str>,
            _limit: u32,
        ) -> Result<ChunkListPage, StorageError> {
            Ok(ChunkListPage {
                items: vec![],
                next_marker: None,
                is_truncated: false,
            })
        }
        fn backend_type(&self) -> BackendType {
            self.btype
        }
        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }
        fn name(&self) -> &'static str {
            "mock-backend"
        }
    }

    // ----- 测试 1：StorageBackendReader 实现 ReaderCapability trait -----

    #[tokio::test]
    async fn test_storage_backend_reader_implements_capability() {
        let payload = b"hello-s3".to_vec();
        let backend = Arc::new(MockBackend::new(
            "local-1",
            BackendType::LocalFs,
            Duration::ZERO,
            false,
            payload.clone(),
        ));
        let reader = StorageBackendReader::new(backend, "obj:test:key:");

        // read_shard 成功
        let result = reader.read_shard(0).await.unwrap();
        assert_eq!(result.as_ref(), payload.as_slice());

        // read_cost 根据 backend_type 映射
        assert_eq!(reader.read_cost(), ShardReadCost::Local);

        // endpoint 包含后端类型和 chunk_id
        assert!(reader.endpoint().contains("local-fs"));
        assert!(reader.endpoint().contains("obj:test:key:"));

        // supports_hedged_read = true
        assert!(reader.supports_hedged_read());

        // chunk_id 访问器
        assert_eq!(reader.chunk_id(), "obj:test:key:");
    }

    #[test]
    fn test_storage_backend_reader_cost_mapping() {
        let make = |btype| {
            let b = Arc::new(MockBackend::new("t", btype, Duration::ZERO, false, vec![]));
            StorageBackendReader::new(b, "cid").read_cost()
        };
        assert_eq!(make(BackendType::LocalFs), ShardReadCost::Local);
        assert_eq!(make(BackendType::InMemory), ShardReadCost::Local);
        assert_eq!(make(BackendType::S3Compatible), ShardReadCost::Remote);
        assert_eq!(make(BackendType::RustFsEcstore), ShardReadCost::SameNode);
        assert_eq!(make(BackendType::Other), ShardReadCost::Unknown);
    }

    #[tokio::test]
    async fn test_storage_backend_reader_error_conversion() {
        let backend = Arc::new(MockBackend::new(
            "fail-backend",
            BackendType::S3Compatible,
            Duration::ZERO,
            true,
            vec![],
        ));
        let reader = StorageBackendReader::new(backend, "obj:missing:");

        let result = reader.read_shard(3).await;
        match result {
            Err(ReadCapabilityError::ReadFailed(idx, msg)) => {
                assert_eq!(idx, 3);
                assert!(msg.contains("mock fail-backend failure"));
            }
            other => panic!("expected ReadFailed, got {other:?}"),
        }
    }

    // ----- 测试 2：S3ReaderPipeline 单后端读取 -----

    #[tokio::test]
    async fn test_s3_reader_pipeline_single_backend() {
        let payload = b"single-backend-data".to_vec();
        let backend = Arc::new(MockBackend::new(
            "mem-1",
            BackendType::InMemory,
            Duration::ZERO,
            false,
            payload.clone(),
        ));
        let pipeline = S3ReaderPipeline::new(vec![backend]);

        assert_eq!(pipeline.backend_count(), 1);

        let data = pipeline.read_object("obj:bucket:key:").await.unwrap();
        assert_eq!(data, payload);
    }

    #[tokio::test]
    async fn test_s3_reader_pipeline_empty() {
        let pipeline = S3ReaderPipeline::empty();
        assert_eq!(pipeline.backend_count(), 0);

        let result = pipeline.read_object("any").await;
        assert!(matches!(result, Err(ReadCapabilityError::AllFailed(0))));
    }

    // ----- 测试 3：S3ReaderPipeline 多后端并发取最快 -----

    #[tokio::test]
    async fn test_s3_reader_pipeline_concurrent_fastest() {
        let slow_payload = b"slow-result".to_vec();
        let fast_payload = b"fast-result".to_vec();

        let slow = Arc::new(MockBackend::new(
            "slow-local",
            BackendType::LocalFs,
            Duration::from_secs(10), // 很慢
            false,
            slow_payload,
        ));
        let fast = Arc::new(MockBackend::new(
            "fast-remote",
            BackendType::S3Compatible,
            Duration::ZERO, // 立即返回
            false,
            fast_payload.clone(),
        ));

        let pipeline = S3ReaderPipeline::new(vec![slow, fast]);
        assert_eq!(pipeline.backend_count(), 2);

        // 并发读应取最快成功的（fast-remote）
        let data = pipeline.read_object("obj:test:key:").await.unwrap();
        assert_eq!(data, fast_payload);
    }

    #[tokio::test]
    async fn test_s3_reader_pipeline_all_fail() {
        let b1 = Arc::new(MockBackend::new(
            "fail-1", BackendType::LocalFs, Duration::ZERO, true, vec![],
        ));
        let b2 = Arc::new(MockBackend::new(
            "fail-2", BackendType::S3Compatible, Duration::ZERO, true, vec![],
        ));
        let pipeline = S3ReaderPipeline::new(vec![b1, b2]);

        let result = pipeline.read_object("obj:missing:").await;
        assert!(result.is_err());
        match result {
            Err(ReadCapabilityError::ReadFailed(_, msg)) => {
                assert!(msg.contains("mock fail-"));
            }
            Err(ReadCapabilityError::AllFailed(_)) => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_s3_reader_pipeline_sequential() {
        let b1 = Arc::new(MockBackend::new(
            "first-fail", BackendType::LocalFs, Duration::ZERO, true, vec![],
        ));
        let b2 = Arc::new(MockBackend::new(
            "second-ok", BackendType::S3Compatible, Duration::ZERO, false, b"ok".to_vec(),
        ));
        let pipeline = S3ReaderPipeline::new(vec![b1, b2]);

        // 顺序读：第一个失败后尝试第二个，成功返回
        let data = pipeline.read_object_sequential("obj:k:").await.unwrap();
        assert_eq!(data, b"ok".to_vec());
    }

    #[test]
    fn test_s3_reader_pipeline_debug() {
        let b = Arc::new(MockBackend::new(
            "dbg", BackendType::InMemory, Duration::ZERO, false, vec![],
        ));
        let pipeline = S3ReaderPipeline::new(vec![b]);
        let dbg = format!("{pipeline:?}");
        assert!(dbg.contains("backend_count"));
        assert!(dbg.contains("in-memory"));
    }
}

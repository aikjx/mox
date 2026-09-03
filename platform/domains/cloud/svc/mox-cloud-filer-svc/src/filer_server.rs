// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! FilerServer：管理当前活跃 MetaStorageProvider；支持切换后端。
//!
//! 对象层：默认桥接真实 S3（通过 `mox-cloud-store-core` 的自研 SigV4 客户端），
//! 仅在 `STORAGE_BACKEND=memory` 时使用内存实现。

use mox_cloud_kernel::buffer_pool::BufferPool;
use parking_lot::Mutex;
use std::{collections::BTreeMap, sync::Arc};

use crate::{
    dir_entry_cache::DirEntryCache,
    error::{FilerError, FilerResult},
    file_lock::FileLockManager,
    meta_pg_citus::PgCitusMeta,
    meta_redis::RedisMeta,
    meta_sqlite::SqliteMeta,
    meta_trait::{MetaBackend, MetaStorageProvider, META_BACKENDS},
    quota_manager::QuotaManager,
    snapshot_filer::SnapshotManager,
};

pub struct FilerServer {
    pub active: Mutex<Arc<dyn MetaStorageProvider>>,
    active_name: Mutex<String>,
    registry: Mutex<BTreeMap<String, Arc<dyn MetaStorageProvider>>>,
    pub object: Arc<dyn ObjectStorage>,
    /// 目录项缓存
    pub dir_cache: Arc<DirEntryCache>,
    /// 文件锁管理器
    pub file_locks: Arc<FileLockManager>,
    /// 配额管理器
    pub quota: Arc<QuotaManager>,
    /// 快照管理器
    pub snapshots: Arc<SnapshotManager>,
    /// 四层分档缓冲池（PooledBuffer 推广，热点路径复用分配）
    pub buffer_pool: Arc<BufferPool>,
}

/// 对象存储接口（S3 兼容）。
pub trait ObjectStorage: Send + Sync {
    fn put(&self, bucket: &str, key: &str, data: &[u8]) -> FilerResult<()>;
    fn get(&self, bucket: &str, key: &str) -> FilerResult<Vec<u8>>;
    fn list(&self, bucket: &str) -> FilerResult<Vec<String>>;
    fn delete(&self, bucket: &str, key: &str) -> FilerResult<()>;
    fn head(&self, bucket: &str, key: &str) -> FilerResult<u64>;
}

impl FilerServer {
    /// 创建 FilerServer，对象存储默认使用真实 S3（从环境变量读取配置）。
    ///
    /// 若 `STORAGE_BACKEND=memory` 则使用内存实现；若 S3 配置缺失且非 memory，
    /// 返回真实错误而非静默降级。
    pub fn new(provider: Arc<dyn MetaStorageProvider>) -> FilerResult<Self> {
        let object = default_object_storage()?;
        Self::with_object_internal(provider, object)
    }

    /// 创建 FilerServer，显式指定对象存储后端。
    pub fn with_object(
        provider: Arc<dyn MetaStorageProvider>,
        obj: Arc<dyn ObjectStorage>,
    ) -> Self {
        Self::with_object_internal(provider, obj)
            .expect("with_object_internal 不应失败（已提供 object）")
    }

    fn with_object_internal(
        provider: Arc<dyn MetaStorageProvider>,
        object: Arc<dyn ObjectStorage>,
    ) -> FilerResult<Self> {
        let mut reg: BTreeMap<String, Arc<dyn MetaStorageProvider>> = BTreeMap::new();
        reg.insert("sqlite".into(), Arc::new(SqliteMeta::new()) as _);
        reg.insert("pg_citus".into(), Arc::new(PgCitusMeta::new()) as _);
        // Redis 后端：默认注册内存实现（registry 仅用于 switch_backend；
        // 生产环境应通过 RedisMeta::new() 直接构造真实后端并设为 active）。
        reg.insert("redis".into(), Arc::new(RedisMeta::new_in_memory()) as _);
        Ok(Self {
            active: Mutex::new(provider),
            active_name: Mutex::new("sqlite".into()),
            registry: Mutex::new(reg),
            object,
            dir_cache: Arc::new(DirEntryCache::new()),
            file_locks: Arc::new(FileLockManager::new()),
            quota: Arc::new(QuotaManager::new()),
            snapshots: Arc::new(SnapshotManager::new()),
            buffer_pool: Arc::new(BufferPool::with_default()),
        })
    }

    /// 按 provider_name 切换后端（来自 META_BACKENDS）。
    pub fn switch_backend(&self, provider_name: &str) -> FilerResult<()> {
        if !META_BACKENDS.contains(&provider_name) {
            return Err(FilerError::BackendSwitch(format!(
                "unknown backend: {provider_name}; allowed = {META_BACKENDS:?}"
            )));
        }
        let reg = self.registry.lock();
        let next = reg.get(provider_name).cloned().ok_or_else(|| {
            FilerError::BackendSwitch(format!("backend {provider_name} not registered"))
        })?;
        *self.active.lock() = next;
        *self.active_name.lock() = provider_name.to_string();
        Ok(())
    }

    pub fn active_name(&self) -> String {
        self.active_name.lock().clone()
    }

    pub fn put_object_chunk(&self, bucket: &str, key: &str, data: &[u8]) -> FilerResult<()> {
        self.object.put(bucket, key, data)
    }
    pub fn get_object_chunk(&self, bucket: &str, key: &str) -> FilerResult<Vec<u8>> {
        self.object.get(bucket, key)
    }
    pub fn list_objects(&self, bucket: &str) -> FilerResult<Vec<String>> {
        self.object.list(bucket)
    }
    pub fn delete_object(&self, bucket: &str, key: &str) -> FilerResult<()> {
        self.object.delete(bucket, key)
    }
    pub fn head_object(&self, bucket: &str, key: &str) -> FilerResult<u64> {
        self.object.head(bucket, key)
    }
    pub fn load_object_chunks(&self) -> bool {
        true
    }
}

impl MetaBackend for FilerServer {
    fn name() -> &'static str {
        "filer_server"
    }
}

// ============================================================================
// 真实 S3 对象存储（桥接 mox-cloud-store-core 的 S3Client）
// ============================================================================

/// 基于 `mox_cloud_store_core::S3Client` 的真实 S3 对象存储。
///
/// 使用独立 tokio Runtime `block_on` 封装异步调用，适配 filer 的同步 ObjectStorage trait。
pub struct S3ObjectStorage {
    client: Arc<mox_cloud_store_core::S3Client>,
    rt: tokio::runtime::Runtime,
    default_bucket: String,
    /// 四层分档缓冲池（PooledBuffer 推广）
    buffer_pool: Arc<BufferPool>,
}

impl S3ObjectStorage {
    /// 从环境变量构建 S3 客户端。
    ///
    /// 环境变量：
    /// - `S3_ENDPOINT`（必填）
    /// - `S3_REGION`（默认 `us-east-1`）
    /// - `S3_ACCESS_KEY`（必填）
    /// - `S3_SECRET_KEY`（必填）
    /// - `S3_BUCKET`（默认 `mox-filer`）
    pub fn from_env() -> FilerResult<Self> {
        let endpoint = std::env::var("S3_ENDPOINT")
            .map_err(|_| FilerError::Other("S3_ENDPOINT 环境变量未设置".into()))?;
        let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into());
        let access_key = std::env::var("S3_ACCESS_KEY")
            .map_err(|_| FilerError::Other("S3_ACCESS_KEY 环境变量未设置".into()))?;
        let secret_key = std::env::var("S3_SECRET_KEY")
            .map_err(|_| FilerError::Other("S3_SECRET_KEY 环境变量未设置".into()))?;
        let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "mox-filer".into());

        let cfg = mox_cloud_store_core::S3ClientConfig {
            endpoint,
            region,
            access_key,
            secret_key,
            bucket: bucket.clone(),
            force_path_style: true,
        };
        let client = mox_cloud_store_core::S3Client::new(&cfg)
            .map_err(|e| FilerError::Other(format!("S3 客户端构建失败: {e}")))?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| FilerError::Other(format!("创建 S3 runtime 失败: {e}")))?;
        Ok(Self {
            client: Arc::new(client),
            rt,
            default_bucket: bucket,
            buffer_pool: Arc::new(BufferPool::with_default()),
        })
    }

    fn logical_key(bucket: &str, key: &str) -> String {
        format!("{}/{}", bucket.trim_matches('/'), key.trim_start_matches('/'))
    }
}

impl ObjectStorage for S3ObjectStorage {
    fn put(&self, bucket: &str, key: &str, data: &[u8]) -> FilerResult<()> {
        let logical = Self::logical_key(bucket, key);
        let logical_mv = logical.clone();
        let client = self.client.clone();
        // PooledBuffer 推广：从缓冲池获取缓冲区，避免重复 Vec 分配
        let mut pooled = self.buffer_pool.acquire(data.len());
        pooled.extend_from_slice(data);
        let data_vec = pooled.into_vec();
        self.rt
            .block_on(async move {
                client.put_object(&logical_mv, "application/octet-stream", &data_vec).await
            })
            .map_err(|e| FilerError::Other(format!("S3 PUT {logical} 失败: {e}")))
    }

    fn get(&self, bucket: &str, key: &str) -> FilerResult<Vec<u8>> {
        let logical = Self::logical_key(bucket, key);
        let logical_mv = logical.clone();
        let client = self.client.clone();
        self.rt
            .block_on(async move { client.get_object(&logical_mv).await })
            .map_err(|e| match e {
                mox_base_store_core::StoreError::NotFound { .. } => FilerError::NotFound,
                other => FilerError::Other(format!("S3 GET {logical} 失败: {other}")),
            })
    }

    fn list(&self, bucket: &str) -> FilerResult<Vec<String>> {
        let prefix = format!("{}/", bucket.trim_matches('/'));
        let prefix_mv = prefix.clone();
        let client = self.client.clone();
        let keys = self
            .rt
            .block_on(async move { client.list_objects(&prefix_mv).await })
            .map_err(|e| FilerError::Other(format!("S3 LIST {prefix} 失败: {e}")))?;
        let out: Vec<String> = keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(&prefix).map(|s| s.to_string()))
            .collect();
        Ok(out)
    }

    fn delete(&self, bucket: &str, key: &str) -> FilerResult<()> {
        let logical = Self::logical_key(bucket, key);
        let logical_mv = logical.clone();
        let client = self.client.clone();
        self.rt
            .block_on(async move { client.delete_object(&logical_mv).await })
            .map_err(|e| FilerError::Other(format!("S3 DELETE {logical} 失败: {e}")))
    }

    fn head(&self, bucket: &str, key: &str) -> FilerResult<u64> {
        let logical = Self::logical_key(bucket, key);
        let logical_mv = logical.clone();
        let client = self.client.clone();
        let info = self
            .rt
            .block_on(async move { client.head_object(&logical_mv).await })
            .map_err(|e| FilerError::Other(format!("S3 HEAD {logical} 失败: {e}")))?;
        match info {
            Some(i) => Ok(i.size_bytes),
            None => Err(FilerError::NotFound),
        }
    }
}

// ============================================================================
// 内存对象存储（仅 STORAGE_BACKEND=memory 时使用）
// ============================================================================

/// In-memory object storage（仅在 `STORAGE_BACKEND=memory` 时作为回退）。
#[derive(Debug)]
pub struct InMemoryObjectStorage {
    inner: Mutex<BTreeMap<(String, String), Vec<u8>>>,
    /// 四层分档缓冲池（PooledBuffer 推广）
    buffer_pool: Arc<BufferPool>,
}

impl InMemoryObjectStorage {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(BTreeMap::new()),
            buffer_pool: Arc::new(BufferPool::with_default()),
        }
    }

    /// 创建带指定缓冲池的实例
    pub fn with_buffer_pool(buffer_pool: Arc<BufferPool>) -> Self {
        Self { inner: Mutex::new(BTreeMap::new()), buffer_pool }
    }
}

impl Default for InMemoryObjectStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectStorage for InMemoryObjectStorage {
    fn put(&self, bucket: &str, key: &str, data: &[u8]) -> FilerResult<()> {
        // PooledBuffer 推广：从缓冲池获取缓冲区
        let mut pooled = self.buffer_pool.acquire(data.len());
        pooled.extend_from_slice(data);
        self.inner.lock().insert((bucket.into(), key.into()), pooled.into_vec());
        Ok(())
    }
    fn get(&self, bucket: &str, key: &str) -> FilerResult<Vec<u8>> {
        self.inner
            .lock()
            .get(&(bucket.into(), key.into()))
            .cloned()
            .ok_or(FilerError::NotFound)
    }
    fn list(&self, bucket: &str) -> FilerResult<Vec<String>> {
        let s = self.inner.lock();
        let mut out = Vec::new();
        for (b, k) in s.keys() {
            if b == bucket {
                out.push(k.clone());
            }
        }
        out.sort();
        Ok(out)
    }
    fn delete(&self, bucket: &str, key: &str) -> FilerResult<()> {
        self.inner.lock().remove(&(bucket.into(), key.into()));
        Ok(())
    }
    fn head(&self, bucket: &str, key: &str) -> FilerResult<u64> {
        self.inner
            .lock()
            .get(&(bucket.into(), key.into()))
            .map(|v| v.len() as u64)
            .ok_or(FilerError::NotFound)
    }
}

// ============================================================================
// 默认对象存储选择
// ============================================================================

/// 根据环境变量选择对象存储后端。
///
/// - `STORAGE_BACKEND=memory` → InMemoryObjectStorage
/// - 其他（默认）→ S3ObjectStorage（从 S3_* 环境变量读取配置）
/// - S3 配置缺失时返回真实错误，不静默降级
fn default_object_storage() -> FilerResult<Arc<dyn ObjectStorage>> {
    let backend = std::env::var("STORAGE_BACKEND").unwrap_or_else(|_| "s3".to_string());
    match backend.to_ascii_lowercase().as_str() {
        "memory" | "mem" | "in-memory" => Ok(Arc::new(InMemoryObjectStorage::new())),
        _ => {
            let s3 = S3ObjectStorage::from_env()?;
            Ok(Arc::new(s3))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_backend_three_rounds() {
        // 测试使用 memory 后端，避免依赖 S3 配置
        std::env::set_var("STORAGE_BACKEND", "memory");
        let srv = FilerServer::new(Arc::new(SqliteMeta::new())).unwrap();
        for b in META_BACKENDS {
            srv.switch_backend(b).unwrap();
            assert_eq!(srv.active_name(), *b);
        }
        assert!(srv.switch_backend("nope").is_err());
    }

    #[test]
    fn in_memory_storage_roundtrip() {
        let s = InMemoryObjectStorage::new();
        s.put("bkt", "key1", b"hello").unwrap();
        assert_eq!(s.get("bkt", "key1").unwrap(), b"hello");
        assert_eq!(s.list("bkt").unwrap(), vec!["key1".to_string()]);
        assert_eq!(s.head("bkt", "key1").unwrap(), 5);
        s.delete("bkt", "key1").unwrap();
        assert!(s.get("bkt", "key1").is_err());
    }

    #[test]
    fn s3_missing_config_returns_error() {
        // 确保 S3_* 环境变量未设置
        std::env::remove_var("S3_ENDPOINT");
        std::env::remove_var("S3_ACCESS_KEY");
        std::env::remove_var("S3_SECRET_KEY");
        std::env::set_var("STORAGE_BACKEND", "s3");
        let result = S3ObjectStorage::from_env();
        assert!(result.is_err(), "缺少 S3 配置应返回错误");
    }

    // ----- PooledBuffer 推广集成测试 -----

    #[test]
    fn pooled_buffer_in_memory_storage_put_get() {
        // 验证 InMemoryObjectStorage 使用 PooledBuffer 进行 put/get 往返
        let pool = Arc::new(BufferPool::with_default());
        let s = InMemoryObjectStorage::with_buffer_pool(pool.clone());

        // 写入数据（内部使用 PooledBuffer acquire + into_vec）
        let payload = b"pooled-buffer-test-data-0123456789";
        s.put("bkt", "file.bin", payload).unwrap();

        // 读取验证
        let data = s.get("bkt", "file.bin").unwrap();
        assert_eq!(data, payload);

        // 缓冲池应记录分配（put 路径 acquire 了一个 buffer）
        let stats = pool.stats();
        assert!(stats.total_allocated > 0, "pool should have allocations from put");
        assert!(stats.current_in_use == 0, "buffers should be returned after into_vec");
    }

    #[test]
    fn pooled_buffer_filer_server_has_pool() {
        // 验证 FilerServer 包含 buffer_pool 字段且默认可用
        std::env::set_var("STORAGE_BACKEND", "memory");
        let srv = FilerServer::new(Arc::new(SqliteMeta::new())).unwrap();

        // buffer_pool 字段存在且可访问
        let stats = srv.buffer_pool.stats();
        assert_eq!(stats.current_in_use, 0, "fresh pool should have no in-use buffers");

        // 可以从 pool 获取缓冲区
        let mut buf = srv.buffer_pool.acquire(1024);
        assert!(buf.capacity() >= 1024);
        buf.extend_from_slice(b"filer-pool-test");
        assert_eq!(&buf[..], b"filer-pool-test");
        drop(buf);

        // 释放后缓冲区回到池中
        let stats_after = srv.buffer_pool.stats();
        assert_eq!(stats_after.current_in_use, 0);
        // total_reused 为 u64，恒 >= 0，无需断言
    }
}

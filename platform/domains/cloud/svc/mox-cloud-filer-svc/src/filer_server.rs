// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! FilerServer：管理当前活跃 MetaStorageProvider；支持切换后端。
//!
//! 同时支持对象层：简化本地 ObjectStorage trait put/get/list + InMemoryObjectStorage 实现
//! （对应任务要求的 S3 模拟）。

use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::{FilerError, FilerResult};
use crate::meta_pg_citus::PgCitusMeta;
use crate::meta_redis::RedisMeta;
use crate::meta_sqlite::SqliteMeta;
use crate::meta_trait::{MetaBackend, MetaStorageProvider, META_BACKENDS};

pub struct FilerServer {
    pub active: Mutex<Arc<dyn MetaStorageProvider>>,
    active_name: Mutex<String>,
    registry: Mutex<BTreeMap<String, Arc<dyn MetaStorageProvider>>>,
    pub object: Arc<dyn ObjectStorage>,
}

/// 简化的对象存储接口（S3 模拟）。
pub trait ObjectStorage: Send + Sync {
    fn put(&self, bucket: &str, key: &str, data: &[u8]) -> FilerResult<()>;
    fn get(&self, bucket: &str, key: &str) -> FilerResult<Vec<u8>>;
    fn list(&self, bucket: &str) -> FilerResult<Vec<String>>;
}

impl FilerServer {
    pub fn new(provider: Arc<dyn MetaStorageProvider>) -> Self {
        let mut reg: BTreeMap<String, Arc<dyn MetaStorageProvider>> = BTreeMap::new();
        reg.insert("sqlite".into(), Arc::new(SqliteMeta::new()) as _);
        reg.insert("pg_citus".into(), Arc::new(PgCitusMeta::new()) as _);
        reg.insert("redis".into(), Arc::new(RedisMeta::new()) as _);
        Self {
            active: Mutex::new(provider),
            active_name: Mutex::new("sqlite".into()),
            registry: Mutex::new(reg),
            object: Arc::new(InMemoryObjectStorage::new()),
        }
    }

    pub fn with_object(
        provider: Arc<dyn MetaStorageProvider>,
        obj: Arc<dyn ObjectStorage>,
    ) -> Self {
        let mut s = Self::new(provider);
        s.object = obj;
        s
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
    pub fn load_object_chunks(&self) -> bool {
        true
    }
}

impl MetaBackend for FilerServer {
    fn name() -> &'static str {
        "filer_server"
    }
}

/// In-memory object storage (S3 mock).
#[derive(Debug, Default)]
pub struct InMemoryObjectStorage {
    inner: Mutex<BTreeMap<(String, String), Vec<u8>>>,
}

impl InMemoryObjectStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ObjectStorage for InMemoryObjectStorage {
    fn put(&self, bucket: &str, key: &str, data: &[u8]) -> FilerResult<()> {
        self.inner
            .lock()
            .insert((bucket.into(), key.into()), data.to_vec());
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_backend_three_rounds() {
        let srv = FilerServer::new(Arc::new(SqliteMeta::new()));
        for b in META_BACKENDS {
            srv.switch_backend(b).unwrap();
            assert_eq!(srv.active_name(), *b);
        }
        assert!(srv.switch_backend("nope").is_err());
    }
}

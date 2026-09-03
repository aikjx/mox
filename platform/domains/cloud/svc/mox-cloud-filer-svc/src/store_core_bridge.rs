// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! store-core 桥接：将 [`mox_cloud_store_core`] 的真实后端（FS/S3）
//! 适配为 filer 的同步 [`ObjectStorage`] 契约。
//!
//! store-core 三路物理口（`ObjectStore`/`KvStore`/`ObjectStreamWriter`）是
//! async trait；本桥接用**独立 tokio Runtime** `block_on` 封装为同步调用，
//! 使 filer 的同步 POSIX 层可直接接入内容寻址去重后端。
//!
//! 逻辑路径映射：`{bucket}/{key}`（与 store-core 的 key 同构，FS/S3 可互换）。

use crate::error::{FilerError, FilerResult};
use crate::ObjectStorage;
use bytes::Bytes;
use mox_cloud_store_core::{list_object_refs, StoreBackend};
use std::sync::Arc;

/// 基于 store-core 真实后端的对象存储桥接。
pub struct StoreCoreObjectStorage {
    backend: Arc<StoreBackend>,
    rt: tokio::runtime::Runtime,
}

impl StoreCoreObjectStorage {
    /// 用装配好的后端构造桥接。
    ///
    /// 持有独立 current-thread Runtime，避免污染调用方运行时；
    /// 注意：勿在已运行的 tokio Runtime 内部调用本对象方法（block_on 限制）。
    pub fn new(backend: StoreBackend) -> FilerResult<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| FilerError::Other(format!("创建桥接 runtime 失败: {e}")))?;
        Ok(Self {
            backend: Arc::new(backend),
            rt,
        })
    }

    /// 后端引用（供上层直连 / 管理面）
    pub fn backend(&self) -> &Arc<StoreBackend> {
        &self.backend
    }

    /// 构造逻辑路径：`{bucket}/{key}`（bucket 去斜杠，key 去前导斜杠）
    fn logical_path(bucket: &str, key: &str) -> String {
        format!(
            "{}/{}",
            bucket.trim_matches('/'),
            key.trim_start_matches('/')
        )
    }
}

impl ObjectStorage for StoreCoreObjectStorage {
    fn put(&self, bucket: &str, key: &str, data: &[u8]) -> FilerResult<()> {
        let path = Self::logical_path(bucket, key);
        let backend = self.backend.clone();
        self.rt
            .block_on(backend.object.put(
                &path,
                "application/octet-stream",
                Bytes::copy_from_slice(data),
            ))
            .map_err(|e| FilerError::Other(format!("put {path} 失败: {e}")))?;
        Ok(())
    }

    fn get(&self, bucket: &str, key: &str) -> FilerResult<Vec<u8>> {
        let path = Self::logical_path(bucket, key);
        let backend = self.backend.clone();
        let data = self
            .rt
            .block_on(backend.object.get(&path))
            .map_err(|e| match e {
                mox_base_store_core::StoreError::NotFound { .. } => FilerError::NotFound,
                other => FilerError::Other(format!("get {path} 失败: {other}")),
            })?;
        Ok(data.to_vec())
    }

    fn list(&self, bucket: &str) -> FilerResult<Vec<String>> {
        let prefix = format!("{}/", bucket.trim_matches('/'));
        let data_dir = self.backend.data_dir.clone();
        let refs = self
            .rt
            .block_on(list_object_refs(&data_dir))
            .map_err(|e| FilerError::Other(format!("list {bucket} 失败: {e}")))?;
        let mut out: Vec<String> = refs
            .into_iter()
            .filter(|(path, _)| path.starts_with(&prefix))
            .map(|(path, _)| path[prefix.len()..].to_string())
            .collect();
        out.sort();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_cloud_store_core::{create_backend, BackendKind, StoreConfig};

    fn bridge(dir: &std::path::Path) -> StoreCoreObjectStorage {
        let cfg = StoreConfig {
            kind: BackendKind::Fs,
            data_dir: dir.to_path_buf(),
            ..Default::default()
        };
        StoreCoreObjectStorage::new(create_backend(&cfg).unwrap()).unwrap()
    }

    #[test]
    fn put_get_list_roundtrip_on_real_backend() {
        let dir = tempfile::tempdir().unwrap();
        let obj = bridge(dir.path());

        obj.put("docs", "a.md", "# 标题".as_bytes()).unwrap();
        obj.put("docs", "b.md", b"content-b").unwrap();
        obj.put("media", "c.png", b"png-data").unwrap();

        // get 回读
        assert_eq!(obj.get("docs", "a.md").unwrap(), "# 标题".as_bytes().to_vec());
        // list 按 bucket 过滤
        let docs = obj.list("docs").unwrap();
        assert_eq!(docs, vec!["a.md".to_string(), "b.md".to_string()]);
        let media = obj.list("media").unwrap();
        assert_eq!(media, vec!["c.png".to_string()]);
    }

    #[test]
    fn get_missing_returns_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let obj = bridge(dir.path());
        assert!(matches!(
            obj.get("nope", "missing"),
            Err(FilerError::NotFound)
        ));
    }
}

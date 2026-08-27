// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! FUSE 客户端：自研逻辑（无需系统 libfuse/dokan 即可测试，模拟 mount/ls/write/s3）。
//!
//! FUSE 内部结构封装：`mount()` 记录 URL + mountpoint 到结构体状态；`ls()` 列出虚拟根；
//! `write_file()` 把字节写往 filer_server 的 ObjectStorage；`s3_visible_key_list()` 返回对象列表。

use std::sync::{Arc, Mutex};

use crate::error::FilerResult;
use crate::filer_server::{FilerServer, ObjectStorage};
use crate::meta_trait::{Attr, MetaStorageProvider, META_BACKENDS};

#[derive(Debug, Clone, Default)]
pub struct NameAttr {
    pub name: String,
    pub ino: u64,
    pub size: u64,
    pub mode: u32,
}

#[derive(Debug, Default)]
pub struct FuseClientInner {
    pub mounted: bool,
    pub url: String,
    pub mountpoint: String,
    pub files: std::collections::BTreeMap<String, Vec<u8>>,
}

#[derive(Clone)]
pub struct FuseClient {
    pub inner: Arc<Mutex<FuseClientInner>>,
    pub s3_backend: Arc<dyn ObjectStorage>,
    pub bucket: String,
    pub provider: Option<Arc<dyn MetaStorageProvider>>,
}

impl FuseClient {
    pub fn new_with_s3(s3_backend: Arc<dyn ObjectStorage>, bucket: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(FuseClientInner::default())),
            s3_backend,
            bucket: bucket.into(),
            provider: None,
        }
    }

    pub fn attach_server(self, server: &FilerServer) -> Self {
        Self {
            inner: self.inner,
            s3_backend: server.object.clone(),
            bucket: "mox-bucket".into(),
            provider: Some(server.active.lock().clone()),
        }
    }

    pub fn mount(&mut self, url: &str, mountpoint: &str) -> FilerResult<()> {
        let mut i = self.inner.lock().unwrap();
        i.mounted = true;
        i.url = url.into();
        i.mountpoint = mountpoint.into();
        // 模拟：ls 根目录时保证为空目录（已初始化）。
        Ok(())
    }

    pub fn is_mounted(&self) -> bool {
        self.inner.lock().unwrap().mounted
    }

    pub fn ls(&self) -> Vec<NameAttr> {
        let i = self.inner.lock().unwrap();
        i.files
            .keys()
            .map(|k| NameAttr {
                name: k.clone(),
                ino: stable_hash(k),
                size: i.files[k].len() as u64,
                mode: 0o100644,
            })
            .collect()
    }

    pub fn write_file(&self, path: &str, bytes: &[u8]) {
        let key = path.trim_start_matches('/');
        // 写入内存 filer
        {
            let mut i = self.inner.lock().unwrap();
            i.files.insert(key.to_string(), bytes.to_vec());
        }
        // 同时写往 S3 backend（对象存储），使 s3 list 可见。
        let _ = self.s3_backend.put(&self.bucket, key, bytes);
    }

    pub fn s3_visible_key_list(&self) -> Vec<String> {
        self.s3_backend.list(&self.bucket).unwrap_or_default()
    }

    /// In-memory meta roundtrip (test helper).
    pub async fn write_read_via_meta(&self, path: &str, bytes: &[u8]) -> Vec<u8> {
        use crate::posix_api::Filer;
        let provider = self
            .provider
            .clone()
            .unwrap_or_else(|| Arc::new(crate::meta_sqlite::SqliteMeta::new()));
        let filer = Filer::new(provider);
        filer.write(path, 0, bytes).await.ok();
        filer.read_all(path).await.unwrap_or_default()
    }

    /// 返回当前使用的后端 name。
    pub fn meta_backend_const() -> &'static [&'static str] {
        META_BACKENDS
    }
}

fn stable_hash(s: &str) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

#[allow(dead_code)]
fn attr_from_name(name: &str, size: u64) -> Attr {
    Attr {
        ino: stable_hash(name),
        parent: 1,
        name: name.to_string(),
        mode: 0o100644,
        uid: 0,
        gid: 0,
        size,
        atime: 0,
        mtime: 0,
        ctime: 0,
        nlink: 1,
        data: Vec::new(),
        symlink: None,
    }
}

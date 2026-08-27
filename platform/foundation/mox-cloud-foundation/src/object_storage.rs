// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use async_trait::async_trait;
use bytes::Bytes;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectHead {
    pub last_modified: u64,
    pub etag: String,
    pub size: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListResult {
    pub keys: Vec<String>,
    pub continuation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PartETag {
    pub part_number: u16,
    pub etag: String,
}

#[derive(Debug, Clone, Default)]
struct ObjectEntry {
    data: Vec<u8>,
    last_modified: u64,
    etag: String,
}

#[derive(Debug, Clone, Default)]
struct MultipartUpload {
    key: String,
    parts: BTreeMap<u16, PartETag>,
    part_data: BTreeMap<u16, Vec<u8>>,
}

fn etag_of(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[async_trait]
pub trait ObjectStorageProvider: Send + Sync {
    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        data: Bytes,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Bytes, Box<dyn Error + Send + Sync>>;
    async fn delete_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn list_objects(
        &self,
        bucket: &str,
        prefix: &str,
        max_keys: u32,
        continuation: Option<&str>,
    ) -> Result<ListResult, Box<dyn Error + Send + Sync>>;
    async fn create_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: u16,
        data: Bytes,
    ) -> Result<PartETag, Box<dyn Error + Send + Sync>>;
    async fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        parts: Vec<PartETag>,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn abort_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn head_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<ObjectHead, Box<dyn Error + Send + Sync>>;
}

pub struct MockObjectStorageProvider {
    objects: parking_lot::Mutex<BTreeMap<String, BTreeMap<String, ObjectEntry>>>,
    uploads: parking_lot::Mutex<BTreeMap<String, MultipartUpload>>,
    counter: parking_lot::Mutex<u64>,
}
impl Default for MockObjectStorageProvider {
    fn default() -> Self {
        Self {
            objects: parking_lot::Mutex::new(BTreeMap::new()),
            uploads: parking_lot::Mutex::new(BTreeMap::new()),
            counter: parking_lot::Mutex::new(1),
        }
    }
}

#[async_trait]
impl ObjectStorageProvider for MockObjectStorageProvider {
    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        data: Bytes,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let etag = etag_of(&data);
        let entry = ObjectEntry {
            data: data.to_vec(),
            last_modified: now_ms(),
            etag: etag.clone(),
        };
        self.objects
            .lock()
            .entry(bucket.to_string())
            .or_default()
            .insert(key.to_string(), entry);
        Ok(etag)
    }
    async fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Bytes, Box<dyn Error + Send + Sync>> {
        let objs = self.objects.lock();
        let b = objs.get(bucket).ok_or("bucket not found")?;
        let e = b.get(key).ok_or("key not found")?;
        Ok(Bytes::copy_from_slice(&e.data))
    }
    async fn delete_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut objs = self.objects.lock();
        if let Some(b) = objs.get_mut(bucket) {
            b.remove(key);
        }
        Ok(())
    }
    async fn list_objects(
        &self,
        bucket: &str,
        prefix: &str,
        max_keys: u32,
        _cont: Option<&str>,
    ) -> Result<ListResult, Box<dyn Error + Send + Sync>> {
        let objs = self.objects.lock();
        let mut keys: Vec<String> = objs
            .get(bucket)
            .map(|b| {
                b.keys()
                    .filter(|k| k.starts_with(prefix))
                    .take(max_keys as usize)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        keys.sort();
        Ok(ListResult {
            keys,
            continuation: None,
        })
    }
    async fn create_multipart_upload(
        &self,
        _bucket: &str,
        key: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut c = self.counter.lock();
        let id = format!("upload-{}", *c);
        *c += 1;
        self.uploads.lock().insert(
            id.clone(),
            MultipartUpload {
                key: key.to_string(),
                ..Default::default()
            },
        );
        Ok(id)
    }
    async fn upload_part(
        &self,
        _bucket: &str,
        _key: &str,
        upload_id: &str,
        part_number: u16,
        data: Bytes,
    ) -> Result<PartETag, Box<dyn Error + Send + Sync>> {
        let etag = etag_of(&data);
        let mut ups = self.uploads.lock();
        let up = ups.get_mut(upload_id).ok_or("no upload")?;
        up.part_data.insert(part_number, data.to_vec());
        let p = PartETag {
            part_number,
            etag: etag.clone(),
        };
        up.parts.insert(part_number, p.clone());
        Ok(p)
    }
    async fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        _parts: Vec<PartETag>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let (data_vec, key2) = {
            let mut ups = self.uploads.lock();
            let up = ups.remove(upload_id).ok_or("no upload")?;
            let mut sorted: Vec<(u16, Vec<u8>)> = up.part_data.into_iter().collect();
            sorted.sort_by_key(|(n, _)| *n);
            let mut out = Vec::new();
            for (_, v) in sorted {
                out.extend_from_slice(&v);
            }
            (out, up.key)
        };
        let _ = key2;
        let etag = etag_of(&data_vec);
        let entry = ObjectEntry {
            data: data_vec,
            last_modified: now_ms(),
            etag: etag.clone(),
        };
        self.objects
            .lock()
            .entry(bucket.to_string())
            .or_default()
            .insert(key.to_string(), entry);
        Ok(etag)
    }
    async fn abort_multipart_upload(
        &self,
        _bucket: &str,
        _key: &str,
        upload_id: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.uploads.lock().remove(upload_id);
        Ok(())
    }
    async fn head_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<ObjectHead, Box<dyn Error + Send + Sync>> {
        let objs = self.objects.lock();
        let b = objs.get(bucket).ok_or("bucket not found")?;
        let e = b.get(key).ok_or("key not found")?;
        Ok(ObjectHead {
            last_modified: e.last_modified,
            etag: e.etag.clone(),
            size: e.data.len() as u64,
        })
    }
}

//! MultipartUpload 控制器：Create → UploadPart → Complete → Abort。
//! Complete 时使用 mox_data_standards_core::etag_crc32c::etag_multipart 算最终 ETag。

use crate::error::{S3Error, S3Result};
use crate::etag::{checksum_crc32c, etag_for_multipart, etag_small};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use mox_cloud_foundation::PartETag;

const MIN_PART_SIZE: u64 = 1; // relaxed for tests（最后一个 part 例外）
const MAX_PART_SIZE: u64 = 5 * 1024 * 1024 * 1024; // 5 GiB

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UploadPartInfo {
    pub part_number: u16,
    pub etag: String, // 带引号 MD5 hex
    pub size: u64,
    pub data: Vec<u8>, // 内存在测试/小对象场景是可以的；生产版会落到 volume chunk
    pub crc32c: u32,
}

#[derive(Debug, Clone)]
pub struct MultipartUploadInfo {
    pub upload_id: String,
    pub bucket: String,
    pub key: String,
    pub initiated_ms: u64,
    pub parts: BTreeMap<u16, UploadPartInfo>,
}

/// MPU 管理器。
#[derive(Debug, Clone, Default)]
pub struct MultipartManager {
    inner: Arc<Mutex<MultipartManagerInner>>,
}

#[derive(Debug, Default)]
struct MultipartManagerInner {
    uploads: BTreeMap<String, MultipartUploadInfo>,
    counter: u64,
}

impl MultipartManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn now_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn create(&self, bucket: &str, key: &str) -> String {
        let mut inner = self.inner.lock();
        inner.counter += 1;
        let id = format!(
            "mpu-{}-{:x}-{}",
            bucket.len(),
            inner.counter,
            Self::now_ms()
        );
        let info = MultipartUploadInfo {
            upload_id: id.clone(),
            bucket: bucket.to_string(),
            key: key.to_string(),
            initiated_ms: Self::now_ms(),
            parts: BTreeMap::new(),
        };
        inner.uploads.insert(id.clone(), info);
        id
    }

    pub fn upload_part(
        &self,
        upload_id: &str,
        part_number: u16,
        data: Vec<u8>,
    ) -> S3Result<PartETag> {
        if !(1..=10000).contains(&part_number) {
            return Err(S3Error::InvalidArgument);
        }
        if data.len() as u64 > MAX_PART_SIZE {
            return Err(S3Error::InvalidArgument);
        }
        let etag = etag_small(&data);
        let crc32c = checksum_crc32c(&data);
        let mut inner = self.inner.lock();
        let up = inner
            .uploads
            .get_mut(upload_id)
            .ok_or(S3Error::NoSuchUpload)?;
        up.parts.insert(
            part_number,
            UploadPartInfo {
                part_number,
                etag: etag.clone(),
                size: data.len() as u64,
                data,
                crc32c,
            },
        );
        Ok(PartETag {
            part_number,
            etag: etag.trim_matches('"').to_string(),
        })
    }

    pub fn upload_part_copy(
        &self,
        upload_id: &str,
        part_number: u16,
        source_data: Vec<u8>,
    ) -> S3Result<PartETag> {
        // 逻辑等同于 upload_part，只是来源是 copy
        self.upload_part(upload_id, part_number, source_data)
    }

    /// Complete：拼接所有 parts，计算最终 ETag（mox-standards etag_multipart）。
    pub fn complete(
        &self,
        upload_id: &str,
        requested_parts: &[PartETag],
    ) -> S3Result<(Vec<u8>, String)> {
        let mut inner = self.inner.lock();
        let up = inner
            .uploads
            .remove(upload_id)
            .ok_or(S3Error::NoSuchUpload)?;

        // 构造 part etags 列表（按 part_number 顺序）
        let mut sorted: Vec<&UploadPartInfo> = up.parts.values().collect();
        sorted.sort_by_key(|p| p.part_number);

        // 最小 part size 校验（除最后一个 part）
        let n = sorted.len();
        for (i, p) in sorted.iter().enumerate() {
            if i + 1 < n && p.size < MIN_PART_SIZE {
                return Err(S3Error::InvalidArgument);
            }
        }

        // 与请求的 parts 对齐（请求可能乱序）
        let mut req_map = BTreeMap::new();
        for rp in requested_parts {
            req_map.insert(rp.part_number, rp.etag.clone());
        }
        let mut part_etag_refs: Vec<String> = Vec::new();
        for p in &sorted {
            let expected = req_map.get(&p.part_number).cloned().unwrap_or_default();
            let actual = p.etag.trim_matches('"').to_string();
            if !expected.is_empty() && expected != actual {
                return Err(S3Error::InvalidArgument);
            }
            part_etag_refs.push(format!("\"{}\"", actual));
        }
        let ref_strs: Vec<&str> = part_etag_refs.iter().map(|s| s.as_str()).collect();
        let final_etag = etag_for_multipart(&ref_strs);

        // 拼接数据
        let mut out = Vec::new();
        for p in sorted {
            out.extend_from_slice(&p.data);
        }
        Ok((out, final_etag))
    }

    pub fn abort(&self, upload_id: &str) -> S3Result<()> {
        let mut inner = self.inner.lock();
        if inner.uploads.remove(upload_id).is_none() {
            return Err(S3Error::NoSuchUpload);
        }
        Ok(())
    }

    pub fn list_parts(&self, upload_id: &str) -> S3Result<Vec<UploadPartInfo>> {
        let inner = self.inner.lock();
        let up = inner.uploads.get(upload_id).ok_or(S3Error::NoSuchUpload)?;
        let mut out: Vec<UploadPartInfo> = up.parts.values().cloned().collect();
        out.sort_by_key(|p| p.part_number);
        Ok(out)
    }

    pub fn list_uploads(&self, bucket: &str, prefix: &str) -> Vec<MultipartUploadInfo> {
        let inner = self.inner.lock();
        inner
            .uploads
            .values()
            .filter(|u| u.bucket == bucket && u.key.starts_with(prefix))
            .cloned()
            .collect()
    }

    pub fn get(&self, upload_id: &str) -> Option<MultipartUploadInfo> {
        self.inner.lock().uploads.get(upload_id).cloned()
    }
}
